//! Lsp adapter — server-backed extraction via `textDocument/documentSymbol`.
//! Spawns one server per root; per-server command comes from
//! `extract.lsp_server`. Missing server = empty extraction
//! (other extractors still run). Quality tier above treesitter:
//! real symbols where a language server exists.

use kuda_core::{LangDef, Symbol, SymbolKind};
use kuda_extract::{Extraction, Extractor};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub struct LspExtractor {
    /// Server command, e.g. "rust-analyzer". None disables.
    pub server_cmd: Option<String>,
}

struct LspClient {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    next_id: u64,
}

impl LspClient {
    fn spawn(cmd: &str, root: &Path) -> Result<Self, String> {
        let mut child = Command::new(cmd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn {cmd}: {e}"))?;
        let stdin = child.stdin.take().ok_or("no stdin")?;
        let reader = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        let mut c = Self {
            child,
            stdin,
            reader,
            next_id: 0,
        };
        let root_uri = format!("file://{}", root.display());
        c.request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {},
                "workspaceFolders": [{"uri": root_uri, "name": "root"}],
            }),
        )?;
        let _ = c.notify("initialized", json!({}));
        Ok(c)
    }

    fn send(&mut self, msg: &Value) -> Result<(), String> {
        let body = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body)
            .and_then(|_| self.stdin.flush())
            .map_err(|e| e.to_string())
    }

    fn recv(&mut self) -> Result<Value, String> {
        let mut len = 0usize;
        loop {
            let mut line = String::new();
            self.reader
                .read_line(&mut line)
                .map_err(|e| e.to_string())?;
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if let Some(n) = line.strip_prefix("Content-Length:") {
                len = n.trim().parse().map_err(|_| "bad length")?;
            }
        }
        let mut buf = vec![0u8; len];
        self.reader
            .read_exact(&mut buf)
            .map_err(|e| e.to_string())?;
        serde_json::from_slice(&buf).map_err(|e| e.to_string())
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        loop {
            let msg = self.recv()?;
            if msg["id"].as_u64() == Some(id) {
                if let Some(err) = msg.get("error") {
                    return Err(format!("{method}: {err}"));
                }
                return Ok(msg["result"].clone());
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        self.send(&json!({"jsonrpc":"2.0","method":method,"params":params}))
    }

    fn shutdown(&mut self) {
        let _ = self.request("shutdown", Value::Null);
        let _ = self.notify("exit", Value::Null);
        let _ = self.child.wait();
    }
}

/// LSP SymbolKind int -> ours.
fn lsp_kind(n: u64) -> SymbolKind {
    match n {
        5 | 23 | 8 | 9 => SymbolKind::Type, // class/struct/interface/enum-member-ish
        10 => SymbolKind::Type,             // enum
        11 => SymbolKind::Trait,            // interface
        12 | 6 => SymbolKind::Function,     // function/method
        13 | 14 => SymbolKind::Const,       // variable/constant
        2 | 3 => SymbolKind::Module,        // module/namespace
        _ => SymbolKind::Other,
    }
}

fn flatten_syms(node: &Value, file: &Path, module: &str, out: &mut Vec<Symbol>, container: &str) {
    let name = node["name"].as_str().unwrap_or("");
    if !name.is_empty() {
        let full = if container.is_empty() {
            name.to_owned()
        } else {
            format!("{container}::{name}")
        };
        out.push(Symbol {
            name: full.clone(),
            kind: lsp_kind(node["kind"].as_u64().unwrap_or(0)),
            file: file.to_path_buf(),
            line: node["range"]["start"]["line"].as_u64().unwrap_or(0) as u32 + 1,
            module: module.to_owned(),
        });
        if let Some(children) = node["children"].as_array() {
            for c in children {
                flatten_syms(c, file, module, out, &full);
            }
        }
        return;
    }
    if let Some(children) = node["children"].as_array() {
        for c in children {
            flatten_syms(c, file, module, out, container);
        }
    }
}

fn module_of(root: &Path, file: &Path, manifests: &[String]) -> String {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if manifests.iter().any(|m| d.join(m).exists()) {
            return d
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| "_root".into());
        }
        if !d.starts_with(root) {
            break;
        }
        dir = d.parent();
    }
    "_root".into()
}

fn walk_files(root: &Path, exts: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if !matches!(
                    e.file_name().to_str(),
                    Some(".git" | "target" | "node_modules" | ".kuda")
                ) && !p.join(".git").is_file()
                {
                    stack.push(p);
                }
            } else if p
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| exts.iter().any(|e| e.trim_start_matches('.') == x))
                .unwrap_or(false)
            {
                out.push(p);
            }
        }
    }
    out
}

/// Semantic references — `workspace/symbol` to locate, then
/// `textDocument/references` per hit. Real cross-file usage, the
/// tier above text search. Returns (file, line) pairs.
pub fn references(
    server_cmd: &str,
    root: &Path,
    name: &str,
) -> Result<Vec<(PathBuf, u32)>, String> {
    let mut client = LspClient::spawn(server_cmd, root)?;
    let res = client.request("workspace/symbol", json!({"query": name}));
    let syms = match res {
        Ok(Value::Array(a)) => a,
        Ok(Value::Null) | Err(_) => {
            client.shutdown();
            return Ok(vec![]);
        }
        Ok(_) => vec![],
    };
    let mut out = Vec::new();
    for s in syms {
        let uri = s["location"]["uri"].as_str().unwrap_or("");
        let pos = &s["location"]["range"]["start"];
        if uri.is_empty() {
            continue;
        }
        let refs = client.request(
            "textDocument/references",
            json!({
                "textDocument": {"uri": uri},
                "position": {"line": pos["line"], "character": pos["character"]},
                "context": {"includeDeclaration": true},
            }),
        );
        if let Ok(Value::Array(locs)) = refs {
            for l in locs {
                let luri = l["uri"].as_str().unwrap_or("");
                let line = l["range"]["start"]["line"].as_u64().unwrap_or(0) as u32 + 1;
                if let Some(path) = luri.strip_prefix("file://") {
                    out.push((PathBuf::from(path), line));
                }
            }
        }
    }
    client.shutdown();
    out.sort();
    out.dedup();
    Ok(out)
}

impl Extractor for LspExtractor {
    fn kind(&self) -> &'static str {
        "lsp"
    }

    fn extract(&self, root: &Path, langs: &[LangDef]) -> Result<Extraction, String> {
        let mut out = Extraction::default();
        let Some(cmd) = self.server_cmd.as_deref() else {
            return Ok(out); // no server configured — silently skip
        };
        let mut client = match LspClient::spawn(cmd, root) {
            Ok(c) => c,
            Err(_) => return Ok(out), // server absent — other tiers still run
        };
        for lang in langs {
            for file in walk_files(root, &lang.extensions) {
                let Ok(text) = std::fs::read_to_string(&file) else {
                    continue;
                };
                let rel = file.strip_prefix(root).unwrap_or(&file).to_path_buf();
                let uri = format!("file://{}", file.display());
                let _ = client.notify(
                    "textDocument/didOpen",
                    json!({"textDocument":{"uri":uri,"languageId":lang.name,"version":1,"text":text}}),
                );
                let res = client.request(
                    "textDocument/documentSymbol",
                    json!({"textDocument":{"uri":uri}}),
                );
                let _ = client.notify("textDocument/didClose", json!({"textDocument":{"uri":uri}}));
                let Ok(syms) = res else { continue };
                let module = module_of(root, &file, &lang.manifests);
                if let Some(arr) = syms.as_array() {
                    for s in arr {
                        if s["location"].is_object() {
                            // SymbolInformation (flat) form
                            out.symbols.push(Symbol {
                                name: s["name"].as_str().unwrap_or("").to_owned(),
                                kind: lsp_kind(s["kind"].as_u64().unwrap_or(0)),
                                file: rel.clone(),
                                line: s["location"]["range"]["start"]["line"]
                                    .as_u64()
                                    .unwrap_or(0) as u32
                                    + 1,
                                module: module.clone(),
                            });
                        } else {
                            flatten_syms(s, &rel, &module, &mut out.symbols, "");
                        }
                    }
                }
                out.files.push(rel);
            }
        }
        client.shutdown();
        Ok(out)
    }
}
