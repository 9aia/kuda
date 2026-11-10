# kuda-driver — `tamias-driver-kuda`

Pavo view driver + engine-ops driver for kuda. One binary, named
`tamias-driver-kuda`, answers the pavo wire ops (`manifest`,
`views`, `view`, `query`) as ndjson JSON-RPC over stdio via
`pavo-driver-kit` — the same `{"op":…}` /
`{"ok":…,"result"|"error":…}` shape every sibling `serve` speaks.
It also links the `kuda` engine lib: the `run` argv verb and the
`cli.*` wire queries dispatch to `kuda::dispatch(Cli)` — argv in,
exit code out.

## Install / discovery

```bash
cargo install --path crates/kuda-driver   # → tamias-driver-kuda on PATH
# or: cp target/{release,debug}/tamias-driver-kuda <tamias-home>/drivers/
```

Pavo discovers drivers named `tamias-driver-*` on `PATH` or in
`<tamias-home>/drivers/` (`$TAMIAS_HOME` or
`~/.local/share/tamias/drivers`). `cargo dist` release packaging
(`dist-workspace.toml`) is the distribution path once kuda cuts a
release that includes this crate.

## Repo context

Views and plain queries are **read-only** — the driver never opens
`.kuda/index.sqlite` for writing. Engine ops are the exception by
design (`cli.sync` mutates `.kuda/` — see below). Per request it
resolves the repo root from:

1. `root` / `dir` / `path` param on the wire request,
2. `$KUDA_DRIVER_ROOT`,
3. nearest ancestor of the cwd containing `.kuda/` or `kuda.toml`
   (pavo inherits your cwd, so `pavo view kuda:graph` inside repo X
   reads X's index),
4. the cwd itself — queries then return `{"error": …}` payloads
   (partial degradation, never a crash).

## Views

| view | route | params | data |
| --- | --- | --- | --- |
| `map` | `/kuda` | — | L0 repo card kv + modules table |
| `modules` | `/kuda/modules` | — | per-card api/deps/drift table |
| `module` | `/kuda/module` | `name` | card kv + deps + api symbols |
| `graph` | `/kuda/graph` | `level=modules\|symbols`, `records=true` | dep graph (`{"nodes","edges"}`) |
| `drift` | `/kuda/drift` | — | `kuda status` drift summary + cards |
| `find` | `/kuda/find` | `pattern` | exact + substring symbol tables |

## Queries

`repo`, `modules`, `module`/`card`, `module_deps`,
`module_symbols`, `graph`, `drift`, `find`, `search`, `dependents`,
`status` — the last five map onto `kuda-query` functions over
`IndexStore`; card/drift queries read the committed `.kuda/*.yaml`
artifacts and recompute git-blob provenance hashes exactly like
`kuda status`.

## Engine ops

`ops` (argv verb or `{"op":"query","query":"ops"}`) returns the ops
manifest — the engine's verb table as JSON:

| wire query | kuda op | params | mutates |
| --- | --- | --- | --- |
| `cli.sync` | `sync` | `full`(flag), `workspace`(flag), `config`(opt) | yes |
| `cli.status` | `status` | `workspace`(flag), `config`(opt) | no |
| `cli.map` | `map` | `workspace`(flag), `config`(opt) | no |
| `cli.card` | `card` | `module`(arg), `workspace`(flag), `config`(opt) | no |
| `cli.find` | `find` | `symbol`(arg), `workspace`(flag), `config`(opt) | no |
| `cli.dependents` | `dependents` | `name`(arg), `workspace`(flag), `config`(opt) | no |
| `cli.refs` | `refs` | `name`(arg), `docs`(flag), `workspace`(flag), `config`(opt) | no |
| `cli.doctor` | `doctor` | `workspace`(flag), `config`(opt) | no |

Params are typed (`arg` = positional, `opt` = `--name [v]`, `flag` =
`--name` bool) — the contract surface drivers generate args from.

Each `cli.*` query re-execs `tamias-driver-kuda run <argv>` with the
resolved repo root as cwd — `kuda::dispatch` writes to the child's
captured stdout, so the serve loop stays a clean ndjson stream. The
payload is `{op, argv, code, stdout, stderr}`; `code` is the
engine's own exit status (`ok:false` is reserved for spawn
failures). `tamias-driver-kuda run <argv>` is also a human
passthrough — `run sync --full` ≡ `kuda sync --full`.

`watch`/`mcp`/`acp` are deliberately not ops — they never return.
They ride the manifest's `surfaces` list instead, which surface
drivers render as streaming subcommands. Two ways to reach them:

- **Standalone surface drivers** (pavo): `tamias-driver-cli kuda
  find parse`, `tamias-driver-mcp kuda` — discovery + dispatch over
  the wire, no kuda feature needed.
- **Embedded features** here: build with `--features cli,mcp` and
  `tamias-driver-kuda cli find parse` / `tamias-driver-kuda mcp`
  serve the same ops in-process. Optional, never required — the
  default build stays a view driver + `run` passthrough.

## Contributes

navbar → `/kuda/modules`, dashboard panel → `/kuda/drift`,
statusbar entry → `/kuda/drift`. Statusbar counts aren't dynamic —
`Contribution` carries only id/title/route, so the entry navigates
to the drift view rather than rendering "N stale" inline.

## Layout

- `src/main.rs` — argv verbs (`manifest`, `capabilities`, `ops`,
  `run`, `serve`, plus feature-gated `cli`/`mcp`) + the ndjson
  dispatch loop.
- `src/surface.rs` — the driver's `Surface` impl for embedded
  surfaces (feature `cli`/`mcp`): `op` → `ops::run_op`, `surface` →
  in-process `kuda::dispatch`.
- `src/ops.rs` — the engine ops manifest (typed params) + `cli.*`
  dispatch (re-exec `run`, captured output → `{code, stdout, stderr}`).
- `src/repo.rs` — root resolution, `kuda.toml` config, card
  parsing, `kuda status`-equivalent drift.
- `src/queries.rs` — query-name → payload implementations.
- `src/views.rs` — the static manifest + ViewNode trees.
