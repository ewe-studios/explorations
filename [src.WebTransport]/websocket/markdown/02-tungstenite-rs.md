---
title: tungstenite-rs — RFC6455 Core Implementation
---

# tungstenite-rs — RFC6455 Core Implementation

tungstenite-rs is the core synchronous WebSocket implementation of RFC6455.

## WebSocket Type

```rust
// tungstenite-rs/src/lib.rs
pub struct WebSocket<Stream> {
    /// The underlying stream.
    stream: Stream,
    /// Protocol state.
    protocol: Protocol,
}
```

Source: `tungstenite-rs/src/lib.rs:1` — `WebSocket` wraps any stream implementing `Read + Write`.

## Message Types

```rust
// tungstenite-rs/src/protocol/message.rs
pub enum Message {
    /// UTF-8 text.
    Text(String),
    /// Binary data.
    Binary(Vec<u8>),
    /// Ping frame (2-125 bytes).
    Ping(Vec<u8>),
    /// Pong frame (reply to ping).
    Pong(Vec<u8>),
    /// Close frame with code and reason.
    Close(Option<CloseFrame<'static>>),
}
```

Source: `tungstenite-rs/src/protocol/message.rs:1` — Five message types matching RFC6455 frame opcodes.

## Client Connection

```rust
// tungstenite-rs/src/client.rs
pub fn connect<Req: IntoClientRequest>(request: Req) -> Result<(WebSocket<TcpStream>, Response)> {
    // 1. Parse request URL
    // 2. Establish TCP connection
    // 3. Perform HTTP handshake
    // 4. Return WebSocket + response
}
```

Source: `tungstenite-rs/src/client.rs:1` — `connect()` performs TCP connection + handshake.

## Server Accept

```rust
// tungstenite-rs/src/server.rs
pub fn accept<Stream: Read + Write>(stream: Stream) -> Result<WebSocket<Stream>> {
    // 1. Read HTTP request
    // 2. Validate handshake key
    // 3. Send 101 response
    // 4. Return WebSocket
}
```

Source: `tungstenite-rs/src/server.rs:1` — `accept()` validates the HTTP upgrade request.

## Handshake Key

The WebSocket handshake uses a magic GUID:

```
Sec-WebSocket-Accept = base64(SHA1(Sec-WebSocket-Key + "258EAFA5-E914-47DA-95CA-5AB62E96A3B4"))
```

Source: `tungstenite-rs/src/handshake/` — Handshake key computation.

**Aha:** tungstenite-rs is synchronous by design. It doesn't depend on any async runtime — it works with `Read + Write` streams. This is why it can be wrapped by tokio, async-std, smol, or any other runtime without duplicating the protocol logic.

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Module map
- [tokio-tungstenite](../markdown/03-tokio-tungstenite.md) — Async wrapper
- [Frame Protocol](../markdown/05-frame-protocol.md) — RFC6455 frame format
