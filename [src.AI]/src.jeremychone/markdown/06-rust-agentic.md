# rust-agentic: MCP Agent Framework

MCP (Model Context Protocol) agent framework built on `rust-genai` and `rust-rpc-router`.

Source: `rust-agentic/src/` — 2 modules.

## Module Structure

```
rust-agentic/src/
├── lib.rs    # Re-exports mcp module, RpcId
└── mcp/      # MCP protocol implementation
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `rust-genai` | AI provider communication |
| `rust-rpc-router` | JSON-RPC routing |

## Cross-References

- [Overview](00-overview.md) — All 21 projects
- [rust-genai](01-rust-genai.md) — AI provider client
- [rust-rpc-router](02-rust-rpc-router.md) — JSON-RPC routing

[Next → rust-pretty-sqlite](07-rust-pretty-sqlite.md)
