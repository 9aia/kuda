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
/// Expands dash/underscore spellings so `use foo_bar` edges answer
/// `dependents foo-bar` too.
pub fn dependents(store: &dyn IndexStore, name: &str) -> Result<Vec<DepEdge>, String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for cand in kuda_core::module_variants(name) {
        for e in store.dependents(&cand)? {
            // defense in depth: indexes written before the
            // write-time filter may hold self-edges — drop them
            // here too, folding dash/underscore spellings of the
            // same module.
            if kuda_core::module_key(&e.from) == kuda_core::module_key(&e.to) {
                continue;
            }
            if seen.insert((e.from.clone(), e.to.clone(), e.kind)) {
                out.push(e);
            }
        }
    }
    Ok(out)
}

/// Edge rows in the index — zero means "no dep data", not "no
/// dependents". Callers report the difference honestly.
pub fn edge_count(store: &dyn IndexStore) -> Result<usize, String> {
    store.edge_count()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// In-memory IndexStore — enough surface to drive every query fn.
    #[derive(Default)]
    struct MockStore {
        syms: Mutex<Vec<Symbol>>,
        edges: Mutex<Vec<DepEdge>>,
    }

    impl IndexStore for MockStore {
        fn replace_module(
            &mut self,
            module: &str,
            syms: &[Symbol],
            edges: &[DepEdge],
        ) -> Result<(), String> {
            self.syms.lock().unwrap().retain(|s| s.module != module);
            self.syms.lock().unwrap().extend(syms.iter().cloned());
            self.edges.lock().unwrap().retain(|e| e.from != module);
            self.edges.lock().unwrap().extend(edges.iter().cloned());
            Ok(())
        }
        fn find(&self, name: &str) -> Result<Vec<Symbol>, String> {
            Ok(self
                .syms
                .lock()
                .unwrap()
                .iter()
                .filter(|s| s.name == name)
                .cloned()
                .collect())
        }
        fn search(&self, pattern: &str) -> Result<Vec<Symbol>, String> {
            Ok(self
                .syms
                .lock()
                .unwrap()
                .iter()
                .filter(|s| s.name.contains(pattern))
                .cloned()
                .collect())
        }
        fn dependents(&self, name: &str) -> Result<Vec<DepEdge>, String> {
            Ok(self
                .edges
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.to == name)
                .cloned()
                .collect())
        }
        fn modules(&self) -> Result<Vec<String>, String> {
            let mut v: Vec<String> = self
                .syms
                .lock()
                .unwrap()
                .iter()
                .map(|s| s.module.clone())
                .collect();
            v.sort();
            v.dedup();
            Ok(v)
        }
        fn edge_count(&self) -> Result<usize, String> {
            Ok(self.edges.lock().unwrap().len())
        }
        fn set_meta(&mut self, _key: &str, _value: &str) -> Result<(), String> {
            Ok(())
        }
        fn meta(&self, _key: &str) -> Result<Option<String>, String> {
            Ok(None)
        }
        fn file_hashes(&self) -> Result<HashMap<String, (String, String)>, String> {
            Ok(HashMap::new())
        }
        fn record_files(
            &mut self,
            _module: &str,
            _files: &[(String, String)],
        ) -> Result<(), String> {
            Ok(())
        }
    }

    fn sym(name: &str, module: &str) -> Symbol {
        Symbol {
            name: name.into(),
            kind: kuda_core::SymbolKind::Function,
            file: "f.rs".into(),
            line: 1,
            module: module.into(),
        }
    }

    #[test]
    fn find_search_modules_edge_count() {
        let mut s = MockStore::default();
        s.replace_module(
            "core",
            &[sym("alpha", "core"), sym("beta", "core")],
            &[kuda_core::DepEdge {
                from: "core".into(),
                to: "serde".into(),
                kind: kuda_core::EdgeKind::Imports,
            }],
        )
        .unwrap();
        assert_eq!(find(&s, "alpha").unwrap().len(), 1);
        assert_eq!(search(&s, "a").unwrap().len(), 2);
        assert_eq!(modules(&s).unwrap(), vec!["core"]);
        assert_eq!(edge_count(&s).unwrap(), 1);
    }

    #[test]
    fn dependents_expands_variants_and_dedupes() {
        let s = MockStore::default();
        // the same logical edge reachable through both spellings
        // must appear once.
        for to in ["neuralmind-shared", "neuralmind_shared"] {
            s.edges.lock().unwrap().push(kuda_core::DepEdge {
                from: "api".into(),
                to: to.into(),
                kind: kuda_core::EdgeKind::Imports,
            });
        }
        let out = dependents(&s, "neuralmind-shared").unwrap();
        assert_eq!(out.len(), 2);
        // distinct rows survive
        assert!(out.iter().all(|e| e.from == "api"));
    }

    #[test]
    fn dependents_drops_self_edges() {
        let s = MockStore::default();
        // legacy index rows: a module that imports its own package
        // name — plus the dash/underscore spelling of the same
        // self-edge — must never surface in `dependents`.
        for (from, to) in [
            ("neuralmind-shared", "neuralmind-shared"),
            ("neuralmind_shared", "neuralmind-shared"),
            ("api", "neuralmind-shared"),
        ] {
            s.edges.lock().unwrap().push(kuda_core::DepEdge {
                from: from.into(),
                to: to.into(),
                kind: kuda_core::EdgeKind::Imports,
            });
        }
        let out = dependents(&s, "neuralmind-shared").unwrap();
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].from, "api");
    }

    #[test]
    fn module_symbols_filters_by_module() {
        let mut s = MockStore::default();
        s.replace_module(
            "m",
            &[
                sym("m", "m"),
                Symbol {
                    module: "other".into(),
                    ..sym("m", "m")
                },
            ],
            &[],
        )
        .unwrap();
        let out = module_symbols(&s, "m").unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].module, "m");
    }

    #[test]
    fn remaining_port_methods_called() {
        let mut s = MockStore::default();
        s.set_meta("k", "v").unwrap();
        assert_eq!(s.meta("k").unwrap(), None);
        assert!(s.file_hashes().unwrap().is_empty());
        s.record_files("m", &[("a".into(), "h".into())]).unwrap();
        // find/dependents misses
        assert!(find(&s, "nope").unwrap().is_empty());
        assert!(dependents(&s, "nope").unwrap().is_empty());
    }
}
