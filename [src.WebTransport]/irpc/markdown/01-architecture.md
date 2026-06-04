---
title: Architecture — Layer Diagram, Module Map, and Feature Flags
---

# Architecture — Layer Diagram, Module Map, and Feature Flags

## Module Map

| Module | Lines | Purpose |
|--------|-------|---------|
| `lib.rs` | 1961 | Core API: Service, Client, Channels, channels module |
| `util.rs` | 450 | Quinn endpoint setup utilities, test helpers |
| `irpc-derive/` | — | `#[rpc_requests]` procedural macro |
| `irpc-iroh/` | — | iroh integration utilities |

Source: `irpc/Cargo.toml:1` — Workspace members.

## Key Types

```rust
// Core traits
pub trait RpcMessage: Debug + Serialize + DeserializeOwned + Send + Sync + Unpin + 'static {}
pub trait Service: Serialize + DeserializeOwned + Send + Sync + Debug + 'static {
    type Message: RpcMessage;
}
pub trait Channels<S: Service>: Send + 'static {
    type Tx;
    type Rx;
}

// Channel types
pub mod channel {
    pub mod oneshot;  // Single response
    pub mod mpsc;     // Multiple messages
    pub mod none;     // Disabled channel
}

// Client
pub struct Client<S: Service>(ClientInner<S::Message>, PhantomData<S>);
pub struct WithChannels<I: Channels<S>, S: Service> { request: I::Tx, service: PhantomData<S> }
pub struct LocalSender<S: Service>(tokio::sync::mpsc::Sender<S::Message>);
```

Source: `irpc/src/lib.rs:1`.

## RPC Module (feature-gated)

When the `rpc` feature is enabled:

```rust
pub mod rpc {
    pub use quinn::{RecvStream, SendStream};
    pub struct QuinnRpcTransport { /* Quinn stream handling */ }
    // Quinn endpoint setup utilities
}
```

Source: `irpc/src/lib.rs:1` (cfg feature = "rpc").

## Workspace Structure

```
irpc/
├── Cargo.toml              # Main crate
├── irpc-derive/            # Procedural macro crate
│   ├── Cargo.toml
│   └── src/lib.rs          # #[rpc_requests] implementation
├── irpc-iroh/              # iroh integration crate
│   ├── Cargo.toml
│   └── src/lib.rs          # iroh transport helpers
└── src/
    ├── lib.rs              # 1961 lines
    └── util.rs             # 450 lines
```

**Aha:** The `irpc-derive` crate is a separate workspace member to avoid pulling proc-macro dependencies when the `derive` feature is disabled. This keeps the minimal configuration (no `rpc` feature) extremely lightweight: only serde, tokio, and tokio-util.

## Error Types

```rust
// irpc/src/lib.rs
pub enum RequestError {
    /// The response channel was dropped before a response was received.
    ResponseChannelClosed,
}

pub enum Error {
    /// Serialization/deserialization error.
    Serialize,
    /// IO error on the transport.
    Io(io::ErrorKind),
    /// The connection was closed.
    ConnectionClosed,
    /// Request error.
    Request(RequestError),
}
```

Source: `irpc/src/lib.rs:1`.

## Related Documents

- [Overview](../markdown/00-overview.md) — What irpc is
- [Service](../markdown/02-service.md) — Service trait
