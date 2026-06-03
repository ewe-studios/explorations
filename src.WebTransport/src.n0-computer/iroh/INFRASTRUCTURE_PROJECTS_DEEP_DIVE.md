# Infrastructure Projects: iroh-n0des, n0-future, and irpc

## Overview

This document explores the infrastructure-layer projects in the n0-computer ecosystem that provide foundational building blocks for decentralized applications:

1. **iroh-n0des** - Node management and metrics collection
2. **n0-future** - Async runtime abstractions for Wasm compatibility
3. **irpc** - Next-generation RPC framework building on quic-rpc

---

# Part 1: iroh-n0des - Node Management Protocol

## Overview

`iroh-n0des` implements a protocol for managing and monitoring iroh nodes, including metrics collection, authentication, and remote management capabilities.

**Repository:** https://github.com/n0-computer/iroh-n0des
**Version:** 0.1.0
**License:** MIT OR Apache-2.0
**Rust Version:** 1.85+

## Architecture

### Protocol Definition

Uses `irpc` for typed RPC protocol:

```rust
pub const ALPN: &[u8] = b"/iroh/n0des/1";

#[rpc_requests(N0desService, message = N0desMessage)]
#[derive(Debug, Serialize, Deserialize)]
pub enum N0desProtocol {
    #[rpc(tx=oneshot::Sender<()>)]
    Auth(Auth),

    #[rpc(tx=oneshot::Sender<RemoteResult<()>>)]
    PutMetrics(PutMetrics),

    #[rpc(tx=oneshot::Sender<Pong>)]
    Ping(Ping),
}
```

### Core Components

#### N0de Trait

Abstract interface for nodes:

```rust
pub trait N0de: 'static + Send + Sync {
    fn spawn(
        endpoint: Endpoint,
        metrics: &mut Registry,
    ) -> impl Future<Output = Result<Self>> + Send;

    fn shutdown(&mut self) -> impl Future<Output = Result<()>> + Send {
        async move { Ok(()) }
    }
}
```

#### Client

```rust
pub type N0desClient = irpc::Client<N0desMessage, N0desProtocol, N0desService>;

pub struct ClientBuilder {
    // Configuration options
}

impl ClientBuilder {
    pub fn build(self) -> Result<Client>;
}
```

### Message Types

#### Authentication

```rust
pub struct Auth {
    pub caps: Rcan<Caps>,  // Capability-based access
}
```

#### Metrics

```rust
pub struct PutMetrics {
    pub session_id: Uuid,
    pub update: iroh_metrics::encoding::Update,
}
```

#### Ping/Pong

```rust
pub struct Ping {
    pub req: [u8; 32],  // Random challenge
}

pub struct Pong {
    pub req: [u8; 32],  // Echoed challenge
}
```

### Capabilities System

Uses `rcan` for capability-based access control:

```rust
use rcan::Rcan;

#[derive(Debug, Serialize, Deserialize)]
pub enum Caps {
    ReadMetrics,
    WriteMetrics,
    Admin,
}
```

### Simulation Support

Macro-based simulation for testing:

```rust
use iroh_n0des::sim;

#[sim]
async fn test_node_behavior() {
    // Simulated node interactions
}
```

## Use Cases

### Metrics Collection

Nodes report metrics to central collectors:

```rust
let client = N0desClient::connect(ticket).await?;
client.put_metrics(session_id, metrics_update).await?;
```

### Remote Management

Authenticate and manage nodes remotely:

```rust
client.auth(capability).await?;
client.ping(challenge).await?;
```

### Health Monitoring

Periodic health checks:

```rust
async fn health_check(client: &N0desClient) -> bool {
    client.ping(random_bytes()).await.is_ok()
}
```

---

# Part 2: n0-future - Wasm-Compatible Async Runtime

## Overview

`n0-future` provides async runtime abstractions that work seamlessly across native Rust and WebAssembly targets, addressing the challenges of browser-based async programming.

**Repository:** https://github.com/n0-computer/n0-future
**Version:** 0.3.2
**License:** MIT OR Apache-2.0
**Rust Version:** 1.85+

## Motivation

### Challenges with Wasm Async

1. **No threads**: Browser Wasm can't spawn threads
2. **!Send types**: `JsValue` and related types are `!Send`
3. **Different timers**: `std::time::Instant` panics in browser
4. **Event loop**: Browser has its own event loop

### Solution

Provide unified API that abstracts these differences:

```rust
// Same code works on native and Wasm
use n0_future::task::spawn;
use n0_future::time::{sleep, Instant};

let handle = spawn(async {
    sleep(Duration::from_secs(1)).await;
    Instant::now()
});
```

## Architecture

### Module Structure

```
n0-future/
├── src/
│   ├── lib.rs           - Main exports
│   ├── task.rs          - Task spawning abstractions
│   ├── time.rs          - Time utilities
│   └── maybe_future.rs  - Future utilities
└── Cargo.toml
```

### Re-exported Crates

```rust
// Core futures utilities
pub use futures_lite;
pub use futures_buffered;
pub use futures_util;  // For Sink

// Platform-specific runtime
#[cfg(not(target_family = "wasm"))]
pub use tokio;

#[cfg(target_family = "wasm")]
pub use wasm_bindgen_futures as tokio;
```

## Key Abstractions

### Time Module

Unified time API across platforms:

```rust
pub mod time {
    #[cfg(not(target_family = "wasm"))]
    pub use tokio::time::{sleep, timeout, Instant, Interval, Sleep};

    #[cfg(target_family = "wasm")]
    pub use web_time::{Instant, SystemTime};
}
```

Usage:
```rust
use n0_future::time::{sleep, Instant};

let start = Instant::now();
sleep(Duration::from_millis(100)).await;
let elapsed = start.elapsed();
```

### Task Module

Unified task spawning:

```rust
pub mod task {
    #[cfg(not(target_family = "wasm"))]
    pub use tokio::task::{spawn, spawn_local, JoinHandle, JoinSet};

    #[cfg(target_family = "wasm")]
    pub use wasm_bindgen_futures::{spawn_local, JsFuture};

    pub use AbortOnDropHandle;
}
```

Usage:
```rust
use n0_future::task::spawn;

let handle = spawn(async {
    // Works on both platforms
    compute_something().await
});
```

### Boxed Futures

Handle Send vs !Send difference:

```rust
pub mod boxed {
    #[cfg(not(target_family = "wasm"))]
    pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

    #[cfg(target_family = "wasm")]
    pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + 'a>>;
}
```

### AbortOnDropHandle

Wrapper that aborts tasks when dropped:

```rust
pub struct AbortOnDropHandle<T> {
    inner: JoinHandle<T>,
}

impl<T> Drop for AbortOnDropHandle<T> {
    fn drop(&mut self) {
        self.inner.abort();
    }
}
```

## Platform Differences

### Native (tokio)

```rust
// Full tokio runtime
- Multi-threaded execution
- Send futures by default
- std::time for timing
- Full filesystem access
- TCP/UDP networking
```

### Wasm (wasm-bindgen-futures)

```rust
// Browser event loop
- Single-threaded
- !Send types (JsValue)
- web-time for timing
- No filesystem (use web APIs)
- WebTransport/WebRTC networking
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `serde` | Enable serde for SystemTime in Wasm |

## Usage Patterns

### Pattern 1: Platform-Agnostic Code

```rust
use n0_future::{task::spawn, time::sleep};

async fn my_function() {
    spawn(async {
        sleep(Duration::from_secs(1)).await;
        do_work().await
    });
}
```

### Pattern 2: Conditional Compilation

```rust
#[cfg(target_family = "wasm")]
use web_sys::Window;

#[cfg(not(target_family = "wasm"))]
use std::net::TcpStream;
```

### Pattern 3: Unified Timeouts

```rust
use n0_future::time::timeout;

let result = timeout(Duration::from_secs(5), async {
    do_something_slow().await
}).await;
```

## Integration with n0-computer Ecosystem

### iroh

Iroh uses n0-future for Wasm compatibility:

```rust
use n0_future::task::spawn;

// Iroh code works in browser and native
spawn(async { handle_connection().await });
```

### irpc

RPC framework uses n0-future abstractions:

```rust
use n0_future::FuturesUnordered;

let mut futures = FuturesUnordered::new();
```

### sendme/dumbpipe

CLI tools use n0-future for consistent async:

```rust
use n0_future::stream::StreamExt;

stream.next().await;
```

## Best Practices

### 1. Use n0-future for Cross-Platform Code

```rust
// Good: Cross-platform
use n0_future::time::Instant;

// Bad: Won't compile in Wasm
use std::time::Instant;
```

### 2. Be Aware of Send Bounds

```rust
// Native: Send
let x: Box<dyn Future + Send> = Box::new(async {});

// Wasm: Not Send
let x: Box<dyn Future> = Box::new(async {});

// Use n0_future::boxed for abstraction
use n0_future::boxed::BoxFuture;
```

### 3. Handle Browser Limitations

```rust
// Don't block the event loop
await sleep(Duration::ZERO).await;

// Use spawn_local in Wasm
spawn_local(async { /* work */ });
```

## Future Directions

Potential enhancements:
1. More comprehensive timer abstractions
2. Better error handling integration
3. Additional stream utilities
4. Performance optimizations

---

# Part 3: irpc - Next-Generation RPC Framework

## Overview

`irpc` is an RPC framework built for iroh, extending `quic-rpc` with additional features and tighter iroh integration.

**Repository:** https://github.com/n0-computer/irpc
**Version:** 0.5.0
**License:** Apache-2.0/MIT
**Rust Version:** 1.76+

## Goals

1. **Streaming RPC**: Full duplex streaming support
2. **Iroh Integration**: Native iroh transport support
3. **Type Safety**: Compile-time protocol verification
4. **Performance**: Minimal overhead
5. **Wasm Compatible**: Works with n0-future

## Architecture

### Core Types

```rust
pub struct Client<Msg, Protocol, Service> {
    // Client state
}

pub struct Server<Msg, Protocol, Service> {
    // Server state
}
```

### Protocol Definition

Using derive macros:

```rust
use irpc::{Service, channel::oneshot, rpc_requests};

#[rpc_requests(MyService, message = MyMessage)]
#[derive(Debug, Serialize, Deserialize)]
enum MyProtocol {
    #[rpc(tx = oneshot::Sender<Response>)]
    MyRequest(Request),

    #[rpc(tx = mpsc::Sender<StreamItem>)]
    MyStreamingRequest(StreamingRequest),
}
```

### Transport Layers

#### Quinn (QUIC) Transport

```rust
#[cfg(feature = "rpc")]
pub mod quinn {
    pub use iroh_quinn::*;
}
```

#### Channel Transport

```rust
pub mod channel {
    pub use tokio::sync::{mpsc, oneshot};
}
```

## Features

| Feature | Description |
|---------|-------------|
| `rpc` | Enable remote transport |
| `quinn_endpoint_setup` | Test utilities |
| `spans` | Tracing span propagation |
| `stream` | Stream support |
| `derive` | Derive macros |

## Message Patterns

### Request/Response

```rust
#[rpc(tx = oneshot::Sender<Result<Response>>)]
GetData(GetDataRequest)
```

### Server Streaming

```rust
#[rpc(tx = mpsc::Sender<Result<StreamItem>>)]
Subscribe(SubscribeRequest)
```

### Client Streaming

```rust
#[rpc(tx = oneshot::Sender<Result<Summary>>)]
Upload(mpsc::Receiver<Chunk>)
```

### Bidirectional

```rust
#[rpc(tx = mpsc::Sender<Result<StreamItem>>,
      rx = mpsc::Receiver<InputItem>)]
Chat(ChatInit)
```

## Usage Example

### Define Service

```rust
use irpc::{Service, rpc_requests};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct ComputeService;

impl Service for ComputeService {}

#[rpc_requests(ComputeService, message = ComputeMessage)]
#[derive(Debug, Serialize, Deserialize)]
enum ComputeProtocol {
    #[rpc(tx = oneshot::Sender<u64>)]
    Fibonacci(u32),

    #[rpc(tx = oneshot::Sender<String>)]
    Status(()),
}
```

### Server Implementation

```rust
async fn handle_compute(
    mut server: irpc::Server<_, _, ComputeService>,
) -> Result<()> {
    loop {
        let (msg, tx) = server.accept().await?;
        tokio::spawn(async move {
            match msg {
                ComputeMessage::Fibonacci(n, tx) => {
                    let result = fibonacci(n);
                    let _ = tx.send(result);
                }
                ComputeMessage::Status(_, tx) => {
                    let _ = tx.send("OK".to_string());
                }
            }
        });
    }
}

fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n-1) + fibonacci(n-2),
    }
}
```

### Client Usage

```rust
async fn use_compute(client: irpc::Client<_, _, ComputeService>) -> Result<()> {
    let result = client.rpc(ComputeMessage::Fibonacci(10)).await?;
    println!("fibonacci(10) = {}", result);

    let status = client.rpc(ComputeMessage::Status(())).await?;
    println!("Status: {}", status);

    Ok(())
}
```

## Comparison with quic-rpc

| Feature | quic-rpc | irpc |
|---------|----------|------|
| Base | Standalone | Built on quic-rpc |
| Iroh Integration | Manual | Native |
| Wasm Support | Limited | Via n0-future |
| Macro Support | Basic | Enhanced |
| Stream Types | Custom | futures-util |

## Performance

### Overhead

- Memory transport: ~100ns per message
- QUIC transport: Network latency + ~50μs
- Serialization: ~1μs per KB (postcard)

### Benchmarks

```rust
#[bench]
fn bench_local_rpc(b: &mut Bencher) {
    b.iter(|| {
        let (server, client) = irpc::local_channel::<MyService>();
        // ... benchmark code
    });
}
```

## Integration Patterns

### Pattern 1: Service Wrapper

```rust
pub struct MyApiClient {
    client: irpc::Client<Msg, Protocol, Service>,
}

impl MyApiClient {
    pub async fn get_data(&self, id: Id) -> Result<Data> {
        self.client.rpc(GetData(id)).await
    }
}
```

### Pattern 2: Event Streaming

```rust
pub async fn subscribe_events(
    client: &irpc::Client<_, _, Service>,
) -> Result<impl Stream<Item = Event>> {
    let (tx, rx) = mpsc::channel(100);
    client.rpc(SubscribeEvents(tx)).await?;
    Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
}
```

### Pattern 3: Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("RPC error: {0}")]
    Rpc(#[from] irpc::Error),
}
```

## Testing

### Local Testing

```rust
#[tokio::test]
async fn test_local_rpc() {
    let (server, client) = irpc::local_channel::<MyService>();

    // Test without network
    let response = client.rpc(Request).await.unwrap();
    assert_eq!(response, expected);
}
```

### Network Testing

```rust
#[tokio::test]
async fn test_remote_rpc() {
    let (server, client) = irpc::quinn_channel::<MyService>().unwrap();

    // Test with actual QUIC connection
    let response = client.rpc(Request).await.unwrap();
}
```

## Future Directions

Planned enhancements:
1. **Load Balancing**: Built-in client-side load balancing
2. **Circuit Breaking**: Automatic failure handling
3. **Retries**: Configurable retry policies
4. **Middleware**: Request/response interceptors
5. **OpenTelemetry**: Native tracing integration

---

## Conclusion

These three infrastructure projects form the foundation for building robust decentralized applications:

- **iroh-n0des** provides node management and observability
- **n0-future** enables cross-platform async code
- **irpc** offers type-safe RPC with streaming support

Together they enable building applications that work seamlessly across desktop, server, and browser environments.

## Related Resources

- [iroh-n0des Repository](https://github.com/n0-computer/iroh-n0des)
- [n0-future Repository](https://github.com/n0-computer/n0-future)
- [irpc Repository](https://github.com/n0-computer/irpc)
- [quic-rpc Documentation](https://docs.rs/quic-rpc)
- [WASM Bindgen Futures](https://rustwasm.github.io/wasm-bindgen/)
