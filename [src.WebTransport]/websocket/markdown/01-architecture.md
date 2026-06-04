---
title: Architecture — tungstenite-rs Layer Diagram and Module Map
---

# Architecture — tungstenite-rs Layer Diagram and Module Map

## Layer Stack

```
┌──────────────────────────────────────────────────┐
│  Application: tokio-tungstenite, websocat        │
├──────────────────────────────────────────────────┤
│  tokio-tungstenite: WebSocketStream<S>            │
│  impl Stream<Item = Result<Message>>             │
│  impl Sink<Message, Error = Error>               │
├──────────────────────────────────────────────────┤
│  tungstenite-rs: WebSocket<Stream>                │
│  RFC6455: handshake, frames, messages, TLS       │
├──────────────────────────────────────────────────┤
│  Stream: TcpStream, TlsStream, UnixStream         │
├──────────────────────────────────────────────────┤
│  OS: TCP sockets, TLS (native-tls / rustls)       │
└──────────────────────────────────────────────────┘
```

## Module Map

### tungstenite-rs

| Module | Purpose |
|--------|---------|
| `lib.rs` | Crate root, re-exports |
| `buffer/` | Read buffer utilities |
| `client/` | Client handshake |
| `server/` | Server accept logic (private: `mod server`, not `pub mod`) |
| `handshake/` | Client/server handshake protocol |
| `protocol/` | WebSocket protocol (Message, WebSocket) |
| `stream/` | Stream abstractions |
| `tls/` | TLS support (native-tls, rustls) |
| `error/` | Error types (thiserror) |
| `util/` | Utility functions |

Source: `tungstenite-rs/src/lib.rs:1`

### tokio-tungstenite

| Module | Purpose |
|--------|---------|
| `lib.rs` | WebSocketStream, connect_async, accept_async |
| `connect.rs` | Async connect + handshake |
| `stream.rs` | MaybeTlsStream wrapper |
| `tls.rs` | TLS configuration |
| `compat.rs` | AllowStd wrapper, ContextWaker, cvt helper |
| `handshake.rs` | client_handshake, server_handshake, without_handshake |

Source: `tokio-tungstenite/src/lib.rs:1`

### websocat

| Module | Purpose |
|--------|---------|
| Main binary | CLI with socat-like specifiers |

Source: `websocat/src/`

## Feature Flags

### tungstenite-rs

| Feature | Enables |
|---------|---------|
| `handshake` | HTTP handshake (data-encoding, http, httparse, sha1) |
| `url` | URL parsing |
| `native-tls` | TLS via native-tls |
| `native-tls-vendored` | Vendored native TLS |
| `rustls-tls-native-roots` | TLS via rustls with system roots |
| `rustls-tls-webpki-roots` | TLS via rustls with webpki roots |

Source: `tungstenite-rs/Cargo.toml:1`

### tokio-tungstenite

| Feature | Enables |
|---------|---------|
| `connect` | `connect_async()` function |
| `handshake` | Handshake support |
| `stream` | Stream/Sink traits |
| `url` | URL parsing |
| `native-tls` | TLS via native-tls |
| `native-tls-vendored` | Vendored native TLS |
| `rustls-tls-native-roots` | TLS via rustls with system roots |
| `rustls-tls-webpki-roots` | TLS via rustls with webpki roots |

Source: `tokio-tungstenite/Cargo.toml:1`

## Key Dependencies

| Dependency | Project | Purpose |
|------------|---------|---------|
| `byteorder` | tungstenite-rs | Byte-level parsing |
| `bytes` | tungstenite-rs | Buffer management |
| `thiserror` | tungstenite-rs | Error type derive |
| `data-encoding` | tungstenite-rs | Sec-WebSocket-Key computation (handshake feature) |
| `rand` | tungstenite-rs | Mask key generation |
| `httparse` | tungstenite-rs | HTTP header parsing |
| `utf-8` | tungstenite-rs | UTF-8 validation |
| `tokio` | tokio-tungstenite | Async runtime |

Source: Both `Cargo.toml` files.

## Related Documents

- [Overview](../markdown/00-overview.md) — What these projects are
- [tungstenite-rs](../markdown/02-tungstenite-rs.md) — Core implementation
