//! Summarize port — Extraction -> ModuleCard.
//! Impls: `kuda-template` (deterministic, the default), `kuda-llm`
//! (prose, provider-pluggable). Cards must end in `verify:` file
//! lists — memory is a hypothesis, not truth.

use kuda_core::{ModuleCard, Provenance};
use kuda_extract::Extraction;
use std::path::Path;

pub trait Summarizer: Send + Sync {
    /// Kind string matched against `summarize.kind` in kuda.toml.
    fn kind(&self) -> &'static str;
    /// Build one module's card from what extraction found. `prov`
    /// is composed by the caller (it owns the sha/hash bookkeeping).
    fn summarize(
        &self,
        module: &str,
        path: &Path,
        ext: &Extraction,
        prov: Provenance,
    ) -> Result<ModuleCard, String>;
}
