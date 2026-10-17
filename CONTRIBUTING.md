# Contributing

Issues and PRs are **disabled** — the project is agent-maintained and
dogfoods its own CI. Fork freely under MIT.

## Layout

`crates/` — rust workspace. `kuda-core` is pure domain (cards,
symbols, edges, provenance); ports are crates: `kuda-extract`,
`kuda-summarize`, `kuda-store`, `kuda-vcs`; impls:
`kuda-manifest`/`kuda-treesitter`/`kuda-lsp` (extractors),
`kuda-template`/`kuda-llm` (summarizers), `kuda-store-sqlite`
(store), `kuda-git` (vcs); `kuda` (cli) is the composition root.
`modules/dagger/` is the shared pipeline module (Python SDK).

## Rules of thumb

- New extractor/summarizer/store = a new impl crate, not a core
  change. New language = a `data/langs/` file, not an engine edit.
- `cargo build --workspace` and `cargo test --workspace` must stay green
  (unit tests + the e2e in `crates/kuda-cli/tests/` — no docker needed).
- Keep the binary dependency-light and sync; no async runtime.
- Commits: conventional prefixes (`feat:`/`fix:`/`refactor:`/`docs:`/`chore:`).
- Never commit secrets — `kuda.toml` takes names of env vars, not values.

## Releasing

Seed once with `kuda deploy release` (publishes workspace crates,
tags `{repo}-v{ver}`). Afterwards `scripts/release.sh` runs on each
`main` push via kuda's own dogfood loop — bump, publish, tag, dist
binaries. See `docs/explanation/release-model.md`.

## Docs

Follow Diátaxis: `docs/tutorials/` (learn) · `docs/how-to/` (tasks) ·
`docs/reference/` (lookup) · `docs/explanation/` (why). API reference
is generated from doc comments with
docs codegen: `docs/gen-*.py` renders cli/config/crate-map from source —
keep `lib.rs` module docs accurate; they feed the generated docs.
