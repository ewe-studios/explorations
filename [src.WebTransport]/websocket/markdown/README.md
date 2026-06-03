---
title: src.websocket Documentation — Index
---

# src.websocket Documentation

> WebSocket implementations: tungstenite-rs, tokio-tungstenite, websocat

RFC6455 WebSocket implementations from the Snapview organization and the websocat CLI tool.

## Foundation

- [Overview](00-overview.html) — What these projects are, WebSocket ecosystem
- [Architecture](01-architecture.html) — Layer diagram, module map

## Core Implementations

- [tungstenite-rs](02-tungstenite-rs.html) — RFC6455 core implementation
- [tokio-tungstenite](03-tokio-tungstenite.html) — Tokio async wrapper
- [websocat](04-websocat.html) — CLI WebSocket tool (socat for ws://)

## Protocol

- [Frame Protocol](05-frame-protocol.html) — RFC6455 frame format, opcodes, masking

## Cross-Cutting

- [Data Flow](09-data-flow.html) — Handshake, message, close sequences
- [Cross-Cutting](10-cross-cutting.html) — TLS, feature flags, deprecated rust-websocket

---

Generated from source code.
