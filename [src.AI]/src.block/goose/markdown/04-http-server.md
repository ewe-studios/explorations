# Goose HTTP Server: REST API, Auth, Routes, OpenAPI

The `goose-server` crate provides an HTTP server with REST API, authentication, and OpenAPI documentation.

Source: `crates/goose-server/src/` — 7 modules, 25 Rust files.

## Module Structure

```
goose-server/src/
├── lib.rs            # Re-exports: openapi, state
├── main.rs           # Entry point
├── auth.rs           # Authentication
├── configuration.rs  # Server configuration
├── error.rs          # Error types
├── logging.rs        # Logging setup
├── openapi.rs        # OpenAPI documentation
├── state.rs          # Server state
├── routes/           # Route handlers
│   └── ...           # Various API routes
└── bin/              # Binary entry point
```

## Auth

Source: `auth.rs`

Authentication middleware for protecting API endpoints.

## Routes

Source: `routes/`

REST API route handlers for chat, configuration, and agent management.

## OpenAPI

Source: `openapi.rs`

OpenAPI/Swagger documentation generation for the REST API.

## State

Source: `state.rs`

Server state management — shared state across route handlers.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Core Library](01-core-library.md) — agents, providers
- [CLI](02-cli.md) — CLI commands
- [MCP Server](03-mcp-server.md) — MCP servers
- [Benchmarks](05-benchmarks.md) — Benchmarks

[Next → Benchmarks](05-benchmarks.md)
