---
title: MoqDev Documentation — Index
---

# MoqDev Documentation

> Media over QUIC: low-latency live streaming over WebTransport

MoqDev implements the IETF MoQ draft protocol for broadcasting media as tracks of groups and frames over QUIC connections.

## Foundation

- [Overview](00-overview.html) — What MoQ is, data model, ecosystem
- [Architecture](01-architecture.html) — Full dependency graph, layer diagram, module map

## Protocol and Core

- [moq-net](02-moq-net.html) — Networking: Origin/Broadcast/Track/Group/Frame, protocol negotiation
- [moq-relay](03-moq-relay.html) — Relay server with JWT auth, clustering, WebSocket fallback
- [hang](04-hang-media.html) — WebCodecs media encoding: catalogs, codecs, containers
- [moq-mux](05-moq-mux.html) — Media muxers: H.264/H.265/AV1/WebM/MP4/HLS

## Transport and Utilities

- [WebTransport](06-web-transport.html) — Trait-based QUIC abstraction: Quinn/Iroh/QUICHE/noq/WASM
- [kio](07-kio.html) — Async producer/consumer with shared state and waker notification
- [Applications](08-applications.md) — moq-cli, moq-audio, moq-video, moq-boy, moq-token

## Cross-Cutting

- [Data Flow](09-data-flow.html) — End-to-end media streaming sequences
- [Bindings](10-bindings.html) — Go, Swift, C FFI, JavaScript, Python bindings

---

Generated from source code. Every claim traces back to implementation.
