# Changelog

## [0.3.0] - 2026-11-24

### Added

- **config:** repo-local [[domain]] descriptors + [domains].disable (`fe49c9e`)
- fuzzy find — exact miss falls back to LIKE substring tier, hits marked (fuzzy); search on store port (`f440700`)
- **kuda-driver:** cli verb is an engine passthrough — the tamias bins[] shim contract (`ebca8ed`)
- **records:** non-tree domain sources — sturn WAL corpus in git refs (`e19ac36`)
- incremental sync — hash-match skips unchanged modules (no agent re-prompt); semantic cards committed (`dcdff04`)
- kuda-acp — live-tested vs cline --acp: session reuse, request_permission auto-answer, section parser for agent prose (purpose/invariants/notes) (`ce36f25`)
- **index:** union graph — cross-root edges + docs corpus (`bbc3e27`)
- **extract:** records adapter — domain corpora as indexable data (`b1927ee`)
- mechanism-version invalidation — provenance.generator compared on reuse; bump GENERATOR forces re-summarize on unchanged bytes (`acb015b`)
- gen-cli-docs for docs pipeline (`a6c6073`)
- L1.5 file cards — chunked per-file agent summaries (text-splitter, code-aware via tree-sitter), per-file hash-gated; AcpSummarizer::complete() shares one session; [summarize] file_cards=true opts in (`a16f1e1`)
- sync --full, tree-drift status, git-dep resolution, file-card sweep (`9bd14c5`)
- workspace find/refs merge, LSP textDocument/references tier, purpose fallback chain (manifest->crate docs->README) (`871fb15`)
- **watch:** re-index on domain-corpus + WAL-ref writes (T-035) (`80c2152`)
- workspace mode — .kuda/workspace.yaml roots, sync + dependents across repos (`6c92970`)
- kuda-lsp (documentSymbol via rust-analyzer) + kuda acp (Agent Client Protocol surface) (`6ac9121`)
- kuda mcp — stdio JSON-RPC server exposing repo_card/module_card/find/dependents/status/refs (`6418f61`)
- sync pipeline — manifest extract + template cards + sqlite store; status/map/find/dependents/refs queries; dogfooded on self (`63d73a7`)
- kuda-sturn — WAL ops (write/rename/delete/claim/release) feed sync's module diff additively via refs/sturn/wal/* + per-ref head in store meta; zero-config, no refs = no work (`4a5c479`)
- kuda-acp summarizer — spawns ACP agent for grounded card prose, template fallback; kuda-llm parked (acp is the semantic path) (`444ff6e`)
- **records:** schema-validated corpora — verify record structure, not just shape (`31359c5`)
- file-level incremental sync — walk once, in-process blob hashes, store diff -> treesitter parses only changed modules' files; per-module file-hash rows in sqlite; seed from committed cards on upgrade; module identity = manifest pkg name (fixes dir-name split) (`2f0a2e7`)
- kuda watch — notify-driven incremental sync (debounced, excluded-dir filtered); todo: embeddings for contradiction clustering (`2cc424d`)
- kuda-treesitter — data-driven symbol extraction (rust query in data/langs), import edges; find/dependents now real (`285056f`)
- **records:** file-as-record corpora — any bytes join the graph (`2426776`)
- [extract] exclude config + broader built-in walk exclusions (dist/build/out/.cache/vendor/.venv/__pycache__/coverage/…) — keeps generated+vendored dirs out of index, hashes, and agent prompts; e2e stub assertion retired (sync is real) (`1d6d512`)
- **init:** gitignore rebuildable .kuda artifacts — index.sqlite + meta.json (T-009) (`15708d4`)

### Changed

- parse/dispatch split — Cli/Cmd pub, ops callable as dispatch(&cli) (`de42142`)
- **config:** scarab-config-file discovery — kuda.{yaml,yml,toml} (P-017) (`bda9016`)
- **vcs:** consume lasius-vcs port — drop kuda-vcs/kuda-git (`7e9ec7c`)
- **crates:** rename extract adapters to kuda-extract-<impl> (`78f3201`)
- **driver:** derive ops manifest from kuda::command() (`6690b1b`)
- **vcs:** input_hash rev-parse bypass onto lasius-vcs (P-050) (`5d84640`)
- extractor + summarizer registries in the sync pipeline (`5798183`)
- engine as lib — kuda-cli/src/lib.rs, bin is ~10 lines (`4d705c4`)
- crate restructure — drop CI domain, land memory topology (ADR-0001) (`3432d05`)
- rename memory tier L1.5 -> L1 file granularity (`0c540dd`)

### Fixed

- silence build warnings — check-cfg coverage, FileCardJob args (`e6b17fb`)
- monorepo scale — bounded cards, real dep edges, SIGPIPE-safe output (`e360ec6`)
- canonicalize repo root, skip git-worktree dirs in extraction, purpose from manifest description (`c2b4981`)
- keep agent CoT out of cards, honest generator stamp, no self-edges (`ba00670`)
- **sync:** stale-generator rebuilds join the parse set (+ card regen) (`b0701c7`)
- &Path/PathBuf mismatches in workspace sync (`b02c2f3`)
- template doc-comment truth + assert acp in help smoke (semantic dogfood findings) (`9cc2ada`)
- **sync:** skip agent prompts for manifest-only pseudo-modules (nothing to ground on); refs/find text search uses hard-noise excludes only so fixtures stay searchable (`97dfaa4`)
- **sync:** preserve human notes + reject monologue acp replies (I-002) (`572cee6`)
- config errors surface instead of silent defaults; kuda.toml reverted to template (acp proven, cline-pass streams no chunks) (`44a8ea3`)
- **acp:** guard unparseable replies — warn + fallback, never pollute cards (I-002) (`0572a88`)
