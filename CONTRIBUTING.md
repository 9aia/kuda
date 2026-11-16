# Contributing

Issues and PRs are **disabled** — the project is agent-maintained and
dogfoods its own CI. Fork freely under MIT.

## Layout

`crates/` — rust workspace. `kuda-core` is pure domain (cards,
symbols, edges, provenance); ports are crates: `kuda-extract`,
`kuda-summarize`, `kuda-store`, `kuda-secret`, `kuda-config`;
adapters: `kuda-extract-manifest`/`kuda-extract-treesitter`/
`kuda-extract-lsp`/`kuda-records` (extractors),
`kuda-template`/`kuda-acp` (summarizers — `llm` is a parked
backlog kind), `kuda-store-sqlite` (store),
`kuda-keyring`/`kuda-dotenv` (secrets), `kuda-query`/`kuda-cards`
(agent-facing reads + `.kuda/` card render). Git goes through the
shared `lasius-vcs` port. `kuda-cli` (package `kuda`) is the engine
lib — no `[[bin]]`: the `kuda` command ships as the tamias pack in
`libraries/kuda` (`bins[]` shim → `tamias-driver-kuda cli` →
`kuda::run_from`, T-012). Engine e2e tests live in
`crates/kuda-driver/tests/` and spawn `tamias-driver-kuda run`.
`libraries/kuda/` is the pack; `scripts/pack.sh` stages the
compiled `tamias-driver-kuda` payload into `drivers/bin/<triple>/`
(gitignored build artifact).
`modules/dagger/` is the shared pipeline module (Python SDK).

## Rules of thumb

- New extractor/summarizer/store = a new adapter crate, not a core
  change. New language = a `data/langs/` file, not an engine edit.
- `cargo build --workspace` and `cargo test --workspace` must stay green
  (unit tests + the engine e2e in `crates/kuda-driver/tests/` —
  no docker needed).
- Coverage: `scripts/coverage.sh` — cargo-llvm-cov HTML report in
  `target/llvm-cov/html`; pass `--fail-under-lines 100` to gate.
- Keep the binary dependency-light and sync; no async runtime.
- Commits: conventional prefixes (`feat:`/`fix:`/`refactor:`/`docs:`/`chore:`).
- Never commit secrets — `kuda.toml` takes names of env vars, not values.

## Releasing

Releases are run manually by an operator on `main` via
`tarsi-serve release --dir . --yes` directly (the org release
plane, project ADR-0005; repo config: `tarsi.toml` — the per-repo
`scripts/release.sh` shims were removed org-wide): plan →
changelog → bump → tag → push. There is **no crates.io publish**: org policy keeps every repo but
tamias off crates.io, and the kuda crates are internal lib-only
engines behind `tamias-driver-kuda`. User-facing installs ride the
tamias pack — after the tag, the operator runs `scripts/pack.sh`
then `tamias publish libraries/kuda` to ship `9aia:kuda@<ver>` to
the org registry. See `docs/explanation/release-model.md`.

## Docs

Follow Diátaxis: `docs/tutorials/` (learn) · `docs/how-to/` (tasks) ·
`docs/reference/` (lookup) · `docs/explanation/` (why). API reference
is generated from doc comments with
docs codegen: `cypera gen` (driven by `cypera.toml`) renders cli/config/crate-map from source —
keep `lib.rs` module docs accurate; they feed the generated docs.
