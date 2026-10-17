# Reference: config

`kuda.toml` — one per indexed repo. Secrets resolve via env var ->
OS keyring -> `~/.config/kuda/env`; never this file.

```toml
[repo]
# name = "kuda"                    # default: directory basename
# root = "."

[extract]
kinds = ["manifest", "treesitter"] # impl crates, tried in order
# lsp_server = "rust-analyzer"

[summarize]
kind = "template"                  # template (default) | llm
# provider = "anthropic"           # llm only; key via KUDA_LLM_KEY

[store]
kind = "sqlite"
path = ".kuda/index.sqlite"        # L2 — rebuildable, gitignored

[cards]
dir = ".kuda"                      # index.yaml + modules/*.yaml

[workspace]
# roots = ["../cli", "../lsp"]
# resolve_git_deps = true
```

Full annotated example: `examples/kuda.toml.example` in the repo.
Field reference is generated from the `Config` structs by
`bun run docs:config`.
