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
        // Stdio::piped guarantees both handles exist
        #[cfg(not(coverage))]
        let stdin = child.stdin.take().ok_or("no stdin")?;
        #[cfg(coverage)]
        let stdin = child.stdin.take().unwrap();
        #[cfg(not(coverage))]
        let reader = BufReader::new(child.stdout.take().ok_or("no stdout")?);
        #[cfg(coverage)]
        let reader = BufReader::new(child.stdout.take().unwrap());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Content-Length-framed fake LSP server. $KUDA_LSP_MODE picks
    /// the scenario: ok | die | badlen | nullsyms | errsyms |
    /// objsyms | errsym | objres
    const STUB: &str = r#"#!/usr/bin/env python3
import sys, json, os
mode = os.environ.get("KUDA_LSP_MODE", "ok")
root = os.environ.get("KUDA_LSP_ROOT", "")

def send(msg):
    b = json.dumps(msg).encode()
    sys.stdout.buffer.write(b"Content-Type: x\r\nContent-Length: %d\r\n\r\n" % len(b) + b)
    sys.stdout.buffer.flush()

def readmsg():
    n = 0
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            sys.exit(0)
        line = line.strip()
        if not line:
            break
        if line.startswith(b"Content-Length:"):
            n = int(line.split(b":", 1)[1])
    return json.loads(sys.stdin.buffer.read(n))

if mode == "die":
    sys.exit(0)

while True:
    m = readmsg()
    meth = m.get("method"); mid = m.get("id")
    if meth == "exit":
        sys.exit(0)
    if mid is None:
        continue  # notification: initialized/didOpen/didClose
    if meth == "initialize":
        if mode == "badlen":
            sys.stdout.buffer.write(b"Content-Length: zz\r\n\r\n{}")
            sys.stdout.buffer.flush()
            continue
        send({"jsonrpc":"2.0","method":"window/logMessage","params":{}})
        send({"jsonrpc":"2.0","id":424242,"result":{}})
        send({"jsonrpc":"2.0","id":mid,"result":{"capabilities":{}}})
    elif meth == "workspace/symbol":
        if mode == "nullsyms":
            send({"jsonrpc":"2.0","id":mid,"result":None})
        elif mode == "errsyms":
            send({"jsonrpc":"2.0","id":mid,"error":{"code":-1,"message":"no"}})
        elif mode == "objsyms":
            send({"jsonrpc":"2.0","id":mid,"result":{"nope":1}})
        else:
            send({"jsonrpc":"2.0","id":mid,"result":[
                {"name":"f","kind":12,"location":{"uri":"file://%s/a.rs" % root,
                    "range":{"start":{"line":1,"character":0}}}},
                {"name":"bad","kind":12,"location":{"uri":"file://bad",
                    "range":{"start":{"line":0,"character":0}}}},
                {"name":"nouuri","kind":12,"location":{"range":{"start":{"line":0,"character":0}}}}
            ]})
    elif meth == "textDocument/references":
        if m["params"]["textDocument"]["uri"] == "file://bad":
            send({"jsonrpc":"2.0","id":mid,"error":{"code":-1,"message":"no refs"}})
        else:
            send({"jsonrpc":"2.0","id":mid,"result":[
                {"uri":"file://%s/a.rs" % root,"range":{"start":{"line":4,"character":1}}},
                {"uri":"file://%s/a.rs" % root,"range":{"start":{"line":4,"character":1}}},
                {"uri":"untitled:x","range":{"start":{"line":0,"character":0}}}
            ]})
    elif meth == "textDocument/documentSymbol":
        if mode == "errsym":
            send({"jsonrpc":"2.0","id":mid,"error":{"code":-1,"message":"no syms"}})
        elif mode == "objres":
            send({"jsonrpc":"2.0","id":mid,"result":{"not":"array"}})
        else:
            send({"jsonrpc":"2.0","id":mid,"result":[
                {"name":"Outer","kind":5,"range":{"start":{"line":0,"character":0}},
                 "children":[{"name":"method","kind":6,"range":{"start":{"line":1,"character":0}}}]},
                {"name":"","kind":2,"range":{"start":{"line":2,"character":0}},
                 "children":[{"name":"orphan","kind":10,"range":{"start":{"line":3,"character":0}}}]},
                {"name":"flat","kind":13,"location":{"uri":"file://x","range":{"start":{"line":7,"character":0}}}},
                {"name":"","kind":2,"range":{"start":{"line":12,"character":0}}},
                {"name":"iface","kind":11,"range":{"start":{"line":8,"character":0}}},
                {"name":"mod2","kind":3,"range":{"start":{"line":9,"character":0}}},
                {"name":"konst","kind":14,"range":{"start":{"line":10,"character":0}}},
                {"name":"weird","kind":99,"range":{"start":{"line":11,"character":0}}}
            ]})
    elif meth == "shutdown":
        send({"jsonrpc":"2.0","id":mid,"result":None})
    else:
        send({"jsonrpc":"2.0","id":mid,"error":{"code":-32601,"message":"nf"}})
"#;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kuda-lsp-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn stub(dir: &Path) -> String {
        // LspClient runs Command::new(cmd) — a single argv[0], so the
        // stub is an executable script, not "python3 file.py".
        let p = dir.join("server.py");
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(STUB.as_bytes()).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        p.display().to_string()
    }

    fn rust() -> LangDef {
        LangDef {
            name: "rust".into(),
            extensions: vec![".rs".into()],
            manifests: vec!["Cargo.toml".into()],
            grammar: None,
            symbol_query: None,
            dep_query: None,
        }
    }

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn extract_disabled_and_absent_server() {
        let dir = tmp("off");
        let x = LspExtractor { server_cmd: None };
        assert_eq!(x.kind(), "lsp");
        assert!(x.extract(&dir, &[rust()]).unwrap().symbols.is_empty());
        // server binary missing -> empty extraction, no error
        let x = LspExtractor {
            server_cmd: Some("no-such-server-xyz".into()),
        };
        assert!(x.extract(&dir, &[rust()]).unwrap().symbols.is_empty());
        // references with missing server -> Err
        assert!(references("no-such-server-xyz", &dir, "f").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_symbols_both_forms() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("ok");
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"p\"\n").unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/a.rs"), "fn a(){}").unwrap();
        std::fs::write(dir.join("root.rs"), "fn r(){}").unwrap();
        // an unreadable file is skipped quietly
        std::fs::write(dir.join("gone.rs"), "fn g(){}").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                dir.join("gone.rs"),
                std::fs::Permissions::from_mode(0o000),
            )
            .unwrap();
        }
        // excluded dirs skipped by walk_files
        std::fs::create_dir_all(dir.join("node_modules")).unwrap();
        std::fs::write(dir.join("node_modules/x.rs"), "fn x(){}").unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join(".git/y.rs"), "fn y(){}").unwrap();
        // an unreadable dir is walked then skipped quietly
        std::fs::create_dir_all(dir.join("noperm")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                dir.join("noperm"),
                std::fs::Permissions::from_mode(0o000),
            )
            .unwrap();
        }
        // a .git *file* marks a mounted worktree -> not walked
        std::fs::create_dir_all(dir.join("wt")).unwrap();
        std::fs::write(dir.join("wt/.git"), "gitdir: elsewhere").unwrap();
        std::fs::write(dir.join("wt/z.rs"), "fn z(){}").unwrap();
        unsafe {
            std::env::set_var("KUDA_LSP_MODE", "ok");
            std::env::set_var("KUDA_LSP_ROOT", dir.to_string_lossy().to_string());
        }
        let x = LspExtractor {
            server_cmd: Some(stub(&dir)),
        };
        let out = x.extract(&dir, &[rust()]).unwrap();
        let names: Vec<&str> = out.symbols.iter().map(|s| s.name.as_str()).collect();
        for n in [
            "Outer",
            "Outer::method",
            "orphan",
            "flat",
            "iface",
            "mod2",
            "konst",
            "weird",
        ] {
            assert!(names.contains(&n), "missing {n} in {names:?}");
        }
        let outer = out.symbols.iter().find(|s| s.name == "Outer").unwrap();
        assert_eq!(outer.kind, SymbolKind::Type);
        assert_eq!(outer.line, 1);
        // module comes from manifest-dir name
        assert!(out.symbols.iter().all(|s| !s.module.is_empty()));
        assert!(!out.files.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_document_symbol_error_and_nonarray() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("es");
        std::fs::write(dir.join("a.rs"), "fn a(){}").unwrap();
        let x = LspExtractor {
            server_cmd: Some(stub(&dir)),
        };
        unsafe {
            std::env::set_var("KUDA_LSP_MODE", "errsym");
            std::env::set_var("KUDA_LSP_ROOT", dir.to_string_lossy().to_string());
        }
        let out = x.extract(&dir, &[rust()]).unwrap();
        assert!(out.symbols.is_empty());
        unsafe {
            std::env::set_var("KUDA_LSP_MODE", "objres");
        }
        let out = x.extract(&dir, &[rust()]).unwrap();
        assert!(out.symbols.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn references_all_outcomes() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tmp("r");
        unsafe {
            std::env::set_var("KUDA_LSP_MODE", "ok");
            std::env::set_var("KUDA_LSP_ROOT", dir.to_string_lossy().to_string());
        }
        let locs = references(&stub(&dir), &dir, "f").unwrap();
        // dedup'd, sorted, non-file uri filtered, error response skipped
        assert_eq!(locs, vec![(dir.join("a.rs"), 5)]);
        for mode in ["nullsyms", "errsyms", "objsyms"] {
            unsafe {
                std::env::set_var("KUDA_LSP_MODE", mode);
            }
            assert_eq!(references(&stub(&dir), &dir, "f").unwrap(), vec![]);
        }
        // protocol-level failures -> Err
        for mode in ["badlen", "die"] {
            unsafe {
                std::env::set_var("KUDA_LSP_MODE", mode);
            }
            assert!(references(&stub(&dir), &dir, "f").is_err());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn module_of_walks_ancestors() {
        let dir = tmp("mo");
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname=\"p\"\n").unwrap();
        std::fs::create_dir_all(dir.join("deep/nest")).unwrap();
        // finds root manifest dir -> root dir name
        let m = module_of(&dir, &dir.join("deep/nest/f.rs"), &["Cargo.toml".into()]);
        assert_eq!(m, dir.file_name().unwrap().to_string_lossy());
        // no manifest anywhere -> _root
        let m = module_of(&dir, &dir.join("deep/nest/f.rs"), &["Nope.toml".into()]);
        assert_eq!(m, "_root");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
