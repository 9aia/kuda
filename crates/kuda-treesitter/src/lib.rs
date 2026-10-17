//! Treesitter impl — grammar+query data -> symbols + import edges.
//! Per-language knowledge lives in `data/langs/*` (`grammar` id,
//! `extensions`, `symbol_query`); this crate only runs a query
//! over a grammar it has compiled in. New language = data file +
//! a `language()` arm — the vocabulary stays fixed.

use kuda_core::{DepEdge, EdgeKind, LangDef, Symbol, SymbolKind};
use kuda_extract::{Extraction, Extractor};
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

pub struct TreesitterExtractor;

/// Grammar id -> compiled language. Impls know concrete grammars;
/// ids come from `data/langs/*`.
fn language(id: &str) -> Option<Language> {
    match id {
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
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

const DEP_QUERY: &str = r#"
(use_declaration argument: (identifier) @use.root)
(use_declaration argument: (scoped_identifier path: (identifier) @use.root))
(use_declaration argument: (scoped_use_list path: (identifier) @use.root))
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

/// Nearest ancestor dir (under root) holding a declared manifest —
/// that dir's name is the module. Fallback: "_root".
fn module_of(root: &Path, file: &Path, manifests: &[String]) -> String {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if manifests.iter().any(|m| d.join(m).exists()) {
            return d
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| "_root".into());
        }
        dir = d.parent();
        if !d.starts_with(root) {
            break;
        }
    }
    "_root".into()
}

fn walk_files(root: &Path, exts: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if !matches!(
                    e.file_name().to_str(),
                    Some(".git" | "target" | "node_modules" | ".kuda")
                ) && !p.join(".git").is_file()
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
        let mut out = Extraction::default();
        for lang in langs {
            let Some(grammar) = lang.grammar.as_deref().and_then(language) else {
                continue; // no compiled grammar for this lang yet
            };
            let src = lang.symbol_query.as_deref().unwrap_or(DEFAULT_RUST_QUERY);
            let sym_q = Query::new(&grammar, src).map_err(|e| format!("query {e}"))?;
            let dep_q = Query::new(&grammar, DEP_QUERY).map_err(|e| format!("query {e}"))?;
            let sym_caps = sym_q.capture_names().to_vec();
            let dep_caps = dep_q.capture_names().to_vec();
            let mut parser = Parser::new();
            parser
                .set_language(&grammar)
                .map_err(|e| format!("lang {e}"))?;

            for file in walk_files(root, &lang.extensions) {
                let Ok(text) = std::fs::read_to_string(&file) else {
                    continue;
                };
                let Some(tree) = parser.parse(&text, None) else {
                    continue;
                };
                let rel = file.strip_prefix(root).unwrap_or(&file).to_path_buf();
                let module = module_of(root, &file, &lang.manifests);
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
                let mut dep_m = cursor.matches(&dep_q, tree.root_node(), text.as_bytes());
                while let Some(m) = dep_m.next() {
                    for cap in m.captures {
                        let dep = &text[cap.node.byte_range()];
                        if matches!(dep, "self" | "crate" | "std" | "core" | "alloc" | "super")
                            || dep_caps[cap.index as usize].is_empty()
                        {
                            continue;
                        }
                        out.edges.push(DepEdge {
                            from: module.clone(),
                            to: dep.to_owned(),
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
