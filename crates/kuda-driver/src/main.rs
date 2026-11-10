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
//! Optional surfaces: features `cli`/`mcp` embed pavo-driver-kit's
//! surface layer — `cli <op> [args]` is a generated CLI over the ops
//! manifest (`watch`/`mcp`/`acp` ride as streaming subcommands),
//! `mcp` serves the ops as MCP tools on stdio. Both are equivalent
//! to the standalone `tamias-driver-cli`/`tamias-driver-mcp` surface
//! drivers, minus the PATH/discovery hop.
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
#[cfg(any(feature = "cli", feature = "mcp"))]
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
            std::process::exit(kuda::dispatch(&kuda::parse_from(argv)));
        }
        // Embedded surface verbs (feature-gated, optional): the
        // pavo-driver-kit surface layer renders this driver's own
        // ops manifest — `cli` = generated clap CLI (op → subcommand,
        // `cli watch` streams), `mcp` = ops → MCP tools over stdio.
        // The standalone tamias-driver-cli/-mcp binaries do the same
        // over process dispatch; these verbs need nothing on PATH.
        #[cfg(feature = "cli")]
        Some("cli") => {
            let m = kit::surface::OpsManifest::from_value(&ops::manifest()).unwrap_or_default();
            let argv: Vec<String> = std::iter::once("kuda".to_string())
                .chain(args.iter().skip(1).cloned())
                .collect();
            kit::surface::cli::run("kuda", &m, &argv, &mut surface::Engine)
        }
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
                "usage: tamias-driver-kuda manifest|capabilities|ops|run <argv>|serve\n       (feature `cli` adds `cli <op> [args]`; feature `mcp` adds `mcp`)"
            );
            1
        }
    };
    std::process::exit(code);
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
                ops::run_op(&resolve_root(params), op, params)
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
