# Tarpc Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC/tarpc
repository: https://github.com/google/tarpc
explored_at: 2026-03-23

## Overview

tarpc is an RPC framework for Rust with a focus on ease of use. Unlike other RPC frameworks that use schema files (like .proto or .capnp), tarpc defines the service schema directly in Rust code using procedural macros.

## Project Structure

```
tarpc/
├── tarpc/                 # Main crate
│   ├── src/
│   │   ├── lib.rs         # Library root
│   │   ├── client/        # Client implementation
│   │   │   ├── mod.rs
│   │   │   ├── stub.rs    # Client stub
│   │   │   └── in_flight_requests.rs
│   │   ├── server/        # Server implementation
│   │   │   ├── mod.rs
│   │   │   ├── incoming.rs
│   │   │   ├── limits.rs
│   │   │   └── request_hook.rs
│   │   ├── transport.rs   # Transport traits
│   │   ├── context.rs     # Request context
│   │   ├── trace.rs       # Tracing support
│   │   ├── serde_transport.rs  # Serde serialization
│   │   └── cancellations.rs    # Request cancellation
│   ├── examples/
│   │   ├── readme.rs      # Basic example
│   │   ├── compression.rs # Compression example
│   │   ├── pubsub.rs      # Pub/sub pattern
│   │   ├── tracing.rs     # Distributed tracing
│   │   ├── tls_over_tcp.rs # TLS example
│   │   └── custom_transport.rs
│   └── Cargo.toml
├── plugins/               # Procedural macros
│   ├── src/
│   │   └── lib.rs         # tarpc::service macro
│   └── Cargo.toml
├── example-service/       # Complete example
│   ├── src/
│   │   ├── client.rs
│   │   ├── server.rs
│   │   └── lib.rs
│   └── Cargo.toml
├── hooks/                 # Git hooks
├── README.md
├── RELEASES.md
└── CONTRIBUTING.md
```

## Cargo Configuration

```toml
[package]
name = "tarpc"
version = "0.37.0"
edition = "2024"
rust-version = "1.85.0"
license = "MIT"

[features]
default = []
serde1 = ["tarpc-plugins/serde1", "serde"]
tokio1 = ["tokio/rt"]
serde-transport = ["serde1", "tokio1", "tokio-serde"]
serde-transport-json = ["serde-transport", "tokio-serde/json"]
serde-transport-bincode = ["serde-transport", "tokio-serde/bincode"]
tcp = ["tokio/net"]
unix = ["tokio/net"]

full = [
    "serde1", "tokio1", "serde-transport",
    "serde-transport-json", "serde-transport-bincode",
    "tcp", "unix"
]

[dependencies]
anyhow = "1.0"
futures = "0.3"
tokio = { version = "1", features = ["time"] }
tokio-util = { version = "0.7", features = ["time"] }
tarpc-plugins = { path = "../plugins", version = "0.14" }
tracing = { version = "0.1", features = ["attributes", "log"] }
tracing-opentelemetry = "0.32"
opentelemetry = "0.31"
pin-project = "1.1"
thiserror = "2.0"

# Optional
serde = { optional = true, version = "1.0", features = ["derive"] }
tokio-serde = { optional = true, version = "0.9" }
```

## Service Definition

### Basic Service

```rust
use tarpc::service;

// Define service with macro
#[tarpc::service]
trait World {
    /// Returns a greeting for name
    async fn hello(name: String) -> String;
}
```

### Generated Code

The `#[tarpc::service]` macro generates:

```rust
// Trait to implement
trait World {
    async fn hello(self, context: Context, name: String) -> String;
}

// Client struct
struct WorldClient<T> {
    transport: T,
    config: client::Config,
}

impl<T> WorldClient<T> {
    fn new(config: client::Config, transport: T) -> Self;
    fn spawn(self) -> WorldClient<SpawnedClient<T>>;
}

// Client methods
impl WorldClient<SpawnedClient<...>> {
    async fn hello(&mut self, context: Context, name: String)
        -> Result<String, ClientError>;
}

// Server trait
trait WorldServe {
    fn serve(self) -> impl Stream<Item = Request>;
}

// Request/Response types
struct HelloRequest {
    context: Context,
    name: String,
}

struct HelloResponse(String);
```

### Multiple Methods

```rust
#[tarpc::service]
trait Calculator {
    async fn add(a: i64, b: i64) -> i64;
    async fn subtract(a: i64, b: i64) -> i64;
    async fn multiply(a: i64, b: i64) -> i64;
    async fn divide(a: i64, b: i64) -> Result<i64, String>;
}
```

### Service with Derives

```rust
#[tarpc::service(derive = [Clone, Debug])]
trait MyService {
    async fn do_something() -> Result<(), String>;
}
```

## Server Implementation

### Basic Server

```rust
use tarpc::{
    context,
    server::{self, Channel},
};
use futures::prelude::*;

#[derive(Clone)]
struct HelloServer;

impl World for HelloServer {
    async fn hello(self, _: context::Context, name: String) -> String {
        format!("Hello, {name}!")
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (client_transport, server_transport) =
        tarpc::transport::channel::unbounded();

    let server = server::BaseChannel::with_defaults(server_transport);

    tokio::spawn(
        server.execute(HelloServer.serve())
            .for_each(|response| async move {
                tokio::spawn(response);
            })
    );

    Ok(())
}
```

### Server with Channels

```rust
use tarpc::server::{BaseChannel, Channel};

// Create channel with limits
let channel = BaseChannel::new(
    server::Config {
        pending_request_buffer: 100,
        request_timeout: Duration::from_secs(10),
    },
    transport,
);

// Handle requests
channel.execute(server.serve())
    .for_each_concurrent(10, |response| async move {
        tokio::spawn(response);
    });
```

### Request Hooks

```rust
use tarpc::server::request_hook::{Before, After, BeforeAndAfter};

// Before hook
struct LogBefore;

impl Before for LogBefore {
    async fn before(&self, req: &Request) {
        tracing::info!("Received request: {:?}", req);
    }
}

// After hook
struct LogAfter;

impl After for LogAfter {
    async fn after(&self, req: &Request, resp: &Response) {
        tracing::info!("Sent response: {:?}", resp);
    }
}

// Apply hooks
let hooked_server = server.with_hooks(LogBefore, LogAfter);
```

### Rate Limiting

```rust
use tarpc::server::limits::{requests_per_channel, channels_per_key};

// Limit requests per channel
let limited = server
    .with_limits(requests_per_channel::Max(10));

// Limit channels per key
let keyed = server
    .with_limits(channels_per_key::Max(100));
```

## Client Implementation

### Basic Client

```rust
use tarpc::client;

// Create client
let mut client = WorldClient::new(
    client::Config::default(),
    client_transport,
).spawn();

// Make RPC call
let greeting = client.hello(context::current(), "Alice".to_string()).await?;
println!("{}", greeting);
```

### Client Configuration

```rust
let config = client::Config {
    // How many requests to buffer
    request_buffer_capacity: 100,

    // Default request timeout
    request_timeout: Duration::from_secs(10),
};

let client = WorldClient::new(config, transport).spawn();
```

### Request Context

```rust
use tarpc::context;

// Create context with deadline
let mut ctx = context::current();
ctx.deadline = Instant::now() + Duration::from_secs(5);

// Add trace context
ctx.trace_context = tracing::span::Id::from_u64(12345);

// Make call with context
let result = client.hello(ctx, "Bob".to_string()).await?;
```

### Cancellation

```rust
// Dropping request cancels it
{
    let future = client.hello(context::current(), "Cancel".to_string());
    // Future dropped without await - sends cancellation
}

// Server receives cancellation and can stop work
```

## Transport

### In-Process Channel

```rust
use tarpc::transport::channel;

// Unbounded channel
let (tx, rx) = channel::unbounded();

// Bounded channel
let (tx, rx) = channel::bounded(100);
```

### TCP Transport

```rust
#[cfg(feature = "tcp")]
use tarpc::serde_transport::tcp;

// Connect to server
let transport = tcp::connect("localhost:5000", Msg::default).await?;

// Or listen for connections
let listener = tcp::listener::bind("0.0.0.0:5000", Msg::default).await?;

while let Some((transport, addr)) = listener.next().await {
    // Handle connection
}
```

### TLS over TCP

```rust
#[cfg(feature = "tls")]
use tokio_rustls::TlsConnector;

// Create TLS connector
let connector = TlsConnector::from(config);

// Connect with TLS
let stream = TcpStream::connect("localhost:443").await?;
let tls_stream = connector.connect(domain, stream).await?;

// Use with tarpc
let transport = serde_transport::new(tls_stream, Msg::default);
```

### Unix Domain Sockets

```rust
#[cfg(feature = "unix")]
use tarpc::serde_transport::unix;

// Connect via Unix socket
let transport = unix::connect("/tmp/tarpc.sock", Msg::default).await?;
```

### Custom Transport

```rust
use tarpc::transport::{Transport, Sink, Stream};

// Implement custom transport
struct MyTransport {
    // Custom fields
}

impl Stream<Item = Result<Request, Error>> for MyTransport {
    // ...
}

impl Sink<Response, Error = Error> for MyTransport {
    // ...
}

// Use with client/server
let client = WorldClient::new(config, MyTransport).spawn();
```

## Serialization

### Serde Integration

```rust
// Enable serde feature
#[tarpc::service(derive_serde)]
trait MyService {
    async fn process(data: Vec<u8>) -> Result<Vec<u8>, String>;
}
```

### JSON Serialization

```rust
use tarpc::serde_transport::json;

// JSON transport
let transport = json::connect("localhost:5000").await?;
```

### Bincode Serialization

```rust
use tarpc::serde_transport::bincode;

// Bincode transport (more efficient than JSON)
let transport = bincode::connect("localhost:5000").await?;
```

### Custom Serialization

```rust
use tokio_serde::{SymmetricallyFramed, Serializer, Deserializer};

struct MyCodec;

impl Serializer<MyRequest> for MyCodec {
    // ...
}

impl Deserializer<MyResponse> for MyCodec {
    // ...
}

let transport = SymmetricallyFramed::new(
    FramedRead::new(io, MyCodec),
    FramedWrite::new(io, MyCodec),
);
```

## Distributed Tracing

### OpenTelemetry Integration

```rust
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry::sdk::export::trace::stdout;

// Setup tracing
let tracer = stdout::new_pipeline().install_simple();
let telemetry = OpenTelemetryLayer::new(tracer);

// Subscribe to traces
tracing_subscriber::registry()
    .with(telemetry)
    .init();
```

### Trace Propagation

```rust
// Context automatically includes trace info
let ctx = context::current();
// ctx.trace_context contains span/trace IDs

// Server receives context and continues trace
async fn handle(ctx: context::Context, req: Request) {
    let span = tracing::span!(Level::INFO, "rpc_call");
    span.follows_from(ctx.trace_context);
    // ...
}
```

### Example with Jaeger

```rust
use opentelemetry_jaeger::new_pipeline;

// Send traces to Jaeger
let tracer = new_pipeline()
    .with_service_name("tarpc-server")
    .with_agent_endpoint("localhost:6831")
    .install_batch(opentelemetry::runtime::Tokio)?;

// Use tracer
tracing_subscriber::registry()
    .with(tracing_opentelemetry::layer().with_tracer(tracer))
    .init();
```

## Error Handling

### Client Errors

```rust
use tarpc::client::Error;

enum Error {
    /// Request timed out
    RequestTimeout,

    /// Transport error
    Transport(TransportError),

    /// Server returned error
    ServerReturnedError(ResponseError),

    /// Request was cancelled
    Cancelled,
}
```

### Server Errors

```rust
use tarpc::server::Error;

enum Error {
    /// Client disconnected
    ClientDisconnected,

    /// Request processing failed
    ProcessingFailed,
}
```

### Custom Error Types

```rust
#[derive(Debug, thiserror::Error)]
enum MyError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error")]
    Internal(#[from] anyhow::Error),
}

#[tarpc::service]
trait MyService {
    async fn do_work(input: String) -> Result<String, MyError>;
}
```

## Examples

### Hello World

```rust
// Service definition
#[tarpc::service]
trait World {
    async fn hello(name: String) -> String;
}

// Server
#[derive(Clone)]
struct HelloServer;

impl World for HelloServer {
    async fn hello(self, _: Context, name: String) -> String {
        format!("Hello, {name}!")
    }
}

// Client
let greeting = client.hello(context::current(), "World".to_string()).await?;
```

### Pub/Sub Pattern

```rust
#[tarpc::service]
trait PubSub {
    async fn subscribe(topic: String) -> Vec<String>;
    async fn publish(topic: String, message: String);
}

struct PubSubServer {
    subscribers: Arc<DashMap<String, Vec<IpcSender<String>>>>,
}

impl PubSub for PubSubServer {
    async fn subscribe(self, _: Context, topic: String) -> Vec<String> {
        // Return recent messages
    }

    async fn publish(self, _: Context, topic: String, message: String) {
        // Broadcast to subscribers
    }
}
```

### Compression

```rust
use flate2::write::GzEncoder;
use flate2::Compression;

// Compress large responses
async fn get_large_data(self, _: Context) -> Result<Vec<u8>, Error> {
    let data = fetch_data();

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&data)?;
    Ok(encoder.finish()?)
}
```

## Testing

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hello() {
        let server = HelloServer;
        let ctx = context::current();

        let result = server.hello(ctx, "Test".to_string()).await;
        assert_eq!(result, "Hello, Test!");
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_rpc_call() {
    // Create in-process transport
    let (client_tx, server_rx) = tarpc::transport::channel::unbounded();
    let (server_tx, client_rx) = tarpc::transport::channel::unbounded();

    // Spawn server
    let server = HelloServer;
    tokio::spawn(async move {
        let channel = BaseChannel::with_defaults(server_rx);
        channel.execute(server.serve())
            .for_each(|resp| async { tokio::spawn(resp) })
            .await;
    });

    // Create client
    let mut client = WorldClient::new(
        client::Config::default(),
        client_tx,
    ).spawn();

    // Make call
    let result = client.hello(context::current(), "Test".to_string()).await;
    assert_eq!(result.unwrap(), "Hello, Test!");
}
```

## Performance Considerations

### Connection Pooling

```rust
use tarpc::client::stub::retry::RetryPolicy;

// Use connection pool
let pool = tarpc::client::Pool::new(
    || async { create_client().await },
    RetryPolicy::default(),
);

let client = pool.client().await?;
```

### Request Batching

```rust
// Batch multiple requests
let batch = futures::future::join_all(vec![
    client.method1(ctx.clone(), arg1),
    client.method2(ctx.clone(), arg2),
    client.method3(ctx.clone(), arg3),
]);

let results = batch.await;
```

### Backpressure

```rust
use tarpc::server::Config;

// Configure backpressure
let config = Config {
    pending_request_buffer: 100,  // Max pending requests
    request_timeout: Duration::from_secs(10),
};

let channel = BaseChannel::new(config, transport);
```

## Comparison with Other Rust RPC

| Feature | tarpc | capnp-rust | tonic |
|---------|-------|------------|-------|
| Schema | Rust traits | .capnp files | .proto files |
| Codegen | Proc macro | External tool | External tool |
| Serialization | Serde | Cap'n Proto | Protobuf |
| Zero-copy | No | Yes | No |
| Streaming | Yes | Yes | Yes |
| Built-in tracing | Yes | No | Yes |

## Resources

- [Documentation](https://docs.rs/tarpc)
- [GitHub Repository](https://github.com/google/tarpc)
- [Examples](https://github.com/google/tarpc/tree/master/example-service)
