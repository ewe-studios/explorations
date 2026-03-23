# RPC Rust Revision Guide

source: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC
revised_at: 2026-03-23

## Overview

This guide provides production-level Rust implementations for RPC functionality covered in the exploration documents. It covers Cap'n Proto serialization, RPC protocol mechanics, schema compilation, and performance optimization.

## Table of Contents

1. [Project Setup](#project-setup)
2. [Cap'n Proto Schema Design](#capn-proto-schema-design)
3. [Code Generation](#code-generation)
4. [Serialization Patterns](#serialization-patterns)
5. [RPC Implementation](#rpc-implementation)
6. [Transport Layers](#transport-layers)
7. [Error Handling](#error-handling)
8. [Performance Optimization](#performance-optimization)
9. [Testing Strategies](#testing-strategies)
10. [Production Checklist](#production-checklist)

## Project Setup

### Cargo.toml

```toml
[package]
name = "my-rpc-service"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"

[dependencies]
# Cap'n Proto runtime
capnp = "0.24"
capnp-rpc = "0.24"
capnp-futures = "0.24"

# Async runtime
tokio = { version = "1", features = ["rt-multi-thread", "net", "io-util", "macros"] }
futures = "0.3"

# Serialization (for non-capnp data)
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "2"
anyhow = "1"

# Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# TLS (optional)
tokio-rustls = "0.26"
rustls = "0.23"

[build-dependencies]
capnpc = "0.24"
```

### Build Script (build.rs)

```rust
fn main() {
    // Compile Cap'n Proto schemas
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/rpc.capnp")
        .file("schema/types.capnp")
        .output_path("src/schema")
        .run()
        .expect("schema compiler failed");

    // Re-run if schemas change
    println!("cargo:rerun-if-changed=schema/");
}
```

### Directory Structure

```
my-rpc-service/
├── Cargo.toml
├── build.rs
├── src/
│   ├── lib.rs
│   ├── main.rs
│   ├── schema/           # Generated code
│   ├── rpc/
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   ├── server.rs
│   │   └── handlers.rs
│   ├── transport/
│   │   ├── mod.rs
│   │   ├── tcp.rs
│   │   └── tls.rs
│   └── error.rs
└── schema/
    ├── rpc.capnp
    └── types.capnp
```

## Cap'n Proto Schema Design

### Basic Service Schema

```capnp
# schema/rpc.capnp
@0xabcdef1234567890;

using Types = import "types.capnp";

# Service interface
interface MyService {
    # Simple RPC call
    hello @0 (name :Text) -> (greeting :Text);

    # Call with complex types
    process @1 (request :MyRequest) -> (response :MyResponse);

    # Streaming call (Level 1+)
    stream @2 (items :List(Types.Item)) -> (count :UInt32);
}

# Request/Response structures
struct MyRequest {
    id @0 :UInt64;
    data @1 :Data;
    metadata @2 :Metadata;
}

struct MyResponse {
    success @0 :Bool;
    result @1 :Text;
    errorCode @2 :UInt32;
}

struct Metadata {
    timestamp @0 :UInt64;
    clientId @1 :Text;
    flags @2 :UInt8;
}
```

### Type Definitions

```capnp
# schema/types.capnp
@0x1234567890abcdef;

# Enumerations
enum Status {
    ok @0;
    error @1;
    pending @2;
}

# Struct with union
struct Value {
    union {
        text @0 :Text;
        number @1 :Float64;
        binary @2 :Data;
        object @3 :Object;
    }
}

struct Object {
    fields @0 :List(Field);
}

struct Field {
    name @0 :Text;
    value @1 :Value;
}

# Lists
struct Item {
    id @0 :UInt32;
    name @1 :Text;
    tags @2 :List(Text);
}
```

### Capability-Based Design

```capnp
# Factory pattern
interface ItemFactory {
    create @0 (name :Text) -> (item :ItemRef);
}

# Reference to remote object
interface ItemRef {
    getName @0 () -> (name :Text);
    update @1 (data :Data) -> (success :Bool);
    delete @2 ();
}

# Observer pattern
interface Observable {
    subscribe @0 (callback :Observer) -> (subscription :Subscription);
}

interface Observer {
    notify @0 (event :Event);
}

interface Subscription {
    unsubscribe @0 ();
}
```

## Code Generation

### Generated Module Structure

```rust
// src/schema/mod.rs
pub mod rpc_capnp {
    include!(concat!(env!("OUT_DIR"), "/rpc_capnp.rs"));
}

pub mod types_capnp {
    include!(concat!(env!("OUT_DIR"), "/types_capnp.rs"));
}
```

### Using Generated Types

```rust
use crate::schema::{rpc_capnp, types_capnp};
use capnp::message::{self, HeapAllocator};

// Creating a message
fn create_request(id: u64, data: &[u8]) -> capnp::Result<()> {
    let mut message = message::HeapMessage::new();

    {
        let mut request = message.init_root::<rpc_capnp::my_request::Builder>()?;
        request.set_id(id);
        request.set_data(data);

        {
            let mut metadata = request.reborrow().init_metadata();
            metadata.set_timestamp(0);
            metadata.set_client_id("client-1");
        }
    }

    // Serialize
    let mut buf = Vec::new();
    capnp::serialize::write_message(&mut buf, &message)?;

    Ok(())
}

// Reading a message
fn read_response(buf: &[u8]) -> capnp::Result<String> {
    let message = capnp::serialize::read_message(
        &mut &buf[..],
        message::ReaderOptions::new(),
    )?;

    let response = message.get_root::<rpc_capnp::my_response::Reader>()?;
    Ok(response.get_result()?.to_string())
}
```

## Serialization Patterns

### Zero-Copy Reading

```rust
use capnp::message::{Message, ReaderOptions};

/// Process message without copying
fn process_message(buffer: &[u8]) -> capnp::Result<()> {
    // Borrow the buffer - no allocation
    let message = capnp::serialize::read_message(
        &mut &buffer[..],
        ReaderOptions::new(),
    )?;

    // Access fields directly from buffer
    let request = message.get_root::<rpc_capnp::my_request::Reader>()?;
    let id = request.get_id();
    let data = request.get_data()?;  // Borrowed slice

    println!("Request {} with {} bytes", id, data.len());

    Ok(())
}
```

### Segmented Messages

```rust
use capnp::message::{HeapAllocator, MultipartSegmentAllocator};

/// For large messages, use multiple segments
fn create_large_message() -> capnp::Result<()> {
    // Start with 8 segments, each 4KB
    let mut allocator = MultipartSegmentAllocator::new(8, 4096);
    let mut message = message::HeapMessage::new_with_allocator(allocator);

    let mut list = message.init_root::<types_capnp::item::Builder>()?
        .init_tags(1000);  // Large list

    for i in 0..1000 {
        list.set(i as usize, &format!("tag-{}", i));
    }

    Ok(())
}
```

### Memory Mapping

```rust
use std::fs::File;
use std::io::Read;
use memmap2::Mmap;

/// Read from memory-mapped file
fn process_mmap_file(path: &str) -> capnp::Result<()> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };

    let message = capnp::serialize::read_message(
        &mut &mmap[..],
        ReaderOptions::new(),
    )?;

    // Zero-copy access to file contents
    let request = message.get_root::<rpc_capnp::my_request::Reader>()?;

    Ok(())
}
```

## RPC Implementation

### Server Implementation

```rust
use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp};
use futures::FutureExt;
use tokio::net::TcpListener;

use crate::schema::rpc_capnp::{my_service, MyService};

/// Server implementation
#[derive(Clone)]
pub struct MyServiceImpl;

impl my_service::Server for MyServiceImpl {
    fn hello<'a>(
        &'a mut self,
        params: my_service::HelloParams,
        mut results: my_service::HelloResults,
    ) -> capnp::Promise<(), capnp::Error> {
        let name = params.get().get_name().unwrap().to_string();
        let greeting = format!("Hello, {}!", name);

        results.get().set_greeting(&greeting);

        capnp::Promise::Ok(())
    }

    fn process<'a>(
        &'a mut self,
        params: my_service::ProcessParams,
        mut results: my_service::ProcessResults,
    ) -> capnp::Promise<(), capnp::Error> {
        let request = params.get().get_request()?;
        let id = request.get_id();

        // Process request
        let mut response = results.get().init_response();
        response.set_success(true);
        response.set_result(&format!("Processed {}", id));

        capnp::Promise::Ok(())
    }
}

/// Start RPC server
pub async fn start_server(addr: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("Listening on {}", addr);

    loop {
        let (stream, peer) = listener.accept().await?;
        tracing::info!("Connection from {}", peer);

        // Setup RPC system
        let (reader, writer) = tokio::io::split(stream);
        let stream = capnp_futures::StreamWrapper::new(reader, writer);

        let network = twoparty::VatNetwork::new(
            stream,
            rpc_twoparty_capnp::Side::Server,
            None,
        );

        let server = capnp_rpc::new_server(MyServiceImpl);
        let rpc_system = RpcSystem::new(network, Some(server));

        tokio::spawn(rpc_system.map(|_| ()));
    }
}
```

### Client Implementation

```rust
use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp, pry};
use futures::FutureExt;
use tokio::net::TcpStream;

use crate::schema::rpc_capnp::{my_service, MyService};

/// RPC client wrapper
pub struct MyServiceClient {
    client: my_service::Client,
}

impl MyServiceClient {
    /// Connect to server
    pub async fn connect(addr: &str) -> anyhow::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (reader, writer) = tokio::io::split(stream);
        let stream = capnp_futures::StreamWrapper::new(reader, writer);

        let network = twoparty::VatNetwork::new(
            stream,
            rpc_twoparty_capnp::Side::Client,
            None,
        );

        let rpc_system = RpcSystem::new(network, None);
        tokio::spawn(rpc_system.map(|_| ()));

        // Get bootstrap capability
        let client = my_service::Client::new(
            network.bootstrap::<dyn MyService>()
        );

        Ok(Self { client })
    }

    /// Call hello method
    pub async fn hello(&mut self, name: &str) -> anyhow::Result<String> {
        let mut request = self.client.hello_request();
        request.get().set_name(name);

        let response = pry!(request.send().promise.await);
        let greeting = response.get()?.get_greeting()?.to_string();

        Ok(greeting)
    }

    /// Call process method
    pub async fn process(
        &mut self,
        id: u64,
        data: &[u8],
    ) -> anyhow::Result<String> {
        let mut request = self.client.process_request();

        {
            let mut req = request.get().init_request();
            req.set_id(id);
            req.set_data(data);
            req.init_metadata().set_client_id("client-1");
        }

        let response = pry!(request.send().promise.await);
        let result = response.get()?;

        if result.get_success() {
            Ok(result.get_result()?.to_string())
        } else {
            Err(anyhow::anyhow!("Error code: {}", result.get_error_code()))
        }
    }
}
```

### Promise Pipelining

```rust
/// Demonstrate promise pipelining
pub async fn pipelined_call(
    factory: &ItemFactoryClient,
    name: &str,
) -> capnp::Result<String> {
    // Start create call but don't await
    let mut create_req = factory.create_request();
    create_req.get().set_name(name);
    let create_future = create_req.send();

    // Pipeline: use result before it arrives
    let item_ref = create_future.get_item()?;

    // This call is pipelined - sent with create
    let mut name_req = item_ref.get_name_request();
    let name_future = name_req.send();

    // Now await both
    let _create_response = create_future.await?;
    let name_response = name_future.await?;

    Ok(name_response.get()?.get_name()?.to_string())
}
```

## Transport Layers

### TCP Transport

```rust
use tokio::net::{TcpListener, TcpStream};
use capnp_futures::{serialize, message};

/// TCP server with message framing
pub async fn tcp_server(
    addr: &str,
    handler: impl Fn(Message) -> capnp::Result<Message> + Send + Sync + 'static,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, peer) = listener.accept().await?;
        let handler = handler.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, handler).await {
                tracing::error!("Connection error from {}: {}", peer, e);
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    handler: impl Fn(Message) -> capnp::Result<Message>,
) -> capnp::Result<()> {
    let (mut reader, mut writer) = tokio::io::split(stream);

    loop {
        // Read message
        let message = serialize::read_message(&mut reader, Default::default()).await?;

        // Process
        let response = handler(message)?;

        // Write response
        serialize::write_message(&mut writer, &response).await?;
    }
}
```

### TLS Transport

```rust
use tokio_rustls::{TlsAcceptor, TlsConnector};
use rustls::{ServerConfig, ClientConfig, Certificate, PrivateKey};
use std::sync::Arc;

/// Create TLS server config
fn server_config(cert: &[u8], key: &[u8]) -> anyhow::Result<ServerConfig> {
    let certs = rustls_pemfile::certs(&mut &cert[..])?
        .into_iter()
        .map(Certificate)
        .collect();

    let key = PrivateKey(rustls_pemfile::pkcs8_private_keys(&mut &key[..])?.remove(0));

    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(config)
}

/// TLS wrapper for RPC
pub async fn tls_server(
    addr: &str,
    cert: &[u8],
    key: &[u8],
) -> anyhow::Result<()> {
    let config = server_config(cert, key)?;
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, peer) = listener.accept().await?;
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            let stream = match acceptor.accept(stream).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("TLS handshake failed: {}", e);
                    return;
                }
            };

            // Use stream for RPC
            handle_rpc_connection(stream).await;
        });
    }
}

/// TLS client
pub async fn tls_client(
    addr: &str,
    server_name: &str,
) -> anyhow::Result<TcpStream> {
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();

    let connector = TlsConnector::from(Arc::new(config));

    let stream = TcpStream::connect(addr).await?;
    let stream = connector.connect(server_name.try_into()?, stream).await?;

    Ok(stream)
}
```

### Unix Domain Sockets

```rust
#[cfg(unix)]
use tokio::net::UnixStream;

/// Unix socket transport
#[cfg(unix)]
pub async fn unix_server(
    path: &str,
) -> anyhow::Result<()> {
    use tokio::net::UnixListener;
    use std::path::Path;

    // Remove existing socket
    let _ = std::fs::remove_file(path);

    let listener = UnixListener::bind(Path::new(path))?;

    loop {
        let (stream, _addr) = listener.accept().await?;
        tokio::spawn(handle_rpc_connection(stream));
    }
}
```

## Error Handling

### Custom Error Types

```rust
use thiserror::Error;
use capnp::{Error as CapnpError, NotInCapTable};

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("Cap'n Proto error: {0}")]
    Capnp(#[from] CapnpError),

    #[error("Not in capability table: {0}")]
    NotInCapTable(#[from] NotInCapTable),

    #[error("Connection lost: {0}")]
    ConnectionLost(String),

    #[error("Request timeout")]
    Timeout,

    #[error("Server error: {code}: {message}")]
    Server { code: u32, message: String },

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<std::io::Error> for RpcError {
    fn from(e: std::io::Error) -> Self {
        RpcError::ConnectionLost(e.to_string())
    }
}
```

### Error Propagation in RPC

```rust
use capnp::Error as CapnpError;
use capnp::error::Type as ErrorType;

/// Convert application error to Cap'n Proto error
fn app_error_to_capnp(msg: &str, code: u32) -> CapnpError {
    CapnpError::failed(format!("{} (code: {})", msg, code))
}

/// Handle RPC result
fn handle_result<T>(result: capnp::Result<T>) -> Result<T, RpcError> {
    match result {
        Ok(v) => Ok(v),
        Err(CapnpError::Failed(s)) => Err(RpcError::Server {
            code: 0,
            message: s,
        }),
        Err(e) => Err(e.into()),
    }
}
```

### Retry Logic

```rust
use std::time::Duration;
use tokio::time::sleep;

/// Retry with exponential backoff
pub async fn retry_with_backoff<T, F, Fut>(
    mut f: F,
    max_retries: u32,
    base_delay: Duration,
) -> Result<T, RpcError>
where
    F: FnMut() -> Fut,
    Fut: futures::Future<Output = Result<T, RpcError>>,
{
    let mut delay = base_delay;

    for attempt in 0..max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(RpcError::ConnectionLost(_)) if attempt < max_retries - 1 => {
                tracing::warn!("Connection lost, retrying in {:?}", delay);
                sleep(delay).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }

    Err(RpcError::Timeout)
}
```

## Performance Optimization

### Connection Pooling

```rust
use tokio::sync::mpsc;
use std::collections::HashMap;

/// Connection pool for RPC clients
pub struct ConnectionPool {
    sender: mpsc::Sender<PoolMessage>,
}

enum PoolMessage {
    Get(String, mpsc::Sender<Option<MyServiceClient>>),
    Return(String, MyServiceClient),
}

impl ConnectionPool {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(100);

        tokio::spawn(async move {
            let mut connections: HashMap<String, Vec<MyServiceClient>> = HashMap::new();

            while let Some(msg) = rx.recv().await {
                match msg {
                    PoolMessage::Get(addr, reply_tx) => {
                        let client = if let Some(vec) = connections.get_mut(&addr) {
                            vec.pop()
                        } else {
                            MyServiceClient::connect(&addr).await.ok()
                        };
                        let _ = reply_tx.send(client).await;
                    }
                    PoolMessage::Return(addr, client) => {
                        connections.entry(addr).or_default().push(client);
                    }
                }
            }
        });

        Self { sender: tx }
    }

    pub async fn get(&self, addr: &str) -> Option<MyServiceClient> {
        let (tx, rx) = mpsc::channel(1);
        self.sender.send(PoolMessage::Get(addr.to_string(), tx)).await.ok()?;
        rx.recv().await?
    }
}
```

### Batching Requests

```rust
use futures::future::join_all;

/// Batch multiple requests
pub async fn batch_process(
    client: &mut MyServiceClient,
    items: Vec<(u64, Vec<u8>)>,
) -> Vec<Result<String, RpcError>> {
    let futures: Vec<_> = items
        .into_iter()
        .map(|(id, data)| client.process(id, &data))
        .collect();

    join_all(futures).await
}
```

### Read Limits

```rust
use capnp::message::ReaderOptions;

/// Configure read limits for security
fn secure_reader_options() -> ReaderOptions {
    ReaderOptions::new()
        .traversal_limit_in_words(8 * 1024 * 1024)  // 8 MB
        .nesting_limit(64)
}

/// Read with limits
pub fn read_secure_message(buf: &[u8]) -> capnp::Result<message::Reader<capnp::message::HeapAllocator>> {
    capnp::serialize::read_message(&mut &buf[..], secure_reader_options())
}
```

## Testing Strategies

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use capnp::message::HeapMessage;

    #[test]
    fn test_request_serialization() -> capnp::Result<()> {
        let mut message = HeapMessage::new();
        let mut request = message.init_root::<rpc_capnp::my_request::Builder>()?;

        request.set_id(123);
        request.set_data(&[1, 2, 3]);

        // Serialize
        let mut buf = Vec::new();
        capnp::serialize::write_message(&mut buf, &message)?;

        // Deserialize
        let message2 = capnp::serialize::read_message(&mut &buf[..], Default::default())?;
        let request2 = message2.get_root::<rpc_capnp::my_request::Reader>()?;

        assert_eq!(request2.get_id(), 123);
        assert_eq!(request2.get_data()?, &[1, 2, 3]);

        Ok(())
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_rpc_hello() -> anyhow::Result<()> {
    // Start server
    let addr = "127.0.0.1:0";  // Ephemeral port
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    let server_handle = tokio::spawn(async move {
        // Handle one connection
        let (stream, _) = listener.accept().await?;
        // ... handle connection
        Ok::<_, anyhow::Error>(())
    });

    // Connect client
    let mut client = MyServiceClient::connect(&format!("127.0.0.1:{}", local_addr.port())).await?;

    // Make call
    let greeting = client.hello("Test").await?;
    assert_eq!(greeting, "Hello, Test!");

    server_handle.await??;
    Ok(())
}
```

### Mock Testing

```rust
/// Mock server for testing
struct MockService;

impl my_service::Server for MockService {
    fn hello<'a>(
        &'a mut self,
        params: my_service::HelloParams,
        mut results: my_service::HelloResults,
    ) -> capnp::Promise<(), capnp::Error> {
        let name = params.get().get_name().unwrap();
        results.get().set_greeting(&format!("Mock: {}", name));
        capnp::Promise::Ok(())
    }
}
```

## Production Checklist

### Security

- [ ] TLS enabled for all network connections
- [ ] Read limits configured (traversal limit, nesting limit)
- [ ] Authentication implemented
- [ ] Input validation on all RPC parameters
- [ ] Rate limiting configured

### Reliability

- [ ] Connection pooling implemented
- [ ] Retry logic with backoff
- [ ] Timeout handling
- [ ] Graceful shutdown
- [ ] Health checks

### Observability

- [ ] Tracing instrumentation (OpenTelemetry)
- [ ] Metrics collection (request latency, error rates)
- [ ] Log aggregation
- [ ] Distributed tracing enabled

### Performance

- [ ] Connection reuse enabled
- [ ] Batching for high-throughput scenarios
- [ ] Zero-copy where applicable
- [ ] Memory limits configured

### Testing

- [ ] Unit tests for serialization
- [ ] Integration tests for RPC calls
- [ ] Load testing completed
- [ ] Chaos testing for failure scenarios

## Resources

- [Cap'n Proto Rust Docs](https://docs.rs/capnp/)
- [capnproto-rust Examples](https://github.com/capnproto/capnproto-rust/tree/master/example)
- [Cap'n Proto RPC Spec](https://capnproto.org/rpc.html)
- [Tokio Documentation](https://tokio.rs/)
