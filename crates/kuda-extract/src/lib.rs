//! Extract port — pull symbols/edges/structure out of source trees.
//! Impls: `kuda-manifest` (manifests -> package graph, no parser),
//! `kuda-treesitter` (grammar+query data), `kuda-lsp` (server-backed),
//! `kuda-records` (domain-corpus descriptors -> record ids + refs).
//! New extractor = new adapter crate; new language/domain = data file.

use kuda_core::{DepEdge, LangDef, Symbol};
use std::path::{Path, PathBuf};

/// What one extractor pass found under `root`.
#[derive(Debug, Default)]
pub struct Extraction {
    pub symbols: Vec<Symbol>,
    pub edges: Vec<DepEdge>,
    /// Files the extraction read — feeds `Provenance.files`.
    pub files: Vec<PathBuf>,
    /// Extraction warnings — invalid records, parse anomalies.
    /// Land on the owning module's card; empty = clean corpus.
    pub warnings: Vec<ExtractWarning>,
}

/// A non-fatal extraction finding, tied to the file that caused it.
#[derive(Debug)]
pub struct ExtractWarning {
    pub file: PathBuf,
    pub message: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub;
    impl Extractor for Stub {
        fn kind(&self) -> &'static str {
            "stub"
        }
        fn extract(&self, _root: &Path, _langs: &[LangDef]) -> Result<Extraction, String> {
            let mut e = Extraction::default();
            e.symbols.push(Symbol {
                name: "s".into(),
                kind: kuda_core::SymbolKind::Function,
                file: "f".into(),
                line: 1,
                module: "m".into(),
            });
            Ok(e)
        }
    }

    #[test]
    fn default_filtered_delegates_to_extract() {
        let x = Stub;
        assert_eq!(x.kind(), "stub");
        let files = std::collections::HashSet::from([PathBuf::from("f")]);
        let e = x.extract_filtered(Path::new("."), &[], &files).unwrap();
        assert_eq!(e.symbols.len(), 1);
    }
}
