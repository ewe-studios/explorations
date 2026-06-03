---
title: Protocol Dispatch — Router and ProtocolHandler
---

# Protocol Dispatch — Router and ProtocolHandler

The Router dispatches incoming QUIC connections to application-specific protocol handlers based on ALPN negotiation.

## The ProtocolHandler Trait

```rust
// iroh/src/protocol.rs
pub trait ProtocolHandler: Send + Sync + 'static {
    /// Called when a new connection is being accepted for this ALPN.
    async fn on_accepting(&self, connection: &noq::Connecting) -> Result<(), AcceptError> {
        Ok(())
    }

    /// Called when a connection is accepted and ready for handling.
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError>;

    /// Called when the router is shutting down.
    async fn shutdown(&self) {}
}
```

Source: `iroh/src/protocol.rs:1` — The `ProtocolHandler` trait has three methods: `on_accepting` (pre-accept hook), `accept` (main handler), and `shutdown` (cleanup).

**Key insight:** The `on_accepting` hook lets you reject connections BEFORE they are fully established. This is essential for rate limiting, IP bans, or any pre-connection validation. The `accept` method only fires after the connection is fully established and the ALPN has been negotiated.

## The Router Builder

```rust
// iroh/src/protocol.rs
pub struct RouterBuilder {
    endpoint: Endpoint,
    protocols: ProtocolMap,
    incoming_filter: Option<IncomingFilter>,
}

impl RouterBuilder {
    pub fn builder(endpoint: Endpoint) -> Self { ... }

    pub fn accept(
        mut self,
        alpn: Vec<u8>,
        handler: Arc<dyn ProtocolHandler>,
    ) -> Self {
        self.protocols.insert(alpn, handler);
        self
    }

    pub fn incoming_filter<F>(mut self, filter: F) -> Self
    where F: Fn(&Incoming) -> IncomingFilterOutcome + Send + Sync + 'static {
        self.incoming_filter = Some(Arc::new(filter));
        self
    }

    pub async fn spawn(self) -> Result<Router> { ... }
}
```

Source: `iroh/src/protocol.rs:1` — `RouterBuilder` collects protocol handlers and an optional incoming connection filter, then spawns the accept loop.

## The ProtocolMap

```rust
// iroh/src/protocol.rs
pub struct ProtocolMap(BTreeMap<Vec<u8>, Box<dyn DynProtocolHandler>>);
```

A `BTreeMap` from ALPN bytes to protocol handlers. `BTreeMap` is used (not `HashMap`) because the dispatch logic needs to find the longest matching ALPN prefix — lexicographic ordering matters.

Source: `iroh/src/protocol.rs:1` — `ProtocolMap::lookup()` finds the handler for a given ALPN.

## The IncomingFilter

```rust
// iroh/src/protocol.rs
pub enum IncomingFilterOutcome {
    Accept,   // Proceed with connection
    Retry,    // Defer and try again (backpressure)
    Reject,   // Actively reject with error
    Ignore,   // Silently drop
}

pub type IncomingFilter = Arc<dyn Fn(&Incoming) -> IncomingFilterOutcome + Send + Sync + 'static>;
```

The incoming filter runs on every incoming connection BEFORE ALPN negotiation. It can:
- **Accept** — proceed normally
- **Retry** — defer (useful for rate limiting under load)
- **Reject** — send an error to the remote
- **Ignore** — silently drop (useful for blocking bad actors)

Source: `iroh/src/protocol.rs:1` — `IncomingFilter` is called from the accept loop before protocol dispatch.

## Building and Running a Router

```rust
// Complete example from iroh/examples/echo.rs
const ALPN: &[u8] = b"iroh-example/echo/0";

let endpoint = Endpoint::bind().await?;

let router = Router::builder(endpoint)
    .accept(ALPN.to_vec(), Arc::new(Echo))
    .spawn()
    .await?;

#[derive(Debug, Clone)]
struct Echo;

impl ProtocolHandler for Echo {
    async fn accept(&self, connection: Connection) -> Result<()> {
        let (mut send, mut recv) = connection.accept_bi().await?;
        let bytes_sent = tokio::io::copy(&mut recv, &mut send).await?;
        send.finish()?;
        connection.closed().await;
        tracing::info!("echoed {} bytes", bytes_sent);
        Ok(())
    }
}
```

Source: `iroh/examples/echo.rs:1-113`

## Multiple Protocol Handlers

A single `Router` can handle multiple ALPNs:

```rust
let router = Router::builder(endpoint)
    .accept(b"myapp/echo/0".to_vec(), Arc::new(Echo))
    .accept(b"myapp/chat/0".to_vec(), Arc::new(Chat))
    .accept(b"myapp/sync/0".to_vec(), Arc::new(Sync))
    .spawn()
    .await?;
```

The router dispatches each incoming connection to the correct handler based on the ALPN negotiated during the QUIC handshake.

## Graceful Shutdown

```rust
// Shutdown the router
router.shutdown().await;

// The Router struct also implements Drop
drop(router); // triggers graceful shutdown
```

Source: `iroh/src/protocol.rs` — `Router::shutdown()` uses a `CancellationToken` to signal all protocol handlers to stop, then waits for the accept loop to complete.

## Connection Type

The `Connection` passed to `ProtocolHandler::accept` is a `noq::Connection` — a fully established QUIC connection with authenticated encryption.

```rust
// Opening streams on the connection
let (mut send, mut recv) = connection.open_bi().await?;  // bidirectional
let send = connection.open_uni().await?;                  // unidirectional send
let recv = connection.accept_uni().await?;               // unidirectional recv
let datagram = connection.send_datagram(bytes).await?;   // datagram
```

Source: `iroh/src/protocol.rs` — The `Connection` type is re-exported from `noq` and provides all QUIC stream operations.

## Error Handling

```rust
// iroh/src/protocol.rs
pub enum AcceptError {
    Connection(noq::ConnectionError),
    /// The connection was rejected by an incoming filter.
    Filtered(IncomingFilterOutcome),
    /// Custom error from protocol handler.
    Custom(Box<dyn std::error::Error + Send + Sync>),
}
```

Source: `iroh/src/protocol.rs:1` — `AcceptError` wraps connection errors, filter rejections, and custom protocol errors.

## Related Documents

- [Endpoint](../markdown/02-endpoint.md) — The Endpoint API
- [Data Flow](../markdown/09-data-flow.md) — Connection sequence diagram
- [TLS Layer](../markdown/06-tls.md) — How ALPN negotiation works at the TLS level
