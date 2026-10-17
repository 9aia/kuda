---
url: /kuda/reference/config.md
---
# Config — `kuda.toml`

## Top level

| field | type | required | notes |
|---|---|---|---|
| `repo` | `RepoCfg` | optional | — |
| `extract` | `ExtractCfg` | optional | — |
| `summarize` | `SummarizeCfg` | optional | — |
| `store` | `StoreCfg` | optional | — |
| `cards` | `CardsCfg` | optional | — |
| `workspace` | `WorkspaceCfg` | optional | — |

## `[repo]`

| field | type | required | notes |
|---|---|---|---|
| `name` | `string` | optional | Defaults to the directory basename. |
| `root` | `path` | optional | Repo root; default = the toml's dir. |

## `[extract]`

| field | type | required | notes |
|---|---|---|---|
| `kinds` | `list` | optional | Impl kinds tried in order: "manifest" | "treesitter" | "lsp". |
| `lsp_server` | `string` | optional | Server command for the lsp adapter (per-language later). |

## `[summarize]`

| field | type | required | notes |
|---|---|---|---|
| `kind` | `string` | optional | "template" (deterministic, default) | "llm". |
| `provider` | `string` | optional | Provider id for the llm adapter; key resolves via secrets chain. |
| `model` | `string` | optional | — |

## `[store]`

| field | type | required | notes |
|---|---|---|---|
| `kind` | `string` | optional | — |
| `path` | `path` | optional | L2 index path — rebuildable, gitignored. |

## `[cards]`

| field | type | required | notes |
|---|---|---|---|
| `dir` | `path` | optional | Committed artifacts dir: index.yaml + modules/\*.yaml. |

## `[workspace]`

| field | type | required | notes |
|---|---|---|---|
| `roots` | `Vec<PathBuf>` | optional | Sibling repo roots indexed together. |
| `resolve_git_deps` | `bool` | optional | Resolve `git = ".../org/repo"` manifest deps to siblings. |

## Constraints

* `queue = "all"` rejects `cancel_running = true` at load —
  killing a run would silently drop a promised commit.
* `runner.kind` and `trigger.kind`/`status.kind` resolve to impl
  crates; unknown kinds fail at `kuda doctor`/startup.
* `deny_unknown_fields`: typos in field names fail the load.
