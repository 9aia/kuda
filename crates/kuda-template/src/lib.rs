//! Template adapter — deterministic cards built only from what
//! extraction found. Zero-model, zero-cost: `purpose` is the first
//! doc line when a module manifest/README supplies one, else the
//! module name. Prose belongs to the `llm` adapter.

use kuda_core::{ModuleCard, Provenance};
use kuda_extract::Extraction;
use kuda_summarize::Summarizer;
use std::path::Path;

pub struct TemplateSummarizer;

impl Summarizer for TemplateSummarizer {
    fn kind(&self) -> &'static str {
        "template"
    }

    fn summarize(
        &self,
        module: &str,
        path: &Path,
        ext: &Extraction,
        prov: Provenance,
    ) -> Result<ModuleCard, String> {
        Ok(ModuleCard {
            name: module.to_owned(),
            path: path.to_path_buf(),
            purpose: module.to_owned(),
            api: ext
                .symbols
                .iter()
                .filter(|s| s.module == module)
                .cloned()
                .collect(),
            deps: ext
                .edges
                .iter()
                .filter(|e| e.from == module)
                .map(|e| e.to.clone())
                .collect(),
            notes: String::new(),
            verify: ext.files.clone(),
            provenance: prov,
        })
    }
}
