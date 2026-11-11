//! tamias-driver-kuda — pavo view driver for kuda.
//!
//! Answers the pavo wire ops (`manifest`, `views`, `view`, `query`)
//! as ndjson JSON-RPC over stdio via `pavo-driver-kit`'s serve loop
//! — the same `{"op":…}` / `{"ok":…,"result"|"error":…}` shape every
//! sibling `serve` speaks (see `pavo/crates/pavo-protocol/src/wire.rs`).
//! Also answers the discovery verbs `manifest` and `capabilities` as
//! plain argv, plus `ops` (the engine ops manifest) and `run`
//! (engine passthrough).
//!
//! `cli <kuda argv>` is the tamias `bins[]` shim verb: `tamias
//! bin exec` calls it with the standard `--dir`/`--name` driver
//! flags ahead of the user's argv; they're stripped and the rest
//! runs through `kuda::run_from` in-process with inherited
//! stdio — full engine parity (`watch` streams, `mcp`/`acp` serve
//! stdio, globals/`--help` all work).
//!
//! Optional surfaces: feature `mcp` embeds pavo-driver-kit's
//! surface layer — `mcp` serves the ops as MCP tools on stdio,
//! equivalent to the standalone `tamias-driver-mcp` surface
//! driver minus the PATH/discovery hop. There is no `cli`
//! feature: `cli` is the shim verb above, not a generated
//! surface — the engine's own argv grammar is the contract.
//!
//! Repo context: views/queries are read-only — the driver never
//! opens `.kuda/index.sqlite` for writing. Engine ops are the
//! exception by design: `cli.<op>` wire queries re-exec the `run`
//! verb in the request's repo and let `kuda::dispatch` own the
//! outcome (`cli.sync` mutates `.kuda/`). The repo resolves from
//! the `root`/`dir`/`path` request param, `$KUDA_DRIVER_ROOT`, or
//! by walking up from the cwd (pavo inherits the user's cwd, so
//! `pavo view kuda:graph` inside repo X reads X's `.kuda/`).
//!
//! Install: `cargo install --path crates/kuda-driver` places
//! `tamias-driver-kuda` on PATH — pavo discovers it there or under
//! `<tamias-home>/drivers/`.

mod ops;
mod queries;
mod repo;
#[cfg(feature = "mcp")]
mod surface;
mod views;

use pavo_driver_kit as kit;
use pavo_protocol::wire::{QueryResult, Request, Response, ViewResult, ViewSummary};
use pavo_protocol::DriverManifest;
use queries::run_query;
use repo::{resolve_root, Ctx};
use serde_json::{json, Map, Value};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("manifest") => {
            println!("{}", kit::manifest_json(&views::manifest()));
            0
        }
        Some("capabilities") => {
            let mut v = kit::capabilities_value(&views::manifest());
            v["driver"] = json!("kuda");
            println!("{v}");
            0
        }
        Some("ops") => {
            println!("{}", ops::manifest());
            0
        }
        // Engine passthrough — `tamias-driver-kuda run sync --full`
        // is `kuda sync --full`. Also the child side of `cli.*` wire
        // queries (ops::run_op re-execs this same verb).
        Some("run") => {
            let argv = std::iter::once("kuda".to_string()).chain(args.iter().skip(1).cloned());
            std::process::exit(kuda::run_from(argv));
        }
        // The tamias `bins[]` shim verb (T-012): `tamias bin exec`
        // runs `<driver> cli --dir <payload> --name <pkg> <kuda
        // argv>` for a declared `shim: cli` bin. kuda's engine IS
        // the CLI, so this is a passthrough — strip the two standard
        // driver flags and hand the rest to `kuda::run_from` with
        // inherited stdio. That keeps full engine parity a generated
        // ops→CLI surface can't express: globals before the verb
        // (`--config`/`--workspace`), `--help`, the unbounded
        // `watch` debounced loop, and the `mcp`/`acp` stdio servers
        // all behave exactly like the retired `kuda` binary.
        Some("cli") => {
            let argv = engine_argv(&args[1..]);
            std::process::exit(kuda::run_from(argv));
        }
        // Optional embedded surface (feature-gated): the
        // pavo-driver-kit surface layer renders this driver's own
        // ops manifest as MCP tools over stdio — the same ops the
        // `cli.*` wire queries expose.
        #[cfg(feature = "mcp")]
        Some("mcp") => {
            let m = kit::surface::OpsManifest::from_value(&ops::manifest()).unwrap_or_default();
            match kit::surface::mcp::serve_stdio(&m, &mut surface::Engine) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("tamias-driver-kuda mcp: {e}");
                    1
                }
            }
        }
        Some("serve") | None => serve(),
        _ => {
            eprintln!(
                "usage: tamias-driver-kuda manifest|capabilities|ops|run <argv>|cli <kuda argv>|serve\n       (feature `mcp` adds `mcp`)"
            );
            1
        }
    };
    std::process::exit(code);
}

/// `cli` verb argv (after the `cli` token) → engine argv (argv[0]
/// included). `tamias bin exec`/`driver exec` prepend the standard
/// driver flags — `--dir <payload>` `--name <pkg>` — which the
/// engine doesn't know; consume the leading run of those two
/// (`--flag v` and `--flag=v` forms) and pass everything else
/// through untouched. `--config`/`--workspace` are deliberately NOT
/// stripped: they're kuda globals, so a user-passed one must reach
/// the engine's own parse.
fn engine_argv(args: &[String]) -> Vec<String> {
    let mut out = vec!["kuda".to_string()];
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--dir" || a == "--name" {
            // flag + separate value — skip both (a trailing flag
            // with no value just ends the scan).
            i = (i + 2).min(args.len());
        } else if a.starts_with("--dir=") || a.starts_with("--name=") {
            i += 1;
        } else {
            break;
        }
    }
    out.extend(args[i..].iter().cloned());
    out
}

/// One parsed wire [`Request`] → response. The kit's serve loop owns
/// the ndjson plumbing and turns parse failures into `ok:false`;
/// ops this driver doesn't implement (`render`, `submit`, `jobs`,
/// `notifications`) get the same `ok:false` convention every
/// non-provider driver answers.
fn dispatch(req: &Request, manifest: &DriverManifest) -> Response {
    match req {
        Request::Manifest => Response::ok_typed(manifest),
        Request::Views => Response::ok_typed(&views_summaries(manifest)),
        Request::View { view, params } => view_result(manifest, view, params),
        Request::Query { query, params } => {
            if query == "ops" {
                Response::ok_typed(&QueryResult {
                    query: query.clone(),
                    data: ops::manifest(),
                })
            } else if let Some(op) = ops::op_of(query) {
                ops::run_op(&resolve_root(params), &op, params)
            } else {
                query_result(query, params)
            }
        }
        Request::Render { .. } => Response::err("unsupported op: render"),
        Request::Submit { .. } => Response::err("unsupported op: submit"),
        Request::Jobs => Response::err("unsupported op: jobs"),
        Request::Notifications => Response::err("unsupported op: notifications"),
    }
}

/// `Vec<ViewSummary>` — id/title/route/description/queries, no trees.
fn views_summaries(manifest: &DriverManifest) -> Vec<ViewSummary> {
    manifest.views.iter().map(ViewSummary::from).collect()
}

/// `view` — resolve one ViewSpec, run every query binding against a
/// fresh `Ctx` for the request's repo, return `ViewResult`.
fn view_result(manifest: &DriverManifest, id: &str, params: &Map<String, Value>) -> Response {
    let Some(spec) = manifest.views.iter().find(|v| v.id == id) else {
        return Response::err(format!("unknown view: {id}"));
    };
    let spec = spec.clone();
    let ctx = Ctx::open(resolve_root(params));
    let mut data = Map::new();
    for b in &spec.queries {
        // static binding params first, caller params override
        let mut merged = b.params.clone();
        for (k, v) in params {
            merged.insert(k.clone(), v.clone());
        }
        data.insert(b.name.clone(), run_query(&ctx, &b.name, &merged));
    }
    Response::ok_typed(&ViewResult { view: spec, data })
}

/// `query` — one named query with caller params.
fn query_result(name: &str, params: &Map<String, Value>) -> Response {
    let ctx = Ctx::open(resolve_root(params));
    let data = run_query(&ctx, name, params);
    Response::ok_typed(&QueryResult {
        query: name.to_string(),
        data,
    })
}

fn serve() -> i32 {
    let manifest = views::manifest();
    match kit::serve_stdio(|req| dispatch(req, &manifest)) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("tamias-driver-kuda: {e}");
            1
        }
    }
}
