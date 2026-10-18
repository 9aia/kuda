//! kuda-core — domain types: cards, symbols, edges, provenance,
//! and the `kuda.toml` config model. No ports, no integrations.
//!
//! Memory tiers (ADR-0001): L0 [`RepoCard`] (always-load), L1
//! [`ModuleCard`] (on-demand), L2 [`Symbol`]/[`DepEdge`] graph
//! (queried). Every card carries [`Provenance`] — staleness is a
//! property of the data, not a promise in prose.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Hex content digest of a file at index time — the staleness key.
/// Algorithm is pinned by the extractor, not the domain.
pub type ContentHash = String;

/// Where a memory artifact's claims came from. `status` diffs
/// `commit_sha` vs HEAD and re-hashes `files` — drift is computed,
/// never assumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// Repo-relative paths the artifact describes.
    pub files: Vec<PathBuf>,
    /// Parallel to `files` — hash at index time.
    pub hashes: Vec<ContentHash>,
    /// Commit the index was built at.
    pub commit_sha: String,
    /// RFC3339 build time.
    pub generated_at: String,
    /// "kuda-template/0.1.0"-style generator stamp.
    pub generator: String,
}

/// A named code element the index tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: PathBuf,
    /// 1-based line of the definition.
    pub line: u32,
    /// Owning module/card name — the index's primary grouping key.
    pub module: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Type,
    Trait,
    Const,
    Module,
    Macro,
    Other,
}

/// A directed edge in the dep graph — module- or package-level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeKind {
    Imports,
    Calls,
    PackageDep,
    WorkspaceMember,
}

/// L1 card — one per module/crate/package. Machine owns `api`/`deps`/
/// `verify`; humans own `notes` (regen never clobbers it).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCard {
    pub name: String,
    /// Module root (dir or manifest path).
    pub path: PathBuf,
    /// One line — template adapter takes the first doc line.
    pub purpose: String,
    /// Public surface found by extraction.
    pub api: Vec<Symbol>,
    /// Outgoing module deps.
    pub deps: Vec<String>,
    /// Human-owned region — survives regeneration.
    #[serde(default)]
    pub notes: String,
    /// Files an agent should open before trusting this card.
    pub verify: Vec<PathBuf>,
    pub provenance: Provenance,
}

/// L0 card — the always-load repo summary (~1k tokens rendered).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCard {
    pub name: String,
    pub root: PathBuf,
    /// One-line purpose (README tagline / manifest description).
    pub purpose: String,
    /// Module card names — the map.
    pub modules: Vec<String>,
    /// Build/test/verify commands (from manifests + known files).
    pub commands: Vec<String>,
    pub provenance: Provenance,
}

/// Language descriptor — loaded from `data/langs/*` at runtime.
/// A new language is a data change, not an engine change (ADR-0001).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangDef {
    pub name: String,
    /// File extensions including the dot (".rs").
    pub extensions: Vec<String>,
    /// Manifest basenames for the `manifest` extractor
    /// ("Cargo.toml", "package.json").
    pub manifests: Vec<String>,
    /// tree-sitter grammar id for the `treesitter` extractor.
    #[serde(default)]
    pub grammar: Option<String>,
    /// tree-sitter symbol query (source text of the query file).
    #[serde(default)]
    pub symbol_query: Option<String>,
    /// Query for import edges — per-language (e.g. `use`/`import`).
    #[serde(default)]
    pub dep_query: Option<String>,
}

// ---- kuda.toml config model ----

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub repo: RepoCfg,
    #[serde(default)]
    pub extract: ExtractCfg,
    #[serde(default)]
    pub summarize: SummarizeCfg,
    #[serde(default)]
    pub store: StoreCfg,
    #[serde(default)]
    pub cards: CardsCfg,
    #[serde(default)]
    pub workspace: WorkspaceCfg,
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct RepoCfg {
    /// Defaults to the directory basename.
    pub name: Option<String>,
    /// Repo root; default = the toml's dir.
    #[serde(default)]
    pub root: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ExtractCfg {
    /// Impl kinds tried in order: "manifest" | "treesitter" | "lsp".
    #[serde(default = "default_extract_kinds")]
    pub kinds: Vec<String>,
    /// Server command for the lsp adapter (per-language later).
    #[serde(default)]
    pub lsp_server: Option<String>,
}

impl Default for ExtractCfg {
    fn default() -> Self {
        Self {
            kinds: default_extract_kinds(),
            lsp_server: None,
        }
    }
}

fn default_extract_kinds() -> Vec<String> {
    vec!["manifest".into(), "treesitter".into()]
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SummarizeCfg {
    /// "template" (deterministic, default) | "llm".
    #[serde(default = "default_summarize_kind")]
    pub kind: String,
    /// Provider id for the llm adapter; key resolves via secrets chain.
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

impl Default for SummarizeCfg {
    fn default() -> Self {
        Self {
            kind: default_summarize_kind(),
            provider: None,
            model: None,
        }
    }
}

fn default_summarize_kind() -> String {
    "template".into()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct StoreCfg {
    #[serde(default = "default_store_kind")]
    pub kind: String,
    /// L2 index path — rebuildable, gitignored.
    #[serde(default = "default_store_path")]
    pub path: PathBuf,
}

impl Default for StoreCfg {
    fn default() -> Self {
        Self {
            kind: default_store_kind(),
            path: default_store_path(),
        }
    }
}

fn default_store_kind() -> String {
    "sqlite".into()
}
fn default_store_path() -> PathBuf {
    ".kuda/index.sqlite".into()
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CardsCfg {
    /// Committed artifacts dir: index.yaml + modules/*.yaml.
    #[serde(default = "default_cards_dir")]
    pub dir: PathBuf,
}

impl Default for CardsCfg {
    fn default() -> Self {
        Self {
            dir: default_cards_dir(),
        }
    }
}

fn default_cards_dir() -> PathBuf {
    ".kuda".into()
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCfg {
    /// Sibling repo roots indexed together.
    #[serde(default)]
    pub roots: Vec<PathBuf>,
    /// Resolve `git = ".../org/repo"` manifest deps to siblings.
    #[serde(default)]
    pub resolve_git_deps: bool,
}

/// A revision-ish string is path-safe — rejects path-injection
/// before values reach worktree paths. (Kept from the fork.)
pub fn is_valid_sha(sha: &str) -> bool {
    (7..=64).contains(&sha.len())
        && !sha.contains("..")
        && !sha.contains('/')
        && sha
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// Publish order for workspace members: every member's path-deps on
/// other members come first. Err on a dependency cycle. Used by
/// scripts/release.sh.
pub fn publish_order(members: &[String], deps: &[(String, String)]) -> Result<Vec<String>, String> {
    let member_set: std::collections::HashSet<&str> = members.iter().map(String::as_str).collect();
    let mut done: std::collections::HashSet<String> = Default::default();
    let mut out = Vec::with_capacity(members.len());
    while out.len() < members.len() {
        let mut progressed = false;
        for m in members {
            if done.contains(m) {
                continue;
            }
            let ready = deps
                .iter()
                .filter(|(a, _)| a == m)
                .all(|(_, d)| !member_set.contains(d.as_str()) || done.contains(d));
            if ready {
                done.insert(m.clone());
                out.push(m.clone());
                progressed = true;
            }
        }
        if !progressed {
            return Err("dependency cycle in workspace".into());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::publish_order;

    #[test]
    fn order_respects_path_deps() {
        let members = vec!["kuda".into(), "kuda-core".into(), "kuda-manifest".into()];
        let deps = vec![
            ("kuda".into(), "kuda-manifest".into()),
            ("kuda".into(), "kuda-core".into()),
            ("kuda-manifest".into(), "kuda-core".into()),
        ];
        let order = publish_order(&members, &deps).unwrap();
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("kuda-core") < pos("kuda-manifest"));
        assert!(pos("kuda-manifest") < pos("kuda"));
    }

    #[test]
    fn cycle_errors() {
        let members = vec!["a".into(), "b".into()];
        let deps = vec![("a".into(), "b".into()), ("b".into(), "a".into())];
        assert!(publish_order(&members, &deps).is_err());
    }
}
