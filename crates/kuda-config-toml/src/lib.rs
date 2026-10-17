//! TOML adapter of `ConfigSource` — explicit path → $KUDA_CONFIG →
//! ./kuda.toml → ~/.config/kuda/kuda.toml.

use kuda_config::ConfigSource;
use std::path::PathBuf;

pub struct TomlSource {
    /// Explicit `--config` path, or None for the default search.
    pub path: Option<String>,
}

impl ConfigSource for TomlSource {
    fn load(&self) -> Result<kuda_core::Config, String> {
        let candidates: Vec<PathBuf> = match self
            .path
            .clone()
            .or_else(|| std::env::var("KUDA_CONFIG").ok())
        {
            Some(p) => vec![PathBuf::from(p)],
            None => vec![
                PathBuf::from("kuda.toml"),
                std::env::var("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| ".".into())
                    .join(".config/kuda/kuda.toml"),
            ],
        };
        for p in candidates {
            if p.exists() {
                let text = std::fs::read_to_string(&p)
                    .map_err(|e| format!("read {}: {e}", p.display()))?;
                return toml::from_str(&text).map_err(|e| format!("parse {}: {e}", p.display()));
            }
        }
        Err("no kuda.toml found (search: $KUDA_CONFIG, ., ~/.config/kuda/)".into())
    }
}
