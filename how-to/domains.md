---
url: /kuda/how-to/domains.md
---
# How-to: index a record corpus

Non-code corpora — task ledgers, ADRs, handoffs — are record
collections the index can see. Domain knowledge is data, not code:
a new corpus means a new `data/domains/*.toml` descriptor — the
`records` extractor's vocabulary stays fixed (ADR-0003).

Enable it (on by default):

```toml
[extract]
kinds = ["manifest", "treesitter", "records"]
```

A corpus that doesn't exist on disk is a silent skip — enabling
`records` on a repo with no matching roots costs nothing.

## The descriptor

`data/domains/myco.toml` is the template:

```toml
name = "myco"
roots = ["project"]               # corpus dirs, repo-relative
extensions = [".md"]              # record files
module = "project"                # card + provenance grouping
purpose = "myco work ledger — tasks, ADRs, issues, plans, handoffs"

record_patterns = [
  # frontmatter: id = "ADR-0001" / item = "T-027"
  "(?:id|item)\\s*=\\s*\"([A-Z]+-[0-9]+)\"",
  # ledger lines: "- [ ] T-001" and "- T-013"
  "-\\s*(?:\\[[ xX]\\]\\s*)?([A-Z]+-[0-9]+)",
]
ref_pattern = "\\b[A-Z]+-[0-9]+\\b"
```

| field             | controls                                              |
| ----------------- | ----------------------------------------------------- |
| `roots`           | dirs walked for records — worktree `.git`-file markers don't stop declared roots |
| `extensions`      | which files are scanned                               |
| `module`          | the card the corpus gets (`kuda card <module>`)        |
| `purpose`         | card purpose (domain modules have no manifest/README) |
| `record_patterns` | per-line regexes; capture group 1 = the record id     |
| `ref_pattern`     | every match becomes a `references` dep-edge target     |

## How records become index entries

Each `record_patterns` match emits a `record`-kind `Symbol` —
`kuda find T-001` resolves to `file:line`. Frontmatter records and
checklist lines are the same mechanism: the pattern is the format
knowledge, not the parser.

`ref_pattern` matches emit `record -> id` edges —
`kuda dependents ADR-0001` answers "which tasks/docs reference
this". Edge scope: a file with one record scans the whole text; a
ledger with many records scopes each record's refs to its own
line, so `- [ ] T-002 blocked on T-001` edges `T-002 -> T-001`
without cross-wiring the whole file. If the pattern has a capture
group, group 1 is the edge target — `"`(ident)`"` strips the
backticks. Fenced code blocks never produce records; mentions
inside them still produce edges.

Corpus files bucket under `module` in the shared walk — the card,
its `verify:` file list, `status` drift, and incremental sync all
ride the existing pipeline.

## Edges into the code graph

`ref_pattern` targets resolve through the same `resolve_module`
pass as code deps — a task line naming `kuda-store` edges
`T-XXX -> kuda-store`, and `kuda dependents kuda-store` lists the
task alongside `PackageDep` edges. Under `--workspace`, the
resolution set is the **union of all roots' modules**: repo A's
`import sister_lib` lands on repo B's canonical `sister-lib`
module. One graph, federated stores — queries already merge
across roots, so cross-repo edges answer normally.

Shipped descriptors: `myco` (`project/` worktrees) and `docs`
(`docs/` markdown — H1 headings as records, backticked
identifiers as edges).

## Try it

```sh
kuda sync
kuda card project        # the corpus card: api = record ids
kuda find T-001          # file:line of the record
kuda dependents ADR-0001 # what references this record
```

## Writing your own descriptor

Point `roots` at any dir of uniform-format record files, write a
`record_patterns` regex whose group 1 is the stable id, and set
`ref_pattern` to match that id shape. Ship it as a
`data/domains/*.toml` and add it to `DOMAIN_TOMLS` in
`crates/kuda-cli/src/main.rs`. Repo-local descriptors
(`[domains]` config) and non-tree sources (git refs, sqlite) are
deferred — see ADR-0003.
