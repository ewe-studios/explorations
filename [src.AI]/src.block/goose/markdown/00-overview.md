# Goose: Overview

AI agent CLI with MCP (Model Context Protocol) server support. Built in Rust with 6 crates.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.block/goose/`
**License:** MIT

## Workspace Structure

| Crate | Files | Purpose |
|-------|-------|---------|
| `goose` | 175 | Core library: agents, providers, tools, session, MCP utilities |
| `goose-cli` | 40 | CLI: commands, session management, recipes |
| `goose-mcp` | 53 | MCP servers: developer, computer controller, memory, tutorial |
| `goose-server` | 25 | HTTP server: REST API, auth, routes |
| `goose-bench` | 38 | Benchmarks |
| `goose-test` | 6 | Test utilities |

## Core Library Modules (26)

```
goose/src/
├── agents/           # Agent implementations
├── config/           # Configuration
├── context_mgmt/     # Context management
├── conversation/     # Conversation handling
├── execution/        # Tool execution
├── hints/            # AI hints
├── logging/          # Logging
├── mcp_utils/        # MCP utilities
├── model/            # Model types
├── oauth/            # OAuth support
├── permission/       # Permission system
├── prompt_template/  # Prompt templates
├── providers/        # AI providers (OpenAI, Anthropic, etc.)
├── recipe/           # Recipe system
├── recipe_deeplink/  # Recipe deeplinks
├── scheduler/        # Task scheduler
├── scheduler_trait/  # Scheduler trait
├── security/         # Security policies
├── session/          # Session management
├── session_context/  # Session context
├── subprocess/       # Subprocess management
├── token_counter/    # Token counting
├── tool_inspection/  # Tool inspection
├── tool_monitor/     # Tool monitoring
├── tracing/          # Tracing
└── utils/            # Utilities
```

## MCP Servers (goose-mcp)

| Server | Purpose |
|--------|---------|
| `developer` | Developer tools (file operations, shell commands) |
| `computercontroller` | Computer control |
| `autovisualiser` | Auto-visualization |
| `memory` | Memory management |
| `tutorial` | Tutorial system |

## Related Documents

- [Core Library](01-core-library.md) — agents, providers, tools, session
- [CLI](02-cli.md) — commands, session management
- [MCP Server](03-mcp-server.md) — MCP servers
- [HTTP Server](04-http-server.md) — REST API
- [Benchmarks](05-benchmarks.md) — Benchmarks

[Next → Core Library](01-core-library.md)
