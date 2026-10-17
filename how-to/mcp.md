---
url: /kuda/how-to/mcp.md
---
# How-to: serve the index over MCP

`kuda mcp` exposes the index as tools to MCP-capable agents
(Devin, Cursor, Claude Code, …).

## Run

```sh
kuda mcp             # stdio server, serves the repo's index
kuda mcp --workspace # all roots in .kuda/workspace.yaml
```

## Tools

| tool          | query                                    |
| ------------- | ---------------------------------------- |
| `repo_card`   | L0 blob — the always-load summary        |
| `module_card` | L1 card + freshness for one module       |
| `find`        | symbol -> file:line                      |
| `dependents`  | reverse dep edges (impact analysis)      |
| `refs`        | code + docs/comments mentions            |
| `status`      | provenance drift vs HEAD                 |

## Register with your agent

Point the agent's MCP config at `kuda mcp` (stdio). Exact config
shape depends on the client — see your client's MCP docs.

## Trust model

Cards are hypotheses. Every `module_card` response includes the
`verify:` file list — agents should open those files before
acting on card content. `status` tells them whether the whole
index drifted.
