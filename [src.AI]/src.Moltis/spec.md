---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/
language: Rust (+ Swift for iOS/macOS apps)
version: (git HEAD, pre-release)
rust_edition: 2024
explored_at: 2026-07-03
license: MIT
---

# spec.md — src.Moltis Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/`
- **Language:** Rust (core) + Swift (iOS/macOS apps)
- **License:** MIT
- **Rust Edition:** 2024
- **Workspace:** 68 crates, ~270K lines of Rust, 1038 Rust files
- **Native Apps:** 51 iOS Swift files, 31 macOS Swift files

## What src.Moltis Is

Moltis is a secure, persistent personal agent server written in Rust. One binary — sandboxed, secure, self-hosted. It serves as a persistent AI agent that connects to multiple communication channels (Telegram, Signal, Discord, Slack, WhatsApp, Matrix, MS Teams, Nostr), runs tools in sandboxes (Docker, Podman, Apple Container, WASM), manages memory via SQLite with FTS and vector search, and provides voice I/O with built-in STT/TTS providers. Native iOS and macOS Swift apps provide a GUI front-end.

## Documentation Goal

A reader should understand:

1. Moltis's architecture: one Rust binary, 68 crates, multiple native apps
2. The crate dependency graph: gateway → agents → chat → providers → tools
3. Gateway internals: 129 files, 55.6K LoC of HTTP/WS/RPC/MCP
4. Agent loop internals: 46 files, 20.3K LoC of agent orchestration
5. Chat engine internals: 27 files, 17.5K LoC of conversation management
6. Provider implementations: 72 files, 33.2K LoC across multiple LLM providers
7. Tools internals: 90 files, 52.9K LoC of tool execution and sandboxing
8. Communication channels: Telegram (27 files), Matrix (14 files), MS Teams (15 files), etc.
9. Memory/RAG: SQLite + FTS + vector search
10. Native apps: iOS (51 Swift files with Live Activity) and macOS (31 Swift files)
11. Security: sandboxing, vault, passkey auth, API keys

## Documentation Structure

```
src.Moltis/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index
│   ├── 00-overview.md             ← What Moltis is, installation, comparison
│   ├── 01-architecture.md         ← Workspace structure, crate map, dependency graph
│   ├── 02-gateway.md              ← HTTP/WS server, RPC, auth, MCP
│   ├── 03-gateway-deep.md         ← Gateway internals: 129 files, 55.6K LoC
│   ├── 04-agents-chat.md          ← Agent loop, chat engine, providers
│   ├── 05-agents-deep.md          ← Agent internals: 46 files, 20.3K LoC
│   ├── 06-providers-deep.md       ← Provider internals: 72 files, 33.2K LoC
│   ├── 07-channels.md             ← Telegram, Signal, Discord, Slack, WhatsApp, Matrix, etc.
│   ├── 08-tools-sandbox.md        ← Tool execution, sandboxing, WASM, skills
│   ├── 09-tools-deep.md           ← Tools internals: 90 files, 52.9K LoC
│   ├── 10-memory.md               ← SQLite + FTS + vector memory
│   ├── 11-native-apps.md          ← iOS and macOS Swift apps
│   └── 12-security.md             ← Sandboxing, vault, auth, security model
├── html/
│   ├── index.html
│   └── *.html
└── build.py
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | README.md (index) | DONE |
| 2 | 00-overview.md | DONE |
| 3 | 01-architecture.md | DONE |
| 4 | 02-gateway.md | DONE |
| 5 | 03-gateway-deep.md | DONE |
| 6 | 04-agents-chat.md | DONE |
| 7 | 05-agents-deep.md | DONE |
| 8 | 06-providers-deep.md | DONE |
| 9 | 07-channels.md | DONE |
| 10 | 08-tools-sandbox.md | DONE |
| 11 | 09-tools-deep.md | DONE |
| 12 | 10-memory.md | DONE |
| 13 | 11-native-apps.md | DONE |
| 14 | 12-security.md | DONE |
| 15 | HTML generation | DONE |
| 16 | Grandfather review | DONE |

## Build System

```bash
cd "[src.AI]/src.Moltis/src.Moltis"
python3 build.py src.Moltis
```

## Expected Outcome

A developer reading these docs understands Moltis's architecture at the crate level, how channels and tools work, and how the native apps connect to the server.
