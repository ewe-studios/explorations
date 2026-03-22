# Quic-Rpc: RPC Over QUIC Transport

## Overview

`quic-rpc` is a streaming RPC (Remote Procedure Call) system built on top of QUIC, designed for efficient communication between Rust services. It provides a type-safe way to define service boundaries with support for various interaction patterns including streaming.

**Version:** 0.3.2+
**Repository:** https://github.com/n0-computer/quic-rpc
**License:** MIT OR Apache-2.0
**Authors:** Rüdiger Klaehn and the n0 team

## Design Philosophy

### Goals

1. **Streaming Support**: Not just request/response, but full duplex streaming
2. **Transport Flexibility**: Memory transport for local, QUIC for remote
3. **Type Safety**: Service definitions in the Rust type system
4. **Async-First**: Built for tokio runtime
5. **Low Overhead**: Minimal overhead compared to manual channel-based isolation

### Non-Goals

- Cross-language interoperability (Rust-only)
- Automatic versioning
- Transparent remote calls (doesn't hide the network)
- Runtime agnosticism (tokio-focused)

## Supported Interaction Patterns

Quic-Rpc supports four interaction patterns:

### 1. Request/Response (1 req → 1 res)

Standard RPC pattern:
```rust
impl RpcMsg<PingService> for Ping {
    type Response = Pong;
}
```

### 2. Request with Update Stream (1 req, update stream → 1 res)

Client sends updates while server processes:
```rust
impl ServerStreaming<PingService> for PingWithUpdates {
    type Update = UpdateMessage;
    type Response = FinalResult;
}
```

### 3. Response Streaming (1 req → res stream)

Server streams multiple responses:
```rust
impl ClientStreaming<PingService> for Subscribe {
    type Response = Stream<Item>;
}
```

### 4. Bidirectional Streaming (1 req, update stream → res stream)

Full duplex communication:
```rust
impl BidiStreaming<PingService> for Chat {
    type Update = ChatMessage;
    type Response = ChatMessage;
}
```

## Architecture

### Core Types

#### Service Trait

Defines a service's request/response types:

```rust
pub trait Service: Send + Sync + Debug + Clone + 'static {
    type Req: RpcMessage;
    type Res: RpcMessage;
}
```

#### RpcMessage Trait

Requirements for RPC messages:

```rust
pub trait RpcMessage:
    Debug + Serialize + DeserializeOwned + Send + Sync + Unpin + 'static
{}
```

#### RpcError Trait

Error type requirements:

```rust
pub trait RpcError:
    Debug + Display + Into<anyhow::Error> + Send + Sync + Unpin + 'static
{}
```

### Client/Server Model

#### RpcClient

```rust
pub struct RpcClient<S: Service, C: Connector<S>> {
    connector: C,
    _phantom: PhantomData<S>,
}

impl<S, C> RpcClient<S, C>
where
    S: Service,
    C: Connector<S>,
{
    pub fn new(connector: C) -> Self;
    pub async fn rpc<M>(&mut self, msg: M) -> Result<M::Response>
    where
        M: RpcMsg<S>;
}
```

#### RpcServer

```rust
pub struct RpcServer<S: Service, L: Listener<S>> {
    listener: L,
    _phantom: PhantomData<S>,
}

impl<S, L> RpcServer<S, L>
where
    S: Service,
    L: Listener<S>,
{
    pub fn new(listener: L) -> Self;
    pub async fn accept(&mut self) -> Result<IncomingRequest<S>>;
}
```

## Transports

### Flume (Memory) Transport

Zero-overhead in-process transport using flume channels:

```rust
use quic_rpc::transport::flume;

let (server, client) = flume::channel(10); // buffer size
```

**Characteristics:**
- No serialization/deserialization
- Minimal overhead (2 flume channels)
- Same API as remote transport
- Good for testing and local subsystem isolation

### Quinn (QUIC) Transport

Network transport using QUIC:

```rust
use quic_rpc::transport::quinn;

let server_endpoint = make_server_endpoint(bind_addr)?;
let listener = QuinnListener::new(server_endpoint)?;
```

**Characteristics:**
- Connection multiplexing
- NAT traversal support
- TLS encryption
- Low latency for small messages

### Hyper (HTTP/2) Transport

HTTP/2 based transport:

```rust
use quic_rpc::transport::hyper;
```

**Characteristics:**
- Higher throughput for bulk data
- Better for persistent connections
- More mature ecosystem tooling

## Usage Example

### Defining a Service

```rust
use derive_more::{From, TryInto};
use quic_rpc::{message::RpcMsg, Service};
use serde::{Deserialize, Serialize};

// Messages
#[derive(Debug, Serialize, Deserialize)]
struct Ping;

#[derive(Debug, Serialize, Deserialize)]
struct Pong;

// Service definition
#[derive(Debug, Clone)]
struct PingService;

#[derive(Debug, Serialize, Deserialize, From, TryInto)]
enum PingRequest {
    Ping(Ping),
}

#[derive(Debug, Serialize, Deserialize, From, TryInto)]
enum PingResponse {
    Pong(Pong),
}

impl Service for PingService {
    type Req = PingRequest;
    type Res = PingResponse;
}

impl RpcMsg<PingService> for Ping {
    type Response = Pong;
}
```

### Server Implementation

```rust
#[derive(Debug, Clone, Copy)]
struct Handler;

impl Handler {
    async fn ping(self, _req: Ping) -> Pong {
        Pong
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (listener, _connector) = quic_rpc::transport::flume::channel(1);
    let mut server = RpcServer::<PingService, _>::new(listener);

    let handler = Handler;
    loop {
        let (msg, chan) = server.accept().await?.read_first().await?;
        match msg {
            PingRequest::Ping(ping) => {
                chan.rpc(ping, handler, Handler::ping).await?
            }
        }
    }
}
```

### Client Implementation

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (_listener, connector) = quic_rpc::transport::flume::channel(1);
    let mut client = RpcClient::<PingService, _>::new(connector);

    let response = client.rpc(Ping).await?;
    println!("Got response: {:?}", response);

    Ok(())
}
```

## Macros

Quic-Rpc provides derive macros to reduce boilerplate:

### rpc_requests Macro

```rust
use quic_rpc::rpc_requests;

#[rpc_requests(MyService, message = MyMessage)]
#[derive(Debug, Serialize, Deserialize)]
enum MyProtocol {
    #[rpc(tx = oneshot::Sender<Result<Response>>)]
    MyRequest(RequestType),

    #[rpc(tx = mpsc::Sender<StreamResponse>)]
    MyStreamingRequest(StreamingRequestType),
}
```

This generates:
- Service implementation
- Message enum variants
- Type-safe channel senders

## Error Handling

### Error Types

```rust
pub enum RpcError {
    /// Serialization error
    Serialize(Box<dyn std::error::Error + Send + Sync>),
    /// Deserialization error
    Deserialize(Box<dyn std::error::Error + Send + Sync>),
    /// Connection error
    Connect(Box<dyn std::error::Error + Send + Sync>),
    /// Server error
    Server(Box<dyn std::error::Error + Send + Sync>),
}
```

### Error Propagation

```rust
async fn handle_request(
    &self,
    msg: RequestType,
) -> Result<ResponseType, RpcError> {
    // Error automatically converted to RpcError
    let result = self.do_work(msg).await?;
    Ok(result)
}
```

## Pattern Implementations

### RpcMsg (Request/Response)

```rust
pub trait RpcMsg<S: Service>: RpcMessage {
    type Response: RpcMessage;
}
```

### ServerStreaming

```rust
pub trait ServerStreaming<S: Service>: RpcMessage {
    type Response: RpcMessage + Stream<Item = Self::Item>;
    type Item: RpcMessage;
}
```

### ClientStreaming

```rust
pub trait ClientStreaming<S: Service>: RpcMessage {
    type Update: RpcMessage;
    type Response: RpcMessage;
}
```

### BidiStreaming

```rust
pub trait BidiStreaming<S: Service>: RpcMessage {
    type Update: RpcMessage;
    type Response: RpcMessage + Stream<Item = Self::Item>;
    type Item: RpcMessage;
}
```

## Testing

### Test Utilities

```rust
#[cfg(feature = "test-utils")]
pub fn quinn_channel<S: Service>() -> anyhow::Result<(
    RpcServer<S, server::QuinnListener<S>>,
    RpcClient<S, client::QuinnConnector<S>>,
)>
```

Creates a local QUIC connection for realistic testing.

### Mock Testing

```rust
use quic_rpc::flume_channel;

#[tokio::test]
async fn test_ping_pong() {
    let (server, mut client) = flume_channel::<PingService>(1);

    let response = client.rpc(Ping).await.unwrap();
    assert!(matches!(response, Pong));
}
```

## Performance Considerations

### Memory Transport Overhead

For in-process communication:
- 2 flume channels per RPC interaction
- No serialization overhead
- Comparable to manual channel implementation

### QUIC Transport Characteristics

- **Latency**: Sub-millisecond for small messages
- **Throughput**: Limited by network, not RPC overhead
- **Connection pooling**: Reuses connections automatically

### Serialization

Using Postcard for serialization:
- Compact binary format
- Zero-copy where possible
- Serde-compatible

## Integration with n0-computer Ecosystem

### iroh-n0des

Uses quic-rpc for node communication:

```rust
use irpc::{Service, channel::oneshot, rpc_requests};

#[rpc_requests(N0desService, message = N0desMessage)]
pub enum N0desProtocol {
    #[rpc(tx=oneshot::Sender<()>)]
    Auth(Auth),
    #[rpc(tx=oneshot::Sender<RemoteResult<()>>)]
    PutMetrics(PutMetrics),
}
```

### irpc

The `irpc` crate builds on quic-rpc with iroh-specific extensions

## Feature Flags

| Feature | Description |
|---------|-------------|
| `flume-transport` | Enable memory transport |
| `quinn-transport` | Enable QUIC transport |
| `hyper-transport` | Enable HTTP/2 transport |
| `macros` | Enable derive macros |
| `test-utils` | Testing utilities |
| `spans` | Tracing span propagation |

## Best Practices

### Service Design

1. **Keep services focused**: Single responsibility
2. **Use enums for requests**: Easy to extend
3. **Define clear error types**: Specific error cases
4. **Consider streaming early**: For large data

### Message Design

1. **Use newtypes**: Type safety for primitives
2. **Version manually**: Include version in messages if needed
3. **Avoid Option where possible**: Use explicit variants
4. **Consider serde aliases**: For backward compatibility

### Error Handling

1. **Use anyhow for servers**: Flexible error handling
2. **Specific errors for clients**: Handle known cases
3. **Log at server boundary**: Debugging assistance
4. **Propagate context**: Use anyhow's context

## Comparison with Alternatives

| Feature | quic-rpc | tonic/gRPC | TTRPC |
|---------|----------|-----------|-------|
| Language | Rust-only | Multi-lang | Multi-lang |
| Transport | QUIC/HTTP2/Mem | HTTP/2 | Unix socket |
| Streaming | Full support | Full support | Limited |
| Runtime | Tokio | Tokio | Tokio |
| Overhead | Minimal | Protobuf | Protobuf |

## Future Directions

Potential improvements:
1. Additional transport backends (WebTransport)
2. Built-in rate limiting
3. Automatic reconnection strategies
4. Enhanced tracing integration
5. Load balancing support

## Conclusion

Quic-Rpc provides a well-designed RPC framework that:
- Balances flexibility with type safety
- Supports multiple transport backends transparently
- Enables clean service boundaries in Rust applications
- Is optimized for the n0-computer ecosystem's needs

Its design philosophy of "optional remotability" allows using the same interface for both in-process and distributed architectures.

## Related Resources

- [Quic-Rpc Documentation](https://docs.rs/quic-rpc)
- [QUIC RFC 9000](https://www.rfc-editor.org/rfc/rfc9000.html)
- [Flume Channels](https://docs.rs/flume)
- [Quinn QUIC](https://docs.rs/quinn)
