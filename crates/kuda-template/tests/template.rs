//! TemplateSummarizer: deterministic cards from extraction only.

use kuda_core::{DepEdge, EdgeKind, Provenance, Symbol, SymbolKind};
use kuda_extract::Extraction;
use kuda_summarize::Summarizer;
use kuda_template::TemplateSummarizer;
use std::path::Path;

fn prov() -> Provenance {
    Provenance {
        files: vec![],
        hashes: vec![],
        commit_sha: "s".into(),
        generated_at: "t".into(),
        generator: "g".into(),
    }
}

fn sym(name: &str, module: &str) -> Symbol {
    Symbol {
        name: name.into(),
        kind: SymbolKind::Function,
        file: "f.rs".into(),
        line: 1,
        module: module.into(),
    }
}

#[test]
fn card_collects_only_own_module() {
    let ext = Extraction {
        symbols: vec![sym("in", "m"), sym("out", "other")],
        edges: vec![
            DepEdge { from: "m".into(), to: "dep".into(), kind: EdgeKind::Imports },
            DepEdge { from: "other".into(), to: "x".into(), kind: EdgeKind::Calls },
        ],
        files: vec!["f.rs".into()],
    };
    let card = TemplateSummarizer
        .summarize("m", Path::new("m/"), &ext, prov())
        .unwrap();
    assert_eq!(card.name, "m");
    assert_eq!(card.purpose, "m");
    assert_eq!(card.api.len(), 1);
    assert_eq!(card.api[0].name, "in");
    assert_eq!(card.deps, vec!["dep"]);
    assert_eq!(card.verify, vec![std::path::PathBuf::from("f.rs")]);
    assert_eq!(TemplateSummarizer.kind(), "template");
}
