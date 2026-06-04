---
title: Channels — oneshot, mpsc, and none Channel Types
---

# Channels — oneshot, mpsc, and none Channel Types

irpc provides three channel types for request/response communication.

## Channel Types

| Type | Purpose | Capacity | Use Case |
|------|---------|----------|----------|
| `oneshot::Sender<T>` / `oneshot::Receiver<T>` | Single message | 1 | RPC (1 request, 1 response) |
| `mpsc::Sender<T>` / `mpsc::Receiver<T>` | Multiple messages | Bounded | Streaming responses |
| `none::NoSender` / `none::NoReceiver` | Disabled | 0 | Fire-and-forget |

Source: `irpc/src/lib.rs:1` — `channel` module.

## oneshot Channel

```rust
// irpc/src/lib.rs
pub mod oneshot {
    pub struct Sender<T>(tokio::sync::oneshot::Sender<T>);
    pub struct Receiver<T>(tokio::sync::oneshot::Receiver<T>);
}
```

Wraps `tokio::sync::oneshot` for single-value communication.

Source: `irpc/src/lib.rs:1`.

## mpsc Channel

```rust
// irpc/src/lib.rs
pub mod mpsc {
    pub struct Sender<T>(tokio_util::sync::PollSender<T>);
    pub struct Receiver<T>(mpsc::ReceiverInner<T>);
}
```

Uses `tokio_util::sync::PollSender` instead of tokio's `mpsc::Sender` because PollSender implements the `Sink` trait needed for stream processing.

Source: `irpc/src/lib.rs:1`.

## none Channel (Disabled)

```rust
// irpc/src/lib.rs
pub mod none {
    pub struct NoSender;
    pub struct NoReceiver;
}
```

Zero-cost disabled channels. When a channel is `none`, no serialization or sending occurs for that direction.

Source: `irpc/src/lib.rs:1`.

## Channel Composition

Each request can have any combination of channels:

```rust
// oneshot response only
#[rpc(tx=oneshot::Sender<Result>)]

// mpsc response (streaming)
#[rpc(tx=mpsc::Sender<Result>)]

// both request and response mpsc (bidi)
#[rpc(tx=mpsc::Sender<Input>, rx=mpsc::Receiver<Output>)]

// no response (fire-and-forget)
#[rpc(tx=none::NoSender)]
```

Source: `irpc/src/lib.rs:1` — `#[rpc]` attribute usage.

## Sender and Receiver Traits

```rust
// irpc/src/lib.rs
pub trait Sender: Debug + Sealed {}
pub trait Receiver: Debug + Sealed {}
```

The `Sealed` trait prevents external implementations. Only the built-in channel types implement these traits.

Source: `irpc/src/lib.rs:1`.

## Related Documents

- [Service](../markdown/02-service.md) — How channels are used in services
- [Client](../markdown/04-client.md) — Client channel usage
