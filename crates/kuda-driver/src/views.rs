//! Driver manifest + ViewNode trees — the static half of the pavo
//! contract. Mirrors `pavo-protocol`'s JSON shapes exactly (the
//! protocol crate isn't a dependency: kuda ships standalone, and the
//! wire is plain serde JSON). Keep in sync with
//! `pavo/crates/pavo-protocol/src/{manifest,view_node,wire}.rs`.

use serde_json::{json, Value};

fn binding(name: &str) -> Value {
    json!({"name": name, "op": "query", "params": {}})
}

fn spec(
    id: &str,
    title: &str,
    route: &str,
    description: &str,
    queries: &[&str],
    root: Value,
) -> Value {
    json!({
        "id": id,
        "title": title,
        "route": route,
        "description": description,
        "queries": queries.iter().map(|q| binding(q)).collect::<Vec<_>>(),
        "root": root,
    })
}

fn table(
    id: Option<&str>,
    title: &str,
    query: &str,
    columns: &[(&str, &str)],
    sortable: bool,
) -> Value {
    let mut t = json!({
        "kind": "table",
        "title": title,
        "query": query,
        "columns": columns.iter().map(|(k, t)| json!({"key": k, "title": t})).collect::<Vec<_>>(),
    });
    if sortable {
        t["sortable"] = json!(true);
    }
    if let Some(i) = id {
        t["id"] = json!(i);
    }
    t
}

fn kv(title: &str, query: &str) -> Value {
    json!({"kind": "kv", "title": title, "query": query, "pairs": []})
}

fn col(children: Vec<Value>) -> Value {
    json!({"kind": "column", "children": children})
}

/// The `manifest` verb body — also served over the wire `manifest`
/// op and probed by pavo discovery as `tamias-driver-kuda manifest`.
pub fn manifest() -> Value {
    let views = vec![
        spec(
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
        ),
        spec(
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
        ),
        spec(
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
        ),
        spec(
            "graph",
            "Dep graph",
            "/kuda/graph",
            "Module dependency graph from the L2 index — --param level=modules|symbols, records=true includes record-corpus edges.",
            &["graph"],
            json!({
                "kind": "graph",
                "title": "Dependency graph",
                "query": "graph",
                "layout": "dot",
            }),
        ),
        spec(
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
        ),
        spec(
            "find",
            "Find symbol",
            "/kuda/find",
            "Symbol search over the L2 index — --param pattern=<text> (exact: query find).",
            &["find", "search"],
            col(vec![
                json!({
                    "kind": "form",
                    "title": "Search",
                    "fields": [
                        {"key": "pattern", "label": "Pattern", "kind": "text", "hint": "substring match; empty lists recent symbols"},
                        {"key": "exact", "label": "Exact only", "kind": "bool"}
                    ],
                    "submit_op": "query",
                    "submit_label": "Search",
                }),
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
        ),
    ];

    json!({
        "name": "kuda",
        "title": "Kuda",
        "icon": "🗺️",
        "version": env!("CARGO_PKG_VERSION"),
        "verbs": ["manifest", "serve"],
        "capabilities": ["pavo"],
        "contributes": {
            "navbar": [
                {"id": "kuda", "title": "Kuda", "route": "/kuda/modules", "priority": 30, "icon": "🗺️"},
            ],
            "dashboard": [
                {"id": "drift", "title": "Kuda drift", "route": "/kuda/drift", "priority": 40},
            ],
            "statusbar": [
                {"id": "status", "title": "Kuda", "route": "/kuda/drift", "priority": 110},
            ],
        },
        "views": views,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn manifest_advertises_pavo_and_declares_views() {
        let m = manifest();
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
        let m = manifest();
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
