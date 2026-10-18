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
    /// e.g. "zed agent", "claude", "gemini --experimental-acp".
    pub cmd: String,
    /// Repo root — the agent gets real paths to cite.
    pub root: PathBuf,
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
            if self.reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                return Err("agent closed stdout".into());
            }
            let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
                continue;
            };
            if msg["method"].as_str() == Some("session/update") {
                if let Some(t) = msg["params"]["update"]["content"]["text"].as_str() {
                    chunks.push_str(t);
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
    let syms: Vec<String> = ext.symbols.iter().map(|s| s.name.clone()).take(30).collect();
    let files: Vec<String> = ext
        .files
        .iter()
        .map(|f| f.display().to_string())
        .take(15)
        .collect();
    format!(
        "You are writing a module card for the repo memory index of module `{module}`.\n\
         Public symbols: {}\nSource files: {}\n\n\
         Reply with ONLY the card body: 1-2 sentence `purpose:` line, then \
         `invariants:` bullets (skip if none evident), then `notes:` bullets \
         for gotchas. Ground every claim — it will be re-verified against \
         the named files. No preamble, no fences.",
        syms.join(", "),
        files.join(", ")
    )
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
        let mut client = AcpClient::spawn(&self.cmd)?;
        client.request(
            "initialize",
            json!({"protocolVersion":1,"clientCapabilities":{},"clientInfo":{"name":"kuda","version":"0.1.0"}}),
        )?;
        let (_, _) = client.request(
            "session/new",
            json!({"cwd": self.root, "mcpServers": []}),
        )?;
        let (_res, prose) = client.request(
            "session/prompt",
            json!({"sessionId":"kuda-default",
                   "prompt":[{"type":"text","text":prompt_for(module, ext)}]}),
        )?;

        // Card stays deterministic in structure — the agent owns
        // only the prose (purpose + notes region is *card-owned*
        // here; human notes merge is a TODO).
        let mut lines = prose.lines().map(str::trim).filter(|l| !l.is_empty());
        let purpose = lines.next().unwrap_or(module).to_owned();
        let notes: String = lines.collect::<Vec<_>>().join("\n");
        Ok(ModuleCard {
            name: module.into(),
            path: path.to_path_buf(),
            purpose,
            api: ext.symbols.clone(),
            deps: ext.edges.iter().map(|e| e.to.clone()).collect(),
            notes,
            verify: ext.files.clone(),
            provenance,
        })
    }
}
