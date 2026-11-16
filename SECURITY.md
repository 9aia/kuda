# Security Policy

Report vulnerabilities privately by emailing
luisfuturist+9aia@gmail.com with "SECURITY" in the subject — do
not open public issues, do not use GitHub's advisory or
private-reporting features, and do not disclose details publicly;
contact us first and we will coordinate disclosure privately.

## Scope

- `kuda sync` parses source trees and (optionally) spawns language
  servers — extractor plugins run with the user's privileges over
  the indexed paths only.
- `kuda mcp` serves the index to local agent clients; it exposes
  repo structure and code locations — treat the bound socket like
  the code itself.
- Provider keys would resolve via env var → OS keyring
  (`secret-tool`) → `~/.config/kuda/env` (chmod 600) through the
  `kuda-secret` port (`kuda-keyring`/`kuda-dotenv` impls); values
  never appear in `kuda.toml` or `.kuda/` artifacts. No shipped
  adapter consumes secrets today — the `llm` summarizer that would
  is parked (backlog kind); `template` (default) and `acp` (your
  own agent binary, no API keys) are the live summarizers.

## Notes

- Index artifacts (`index.sqlite`, cards) contain paths, symbol
  names, and snippets — they inherit the repo's confidentiality;
  `.kuda/` files committed to a repo travel with it.
