---
title: Local — In-Process Transport via mpsc Channels
---

# Local — In-Process Transport via mpsc Channels

The `LocalSender` provides in-process communication without any serialization overhead.

## LocalSender

```rust
// irpc/src/lib.rs
pub struct LocalSender<S: Service>(tokio::sync::mpsc::Sender<S::Message>);

impl<S: Service> LocalSender<S> {
    pub fn new(sender: tokio::sync::mpsc::Sender<S::Message>) -> Self { ... }
}
```

Source: `irpc/src/lib.rs:1`.

## Zero Serialization Overhead

When using `LocalSender`, messages are sent directly through the tokio mpsc channel — no postcard serialization, no length-prefix encoding, no network I/O. This makes irpc suitable as a replacement for manual mpsc channel patterns within a single process.

**Aha:** This is the key design insight of irpc — the same `Client<S>` API works identically whether backed by a `LocalSender` (zero overhead, in-process) or a `QuinnRpcTransport` (network RPC). The service definition doesn't change; only the transport does.

## Usage Pattern

```rust
// Server side: receive messages from channel
let (tx, rx) = tokio::sync::mpsc::channel(32);
let local = LocalSender::new(tx);
let client = Client::new(local);

// Client side: make requests
let response = client.call(Multiply { a: 2, b: 3 }).oneshot().await?;
```

Source: `irpc/src/lib.rs:1` — Example from doc comments.

## Related Documents

- [RPC Transport](../markdown/05-rpc-transport.md) — Remote transport
- [Client](../markdown/04-client.md) — Client API
