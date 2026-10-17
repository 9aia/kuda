---
url: /kuda/how-to/multi-repo.md
---
# How-to: index a multi-repo workspace

For orgs spread across sibling checkouts (mono-repos too — a
workspace is just a set of roots).

## Workspace file

Create `.kuda/workspace.yaml` in the parent dir of your checkouts:

```yaml
roots:
  - platform
  - cli
  - lsp
  - kit-canonical
resolve_git_deps: true   # github.com/org/repo deps -> siblings
```

## Sync the workspace

```sh
kuda sync --workspace          # indexes every root
kuda dependents Foo --workspace # cross-repo reverse lookup
```

With `resolve_git_deps`, manifest edges like
`git = "https://github.com/org/cli"` resolve to the `../cli`
checkout — "X moved to the sibling repo" stops being prose-only
knowledge and becomes a graph edge.

## Tips

* Each repo keeps its own `kuda.toml` + `.kuda/`; the workspace
  layer only links them.
* Run `kuda status --workspace` after pulling several repos —
  it reports drift per repo.
