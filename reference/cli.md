---
url: /kuda/reference/cli.md
---
# Command-Line Help for `kuda`

This document contains the help content for the `kuda` command-line program.

**Command Overview:**

* [`kuda`↴](#kuda)
* [`kuda init`↴](#kuda-init)
* [`kuda sync`↴](#kuda-sync)
* [`kuda status`↴](#kuda-status)
* [`kuda map`↴](#kuda-map)
* [`kuda card`↴](#kuda-card)
* [`kuda find`↴](#kuda-find)
* [`kuda dependents`↴](#kuda-dependents)
* [`kuda refs`↴](#kuda-refs)
* [`kuda watch`↴](#kuda-watch)
* [`kuda mcp`↴](#kuda-mcp)
* [`kuda acp`↴](#kuda-acp)
* [`kuda doctor`↴](#kuda-doctor)

## `kuda`

A hippocampus for your codebase — repo memory: index, query, verify

**Usage:** `kuda [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `init` — Detect languages -> kuda.toml + .kuda/ scaffold
* `sync` — Incremental index (hash -> extract -> summarize -> store)
* `status` — Provenance vs HEAD: per-card drift report
* `map` — Render the L0 repo card (the always-load blob)
* `card` — Module card + freshness
* `find` — Symbol -> file:line
* `dependents` — Reverse dep lookup — impact analysis
* `refs` — Code + docs/comments mentions — the doc-sweep query
* `watch` — Watch the repo — fs events trigger incremental syncs (debounced; only changed modules re-summarize)
* `mcp` — Serve the index over MCP (stdio)
* `acp` — Speak ACP — Agent Client Protocol surface for any agent client (zed, cline, cursor)
* `doctor` — Extractor/store/config health

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml

## `kuda init`

Detect languages -> kuda.toml + .kuda/ scaffold

**Usage:** `kuda init`

## `kuda sync`

Incremental index (hash -> extract -> summarize -> store)

**Usage:** `kuda sync [OPTIONS]`

###### **Options:**

* `--full` — Rebuild everything, ignoring content-hash freshness

## `kuda status`

Provenance vs HEAD: per-card drift report

**Usage:** `kuda status`

## `kuda map`

Render the L0 repo card (the always-load blob)

**Usage:** `kuda map`

## `kuda card`

Module card + freshness

**Usage:** `kuda card <MODULE>`

###### **Arguments:**

* `<MODULE>`

## `kuda find`

Symbol -> file:line

**Usage:** `kuda find <SYMBOL>`

###### **Arguments:**

* `<SYMBOL>`

## `kuda dependents`

Reverse dep lookup — impact analysis

**Usage:** `kuda dependents <NAME>`

###### **Arguments:**

* `<NAME>`

## `kuda refs`

Code + docs/comments mentions — the doc-sweep query

**Usage:** `kuda refs [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>`

###### **Options:**

* `--docs` — Only docs/comments/markdown mentions

## `kuda watch`

Watch the repo — fs events trigger incremental syncs (debounced; only changed modules re-summarize)

**Usage:** `kuda watch`

## `kuda mcp`

Serve the index over MCP (stdio)

**Usage:** `kuda mcp`

## `kuda acp`

Speak ACP — Agent Client Protocol surface for any agent client (zed, cline, cursor)

**Usage:** `kuda acp`

## `kuda doctor`

Extractor/store/config health

**Usage:** `kuda doctor`

&#xA;This document was generated automatically by
clap-markdown.&#xA;
