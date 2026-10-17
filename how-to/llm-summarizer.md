# How-to: opt into the LLM summarizer

The default `template` summarizer is deterministic and zero-model —
cards contain only what extraction found (APIs, deps, commands).
The `llm` adapter adds prose: purpose, invariants, "who calls me".

## Enable

```toml
# kuda.toml
[summarize]
kind = "llm"
provider = "anthropic"
# model = "..."
```

Set the key — never in the toml:

```sh
export KUDA_LLM_KEY=...   # or: kuda secret set KUDA_LLM_KEY
```

Resolution order: env var -> OS keyring -> `~/.config/kuda/env`.

## Grounding rules (enforced by the adapter)

- Cards must cite the files they summarize — emitted as the
  `verify:` list.
- No invented APIs: every symbol in a card must resolve in the
  index.
- `notes:` regions are human-owned — regen never overwrites them.

## Cost control

Summaries regenerate only when a card's `files` change
(content-hash invalidation) — `sync` stays cheap; `--full`
re-summarizes everything.
