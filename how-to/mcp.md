# How-to: serve the index over MCP

`kuda mcp` exposes the index as tools to MCP-capable agents
(Devin, Cursor, Claude Code, Zed, …). It is a stdio JSON-RPC
server — no ports, no daemon.

## Run

```sh
cd your-repo
kuda mcp
```

## Tools

| tool          | query                                   |
| ------------- | --------------------------------------- |
| `repo_card`   | L0 blob — the always-load summary       |
| `module_card` | L1 card + freshness for one module      |
| `find`        | symbol -> file:line                     |
| `dependents`  | reverse dep edges (impact analysis)     |
| `refs`        | code + docs/comments mentions           |
| `status`      | provenance drift vs HEAD                |

## Register with your agent

stdio servers are declared in the client's MCP config — the shape
varies, but it is always "command + args":

```json
{
  "mcpServers": {
    "kuda": { "command": "kuda", "args": ["mcp"] }
  }
}
```

Run it from a repo that has been `kuda sync`'d — the tools read
`.kuda/` at call time, so a re-`sync` is picked up without
restarting the server.

## Verify the handshake

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"find","arguments":{"name":"IndexStore"}}}' \
  | kuda mcp
```

Two replies: the `initialize` result, then a `content[0].text`
body like `crates/kuda-store/src/lib.rs:9 Trait kuda-store`.

## Trust model

Cards are hypotheses. Every `module_card` response includes the
`verify:` file list — agents should open those files before
acting on card content, and `status` tells them whether the whole
index has drifted. See [architecture](../explanation/architecture.md#trust-model).

See also: [ACP surface](acp.md) — same index, the other wire
protocol.
