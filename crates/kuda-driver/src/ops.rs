//! Engine ops — `cli.<verb>` wire queries dispatch into the `kuda`
//! engine lib through this binary's own `run` verb: each call
//! re-execs `tamias-driver-kuda run <argv>` with cwd set to the
//! request's repo, so `kuda::dispatch` writes to the child's
//! captured stdout and never touches the serve loop's ndjson stream
//! (the engine's `emit()` exits on broken pipe — one more reason
//! ops stay out of the serve process).
//!
//! `ops` answers the ops manifest as JSON — *derived from the
//! engine's own clap tree* ([`kuda::command`]), so a verb can't
//! exist on the CLI and be missing here: declared args become
//! typed params (`arg`/`opt`/`flag`), doc strings ride along
//! unchanged. The CLI's globals (`--config`, `--workspace`) ride
//! the manifest's `globals` list once, not per-op.
//!
//! `watch`/`mcp`/`acp` can never be request/response ops — they
//! never return, so they ride `surfaces` instead: surface drivers
//! (tamias-driver-cli / -mcp, the pavo-driver-kit `surface` layer)
//! expose them as streaming subcommands —
//! `tamias-driver-kuda cli watch` ≡ `kuda watch`. Hidden plumbing
//! (`gen-cli-docs`) stays reachable through `run` passthrough only.
//!
//! `mutates` is honest: `init` scaffolds `kuda.toml` + `.kuda/`,
//! `sync` rewrites cards + the sqlite index; `status`/`map`/
//! `card`/`find`/`dependents`/`refs`/`doctor` only read.
//! Streaming *flags* would join `SKIP_PARAMS` — none exist among
//! bounded verbs today (`watch --poll-refs` rides the surface).

use clap::{Arg, ArgAction, Command};
use pavo_protocol::wire::{QueryResult, Response};
use serde_json::{json, Map, Value};
use std::path::Path;

/// Unbounded surfaces — verbs that never return (`watch`'s
/// debounced loop, `mcp`/`acp` stdio servers). They can never be
/// request/response ops; they need inherited stdio.
const SURFACES: &[&str] = &["watch", "mcp", "acp"];

/// Params excluded from ops — flags that turn a bounded verb into
/// a stream, or params that must never ride argv. None today.
const SKIP_PARAMS: &[&str] = &[];

/// Read-only verbs — everything else (`init`, `sync`) writes
/// `kuda.toml`/`.kuda/` cards + the sqlite index. Mutators belong
/// behind a deliberate trigger, not ambient view polling.
const READ_ONLY: &[&str] = &[
    "status",
    "map",
    "card",
    "find",
    "dependents",
    "refs",
    "doctor",
];

/// `(name, clap spec)` for every leaf verb. Hidden plumbing and
/// clap's `help` are skipped — they stay reachable through `run`
/// passthrough.
fn verbs() -> Vec<(String, Command)> {
    fn walk(prefix: &str, cmd: &Command, out: &mut Vec<(String, Command)>) {
        for sub in cmd.get_subcommands() {
            let name = sub.get_name();
            if name == "help" || sub.is_hide_set() {
                continue;
            }
            let dotted = match prefix {
                "" => name.to_string(),
                _ => format!("{prefix}.{name}"),
            };
            if sub.has_subcommands() {
                walk(&dotted, sub, out);
            } else {
                out.push((dotted, sub.clone()));
            }
        }
    }
    let mut out = Vec::new();
    walk("", &kuda::command(), &mut out);
    out
}

fn op_cmd(name: &str) -> Option<Command> {
    verbs()
        .into_iter()
        .find(|(n, _)| n == name && !SURFACES.contains(&n.as_str()))
        .map(|(_, c)| c)
}

/// Args that can't ride the wire: streaming flags (above) and
/// clap's own help/version actions. Globals are handled separately
/// — listed once on the manifest, prepended in [`argv`], never
/// per-op (callers filter `is_global_set()` themselves).
fn skipped(a: &Arg) -> bool {
    SKIP_PARAMS.contains(&a.get_id().as_str())
        || matches!(
            a.get_action(),
            ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
        )
}

/// A clap arg → the manifest's typed param kind — `arg` =
/// positional, `opt` = `--name <v>`, `flag` = `--name` — the
/// contract `pavo-driver-kit`'s surface layer renders args from.
fn kind(a: &Arg) -> &'static str {
    if a.is_positional() {
        "arg"
    } else if matches!(
        a.get_action(),
        ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count
    ) {
        "flag"
    } else {
        "opt"
    }
}

fn param_json(a: &Arg) -> Value {
    // Wire name = the flag name (`--poll-refs` → `poll-refs`), not
    // the field id — generated surfaces spell params `--<name>`.
    let id = a.get_id().as_str();
    let name = match a.is_positional() {
        true => id,
        false => a.get_long().unwrap_or(id),
    };
    json!({
        "name": name,
        "kind": kind(a),
        "doc": a.get_help().map(|d| d.to_string()).unwrap_or_default(),
    })
}

/// The `ops` query/`ops` argv verb payload — self-describing so
/// hosts and agents discover the engine surface without argv help.
/// Globals (`--config`, `--workspace`) are accepted by every op —
/// listed once on the manifest; the child gets them as argv flags.
pub fn manifest() -> Value {
    json!({
        "engine": "kuda",
        "invoke": "query `cli.<verb>` with the listed params, or argv `tamias-driver-kuda run <kuda argv>`",
        "globals": kuda::command()
            .get_arguments()
            .filter(|a| a.is_global_set() && !skipped(a))
            .map(param_json)
            .collect::<Vec<_>>(),
        "ops": verbs()
            .iter()
            .filter(|(n, _)| !SURFACES.contains(&n.as_str()))
            .map(|(n, c)| json!({
                "query": format!("cli.{n}"),
                "doc": c.get_about().map(|d| d.to_string()).unwrap_or_default(),
                "params": c.get_arguments()
                    .filter(|a| !skipped(a) && !a.is_global_set())
                    .map(param_json)
                    .collect::<Vec<_>>(),
                "mutates": !READ_ONLY.contains(&n.as_str()),
            }))
            .collect::<Vec<_>>(),
        "surfaces": SURFACES,
    })
}

/// Is `query` an engine op (`cli.<verb>`)? Returns the verb name.
pub fn op_of(query: &str) -> Option<String> {
    query
        .strip_prefix("cli.")
        .filter(|n| op_cmd(n).is_some())
        .map(str::to_string)
}

/// A wire scalar → its argv token (numbers/bools stringify).
fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// All values of a param — arrays flatten, scalars wrap.
fn values(v: &Value) -> Vec<String> {
    match v {
        Value::Array(xs) => xs.iter().filter_map(scalar).collect(),
        other => scalar(other).into_iter().collect(),
    }
}

/// Emit `--<long> <v>` / `--<long>` for one arg when `params`
/// carries it — looked up by wire name (long flag), field id, and
/// kebab-case id, so callers may spell params either way. Flags
/// read truthiness; opts/args take every value.
fn push_param(out: &mut Vec<String>, a: &Arg, params: &Map<String, Value>) {
    let id = a.get_id().as_str();
    let long = a.get_long().unwrap_or(id);
    let kebab = id.replace('_', "-");
    let raw = params
        .get(long)
        .or_else(|| params.get(id))
        .or_else(|| params.get(kebab.as_str()));
    let Some(raw) = raw else { return };
    match kind(a) {
        "flag" => {
            if raw.as_bool().unwrap_or(false) {
                out.push(format!("--{long}"));
            }
        }
        _ => {
            for s in values(raw) {
                out.push(format!("--{long}"));
                out.push(s);
            }
        }
    }
}

/// `cli.<verb>` params → kuda argv (no argv[0] — `run` prepends
/// it). Globals go first (clap accepts them either side of the
/// verb), then the verb's named flags, then its positionals in
/// schema order.
fn argv(name: &str, cmd: &Command, params: &Map<String, Value>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for a in kuda::command().get_arguments() {
        if a.is_global_set() && !skipped(a) {
            push_param(&mut out, a, params);
        }
    }
    out.extend(name.split('.').map(String::from));
    for a in cmd.get_arguments() {
        if skipped(a) || a.is_positional() || a.is_global_set() {
            continue;
        }
        push_param(&mut out, a, params);
    }
    // `trailing_var_arg`/`last` positionals only parse after a
    // `--` separator — none today, kept generic.
    if cmd.get_positionals().any(|a| a.is_last_set()) {
        out.push("--".into());
    }
    let mut positionals: Vec<String> = Vec::new();
    for a in cmd.get_positionals() {
        if skipped(a) {
            continue;
        }
        let id = a.get_id().as_str();
        let raw = params
            .get(id)
            .or_else(|| params.get(id.replace('_', "-").as_str()));
        if let Some(raw) = raw {
            positionals.extend(values(raw));
        }
    }
    out.extend(positionals);
    out
}

/// Run one engine op against `root`: re-exec `run` in a child whose
/// cwd is the request's repo, and hand back
/// `{op, argv, code, stdout, stderr}` as the query payload.
/// `ok:false` is reserved for unknown ops and spawn failures — a
/// nonzero `code` is the engine's own exit status (e.g. `find`
/// miss), honest data. The child's stdin is null; the repo comes
/// from `root` (driver-resolved) — unbounded/host-interactive
/// verbs (`watch`/`mcp`/`acp`) are surfaces, never ops, precisely
/// because they need inherited stdio.
pub fn run_op(root: &Path, name: &str, params: &Map<String, Value>) -> Response {
    let name = name.strip_prefix("cli.").unwrap_or(name);
    let Some(cmd) = op_cmd(name) else {
        return Response::err(format!("unknown op: {name}"));
    };
    let args = argv(name, &cmd, params);
    let out = std::env::current_exe().and_then(|exe| {
        std::process::Command::new(exe)
            .arg("run")
            .args(&args)
            .current_dir(root)
            .output()
    });
    let data = match out {
        Ok(o) => json!({
            "op": name,
            "argv": args,
            "code": o.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&o.stdout),
            "stderr": String::from_utf8_lossy(&o.stderr),
        }),
        Err(e) => return Response::err(format!("run {name}: {e}")),
    };
    Response::ok_typed(&QueryResult {
        query: format!("cli.{name}"),
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
        assert_eq!(op_of("cli.sync").unwrap(), "sync");
        assert_eq!(op_of("cli.doctor").unwrap(), "doctor");
        assert!(op_of("cli.watch").is_none()); // unbounded surface
        assert!(op_of("cli.mcp").is_none()); // stdio surface
        assert!(op_of("cli.acp").is_none()); // stdio surface
        assert!(op_of("cli.gen-cli-docs").is_none()); // hidden plumbing
        assert!(op_of("sync").is_none()); // view query, not an op
        assert!(op_of("cli.bogus").is_none());
    }

    #[test]
    fn manifest_lists_every_non_surface_verb_as_cli_query() {
        let m = manifest();
        let ops = m["ops"].as_array().unwrap();
        for (n, _) in verbs()
            .iter()
            .filter(|(n, _)| !SURFACES.contains(&n.as_str()))
        {
            assert!(
                ops.iter().any(|o| o["query"] == json!(format!("cli.{n}"))),
                "{n} missing from ops manifest"
            );
        }
        assert_eq!(m["surfaces"], json!(SURFACES));
        for o in ops {
            for p in o["params"].as_array().unwrap() {
                assert!(["arg", "opt", "flag"].contains(&p["kind"].as_str().unwrap()));
                // streaming/stdin params never ride the wire
                assert!(!SKIP_PARAMS.contains(&p["name"].as_str().unwrap()));
                // globals are manifest-level, never per-op params
                assert!(!m["globals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|g| g["name"] == p["name"]));
            }
        }
        // mutates honesty — the side-effect set is exactly these
        let mutators: Vec<&str> = ops
            .iter()
            .filter(|o| o["mutates"] == json!(true))
            .filter_map(|o| o["query"].as_str())
            .collect();
        assert_eq!(mutators, vec!["cli.init", "cli.sync"]);
    }

    #[test]
    fn argv_maps_params_to_kuda_flags() {
        let cmd = op_cmd("sync").unwrap();
        let mut p = Map::new();
        p.insert("full".into(), json!(true));
        p.insert("workspace".into(), json!(true));
        assert_eq!(
            argv("sync", &cmd, &p),
            vec!["--workspace", "sync", "--full"]
        );
    }

    #[test]
    fn argv_maps_positionals_and_named_params() {
        let cmd = op_cmd("card").unwrap();
        let mut p = Map::new();
        p.insert("module".into(), json!("kuda-store"));
        assert_eq!(argv("card", &cmd, &p), vec!["card", "kuda-store"]);

        let cmd = op_cmd("refs").unwrap();
        let mut p = Map::new();
        p.insert("name".into(), json!("X"));
        p.insert("docs".into(), json!(true));
        // named flags emit before positionals — clap accepts
        // interspersed args either way.
        assert_eq!(argv("refs", &cmd, &p), vec!["refs", "--docs", "X"]);
    }

    #[test]
    fn argv_carries_globals_and_config_opt() {
        let cmd = op_cmd("status").unwrap();
        let mut p = Map::new();
        p.insert("config".into(), json!("/tmp/kuda.toml"));
        p.insert("workspace".into(), json!(true));
        assert_eq!(
            argv("status", &cmd, &p),
            vec!["--config", "/tmp/kuda.toml", "--workspace", "status"]
        );
    }

    #[test]
    fn argv_omits_absent_params() {
        let cmd = op_cmd("status").unwrap();
        assert_eq!(argv("status", &cmd, &Map::new()), vec!["status"]);
    }
}
