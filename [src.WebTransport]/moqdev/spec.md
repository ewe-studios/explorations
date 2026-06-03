---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.MoqDev
repository: git@github.com:moq-dev/moq.git (core)
revised_at: 2026-06-03T00:00:00Z
workspace: moq
---

# MoqDev — Spec File

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.MoqDev`
- **Language:** Rust (primary), Go, Swift, TypeScript, JavaScript, Python
- **Edition:** 2024
- **Rust Version:** 1.85
- **License:** MIT OR Apache-2.0 (varies)
- **Version:** moq v0.18.0-next.1
- **Remotes:** `git@github.com:moq-dev/moq.git`, `git@github.com:moq-dev/web-transport`

## What the Project Is

MoqDev is the Media over QUIC ecosystem — an IETF draft protocol and full implementation for low-latency live media streaming over WebTransport. The Rust workspace has 17 crates: moq-net (networking), moq-relay (relay server), hang (WebCodecs media), moq-mux (muxers), kio (async channels), and application crates (audio, video, CLI, FFI, GStreamer). The WebTransport workspace has 11 crates providing QUIC backend implementations.

## Documentation Goal

After reading, a reader should understand:
1. The MoQ data model: Origin → Broadcast → Track → Group → Frame
2. Protocol negotiation between moq-lite and IETF moq-transport
3. The WebTransport trait abstraction across Quinn/Iroh/QUICHE/noq/WASM
4. The relay server architecture with JWT auth
5. Media encoding with hang (WebCodecs) and moq-mux (H.264/H.265/AV1)
6. The kio async producer/consumer pattern
7. Go/Swift/C FFI bindings
8. Cross-language smoke testing

## Documentation Structure

```
moqdev/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-moq-net.md
│   ├── 03-moq-relay.md
│   ├── 04-hang-media.md
│   ├── 05-moq-mux.md
│   ├── 06-web-transport.md
│   ├── 07-kio.md
│   ├── 08-applications.md
│   ├── 09-data-flow.md
│   └── 10-bindings.md
├── html/
└── build.py
```

## Tasks

| Phase | Document | Status |
|-------|----------|--------|
| Foundation | README.md | DONE |
| Foundation | 00-overview.md | DONE |
| Architecture | 01-architecture.md | DONE |
| Deep Dive | 02-moq-net.md | DONE |
| Deep Dive | 03-moq-relay.md | DONE |
| Deep Dive | 04-hang-media.md | DONE |
| Deep Dive | 05-moq-mux.md | DONE |
| Deep Dive | 06-web-transport.md | DONE |
| Deep Dive | 07-kio.md | DONE |
| Deep Dive | 08-applications.md | DONE |
| Cross-Cutting | 09-data-flow.md | DONE |
| Cross-Cutting | 10-bindings.md | DONE |
| Grandfather Review | All documents | DONE |
| HTML Generation | build.py | DONE |

## Build System

```bash
cd moqdev && python3 build.py
```

## Quality Requirements

All 10 iron rules.

## Expected Outcome

After reading, a developer can understand the MoQ protocol, set up a relay, and integrate with the SDKs.

## Resume Point

Write documents in order: 00 → 01 → 02 → ... → 10.
