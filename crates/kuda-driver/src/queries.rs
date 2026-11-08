//! Query implementations — pavo `query` names mapped onto kuda's
//! surfaces: `kuda-query` functions over `IndexStore`, the committed
//! `.kuda/` cards, and drift recomputation (`kuda status` semantics).
//!
//! Every query returns a `serde_json::Value` payload. Failures come
//! back as `{"error": "…"}` markers — the pavo wire contract treats
//! those as partial degradation, never a crashed view.

use crate::repo::{card_drift, safe, Ctx};
use kuda_core::{module_key, DepEdge, Symbol, SymbolKind};
use kuda_store::IndexStore;
use kuda_store_sqlite::SqliteStore;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Cap on symbol rows any list query returns — keeps `find`/`search`
/// payloads bounded on large repos.
const ROW_CAP: usize = 200;
/// Cap on symbol nodes in `level=symbols` graph — beyond this the
/// graph is noise; callers get an honest error pointing at
/// `level=modules`.
const SYMBOL_GRAPH_CAP: usize = 800;

fn err(msg: impl Into<String>) -> Value {
    json!({"error": msg.into()})
}

fn need_store(ctx: &Ctx) -> Result<&dyn IndexStore, Value> {
    ctx.store
        .as_ref()
        .map(|s| s as &dyn IndexStore)
        .ok_or_else(|| err("no .kuda/index.sqlite — run `kuda sync` first"))
}

fn need_indexed(ctx: &Ctx) -> Result<(), Value> {
    if ctx.indexed() {
        Ok(())
    } else {
        Err(err(format!(
            "{} is not kuda-indexed (no .kuda/ or kuda.toml found)",
            ctx.root.display()
        )))
    }
}

fn param_str<'a>(params: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| params.get(*k).and_then(Value::as_str))
}

fn symbol_kind_str(k: SymbolKind) -> &'static str {
    match k {
        SymbolKind::Function => "function",
        SymbolKind::Type => "type",
        SymbolKind::Trait => "trait",
        SymbolKind::Const => "const",
        SymbolKind::Module => "module",
        SymbolKind::Macro => "macro",
        SymbolKind::Record => "record",
        SymbolKind::Other => "other",
    }
}

fn edge_kind_str(k: kuda_core::EdgeKind) -> &'static str {
    match k {
        kuda_core::EdgeKind::Imports => "imports",
        kuda_core::EdgeKind::Calls => "calls",
        kuda_core::EdgeKind::PackageDep => "package-dep",
        kuda_core::EdgeKind::WorkspaceMember => "workspace-member",
        kuda_core::EdgeKind::References => "references",
    }
}

fn symbol_row(s: &Symbol) -> Value {
    json!({
        "name": s.name,
        "kind": symbol_kind_str(s.kind),
        "module": s.module,
        "file": s.file.to_string_lossy(),
        "line": s.line,
    })
}

/// First line of a card purpose, capped — card purposes can carry
/// whole agent prose; tables need one-line cells.
fn short_purpose(p: &str) -> String {
    let first = p.lines().next().unwrap_or("").trim();
    const CAP: usize = 90;
    if first.chars().count() <= CAP {
        first.to_string()
    } else {
        format!("{}…", first.chars().take(CAP).collect::<String>())
    }
}

fn capped(mut rows: Vec<Value>) -> Value {
    rows.truncate(ROW_CAP);
    Value::Array(rows)
}

/// Per-card drift rows shared by `modules`, `drift`, and `status` —
/// `(name, drift)` sorted by card name.
fn drift_rows(ctx: &Ctx) -> Vec<(String, &'static str)> {
    ctx.module_cards()
        .iter()
        .map(|(name, card)| (name.clone(), card_drift(&ctx.root, card)))
        .collect()
}

/// Dispatch one named query. Unknown names get an error payload —
/// hosts see `{"error": …}` data, matching the partial-degradation
/// contract.
pub fn run_query(ctx: &Ctx, name: &str, params: &Map<String, Value>) -> Value {
    match name {
        "repo" => repo(ctx),
        "modules" => modules(ctx),
        "module" | "card" => module(ctx, params),
        "module_deps" => module_deps(ctx, params),
        "module_symbols" => module_symbols(ctx, params),
        "graph" => graph(ctx, params),
        "drift" => drift(ctx),
        "find" => find(ctx, params, false),
        "search" => find(ctx, params, true),
        "dependents" => dependents(ctx, params),
        "status" => status(ctx),
        other => err(format!("unknown query: {other}")),
    }
}

/// `repo` — L0 card summary for the `kuda:map` kv node.
fn repo(ctx: &Ctx) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let Some(card) = &ctx.repo_card else {
        return err(format!("no .kuda/index.yaml under {}", ctx.root.display()));
    };
    json!({
        "name": card.name,
        "purpose": card.purpose,
        "modules": card.modules.len(),
        "files_total": card.files_total,
        "tree_digest": card.tree_digest,
        "commands": card.commands,
        "generated_at": card.provenance.generated_at,
        "commit_sha": card.provenance.commit_sha,
        "head": ctx.head,
    })
}

/// `modules` — one row per module card: name, purpose, api symbol
/// count (incl. omitted), dep count, drift status.
fn modules(ctx: &Ctx) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let drift = drift_rows(ctx);
    let drift_of: BTreeMap<&str, &str> = drift.iter().map(|(n, d)| (n.as_str(), *d)).collect();
    let rows: Vec<Value> = ctx
        .module_cards()
        .iter()
        .map(|(_, card)| {
            json!({
                "name": card.name,
                "purpose": short_purpose(&card.purpose),
                "api": card.api.len() + card.api_omitted,
                "deps": card.deps.len(),
                "drift": drift_of.get(card.name.as_str()).copied().unwrap_or("unreadable"),
            })
        })
        .collect();
    Value::Array(rows)
}

/// `module`/`card` — one module card as a flat kv object plus its
/// drift status. Params: `name` (required).
fn module(ctx: &Ctx, params: &Map<String, Value>) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let Some(name) = param_str(params, &["name", "module"]) else {
        return err("param 'name' required");
    };
    let path = ctx.modules_dir().join(format!("{}.yaml", safe(name)));
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            return err(format!("no card for '{name}' (tried {})", path.display()));
        }
    };
    let card = match kuda_cards::parse_module_card(&text) {
        Ok(c) => c,
        Err(e) => return err(format!("parse {}: {e}", path.display())),
    };
    json!({
        "name": card.name,
        "path": card.path.to_string_lossy(),
        "purpose": card.purpose,
        "api_count": card.api.len() + card.api_omitted,
        "deps": card.deps,
        "notes": card.notes,
        "warnings": card.warnings,
        "verify": card.verify.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "provenance_commit": card.provenance.commit_sha,
        "provenance_generated": card.provenance.generated_at,
        "provenance_files": card.provenance.files.len(),
        "drift": card_drift(&ctx.root, &card),
    })
}

/// `module_deps` — outgoing deps of one module card as table rows.
/// Params: `name` (required).
fn module_deps(ctx: &Ctx, params: &Map<String, Value>) -> Value {
    let v = module(ctx, params);
    if v.get("error").is_some() {
        return v;
    }
    let deps = v
        .get("deps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Value::Array(
        deps.iter()
            .filter_map(Value::as_str)
            .map(|d| json!({"dep": d}))
            .collect(),
    )
}

/// `module_symbols` — api symbols of one module as table rows.
/// Prefers the L2 store (full symbol set); falls back to the card's
/// committed `api` list when the store is absent. Params: `name`.
fn module_symbols(ctx: &Ctx, params: &Map<String, Value>) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let Some(name) = param_str(params, &["name", "module"]) else {
        return err("param 'name' required");
    };
    if let Some(store) = ctx.store.as_ref() {
        let syms: Vec<Symbol> = store
            .all_symbols()
            .unwrap_or_default()
            .into_iter()
            .filter(|s| module_key(&s.module) == module_key(name))
            .collect();
        if !syms.is_empty() {
            return capped(syms.iter().map(symbol_row).collect());
        }
    }
    // card fallback — api symbols carry file/line too
    let v = module(ctx, params);
    if v.get("error").is_some() {
        return err(format!("no symbols or card for '{name}'"));
    }
    // module() doesn't emit the api list; re-read the card directly.
    let path = ctx.modules_dir().join(format!("{}.yaml", safe(name)));
    match std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| kuda_cards::parse_module_card(&t).ok())
    {
        Some(card) => capped(card.api.iter().map(symbol_row).collect()),
        None => err(format!("no symbols or card for '{name}'")),
    }
}

/// `graph` — dep graph for `kuda:graph`. Params: `level` =
/// `modules` (default) | `symbols`.
fn graph(ctx: &Ctx, params: &Map<String, Value>) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let Some(store) = ctx.store.as_ref() else {
        return err("no .kuda/index.sqlite — run `kuda sync` first");
    };
    let level = param_str(params, &["level"]).unwrap_or("modules");
    // `references` edges are record-corpus mentions (ADR/task ids),
    // not module deps — excluded from the showcase graph unless
    // `records=true`/`all=true` asks for the full edge set.
    let include_records = params
        .get("records")
        .or_else(|| params.get("all"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match level {
        "symbols" => graph_symbols(ctx, store, include_records),
        _ => graph_modules(ctx, store, include_records),
    }
}

fn edge_json(e: &DepEdge) -> Value {
    json!({
        "from": e.from,
        "to": e.to,
        "label": edge_kind_str(e.kind),
    })
}

/// Module-level graph: nodes = indexed modules ∪ edge endpoints,
/// edges = every `edges` row minus self-edges. Endpoints that aren't
/// indexed modules are marked `kind: "external"`.
fn graph_modules(ctx: &Ctx, store: &SqliteStore, include_records: bool) -> Value {
    let indexed: BTreeSet<String> = kuda_query::modules(store)
        .unwrap_or_default()
        .into_iter()
        .collect();
    let mut node_ids: BTreeSet<String> = indexed.iter().cloned().collect();
    let mut edges = Vec::new();
    match store.all_edges() {
        Ok(rows) => {
            for e in &rows {
                // drop self-edges (same rule as kuda_query::dependents)
                if module_key(&e.from) == module_key(&e.to) {
                    continue;
                }
                if !include_records && e.kind == kuda_core::EdgeKind::References {
                    continue;
                }
                node_ids.insert(e.from.clone());
                node_ids.insert(e.to.clone());
                edges.push(edge_json(e));
            }
        }
        Err(e) => return err(e),
    }
    let nodes: Vec<Value> = node_ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "label": id,
                "kind": if indexed.contains(id) { "module" } else { "external" },
            })
        })
        .collect();
    json!({
        "level": "modules",
        "root": ctx.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        "nodes": nodes,
        "edges": edges,
    })
}

/// Symbol-level graph: module nodes + symbol nodes (`member` edges)
/// plus the module dep edges. Capped — large repos should use
/// `level=modules`.
fn graph_symbols(ctx: &Ctx, store: &SqliteStore, include_records: bool) -> Value {
    let mut syms = match store.all_symbols() {
        Ok(s) => s,
        Err(e) => return err(e),
    };
    syms.sort_by(|a, b| {
        (&a.module, &a.name, &a.file, a.line).cmp(&(&b.module, &b.name, &b.file, b.line))
    });
    if syms.len() > SYMBOL_GRAPH_CAP {
        return err(format!(
            "symbol graph too large ({} symbols > {SYMBOL_GRAPH_CAP}) — use level=modules",
            syms.len()
        ));
    }
    let mut nodes: Vec<Value> = Vec::new();
    let mut edges: Vec<Value> = Vec::new();
    let mut seen_modules: BTreeSet<String> = BTreeSet::new();
    for s in &syms {
        if seen_modules.insert(s.module.clone()) {
            nodes.push(json!({
                "id": format!("mod:{}", s.module),
                "label": s.module,
                "kind": "module",
            }));
        }
        let sid = format!("sym:{}:{}:{}", s.module, s.name, s.line);
        nodes.push(json!({
            "id": sid,
            "label": s.name,
            "kind": symbol_kind_str(s.kind),
            "props": {"module": s.module, "file": s.file.to_string_lossy(), "line": s.line},
        }));
        edges.push(json!({
            "from": sid,
            "to": format!("mod:{}", s.module),
            "label": "member",
        }));
    }
    // module dep edges between the module container nodes
    if let Ok(rows) = store.all_edges() {
        let mut mod_ids: BTreeSet<String> = seen_modules.iter().cloned().collect();
        for e in &rows {
            if module_key(&e.from) == module_key(&e.to) {
                continue;
            }
            if !include_records && e.kind == kuda_core::EdgeKind::References {
                continue;
            }
            mod_ids.insert(e.from.clone());
            mod_ids.insert(e.to.clone());
            edges.push(json!({
                "from": format!("mod:{}", e.from),
                "to": format!("mod:{}", e.to),
                "label": edge_kind_str(e.kind),
            }));
        }
        // ensure every dep endpoint has a module node even with no symbols
        for id in mod_ids {
            if !seen_modules.contains(&id) {
                nodes.push(json!({
                    "id": format!("mod:{id}"),
                    "label": id,
                    "kind": "module",
                }));
            }
        }
    }
    json!({
        "level": "symbols",
        "root": ctx.root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        "nodes": nodes,
        "edges": edges,
    })
}

/// `drift` — `kuda status` summary: per-card drift counts + HEAD.
/// (Repo-level `tree_digest` re-walk is `kuda status`'s job — the
/// driver reports card drift, which is the cheap 95%.)
fn drift(ctx: &Ctx) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let rows = drift_rows(ctx);
    let fresh = rows.iter().filter(|(_, d)| *d == "fresh").count();
    let drifted = rows.iter().filter(|(_, d)| *d == "drifted").count();
    json!({
        "head": ctx.head,
        "fresh": fresh,
        "drifted": drifted,
        "total": rows.len(),
        "tree": "not-checked",
    })
}

/// `find`/`search` — exact vs substring symbol lookup over L2.
fn find(ctx: &Ctx, params: &Map<String, Value>, fuzzy: bool) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let store = match need_store(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let pattern = param_str(params, &["pattern", "query", "name", "symbol"]).unwrap_or("");
    let result = if fuzzy {
        kuda_query::search(store, pattern)
    } else {
        kuda_query::find(store, pattern)
    };
    match result {
        Ok(syms) => capped(syms.iter().map(symbol_row).collect()),
        Err(e) => err(e),
    }
}

/// `dependents` — reverse dep lookup (impact analysis).
fn dependents(ctx: &Ctx, params: &Map<String, Value>) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let store = match need_store(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let Some(name) = param_str(params, &["name", "module"]) else {
        return err("param 'name' required");
    };
    match kuda_query::dependents(store, name) {
        Ok(edges) => capped(edges.iter().map(edge_json).collect()),
        Err(e) => err(e),
    }
}

/// `status` — one-object summary: counts + drift + head.
fn status(ctx: &Ctx) -> Value {
    if let Err(e) = need_indexed(ctx) {
        return e;
    }
    let (modules_n, edges_n, symbols_n) = match ctx.store.as_ref() {
        Some(s) => (
            kuda_query::modules(s).map(|m| m.len()).unwrap_or(0),
            kuda_query::edge_count(s).unwrap_or(0),
            s.all_symbols().map(|v| v.len()).unwrap_or(0),
        ),
        None => (0, 0, 0),
    };
    let d = drift(ctx);
    let get = |k: &str| d.get(k).cloned().unwrap_or(Value::Null);
    json!({
        "root": ctx.root.to_string_lossy(),
        "head": ctx.head,
        "modules": modules_n,
        "edges": edges_n,
        "symbols": symbols_n,
        "fresh": get("fresh"),
        "drifted": get("drifted"),
        "cards": get("total"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::repo::Ctx;

    #[test]
    fn unknown_query_is_error_payload() {
        let tmp = std::env::temp_dir().join(format!("kuda-drv-q-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let ctx = Ctx::open(tmp.clone());
        let v = run_query(&ctx, "bogus", &Map::new());
        assert!(v.get("error").unwrap().as_str().unwrap().contains("bogus"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn short_purpose_takes_first_line() {
        assert_eq!(short_purpose("one\ntwo"), "one");
        let long = "x".repeat(200);
        assert!(short_purpose(&long).ends_with('…'));
        assert!(short_purpose(&long).chars().count() <= 92);
    }
}
