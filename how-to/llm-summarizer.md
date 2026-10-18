# How-to: summarizers

`kuda-cards` turns extractions into the committed YAML artifacts.
Two shipping adapters: `template` (deterministic) and `acp`
(agent-written prose); `llm` is parked — ACP is the semantic path.

## `acp` — agent-written cards

```toml
[summarize]
kind = "acp"
acp_cmd = "cline --acp -P cline-pass -m cline-free/mimo-v2.6-flash"
```

`sync` spawns the agent once per run (one session, reused across
modules) and sends each module a session/prompt asking for
`purpose:`/`invariants:`/`notes:` grounded in the file list. The
agent's prose fills the card's `purpose` + `notes` regions —
`api`/`deps`/`verify`/`provenance` stay mechanical.

Protocol notes:

- session/request_permission from the agent is auto-answered
  with the first option — required, or the prompt deadlocks
- Any ACP-capable agent works: `cline --acp`, `zed agent`,
  `gemini --experimental-acp`, …
- Auth lives in the agent — kuda holds no keys
- Agent failure → silent `template` fallback per module

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
