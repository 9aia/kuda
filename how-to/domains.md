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
| `roots`           | dirs walked for records — worktree `.git`-file markers don't stop declared roots (omit for non-tree sources) |
| `extensions`      | which files are scanned (unused by non-tree sources)  |
| `source`          | non-tree corpus location — see [Non-tree sources](#non-tree-sources) |
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

Shipped descriptors: `myco` (`project/` worktrees), `docs`
(`docs/` markdown — H1 headings as records, backticked
identifiers as edges), `media` (`file_records` mode — a record
per binary file), and `sturn` (non-tree — the coordination WAL
in git refs).

## Non-tree sources

A corpus doesn't have to live in the file tree. `[source]`
declares a non-filesystem corpus; today that's `sturn-wal` —
sturn's coordination WAL in git refs (`data/domains/sturn.toml`):

```toml
name = "sturn"
module = "sturn-wal"
purpose = "sturn coordination WAL — claims, writes, snaps, review events in refs/sturn/*"

record_patterns = ['"record":\s*"([^"]+)"']
ref_pattern = '"(?:path|from|to|glob|subject|event|ask|actor|ws)":\s*"([^"]+)"'

# [source] must come LAST: TOML nests every key after a table
# header inside that table, so a misplaced [source] would swallow
# the patterns into itself.
[source]
type = "sturn-wal"
ref_prefix = "refs/sturn"        # wal + archive refs
```

sturn keeps one ref per workspace — `refs/sturn/wal/<ws>`
(archives at `refs/sturn/archive/<ws>` on compaction) — each a
chain of commits whose tree holds one `event.json`. The
`records` extractor reads each ref as a **virtual JSONL file**:
one event per line, the ref name as `Symbol.file`, the event's
position as `Symbol.line`. The ordinary per-line record/edge
scan then applies unchanged — a WAL ref is just a multi-record
file that happens not to exist on disk.

Two fields are injected into each event line before scanning:
`seq` (1-based entry offset) and `record` — the queryable id
`<actor>:<op>@<ws>#<id8>` (`id8` = event/commit hash, stable
across compaction; `seq` is not). The descriptor's
`record_patterns` captures `record`, so `kuda find luis`,
`kuda find claim`, and `kuda find devin-wal-corpus` all land on
events, while `ref_pattern` edges each event to its payload —
`kuda dependents luis` lists an actor's ops, `kuda dependents
"crates/*"` finds claims on a glob.

Incremental sync works because each ref is registered as a
module input whose "content hash" is the ref's head sha: a
moved ref diffs exactly like a file write. A repo without
matching refs gets no module and no work — silent skip, same
as an absent corpus dir; `kuda doctor` reports `sturn` with
`(corpus absent)` there.

## Schema validation

Descriptors can carry `schema_json` — a JSON Schema (draft
2020-12) applied to frontmatter-bearing corpus files (`+++`
toml / `---` yaml). Regex finds records; the schema verifies
them:

```toml
schema_json = '''
{ "anyOf": [{"required": ["id"]}, {"required": ["item"]}],
  "properties": {
    "id":    {"type": "string", "pattern": "^[A-Z]+-[0-9]+$"},
    "status": {"enum": ["open", "claimed", "in-progress", "done"]} } }
'''
```

Invalid records stay indexed — they exist — but land as
warnings everywhere the card travels: `kuda sync` prints them,
`kuda card <module>` carries a `warnings:` field (so MCP's
`module_card` and the ACP surface see them too), `kuda status`
reports an invalid-record count, and `kuda doctor` reports
descriptor + schema compile health per domain. Files with no
frontmatter skip silently — line-records can't be schema'd.

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
`ref_pattern` to match that id shape. For a shared built-in, ship
it as a `data/domains/*.toml` and add it to `DOMAIN_TOMLS` in
`crates/kuda-cli/src/lib.rs`. For a repo-local corpus, declare it
in `kuda.toml` instead — no engine change needed. For a corpus
that isn't files at all, declare a `[source]` (see
[Non-tree sources](#non-tree-sources)) — new source types are an
engine change, new corpora of an existing type are data.

## Repo-local descriptors and disabling built-ins

`kuda.toml` can carry descriptors inline via `[[domain]]` — the
same fields as `data/domains/*` — and skip built-ins by name via
`[domains].disable`:

```toml
[domains]
disable = ["media"]        # built-ins: myco, docs, media

[[domain]]
name = "myco"              # same name as a built-in -> replace it
roots = ["ops/project"]
extensions = [".md"]
module = "ops-ledger"
purpose = "ops ledger"
record_patterns = ['-\s*(?:\[[ xX]\]\s*)?([A-Z]+-[0-9]+)']
ref_pattern = '\b[A-Z]+-[0-9]+\b'
```

Resolution is deterministic: built-ins load first, `disable`
names drop out, then `[[domain]]` entries apply in declared
order — a `name` matching a built-in replaces it (the repo wins),
any other name appends. The resolved set is what `sync`,
`status`, and `doctor` use — `kuda doctor` lists it with per
descriptor health, so a bad regex or schema in an inline entry
surfaces the same way a shipped one would.
