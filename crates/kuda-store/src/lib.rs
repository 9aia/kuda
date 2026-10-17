//! Store port — L2 index persistence. Impls: `kuda-store-sqlite`;
//! redb/remote/embedding = new impl crates. The store is
//! rebuildable — never the source of truth.

use kuda_core::{DepEdge, Symbol};

/// Symbol/edge persistence for the query layer. Writes are keyed
/// by module so incremental sync replaces one module at a time.
pub trait IndexStore: Send + Sync {
    /// Replace a module's symbols and its outgoing edges.
    fn replace_module(
        &mut self,
        module: &str,
        syms: &[Symbol],
        edges: &[DepEdge],
    ) -> Result<(), String>;
    /// Exact-name lookup — `kuda find`.
    fn find(&self, name: &str) -> Result<Vec<Symbol>, String>;
    /// Reverse lookup — edges pointing at `name`. Impact analysis.
    fn dependents(&self, name: &str) -> Result<Vec<DepEdge>, String>;
    /// Module names currently indexed.
    fn modules(&self) -> Result<Vec<String>, String>;
    /// Provenance bookkeeping ("commit_sha", "extractor_versions").
    fn set_meta(&mut self, key: &str, value: &str) -> Result<(), String>;
    fn meta(&self, key: &str) -> Result<Option<String>, String>;
}
