---
title: websocat — CLI WebSocket Tool
---

# websocat — CLI WebSocket Tool (socat for ws://)

websocat is a command-line WebSocket tool — like netcat or socat but for `ws://` and `wss://`.

## Features

| Feature | Description |
|---------|-------------|
| **Client mode** | Connect to WebSocket servers |
| **Server mode** | Accept WebSocket connections |
| **Proxy** | WebSocket-to-TCP proxy |
| **Unix sockets** | Unix domain socket support |
| **UDP** | UDP transport |
| **Process pipes** | Pipe to/from subprocesses |
| **TLS** | SSL/TLS support |
| **Compression** | flate2 compression |
| **Crypto peer** | chacha20poly1305 + argon2 encryption |
| **Metrics** | Prometheus metrics |
| **Specifiers** | Socat-like specifier syntax |

Source: `websocat/src/`

## Usage

```bash
# Client
websocat ws://localhost:8080

# Server
websocat --server ws-listen:0.0.0.0:8080

# Proxy
websocat ws-listen:0.0.0.0:8080 tcp:backend:3000

# With encryption
websocat --crypto-pass mypassword ws://server
```

Source: `websocat/Cargo.toml:1`, `websocat/src/`

## Architecture

```
┌──────────────────────────────────────────┐
│              websocat                    │
│                                          │
│  Specifier Parser ──▶ Lints ──▶ Execute  │
│                                          │
│  Specifiers:                             │
│  ws://, wss://, tcp:, udp:, unix:,       │
│  exec:, mirror:, autoreconnect:          │
└──────────────────────────────────────────┘
```

Source: `websocat/src/` — Specifier parsing and execution.

## Dependencies

websocat uses older dependencies:
- `tokio 0.1`
- `futures 0.1`
- `websocket 0.27.1` (the deprecated rust-websocket crate)

Source: `websocat/Cargo.toml:1`

**Key insight:** websocat is NOT compatible with modern tokio (1.x). It uses tokio 0.1 and futures 0.1, which are incompatible with the current async ecosystem. A modernization effort would be valuable.

## Related Documents

- [Overview](../markdown/00-overview.md) — WebSocket ecosystem
- [Cross-Cutting](../markdown/10-cross-cutting.md) — Feature flags, deprecated crates
