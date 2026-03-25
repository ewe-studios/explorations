# WebSocket Comprehensive Exploration

## Executive Summary

This document provides a comprehensive exploration of WebSocket protocol implementations in Rust and related technologies, based on analysis of the `tungstenite-rs`, `tokio-tungstenite`, `websocat`, `rust-websocket`, and `sunrise` projects.

**Key Findings:**
- `tungstenite-rs` is the de facto standard Rust WebSocket library
- `tokio-tungstenite` provides async/await support for production use
- `websocat` demonstrates extensive WebSocket CLI capabilities
- `rust-websocket` is legacy (deprecated dependencies)
- WebSocket protocol (RFC 6455) is well-implemented with full feature support

---

## Directory Structure

```
/home/darkvoid/Boxxed/@dev/repo-expolorations/src.websocket/
├── exploration.md                    # This document (main overview)
├── websocket-protocol.md             # RFC 6455 deep dive
├── tungstenite-implementation.md     # tungstenite-rs analysis
├── tokio-tungstenite.md              # Async implementation
├── websocat.md                       # CLI tool analysis
├── alternative-implementations.md    # rust-websocket, sunrise
├── production-patterns.md            # Scaling, reliability patterns
└── rust-revision.md                  # Rust replication plan
```

---

## Source Projects Analyzed

### 1. tungstenite-rs (Core Library)

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.websocket/tungstenite-rs/`

**Purpose:** Lightweight, stream-based WebSocket implementation

**Key Features:**
- Complete RFC 6455 implementation
- Synchronous API (works with any `Read + Write`)
- TLS support (native-tls, rustls)
- Minimal dependencies

**Architecture:**
```
tungstenite-rs/
├── protocol/          # WebSocket state machine, messages, frames
├── handshake/         # Client/server handshake
├── client.rs          # Client API
├── server.rs          # Server API
├── error.rs           # Error types
└── frame/             # Frame parsing/formatting
```

**Core API:**
```rust
use tungstenite::{connect, accept, Message};

// Client
let (mut socket, response) = connect("ws://localhost:9001/")?;
socket.send(Message::Text("Hello".into()))?;
let msg = socket.read()?;

// Server
let mut socket = accept(stream)?;
```

---

### 2. tokio-tungstenite (Async Wrapper)

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.websocket/tokio-tungstenite/`

**Purpose:** Async/await WebSocket with Tokio integration

**Key Features:**
- Stream/Sink traits for composability
- Tokio runtime integration
- TLS via native-tls or rustls
- Connection utilities

**Architecture:**
```
tokio-tungstenite/
├── lib.rs             # WebSocketStream, main exports
├── compat.rs          # Sync/async bridge
├── connect.rs         # Async connection helpers
├── handshake.rs       # Async handshake
├── stream.rs          # Stream types
└── tls.rs             # TLS connectors
```

**Core API:**
```rust
use tokio_tungstenite::{connect_async, accept_async};
use futures_util::{SinkExt, StreamExt};

// Client
let (mut ws, _) = connect_async("ws://localhost/").await?;
ws.send(Message::Text("Hello".into())).await?;
let msg = ws.next().await;

// Server
let ws = accept_async(tcp_stream).await?;
```

---

### 3. websocat (CLI Tool)

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.websocket/websocat/`

**Purpose:** netcat/curl/socat for WebSockets

**Key Features:**
- 40+ address types (ws, tcp, udp, unix, exec, etc.)
- Overlay system for transformations
- Auto-reconnect, broadcast, multiplexing
- SOCKS5, TLS, encryption support

**Architecture:**
```
websocat/
├── main.rs            # CLI parsing
├── specparse.rs       # Specifier parser
├── specifier.rs       # Address types
├── sessionserve.rs    # Session handling
├── net_peer.rs        # TCP networking
├── broadcast_reuse_peer.rs
├── reconnect_peer.rs
└── crypto_peer.rs
```

**Usage Examples:**
```bash
# Simple client
websocat ws://echo.server/

# WebSocket to TCP proxy
websocat ws-l:8080 tcp:backend:9000

# Broadcast server
websocat -t ws-l:1234 broadcast:mirror:

# Auto-reconnect
websocat autoreconnect:ws://unreliable-server/
```

---

### 4. rust-websocket (Legacy)

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.websocket/rust-websocket/`

**Status:** DEPRECATED - Uses Hyper 0.10, Tokio 0.1

**Not Recommended For:** New projects

**Key Issues:**
- Outdated dependencies (pre-async/await)
- Futures 0.1 (incompatible with modern code)
- No active maintenance

**Migration Path:** Switch to `tungstenite-rs` / `tokio-tungstenite`

---

### 5. sunrise / sunrise-dom (TypeScript)

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.websocket/sunrise/`

**Purpose:** Spreadsheet-like dataflow programming (not WebSocket)

**Relevance:** Appears to be same author (Snapview), potential DOM binding layer for WebSocket data

---

## WebSocket Protocol Deep Dive

### Protocol Fundamentals

WebSocket (RFC 6455) provides:
- **Full-duplex** communication over single TCP connection
- **Low overhead** (2-14 bytes per message)
- **Message-oriented** (preserves message boundaries)
- **Origin-based security**

### Handshake Process

```
Client Request:                           Server Response:
GET /chat HTTP/1.1                        HTTP/1.1 101 Switching Protocols
Host: example.com                         Upgrade: websocket
Upgrade: websocket                        Connection: Upgrade
Connection: Upgrade                       Sec-WebSocket-Accept: <computed>
Sec-WebSocket-Key: <random base64>
Sec-WebSocket-Version: 13
```

**Accept Key Computation:**
```rust
// SHA1(key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11") -> base64
pub fn derive_accept_key(request_key: &[u8]) -> String {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::default();
    sha1.update(request_key);
    sha1.update(WS_GUID);
    data_encoding::BASE64.encode(&sha1.finalize())
}
```

### Frame Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-------+-+-------------+-------------------------------+
|F|R|R|R| opcode|M| Payload len |    Extended payload length    |
|I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
|N|V|V|V|       |S|             |   (if payload len==126/127)   |
| |1|2|3|       |K|             |                               |
+-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
|     Extended payload length continued, if payload len == 127  |
+ - - - - - - - - - - - - - - - +-------------------------------+
|                               |Masking-key, if MASK set to 1  |
+-------------------------------+-------------------------------+
| Masking-key (continued)       |          Payload Data         |
+---------------------------------------------------------------+
```

**Opcodes:**
| Code | Type | Description |
|------|------|-------------|
| 0x0 | Continuation | Fragment continuation |
| 0x1 | Text | UTF-8 text message |
| 0x2 | Binary | Binary message |
| 0x8 | Close | Connection close |
| 0x9 | Ping | Keepalive ping |
| 0xA | Pong | Keepalive response |

### Masking

**Client-to-server frames MUST be masked** (security requirement):

```rust
// Fast 32-bit masking
pub fn apply_mask_fast32(buf: &mut [u8], mask: [u8; 4]) {
    let mask_u32 = u32::from_ne_bytes(mask);
    let (prefix, words, suffix) = unsafe { buf.align_to_mut::<u32>() };

    for word in words.iter_mut() {
        *word ^= mask_u32;  // XOR 4 bytes at once
    }
}
```

---

## Implementation Comparison

| Feature | tungstenite-rs | tokio-tungstenite | rust-websocket |
|---------|---------------|-------------------|----------------|
| Async | No | Yes (async/await) | Yes (legacy futures) |
| TLS | native-tls, rustls | native-tls, rustls | native-tls only |
| API Style | Direct functions | Stream/Sink | Builder pattern |
| Dependencies | Minimal | Tokio + tungstenite | Hyper 0.10, Tokio 0.1 |
| Status | Active | Active | Deprecated |

---

## Production Patterns

### 1. Connection Pooling

```rust
use std::collections::HashMap;
use tokio::sync::mpsc;

struct ConnectionPool {
    connections: HashMap<u64, mpsc::Sender<Message>>,
}

impl ConnectionPool {
    fn broadcast(&self, msg: Message) {
        for tx in self.connections.values() {
            let _ = tx.try_send(msg.clone());
        }
    }
}
```

### 2. Heartbeat/Keepalive

```rust
use tokio::time::{interval, Duration};

async fn with_heartbeat(mut ws: WebSocketStream<TcpStream>) {
    let mut ping_interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                ws.send(Message::Ping(Vec::new())).await?;
            }
            msg = ws.next() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => { /* OK */ }
                    Some(Ok(Message::Ping(data))) => {
                        ws.send(Message::Pong(data)).await?;
                    }
                    _ => break,
                }
            }
        }
    }
}
```

### 3. Exponential Backoff Reconnection

```rust
async fn connect_with_backoff(url: &str) -> Result<WSStream, Error> {
    let mut delay = Duration::from_secs(1);

    loop {
        match connect_async(url).await {
            Ok(ws) => return Ok(ws),
            Err(_) => {
                sleep(delay).await;
                delay = min(delay * 2, Duration::from_secs(60));
            }
        }
    }
}
```

### 4. Backpressure Handling

```rust
// Bounded channel for backpressure
let (tx, rx) = mpsc::channel(100);  // Blocks at 100 messages

// Handle WriteBufferFull
match ws.send(msg).await {
    Err(WsError::WriteBufferFull(m)) => {
        ws.flush().await?;
        ws.send(m).await?;
    }
    _ => {}
}
```

---

## Crate Recommendations

### For New Projects

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.21", features = ["rustls-tls-native-roots"] }
futures-util = "0.3"
```

### For CLI Tools

```toml
[dependencies]
tokio-tungstenite = "0.21"
# Or use websocat as a library
```

### For Libraries (Sync API)

```toml
[dependencies]
tungstenite = "0.21"
```

---

## Security Considerations

### Authentication

```rust
async fn authenticate(request: &Request) -> Result<Claims, Error> {
    let auth = request.headers().get("Authorization")?;
    let token = auth.to_str()?.strip_prefix("Bearer ")?;
    Ok(validate_jwt(token)?)
}
```

### Rate Limiting

```rust
use governor::{Quota, RateLimiter};

let limiter = RateLimiter::direct(Quota::per_second(nonzero!(100u32)));

limiter.until_ready().await;
ws.send(msg).await?;
```

### Input Validation

```rust
// Always validate message sizes
let config = WebSocketConfig {
    max_message_size: Some(64 << 20),  // 64MB limit
    max_frame_size: Some(16 << 20),    // 16MB limit
    ..Default::default()
};
```

---

## Testing Strategies

### Unit Tests

```rust
#[tokio::test]
async fn test_echo() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Server task, client task, assertions...
}
```

### Autobahn Test Suite

Both `tungstenite-rs` and `tokio-tungstenite` pass the [Autobahn Test Suite](https://github.com/crossbario/autobahn-testsuite) for WebSocket compliance.

---

## Performance Characteristics

### Throughput

- **tungstenite-rs:** Good for moderate workloads
- **tokio-tungstenite:** Same performance + async overhead
- **Limitation:** Not fastest (30% slower than `fastwebsockets`)

### Memory

- Default message limit: 64MB
- Default frame limit: 16MB
- Write buffer: 128KB default

### Scaling

- Single connection per WebSocket struct
- Use Tokio tasks for concurrent connections
- External pub/sub for multi-node (Redis, etc.)

---

## API Reference Summary

### Core Types

| Type | Purpose |
|------|---------|
| `Message` | Text, Binary, Ping, Pong, Close |
| `WebSocket<Stream>` | Protocol state machine |
| `WebSocketStream<S>` | Async wrapper (tokio-tungstenite) |
| `WebSocketConfig` | Configuration |
| `CloseFrame` | Close reason |

### Key Functions

| Function | Module | Purpose |
|----------|--------|---------|
| `connect()` | tungstenite | Sync client connection |
| `accept()` | tungstenite | Sync server acceptance |
| `connect_async()` | tokio-tungstenite | Async client |
| `accept_async()` | tokio-tungstenite | Async server |
| `read()` / `send()` | tungstenite | Sync I/O |
| `next()` / `send()` | futures | Async I/O |

---

## Files Created

| File | Content |
|------|---------|
| `websocket-protocol.md` | RFC 6455 deep dive, frame format, handshake |
| `tungstenite-implementation.md` | tungstenite-rs architecture and API |
| `tokio-tungstenite.md` | Async patterns, examples |
| `websocat.md` | CLI tool usage, address types |
| `alternative-implementations.md` | rust-websocket, sunrise analysis |
| `production-patterns.md` | Connection pooling, heartbeat, scaling |
| `rust-revision.md` | Implementation guide, best practices |

---

## References

- [RFC 6455 - The WebSocket Protocol](https://tools.ietf.org/html/rfc6455)
- [tungstenite-rs GitHub](https://github.com/snapview/tungstenite-rs)
- [tokio-tungstenite GitHub](https://github.com/snapview/tokio-tungstenite)
- [websocat GitHub](https://github.com/vi/websocat)
- [MDN WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
- [Autobahn Test Suite](https://github.com/crossbario/autobahn-testsuite)

---

**Document Generated:** 2026-03-26
**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.websocket/`
**Output Directory:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/src.websocket/`
