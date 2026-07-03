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
- **Workspace:** 59+ crates, ~270K lines of Rust

## What src.Moltis Is

Moltis is a secure, persistent personal agent server written in Rust. One binary — sandboxed, secure, self-hosted. It serves as a persistent AI agent that connects to multiple communication channels (Telegram, Signal, Discord, Slack, WhatsApp, Matrix, MS Teams, Nostr), runs tools in sandboxes (Docker, Podman, Apple Container, WASM), manages memory via SQLite with FTS and vector search, and provides voice I/O with built-in STT/TTS providers. Native iOS and macOS Swift apps provide a GUI front-end.

## Documentation Goal

A reader should understand:

1. Moltis's architecture: one Rust binary, 59 crates, multiple native apps
2. The crate dependency graph: gateway → agents → chat → providers → tools
3. Communication channels: Telegram, Signal, Discord, Slack, WhatsApp, Matrix, MS Teams, Nostr, CalDAV
4. The tool system: sandboxing (Docker, WASM, Apple Container), skill execution
5. Memory/RAG: SQLite + FTS + vector search
6. The gateway: HTTP/WS server, RPC, auth, MCP support
7. Native apps: iOS (with Live Activity widget) and macOS (with multiple panes)
8. Security: sandboxing, vault, passkey auth, API keys

## Documentation Structure

```
src.Moltis/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index
│   ├── 00-overview.md             ← What Moltis is, installation, comparison
│   ├── 01-architecture.md         ← Workspace structure, crate map, dependency graph
│   ├── 02-gateway.md              ← HTTP/WS server, RPC, auth, MCP
│   ├── 03-agents-chat.md          ← Agent loop, chat engine, providers
│   ├── 04-channels.md             ← Telegram, Signal, Discord, Slack, WhatsApp, Matrix, etc.
│   ├── 05-tools-sandbox.md        ← Tool execution, sandboxing, WASM, skills
│   ├── 06-memory.md               ← SQLite + FTS + vector memory
│   ├── 07-native-apps.md          ← iOS and macOS Swift apps
│   └── 08-security.md             ← Sandboxing, vault, auth, security model
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
| 5 | 03-agents-chat.md | DONE |
| 6 | 04-channels.md | DONE |
| 7 | 05-tools-sandbox.md | DONE |
| 8 | 06-memory.md | DONE |
| 9 | 07-native-apps.md | DONE |
| 10 | 08-security.md | DONE |
| 11 | HTML generation | DONE |
| 12 | Grandfather review | DONE (fixed crate count 59→68) |

## Build System

```bash
cd "[src.AI]/src.Moltis/src.Moltis"
python3 build.py src.Moltis
```

## Expected Outcome

A developer reading these docs understands Moltis's architecture at the crate level, how channels and tools work, and how the native apps connect to the server.
