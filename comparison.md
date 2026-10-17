# kuda vs the alternatives

## vs grep + re-read every session

|                       | grep/ripgrep          | kuda                       |
| --------------------- | --------------------- | -------------------------- |
| find a symbol         | fast, per-query       | fast — plus kind + card    |
| "what depends on X"   | manual, cross-repo pain | one query                |
| overall comprehension | none                  | L0 card in ~1k tokens      |
| freshness             | always current        | provenance-checked         |
| doc/comment mentions  | same grep             | unified `refs --docs`      |

## vs embeddings / RAG

|                       | RAG store             | kuda                       |
| --------------------- | --------------------- | -------------------------- |
| fuzzy semantic lookup | strong                | (optional store impl later)|
| dep graph, provenance | weak                  | first-class                |
| staleness             | opaque                | explicit sha/hash diff     |
| determinism           | model-dependent       | `template` = deterministic |

## vs LSP alone

|                       | LSP                   | kuda                       |
| --------------------- | --------------------- | -------------------------- |
| cross-file refs       | strong (per language) | `lsp` impl reuses them     |
| needs a running server| yes                   | only for the `lsp` tier    |
| prose/cards           | none                  | L1/L0 artifacts            |
| persistence           | session-scoped        | committed + rebuildable    |

Trade-off: kuda adds an index step and committed artifacts — the
payoff is that "overall comprehension" becomes a fixed ~1k-token
load instead of a per-session expedition.
