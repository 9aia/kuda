//! Embedded surface dispatch — the driver serving its own engine's
//! ops through `pavo-driver-kit`'s surface layer, no second binary
//! needed. `cli`/`mcp` argv verbs (feature-gated) plug this
//! [`Surface`] impl into `surface::cli::run` / `surface::mcp::serve_stdio`.
//!
//! `op` reuses the same `cli.*` path wire queries take (`ops::run_op`
//! re-execs `run` in the request's repo — output stays captured off
//! the serve stream). `surface` dispatches `watch`/`mcp`/`acp`
//! in-process: those verbs are unbounded by design, so argv goes
//! straight to `kuda::dispatch` with stdio streaming.

use pavo_driver_kit::surface::{OpOutcome, OpSpec, Surface};
use serde_json::{Map, Value};

use crate::ops;
use crate::repo::resolve_root;

/// kuda's [`Surface`] — engine dispatch, already linked.
pub struct Engine;

impl Surface for Engine {
    /// Same path as the `cli.<op>` wire query: re-exec `run` in the
    /// resolved repo, captured output → `OpOutcome`.
    fn op(&mut self, op: &OpSpec, params: &Map<String, Value>) -> OpOutcome {
        OpOutcome::from_response(&ops::run_op(&resolve_root(params), &op.name, params))
    }

    /// `kuda <name> <args>` in-process — unbounded surfaces stream
    /// until they stop themselves (signals), which is exactly why
    /// they're surfaces and not ops.
    fn surface(&mut self, name: &str, args: &[String]) -> i32 {
        let argv = std::iter::once("kuda".to_string())
            .chain(std::iter::once(name.to_string()))
            .chain(args.iter().cloned());
        kuda::dispatch(&kuda::parse_from(argv))
    }
}
