//! Manifest adapter — package manifests -> module graph. No parser,
//! no language knowledge: reads manifest basenames declared by
//! `LangDef` entries (Cargo.toml, package.json, go.mod, …).

use kuda_core::{DepEdge, EdgeKind, LangDef, Symbol, SymbolKind};
use kuda_extract::{Extraction, Extractor};
use std::collections::HashSet;
use std::path::Path;

/// `excludes`: extra dir names from `[extract] exclude`.
/// `resolve_git_deps`: from `[workspace] resolve_git_deps` — emit
/// a Cargo `git = "<url>"` dep's target as the URL's repo basename
/// so the edge lands on a sibling repo module.
pub struct ManifestExtractor {
    pub excludes: Vec<String>,
    pub resolve_git_deps: bool,
}

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
                    if !kuda_core::dir_excluded(name.as_ref(), &self.excludes)
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
                let (pkg, deps) =
                    parse_manifest(name.as_ref(), &text, self.resolve_git_deps);
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
/// detection by filename — adapters are allowed to know concrete tech.
fn parse_manifest(
    basename: &str,
    text: &str,
    resolve_git_deps: bool,
) -> (Option<String>, Vec<String>) {
    match basename {
        "Cargo.toml" => cargo(text, resolve_git_deps),
        "package.json" => package_json(text),
        "pyproject.toml" => pyproject(text),
        "setup.py" => setup_py(text),
        "go.mod" => go_mod(text),
        _ => (None, Vec::new()),
    }
}

/// PEP 508 requirement string -> bare package name
/// ("neuralmind-shared[extra]>=1.0; python>=3.9" -> "neuralmind-shared").
fn pep508_name(req: &str) -> String {
    req.trim()
        .split([' ', '[', '<', '>', '=', '~', '!', ';', '('])
        .next()
        .unwrap_or("")
        .to_owned()
}

fn pyproject(text: &str) -> (Option<String>, Vec<String>) {
    let Ok(t) = text.parse::<toml::Table>() else {
        return (None, Vec::new());
    };
    let project = t.get("project").and_then(|p| p.as_table());
    let poetry = t
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.as_table());
    let name = project
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .or_else(|| poetry.and_then(|p| p.get("name")).and_then(|n| n.as_str()))
        .map(str::to_owned);
    let mut deps = Vec::new();
    // PEP 621: dependencies = ["a>=1", ...]; optional-dependencies = {grp=[...]}
    for key in ["dependencies", "optional-dependencies"] {
        match project.and_then(|p| p.get(key)) {
            Some(toml::Value::Array(a)) => {
                deps.extend(a.iter().filter_map(|v| v.as_str()).map(pep508_name))
            }
            Some(toml::Value::Table(tbl)) => {
                for v in tbl.values().filter_map(|v| v.as_array()).flatten() {
                    if let Some(s) = v.as_str() {
                        deps.push(pep508_name(s));
                    }
                }
            }
            _ => {}
        }
    }
    // Poetry: [tool.poetry.dependencies] + dev-dependencies table keys
    // (skip `python`); group deps nest one level deeper.
    let mut poetry_tables: Vec<&toml::map::Map<String, toml::Value>> = Vec::new();
    for key in ["dependencies", "dev-dependencies"] {
        poetry_tables.extend(poetry.and_then(|p| p.get(key)).and_then(|d| d.as_table()));
    }
    if let Some(groups) = poetry.and_then(|p| p.get("group")).and_then(|g| g.as_table()) {
        for g in groups.values() {
            poetry_tables.extend(g.get("dependencies").and_then(|d| d.as_table()));
        }
    }
    for tbl in poetry_tables {
        deps.extend(tbl.keys().filter(|k| k.as_str() != "python").cloned());
    }
    // PEP 735 / uv: [dependency-groups] dev = ["pytest>=8", ...]
    if let Some(groups) = t.get("dependency-groups").and_then(|g| g.as_table()) {
        for v in groups.values().filter_map(|v| v.as_array()).flatten() {
            if let Some(s) = v.as_str() {
                deps.push(pep508_name(s));
            }
        }
    }
    deps.retain(|d| !d.is_empty());
    (name, deps)
}

/// `name=` kwarg scan — setup.py deps live in code, not data; the
/// package name alone is enough for module identity.
fn setup_py(text: &str) -> (Option<String>, Vec<String>) {
    let name = text
        .split("name=")
        .nth(1)
        .and_then(|rest| rest.trim_start().strip_prefix(['\'', '"']))
        .and_then(|rest| rest.split(['\'', '"']).next())
        .map(str::to_owned)
        .filter(|n| !n.is_empty());
    (name, Vec::new())
}

fn cargo(text: &str, resolve_git_deps: bool) -> (Option<String>, Vec<String>) {
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
            deps.extend(
                tbl.iter()
                    .map(|(k, v)| cargo_dep_name(k, v, resolve_git_deps)),
            );
        }
    }
    (name, deps)
}

/// Cargo dep-table entry -> edge target. `package = "real"` renames
/// win (with or without `git`); then, when `resolve_git_deps` is on,
/// a `git = "<url>"` source yields the URL's repo basename (last
/// path segment minus `.git`). `workspace = true` and everything
/// else keep the table key.
fn cargo_dep_name(key: &str, v: &toml::Value, resolve_git_deps: bool) -> String {
    let Some(tbl) = v.as_table() else {
        return key.to_owned();
    };
    if let Some(pkg) = tbl.get("package").and_then(|p| p.as_str()) {
        return pkg.to_owned();
    }
    if resolve_git_deps && !tbl.contains_key("workspace") {
        if let Some(url) = tbl.get("git").and_then(|g| g.as_str()) {
            let base = url
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or_default();
            let base = base.strip_suffix(".git").unwrap_or(base);
            if !base.is_empty() {
                return base.to_owned();
            }
        }
    }
    key.to_owned()
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

#[cfg(test)]
mod tests {
    use super::{parse_manifest, pep508_name};

    #[test]
    fn pep508_strips_extras_markers_and_versions() {
        assert_eq!(pep508_name("neuralmind-shared[extra]>=1.0; python_version>=\"3.9\""), "neuralmind-shared");
        assert_eq!(pep508_name("pytest (>=8,<9)"), "pytest");
        assert_eq!(pep508_name("requests"), "requests");
    }

    #[test]
    fn pyproject_pep621() {
        let text = r#"
[project]
name = "neuralmind-shared"
dependencies = ["httpx>=0.27", "pydantic[email]"]

[project.optional-dependencies]
dev = ["pytest>=8"]

[dependency-groups]
lint = ["ruff"]
"#;
        let (name, deps) = parse_manifest("pyproject.toml", text, false);
        assert_eq!(name.as_deref(), Some("neuralmind-shared"));
        for d in ["httpx", "pydantic", "pytest", "ruff"] {
            assert!(deps.iter().any(|x| x == d), "missing dep {d}");
        }
    }

    #[test]
    fn pyproject_poetry() {
        let text = r#"
[tool.poetry]
name = "poet-pkg"

[tool.poetry.dependencies]
python = "^3.11"
requests = "^2"

[tool.poetry.group.dev.dependencies]
pytest = "^8"
"#;
        let (name, deps) = parse_manifest("pyproject.toml", text, false);
        assert_eq!(name.as_deref(), Some("poet-pkg"));
        assert!(deps.iter().any(|d| d == "requests"));
        assert!(deps.iter().any(|d| d == "pytest"));
        assert!(!deps.iter().any(|d| d == "python"));
    }

    #[test]
    fn setup_py_name_kwarg() {
        let (name, _) = parse_manifest("setup.py", "setup(\n    name='legacypkg',\n    version='1.0',\n)", false);
        assert_eq!(name.as_deref(), Some("legacypkg"));
    }

    #[test]
    fn cargo_git_dep_resolves_to_repo_basename() {
        let text = r#"
[package]
name = "app"

[dependencies]
helper = { git = "https://github.com/org/sibling-repo.git" }
other = { git = "git@github.com:org/trailing-slash/" }
"#;
        let (_, deps) = parse_manifest("Cargo.toml", text, true);
        assert!(deps.iter().any(|d| d == "sibling-repo"), "deps: {deps:?}");
        assert!(deps.iter().any(|d| d == "trailing-slash"), "deps: {deps:?}");
        assert!(!deps.iter().any(|d| d == "helper"), "deps: {deps:?}");
    }

    #[test]
    fn cargo_package_rename_uses_real_name() {
        let text = r#"
[dependencies]
serde2 = { package = "serde", version = "1" }
renamed = { git = "https://github.com/org/whatever", package = "real-crate" }
"#;
        // `package` wins whether or not git is present — and the flag
        // doesn't gate it (a rename is a rename).
        let (_, deps) = parse_manifest("Cargo.toml", text, false);
        assert!(deps.iter().any(|d| d == "serde"), "deps: {deps:?}");
        assert!(deps.iter().any(|d| d == "real-crate"), "deps: {deps:?}");
        assert!(!deps.iter().any(|d| d == "serde2" || d == "renamed"), "deps: {deps:?}");
    }

    #[test]
    fn cargo_plain_and_workspace_deps_keep_key() {
        let text = r#"
[dependencies]
serde = "1"
shared = { workspace = true }
local = { path = "../local" }
"#;
        let (_, deps) = parse_manifest("Cargo.toml", text, true);
        for d in ["serde", "shared", "local"] {
            assert!(deps.iter().any(|x| x == d), "missing dep {d}");
        }
    }

    #[test]
    fn cargo_git_dep_flag_off_keeps_key() {
        let text = r#"
[dependencies]
helper = { git = "https://github.com/org/sibling-repo.git" }
"#;
        let (_, deps) = parse_manifest("Cargo.toml", text, false);
        assert!(deps.iter().any(|d| d == "helper"), "deps: {deps:?}");
        assert!(!deps.iter().any(|d| d == "sibling-repo"), "deps: {deps:?}");
    }
}
