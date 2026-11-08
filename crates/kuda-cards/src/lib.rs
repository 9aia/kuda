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

/// `.kuda/files/<module>/<file>.yaml` — L1 per-file cards.
pub fn render_file_card(card: &FileCard) -> Result<String, String> {
    serde_yaml::to_string(card).map_err(|e| e.to_string())
}

pub fn parse_file_card(text: &str) -> Result<FileCard, String> {
    serde_yaml::from_str(text).map_err(|e| e.to_string())
}

/// `notes:` is the human-owned region of a module card (T-008):
/// regeneration rewrites the machine fields and carries the
/// previously committed card's notes forward verbatim. Freshly
/// generated notes land only when the committed region was empty —
/// agent-suggested bullets must never overwrite human prose, and
/// stale machine notes must not resurrect once written.
pub fn carry_notes(card: &mut ModuleCard, prev: &ModuleCard) {
    if !prev.notes.trim().is_empty() {
        card.notes = prev.notes.clone();
    }
}
