<!-- myco:agent-awareness -->

## Agent awareness

- Session start / any work discussion: run `myco status --prompt`
  before proposing next steps — know the board first.
- User describes new work → offer to capture it (`myco todo add`,
  `issue new`, `proposal new`). Don't wait to be asked.
- Picking up a task: `myco resume <ID>` is the briefing.
- Wrapping up: `myco handoff <ID>` anything resumable; keep statuses
  honest (claimed → in-progress → blocked/done).

<!-- /myco:agent-awareness -->

<!-- sturn:agent-awareness -->

## Sturn awareness

- **No PRs.** The pushed branch/commit is the deliverable; review
  streams continuously (sturn WAL), merge is a human/sturn call.
- sturn-wired repo (`sturn.toml` present): `sturn claim "<glob>"
  --ws <ws>` *before* edits — claims surface conflicts in ms.
- Ops via `sturn write|rename|delete`; `sturn snap` folds the ws
  into a real commit; `sturn push` publishes heads + refs/sturn/*.
- `sturn leave` on exit — drops the actor's claims.

<!-- /sturn:agent-awareness -->

<!-- kuda:agent-awareness -->

## Kuda awareness

- `.kuda/` cards are committed artifacts — read them before
  re-exploring (`kuda map`, `kuda card <mod>`); `kuda status`
  reports per-card drift vs HEAD.
- Code changes that invalidate a card → regenerate it (`kuda
  sync`) in the same change; cards ride the diff they describe.
- `.kuda/index.sqlite` is gitignored + rebuildable; the `*.yaml`
  cards are the checked-in source of truth.

<!-- /kuda:agent-awareness -->

# Kuda

Code-map engine — verified per-module cards agents read before
code. Named for *kuda* — repository memory: `kuda sync` crawls your
repos into a compressed, freshness-tracked index (`.kuda/` cards +
`index.sqlite`), and agents query it instead of re-exploring every
session. Cards are committed artifacts with provenance
(`{files, hashes, commit_sha}`); `kuda status` re-hashes and
reports drift.

## Layout

Flat `crates/` workspace, ports-and-adapters. `kuda-core` is pure
domain (cards, symbols, edges, provenance); ports are traits, impls
are separate crates; `kuda-cli` (package `kuda`) is the engine
lib — **no `[[bin]]`**: the `kuda` command ships as the tamias pack
in `libraries/kuda` (`bins[]` shim → `tamias-driver-kuda cli` →
`kuda::run_from`, T-012).

| Crate | Role |
| --- | --- |
| `kuda-core` | card/symbol/edge/provenance model — pure, no I/O |
| `kuda-cli` (pkg `kuda`) | the engine lib — clap, `--json`, `manifest`/`schema`, `run_from()` for drivers |
| `kuda-driver` | `tamias-driver-kuda` — pavo view driver + engine ops surface + the `cli` bins-shim passthrough; engine e2e in `tests/` |
| `kuda-extract` + `-manifest`/`-treesitter`/`-lsp` | extraction port + impls — manifest → treesitter → rust-analyzer `documentSymbol`; best available wins |
| `kuda-records` | records-corpus extractor — `data/domains/*` descriptors turn task/ADR/ledger ids into symbols + reference edges |
| `kuda-summarize` + `-template`/`-acp` | card-prose port + impls — deterministic template (default) or spawn an ACP agent (`zed agent`, `claude`, …); the `llm` kind is parked |
| `kuda-store` + `-sqlite` | index port + impl — `index.sqlite` (gitignored, rebuildable) |
| `kuda-query` | agent-facing reads over `IndexStore` — `find`/`dependents`/`refs`/`map` |
| `kuda-cards` | domain types ↔ committed `.kuda/*.yaml` card render |
| `kuda-config` | `kuda.{yaml,yml,toml}` loading via `scarab-config-file` discovery |
| `kuda-secret` + `-keyring`/`-dotenv` | secrets port + impls — no shipped consumer today (`llm` summarizer parked); `template`/`acp` need none |
| `data/langs/` | per-language `LangDef` data — a new language is a data file, not an engine edit |
| `data/domains/` | per-domain `DomainDef` record-corpus descriptors |
| `libraries/kuda` | the tamias pack — `bins[]` shim + driver payload |
| `modules/dagger/` | shared pipeline module (Python SDK) |
| `scripts/` | `pack.sh`, `coverage.sh` |
| `presentation/` | slide deck (pnpm) |

Git goes through the shared `lasius-vcs` port.

## Commands

No xtask — plain cargo + scripts:

```bash
cargo build --workspace && cargo test --workspace   # unit + e2e
                                               # (e2e spawn tamias-driver-kuda)
scripts/coverage.sh            # llvm-cov HTML → target/llvm-cov/html;
                               # extra args pass through (--fail-under-lines 100)
scripts/pack.sh                # release tamias-driver-kuda →
                               # libraries/kuda/drivers/bin/<triple>/
tarsi-serve release --dir . --yes
                               # operator-run: version bump + tag
                               # (dry-run without --yes)
```

Deploy the command: `scripts/pack.sh` then `tamias install
--allow-drivers ./libraries/kuda`.

## Boundary

kuda owns **code memory** — extraction, cards, the symbol/dep
index. Neighbors:

- **mimus** — the critic; verifies card claims (`mimus audit`),
  owns checkers/rules/`fix`.
- **myco** — work memory; kuda indexes code, not work items.
- **tamias** — distribution; kuda ships as a `type: kuda`-style
  pack + `kuda-domain` content packs.
- **sturn** — ephemeral coordination (`sturn` source for sync
  watch, `--poll-refs`).
- **scarab** — config expertise; `kuda-config` is scarab-config
  wiring, not a new loader.
- **lasius** — git expertise; all git access via `lasius-vcs`.
- **pavo** — the view host `tamias-driver-kuda` plugs into.

## Docs

`docs/` worktree — VitePress + Diátaxis (tutorials/how-to/reference/
explanation) → GitHub Pages at `9aia.com/kuda`; plus `comparison.md`.
`cypera gen` (driven by `cypera.toml`) renders cli/config/crate-map from source — keep
`lib.rs` module docs accurate.

## Specs

`specs/<name>.toml` is this repo's mimus-standard conformance spec —
approval-gated. Any change requires a live org proposal
(`proposal-NNNN`, status open|done) referenced in the spec header or
commit range, and no coverage regression vs merge-base. Guard:
workspace `../../scripts/spec-guard.py`; sweep:
`../../scripts/standard.sh`; regenerate coverage only via
`../../scripts/gen-repo-specs.py`.
