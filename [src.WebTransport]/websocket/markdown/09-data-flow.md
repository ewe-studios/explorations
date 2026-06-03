---
title: Data Flow — WebSocket Handshake and Message Sequences
---

# Data Flow — WebSocket Handshake and Message Sequences

## HTTP Handshake Flow

```mermaid
sequenceDiagram
    participant Client
    participant Server

    Client->>Server: GET /chat HTTP/1.1
    Note over Client,Server: Upgrade: websocket
    Note over Client,Server: Connection: Upgrade
    Note over Client,Server: Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
    Note over Client,Server: Sec-WebSocket-Version: 13

    Server->>Server: compute accept key
    Server-->>Client: HTTP/1.1 101 Switching Protocols
    Note over Client,Server: Upgrade: websocket
    Note over Client,Server: Connection: Upgrade
    Note over Client,Server: Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
```

Source: `tungstenite-rs/src/client.rs:1` (client handshake), `tungstenite-rs/src/server.rs:1` (server accept).

## Message Exchange Flow

```mermaid
sequenceDiagram
    participant Client
    participant Server

    Client->>Server: Text frame: "Hello!"
    Server->>Client: Text frame: "World!"
    Client->>Server: Binary frame: [0x01, 0x02, 0x03]
    Server-->>Client: Binary frame: [0x04, 0x05]
    Client->>Server: Ping frame
    Server-->>Client: Pong frame
    Client->>Server: Close frame: 1000 "bye"
    Server-->>Client: Close frame: 1000
```

Source: `tungstenite-rs/src/protocol/mod.rs:1` — Frame send/receive.

## Fragmented Message Flow

```mermaid
sequenceDiagram
    participant Sender
    participant Receiver

    Sender->>Receiver: Text frame (FIN=0): "Hello"
    Sender->>Receiver: Continuation frame (FIN=0): ", "
    Sender->>Receiver: Continuation frame (FIN=1): "World!"
    Receiver->>Receiver: concatenate fragments
    Note over Receiver: Complete message: "Hello, World!"
```

Source: `tungstenite-rs/src/protocol/message.rs:1` — Fragmented message handling.

## Related Documents

- [Frame Protocol](../markdown/05-frame-protocol.md) — Frame format
- [tungstenite-rs](../markdown/02-tungstenite-rs.md) — Core implementation
