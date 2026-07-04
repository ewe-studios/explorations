# Goose MCP Server: Developer, Computer Controller, Memory, Tutorial, Autovisualiser

The `goose-mcp` crate provides 5 MCP (Model Context Protocol) servers that extend goose's capabilities.

Source: `crates/goose-mcp/src/` — 8 modules, 53 Rust files.

## Module Structure

```
goose-mcp/src/
├── lib.rs
├── mcp_server_runner.rs  # MCP server runner
├── developer/            # Developer tools (8 files)
├── computercontroller/   # Computer control
├── autovisualiser/       # Auto-visualization
├── memory/               # Memory management
└── tutorial/             # Tutorial system
```

## Developer MCP Server

Source: `developer/` — 8 files.

| Module | Purpose |
|--------|---------|
| `rmcp_developer.rs` | MCP server runner |
| `shell.rs` | Shell command execution |
| `text_editor.rs` | File read/write/replace operations |
| `lang.rs` | Language detection |
| `analyze/` | Code analysis |
| `editor_models/` | Editor model definitions |
| `prompts/` | Developer prompts |

### Tools

| Tool | Purpose |
|------|---------|
| Shell | Execute shell commands |
| Text Editor | File read/write/replace operations |
| Code Analysis | Analyze code structure |

## Computer Controller MCP Server

Source: `computercontroller/`

Computer control capabilities — screenshot, mouse/keyboard control, window management.

## Memory MCP Server

Source: `memory/`

Persistent memory management — store, retrieve, and search knowledge across sessions.

## Tutorial MCP Server

Source: `tutorial/`

Interactive tutorial system — guided learning experiences.

## Autovisualiser MCP Server

Source: `autovisualiser/`

Auto-visualization — automatically generate visual output from code.

## MCP Server Runner

Source: `mcp_server_runner.rs`

Runs an MCP server with the given tools and capabilities.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Core Library](01-core-library.md) — agents, providers
- [CLI](02-cli.md) — CLI commands
- [HTTP Server](04-http-server.md) — REST API

[Next → HTTP Server](04-http-server.md)
