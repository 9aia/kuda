# kuda-driver — `tamias-driver-kuda`

Pavo view driver for kuda. One binary, named `tamias-driver-kuda`,
answers the pavo wire ops (`manifest`, `views`, `view`, `query`) as
ndjson JSON-RPC over stdio — the same `{"op":…}` /
`{"ok":…,"result"|"error":…}` shape every sibling `serve` speaks
(mirrored from `pavo-protocol`; the protocol crate is deliberately
not a dependency so kuda ships standalone).

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

The driver is **read-only** — it never creates `.kuda/index.sqlite`.
Per request it resolves the repo root from:

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

## Contributes

navbar → `/kuda/modules`, dashboard panel → `/kuda/drift`,
statusbar entry → `/kuda/drift`. Statusbar counts aren't dynamic —
`Contribution` carries only id/title/route, so the entry navigates
to the drift view rather than rendering "N stale" inline.

## Layout

- `src/main.rs` — argv verbs (`manifest`, `capabilities`, `serve`)
  + the ndjson dispatch loop.
- `src/repo.rs` — root resolution, `kuda.toml` config, card
  parsing, `kuda status`-equivalent drift.
- `src/queries.rs` — query-name → payload implementations.
- `src/views.rs` — the static manifest + ViewNode trees.
