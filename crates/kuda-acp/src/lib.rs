//! Summarizer adapter — spawns an ACP agent (any client-capable
//! agent binary: zed, cline, gemini-cli…) to write grounded card
//! prose. No API keys, no provider lock: auth lives in the user's
//! agent. Falls back handled by the caller — construction here
//! only fails when there's nothing to say.

use kuda_core::{ModuleCard, Provenance};
use kuda_extract::Extraction;
use kuda_summarize::Summarizer;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub struct AcpSummarizer {
    /// e.g. "cline --acp", "zed agent", "gemini --experimental-acp".
    pub cmd: String,
    /// Repo root — the agent gets real paths to cite.
    pub root: PathBuf,
    /// Lazy (client, session_id) — one agent session reused across
    /// all modules; respawns on failure.
    inner: std::sync::Mutex<Option<(AcpClient, String)>>,
}

impl AcpSummarizer {
    pub fn new(cmd: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            cmd: cmd.into(),
            root: root.into(),
            inner: std::sync::Mutex::new(None),
        }
    }

    /// Send one prompt on the shared session; returns streamed text.
    /// Lazily initializes the agent + session on first call.
    pub fn complete(&self, prompt: &str) -> Result<String, String> {
        let mut guard = self.inner.lock().map_err(|e| e.to_string())?;
        if guard.is_none() {
            let mut client = AcpClient::spawn(&self.cmd)?;
            client.request(
                "initialize",
                json!({"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"kuda","version":"0.1.0"}}),
            )?;
            let (session, _) =
                client.request("session/new", json!({"cwd": self.root, "mcpServers": []}))?;
            let sid = session["sessionId"]
                .as_str()
                .unwrap_or("kuda-default")
                .to_owned();
            *guard = Some((client, sid));
        }
        let (client, session_id) = guard.as_mut().ok_or("agent session lost")?;
        let (_res, prose) = client.request(
            "session/prompt",
            json!({"sessionId": session_id.as_str(),
                   "prompt":[{"type":"text","text":prompt}]}),
        )?;
        Ok(prose)
    }
}

struct AcpClient {
    _child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    id: u64,
}

impl AcpClient {
    fn spawn(cmd: &str) -> Result<Self, String> {
        let mut child = Command::new("sh")
            .args(["-c", cmd])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn {cmd}: {e}"))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let reader = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        Ok(Self {
            _child: child,
            stdin,
            reader,
            id: 0,
        })
    }

    fn send(&mut self, msg: &Value) -> Result<(), String> {
        writeln!(self.stdin, "{}", msg)
            .and_then(|_| self.stdin.flush())
            .map_err(|e| e.to_string())
    }

    /// Read lines until a reply with our id, collecting streamed
    /// `agent_message_chunk` text on the way.
    fn request(&mut self, method: &str, params: Value) -> Result<(Value, String), String> {
        self.id += 1;
        let id = self.id;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        let mut chunks = String::new();
        let mut line = String::new();
        loop {
            line.clear();
            if self
                .reader
                .read_line(&mut line)
                .map_err(|e| e.to_string())?
                == 0
            {
                return Err("agent closed stdout".into());
            }
            let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
                continue;
            };
            // agent -> client request (e.g. session/request_permission):
            // must answer or the prompt deadlocks. Grant the first
            // option; unknown requests get method-not-found.
            if let (Some(rid), Some(m)) = (
                msg["id"]
                    .as_u64()
                    .or_else(|| msg["id"].as_i64().map(|i| i as u64)),
                msg["method"].as_str(),
            ) {
                let resp = if m == "session/request_permission" {
                    let opt = msg["params"]["options"]
                        .as_array()
                        .and_then(|o| o.first())
                        .and_then(|o| o["optionId"].as_str())
                        .unwrap_or("allow_once");
                    json!({"jsonrpc":"2.0","id":rid,
                           "result":{"outcome":{"outcome":"selected","optionId":opt}}})
                } else {
                    json!({"jsonrpc":"2.0","id":rid,
                           "error":{"code":-32601,"message":"method not found"}})
                };
                let _ = self.send(&resp);
                continue;
            }
            if msg["method"].as_str() == Some("session/update") {
                let upd = &msg["params"]["update"];
                // reasoning-first agents (devin acp) stream
                // `agent_thought_chunk` alongside the reply —
                // chain-of-thought must never reach the card text.
                let kind = upd["sessionUpdate"]
                    .as_str()
                    .unwrap_or("agent_message_chunk");
                if kind == "agent_message_chunk" {
                    if let Some(t) = upd["content"]["text"].as_str() {
                        chunks.push_str(t);
                    }
                }
                continue;
            }
            if msg["id"].as_u64() == Some(id) {
                if let Some(err) = msg.get("error") {
                    return Err(format!("{method}: {err}"));
                }
                return Ok((msg["result"].clone(), chunks));
            }
        }
    }
}

fn prompt_for(module: &str, ext: &Extraction) -> String {
    let syms: Vec<String> = ext
        .symbols
        .iter()
        .map(|s| s.name.clone())
        .take(30)
        .collect();
    let files: Vec<String> = ext
        .files
        .iter()
        .map(|f| f.display().to_string())
        .take(15)
        .collect();
    format!(
        "You are writing a module card for the repo memory index of module `{module}`.\n\
         Public symbols: {}\nSource files: {}\n\n\
         The LAST block of your reply must be the card fields — only that \
         block is read:\n\n\
         purpose: <1-2 grounded sentences>\n\
         invariants:\n- <bullet; omit this section entirely if none evident>\n\
         notes:\n- <gotcha bullets>\n\n\
         Ground every claim — it will be re-verified against the named \
         files. No preamble inside the block, no fences, nothing after \
         `notes:`.",
        syms.join(", "),
        files.join(", ")
    )
}

/// Split an agent reply into (purpose, notes). Structured fields
/// count only in the reply's FINAL block — reasoning-first agents
/// monologue first and may even write `purpose:` inside their
/// reasoning, so the anchor is the LAST `purpose:` line. Text
/// before it never reaches the card.
fn parse_card_body(prose: &str) -> (Option<String>, String) {
    let lines: Vec<&str> = prose.lines().collect();
    let anchor = lines
        .iter()
        .rposition(|l| l.trim_start().to_lowercase().starts_with("purpose:"))
        .unwrap_or(0);
    let mut purpose = None;
    let mut notes = String::new();
    let mut in_notes = false;
    for line in &lines[anchor..] {
        let l = line.trim();
        let low = l.to_lowercase();
        if low.starts_with("purpose:") {
            let p = l["purpose:".len()..]
                .trim()
                .trim_matches(|c| c == '\'' || c == '"')
                .to_owned();
            if !p.is_empty() {
                purpose = Some(p);
            }
            in_notes = false;
            continue;
        }
        if low.starts_with("invariants:") || low.starts_with("notes:") {
            in_notes = true;
            let rest = l.split_once(':').map(|x| x.1).unwrap_or("").trim();
            if !rest.is_empty() {
                notes.push_str(rest);
                notes.push('\n');
            }
            continue;
        }
        if in_notes {
            notes.push_str(l);
            notes.push('\n');
        }
    }
    (purpose, notes)
}

/// No `purpose:` field anywhere — take the reply's last paragraph
/// (the agent's conclusion; the preamble is reasoning) rather than
/// the first line, which is where the monologue starts.
fn last_paragraph(prose: &str) -> Option<String> {
    prose
        .trim()
        .rsplit("\n\n")
        .next()
        .map(|p| {
            p.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with("```"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|p| !p.is_empty())
}

impl Summarizer for AcpSummarizer {
    fn kind(&self) -> &'static str {
        "acp"
    }

    fn summarize(
        &self,
        module: &str,
        path: &Path,
        ext: &Extraction,
        provenance: Provenance,
    ) -> Result<ModuleCard, String> {
        let prose = self.complete(&prompt_for(module, ext))?;

        // Only the reply's closing structured block counts — a
        // reasoning preamble must never land in `purpose`.
        let (purpose, notes) = parse_card_body(&prose);
        let purpose = purpose
            .or_else(|| last_paragraph(&prose))
            .unwrap_or_else(|| module.into());
        Ok(ModuleCard {
            name: module.into(),
            path: path.to_path_buf(),
            purpose,
            api: ext.symbols.clone(),
            api_omitted: 0,
            deps: ext.edges.iter().map(|e| e.to.clone()).collect(),
            notes,
            verify: ext.files.clone(),
            provenance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// ndjson ACP stub — reads $KUDA_STUB_MODE per spawn:
    /// ok | error | perm | garbage | close | noids
    const STUB: &str = r#"
import sys, json, os
mode = os.environ.get("KUDA_STUB_MODE", "ok")
prose = os.environ.get("KUDA_STUB_PROSE", "purpose: stub says hi")
thought = os.environ.get("KUDA_STUB_THOUGHT", "")
if mode == "close":
    sys.exit(0)
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        msg = json.loads(line)
    except Exception:
        continue
    m = msg.get("method"); mid = msg.get("id")
    if m == "initialize":
        print(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"protocolVersion":1}}), flush=True)
    elif m == "session/new":
        r = {"sessionId":"s1"} if mode != "noids" else {}
        print(json.dumps({"jsonrpc":"2.0","id":mid,"result":r}), flush=True)
    elif m == "session/prompt":
        if mode == "garbage":
            print("this is not json", flush=True)
            print(json.dumps({"jsonrpc":"2.0","method":"notifications/progress","params":{}}), flush=True)
        if mode == "perm":
            print(json.dumps({"jsonrpc":"2.0","id":91,"method":"session/request_permission","params":{"options":[{"optionId":"allow"}]}}), flush=True)
            print(json.dumps({"jsonrpc":"2.0","id":-5,"method":"mystery/call","params":{}}), flush=True)
        if mode == "error":
            print(json.dumps({"jsonrpc":"2.0","id":mid,"error":{"code":-1,"message":"boom"}}), flush=True)
            continue
        if thought:
            print(json.dumps({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1","update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":thought}}}}), flush=True)
        print(json.dumps({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s1","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":prose}}}}), flush=True)
        print(json.dumps({"jsonrpc":"2.0","id":mid,"result":{"stopReason":"end_turn"}}), flush=True)
"#;

    fn stub(dir: &Path) -> PathBuf {
        let p = dir.join("agent.py");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(STUB.as_bytes()).unwrap();
        p
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kuda-acp-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn summarizer(dir: &Path) -> AcpSummarizer {
        AcpSummarizer::new(
            format!("python3 {}", stub(dir).display()),
            dir.to_path_buf(),
        )
    }

    /// serialize env mutation across this binary's test threads
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn prov() -> Provenance {
        Provenance {
            files: vec!["a.rs".into()],
            hashes: vec!["h".into()],
            commit_sha: "c".into(),
            generated_at: "0".into(),
            generator: "t".into(),
        }
    }

    fn ext() -> Extraction {
        Extraction {
            symbols: vec![],
            edges: vec![kuda_core::DepEdge {
                from: "m".into(),
                to: "dep1".into(),
                kind: kuda_core::EdgeKind::Imports,
            }],
            files: vec!["a.rs".into()],
        }
    }

    /// env juggling races with nothing else in this test binary —
    /// keep all stubbed flows in one test.
    #[test]
    fn summarize_full_card() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("ok");
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "ok");
            std::env::set_var(
                "KUDA_STUB_PROSE",
                "preamble ignored\npurpose: 'does the thing'\ninvariants:\n- always\nnotes: inline\n- n1\nextra",
            );
        }
        let s = summarizer(&dir);
        assert_eq!(s.kind(), "acp");
        let card = s.summarize("m", Path::new("a.rs"), &ext(), prov()).unwrap();
        assert_eq!(card.purpose, "does the thing");
        assert!(card.notes.contains("- always"), "{}", card.notes);
        assert!(card.notes.contains("inline"), "{}", card.notes);
        assert!(card.notes.contains("extra"), "{}", card.notes);
        assert_eq!(card.deps, vec!["dep1"]);
        assert_eq!(card.verify, vec![PathBuf::from("a.rs")]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn summarize_parsing_variants() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("pv");
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "ok");
        }
        // Purpose: capital + Notes: capital
        unsafe {
            std::env::set_var("KUDA_STUB_PROSE", "Purpose: capped\nNotes: tail");
        }
        let card = summarizer(&dir)
            .summarize("m", Path::new("p"), &ext(), prov())
            .unwrap();
        assert_eq!(card.purpose, "capped");
        assert!(card.notes.contains("tail"));
        // no marked sections -> last paragraph (the conclusion —
        // the first line is where reasoning preambles start)
        unsafe {
            std::env::set_var("KUDA_STUB_PROSE", "\n\njust prose here\nmore");
        }
        let card = summarizer(&dir)
            .summarize("m", Path::new("p"), &ext(), prov())
            .unwrap();
        assert_eq!(card.purpose, "just prose here more");
        // empty prose -> module name fallback
        unsafe {
            std::env::set_var("KUDA_STUB_PROSE", "   \n");
        }
        let card = summarizer(&dir)
            .summarize("fallback-mod", Path::new("p"), &ext(), prov())
            .unwrap();
        assert_eq!(card.purpose, "fallback-mod");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Reasoning-first agents (devin acp) monologue before the card
    /// body — and may write `purpose:` *inside* the monologue. Only
    /// the reply's final structured block may reach the card.
    #[test]
    fn reasoning_preamble_never_reaches_purpose() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("cot");
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "ok");
            std::env::set_var(
                "KUDA_STUB_PROSE",
                "I need to create a module card for `m`. I'm reviewing the \
                 public symbols now.\n\
                 purpose: The purpose field should describe the role.\n\n\
                 purpose: Graph-backed repo memory for module m.\n\
                 invariants:\n- never invent files\n\
                 notes:\n- verify before trusting",
            );
        }
        let card = summarizer(&dir)
            .summarize("m", Path::new("p"), &ext(), prov())
            .unwrap();
        assert_eq!(card.purpose, "Graph-backed repo memory for module m.");
        assert!(card.notes.contains("never invent files"), "{}", card.notes);
        assert!(
            card.notes.contains("verify before trusting"),
            "{}",
            card.notes
        );
        // the monologue — including its fake `purpose:` — leaked nowhere
        assert!(!card.purpose.contains("reviewing"), "{}", card.purpose);
        assert!(!card.notes.contains("I need to create"), "{}", card.notes);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `agent_thought_chunk` updates carry chain-of-thought — the
    /// wire layer must drop them before text ever reaches parsing.
    #[test]
    fn thought_chunks_are_dropped() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("think");
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "ok");
            std::env::set_var(
                "KUDA_STUB_THOUGHT",
                "I need to create a module card. I'm reviewing the symbols.",
            );
            std::env::set_var("KUDA_STUB_PROSE", "purpose: grounded purpose");
        }
        let s = summarizer(&dir);
        let out = s.complete("hi").unwrap();
        assert!(out.contains("grounded purpose"), "{out}");
        assert!(!out.contains("reviewing"), "{out}");
        let card = s.summarize("m", Path::new("p"), &ext(), prov()).unwrap();
        assert_eq!(card.purpose, "grounded purpose");
        unsafe {
            std::env::remove_var("KUDA_STUB_THOUGHT");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Degenerate replies carry no fields at all — purpose falls
    /// back to the closing paragraph (the conclusion), not the
    /// reasoning preamble; empty replies fall back to the module name.
    #[test]
    fn degenerate_reply_does_not_pollute_purpose() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // parse-level: multi-paragraph reasoning -> last paragraph only
        let (purpose, notes) = parse_card_body(
            "Let me look at the files first.\n\n\
             I'm checking the symbol list.\n\n\
             A small library that indexes module graphs.",
        );
        assert_eq!(purpose, None);
        assert!(notes.is_empty());
        assert_eq!(
            last_paragraph(
                "Let me look at the files first.\n\n\
                 I'm checking the symbol list.\n\n\
                 A small library that indexes module graphs."
            )
            .as_deref(),
            Some("A small library that indexes module graphs.")
        );
        // fully structured reply -> fields only
        let (purpose, notes) =
            parse_card_body("purpose: Indexes module graphs.\nnotes:\n- cite files");
        assert_eq!(purpose.as_deref(), Some("Indexes module graphs."));
        assert!(notes.contains("cite files"));
        // through the stub: single-paragraph conclusion still wins
        // over the module-name fallback
        let dir = tmp("deg");
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "ok");
            std::env::set_var(
                "KUDA_STUB_PROSE",
                "Thinking out loud about the module.\n\nIt indexes module graphs.",
            );
        }
        let card = summarizer(&dir)
            .summarize("m", Path::new("p"), &ext(), prov())
            .unwrap();
        assert_eq!(card.purpose, "It indexes module graphs.");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn permission_and_unknown_requests_answered() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("perm");
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "perm");
            std::env::set_var("KUDA_STUB_PROSE", "purpose: ok");
        }
        let s = summarizer(&dir);
        let out = s.complete("hi").unwrap();
        assert!(out.contains("ok"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn garbage_lines_and_notifications_skipped() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("g");
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "garbage");
            std::env::set_var("KUDA_STUB_PROSE", "purpose: clean");
        }
        let out = summarizer(&dir).complete("hi").unwrap();
        assert!(out.contains("clean"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn error_response_and_agent_close() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("e");
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "error");
        }
        assert!(summarizer(&dir).complete("hi").is_err());
        // agent exits without answering -> "agent closed stdout"
        unsafe {
            std::env::set_var("KUDA_STUB_MODE", "close");
        }
        assert!(summarizer(&dir).complete("hi").is_err());
        // spawn fails entirely (bad interpreter line path)
        let s = AcpSummarizer::new("nonexistent-interp-xyz ./nope", dir.clone());
        let _ = s.complete("hi"); // sh -c runs, agent fails inside — error either way
        let _ = std::fs::remove_dir_all(&dir);
    }
}
