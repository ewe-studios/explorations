# rust-rpc-router — Documentation

**Source:** `src/` — 32 Rust files across 7 modules. JSON-RPC 2.0 router with typed handlers, dependency injection, and full request/response parsing.

`rpc-router` is a JSON-RPC 2.0 compliant router that routes method calls to async handler functions. It provides a typed handler system with dependency injection via `Resources`, parameter parsing via `IntoParams`, and full JSON-RPC request/response/notification parsing with validation.

## Documentation

- [Overview](00-overview.md) — Architecture, router, handler system, resources, IntoParams, RpcId generation
- [RPC Messages](01-rpc-messages.md) — RpcRequest, RpcNotification, parsing, validation, RpcRequestParsingError
- [RPC Response](02-rpc-response.md) — RpcResponse, RpcError, serialization, deserialization, error mapping
