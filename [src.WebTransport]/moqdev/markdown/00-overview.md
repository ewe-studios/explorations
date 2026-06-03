---
title: Overview — What MoQ Is and the Media over QUIC Ecosystem
---

# Overview — What MoQ Is and the Media over QUIC Ecosystem

MoQ (Media over QUIC) is an IETF draft protocol for low-latency live media streaming over WebTransport, broadcasting media as tracks of groups and frames.

## The MoQ Data Model

```
Origin → Broadcast → Track → Group → Frame
```

| Level | Description | Ordering |
|-------|-------------|----------|
| **Origin** | 62-bit varint identity for a relay/session | Identity |
| **Broadcast** | Collection of tracks with hop chain | Unordered tracks |
| **Track** | Collection of groups, out-of-order until expired | Out-of-order |
| **Group** | Collection of frames, in-order until cancelled | In-order |
| **Frame** | Chunks with upfront size | In-order |

**Key insight:** The multi-level ordering allows parallel group delivery (different GOPs arrive independently) while maintaining frame ordering within each group. This is essential for live streaming where a viewer can join mid-broadcast and start receiving from the next group without waiting for prior groups.

Source: `moq/rs/moq-net/src/lib.rs:1`, `moq/rs/moq-net/src/model/` — Data model definitions.

## Architecture at a Glance

```mermaid
flowchart TD
    subgraph "Application"
        OBS[OBS Studio / moqbs]
        CLI[moq-cli]
        Web[hang.live / SolidJS]
    end

    subgraph "MoQ Protocol"
        Lite[moq-lite (simplified)]
        IETF[moq-transport (full spec)]
        Negotiator[moq-net negotiator]
    end

    subgraph "Media Layer"
        hang[hang: WebCodecs]
        mux[moq-mux: H.264/H.265/AV1]
        kio[kio: async channels]
    end

    subgraph "WebTransport"
        Quinn[Quinn QUIC]
        Iroh[Iroh QUIC]
        QUICHE[QUICHE]
        noq[noq QUIC]
        WASM[WASM browser]
    end

    subgraph "Infrastructure"
        Relay[moq-relay: media relay]
        Token[moq-token: JWT auth]
    end

    OBS --> hang
    CLI --> Negotiator
    Web --> hang
    hang --> mux
    mux --> kio
    kio --> Negotiator
    Negotiator --> Lite
    Negotiator --> IETF
    Lite --> Quinn
    IETF --> Quinn
    Lite --> Iroh
    IETF --> Iroh
    Lite --> QUICHE
    IETF --> QUICHE
    Lite --> noq
    IETF --> noq
    Lite --> WASM
    Quinn --> Relay
    Iroh --> Relay
    Relay --> Token
```

## Ecosystem Projects

| Project | Description |
|---------|-------------|
| **moq** | Core Rust workspace (17 crates) |
| **web-transport** | WebTransport implementations (11 crates) |
| **moq-go** | Go bindings (v0.2.15) |
| **moq-swift** | Swift bindings (v0.3.0) |
| **moqbs** | OBS Studio fork with MoQ capture |
| **obs** | OBS Studio plugin for MoQ publishing |
| **hang.live** | SolidJS/Vite 8 live streaming app |
| **moq.dev** | Project website (Astro) |
| **smoke** | Cross-language smoke tests (C, Go, JS, Kotlin, Python, Swift) |

Source: `moq/Cargo.toml:1` — Workspace members.

## Quick Start

```rust
// Create a relay
let relay = moq_relay::Server::new(config).await?;

// Publish a broadcast
let session = moq_net::Session::connect(addr).await?;
let broadcast = session.create_broadcast("my-stream").await?;
let track = broadcast.create_track("video").await?;
track.push_group(frames).await?;
```

Source: `moq/rs/moq-relay/src/`, `moq/rs/moq-net/src/`

## Feature Flags

| Feature | Purpose |
|---------|---------|
| `rabbitmq` | RabbitMQ support (moq-net) |

Source: `moq/rs/moq-net/Cargo.toml:features`

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Full dependency graph
- [moq-net](../markdown/02-moq-net.md) — Networking layer
- [WebTransport](../markdown/06-web-transport.md) — QUIC backends
