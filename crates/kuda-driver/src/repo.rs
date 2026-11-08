//! Repo context — resolve which kuda-indexed repo this request is
//! about, load its config, cards, and L2 store.
//!
//! Resolution order for the repo root:
//! 1. `root` / `dir` / `path` param on the wire request (JSON string).
//! 2. `$KUDA_DRIVER_ROOT` env var.
//! 3. Walk up from the process cwd to the nearest dir containing
//!    `.kuda/` or `kuda.toml` — pavo spawns drivers with the user's
//!    cwd, so `pavo view kuda:graph` inside repo X reads X's index.
//! 4. The cwd itself (queries then report "not indexed" honestly).

use kuda_core::{Config, ModuleCard, RepoCard};
use kuda_store_sqlite::SqliteStore;
use kuda_vcs::Vcs;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// Everything a query needs for one repo — opened lazily per wire
/// request (sqlite open is cheap; cards parse on demand).
pub struct Ctx {
    /// Repo root the context was resolved against.
    pub root: PathBuf,
    /// Parsed `kuda.toml` (defaults when absent/unparseable).
    pub cfg: Config,
    /// `.kuda/index.yaml` L0 card, when present and parseable.
    pub repo_card: Option<RepoCard>,
    /// L2 store — `None` when `.kuda/index.sqlite` doesn't exist
    /// (never created: the driver is read-only).
    pub store: Option<SqliteStore>,
    /// `git rev-parse HEAD` at open time (empty when not a repo).
    pub head: String,
}

impl Ctx {
    /// Open a context for `root`. Never fails hard — missing pieces
    /// surface as `None`/empty so views degrade to error payloads.
    pub fn open(root: PathBuf) -> Self {
        let cfg = load_config(&root);
        let cards_dir = cards_dir(&root, &cfg);
        let repo_card = std::fs::read_to_string(cards_dir.join("index.yaml"))
            .ok()
            .and_then(|t| kuda_cards::parse_repo_card(&t).ok());
        let store_path = root.join(&cfg.store.path);
        let store = store_path
            .is_file()
            .then(|| SqliteStore::open(&store_path).ok())
            .flatten();
        let head = kuda_git::GitVcs.head_sha(&root).unwrap_or_default();
        Self {
            root,
            cfg,
            repo_card,
            store,
            head,
        }
    }

    /// `.kuda/modules/` dir for this context.
    pub fn modules_dir(&self) -> PathBuf {
        cards_dir(&self.root, &self.cfg).join("modules")
    }

    /// All parseable module cards, sorted by name. Unparseable files
    /// are skipped (they surface as drift "error" rows only when the
    /// file itself can't be read as YAML at all — parse failures are
    /// counted separately by callers that care).
    pub fn module_cards(&self) -> Vec<(String, ModuleCard)> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(self.modules_dir()) else {
            return out;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.extension()
                .map(|x| x == "yaml" || x == "yml")
                .unwrap_or(false)
            {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    if let Ok(card) = kuda_cards::parse_module_card(&text) {
                        out.push((card.name.clone(), card));
                    }
                }
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// True when the context has anything kuda-shaped at all.
    pub fn indexed(&self) -> bool {
        self.repo_card.is_some() || self.store.is_some() || self.modules_dir().is_dir()
    }
}

/// Resolve the repo root for one request. `params` are the merged
/// view/query params; string values under `root`/`dir`/`path` win.
pub fn resolve_root(params: &Map<String, Value>) -> PathBuf {
    for key in ["root", "dir", "path"] {
        if let Some(s) = params.get(key).and_then(Value::as_str) {
            let p = PathBuf::from(s);
            return std::fs::canonicalize(&p).unwrap_or(p);
        }
    }
    if let Ok(r) = std::env::var("KUDA_DRIVER_ROOT") {
        if !r.is_empty() {
            let p = PathBuf::from(r);
            return std::fs::canonicalize(&p).unwrap_or(p);
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // Walk up: nearest ancestor with `.kuda/` or `kuda.toml`.
    for dir in cwd.ancestors() {
        if dir.join(".kuda").is_dir() || dir.join("kuda.toml").is_file() {
            return dir.to_path_buf();
        }
    }
    cwd
}

/// Parse `<root>/kuda.toml` into a `Config`; defaults on absence or
/// parse failure (the driver never refuses to serve).
fn load_config(root: &Path) -> Config {
    let p = root.join("kuda.toml");
    let Ok(text) = std::fs::read_to_string(&p) else {
        return Config::default();
    };
    toml::from_str(&text).unwrap_or_else(|e| {
        eprintln!("kuda-driver: parse {}: {e} — defaults in use", p.display());
        Config::default()
    })
}

fn cards_dir(root: &Path, cfg: &Config) -> PathBuf {
    if cfg.cards.dir.is_absolute() {
        cfg.cards.dir.clone()
    } else {
        root.join(&cfg.cards.dir)
    }
}

/// `kuda status` drift for one card — recompute the git-blob hash of
/// every provenance file and compare. Mirrors `status()` in
/// `kuda-cli`: stale on any hash mismatch or a file-count change.
pub fn card_drift(root: &Path, card: &ModuleCard) -> &'static str {
    let hashes: Vec<String> = card
        .provenance
        .files
        .iter()
        .map(|f| blob_hash(&root.join(f)))
        .collect();
    let stale = card
        .provenance
        .hashes
        .iter()
        .zip(&hashes)
        .any(|(a, b)| a != b)
        || card.provenance.files.len() != hashes.len();
    if stale {
        "drifted"
    } else {
        "fresh"
    }
}

/// git blob hash — identical values to `git hash-object`
/// ("blob <len>\0" + bytes), same recipe as `kuda status`.
fn blob_hash(path: &Path) -> String {
    use sha1::Digest;
    let Ok(bytes) = std::fs::read(path) else {
        return String::new();
    };
    let mut h = sha1::Sha1::new();
    h.update(format!("blob {}\0", bytes.len()).as_bytes());
    h.update(&bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Filename-safe module name → card file stem (same rule as
/// `kuda card`: keep alnum, `-`, `_`, `.`; fold the rest to `_`).
pub fn safe(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn safe_folds_unsafe_chars() {
        assert_eq!(safe("kuda-core"), "kuda-core");
        assert_eq!(safe("a/b c"), "a_b_c");
    }

    #[test]
    fn resolve_root_prefers_params() {
        let tmp = std::env::temp_dir().join(format!("kuda-drv-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let mut params = Map::new();
        params.insert("root".into(), json!(tmp.to_string_lossy()));
        let r = resolve_root(&params);
        assert_eq!(r, std::fs::canonicalize(&tmp).unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unindexed_ctx_reports_not_indexed() {
        let tmp = std::env::temp_dir().join(format!("kuda-drv-empty-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let ctx = Ctx::open(tmp.clone());
        assert!(!ctx.indexed());
        assert!(ctx.repo_card.is_none());
        assert!(ctx.store.is_none());
        let q = crate::queries::run_query(&ctx, "status", &Map::new());
        assert!(q.get("error").is_some(), "{q}");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
