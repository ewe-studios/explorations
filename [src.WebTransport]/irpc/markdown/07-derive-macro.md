---
title: Derive Macro — #[rpc_requests] Procedural Macro
---

# Derive Macro — #[rpc_requests] Procedural Macro

The `#[rpc_requests]` macro generates trait implementations for RPC service enums.

## Usage

```rust
use irpc::rpc_requests;

#[rpc_requests(message = MyMessage)]
#[derive(Debug, Serialize, Deserialize)]
enum MyProtocol {
    /// Request variant with oneshot response.
    #[rpc(tx=oneshot::Sender<i64>)]
    MyRequest(MyRequestType),
}
```

Source: `irpc/src/lib.rs:1` — Macro documentation.

## Generated Implementations

The macro generates:
- `Service` implementation for the enum
- `RpcMessage` implementations for all request types
- Channel type mappings
- Message enum type alias

## Variant Attributes

| Attribute | Purpose |
|-----------|---------|
| `#[rpc(tx=...)]` | Request channel type |
| `#[rpc(rx=...)]` | Response channel type |
| `#[rpc(wrap)]` | Wrap variant in outer enum |

Source: `irpc/src/lib.rs:1` — `#[rpc]` attribute documentation.

## irpc-derive Crate

```
irpc-derive/
├── Cargo.toml          # proc-macro = true
└── src/lib.rs          # Macro implementation
```

The derive macro is in a separate crate to avoid pulling proc-macro dependencies when the `derive` feature is disabled.

Source: `irpc/Cargo.toml:1` — Workspace members.

## irpc-iroh Crate

```
irpc-iroh/
├── Cargo.toml
└── src/lib.rs          # iroh transport integration
```

Provides iroh-specific transport helpers for building Quinn connections from iroh endpoints.

Source: `irpc/Cargo.toml:1`.

## Examples

| Example | Features | Description |
|---------|----------|-------------|
| `derive` | rpc, derive, quinn_endpoint_setup | Basic derive macro usage |
| `compute` | rpc, derive, quinn_endpoint_setup | Calculator service |
| `local` | derive | Local-only (no RPC) usage |
| `storage` | rpc, quinn_endpoint_setup | Storage service example |

Source: `irpc/Cargo.toml:[[example]]` sections.

## Related Documents

- [Service](../markdown/02-service.md) — Service trait
- [Overview](../markdown/00-overview.md) — Design goals
