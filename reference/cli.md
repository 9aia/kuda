---
url: /kuda/reference/cli.md
---
# Command-Line Help for `kuda`

This document contains the help content for the `kuda` command-line program.

A hippocampus for your codebase — repo memory: index, query, verify

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

###### **Commands:**

* `init` — Detect languages -> kuda.toml + .kuda/ scaffold
* `sync` — Incremental index (hash -> extract -> summarize -> store)
* `status` — Provenance vs HEAD: per-card drift report
* `map` — Render the L0 repo card (the always-load blob)
* `card` — Module card + freshness
* `find` — Symbol -> file:line
* `dependents` — Reverse dep lookup — impact analysis
* `refs` — Code + docs/comments mentions — the doc-sweep query
* `watch` — Watch the repo — fs events trigger incremental syncs (debounced; only changed modules re-summarize). Domain corpus roots are watched explicitly, so record writes (myco verbs, doc edits) re-index like code writes
* `mcp` — Serve the index over MCP (stdio)
* `acp` — Speak ACP — Agent Client Protocol surface for any agent client (zed, cline, cursor)
* `doctor` — Extractor/store/config health

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help
* `-V, --version` — Print version

## `kuda init`

Detect languages -> kuda.toml + .kuda/ scaffold

**Usage:** `kuda init [OPTIONS]`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda sync`

Incremental index (hash -> extract -> summarize -> store)

**Usage:** `kuda sync [OPTIONS]`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--full` — Rebuild everything, ignoring content-hash freshness
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda status`

Provenance vs HEAD: per-card drift report

**Usage:** `kuda status [OPTIONS]`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda map`

Render the L0 repo card (the always-load blob)

**Usage:** `kuda map [OPTIONS]`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda card`

Module card + freshness

**Usage:** `kuda card [OPTIONS] <MODULE>`

###### **Arguments:**

* `<MODULE>`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda find`

Symbol -> file:line

**Usage:** `kuda find [OPTIONS] <SYMBOL>`

###### **Arguments:**

* `<SYMBOL>`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda dependents`

Reverse dep lookup — impact analysis

**Usage:** `kuda dependents [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda refs`

Code + docs/comments mentions — the doc-sweep query

**Usage:** `kuda refs [OPTIONS] <NAME>`

###### **Arguments:**

* `<NAME>`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--docs` — Only docs/comments/markdown mentions
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda watch`

Watch the repo — fs events trigger incremental syncs (debounced; only changed modules re-summarize). Domain corpus roots are watched explicitly, so record writes (myco verbs, doc edits) re-index like code writes

**Usage:** `kuda watch [OPTIONS]`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--poll-refs <SECS>` — Also poll non-tree domain sources every N seconds. Git refs (refs/sturn/\*) produce no fs events a watcher can use — a moved/new/deleted ref fingerprint triggers the same debounced sync
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda mcp`

Serve the index over MCP (stdio)

**Usage:** `kuda mcp [OPTIONS]`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda acp`

Speak ACP — Agent Client Protocol surface for any agent client (zed, cline, cursor)

**Usage:** `kuda acp [OPTIONS]`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help

## `kuda doctor`

Extractor/store/config health

**Usage:** `kuda doctor [OPTIONS]`

###### **Options:**

* `--config <CONFIG>` — Config path (else $KUDA_CONFIG, ./kuda.toml, ~/.config/kuda/kuda.toml)
* `--workspace` — Workspace mode — all roots in .kuda/workspace.yaml
* `-h, --help` — Print help
