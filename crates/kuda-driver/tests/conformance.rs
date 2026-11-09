//! Driver conformance gate — runs pavo's wire-contract suite
//! ([`pavo_testkit::conformance::check_driver`]) against this crate's
//! own `tamias-driver-kuda serve` binary.
//!
//! The driver builds its manifest/trees with `pavo-driver-kit` over
//! `pavo-protocol` types; this test remains the drift guard — if the
//! pavo driver contract moves, this fails here before it fails in
//! pavo.
//!
//! The spawned driver inherits this test's cwd (`crates/kuda-driver`,
//! inside the kuda repo), so `.kuda/` discovery exercises the real
//! repo path — hermetic, no network, no user state.

use std::path::Path;

#[test]
fn driver_passes_pavo_conformance() {
    let violations = pavo_testkit::conformance::check_driver(Path::new(env!(
        "CARGO_BIN_EXE_tamias-driver-kuda"
    )));
    assert!(
        violations.is_empty(),
        "pavo conformance violations:\n{}",
        violations
            .iter()
            .map(|v| format!("  - {v}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
