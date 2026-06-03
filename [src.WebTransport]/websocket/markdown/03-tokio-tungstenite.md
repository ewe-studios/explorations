---
title: tokio-tungstenite — Tokio Async WebSocket Wrapper
---

# tokio-tungstenite — Tokio Async WebSocket Wrapper

tokio-tungstenite wraps tungstenite-rs to provide async WebSocket support for Tokio.

## WebSocketStream

```rust
// tokio-tungstenite/src/lib.rs
pub struct WebSocketStream<S> {
    /// Inner synchronous WebSocket.
    inner: WebSocket<MaybeTlsStream<S>>,
    /// Reading state.
    reading: Reading,
    /// Writing state.
    writing: Writing,
}
```

Source: `tokio-tungstenite/src/lib.rs:1` — `WebSocketStream` wraps the sync WebSocket with async state tracking.

## Stream and Sink Implementation

```rust
impl<S> Stream for WebSocketStream<S>
where S: AsyncRead + AsyncWrite + Unpin {
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Read from inner WebSocket with async wake
    }
}

impl<S> Sink<Message> for WebSocketStream<S>
where S: AsyncRead + AsyncWrite + Unpin {
    type Error = Error;

    fn poll_ready(...) -> Poll<Result<(), Self::Error>> { ... }
    fn start_send(...) -> Result<(), Self::Error> { ... }
    fn poll_flush(...) -> Poll<Result<(), Self::Error>> { ... }
    fn poll_close(...) -> Poll<Result<(), Self::Error>> { ... }
}
```

Source: `tokio-tungstenite/src/lib.rs:1` — Implements both `Stream` (receive) and `Sink` (send).

## connect_async

```rust
// tokio-tungstenite/src/connect.rs
pub async fn connect_async<R>(request: R) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response)>
where R: IntoClientRequest + Unpin {
    // 1. Establish TCP connection (tokio::net::TcpStream)
    // 2. Optionally wrap with TLS
    // 3. Perform async handshake
    // 4. Return WebSocketStream
}
```

Source: `tokio-tungstenite/src/connect.rs:1` — Async version of `tungstenite::connect`.

## MaybeTlsStream

```rust
// tokio-tungstenite/src/stream.rs
pub enum MaybeTlsStream<S> {
    /// Plain TCP stream.
    Plain(S),
    /// TLS-wrapped stream (native-tls).
    NativeTls(native_tls_crate::TlsStream<S>),
    /// TLS-wrapped stream (rustls).
    Rustls(ClientTlsStream<S>),
}
```

Source: `tokio-tungstenite/src/stream.rs:1` — `MaybeTlsStream` abstracts over plain and TLS streams.

## Usage

```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};

let (ws, _) = connect_async("ws://localhost:8080").await?;
let (mut write, mut read) = ws.split();

// Send
write.send(Message::Text("hello".into())).await?;

// Receive
if let Some(Ok(Message::Text(text))) = read.next().await {
    println!("received: {}", text);
}
```

Source: `tokio-tungstenite/src/lib.rs:1` — Example usage pattern.

**Aha:** The `Stream`/`Sink` implementation on `WebSocketStream` means it works seamlessly with any Tokio combinator — `split()`, `forward()`, `next()`, and the full `futures` ecosystem. This is why it's the most popular WebSocket library in the Tokio ecosystem.

## Related Documents

- [tungstenite-rs](../markdown/02-tungstenite-rs.md) — Synchronous core
- [Data Flow](../markdown/09-data-flow.md) — Handshake sequence
