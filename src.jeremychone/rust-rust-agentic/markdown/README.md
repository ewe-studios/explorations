# rust-agentic — Documentation

**Source:** `src/` — 54 Rust files across 12 modules. MCP (Model Context Protocol) client library implementing protocol version `2025-03-26`.

`rust-agentic` provides typed request/response/notification types for tools, resources, prompts, sampling, logging, and completion; stdio and HTTP transports; a `Client` struct with DashMap response queue and three async runner tasks; and a sampling handler abstraction for LLM proxying.

## Documentation

- [Overview](00-overview.md) — Architecture, MCP protocol version, McpMessage dispatch, Client structure, IntoMcpRequest trait, domain patterns, error model
- [MCP Messages](01-mcp-messages.md) — McpMessage enum with from_value dispatch, McpRequest/McpResponse/McpNotification/McpError, IntoMcpRequest/IntoMcpNotification traits, test suite
- [MCP Types](02-mcp-types.md) — Tools (discovery, invocation, annotations), Resources (read, subscribe), Prompts (templates, multi-modal content), Sampling (CreateMessage, model preferences), Capabilities (empty-object serialization), lifecycle, roots, common types
- [Transports](03-mcp-transports.md) — Client architecture with DashMap response queue, Trx channel pairs, IntoClientTransport sealed trait, stdio transport (3 tokio tasks, newline protocol), HTTP transport (reqwest + SSE, session tracking), SamplingHandlerAsyncFn trait
