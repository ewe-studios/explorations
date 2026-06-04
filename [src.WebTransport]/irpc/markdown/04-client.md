---
title: Client — Client API and WithChannels
---

# Client — Client API and WithChannels

The `Client` type is the main entry point for making RPC requests.

## Client

```rust
// irpc/src/lib.rs
pub struct Client<S: Service>(ClientInner<S::Message>, PhantomData<S>);

impl<S: Service> Client<S> {
    /// Create a client from a local sender.
    pub fn new(sender: LocalSender<S>) -> Self { ... }

    /// Create a client from an RPC transport.
    #[cfg(feature = "rpc")]
    pub fn from_rpc(transport: rpc::QuinnRpcTransport) -> Self { ... }

    /// Send a request with the specified channels.
    pub fn call<I: Channels<S>>(&self, request: I::Tx) -> WithChannels<I, S> { ... }
}
```

Source: `irpc/src/lib.rs:1`.

## WithChannels

```rust
// irpc/src/lib.rs
pub struct WithChannels<I: Channels<S>, S: Service> {
    request: I::Tx,
    service: PhantomData<S>,
}
```

`WithChannels` is the intermediate type returned by `Client::call()`. It holds the request and channel types, ready to be sent.

Source: `irpc/src/lib.rs:1`.

## Request/Response Flow

```rust
// Example: oneshot RPC
let client: Client<ComputeProtocol> = /* ... */;
let response = client
    .call(Multiply { a: 2, b: 3 })
    .oneshot()
    .await?;
assert_eq!(response, 6);
```

Source: `irpc/src/lib.rs:1` — Example from doc comments.

## Error Handling

```rust
// irpc/src/lib.rs
pub enum RequestError {
    ResponseChannelClosed,
}

pub enum Error {
    Serialize,
    Io(io::ErrorKind),
    ConnectionClosed,
    Request(RequestError),
}
```

Source: `irpc/src/lib.rs:1`.

## Related Documents

- [Local](../markdown/06-local.md) — LocalSender for in-process
- [RPC Transport](../markdown/05-rpc-transport.md) — Quinn transport for remote
