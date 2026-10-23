//! Card render/parse round-trips — the committed .kuda artifacts.

use kuda_cards::*;
use kuda_core::*;
use std::path::PathBuf;

fn prov() -> Provenance {
    Provenance {
        files: vec![PathBuf::from("src/lib.rs")],
        hashes: vec!["abc".into()],
        commit_sha: "deadbeef".into(),
        generated_at: "2026-01-01T00:00:00Z".into(),
        generator: "kuda-template/0.1.0".into(),
    }
}

#[test]
fn repo_card_roundtrip() {
    let c = RepoCard {
        name: "kuda".into(),
        root: PathBuf::from("."),
        purpose: "repo memory".into(),
        modules: vec!["core".into(), "cli".into()],
        commands: vec!["cargo test".into()],
        tree_digest: "sha".into(),
        files_total: 42,
        provenance: prov(),
    };
    let y = render_repo_card(&c).unwrap();
    let back = parse_repo_card(&y).unwrap();
    assert_eq!(back.name, "kuda");
    assert_eq!(back.modules.len(), 2);
    assert_eq!(back.files_total, 42);
}

#[test]
fn module_card_roundtrip_with_omitted() {
    let c = ModuleCard {
        name: "core".into(),
        path: PathBuf::from("crates/core"),
        purpose: "domain".into(),
        api: vec![Symbol {
            name: "Symbol".into(),
            kind: SymbolKind::Type,
            file: PathBuf::from("src/lib.rs"),
            line: 35,
            module: "core".into(),
        }],
        api_omitted: 7,
        deps: vec!["serde".into()],
        notes: "human notes survive".into(),
        verify: vec![PathBuf::from("src/lib.rs")],
        provenance: prov(),
    };
    let y = render_module_card(&c).unwrap();
    let back = parse_module_card(&y).unwrap();
    assert_eq!(back.api_omitted, 7);
    assert_eq!(back.notes, "human notes survive");
    assert_eq!(back.api[0].kind, SymbolKind::Type);
}

#[test]
fn file_card_roundtrip_and_bad_yaml() {
    let c = FileCard {
        path: PathBuf::from("src/x.rs"),
        module: "m".into(),
        purpose: "does x".into(),
        symbols: vec!["f".into()],
        provenance: prov(),
    };
    let back = parse_file_card(&render_file_card(&c).unwrap()).unwrap();
    assert_eq!(back.symbols, vec!["f"]);
    assert!(parse_repo_card(":::").is_err());
}
