//! Engine ops — `cli.<op>` wire queries dispatch into the `kuda`
//! engine lib through this binary's own `run` verb: each call
//! re-execs `tamias-driver-kuda run <argv>` with the request's repo
//! as cwd, so `kuda::dispatch` writes to the child's captured stdout
//! and never touches the serve loop's ndjson stream (the engine's
//! emit() exits on broken pipe — one more reason ops stay out of
//! the serve process).
//!
//! `ops` answers the manifest table below as JSON. Long-running
//! surfaces (`watch`, `mcp`, `acp`) are deliberately *not* ops —
//! they never return, so they stay out of the request/response wire.
//! They live in the manifest's `surfaces` list instead: surface
//! drivers (tamias-driver-cli / -mcp, the pavo-driver-kit `surface`
//! layer) expose them as streaming subcommands —
//! `tamias-driver-kuda cli watch` ≡ `kuda watch`.

use pavo_protocol::wire::{QueryResult, Response};
use serde_json::{json, Map, Value};
use std::path::Path;

/// One op param, typed the way `pavo-driver-kit`'s surface layer
/// spells it: `arg` = positional, `opt` = `--name [v]`, `flag` =
/// `--name` bool. (Bare-string params in a manifest read as `opt` —
/// typed entries are what let generated surfaces not guess.)
struct Param {
    name: &'static str,
    kind: &'static str,
}

const fn arg(name: &'static str) -> Param {
    Param { name, kind: "arg" }
}
const fn opt(name: &'static str) -> Param {
    Param { name, kind: "opt" }
}
const fn flag(name: &'static str) -> Param {
    Param { name, kind: "flag" }
}

/// One engine verb in the ops manifest.
struct Op {
    /// `cli.<name>` wire query / `kuda <name>` argv verb.
    name: &'static str,
    /// One-line doc — mirrors the clap subcommand's help.
    doc: &'static str,
    /// Wire params the op reads (positional args + flags); `workspace`
    /// and `config` are the engine's globals, accepted by every op.
    params: &'static [Param],
    /// `true` when the op writes `.kuda/` — mutators belong behind a
    /// deliberate trigger, not ambient view polling.
    mutates: bool,
}

/// The ops manifest — the engine's op table as driver-facing data.
const OPS: &[Op] = &[
    Op {
        name: "sync",
        doc: "incremental index (hash -> extract -> summarize -> store)",
        params: &[flag("full"), flag("workspace"), opt("config")],
        mutates: true,
    },
    Op {
        name: "status",
        doc: "per-card provenance drift vs HEAD",
        params: &[flag("workspace"), opt("config")],
        mutates: false,
    },
    Op {
        name: "map",
        doc: "L0 repo card — the always-load blob",
        params: &[flag("workspace"), opt("config")],
        mutates: false,
    },
    Op {
        name: "card",
        doc: "module card + freshness",
        params: &[arg("module"), flag("workspace"), opt("config")],
        mutates: false,
    },
    Op {
        name: "find",
        doc: "symbol -> file:line",
        params: &[arg("symbol"), flag("workspace"), opt("config")],
        mutates: false,
    },
    Op {
        name: "dependents",
        doc: "reverse dep lookup — impact analysis",
        params: &[arg("name"), flag("workspace"), opt("config")],
        mutates: false,
    },
    Op {
        name: "refs",
        doc: "code + docs mentions — the doc-sweep",
        params: &[arg("name"), flag("docs"), flag("workspace"), opt("config")],
        mutates: false,
    },
    Op {
        name: "doctor",
        doc: "extractor/store/config health",
        params: &[flag("workspace"), opt("config")],
        mutates: false,
    },
];

/// The `ops` query/`ops` argv verb payload — self-describing so
/// hosts and agents discover the engine surface without argv help.
/// `params` are typed (`{name, kind: arg|opt|flag}`) — the contract
/// surface drivers consume to render args without guessing.
pub fn manifest() -> Value {
    json!({
        "engine": "kuda",
        "invoke": "query `cli.<op>` with the listed params, or argv `tamias-driver-kuda run <kuda argv>`",
        "ops": OPS.iter().map(|o| json!({
            "query": format!("cli.{}", o.name),
            "doc": o.doc,
            "params": o.params.iter().map(|p| json!({
                "name": p.name,
                "kind": p.kind,
            })).collect::<Vec<_>>(),
            "mutates": o.mutates,
        })).collect::<Vec<_>>(),
        "surfaces": ["watch", "mcp", "acp"],
    })
}

/// Is `query` an engine op (`cli.<op>`)? Returns the op name.
pub fn op_of(query: &str) -> Option<&'static str> {
    query
        .strip_prefix("cli.")
        .and_then(|n| OPS.iter().find(|o| o.name == n).map(|o| o.name))
}

fn bool_param(params: &Map<String, Value>, key: &str) -> bool {
    params.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn str_param<'a>(params: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| params.get(*k).and_then(Value::as_str))
}

/// `cli.<op>` params → kuda argv (no argv[0] — `run` prepends it).
/// Globals go first; clap accepts them either side of the verb.
fn argv(op: &str, params: &Map<String, Value>) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if bool_param(params, "workspace") {
        v.push("--workspace".into());
    }
    if let Some(c) = str_param(params, &["config"]) {
        v.extend(["--config".into(), c.to_owned()]);
    }
    v.push(op.into());
    match op {
        "sync" => {
            if bool_param(params, "full") {
                v.push("--full".into());
            }
        }
        "card" => {
            if let Some(m) = str_param(params, &["module", "name"]) {
                v.push(m.into());
            }
        }
        "find" => {
            if let Some(s) = str_param(params, &["symbol", "name", "pattern", "query"]) {
                v.push(s.into());
            }
        }
        "dependents" | "refs" => {
            if let Some(n) = str_param(params, &["name", "module", "symbol"]) {
                v.push(n.into());
            }
            if op == "refs" && bool_param(params, "docs") {
                v.push("--docs".into());
            }
        }
        _ => {}
    }
    v
}

/// Run one engine op against `root`: re-exec `run` in a child whose
/// cwd is the request's repo, and hand back
/// `{op, argv, code, stdout, stderr}` as the query payload.
/// `ok:false` is reserved for spawn failures — a nonzero `code` is
/// the engine's own exit status (e.g. `find` miss), honest data.
pub fn run_op(root: &Path, op: &str, params: &Map<String, Value>) -> Response {
    let args = argv(op, params);
    let out = std::env::current_exe().and_then(|exe| {
        std::process::Command::new(exe)
            .arg("run")
            .args(&args)
            .current_dir(root)
            .output()
    });
    let data = match out {
        Ok(o) => json!({
            "op": op,
            "argv": args,
            "code": o.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&o.stdout),
            "stderr": String::from_utf8_lossy(&o.stderr),
        }),
        Err(e) => return Response::err(format!("run {op}: {e}")),
    };
    Response::ok_typed(&QueryResult {
        query: format!("cli.{op}"),
        data,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn op_of_accepts_cli_prefix_only() {
        assert_eq!(op_of("cli.sync"), Some("sync"));
        assert_eq!(op_of("cli.doctor"), Some("doctor"));
        assert_eq!(op_of("cli.watch"), None); // surface, not an op
        assert_eq!(op_of("find"), None); // view query, not an op
        assert_eq!(op_of("cli.bogus"), None);
    }

    #[test]
    fn argv_maps_params_to_kuda_flags() {
        let mut p = Map::new();
        assert_eq!(argv("status", &p), vec!["status"]);
        p.insert("full".into(), json!(true));
        p.insert("workspace".into(), json!(true));
        assert_eq!(argv("sync", &p), vec!["--workspace", "sync", "--full"]);
        let mut p = Map::new();
        p.insert("module".into(), json!("kuda-store"));
        assert_eq!(argv("card", &p), vec!["card", "kuda-store"]);
        let mut p = Map::new();
        p.insert("name".into(), json!("X"));
        p.insert("docs".into(), json!(true));
        assert_eq!(argv("refs", &p), vec!["refs", "X", "--docs"]);
    }

    #[test]
    fn manifest_lists_every_op_as_cli_query() {
        let m = manifest();
        let ops = m["ops"].as_array().unwrap();
        assert_eq!(ops.len(), OPS.len());
        for (i, o) in OPS.iter().enumerate() {
            assert_eq!(ops[i]["query"], json!(format!("cli.{}", o.name)));
            assert_eq!(ops[i]["mutates"], json!(o.mutates));
            // params ride typed ({name, kind: arg|opt|flag}) — the
            // contract surface drivers render args from.
            for p in ops[i]["params"].as_array().unwrap() {
                assert!(["arg", "opt", "flag"].contains(&p["kind"].as_str().unwrap()));
            }
        }
        assert_eq!(m["surfaces"], json!(["watch", "mcp", "acp"]));
    }
}
