#!/usr/bin/env python3
"""Generate the crate-map table (into architecture.md) and
reference/crate-api.md from workspace Cargo.tomls — never hand-edit
either."""
import pathlib, re

ROOT = pathlib.Path(__file__).parent / ".."
WST = (ROOT / "Cargo.toml").read_text()

LAYER = {
    "kuda": "binary",
    "kuda-core": "domain",
    "kuda-runner": "port", "kuda-tunnel": "port",
    "kuda-trigger": "port", "kuda-status": "port",
    "kuda-vcs": "port", "kuda-queue": "port",
    "kuda-service": "port", "kuda-secret": "port",
    "kuda-config": "port", "kuda-cache": "port",
}

# workspace members — match `"crates/…"` paths only (not dep paths)
members = re.findall(r'"(crates/[^"]+)"', WST)
seen, order_dedup = set(), []
for m in members:
    if m not in seen:
        seen.add(m)
        order_dedup.append(m)
# include the binary crate (path crates/kuda-cli, package name kuda)
if "crates/kuda-cli" not in order_dedup:
    order_dedup.append("crates/kuda-cli")

rows = []
for rel in order_dedup:
    t = (ROOT / rel / "Cargo.toml").read_text()
    name = re.search(r'^name = "([^"]+)"', t, re.M).group(1)
    desc = re.search(r'^description = "([^"]+)"', t, re.M).group(1)
    desc = desc.replace("<", "&lt;").replace(">", "&gt;")
    rows.append((name, LAYER.get(name, "impl"), desc))

order = {"domain": 0, "port": 1, "impl": 2, "binary": 3}
rows.sort(key=lambda r: (order[r[1]], r[0]))
print(f"crate map: {len(rows)} crates")

# 1. crate map table inside explanation/architecture.md
arch = pathlib.Path("explanation/architecture.md")
s = arch.read_text()
table = "| crate | layer | purpose |\n|---|---|---|\n" + "\n".join(
    f"| `{n}` | {l} | {d} |" for n, l, d in rows)
s = re.sub(
    r"\| crate[^\n]*\n\| *[-| ]+[- ]*\|\n?(?:\|[^\n]*\|\n?)*",
    table + "\n", s, count=1)
arch.write_text(s)

# 2. crate-api page — one rustdoc link per crate
out = ["# Crate API", "",
       "Rustdoc for every workspace crate — public APIs are",
       "documented at source (doc comments).", ""]
for n, l, _ in rows:
    und = n.replace("-", "_")
    out.append(f"- [{n}](/kuda/api/{und}/index.html) — {l}")
pathlib.Path("reference/crate-api.md").write_text("\n".join(out) + "\n")
print("crate-api regenerated")
