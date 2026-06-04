---
title: Data Flow — Request/Response Sequences
---

# Data Flow — Request/Response Sequences

## Local Request Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant Local as LocalSender (mpsc)
    participant Server as Server Handler

    Client->>Local: call(Multiply { a: 2, b: 3 })
    Local->>Server: send message via mpsc
    Server->>Server: handle request
    Server-->>Local: send response via oneshot
    Local-->>Client: return response
```

Source: `irpc/src/lib.rs:1` — LocalSender implementation.

## RPC Request Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant Transport as QuinnRpcTransport
    participant Quinn as Quinn Streams
    participant Server as Server Handler

    Client->>Transport: call(Multiply { a: 2, b: 3 })
    Transport->>Quinn: open_bi()
    Transport->>Quinn: write [varint + postcard]
    Quinn->>Server: read message
    Server->>Server: handle request
    Server-->>Quinn: write [varint + postcard response]
    Quinn-->>Transport: read response
    Transport-->>Client: return deserialized response
```

Source: `irpc/src/lib.rs:1` — RPC transport documentation.

## Bidi Streaming Flow

```mermaid
sequenceDiagram
    participant Client as Client
    participant Transport as QuinnRpcTransport
    participant Server as Server Handler

    Client->>Transport: call(SumInput) via mpsc
    loop per input
        Client->>Transport: send next number
        Transport->>Server: forward via stream
        Server->>Server: update sum
        Server-->>Transport: send updated sum
        Transport-->>Client: yield via receiver stream
    end
```

Source: `irpc/src/lib.rs:1` — Bidi streaming example.

## Related Documents

- [Local](../markdown/06-local.md) — Local transport
- [RPC Transport](../markdown/05-rpc-transport.md) — Remote transport
