---
url: /kuda/how-to/multi-repo.md
---
# How-to: index a multi-repo workspace

For sibling checkouts — an org of repos that import each other —
kuda keeps one index per repo *and* answers cross-repo questions
over all of them.

## 1. Declare the workspace

Create `.kuda/workspace.yaml` at the checkout root (the directory
containing the sibling repos), or inside one repo's `.kuda/`:

```yaml
roots:
  - kuda
  - mimus
  - ../cli
  - ../lsp
```

Paths resolve relative to the file's directory. `[workspace]` in
`kuda.toml` documents the same intent:

```toml
[workspace]
roots = ["../cli", "../lsp"]
resolve_git_deps = true
```

## 2. Sync all roots

```sh
kuda sync --workspace
```

```
sync /home/you/GitHub/9aia/kuda
indexed 19 module(s) at af26b8c
sync /home/you/GitHub/9aia/mimus
indexed 15 module(s) at 323421e
```

Each root gets its own `.kuda/` artifacts — the per-repo indexes
stay independent and committable in their own repos.

## 3. Query across the set

```sh
kuda dependents kuda-core --workspace
```

Edges found in *every* root's store are merged — so an import of
`kuda-core` inside `mimus` shows up alongside internal users.
That is the impact-analysis query that matters for a breaking
change: "who calls this, anywhere in the org".

## 4. Git dependency edges

`resolve_git_deps = true` records the intent for the extractor
tier that maps `git = "…"` manifest dependencies onto sibling
checkouts — turning "depends on github.com/x/y" into "depends on
the sibling repo y". Until that lands, cross-repo edges come from
module-name matching across the merged stores.

## Current limits

* `find` and `refs` remain per-root; `dependents` is the merged
  query today.
* Cards are generated per root — there is no aggregate workspace
  card yet. Compose one from `kuda map` output across roots.
