# Tokio-Tungstenite: Async WebSocket Implementation

## Overview

`tokio-tungstenite` provides asynchronous WebSocket support by wrapping `tungstenite-rs` with Tokio's async runtime. It implements `Stream` and `Sink` traits for seamless integration with the Tokio ecosystem.

**Key Features:**
- Async/await API
- Tokio runtime integration
- Stream/Sink traits for composability
- TLS support (native-tls and rustls)
- Connection utilities

## Architecture

```
tokio-tungstenite/
├── src/
│   ├── lib.rs              # Main exports, WebSocketStream
│   ├── compat.rs           # Sync/async compatibility layer
│   ├── connect.rs          # Async connection helpers
│   ├── handshake.rs        # Async handshake
│   ├── stream.rs           # Stream types (MaybeTlsStream)
│   └── tls.rs              # TLS connectors
└── examples/
    ├── echo-server.rs      # Echo server example
    ├── client.rs           # Client example
    ├── autobahn-client.rs  # Test suite client
    └── server.rs           # Server example
```

## WebSocketStream

The core type wrapping synchronous WebSocket with async I/O:

```rust
pub struct WebSocketStream<S> {
    inner: WebSocket<AllowStd<S>>,  // Wrapped tungstenite WebSocket
    closing: bool,                   // Close handshake state
    ended: bool,                     // Terminal state
    ready: bool,                     // Ready to send
}
```

### AllowStd Wrapper

Bridges async I/O with synchronous tungstenite API:

```rust
pub struct AllowStd<S> {
    inner: S,
    // Waker management for async notification
}

impl<S> Read for AllowStd<S>
where
    S: AsyncRead + Unpin,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Convert WouldBlock to proper async polling
        cvt(Pin::new(&mut self.inner).poll_read(cx, buf))
    }
}
```

## Stream Implementation

```rust
impl<T> Stream for WebSocketStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<Message, WsError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.ended {
            return Poll::Ready(None);  // Fused stream
        }

        match futures_util::ready!(self.with_context(Some((ContextWaker::Read, cx)), |s| {
            cvt(s.read())
        })) {
            Ok(v) => Poll::Ready(Some(Ok(v))),
            Err(e) => {
                self.ended = true;
                if matches!(e, WsError::AlreadyClosed | WsError::ConnectionClosed) {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Err(e)))
                }
            }
        }
    }
}
```

## Sink Implementation

```rust
impl<T> Sink<Message> for WebSocketStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Error = WsError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.ready {
            Poll::Ready(Ok(()))
        } else {
            // Try to flush blocked writes
            (*self).with_context(Some((ContextWaker::Write, cx)), |s| cvt(s.flush())).map(|r| {
                self.ready = true;
                r
            })
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        match (*self).with_context(None, |s| s.write(item)) {
            Ok(()) => {
                self.ready = true;
                Ok(())
            }
            Err(WsError::Io(err)) if err.kind() == WouldBlock => {
                // Message queued, will flush on poll_ready
                self.ready = false;
                Ok(())
            }
            Err(e) => {
                self.ready = true;
                Err(e)
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        (*self).with_context(Some((ContextWaker::Write, cx)), |s| cvt(s.flush())).map(|r| {
            self.ready = true;
            match r {
                Err(WsError::ConnectionClosed) => Ok(()),
                other => other,
            }
        })
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.ready = true;
        let res = if self.closing {
            (*self).with_context(Some((ContextWaker::Write, cx)), |s| s.flush())
        } else {
            (*self).with_context(Some((ContextWaker::Write, cx)), |s| s.close(None))
        };

        match res {
            Ok(()) => Poll::Ready(Ok(())),
            Err(WsError::ConnectionClosed) => Poll::Ready(Ok(())),
            Err(WsError::Io(err)) if err.kind() == WouldBlock => {
                self.closing = true;
                Poll::Pending
            }
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}
```

## Client API

### Basic Connection

```rust
use tokio_tungstenite::connect_async;
use futures_util::{StreamExt, TryStreamExt};

#[tokio::main]
async fn main() {
    let (ws_stream, response) = connect_async("ws://localhost:9001/socket")
        .await
        .expect("Connection failed");

    let (mut write, mut read) = ws_stream.split();

    // Send message
    write.send(Message::Text("Hello!".into())).await?;

    // Receive messages
    while let Some(Ok(msg)) = read.next().await {
        println!("Received: {}", msg);
    }
}
```

### With Configuration

```rust
use tokio_tungstenite::{connect_async_with_config, tungstenite::protocol::WebSocketConfig};

let config = Some(WebSocketConfig {
    max_message_size: Some(1024 * 1024),  // 1MB limit
    max_frame_size: Some(256 * 1024),     // 256KB limit
    write_buffer_size: 64 * 1024,         // 64KB buffer
    ..Default::default()
});

let (ws_stream, response) = connect_async_with_config(
    "ws://localhost:9001/socket",
    config,
).await?;
```

### TLS Connection

```rust
// Using rustls
use tokio_tungstenite::{connect_async_tls_with_config, Connector};
use tokio_rustls::rustls::ClientConfig;

let connector = Connector::Rustls(Arc::new(ClientConfig::builder().build()));

let (ws_stream, _) = connect_async_tls_with_config(
    "wss://localhost:9001/socket",
    None,  // config
    true,  // enable nagle
    Some(connector),
).await?;
```

### Custom Request

```rust
use http::{Request, HeaderMap};
use tokio_tungstenite::connect_async;

let mut headers = HeaderMap::new();
headers.insert("Authorization", "Bearer token123".parse().unwrap());
headers.insert("Sec-WebSocket-Protocol", "chat".parse().unwrap());

let request = Request::builder()
    .uri("ws://localhost:9001/socket")
    .header("Origin", "http://localhost:3000")
    .body(())
    .unwrap();

let (ws_stream, response) = connect_async(request).await?;
```

## Server API

### Basic Echo Server

```rust
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures_util::{future, StreamExt, TryStreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }
    Ok(())
}

async fn accept_connection(stream: TcpStream) {
    let ws_stream = accept_async(stream).await.expect("Handshake failed");

    let (write, read) = ws_stream.split();
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .forward(write)
        .await
        .expect("Forward failed");
}
```

### Server with Custom Headers

```rust
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

let callback = |req: &Request, mut response: Response| {
    println!("Connection request from: {}", req.headers().get("Origin").unwrap_or(&"unknown".into()));

    let headers = response.headers_mut();
    headers.append("X-Custom-Header", "value".parse().unwrap());

    Ok(response)
};

let ws_stream = accept_hdr_async(stream, callback).await?;
```

### With Configuration

```rust
use tokio_tungstenite::accept_async_with_config;

let config = Some(WebSocketConfig {
    max_message_size: Some(64 << 20),  // 64MB
    max_frame_size: Some(16 << 20),    // 16MB
    ..Default::default()
});

let ws_stream = accept_async_with_config(stream, config).await?;
```

## Concurrency Patterns

### Broadcast to Multiple Clients

```rust
use tokio::sync::broadcast;
use std::sync::Arc;

struct AppState {
    tx: broadcast::Sender<String>,
}

async fn handle_client(
    ws_stream: WebSocketStream<TcpStream>,
    state: Arc<AppState>,
) {
    let (mut write, mut read) = ws_stream.split();
    let mut rx = state.tx.subscribe();

    // Task to receive from WebSocket and broadcast
    let tx = state.tx.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            if let Message::Text(text) = msg {
                let _ = tx.send(text);
            }
        }
    });

    // Task to receive broadcasts and send to WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(text) = rx.recv().await {
            if write.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });

    let _ = tokio::join!(recv_task, send_task);
}
```

### Request-Response Pattern

```rust
use tokio::sync::mpsc;
use std::collections::HashMap;
use uuid::Uuid;

struct PendingRequest {
    id: String,
    tx: oneshot::Sender<Message>,
}

async fn request_response(
    ws_stream: WebSocketStream<TcpStream>,
) {
    let (mut write, mut read) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<Message>(100);

    // Writer task
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Reader task
    let read_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            // Handle response matching logic here
            println!("Received: {:?}", msg);
        }
    });

    let _ = tokio::join!(read_task, write_task);
}
```

### Ping/Pong Heartbeat

```rust
use tokio::time::{interval, Duration};

async fn with_heartbeat(mut ws_stream: WebSocketStream<TcpStream>) {
    let (mut write, mut read) = ws_stream.split();
    let mut ping_interval = interval(Duration::from_secs(30));

    let (tx, mut rx) = mpsc::channel::<Message>(100);

    // Heartbeat sender
    let heartbeat_task = tokio::spawn(async move {
        loop {
            ping_interval.tick().await;
            if tx.send(Message::Ping(Vec::new())).await.is_err() {
                break;
            }
        }
    });

    // Message writer
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Message reader
    let read_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            match msg {
                Message::Pong(_) => println!("Heartbeat OK"),
                Message::Ping(data) => {
                    let _ = write.send(Message::Pong(data)).await;
                }
                Message::Close(_) => break,
                _ => println!("Message: {:?}", msg),
            }
        }
    });

    let _ = tokio::join!(heartbeat_task, write_task, read_task);
}
```

## TLS Support

### Native TLS

```toml
[dependencies]
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
```

```rust
use tokio_tungstenite::{connect_async_tls_with_config, Connector};
use tokio_native_tls::native_tls::TlsConnector;

let tls_connector = TlsConnector::builder()
    .danger_accept_invalid_certs(true)  // Development only!
    .build()
    .unwrap();

let connector = Connector::NativeTls(tls_connector);

let (ws_stream, _) = connect_async_tls_with_config(
    "wss://example.com/socket",
    None,
    true,
    Some(connector),
).await?;
```

### Rustls

```toml
[dependencies]
tokio-tungstenite = { version = "0.21", features = ["rustls-tls-native-roots"] }
tokio-rustls = "0.25"
```

```rust
use tokio_tungstenite::{connect_async_tls_with_config, Connector};
use tokio_rustls::rustls::ClientConfig;
use std::sync::Arc;

let config = ClientConfig::builder()
    .with_native_roots()  // Uses system cert store
    .unwrap()
    .with_no_client_auth();

let connector = Connector::Rustls(Arc::new(config));

let (ws_stream, _) = connect_async_tls_with_config(
    "wss://example.com/socket",
    None,
    true,
    Some(connector),
).await?;
```

## Integration with Other Crates

### Warp Framework

```rust
use warp::Filter;

let ws_route = warp::path("ws")
    .and(warp::ws())
    .map(|ws: warp::ws::Ws| {
        ws.on_upgrade(|websocket| async move {
            // Handle WebSocket connection
        })
    });
```

### Actix-Web

```rust
use actix_web::{web, App, HttpServer, Error};
use actix::StreamExt;

async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
) -> Result<impl actix_web::Responder, Error> {
    let (resp, session, stream) = actix_ws::handle(&req, stream)?;

    actix::spawn(async move {
        // Handle session
    });

    Ok(resp)
}
```

### Axum

```rust
use axum::{
    extract::ws::{WebSocketUpgrade, WebSocket},
    response::IntoResponse,
    routing::get,
    Router,
};

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        // Handle message
    }
}

let app = Router::new().route("/ws", get(ws_handler));
```

## Error Handling

```rust
use tokio_tungstenite::tungstenite::Error as WsError;

async fn robust_connection(url: &str) -> Result<(), WsError> {
    match connect_async(url).await {
        Ok((ws_stream, _)) => {
            // Handle connection
            Ok(())
        }
        Err(WsError::ConnectionClosed) => {
            // Normal closure
            Ok(())
        }
        Err(WsError::AlreadyClosed) => {
            // Retry logic
            Err(WsError::AlreadyClosed)
        }
        Err(WsError::Io(e)) if e.kind() == ErrorKind::WouldBlock => {
            // Transient error
            Err(WsError::Io(e))
        }
        Err(e) => {
            // Fatal error
            eprintln!("WebSocket error: {}", e);
            Err(e)
        }
    }
}
```

## Performance Considerations

1. **Write Buffering**: Default 128KB buffer before flushing
2. **Message Size Limits**: Configure `max_message_size` to prevent memory exhaustion
3. **Backpressure**: Use `Sink::poll_ready` to handle slow consumers
4. **Connection Limits**: Implement connection pooling for high concurrency

## Testing

### Unit Test with Mock Streams

```rust
use tokio_tungstenite::WebSocketStream;
use tokio::io::{AsyncRead, AsyncWrite};

struct MockStream {
    // Custom mock implementation
}

impl AsyncRead for MockStream {
    // Implement read
}

impl AsyncWrite for MockStream {
    // Implement write
}

#[tokio::test]
async fn test_websocket() {
    let stream = MockStream::new();
    let ws = WebSocketStream::from_raw_socket(
        stream,
        tokio_tungstenite::tungstenite::protocol::Role::Client,
        None,
    ).await;

    // Test WebSocket operations
}
```

## Migration from Sync to Async

### Sync (tungstenite-rs)

```rust
use tungstenite::{connect, Message};

let (mut socket, _) = connect("ws://localhost/socket")?;
socket.send(Message::Text("Hello".into()))?;
let msg = socket.read()?;
```

### Async (tokio-tungstenite)

```rust
use tokio_tungstenite::connect_async;
use futures_util::{SinkExt, StreamExt};

let (mut ws, _) = connect_async("ws://localhost/socket").await?;
ws.send(Message::Text("Hello".into())).await?;
let msg = ws.next().await;
```
