//! Card render — domain types <-> committed `.kuda/*.yaml`
//! artifacts. Machine fields and the human-owned `notes:` region
//! round-trip through these functions.

use kuda_core::{FileCard, ModuleCard, RepoCard};

/// `.kuda/index.yaml` — the L0 always-load artifact.
pub fn render_repo_card(card: &RepoCard) -> Result<String, String> {
    serde_yaml::to_string(card).map_err(|e| e.to_string())
}

pub fn parse_repo_card(text: &str) -> Result<RepoCard, String> {
    serde_yaml::from_str(text).map_err(|e| e.to_string())
}

/// `.kuda/modules/<name>.yaml` — L1 card artifacts.
pub fn render_module_card(card: &ModuleCard) -> Result<String, String> {
    serde_yaml::to_string(card).map_err(|e| e.to_string())
}

pub fn parse_module_card(text: &str) -> Result<ModuleCard, String> {
    serde_yaml::from_str(text).map_err(|e| e.to_string())
}

/// `.kuda/files/<module>/<file>.yaml` — L1.5 per-file cards.
pub fn render_file_card(card: &FileCard) -> Result<String, String> {
    serde_yaml::to_string(card).map_err(|e| e.to_string())
}

pub fn parse_file_card(text: &str) -> Result<FileCard, String> {
    serde_yaml::from_str(text).map_err(|e| e.to_string())
}
