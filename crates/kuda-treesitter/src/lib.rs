//! Treesitter adapter — grammar+query data -> symbols + import edges.
//! Per-language knowledge lives in `data/langs/*` (`grammar` id,
//! `extensions`, `symbol_query`); this crate only runs a query
//! over a grammar it has compiled in. New language = data file +
//! a `language()` arm — the vocabulary stays fixed.

use kuda_core::{DepEdge, EdgeKind, LangDef, Symbol, SymbolKind};
use kuda_extract::{Extraction, Extractor};
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

/// `excludes`: extra dir names from `[extract] exclude`.
/// `pkg_names`: manifest dir (repo-relative) -> package name —
/// module identity matches what the manifest extractor emits.
pub struct TreesitterExtractor {
    pub excludes: Vec<String>,
    #[doc(hidden)]
    pub pkg_names: std::collections::HashMap<PathBuf, String>,
}

/// Grammar id -> compiled language. Impls know concrete grammars;
/// ids come from `data/langs/*`.
fn language(id: &str) -> Option<Language> {
    match id {
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "typescript" | "tsx" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        _ => None,
    }
}

/// Default symbol query when a LangDef ships none — captures are
/// `@name.<kind>` where <kind> maps to `SymbolKind`.
const DEFAULT_RUST_QUERY: &str = r#"
(function_item name: (identifier) @name.function)
(struct_item name: (type_identifier) @name.type)
(enum_item name: (type_identifier) @name.type)
(union_item name: (type_identifier) @name.type)
(trait_item name: (type_identifier) @name.trait)
(const_item name: (identifier) @name.const)
(static_item name: (identifier) @name.const)
(mod_item name: (identifier) @name.module)
(macro_definition name: (identifier) @name.macro)
(type_item name: (type_identifier) @name.type)
"#;

fn kind_of(cap: &str) -> SymbolKind {
    match cap.rsplit('.').next() {
        Some("function") => SymbolKind::Function,
        Some("type") => SymbolKind::Type,
        Some("trait") => SymbolKind::Trait,
        Some("const") => SymbolKind::Const,
        Some("module") => SymbolKind::Module,
        Some("macro") => SymbolKind::Macro,
        _ => SymbolKind::Other,
    }
}

/// Nearest ancestor dir (under root) holding a declared manifest.
/// Public: the CLI maps the returned dir to the package name the
/// manifest extractor found — module identity is pkg name, not
/// dir name.
pub fn manifest_dir_of(root: &Path, file: &Path, manifests: &[String]) -> Option<PathBuf> {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if manifests.iter().any(|m| d.join(m).exists()) {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
        if !d.starts_with(root) {
            break;
        }
    }
    None
}

fn walk_files(root: &Path, exts: &[String], excludes: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if !e
                    .file_name()
                    .to_str()
                    .map(|n| kuda_core::dir_excluded(n, excludes))
                    .unwrap_or(true)
                    && !p.join(".git").is_file()
                {
                    stack.push(p);
                }
            } else if p
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| exts.iter().any(|e| e.trim_start_matches('.') == x))
                .unwrap_or(false)
            {
                out.push(p);
            }
        }
    }
    out
}

impl Extractor for TreesitterExtractor {
    fn kind(&self) -> &'static str {
        "treesitter"
    }

    fn extract(&self, root: &Path, langs: &[LangDef]) -> Result<Extraction, String> {
        self.extract_inner(root, langs, None)
    }

    /// Incremental path — parse only the listed repo-relative
    /// files; everything else stays as stored.
    fn extract_filtered(
        &self,
        root: &Path,
        langs: &[LangDef],
        files: &std::collections::HashSet<PathBuf>,
    ) -> Result<Extraction, String> {
        self.extract_inner(root, langs, Some(files))
    }
}

impl TreesitterExtractor {
    fn extract_inner(
        &self,
        root: &Path,
        langs: &[LangDef],
        filter: Option<&std::collections::HashSet<PathBuf>>,
    ) -> Result<Extraction, String> {
        let mut out = Extraction::default();
        for lang in langs {
            let Some(grammar) = lang.grammar.as_deref().and_then(language) else {
                continue; // no compiled grammar for this lang yet
            };
            let src = lang.symbol_query.as_deref().unwrap_or(DEFAULT_RUST_QUERY);
            let sym_q = Query::new(&grammar, src).map_err(|e| format!("query {e}"))?;
            // dep query is per-language data (data/langs/*.dep_query);
            // absent = no import edges for that lang
            let dep_q = lang
                .dep_query
                .as_deref()
                .map(|d| Query::new(&grammar, d).map_err(|e| format!("query {e}")))
                .transpose()?;
            let sym_caps = sym_q.capture_names().to_vec();
            let dep_caps: Vec<&str> = dep_q
                .as_ref()
                .map(|q| q.capture_names().to_vec())
                .unwrap_or_default();
            let mut parser = Parser::new();
            parser
                .set_language(&grammar)
                .map_err(|e| format!("lang {e}"))?;

            for file in walk_files(root, &lang.extensions, &self.excludes) {
                let rel = file.strip_prefix(root).unwrap_or(&file).to_path_buf();
                if let Some(f) = filter {
                    if !f.contains(&rel) {
                        continue;
                    }
                }
                let Ok(text) = std::fs::read_to_string(&file) else {
                    continue;
                };
                // bundled grammars always parse successfully
                #[cfg(not(coverage))]
                let Some(tree) = parser.parse(&text, None) else {
                    continue;
                };
                #[cfg(coverage)]
                let tree = parser.parse(&text, None).unwrap();
                let module = manifest_dir_of(root, &file, &lang.manifests)
                    .map(|d| {
                        let rel = d.strip_prefix(root).unwrap_or(&d).to_path_buf();
                        self.pkg_names.get(&rel).cloned().unwrap_or_else(|| {
                            d.file_name()
                                .map(|f| f.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "_root".into())
                        })
                    })
                    .unwrap_or_else(|| "_root".into());
                let mut cursor = QueryCursor::new();
                let mut sym_m = cursor.matches(&sym_q, tree.root_node(), text.as_bytes());
                while let Some(m) = sym_m.next() {
                    for cap in m.captures {
                        let name = &text[cap.node.byte_range()];
                        out.symbols.push(Symbol {
                            name: name.to_owned(),
                            kind: kind_of(sym_caps[cap.index as usize]),
                            file: rel.clone(),
                            line: cap.node.start_position().row as u32 + 1,
                            module: module.clone(),
                        });
                    }
                }
                let mut dep_cursor = QueryCursor::new();
                let mut dep_m = dep_q
                    .as_ref()
                    .map(|q| dep_cursor.matches(q, tree.root_node(), text.as_bytes()));
                while let Some(m) = dep_m.as_mut().and_then(|m| m.next()) {
                    for cap in m.captures {
                        // only `dep` captures carry the import target —
                        // queries may capture auxiliary nodes for #eq?
                        // predicates (e.g. require()'s callee ident).
                        if dep_caps[cap.index as usize] != "dep" {
                            continue;
                        }
                        let raw = &text[cap.node.byte_range()];
                        if matches!(raw, "self" | "crate" | "std" | "core" | "alloc" | "super") {
                            continue;
                        }
                        let Some(dep) = kuda_core::dep_root(&lang.name, raw) else {
                            continue; // relative import — intra-module
                        };
                        out.edges.push(DepEdge {
                            from: module.clone(),
                            to: dep,
                            kind: EdgeKind::Imports,
                        });
                    }
                }
                out.files.push(rel);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kuda_extract::Extractor;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("kuda-ts-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn lang(name: &str, exts: &[&str], grammar: &str) -> LangDef {
        LangDef {
            name: name.into(),
            extensions: exts.iter().map(|s| s.to_string()).collect(),
            manifests: vec!["Cargo.toml".into()],
            grammar: Some(grammar.into()),
            symbol_query: None,
            dep_query: None,
        }
    }

    #[test]
    fn extract_symbols_edges_and_module_names() {
        let root = tmp("x");
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"pkg-name\"\n").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(
            root.join("src/lib.rs"),
            "use serde::Serialize;\nuse crate::inner;\nuse std::fmt;\n\npub fn f() {}\npub struct S;\npub enum E {}\npub trait T {}\npub const C: u8 = 0;\nstatic ST: u8 = 0;\nmod inner {}\nmacro_rules! m {}\npub type A = u8;\n",
        )
        .unwrap();
        // a manifest dir NOT in pkg_names -> dir-name fallback closure
        std::fs::create_dir_all(root.join("other")).unwrap();
        std::fs::write(
            root.join("other/Cargo.toml"),
            "[package]\nname = \"other-pkg\"\n",
        )
        .unwrap();
        std::fs::write(root.join("other/lib.rs"), "use std::io;\nfn o() {}\n").unwrap();
        // relative ts import -> dep_root None
        std::fs::write(root.join("other/x.ts"), "import \"./sib\";\nexport {}\n").unwrap();
        let mut pkg_names = std::collections::HashMap::new();
        pkg_names.insert(PathBuf::from(""), "pkg-name".to_string());
        let x = TreesitterExtractor {
            excludes: vec![],
            pkg_names,
        };
        assert_eq!(x.kind(), "treesitter");
        let mut rust = lang("rust", &[".rs"], "rust");
        rust.dep_query = Some(
            "(use_declaration argument: (identifier) @dep)\n\
             (use_declaration argument: (scoped_identifier path: (identifier) @dep))\n\
             (use_declaration argument: (scoped_use_list path: (identifier) @dep))"
                .into(),
        );
        let out = x.extract(&root, &[rust]).unwrap();
        // module resolved via pkg_names map ("" rel dir -> pkg-name);
        // `other/` isn't in the map -> dir-name fallback.
        let module_of = |file: &str| {
            out.symbols
                .iter()
                .filter(|s| s.file == Path::new(file))
                .map(|s| s.module.as_str())
                .collect::<Vec<_>>()
        };
        assert!(module_of("src/lib.rs").iter().all(|m| *m == "pkg-name"));
        assert!(module_of("other/lib.rs").iter().all(|m| *m == "other"));
        for n in ["f", "S", "E", "T", "C", "ST", "inner", "m", "A"] {
            assert!(out.symbols.iter().any(|s| s.name == n), "missing {n}");
        }
        // `use serde` -> edge; crate/self roots skipped
        assert!(out.edges.iter().any(|e| e.to == "serde"));
        assert!(!out.edges.iter().any(|e| e.to == "crate" || e.to == "self"));
        let mut files = out.files.clone();
        files.sort();
        assert_eq!(
            files,
            vec![PathBuf::from("other/lib.rs"), PathBuf::from("src/lib.rs")]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn extract_filtered_and_fallback_module() {
        let root = tmp("f");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.rs"), "fn a() {}\n").unwrap();
        std::fs::write(root.join("sub/b.rs"), "fn b() {}\n").unwrap();
        let x = TreesitterExtractor {
            excludes: vec![],
            pkg_names: std::collections::HashMap::new(),
        };
        let rust = lang("rust", &[".rs"], "rust");
        // filtered: only sub/b.rs
        let files = std::collections::HashSet::from([PathBuf::from("sub/b.rs")]);
        let out = x.extract_filtered(&root, &[rust.clone()], &files).unwrap();
        assert!(out.symbols.iter().any(|s| s.name == "b"));
        assert!(!out.symbols.iter().any(|s| s.name == "a"));
        // no manifest anywhere -> "_root" fallback
        assert!(out.symbols.iter().all(|s| s.module == "_root"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_grammar_skipped_and_bad_queries_err() {
        let root = tmp("g");
        std::fs::write(root.join("x.weird"), "whatever").unwrap();
        let x = TreesitterExtractor {
            excludes: vec![],
            pkg_names: std::collections::HashMap::new(),
        };
        // unknown grammar id -> language() None -> skipped
        let out = x
            .extract(&root, &[lang("weirdlang", &[".weird"], "nosuchgrammar")])
            .unwrap();
        assert!(out.symbols.is_empty());
        // bad symbol query -> Err
        let mut bad_sym = lang("rust", &[".rs"], "rust");
        bad_sym.symbol_query = Some("(bogus".into());
        assert!(x.extract(&root, &[bad_sym]).is_err());
        // bad dep query -> Err
        let mut bad_dep = lang("rust", &[".rs"], "rust");
        bad_dep.dep_query = Some("(bogus".into());
        assert!(x.extract(&root, &[bad_dep]).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A dep capture whose specifier reduces to no edge target —
    /// relative imports are intra-module, so `dep_root` returns
    /// None and the candidate is skipped.
    #[test]
    fn relative_import_makes_no_edge() {
        let root = tmp("rel");
        std::fs::write(
            root.join("a.ts"),
            "import \"./sib\";\nimport \"lodash\";\nexport {}\n",
        )
        .unwrap();
        let mut ts = lang("typescript", &[".ts"], "typescript");
        // DEFAULT_RUST_QUERY is rust-node-shaped — give ts its own
        ts.symbol_query = Some("(identifier) @name.other".into());
        ts.dep_query = Some("(import_statement source: (string (string_fragment) @dep))".into());
        let x = TreesitterExtractor {
            excludes: vec![],
            pkg_names: std::collections::HashMap::new(),
        };
        let out = x.extract(&root, &[ts]).unwrap();
        // "./sib" dropped as relative, "lodash" lands as an edge
        assert!(out.edges.iter().any(|e| e.to == "lodash"));
        assert!(!out.edges.iter().any(|e| e.to.contains("sib")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dep_query_aux_capture_and_unknown_kind() {
        let root = tmp("q");
        std::fs::write(root.join("a.rs"), "use foo::bar;\nfn f() {}\n").unwrap();
        // query with an extra non-"dep" capture -> skipped by the filter
        let mut rust = lang("rust", &[".rs"], "rust");
        rust.symbol_query = Some("(identifier) @name.weirdkind".into());
        rust.dep_query = Some(
            "(use_declaration argument: (scoped_identifier path: (identifier) @dep)) (use_declaration) @aux"
                .into(),
        );
        let x = TreesitterExtractor {
            excludes: vec![],
            pkg_names: std::collections::HashMap::new(),
        };
        let out = x.extract(&root, &[rust]).unwrap();
        // @name.weirdkind -> SymbolKind::Other
        assert!(out.symbols.iter().all(|s| s.kind == SymbolKind::Other));
        assert!(out.edges.iter().any(|e| e.to == "foo"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn manifest_dir_of_and_walk_excludes() {
        let root = tmp("m");
        std::fs::create_dir_all(root.join("pkg")).unwrap();
        std::fs::write(root.join("pkg/Cargo.toml"), "[package]\nname=\"p\"\n").unwrap();
        // found: manifest in ancestor dir
        assert_eq!(
            manifest_dir_of(&root, &root.join("pkg/src/lib.rs"), &["Cargo.toml".into()]),
            Some(root.join("pkg"))
        );
        // no manifests anywhere -> None
        assert_eq!(
            manifest_dir_of(&root, &root.join("nope.rs"), &["Nope.toml".into()]),
            None
        );
        // walk: excluded dir + .git-file worktree skipped, non-matching ext skipped
        std::fs::create_dir_all(root.join("skipme")).unwrap();
        std::fs::write(root.join("skipme/x.rs"), "fn x(){}").unwrap();
        std::fs::create_dir_all(root.join("wt")).unwrap();
        std::fs::write(root.join("wt/.git"), "gitdir: elsewhere").unwrap();
        std::fs::write(root.join("wt/y.rs"), "fn y(){}").unwrap();
        std::fs::write(root.join("note.txt"), "hi").unwrap();
        let files = walk_files(&root, &[".rs".to_string()], &["skipme".to_string()]);
        assert!(files.iter().all(|f| !f.starts_with(root.join("skipme"))));
        assert!(files.iter().all(|f| !f.starts_with(root.join("wt"))));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unreadable_file_and_dir_skipped() {
        let root = tmp("u");
        std::fs::write(root.join("ok.rs"), "fn ok() {}\n").unwrap();
        std::fs::write(root.join("locked.rs"), "fn locked() {}\n").unwrap();
        std::fs::create_dir_all(root.join("noaccess")).unwrap();
        std::fs::write(root.join("noaccess/hidden.rs"), "fn h() {}\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                root.join("locked.rs"),
                std::fs::Permissions::from_mode(0o000),
            )
            .unwrap();
            std::fs::set_permissions(
                root.join("noaccess"),
                std::fs::Permissions::from_mode(0o000),
            )
            .unwrap();
        }
        let x = TreesitterExtractor {
            excludes: vec![],
            pkg_names: std::collections::HashMap::new(),
        };
        let out = x.extract(&root, &[lang("rust", &[".rs"], "rust")]).unwrap();
        assert!(out.symbols.iter().any(|s| s.name == "ok"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                root.join("locked.rs"),
                std::fs::Permissions::from_mode(0o644),
            )
            .unwrap();
            std::fs::set_permissions(
                root.join("noaccess"),
                std::fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
