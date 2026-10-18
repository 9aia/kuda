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
                let Some(tree) = parser.parse(&text, None) else {
                    continue;
                };
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
