---
title: RPC Transport — Quinn QUIC Streams and Serialization
---

# RPC Transport — Quinn QUIC Streams and Serialization

When the `rpc` feature is enabled, irpc supports remote communication over Quinn QUIC streams.

## QuinnRpcTransport

```rust
// irpc/src/lib.rs (cfg feature = "rpc")
pub mod rpc {
    pub use quinn::{RecvStream, SendStream};
    pub struct QuinnRpcTransport { /* stream handling */ }
}
```

The transport wraps Quinn's `SendStream` and `RecvStream` for bidirectional communication.

Source: `irpc/src/lib.rs:1` (rpc module).

## Serialization

Messages are serialized using **postcard** with **length-prefix varints**:

```
[varint: message length] [postcard-serialized message bytes]
```

The length prefix is always present, even for oneshot channels, ensuring the receiver knows exactly how many bytes to read.

Source: `irpc/src/lib.rs:1` — Serialization documentation.

## Stream Tuning

Since irpc doesn't abstract over the stream type, Quinn stream features are directly accessible:

- **Stream priority** — Set per-request via `send_stream.set_priority()`
- **Out-of-order receiving** — Use `recv_stream.read_chunk()` directly
- **Flow control** — Quinn's built-in flow control applies

Source: `irpc/src/lib.rs:1` — Transport documentation.

## Endpoint Setup Utilities

```rust
// irpc/src/util.rs (cfg feature = "quinn_endpoint_setup")
pub fn make_server_endpoint(
    addr: SocketAddr,
) -> Result<(quinn::Endpoint, quinn::TransportConfig)> { ... }

pub fn make_client_endpoint() -> Result<quinn::Endpoint> { ... }
```

Utilities for creating Quinn endpoints for testing and localhost RPC. Uses self-signed certificates via `rcgen`.

Source: `irpc/src/util.rs:1`.

## Request/Response Enum

```rust
// irpc/src/lib.rs (cfg feature = "rpc")
pub enum Request<L, R> {
    /// Local request (in-process).
    Local(L),
    /// Remote request (serialized message).
    Remote(R),
}
```

Source: `irpc/src/lib.rs:1`.

## Related Documents

- [Local](../markdown/06-local.md) — In-process transport
- [Client](../markdown/04-client.md) — Client API
