---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/irpc
repository: git@github.com:n0-computer/irpc
revised_at: 2026-06-03T00:00:00Z
workspace: irpc (with irpc-derive, irpc-iroh)
---

# irpc — Spec File

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/irpc`
- **Language:** Rust
- **Edition:** 2021
- **Rust Version:** 1.76
- **License:** MIT OR Apache-2.0
- **Version:** 0.5.0
- **Remote:** `git@github.com:n0-computer/irpc`
- **Workspace Members:** `irpc`, `irpc-derive`, `irpc-iroh`

## What the Project Is

irpc is a minimal streaming RPC library designed for iroh. It evolved from quic-rpc but removes transport abstraction — it's specifically for Quinn/iroh QUIC streams. Key design: lightweight enough for in-process async boundaries without overhead, while also supporting cross-process and cross-network RPC transparently.

## Documentation Goal

After reading, a reader should understand:
1. The Service trait and rpc_requests derive macro
2. Channel types: oneshot, mpsc, none, and their composition
3. Client/Server interaction patterns (rpc, streaming, bidi)
4. Local vs RPC transport transparency
5. Postcard serialization with length prefixes
6. The irpc-derive procedural macro
7. irpc-iroh integration

## Documentation Structure

```
irpc/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-service.md
│   ├── 03-channels.md
│   ├── 04-client.md
│   ├── 05-rpc-transport.md
│   ├── 06-local.md
│   ├── 07-derive-macro.md
│   ├── 09-data-flow.md
│   └── 10-cross-cutting.md
├── html/
└── build.py
```

## Tasks — All DONE

## Build System

```bash
cd irpc && python3 build.py
```

## Quality Requirements

All 10 iron rules.

## Expected Outcome

After reading, a developer can define irpc services, use the derive macro, and integrate with iroh.

## Resume Point

Write documents in order.
