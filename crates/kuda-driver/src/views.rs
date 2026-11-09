//! Driver manifest + ViewNode trees — the static half of the pavo
//! contract, built with `pavo-driver-kit` constructors over
//! `pavo-protocol` types. Keep in sync with
//! `pavo/crates/pavo-protocol/src/{manifest,view_node,wire}.rs`.

use pavo_driver_kit as kit;
use pavo_protocol::{Contribution, DriverManifest, FormFieldKind, QueryBinding, ViewNode};

/// One `query` wire binding — `{"name":n,"op":"query"}`; the host
/// calls the driver's `query` op with the binding name.
fn binding(name: &str) -> QueryBinding {
    kit::binding(name, "query")
}

/// A `ViewSpec` builder shorthand — id/title/route/description plus
/// the query bindings the tree references, and the declarative root.
fn spec(
    id: &str,
    title: &str,
    route: &str,
    description: &str,
    queries: &[&str],
    root: ViewNode,
) -> kit::View {
    let mut v = kit::view(id, title, route, root).description(description);
    for q in queries {
        v = v.query(binding(q));
    }
    v
}

/// `table` node — declared columns, query-fed rows (empty inline
/// `rows`); `sortable` only where the current view had it.
fn table(
    id: Option<&str>,
    title: &str,
    query: &str,
    columns: &[(&str, &str)],
    sortable: bool,
) -> ViewNode {
    let mut base = kit::base().title(title).query(query);
    if let Some(i) = id {
        base = base.id(i);
    }
    let cols = columns
        .iter()
        .map(|(k, t)| kit::table_column(k, t))
        .collect();
    if sortable {
        kit::sortable_table(base, cols, Vec::new())
    } else {
        kit::table(base, cols, Vec::new())
    }
}

/// `kv` node — title + query binding, pairs filled by the host.
fn kv(title: &str, query: &str) -> ViewNode {
    kit::kv(kit::base().title(title).query(query), Vec::new())
}

/// `column` layout container.
fn col(children: Vec<ViewNode>) -> ViewNode {
    kit::column(kit::base(), children)
}

/// `graph` node — query-fed (`graph` binding); `layout: "dot"` is a
/// renderer hint the kit constructor leaves unset, so patch it in.
fn graph_node() -> ViewNode {
    let mut g = kit::graph(
        kit::base().title("Dependency graph").query("graph"),
        Vec::new(),
        Vec::new(),
    );
    if let ViewNode::Graph(ref mut node) = g {
        node.layout = Some("dot".to_string());
    }
    g
}

/// The `find` view's search form — `submit_op: "query"` with a
/// "Search" label (the kit's `form_submit` leaves `submit_label`
/// unset; patch it like `graph`'s layout).
fn search_form() -> ViewNode {
    let mut pattern = kit::field_kind("pattern", "Pattern", FormFieldKind::Text);
    pattern.hint = Some("substring match; empty lists recent symbols".to_string());
    let exact = kit::field_kind("exact", "Exact only", FormFieldKind::Bool);
    let mut form = kit::form_submit(kit::base().title("Search"), vec![pattern, exact], "query");
    if let ViewNode::Form(ref mut node) = form {
        node.submit_label = Some("Search".to_string());
    }
    form
}

/// A chrome-slot [`Contribution`] with explicit priority/icon — the
/// kit's `contribution()` defaults priority to 100 and no icon.
fn contrib(id: &str, title: &str, route: &str, priority: i32, icon: Option<&str>) -> Contribution {
    let mut c = kit::contribution(id, title, route);
    c.priority = priority;
    c.icon = icon.map(str::to_string);
    c
}

/// The `manifest` verb body — also served over the wire `manifest`
/// op and probed by pavo discovery as `tamias-driver-kuda manifest`.
pub fn manifest() -> DriverManifest {
    kit::Manifest::new("kuda", "Kuda")
        .version(env!("CARGO_PKG_VERSION"))
        .icon("🗺️")
        .navbar(contrib("kuda", "Kuda", "/kuda/modules", 30, Some("🗺️")))
        .dashboard(contrib("drift", "Kuda drift", "/kuda/drift", 40, None))
        .statusbar(contrib("status", "Kuda", "/kuda/drift", 110, None))
        .view(spec(
            "map",
            "Repo map",
            "/kuda",
            "L0 repo card — name, purpose, module/file counts, provenance sha.",
            &["repo", "modules"],
            col(vec![
                kv("Repository", "repo"),
                table(
                    None,
                    "Modules",
                    "modules",
                    &[
                        ("name", "Module"),
                        ("purpose", "Purpose"),
                        ("api", "API"),
                        ("deps", "Deps"),
                        ("drift", "Drift"),
                    ],
                    true,
                ),
            ]),
        ))
        .view(spec(
            "modules",
            "Modules",
            "/kuda/modules",
            "Every module card: purpose, api surface size, dep count, drift status.",
            &["modules"],
            table(
                None,
                "Module cards",
                "modules",
                &[
                    ("name", "Module"),
                    ("purpose", "Purpose"),
                    ("api", "API"),
                    ("deps", "Deps"),
                    ("drift", "Drift"),
                ],
                true,
            ),
        ))
        .view(spec(
            "module",
            "Module card",
            "/kuda/module",
            "One module card in detail — pass --param name=<module>.",
            &["module", "module_deps", "module_symbols"],
            col(vec![
                kv("Card", "module"),
                table(None, "Deps", "module_deps", &[("dep", "Dependency")], false),
                table(
                    None,
                    "API symbols",
                    "module_symbols",
                    &[
                        ("name", "Symbol"),
                        ("kind", "Kind"),
                        ("file", "File"),
                        ("line", "Line"),
                    ],
                    true,
                ),
            ]),
        ))
        .view(spec(
            "graph",
            "Dep graph",
            "/kuda/graph",
            "Module dependency graph from the L2 index — --param level=modules|symbols, records=true includes record-corpus edges.",
            &["graph"],
            graph_node(),
        ))
        .view(spec(
            "drift",
            "Drift",
            "/kuda/drift",
            "kuda status equivalent — per-card provenance drift vs working tree.",
            &["drift", "modules"],
            col(vec![
                kv("Drift summary", "drift"),
                table(
                    None,
                    "Cards",
                    "modules",
                    &[
                        ("name", "Module"),
                        ("purpose", "Purpose"),
                        ("api", "API"),
                        ("deps", "Deps"),
                        ("drift", "Drift"),
                    ],
                    true,
                ),
            ]),
        ))
        .view(spec(
            "find",
            "Find symbol",
            "/kuda/find",
            "Symbol search over the L2 index — --param pattern=<text> (exact: query find).",
            &["find", "search"],
            col(vec![
                search_form(),
                table(
                    None,
                    "Exact matches",
                    "find",
                    &[
                        ("name", "Symbol"),
                        ("kind", "Kind"),
                        ("module", "Module"),
                        ("file", "File"),
                        ("line", "Line"),
                    ],
                    true,
                ),
                table(
                    None,
                    "Substring matches",
                    "search",
                    &[
                        ("name", "Symbol"),
                        ("kind", "Kind"),
                        ("module", "Module"),
                        ("file", "File"),
                        ("line", "Line"),
                    ],
                    true,
                ),
            ]),
        ))
        .build()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn manifest_advertises_pavo_and_declares_views() {
        let m = serde_json::to_value(manifest()).unwrap();
        assert_eq!(m["name"], "kuda");
        assert!(m["verbs"].as_array().unwrap().iter().any(|v| v == "serve"));
        assert!(m["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "pavo"));
        let views = m["views"].as_array().unwrap();
        assert!(!views.is_empty());
        for v in views {
            assert!(v["root"]["kind"].is_string(), "view {} has root", v["id"]);
            for q in v["queries"].as_array().unwrap() {
                assert_eq!(q["op"], "query");
                assert!(q["name"].is_string());
            }
        }
    }

    #[test]
    fn every_binding_name_has_a_query_impl() {
        // run_query's match arms — keep this list in sync with
        // queries::run_query.
        let known = [
            "repo",
            "modules",
            "module",
            "card",
            "module_deps",
            "module_symbols",
            "graph",
            "drift",
            "find",
            "search",
            "dependents",
            "status",
        ];
        let m = serde_json::to_value(manifest()).unwrap();
        for v in m["views"].as_array().unwrap() {
            for q in v["queries"].as_array().unwrap() {
                let name = q["name"].as_str().unwrap();
                assert!(
                    known.contains(&name),
                    "unbound query '{name}' in view {}",
                    v["id"]
                );
            }
        }
    }
}
