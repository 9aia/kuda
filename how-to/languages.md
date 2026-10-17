---
url: /kuda/how-to/languages.md
---
# How-to: add a language

Per-language knowledge is data, not code. A new language means a
new `data/langs/*.toml` descriptor — the extractors' vocabulary
stays fixed.

## The descriptor

`data/langs/rust.toml` is the template:

```toml
name = "rust"
extensions = [".rs"]
manifests = ["Cargo.toml"]          # module-boundary markers
grammar = "rust"                    # adapter id — see below

# Tree-sitter symbol query. Captures are @name.<kind> where
# <kind> maps to SymbolKind: function|type|trait|const|module|macro.
symbol_query = '''
(function_item name: (identifier) @name.function)
(struct_item name: (type_identifier) @name.type)
...
'''
```

| field          | used by                                  |
| -------------- | ---------------------------------------- |
| `name`         | reports, LSP `languageId`                |
| `extensions`   | both extractors' file walking            |
| `manifests`    | package graph + module attribution       |
| `grammar`      | `kuda-treesitter` language registry      |
| `symbol_query` | optional; falls back to a builtin        |

## What the adapter id controls

`grammar` is looked up in the adapter's registry — the match arm
inside `kuda-treesitter::language()`. Today that registry has one
entry (`rust`). Adding e.g. `python` needs the compiled
`tree-sitter-python` crate as a dependency plus one arm — an
adapter change, not an engine change. Descriptors for languages
without a compiled grammar still get `manifest` extraction
(module graph) for free.

## Writing a symbol query

1. Find the grammar's `node-types.json` (in the grammar repo) for
   the node names your language uses.
2. Capture the *name* child of each public-ish item as
   `@name.<kind>` — the kind suffix must match `SymbolKind`
   names or it lands in `Other`.
3. Test locally: put the query in `symbol_query`, run
   `kuda sync`, then `kuda find <a-symbol>`.

## Deploying descriptors

The CLI embeds `data/langs/*.toml` at build time. Adding a file
there + rebuilding is the whole distribution path today — a
config point for user-supplied descriptor dirs is a TODO item.
