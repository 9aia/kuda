---
url: /kuda/how-to/acp.md
---
# How-to: the ACP surface

`kuda acp` speaks the Agent Client Protocol — the editor↔agent
wire protocol used by Zed and supported by agent clients such as
cline and Cursor. Where `kuda mcp` exposes *tools*, `kuda acp`
exposes the index as an *agent* you can prompt.

## Run

```sh
cd your-repo
kuda acp     # stdio JSON-RPC server
```

## Prompts

The session/prompt method accepts compact query strings:

| prompt              | returns                                  |
| ------------------- | ---------------------------------------- |
| `find <symbol>`     | `file:line kind module` hits             |
| `dependents <name>` | reverse dep edges                        |
| `card <module>`     | the module card YAML                     |
| `map` / `status`    | the L0 card (fallback for anything else) |

Replies stream back as session/update notifications with
`agent_message_chunk` content, then a `stopReason: end_turn`
result.

## Verify

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize"}' \
  '{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"'"$PWD"'","mcpServers":[]}}' \
  '{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"kuda-default","prompt":[{"type":"text","text":"find IndexStore"}]}}' \
  | kuda acp
```

You get `agentInfo`, a `sessionId`, an `agent_message_chunk`
carrying `crates/kuda-store/src/lib.rs:9 Trait kuda-store`, and
`end_turn`.

## Which protocol when?

* **MCP** — the agent already has tool-use wired and you want the
  index as callable tools. Most current clients.
* **ACP** — you want kuda to appear as an *agent* inside an
  ACP-speaking client, sharing its own session model.

Both read the same `.kuda/` artifacts — pick per client, or run
both.
