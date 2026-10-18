//! Extract port — pull symbols/edges/structure out of source trees.
//! Impls: `kuda-manifest` (manifests -> package graph, no parser),
//! `kuda-treesitter` (grammar+query data), `kuda-lsp` (server-backed).
//! New extractor = new adapter crate; new language = data file.

use kuda_core::{DepEdge, LangDef, Symbol};
use std::path::{Path, PathBuf};

/// What one extractor pass found under `root`.
#[derive(Debug, Default)]
pub struct Extraction {
    pub symbols: Vec<Symbol>,
    pub edges: Vec<DepEdge>,
    /// Files the extraction read — feeds `Provenance.files`.
    pub files: Vec<PathBuf>,
}

/// Mechanical extraction only — report what the grammar/manifest
/// actually says, never inferred structure.
pub trait Extractor: Send + Sync {
    /// Kind string matched against `extract.kinds` in kuda.toml.
    fn kind(&self) -> &'static str;
    /// Extract under `root` for the given language descriptors.
    fn extract(&self, root: &Path, langs: &[LangDef]) -> Result<Extraction, String>;
    /// Extract only the listed files (repo-relative). Default
    /// ignores the filter — adapters without per-file control are
    /// always cheap or server-backed anyway.
    fn extract_filtered(
        &self,
        root: &Path,
        langs: &[LangDef],
        _files: &std::collections::HashSet<PathBuf>,
    ) -> Result<Extraction, String> {
        self.extract(root, langs)
    }
}
