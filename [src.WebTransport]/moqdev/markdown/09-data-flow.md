---
title: Data Flow — End-to-End Media Streaming Sequences
---

# Data Flow — End-to-End Media Streaming Sequences

This document traces the complete data flow from media capture to playback.

## Publish and Subscribe Flow

```mermaid
sequenceDiagram
    participant OBS as OBS Studio (moqbs)
    participant Hang as hang encoder
    participant Mux as moq-mux
    participant Session as moq-net Session
    participant WT as WebTransport
    participant Relay as moq-relay
    participant Sub as Subscriber

    OBS->>Hang: capture video/audio frames
    Hang->>Mux: encode (H.264/Opus)
    Mux->>Session: create broadcast + track
    Session->>WT: open WebTransport connection
    WT->>Relay: connect
    Relay->>Relay: validate JWT
    Session->>Relay: publish broadcast
    Sub->>Relay: connect + validate JWT
    Sub->>Relay: subscribe to track
    Relay->>Sub: stream groups/frames
    Sub->>Sub: decode and render
```

Source: `moq/rs/moq-net/src/session/`, `moq/rs/moq-relay/src/`, `moq/rs/hang/src/`.

## Protocol Negotiation Flow

```mermaid
sequenceDiagram
    participant Pub as Publisher
    participant Net as moq-net
    participant Sub as Subscriber

    Pub->>Net: connect with supported versions
    Net->>Net: select best common protocol
    Net-->>Pub: negotiated version
    Pub->>Net: create broadcast (negotiated protocol)
    Sub->>Net: connect with supported versions
    Net->>Net: select version
    Net-->>Sub: negotiated version
    Sub->>Net: subscribe (negotiated protocol)
```

Source: `moq/rs/moq-net/src/version.rs:1`, `moq/rs/moq-net/src/setup/`.

## Related Documents

- [moq-net](../markdown/02-moq-net.md) — Protocol negotiation
- [moq-relay](../markdown/03-moq-relay.md) — Relay server
