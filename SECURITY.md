# Security Policy

Report vulnerabilities privately to the maintainers via GitHub:
https://github.com/9aia/kuda/security/advisories/new

Do not open public issues for security reports.

## Scope

- `kuda sync` parses source trees and (optionally) spawns language
  servers — extractor plugins run with the user's privileges over
  the indexed paths only.
- `kuda mcp` serves the index to local agent clients; it exposes
  repo structure and code locations — treat the bound socket like
  the code itself.
- Secrets (`KUDA_LLM_KEY` and future provider keys) resolve via
  env var → OS keyring (`secret-tool`) → `~/.config/kuda/env`
  (chmod 600); values never appear in `kuda.toml` or `.kuda/`
  artifacts.
- The `llm` summarizer sends code fragments to a third-party
  provider — it is opt-in; `template` (default) is fully local.

## Notes

- Index artifacts (`index.sqlite`, cards) contain paths, symbol
  names, and snippets — they inherit the repo's confidentiality;
  `.kuda/` files committed to a repo travel with it.
