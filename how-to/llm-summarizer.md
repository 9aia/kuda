---
url: /kuda/how-to/llm-summarizer.md
---
# How-to: summarizers

`kuda-cards` turns extractions into the committed YAML artifacts.
Two shipping adapters: `template` (deterministic) and `acp`
(agent-written prose); `llm` is parked — ACP is the semantic path.

## `acp` — agent-written cards

```toml
[summarize]
kind = "acp"
acp_cmd = "env CLINE_PROVIDER=cline-pass CLINE_MODEL=cline-free/mimo-v2.6-flash cline --acp"
# NOTE: provider/model are env vars, not CLI flags — -P/-m are
# ignored in ACP mode.
```

`sync` spawns the agent once per run (one session, reused across
modules) and sends each module a session/prompt asking for
`purpose:`/`invariants:`/`notes:` grounded in the file list. The
agent's prose fills the card's `purpose` + `notes` regions —
`api`/`deps`/`verify`/`provenance` stay mechanical.

Protocol notes:

* session/request_permission from the agent is auto-answered
  with the first option — required, or the prompt deadlocks
* Any ACP-capable agent works: `cline --acp`, `zed agent`,
  `gemini --experimental-acp`, …
* Auth lives in the agent — kuda holds no keys
* Agent failure → silent `template` fallback per module

## `template` (default, shipping)

Deterministic, zero-model. Each module card's `purpose` comes
from the manifest `description` field (Cargo.toml
`[package].description`, package.json `description`); `api` lists
the extracted public symbols; `deps` lists the module's edges;
`verify:` lists the files the card was built from.

Because it is mechanical, `template` output is stable — the same
inputs always produce the same card, so diffs mean real change.
That is the property you want in committed, reviewed artifacts.

## `llm` (parked)

A direct-provider adapter (`provider`/`model` + keyring) is in
the backlog — ACP is preferred because it is agent-agnostic and
key-free. Same grounding rules apply when it lands: cite files or
stay silent, never clobber `verify:` or human `notes:`.

## Incremental sync

`sync` compares each module's input-file hashes with the committed
card's `provenance.hashes` before calling the summarizer — identical
means the card is reused and the agent is never prompted. Change one
file and only that module re-summarizes; the agent spend is
per-drift, not per-sync.

## File-tier cards

`file_cards = true` under `[summarize]` adds an L1.5 level: every
source file gets a `.kuda/files/<module>/<file>.yaml` card. Files
are split into ~1.5k-char semantic chunks (tree-sitter aware via
text-splitter), budgeted to ~14k chars per prompt, and each card
carries its own blob hash — an unchanged file never re-prompts.
