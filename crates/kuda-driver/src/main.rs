//! tamias-driver-kuda — pavo view driver for kuda.
//!
//! Answers the pavo wire ops (`manifest`, `views`, `view`, `query`)
//! as ndjson JSON-RPC over stdio — the same `{"op":…}` /
//! `{"ok":…,"result"|"error":…}` shape every sibling `serve` speaks
//! (see `pavo/crates/pavo-protocol/src/wire.rs`). Also answers the
//! discovery verbs `manifest` and `capabilities` as plain argv.
//!
//! Repo context: the driver is read-only. It resolves the kuda repo
//! from the `root`/`dir`/`path` request param, `$KUDA_DRIVER_ROOT`,
//! or by walking up from its cwd (pavo inherits the user's cwd, so
//! `pavo view kuda:graph` inside repo X reads X's `.kuda/`).
//!
//! Install: `cargo install --path crates/kuda-driver` places
//! `tamias-driver-kuda` on PATH — pavo discovers it there or under
//! `<tamias-home>/drivers/`.

mod queries;
mod repo;
mod views;

use queries::run_query;
use repo::{resolve_root, Ctx};
use serde_json::{json, Map, Value};
use std::io::{BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("manifest") => {
            println!(
                "{}",
                serde_json::to_string_pretty(&views::manifest()).unwrap_or_default()
            );
            0
        }
        Some("capabilities") => {
            println!(
                "{}",
                json!({
                    "capabilities": ["pavo"],
                    "verbs": ["manifest", "serve"],
                    "driver": "kuda",
                })
            );
            0
        }
        Some("serve") | None => serve(),
        _ => {
            eprintln!("usage: tamias-driver-kuda manifest|capabilities|serve");
            1
        }
    };
    std::process::exit(code);
}

fn emit(v: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

fn ok(result: Value) {
    emit(&json!({"ok": true, "result": result}));
}

fn err(msg: impl Into<String>) {
    emit(&json!({"ok": false, "error": msg.into()}));
}

/// One ndjson request line → response. Never panics on bad input.
fn dispatch(line: &str) {
    let parsed: Result<Value, _> = serde_json::from_str(line);
    let Ok(obj) = parsed else {
        err("invalid JSON request");
        return;
    };
    let Some(op) = obj.get("op").and_then(Value::as_str) else {
        err("missing 'op'");
        return;
    };
    let params = obj
        .get("params")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    match op {
        "manifest" => ok(views::manifest()),
        "views" => ok(views_summaries()),
        "view" => view(&obj, &params),
        "query" => query(&obj, &params),
        other => err(format!("unknown op: {other}")),
    }
}

/// `Vec<ViewSummary>` — id/title/route/description/queries, no trees.
fn views_summaries() -> Value {
    let m = views::manifest();
    Value::Array(
        m.get("views")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|v| {
                json!({
                    "id": v.get("id").cloned().unwrap_or(Value::Null),
                    "title": v.get("title").cloned().unwrap_or(Value::Null),
                    "route": v.get("route").cloned().unwrap_or(Value::Null),
                    "description": v.get("description").cloned().unwrap_or(Value::Null),
                    "queries": v.get("queries").and_then(Value::as_array).map(|qs| {
                        qs.iter()
                            .filter_map(|b| b.get("name").cloned())
                            .collect::<Vec<_>>()
                    }).unwrap_or_default(),
                })
            })
            .collect(),
    )
}

/// `view` — resolve one ViewSpec, run every query binding against a
/// fresh `Ctx` for the request's repo, return `ViewResult`.
fn view(obj: &Value, params: &Map<String, Value>) {
    let Some(id) = obj.get("view").and_then(Value::as_str) else {
        err("missing 'view'");
        return;
    };
    let manifest = views::manifest();
    let Some(spec) = manifest
        .get("views")
        .and_then(Value::as_array)
        .and_then(|vs| {
            vs.iter()
                .find(|v| v.get("id").and_then(Value::as_str) == Some(id))
        })
        .cloned()
    else {
        err(format!("unknown view: {id}"));
        return;
    };
    let ctx = Ctx::open(resolve_root(params));
    let mut data = Map::new();
    if let Some(bindings) = spec.get("queries").and_then(Value::as_array) {
        for b in bindings {
            let Some(name) = b.get("name").and_then(Value::as_str) else {
                continue;
            };
            // static binding params first, caller params override
            let mut merged = b
                .get("params")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            for (k, v) in params {
                merged.insert(k.clone(), v.clone());
            }
            data.insert(name.to_string(), run_query(&ctx, name, &merged));
        }
    }
    ok(json!({"view": spec, "data": data}));
}

/// `query` — one named query with caller params.
fn query(obj: &Value, params: &Map<String, Value>) {
    let Some(name) = obj.get("query").and_then(Value::as_str) else {
        err("missing 'query'");
        return;
    };
    let ctx = Ctx::open(resolve_root(params));
    let data = run_query(&ctx, name, params);
    ok(json!({"query": name, "data": data}));
}

fn serve() -> i32 {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // dispatch writes via emit(); keep stdout lock coherent by
        // flushing through the same handle.
        dispatch(line);
        let _ = stdout.flush();
    }
    0
}
