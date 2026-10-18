//! Query engine — the agent-facing reads over `IndexStore` plus
//! the committed cards. Thin: filtering/formatting lives here,
//! storage lives behind the port.

use kuda_core::{DepEdge, Symbol};
use kuda_store::IndexStore;

/// `kuda find <name>` — exact symbol lookup.
pub fn find(store: &dyn IndexStore, name: &str) -> Result<Vec<Symbol>, String> {
    store.find(name)
}

/// Substring symbol search — the fuzzy tier under `find`.
pub fn search(store: &dyn IndexStore, pattern: &str) -> Result<Vec<Symbol>, String> {
    store.search(pattern)
}

/// `kuda dependents <name>` — who points at it (impact analysis).
pub fn dependents(store: &dyn IndexStore, name: &str) -> Result<Vec<DepEdge>, String> {
    store.dependents(name)
}

/// `kuda card <module>` — the symbols a module owns.
pub fn module_symbols(store: &dyn IndexStore, module: &str) -> Result<Vec<Symbol>, String> {
    Ok(store
        .find(module)?
        .into_iter()
        .chain(vec![])
        .filter(|s| s.module == module)
        .collect())
}

/// `kuda status` support — list of indexed modules.
pub fn modules(store: &dyn IndexStore) -> Result<Vec<String>, String> {
    store.modules()
}
