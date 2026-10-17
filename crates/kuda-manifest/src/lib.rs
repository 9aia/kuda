//! Manifest impl — package manifests -> module graph. No parser,
//! no language knowledge: reads manifest basenames declared by
//! `LangDef` entries (Cargo.toml, package.json, go.mod, …).

use kuda_core::{DepEdge, EdgeKind, LangDef, Symbol, SymbolKind};
use kuda_extract::{Extraction, Extractor};
use std::collections::HashSet;
use std::path::Path;

pub struct ManifestExtractor;

impl Extractor for ManifestExtractor {
    fn kind(&self) -> &'static str {
        "manifest"
    }

    fn extract(&self, root: &Path, langs: &[LangDef]) -> Result<Extraction, String> {
        let names: HashSet<&str> = langs
            .iter()
            .flat_map(|l| l.manifests.iter().map(String::as_str))
            .collect();
        let mut out = Extraction::default();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in entries.flatten() {
                let path = e.path();
                let name = e.file_name();
                let name = name.to_string_lossy();
                if path.is_dir() {
                    // skip noise dirs + mounted git worktrees (.git
                    // as a *file* marks a worktree, not a repo dir)
                    if !matches!(name.as_ref(), ".git" | "target" | "node_modules" | ".kuda")
                        && !path.join(".git").is_file()
                    {
                        stack.push(path);
                    }
                    continue;
                }
                if !names.contains(name.as_ref()) {
                    continue;
                }
                let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let (pkg, deps) = parse_manifest(name.as_ref(), &text);
                let module = pkg.unwrap_or_else(|| {
                    path.parent()
                        .and_then(|p| p.file_name())
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_else(|| name.clone().into_owned())
                });
                out.symbols.push(Symbol {
                    name: module.clone(),
                    kind: SymbolKind::Module,
                    file: rel.clone(),
                    line: 1,
                    module: module.clone(),
                });
                for d in deps {
                    out.edges.push(DepEdge {
                        from: module.clone(),
                        to: d,
                        kind: EdgeKind::PackageDep,
                    });
                }
                out.files.push(rel);
            }
        }
        Ok(out)
    }
}

/// Manifest basename -> (package name, declared deps). Format
/// detection by filename — impls are allowed to know concrete tech.
fn parse_manifest(basename: &str, text: &str) -> (Option<String>, Vec<String>) {
    match basename {
        "Cargo.toml" => cargo(text),
        "package.json" => package_json(text),
        "go.mod" => go_mod(text),
        _ => (None, Vec::new()),
    }
}

fn cargo(text: &str) -> (Option<String>, Vec<String>) {
    let Ok(t) = text.parse::<toml::Table>() else {
        return (None, Vec::new());
    };
    let name = t
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_owned);
    let mut deps = Vec::new();
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(tbl) = t.get(key).and_then(|d| d.as_table()) {
            deps.extend(tbl.keys().cloned());
        }
    }
    (name, deps)
}

fn package_json(text: &str) -> (Option<String>, Vec<String>) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else {
        return (None, Vec::new());
    };
    let name = v.get("name").and_then(|n| n.as_str()).map(str::to_owned);
    let mut deps = Vec::new();
    for key in ["dependencies", "devDependencies"] {
        if let Some(tbl) = v.get(key).and_then(|d| d.as_object()) {
            deps.extend(tbl.keys().cloned());
        }
    }
    (name, deps)
}

fn go_mod(text: &str) -> (Option<String>, Vec<String>) {
    let name = text
        .lines()
        .find_map(|l| l.strip_prefix("module ").map(str::trim).map(str::to_owned));
    let deps = text
        .lines()
        .filter(|l| l.starts_with('\t') && l.contains(" v"))
        .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
        .collect();
    (name, deps)
}
