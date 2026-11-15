---
url: /kuda/explanation/boundaries.md
---
# Boundaries

kuda owns **repo memory** — extraction, cards, the symbol graph,
freshness. It does not own:

* **Verification** — `mimus` audits the cards; kuda emits claims,
  never verdicts.
* **Distribution** — the `kuda` command is a tamias pack
  (`libraries/kuda`); tamias owns install/transport.
* **Orchestration** — campaigns and scheduling are peregrin's;
  kuda exposes ops, never workflows.
* **Secrets** — credentials resolve through `kuda-secret` ports
  (keyring/dotenv); kuda never stores values.
