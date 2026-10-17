---
url: /kuda/how-to/llm-summarizer.md
---
# How-to: summarizers

`kuda-cards` turns extractions into the committed YAML artifacts.
The `summarize` port has one shipping adapter — `template` — and
one planned — `llm`.

## `template` (default, shipping)

Deterministic, zero-model. Each module card's `purpose` comes
from the manifest `description` field (Cargo.toml
`[package].description`, package.json `description`); `api` lists
the extracted public symbols; `deps` lists the module's edges;
`verify:` lists the files the card was built from.

Because it is mechanical, `template` output is stable — the same
inputs always produce the same card, so diffs mean real change.
That is the property you want in committed, reviewed artifacts.

## `llm` (planned adapter)

Adds prose the manifest cannot carry: purpose narrative,
invariants, "who calls me and why". Design constraints for when
it lands:

* **Grounded or silent** — every generated claim cites the files
  it came from; uncited prose is not emitted.
* **Never replaces `verify:`** — the file list stays mechanical.
* **Human regions preserved** — `notes:` in cards is human-owned;
  regeneration merges, never clobbers.
* **Key resolution** — `provider`/`model` in `kuda.toml`, the key
  itself via `KUDA_LLM_KEY` env → OS keyring →
  `~/.config/kuda/env`. Never in the file.

## Choosing

```toml
[summarize]
kind = "template"     # deterministic (default)
# kind = "llm"
# provider = "anthropic"
# model = "..."
```

Until `llm` ships, the meaningful `purpose:` source is the
manifest `description` — worth writing well, since it becomes
part of the agent-facing map.
