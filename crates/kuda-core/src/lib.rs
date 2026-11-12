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
    /// Non-code record from a domain corpus (`kuda-records`) —
    /// task/ADR/issue ids found by descriptor regexes.
    Record,
    Other,
}

/// A directed edge in the dep graph — module- or package-level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EdgeKind {
    Imports,
    Calls,
    PackageDep,
    WorkspaceMember,
    /// Record-to-record mention in a domain corpus (`kuda-records`).
    References,
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
    /// Public surface found by extraction (capped — `api_omitted`
    /// carries the dropped count so the card stays honest).
    pub api: Vec<Symbol>,
    /// Symbols beyond the render cap — the "and N more" count.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub api_omitted: usize,
    /// Outgoing module deps.
    pub deps: Vec<String>,
    /// Machine-owned warnings — invalid records, extraction
    /// anomalies. Surfaced by `card`/status/MCP; empty = clean.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Human-owned region — survives regeneration.
    #[serde(default)]
    pub notes: String,
    /// Files an agent should open before trusting this card.
    pub verify: Vec<PathBuf>,
    pub provenance: Provenance,
}

/// L1 card at file granularity — one per source file. Chunk-fed agent prose, the
/// query tier between module cards and raw source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCard {
    /// Repo-relative path.
    pub path: PathBuf,
    pub module: String,
    /// 1-3 sentence file purpose (agent or template).
    pub purpose: String,
    /// Symbol names defined here (cheap grep surface).
    #[serde(default)]
    pub symbols: Vec<String>,
    /// Single-file provenance — `files`/`hashes` len 1.
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
    /// sha1 over sorted "path:hash" lines — a content-addressed
    /// snapshot of every walked file without embedding the list
    /// (embedding it puts ~10k rows in the always-load card).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub tree_digest: ContentHash,
    /// Files walked at index time — count pairs with `tree_digest`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub files_total: usize,
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

/// Where a domain's corpus lives. Absent = the filesystem tree
/// (`roots`/`extensions`); a non-tree source reads its corpus from
/// somewhere else — `sturn-wal` walks event commits on git refs.
/// The record/edge machinery is identical either way: the
/// descriptor still declares the regexes, the source only changes
/// where "files" and "lines" come from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Source {
    /// sturn coordination WAL: one ref per workspace under
    /// `ref_prefix` (`refs/sturn/wal/<ws>`, compaction archives at
    /// `refs/sturn/archive/<ws>`), each commit's tree holding an
    /// `event.json` op. A repo without matching refs is a silent
    /// skip, same as an absent corpus dir.
    SturnWal {
        /// Ref namespace prefix — `refs/sturn` covers wal and
        /// archive refs alike.
        #[serde(default = "default_ref_prefix")]
        ref_prefix: String,
    },
}

fn default_ref_prefix() -> String {
    "refs/sturn".into()
}

/// Domain descriptor — a non-code record corpus as data, loaded
/// from `data/domains/*` (same trick as [`LangDef`], one level up).
/// The `records` extractor turns descriptor regexes into
/// [`SymbolKind::Record`] symbols and [`EdgeKind::References`]
/// edges; a new corpus is a data change, not an engine change.
/// Also the `[[domain]]` inline-descriptor shape in `kuda.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainDef {
    pub name: String,
    /// Corpus dirs relative to the repo root ("project"). Declared
    /// roots are walked even when they carry a `.git`-file worktree
    /// marker — the declaration is the intent. Empty/absent for
    /// non-tree sources.
    #[serde(default)]
    pub roots: Vec<PathBuf>,
    /// Record file extensions including the dot (".md"). Unused by
    /// non-tree sources.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Non-tree corpus source — absent means the filesystem walk.
    #[serde(default)]
    pub source: Option<Source>,
    /// Module name the corpus groups under — its card + provenance.
    pub module: String,
    /// Card purpose — domain modules have no manifest/README to
    /// fall back on, so the descriptor carries it.
    #[serde(default)]
    pub purpose: Option<String>,
    /// Regexes matched per line; capture group 1 is the record id.
    /// First match line pins the `Symbol` location. Covers both
    /// frontmatter (`id = "T-001"`) and checklist (`- [ ] T-001`)
    /// records — the pattern is the format knowledge. Optional:
    /// `file_records` descriptors emit a record per file and never
    /// scan text.
    #[serde(default)]
    pub record_patterns: Vec<String>,
    /// Regex over file text — every match is a dep-edge target for
    /// the file's records (same-line scope when a file holds
    /// multiple records, whole-file when it holds one).
    #[serde(default)]
    pub ref_pattern: Option<String>,
    /// JSON Schema (draft 2020-12) applied to frontmatter-bearing
    /// corpus files (`+++` toml / `---` yaml). Invalid records stay
    /// indexed — they exist — but land as warnings on the card.
    #[serde(default)]
    pub schema_json: Option<String>,
    /// File-as-record mode: every matched file emits a record
    /// symbol named by its path — images, PDFs, binaries, any
    /// bytes. No text is read; drift rides the content hash.
    /// `record_patterns`/`schema_json` don't apply.
    #[serde(default)]
    pub file_records: bool,
}

// ---- kuda.toml config model ----

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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
    pub domains: DomainsCfg,
    /// `[[domain]]` inline domain descriptors — same shape as
    /// `data/domains/*`. A `name` matching a built-in replaces it
    /// (repo wins); others append. `[domains] domain` is the nested
    /// equivalent — both feed the resolved set.
    #[serde(default)]
    pub domain: Vec<DomainDef>,
    #[serde(default)]
    pub workspace: WorkspaceCfg,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct RepoCfg {
    /// Defaults to the directory basename.
    pub name: Option<String>,
    /// Repo root; default = the toml's dir.
    #[serde(default)]
    pub root: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ExtractCfg {
    /// Impl kinds tried in order:
    /// "manifest" | "treesitter" | "lsp" | "records".
    #[serde(default = "default_extract_kinds")]
    pub kinds: Vec<String>,
    /// Server command for the lsp adapter (per-language later).
    #[serde(default)]
    pub lsp_server: Option<String>,
    /// Extra dir/file names skipped during walks — beyond the
    /// built-in noise set (`default_excludes`). Keeps generated,
    /// vendored, and build output out of index, hashes, and agent
    /// prompts (agent cost is per-file fed in).
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Dir names never worth indexing — applied to every walk.
/// `[extract] exclude` adds repo-specific names on top.
pub fn default_excludes() -> &'static [&'static str] {
    &[
        ".git",
        ".kuda",
        "target",
        "node_modules",
        "dist",
        "build",
        "out",
        ".cache",
        ".next",
        ".nuxt",
        "vendor",
        ".venv",
        "venv",
        "__pycache__",
        "coverage",
        ".turbo",
        ".parcel-cache",
        "generated",
    ]
}

/// Walk-time exclusion check: name against built-ins + config extras.
pub fn dir_excluded(name: &str, extra: &[String]) -> bool {
    default_excludes().contains(&name) || extra.iter().any(|e| e == name)
}

impl Default for ExtractCfg {
    fn default() -> Self {
        Self {
            kinds: default_extract_kinds(),
            lsp_server: None,
            exclude: Vec::new(),
        }
    }
}

fn default_extract_kinds() -> Vec<String> {
    vec!["manifest".into(), "treesitter".into(), "records".into()]
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SummarizeCfg {
    /// "template" (deterministic, default) | "acp" (agent-spawned).
    /// "llm" is a backlog adapter — ACP is the semantic path.
    #[serde(default = "default_summarize_kind")]
    pub kind: String,
    /// ACP agent command for kind="acp" — spawns per summarize.
    #[serde(default)]
    pub acp_cmd: Option<String>,
    /// L1 per-file cards — chunked agent summaries per source
    /// file. Opt-in: ~10-50x the prompt count of module cards,
    /// bounded by per-file hash-skip.
    #[serde(default)]
    pub file_cards: bool,
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
            acp_cmd: None,
            file_cards: false,
            provider: None,
            model: None,
        }
    }
}

fn default_summarize_kind() -> String {
    "template".into()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

/// Repo-local domain overrides — `disable` drops built-in
/// descriptors by `name`; `domain` entries (usually written as the
/// top-level `[[domain]]` array, see [`Config::domain`]) add a
/// corpus or replace a same-named built-in — the repo wins.
/// Resolution is deterministic: built-ins -> minus `disable` ->
/// overlaid with `domain` entries, in declared order.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct DomainsCfg {
    /// Built-in domain names to skip ("myco", "docs", "media").
    #[serde(default)]
    pub disable: Vec<String>,
    /// Inline descriptors nested as `[[domains.domain]]` — the flat
    /// `[[domain]]` spelling ([`Config::domain`]) is the canonical
    /// form; both merge in declared order.
    #[serde(default)]
    pub domain: Vec<DomainDef>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceCfg {
    /// Sibling repo roots indexed together.
    #[serde(default)]
    pub roots: Vec<PathBuf>,
    /// Resolve `git = ".../org/repo"` manifest deps to siblings.
    #[serde(default)]
    pub resolve_git_deps: bool,
}

/// Canonical matching key for module names — package manifests use
/// dist names (`neuralmind-shared`) while imports use crate/module
/// names (`neuralmind_shared`). Both fold to one key.
pub fn module_key(name: &str) -> String {
    name.replace('-', "_").to_lowercase()
}

/// The name spellings an edge target might carry — dependents
/// queries expand through these so dash/underscore misses can't
/// produce false empty results.
pub fn module_variants(name: &str) -> Vec<String> {
    let mut out = vec![name.to_owned()];
    for v in [name.replace('-', "_"), name.replace('_', "-")] {
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Reduce a raw import/dep string to the package specifier an edge
/// should point at. Language-aware: python imports are dotted,
/// go imports keep their full path (module match is prefix-based),
/// npm keeps `@scope/pkg` and drops relative specifiers entirely.
/// None = intra-module (relative) import — no edge.
pub fn dep_root(lang: &str, raw: &str) -> Option<String> {
    let r = raw.trim_matches(|c| c == '"' || c == '\'');
    if r.is_empty() || r.starts_with('.') || r.starts_with("@/") {
        return None;
    }
    match lang {
        "python" => Some(r.split('.').next().unwrap_or(r).to_owned()),
        "go" => Some(r.to_owned()),
        _ if r.starts_with('@') => {
            let mut it = r.split('/');
            match (it.next(), it.next()) {
                (Some(s), Some(p)) if !p.is_empty() => Some(format!("{s}/{p}")),
                _ => None,
            }
        }
        _ => Some(r.split('/').next().unwrap_or(r).to_owned()),
    }
}

/// Resolve a dep specifier (from `dep_root` or a manifest dep) to a
/// known module name — canonicalizing dash/underscore so the edge
/// lands on the module's real card name.
pub fn resolve_module(dep: &str, modules: &[String]) -> Option<String> {
    let want = module_key(dep);
    if let Some(m) = modules.iter().find(|m| module_key(m) == want) {
        return Some(m.clone());
    }
    // subpath imports: "pkg/sub" or "host/org/repo/pkg" (go) belong
    // to the module whose name is a '/'-separated prefix.
    modules
        .iter()
        .find(|m| want.starts_with(&format!("{}/", module_key(m))))
        .cloned()
}

fn is_zero(v: &usize) -> bool {
    *v == 0
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
        let members = vec![
            "kuda".into(),
            "kuda-core".into(),
            "kuda-extract-manifest".into(),
        ];
        let deps = vec![
            ("kuda".into(), "kuda-extract-manifest".into()),
            ("kuda".into(), "kuda-core".into()),
            ("kuda-extract-manifest".into(), "kuda-core".into()),
        ];
        let order = publish_order(&members, &deps).unwrap();
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("kuda-core") < pos("kuda-extract-manifest"));
        assert!(pos("kuda-extract-manifest") < pos("kuda"));
    }

    #[test]
    fn cycle_errors() {
        let members = vec!["a".into(), "b".into()];
        let deps = vec![("a".into(), "b".into()), ("b".into(), "a".into())];
        assert!(publish_order(&members, &deps).is_err());
    }

    #[test]
    fn module_key_folds_dist_and_import_spellings() {
        assert_eq!(super::module_key("neuralmind-shared"), "neuralmind_shared");
        assert_eq!(super::module_key("NeuralMind_Shared"), "neuralmind_shared");
    }

    #[test]
    fn dep_root_per_language() {
        use super::dep_root;
        assert_eq!(dep_root("python", "a.b.c"), Some("a".into()));
        assert_eq!(
            dep_root("python", "neuralmind_shared"),
            Some("neuralmind_shared".into())
        );
        assert_eq!(
            dep_root("typescript", "@org/pkg/sub"),
            Some("@org/pkg".into())
        );
        assert_eq!(dep_root("typescript", "pkg/sub"), Some("pkg".into()));
        assert_eq!(dep_root("typescript", "./sibling"), None);
        assert_eq!(dep_root("typescript", "../up"), None);
        assert_eq!(dep_root("typescript", "@/alias"), None);
        assert_eq!(
            dep_root("go", "github.com/org/repo/sub"),
            Some("github.com/org/repo/sub".into())
        );
        assert_eq!(dep_root("rust", "my_crate"), Some("my_crate".into()));
        assert_eq!(dep_root("typescript", "\"quoted\""), Some("quoted".into()));
    }

    #[test]
    fn resolve_module_normalizes_and_matches_subpaths() {
        let modules = vec![
            "neuralmind-shared".to_owned(),
            "@org/api".to_owned(),
            "github.com/org/repo".to_owned(),
        ];
        let r = |d| super::resolve_module(d, &modules);
        assert_eq!(r("neuralmind_shared"), Some("neuralmind-shared".into()));
        assert_eq!(r("@org/api"), Some("@org/api".into()));
        assert_eq!(
            r("github.com/org/repo/sub"),
            Some("github.com/org/repo".into())
        );
        assert_eq!(r("serde"), None);
    }

    #[test]
    fn module_variants_expands_spellings() {
        assert_eq!(super::module_variants("a"), vec!["a"]);
        assert_eq!(super::module_variants("a-b"), vec!["a-b", "a_b"]);
        assert_eq!(super::module_variants("a_b"), vec!["a_b", "a-b"]);
        // both spellings already present -> no dupes
        assert_eq!(
            super::module_variants("a-b_c"),
            vec!["a-b_c", "a_b_c", "a-b-c"]
        );
    }

    #[test]
    fn dep_root_edge_cases() {
        use super::dep_root;
        assert_eq!(dep_root("rust", ""), None);
        assert_eq!(dep_root("rust", "''"), None);
        assert_eq!(dep_root("typescript", "@scope"), None);
        assert_eq!(dep_root("typescript", "@scope/"), None);
        assert_eq!(
            dep_root("typescript", "@scope/pkg"),
            Some("@scope/pkg".into())
        );
        assert_eq!(dep_root("other", "a/b/c"), Some("a".into()));
    }

    #[test]
    fn is_valid_sha_rules() {
        use super::is_valid_sha;
        assert!(is_valid_sha("abcdef1"));
        assert!(!is_valid_sha("abc")); // too short
        assert!(!is_valid_sha(&"a".repeat(65))); // too long
        assert!(!is_valid_sha("abc..def1"));
        assert!(!is_valid_sha("abc/def1"));
        assert!(!is_valid_sha("abc def1"));
        assert!(is_valid_sha("a-b_c.d12"));
    }

    #[test]
    fn dir_excluded_builtin_and_extra() {
        assert!(super::dir_excluded("target", &[]));
        assert!(super::dir_excluded(".kuda", &[]));
        assert!(!super::dir_excluded("src", &[]));
        assert!(super::dir_excluded("fixtures", &["fixtures".to_owned()]));
    }

    #[test]
    fn publish_order_ignores_external_and_missing_deps() {
        let members = vec!["solo".into(), "lib".into()];
        let deps = vec![("solo".into(), "crates-io-dep".into())];
        let order = super::publish_order(&members, &deps).unwrap();
        assert_eq!(order.len(), 2);
        // dep on a member not yet done still orders correctly
        let order =
            super::publish_order(&["b".into(), "a".into()], &[("b".into(), "a".into())]).unwrap();
        assert_eq!(order, vec!["a", "b"]);
    }

    #[test]
    fn config_defaults_and_toml_parse() {
        let c = super::Config::default();
        assert_eq!(c.summarize.kind, "template");
        assert_eq!(c.store.kind, "sqlite");
        assert_eq!(c.store.path, std::path::PathBuf::from(".kuda/index.sqlite"));
        assert_eq!(c.cards.dir, std::path::PathBuf::from(".kuda"));
        assert_eq!(c.extract.kinds, vec!["manifest", "treesitter", "records"]);
        assert!(c.repo.name.is_none());
        assert!(c.workspace.roots.is_empty());
        // full toml exercises the serde paths
        let t = r#"
[repo]
name = "x"
root = "."
[extract]
kinds = ["lsp"]
lsp_server = "rust-analyzer"
exclude = ["gen"]
[summarize]
kind = "acp"
acp_cmd = "agent --acp"
file_cards = true
provider = "anthropic"
model = "m"
[store]
kind = "sqlite"
path = "idx.db"
[cards]
dir = "cards"
[workspace]
roots = ["../a"]
resolve_git_deps = true
"#;
        let c: super::Config = toml::from_str(t).unwrap();
        assert_eq!(c.repo.name.as_deref(), Some("x"));
        assert_eq!(c.extract.lsp_server.as_deref(), Some("rust-analyzer"));
        assert_eq!(c.extract.exclude, vec!["gen"]);
        assert!(c.summarize.file_cards);
        assert_eq!(c.summarize.provider.as_deref(), Some("anthropic"));
        assert_eq!(c.summarize.model.as_deref(), Some("m"));
        assert!(c.workspace.resolve_git_deps);
        // deny_unknown_fields
        assert!(toml::from_str::<super::Config>("[bogus]\nx = 1").is_err());
        assert!(toml::from_str::<super::Config>("[extract]\nbogus = 1").is_err());
    }

    /// `[domains]` — `disable` names + inline `[[domain]]`
    /// descriptors parse straight into `DomainDef` (the same shape
    /// `data/domains/*` carries).
    #[test]
    fn domains_cfg_parses_inline_descriptors() {
        let t = r#"
[domains]
disable = ["myco", "media"]

[[domain]]
name = "ops"
roots = ["ops/project"]
extensions = [".md"]
module = "ops-ledger"
purpose = "ops ledger"
record_patterns = ["-\\s*(?:\\[[ xX]\\]\\s*)?([A-Z]+-[0-9]+)"]
ref_pattern = "\\b[A-Z]+-[0-9]+\\b"

[[domains.domain]]
name = "assets2"
roots = ["assets"]
extensions = [".png"]
module = "assets"
file_records = true
"#;
        let c: super::Config = toml::from_str(t).unwrap();
        assert_eq!(c.domains.disable, vec!["myco", "media"]);
        // flat [[domain]] and nested [[domains.domain]] both land
        assert_eq!(c.domain.len(), 1);
        assert_eq!(c.domains.domain.len(), 1);
        let d = &c.domain[0];
        assert_eq!(d.name, "ops");
        assert_eq!(d.module, "ops-ledger");
        assert_eq!(d.roots, vec![std::path::PathBuf::from("ops/project")]);
        assert_eq!(d.record_patterns.len(), 1);
        assert_eq!(d.ref_pattern.as_deref(), Some("\\b[A-Z]+-[0-9]+\\b"));
        assert!(!d.file_records);
        assert!(c.domains.domain[0].file_records);
        // defaults stay empty
        assert!(super::Config::default().domain.is_empty());
        // unknown keys inside a [[domain]] entry are rejected
        assert!(toml::from_str::<super::Config>(
            "[[domain]]\nname = \"x\"\nroots = []\nextensions = []\nmodule = \"m\"\nrecord_patterns = []\nbogus = 1"
        )
        .is_err());
    }

    #[test]
    fn domain_types_serde_and_clone() {
        // Clone/Debug/Serialize on the plain domain types.
        let s = super::Symbol {
            name: "f".into(),
            kind: super::SymbolKind::Macro,
            file: "a.rs".into(),
            line: 1,
            module: "m".into(),
        };
        let s2 = s.clone();
        assert_eq!(s2.kind, super::SymbolKind::Macro);
        let e = super::DepEdge {
            from: "a".into(),
            to: "b".into(),
            kind: super::EdgeKind::Calls,
        };
        let _ = e.clone();
        assert_eq!(e.kind, super::EdgeKind::Calls);
        // every SymbolKind variant serializes lowercase
        for (k, want) in [
            (super::SymbolKind::Function, "function"),
            (super::SymbolKind::Type, "type"),
            (super::SymbolKind::Trait, "trait"),
            (super::SymbolKind::Const, "const"),
            (super::SymbolKind::Module, "module"),
            (super::SymbolKind::Macro, "macro"),
            (super::SymbolKind::Other, "other"),
        ] {
            assert_eq!(serde_json::to_value(k).unwrap(), want);
        }
        for (k, want) in [
            (super::EdgeKind::Imports, "imports"),
            (super::EdgeKind::Calls, "calls"),
            (super::EdgeKind::PackageDep, "package-dep"),
            (super::EdgeKind::WorkspaceMember, "workspace-member"),
        ] {
            assert_eq!(serde_json::to_value(k).unwrap(), want);
        }
    }
}
