//! SqliteStore against a temp db: replace/find/search/dependents/
//! meta/file-hashes — the IndexStore contract end to end.

use kuda_core::{DepEdge, EdgeKind, Symbol, SymbolKind};
use kuda_store::IndexStore;
use kuda_store_sqlite::SqliteStore;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("kuda-store-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn sym(name: &str, module: &str) -> Symbol {
    Symbol {
        name: name.into(),
        kind: SymbolKind::Function,
        file: format!("{module}/f.rs").into(),
        line: 1,
        module: module.into(),
    }
}

#[test]
fn replace_find_search() {
    let mut s = SqliteStore::open(&scratch("s").join("i.db")).unwrap();
    s.replace_module(
        "core",
        &[sym("alpha", "core"), sym("beta", "core")],
        &[DepEdge { from: "core".into(), to: "serde".into(), kind: EdgeKind::PackageDep }],
    )
    .unwrap();
    s.replace_module("cli", &[sym("gamma", "cli")], &[]).unwrap();

    assert_eq!(s.find("alpha").unwrap().len(), 1);
    assert_eq!(s.search("a").unwrap().len(), 3);
    assert_eq!(s.search("zzz").unwrap().len(), 0);
    let mut mods = s.modules().unwrap();
    mods.sort();
    assert_eq!(mods, vec!["cli", "core"]);
    assert_eq!(s.edge_count().unwrap(), 1);
    assert_eq!(s.dependents("serde").unwrap().len(), 1);
    assert_eq!(s.dependents("none").unwrap().len(), 0);
}

#[test]
fn replace_module_replaces() {
    let mut s = SqliteStore::open(&scratch("r").join("i.db")).unwrap();
    s.replace_module("m", &[sym("old", "m")], &[]).unwrap();
    s.replace_module("m", &[sym("new", "m")], &[]).unwrap();
    assert_eq!(s.find("old").unwrap().len(), 0);
    assert_eq!(s.find("new").unwrap().len(), 1);
}

#[test]
fn meta_and_file_hashes() {
    let mut s = SqliteStore::open(&scratch("m").join("i.db")).unwrap();
    assert_eq!(s.meta("k").unwrap(), None);
    s.set_meta("k", "v").unwrap();
    s.set_meta("k", "v2").unwrap();
    assert_eq!(s.meta("k").unwrap().as_deref(), Some("v2"));

    s.record_files("m", &[("a.rs".into(), "h1".into()), ("b.rs".into(), "h2".into())])
        .unwrap();
    let h = s.file_hashes().unwrap();
    assert_eq!(h["a.rs"], ("h1".into(), "m".into()));
    // Re-record replaces the module's set.
    s.record_files("m", &[("b.rs".into(), "h2b".into())]).unwrap();
    let h = s.file_hashes().unwrap();
    assert!(!h.contains_key("a.rs"));
    assert_eq!(h["b.rs"].0, "h2b");
}
