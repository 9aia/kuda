# How-to: sturn WAL integration

When a repo is [sturn](https://9aia.com/sturn/)-managed, kuda reads
the write-ahead log and feeds its ops into sync's diff — a typed,
evented change feed on top of the hash walk.

## What it does

`sync` already diffs by hashing every file. On sturn repos it
additionally walks `refs/sturn/wal/*` and marks modules touched by
these ops:

| op | feeds |
|---|---|
| `write` | `path` |
| `rename` / `delete` | `from` / `to` / `path` |
| `claim` / `release` | static prefix of `glob` |

Each ref's consumed head is persisted in the index (`sturn_wal:*`
meta keys), so ops are consumed exactly once.

**No config.** No WAL refs = no work. It's purely additive — a
write that bypassed sturn is still caught by the hash diff.

## Setup

```console
$ sturn init            # repo gets refs/sturn/wal/*
$ sturn ws create main  # a workspace to write through
$ kuda sync             # WAL ops now feed the diff
```

## The agent contract (advisory)

Agents should write *through* sturn when they can — claim before
editing, record intent — but sturn is **metadata, not a write
gate**. An agent that uses plain `write_file` doesn't break
anything; its change still lands in the hash diff, only the
coordination signal (claim/intent) is lost.

Point agents at sturn's MCP surface in their rules:

```markdown
## Writes
- Prefer `sturn` MCP tools for file writes: claim the path,
  write, then snap. Intent text is required.
- If sturn is unavailable, write normally — the index stays
  correct either way.
```

## Why additive, not exclusive

The WAL is ground truth *only* for writes that flowed through it.
The filesystem is ground truth for everything. kuda consumes both:
sturn says *who touched what and why*; hashes say *what actually
changed*. Neither is trusted alone.
