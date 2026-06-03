---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.websocket
repository: N/A (snapshot copies)
revised_at: 2026-06-03T00:00:00Z
---

# src.websocket — Spec File

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.websocket/`
- **Language:** Rust (primary), TypeScript
- **License:** MIT OR Apache-2.0
- **Note:** Snapshot copies, not git clones

## What the Project Is

Collection of WebSocket implementations: tungstenite-rs (RFC6455 core), tokio-tungstenite (async), websocat (CLI), deprecated rust-websocket, and Sunrise reactive DOM library.

## Documentation Structure

```
websocket/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-tungstenite-rs.md
│   ├── 03-tokio-tungstenite.md
│   ├── 04-websocat.md
│   ├── 05-frame-protocol.md
│   ├── 09-data-flow.md
│   └── 10-cross-cutting.md
├── html/
└── build.py
```

## Tasks — All DONE

## Build System

```bash
cd websocket && python3 build.py
```

## Quality Requirements

All 10 iron rules.

## Expected Outcome

After reading, a developer understands the tungstenite-rs WebSocket implementation, the async stack, and the websocat CLI tool.

## Resume Point

Write documents in order.
