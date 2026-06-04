---
title: Service — Service Trait, RpcMessage, and Channels
---

# Service — Service Trait, RpcMessage, and Channels

The Service trait defines an RPC protocol as a message enum.

## Service Trait

```rust
// irpc/src/lib.rs
pub trait Service: Serialize + DeserializeOwned + Send + Sync + Debug + 'static {
    /// The message enum type for this service.
    type Message: RpcMessage;
}
```

A `Service` is defined by its message enum. Each variant of the enum represents a different RPC request type.

Source: `irpc/src/lib.rs:1`.

## RpcMessage Trait

```rust
// irpc/src/lib.rs
pub trait RpcMessage: Debug + Serialize + DeserializeOwned + Send + Sync + Unpin + 'static {}
```

All RPC messages must implement `RpcMessage`. This is automatically satisfied by types that derive `Serialize`, `Deserialize`, `Debug`, and are `Send + Sync`.

Source: `irpc/src/lib.rs:1`.

## Channels Trait

```rust
// irpc/src/lib.rs
pub trait Channels<S: Service>: Send + 'static {
    type Tx: Sender;
    type Rx: Receiver;
}
```

`Channels` defines the request/response channel types for a service. The `Tx` type sends requests, and `Rx` receives responses.

Source: `irpc/src/lib.rs:1`.

## Service Definition Example

```rust
// Using the derive macro (see 07-derive-macro.md for details)
#[rpc_requests(message = ComputeMessage)]
#[derive(Debug, Serialize, Deserialize)]
enum ComputeProtocol {
    /// Multiply two numbers.
    #[rpc(tx=oneshot::Sender<i64>)]
    Multiply(Multiply),
    /// Sum numbers from stream, reply with updating sum.
    #[rpc(tx=mpsc::Sender<i64>, rx=mpsc::Receiver<i64>)]
    Sum(SumInput),
}

#[derive(Debug, Serialize, Deserialize)]
struct Multiply { a: i64, b: i64 }

#[derive(Debug, Serialize, Deserialize)]
struct SumInput;
```

Source: `irpc/src/lib.rs:1` — Example from doc comments.

## Related Documents

- [Derive Macro](../markdown/07-derive-macro.md) — #[rpc_requests] implementation
- [Channels](../markdown/03-channels.md) — Channel types in detail
