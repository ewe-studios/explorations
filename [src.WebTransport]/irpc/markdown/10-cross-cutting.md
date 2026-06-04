---
title: Cross-Cutting — irpc-derive, irpc-iroh, Features, Error Handling
---

# Cross-Cutting Concerns — irpc-derive, irpc-iroh, Features, Error Handling

## irpc-derive

The procedural macro crate implements `#[rpc_requests]`:

- Parses service enum with `#[rpc(tx=..., rx=...)]` attributes
- Generates `Service` and `RpcMessage` implementations
- Creates message type aliases
- Handles `#[rpc(wrap)]` for variant wrapping

Source: `irpc/irpc-derive/src/lib.rs:1`.

## irpc-iroh

Provides iroh-specific transport helpers:

- Creates Quinn connections from iroh endpoints
- Handles iroh NodeAddr resolution
- Integrates with iroh's Router

Source: `irpc/irpc-iroh/src/lib.rs:1`.

## Feature Dependency Tree

```
default = ["rpc", "quinn_endpoint_setup", "spans", "stream", "derive"]
  ├── rpc → quinn, postcard, anyhow, smallvec, tracing
  ├── quinn_endpoint_setup → rpc, rustls, rcgen, futures-buffered
  ├── spans → tracing
  ├── stream → futures-util
  └── derive → irpc-derive
```

Disabling `rpc` removes all network dependencies:
- Remaining: serde, tokio (sync only), tokio-util, thiserror, n0-future

Source: `irpc/Cargo.toml:features`.

## Error Types

```rust
// irpc/src/lib.rs
pub enum Error {
    Serialize,        // Postcard serialization failed
    Io(io::ErrorKind), // Quinn IO error
    ConnectionClosed,  // QUIC connection closed
    Request(RequestError),
}

pub enum RequestError {
    ResponseChannelClosed,  // oneshot/mpsc dropped before response
}
```

Source: `irpc/src/lib.rs:1`.

## Tracing Spans

When the `spans` feature is enabled, messages carry parent tracing spans:

```rust
// Messages capture the current span when created
// Span is restored when message is processed
```

This prevents losing tracing context when messages pass through async channels.

Source: `irpc/Cargo.toml:features` — `spans` feature.

## Comparison with quic-rpc

| Feature | irpc | quic-rpc |
|---------|------|----------|
| Transport abstraction | No (Quinn only) | Yes (generic) |
| In-process use | Zero overhead | Some overhead |
| Cross-language | No | No |
| Runtime | Tokio only | Runtime agnostic |
| Focus | iroh ecosystem | General purpose |

Source: `irpc/src/lib.rs:1` — History section mentions evolution from quic-rpc.

## Related Documents

- [Derive Macro](../markdown/07-derive-macro.md) — irpc-derive details
- [RPC Transport](../markdown/05-rpc-transport.md) — Quinn transport
