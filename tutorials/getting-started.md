---
url: /kuda/tutorials/getting-started.md
---
# Getting started

Go from a checkout to a queryable, freshness-tracked memory of the
repo — the artifact an agent loads before planning instead of
re-exploring from scratch.

## Install

The `kuda` command is a tamias pack (`libraries/kuda`): a generated
`bins[]` shim → `tamias-driver-kuda` → the engine lib. There is no
cargo-installed `kuda` binary. With `tamias` installed:

```sh
tamias install 9aia:kuda@<version>    # org registry (needs TAMIAS_REGISTRY_TOKEN)
```

Or from a checkout of this repo:

```sh
scripts/pack.sh                                  # stage the driver payload
tamias install --allow-drivers ./libraries/kuda   # deploys `kuda`
```

## 1. Init

```sh
cd your-repo
kuda init
```

This writes a commented `kuda.toml` and a `.kuda/` skeleton. Open
the toml — the defaults are fully local and deterministic:

```toml
[extract]
kinds = ["manifest", "treesitter"]   # adapter names, tried in order

[summarize]
kind = "template"                    # deterministic cards

[store]
kind = "sqlite"
path = ".kuda/index.sqlite"
```

If `rust-analyzer` is on PATH you can add `"lsp"` to `kinds` —
extraction quality tiers up automatically (see
[extraction tiers](/explanation/architecture#extraction-tiers)).

## 2. Sync

```sh
kuda sync
```

Output:

```
sync /home/you/your-repo
indexed 19 module(s) at 3432d05
```

`sync` walks manifests, runs the tree-sitter symbol queries from
`data/langs/*`, summarizes each module into a card, and stores the
symbol/edge graph in sqlite. It writes:

| path                   | tier | commit?                   |
| ---------------------- | ---- | ------------------------- |
| `.kuda/index.yaml`     | L0   | yes — the always-load card |
| `.kuda/modules/*.yaml` | L1   | yes — reviewable in diffs  |
| `.kuda/index.sqlite`   | L2   | no — rebuildable graph     |
| `.kuda/meta.json`      | meta | no — provenance record     |

## 3. Look at what it learned

```sh
kuda map
```

The L0 card: repo name, the module list, detected commands, and the
provenance block (`commit_sha`, file hashes, generator). This is the
\~1k-token blob an agent reads first.

```sh
kuda card kuda-store
```

An L1 card: purpose (from the manifest `description`), public API
symbols, dependency edges, `verify:` — the files to open before
trusting the card — and its own provenance.

## 4. Query

```sh
kuda find IndexStore
```

```
crates/kuda-store/src/lib.rs:9 Trait kuda-store
```

```sh
kuda dependents serde
```

```
kuda-cards --PackageDep--> serde
kuda-core --Imports--> serde
```

And the doc-sweep query — everything that mentions a name, useful
when a change must update code *and* docs and comments:

```sh
kuda refs SqliteStore --docs
```

## 5. Break it, then watch kuda notice

Edit any `crates/*/Cargo.toml` — change a `description` — and:

```sh
kuda status
```

```
drifted  kuda-store
19 fresh, 1 drifted (HEAD af26b8c)
```

`status` re-hashes each card's `verify:` files and compares with the
recorded provenance. `kuda sync` regenerates only what drifted.

## What you now know

* `init` scaffolds `kuda.toml` + `.kuda/`; extraction, summarization,
  and storage are config-swappable adapters.
* `sync` produces three tiers: L0 `index.yaml`, L1 `modules/*.yaml`
  (committed, reviewable), L2 `index.sqlite` (rebuildable).
* `map`/`card` read cards; `find`/`dependents`/`refs` query the
  symbol graph.
* `status` detects drift by re-hashing `verify:` files against
  recorded provenance; `sync` repairs incrementally.

You have built a committed, queryable, self-checking map of the
repository. Point agents at it — via `kuda mcp` or `kuda acp` — and
they plan against the map instead of grepping blindly.

## Where next

* **How-to** (tasks): [multi-repo](/how-to/multi-repo) ·
  [mcp](/how-to/mcp) · [acp](/how-to/acp) ·
  [llm summarizer](/how-to/llm-summarizer)
* **Reference** (lookup): [cli](/reference/cli) ·
  [config](/reference/config) · [crate api](/reference/crate-api)
* **Explanation** (why): [architecture](/explanation/architecture) ·
  [release model](/explanation/release-model) ·
  [comparison](/comparison) — vs grep / RAG
