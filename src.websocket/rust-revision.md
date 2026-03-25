# Rust WebSocket Implementation Plan

## Overview

This document outlines how to build a production-grade WebSocket implementation in Rust, drawing from the analysis of tungstenite-rs, tokio-tungstenite, and websocat.

## Architecture Decision

### Recommended Stack

For new projects, use this layered approach:

```
┌─────────────────────────────────────┐
│     Your Application Logic          │
├─────────────────────────────────────┤
│     tokio-tungstenite               │  (Async WebSocket)
├─────────────────────────────────────┤
│     tungstenite-rs                  │  (Protocol implementation)
├─────────────────────────────────────┤
│     Tokio Runtime                   │  (Async runtime)
└─────────────────────────────────────┘
```

### Crate Selection

| Purpose | Crate | Version |
|---------|-------|---------|
| Core Protocol | `tungstenite` | 0.21+ |
| Async Support | `tokio-tungstenite` | 0.21+ |
| HTTP Types | `http` | 1.0+ |
| TLS (native) | `native-tls` | 0.2+ |
| TLS (rustls) | `rustls` + `tokio-rustls` | 0.22+ / 0.25+ |
| Futures Utils | `futures-util` | 0.3+ |

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.21", features = ["rustls-tls-native-roots"] }
tungstenite = "0.21"
futures-util = "0.3"
http = "1.0"
log = "0.4"
tracing = "0.1"
```

---

## Implementation Patterns

### Basic Echo Server

```rust
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures_util::{future, StreamExt, TryStreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("WebSocket server listening on: {}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("New connection from: {}", addr);
                tokio::spawn(handle_connection(stream));
            }
            Err(e) => eprintln!("Accept error: {}", e),
        }
    }
}

async fn handle_connection(stream: TcpStream) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("Handshake error: {}", e);
            return;
        }
    };

    let (write, read) = ws_stream.split();

    // Filter to only text/binary, echo back
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .try_forward(write)
        .await
        .unwrap();
}
```

### Multi-Client Chat Server

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use futures_util::{SinkExt, StreamExt};

type PeerMap = Arc<RwLock<HashMap<u64, mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    let peers: PeerMap = Arc::new(RwLock::new(HashMap::new()));
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let mut peer_id = 0;
    while let Ok((stream, _)) = listener.accept().await {
        let peers = peers.clone();
        let id = peer_id;
        peer_id += 1;

        tokio::spawn(async move {
            handle_chat_client(stream, peers, id).await;
        });
    }
}

async fn handle_chat_client(
    stream: TcpStream,
    peers: PeerMap,
    id: u64,
) {
    let ws_stream = accept_async(stream).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Register connection
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    peers.write().await.insert(id, tx);

    // Broadcast join message
    let join_msg = Message::Text(format!("User {} joined", id));
    for peer_tx in peers.read().await.values() {
        let _ = peer_tx.send(join_msg.clone());
    }

    // Task: Receive from channel and write to WebSocket
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Task: Read from WebSocket and broadcast
    while let Some(Ok(msg)) = read.next().await {
        match msg {
            Message::Text(text) => {
                let broadcast = Message::Text(format!("User {}: {}", id, text));
                for peer_tx in peers.read().await.values() {
                    let _ = peer_tx.send(broadcast.clone());
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup on disconnect
    peers.write().await.remove(&id);
    write_task.abort();
}
```

---

## Key Design Decisions

### Async vs Sync

**Use Async (tokio-tungstenite) when:**
- Building servers handling multiple connections
- Need to integrate with other async code
- Building real-time applications
- Performance is critical

**Use Sync (tungstenite-rs) when:**
- Simple CLI tools
- Blocking is acceptable
- Integrating with sync-only code

### TLS Selection

**native-tls:**
- Uses system TLS (OpenSSL/Schannel/SecureTransport)
- Better for corporate environments
- Smaller binary

**rustls:**
- Pure Rust implementation
- No external dependencies
- Better security auditability
- Recommended for new projects

### Message Handling

```rust
// Pattern 1: Direct handling
while let Some(Ok(msg)) = ws.next().await {
    match msg {
        Message::Text(t) => handle_text(&t),
        Message::Binary(b) => handle_binary(&b),
        Message::Ping(d) => { let _ = ws.send(Message::Pong(d)).await; }
        Message::Close(_) => break,
        _ => {}
    }
}

// Pattern 2: Channel-based (for complex apps)
let (tx, mut rx) = mpsc::channel(100);

// Reader task
let read_task = tokio::spawn(async move {
    while let Some(Ok(msg)) = ws.next().await {
        let _ = tx.send(msg);
    }
});

// Processor task
while let Some(msg) = rx.recv().await {
    process_message(msg).await;
}
```

---

## Production Checklist

### Configuration

```rust
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

fn default_config() -> WebSocketConfig {
    WebSocketConfig {
        max_message_size: Some(64 << 20),     // 64MB
        max_frame_size: Some(16 << 20),       // 16MB
        write_buffer_size: 128 << 10,         // 128KB
        max_write_buffer_size: usize::MAX,    // Unlimited
        accept_unmasked_frames: false,        // RFC compliant
    }
}
```

### Error Handling

```rust
use tokio_tungstenite::tungstenite::Error as WsError;

async fn robust_handler(mut ws: WebSocketStream<TcpStream>) {
    loop {
        match ws.next().await {
            Some(Ok(msg)) => {
                // Handle message
            }
            Some(Err(WsError::ConnectionClosed)) => {
                println!("Connection closed normally");
                break;
            }
            Some(Err(WsError::AlreadyClosed)) => {
                println!("Already closed - programmer error");
                break;
            }
            Some(Err(WsError::Io(e))) if e.kind() == ErrorKind::WouldBlock => {
                // Transient, continue
                continue;
            }
            Some(Err(e)) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            None => {
                println!("Stream ended");
                break;
            }
        }
    }
}
```

### Graceful Shutdown

```rust
use tokio::signal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

struct Shutdown {
    flag: Arc<AtomicBool>,
}

impl Shutdown {
    fn new() -> Self {
        Self { flag: Arc::new(AtomicBool::new(false)) }
    }

    fn is_shutdown(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }

    async fn wait_for_shutdown(&self) {
        while !self.is_shutdown() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[tokio::main]
async fn main() {
    let shutdown = Shutdown::new();
    let shutdown_clone = shutdown.clone();

    // Signal handler
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        println!("Shutting down...");
        shutdown_clone.flag.store(true, Ordering::Relaxed);
    });

    // Server loop with shutdown
    while !shutdown.is_shutdown() {
        // Accept connections with timeout
        tokio::select! {
            _ = shutdown.wait_for_shutdown() => break,
            // ... accept connections
        }
    }
}
```

---

## Performance Optimization

### Batch Writes

```rust
use tokio::sync::mpsc;

async fn batch_writer(
    mut ws: WebSocketStream<TcpStream>,
    mut rx: mpsc::Receiver<Message>,
) {
    let mut batch = Vec::with_capacity(16);
    let mut interval = tokio::time::interval(Duration::from_millis(10));

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some(m) => batch.push(m),
                    None => break,
                }
                if batch.len() >= batch.capacity() {
                    flush_batch(&mut ws, &mut batch).await;
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() {
                    flush_batch(&mut ws, &mut batch).await;
                }
            }
        }
    }
}

async fn flush_batch(
    ws: &mut WebSocketStream<TcpStream>,
    batch: &mut Vec<Message>,
) {
    for msg in batch.drain(..) {
        let _ = ws.send(msg).await;
    }
    let _ = ws.flush().await;
}
```

### Zero-Copy Binary Handling

```rust
use bytes::Bytes;

// For large binary messages, avoid copies
async fn handle_large_binary(
    mut ws: WebSocketStream<TcpStream>,
) {
    while let Some(Ok(msg)) = ws.next().await {
        match msg {
            Message::Binary(data) => {
                // data is already Vec<u8>, no copy needed
                process_binary(data).await;
            }
            _ => {}
        }
    }
}
```

---

## Testing

### Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_processing() {
        let msg = Message::Text("test".into());
        assert!(msg.is_text());
        assert_eq!(msg.to_text().unwrap(), "test");
    }
}
```

### Integration Test

```rust
#[tokio::test]
async fn test_full_connection() {
    // Start server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();

        while let Some(Ok(msg)) = ws.next().await {
            if msg.is_text() {
                ws.send(msg).await.unwrap();
            }
        }
    });

    // Connect client
    let (mut client, _) = connect_async(format!("ws://{}", addr))
        .await
        .unwrap();

    client.send(Message::Text("hello".into())).await.unwrap();
    let response = client.next().await.unwrap().unwrap();
    assert_eq!(response.to_text().unwrap(), "hello");

    server.abort();
}
```

---

## Deployment Considerations

### Container Resource Limits

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/myapp /usr/local/bin/

# Set resource limits
ENV RUST_MAX_THREADS=100
ENV WEBSOCKET_MAX_CONNECTIONS=1000

EXPOSE 8080
CMD ["myapp"]
```

### Kubernetes Health Checks

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: websocket-server
spec:
  containers:
  - name: app
    image: myapp:latest
    ports:
    - containerPort: 8080
    readinessProbe:
      httpGet:
        path: /health
        port: 8080
      initialDelaySeconds: 5
      periodSeconds: 10
    livenessProbe:
      httpGet:
        path: /health
        port: 8080
      initialDelaySeconds: 15
      periodSeconds: 20
    resources:
      limits:
        memory: "512Mi"
        cpu: "500m"
      requests:
        memory: "256Mi"
        cpu: "250m"
```

### Monitoring Setup

```rust
// metrics.rs
use prometheus::{Encoder, TextEncoder, Registry, IntGauge, IntCounter};

pub struct Metrics {
    registry: Registry,
    connections: IntGauge,
    messages: IntCounter,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        let connections = IntGauge::new("ws_connections", "Active connections").unwrap();
        let messages = IntCounter::new("ws_messages_total", "Total messages").unwrap();

        registry.register(Box::new(connections.clone())).unwrap();
        registry.register(Box::new(messages.clone())).unwrap();

        Self { registry, connections, messages }
    }

    pub fn on_connect(&self) { self.connections.inc(); }
    pub fn on_disconnect(&self) { self.connections.dec(); }
    pub fn on_message(&self) { self.messages.inc(); }

    pub fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder.encode(&self.registry.gather(), &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}
```

---

## Common Pitfalls

### 1. Not Flushing Writes

```rust
// WRONG: Message may not be sent immediately
ws.send(msg).await?;
// Need to flush if not using send()

// RIGHT: Use send() which flushes, or call flush explicitly
ws.send(msg).await?;
ws.flush().await?;
```

### 2. Ignoring Backpressure

```rust
// WRONG: Unbounded channel can cause memory issues
let (tx, rx) = mpsc::unbounded_channel();

// RIGHT: Use bounded channel
let (tx, rx) = mpsc::channel(100);  // Backpressure at 100 messages
```

### 3. Not Handling Ping/Pong

```rust
// WRONG: Connection may timeout
while let Some(Ok(msg)) = ws.next().await {
    // Only handling data messages
}

// RIGHT: Handle control messages
while let Some(Ok(msg)) = ws.next().await {
    match msg {
        Message::Ping(data) => {
            ws.send(Message::Pong(data)).await?;
        }
        // ... handle other messages
    }
}
```

### 4. Missing Close Handshake

```rust
// WRONG: Abrupt close
drop(ws);

// RIGHT: Graceful close
ws.close(Some(CloseFrame {
    code: CloseCode::Normal,
    reason: "Goodbye".into(),
})).await?;
ws.flush().await?;
```

---

## Resources

- [tungstenite-rs GitHub](https://github.com/snapview/tungstenite-rs)
- [tokio-tungstenite GitHub](https://github.com/snapview/tokio-tungstenite)
- [RFC 6455](https://tools.ietf.org/html/rfc6455)
- [Tokio Documentation](https://tokio.rs/)
- [WebSocket MDN Guide](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
