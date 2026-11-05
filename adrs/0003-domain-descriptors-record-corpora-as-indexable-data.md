+++
id = "ADR-0003"
title = "Domain descriptors — record corpora as indexable data"
status = "open"
created = "2026-11-05"
owner = "devin"
priority = "medium"
+++

Status: proposed (implemented on feat/domain-records)

## Context

ADR-0001 made language knowledge data (`data/langs/*`): a new
language is a toml, not an engine change. The same trick applies
one level up: agent work-state corpora (myco `project/`, and later
sturn/arca surfaces) are record collections — tasks, ADRs, issues,
handoffs — that the index can't see today because extraction only
understands code.

## Decision

`DomainDef` descriptors in `data/domains/*` — the LangDef trick,
one level up. A descriptor declares corpus roots, record-file
extensions, the module its records group under, and regexes:

- `record_patterns` — per-line regexes, capture group 1 = record
  id. Frontmatter records (`id = "T-001"`, `item = "T-027"`) and
  checklist records (`- [ ] T-001`, `- T-013`) are one mechanism:
  the pattern is the format knowledge.
- `ref_pattern` — id mentions become `EdgeKind::References` edges.
  Scope: whole file for single-record files, own line for
  multi-record ledgers.

New `kuda-records` adapter (kind `"records"`, on by default —
absent corpus = silent skip) turns descriptors into
`SymbolKind::Record` symbols. Corpus files bucket under the
descriptor's module in the shared walk, so cards, provenance,
`status` drift, and incremental sync ride the existing pipeline.
Declared roots are walked even behind a `.git`-file worktree
marker (myco's `project/` is one).

## Consequences

- `find T-001`, `card project`, `dependents ADR-0001` work on
  work-state corpora; the index becomes the single memory surface
  across code + records.
- Engines compose through artifacts (their files), never code
  deps — kuda knows nothing about myco; the descriptor does.
- Sub-module edges (record -> record) required grouping store
  writes by *symbol's* module, not just module-name `from`.
- Deferred: repo-local descriptor overrides (`[domains]` config),
  non-tree corpus sources (git refs / sqlite — sturn's WAL),
  record -> code-module edges.
