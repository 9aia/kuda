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
| `cli.sync` | `sync` | `full`, `workspace` | yes |
| `cli.status` | `status` | — | no |
| `cli.map` | `map` | — | no |
| `cli.card` | `card` | `module` | no |
| `cli.find` | `find` | `symbol`, `workspace` | no |
| `cli.dependents` | `dependents` | `name`, `workspace` | no |
| `cli.refs` | `refs` | `name`, `docs`, `workspace` | no |
| `cli.doctor` | `doctor` | — | no |

Each `cli.*` query re-execs `tamias-driver-kuda run <argv>` with the
resolved repo root as cwd — `kuda::dispatch` writes to the child's
captured stdout, so the serve loop stays a clean ndjson stream. The
payload is `{op, argv, code, stdout, stderr}`; `code` is the
engine's own exit status (`ok:false` is reserved for spawn
failures). `tamias-driver-kuda run <argv>` is also a human
passthrough — `run sync --full` ≡ `kuda sync --full`.

`watch`/`mcp`/`acp` are deliberately not ops — they never return and
belong to surface drivers (`tamias-driver-cli` / `-mcp`, follow-up).

## Contributes

navbar → `/kuda/modules`, dashboard panel → `/kuda/drift`,
statusbar entry → `/kuda/drift`. Statusbar counts aren't dynamic —
`Contribution` carries only id/title/route, so the entry navigates
to the drift view rather than rendering "N stale" inline.

## Layout

- `src/main.rs` — argv verbs (`manifest`, `capabilities`, `ops`,
  `run`, `serve`) + the ndjson dispatch loop.
- `src/ops.rs` — the engine ops manifest + `cli.*` dispatch (re-exec
  `run`, captured output → `{code, stdout, stderr}`).
- `src/repo.rs` — root resolution, `kuda.toml` config, card
  parsing, `kuda status`-equivalent drift.
- `src/queries.rs` — query-name → payload implementations.
- `src/views.rs` — the static manifest + ViewNode trees.
