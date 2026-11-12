//! Config port + file impl — `kuda.{yaml,yml,toml}` loading through
//! scarab-config's file discovery (P-017): explicit `--config` →
//! `$KUDA_CONFIG` → `./kuda.*` (YAML preferred) →
//! `~/.config/kuda/`. The generic machinery is scarab's
//! (`scarab-config-file`); this crate is only the wiring plus the
//! [`ConfigSource`] port front-ends bind. The retired
//! `kuda-config-toml` crate's hand-rolled search lives here now.

use std::path::{Path, PathBuf};

use scarab_config_file::FileSource as ScarabSource;

/// A config source — returns the loaded `Config`.
pub trait ConfigSource: Send + Sync {
    /// Load + validate the config; Err carries a readable message.
    fn load(&self) -> Result<kuda_core::Config, String>;
}

/// The file-backed source — explicit `--config` path wins over
/// `$KUDA_CONFIG`, then the conventional discovery.
pub struct FileSource {
    /// Explicit `--config` path, or None for the default search.
    pub path: Option<String>,
}

/// The underlying scarab source — `for_app("kuda")` conventions
/// (`$KUDA_CONFIG`, `./`, `~/.config/kuda/`) with the standard
/// YAML-preferred formats.
fn source() -> ScarabSource<kuda_core::Config> {
    ScarabSource::for_app("kuda")
}

impl FileSource {
    fn with_explicit(&self) -> ScarabSource<kuda_core::Config> {
        let mut src = source();
        src.explicit = self.path.clone();
        src
    }

    /// Candidate config paths in search order — for status/hint
    /// views that report which file the chain would read. The
    /// explicit/env path heads the list even when missing (the
    /// loader turns that into an error, not a skip).
    #[must_use]
    pub fn candidates(&self) -> Vec<PathBuf> {
        self.with_explicit()
            .candidates_with(&|k| std::env::var(k).ok())
    }
}

impl ConfigSource for FileSource {
    fn load(&self) -> Result<kuda_core::Config, String> {
        self.with_explicit().load()
    }
}

/// Parse exactly one config file — no search, no discovery
/// (kuda-driver's root-anchored `<root>/kuda.toml` read).
///
/// # Errors
/// Fails on a read error, an unclaimed extension, or a parse error.
pub fn parse_file(path: &Path) -> Result<kuda_core::Config, String> {
    source().load_path(path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kuda-cfg-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const CFG: &str = r#"
[extract]
kinds = ["manifest", "treesitter"]
[summarize]
kind = "template"
[store]
kind = "sqlite"
"#;

    #[test]
    fn explicit_path_loads() {
        let dir = scratch("load");
        let f = dir.join("kuda.toml");
        std::fs::write(&f, CFG).unwrap();
        let c = FileSource {
            path: Some(f.to_string_lossy().into()),
        }
        .load()
        .unwrap();
        assert_eq!(c.summarize.kind, "template");
        assert_eq!(c.extract.kinds[0], "manifest");
        assert_eq!(c.store.kind, "sqlite");
    }

    #[test]
    fn yaml_config_loads() {
        let dir = scratch("yaml");
        let f = dir.join("kuda.yaml");
        std::fs::write(&f, "store:\n  kind: yaml-store\n").unwrap();
        let c = FileSource {
            path: Some(f.to_string_lossy().into()),
        }
        .load()
        .unwrap();
        assert_eq!(c.store.kind, "yaml-store");
    }

    #[test]
    fn parse_error_and_missing() {
        let dir = scratch("bad");
        let f = dir.join("kuda.toml");
        std::fs::write(&f, "not [valid toml").unwrap();
        let e = FileSource {
            path: Some(f.to_string_lossy().into()),
        }
        .load()
        .unwrap_err();
        assert!(e.contains("parse"), "{e}");

        let e = FileSource {
            path: Some("/nonexistent/x".into()),
        }
        .load()
        .unwrap_err();
        assert!(e.contains("config file not found"), "{e}");
    }

    #[test]
    fn env_var_drives_search() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch("env");
        let f = dir.join("env.toml");
        std::fs::write(&f, CFG).unwrap();
        let saved = std::env::var("KUDA_CONFIG").ok();
        std::env::set_var("KUDA_CONFIG", f.to_str().unwrap());
        let c = FileSource { path: None }.load().unwrap();
        match saved {
            Some(v) => std::env::set_var("KUDA_CONFIG", v),
            None => std::env::remove_var("KUDA_CONFIG"),
        }
        assert_eq!(c.store.kind, "sqlite");
    }

    #[test]
    fn dotfile_fallback_and_candidates() {
        let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = scratch("cwd");
        std::fs::write(dir.join("kuda.toml"), CFG).unwrap();
        let saved_cfg = std::env::var("KUDA_CONFIG").ok();
        let saved_home = std::env::var("HOME").ok();
        std::env::remove_var("KUDA_CONFIG");
        std::env::set_var("HOME", "/nonexistent-home");
        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let c = FileSource { path: None }.load().unwrap();
        let cands = FileSource { path: None }.candidates();
        std::env::set_current_dir(cwd).unwrap();
        match saved_cfg {
            Some(v) => std::env::set_var("KUDA_CONFIG", v),
            None => std::env::remove_var("KUDA_CONFIG"),
        }
        match saved_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        assert_eq!(c.store.kind, "sqlite");
        assert!(cands.iter().any(|p| p.ends_with("kuda.toml")));
        assert!(cands.iter().any(|p| p.ends_with("kuda.yaml")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_file_reads_one_path() {
        let dir = scratch("file");
        let f = dir.join("kuda.toml");
        std::fs::write(&f, CFG).unwrap();
        let c = parse_file(&f).unwrap();
        assert_eq!(c.store.kind, "sqlite");
        assert!(parse_file(&dir.join("gone.toml")).is_err());
    }

    /// env/cwd mutation is process-global — serialize the tests
    /// that touch them (plain `cargo test` shares the process;
    /// nextest already isolates per-test).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
