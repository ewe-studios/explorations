---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/
language: Rust (+ Swift for iOS/macOS apps)
version: (git HEAD, pre-release)
rust_edition: 2024
explored_at: 2026-07-03
updated_at: 2026-07-10 — mermaid diagrams and Aha moments injected into all docs; grandfather review completed
license: MIT
---

# spec.md — src.Moltis Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/`
- **Language:** Rust (core) + Swift (iOS/macOS apps)
- **License:** MIT
- **Rust Edition:** 2024
- **Workspace:** 68 crates + 3 apps, ~270K lines of Rust, 1038 Rust files
- **Native Apps:** 51 iOS Swift files, 31 macOS Swift files

## What src.Moltis Is

Moltis is a secure, persistent personal agent server written in Rust. One binary — sandboxed, secure, self-hosted. It serves as a persistent AI agent that connects to multiple communication channels (Telegram, Signal, Discord, Slack, WhatsApp, Matrix, MS Teams, Nostr), runs tools in sandboxes (Docker, Podman, Apple Container, WASM), manages memory via SQLite with FTS and vector search, and provides voice I/O with built-in STT/TTS providers. Native iOS and macOS Swift apps provide a GUI front-end.

## Documentation Goal

A reader should understand:

1. Moltis's architecture: one Rust binary, 68 crates + 3 apps, multiple native apps
2. The crate dependency graph: gateway → agents → chat → providers → tools
3. Gateway internals: method dispatch, session registry, channel store, webhook middleware
4. Agent loop internals: streaming execution, tool accumulation, retry, loop detection, compaction
5. Chat engine internals: conversation management, provider chain failover
6. Provider implementations: LlmProvider trait, model discovery, capability detection
7. Tools internals: tool execution, sandboxing, policy engine
8. Communication channels: ChannelPlugin trait, 9 channel types, webhook verification
9. Memory/RAG: SQLite + FTS + vector search, hybrid search with RRF
10. Native apps: iOS (51 Swift files with Live Activity) and macOS (31 Swift files)
11. Security: sandboxing, vault, passkey auth, API keys
12. Voice pipeline: 11 STT + 6 TTS providers

## Documentation Structure

```
src.Moltis/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index
│   ├── 00-overview.md             ← What Moltis is, architecture, key decisions
│   ├── 01-architecture.md         ← Workspace structure, crate map, dependency graph
│   ├── 01-gateway.md              ← Gateway: central hub, method dispatch, sessions
│   ├── 02-gateway.md              ← Config: loading, validation, schema
│   ├── 02-agent-loop.md           ← Agent loop: streaming, retry, loop detection
│   ├── 03-agents-chat.md          ← Complete message flow: channel → agent → reply
│   ├── 03-channels.md             ← ChannelPlugin trait, 9 channel types
│   ├── 03-gateway-deep.md         ← Deep dive: voice, channels, agents, providers, apps
│   ├── 04-channels.md             ← Channel event flow, webhook verification
│   ├── 04-providers.md            ← LLM provider registry, model discovery
│   ├── 05-agents-deep.md          ← Voice pipeline: 11 STT + 6 TTS providers
│   ├── 05-tools-sandbox.md        ← Tool execution, 6-layer policy, sandboxes
│   ├── 06-memory.md               ← Memory: chunking, embeddings, hybrid search
│   ├── 06-providers-deep.md       ← Provider implementations, model capabilities
│   ├── 07-native-apps.md          ← iOS/macOS Swift apps, FFI bridges
│   ├── 08-security.md             ← Auth, sandboxing, unsafe code isolation
│   ├── 09-tools-deep.md           ← Tool registration, sandbox routing, background tasks
│   └── ... (additional deep dives)
├── html/
│   ├── index.html
│   └── *.html
└── build.py
```

## Tasks

| # | Document | Status | Mermaid | Aha Moments |
|---|----------|--------|---------|-------------|
| 1 | README.md (index) | DONE | N/A | N/A |
| 2 | 00-overview.md | DONE | 2 | 1 |
| 3 | 01-architecture.md | DONE | 3 | 4 |
| 4 | 01-gateway.md | DONE | 2 | 2 |
| 5 | 02-gateway.md | DONE | 2 | 4 |
| 6 | 02-agent-loop.md | DONE | 2 | 3 |
| 7 | 03-agents-chat.md | DONE | 2 | 5 |
| 8 | 03-channels.md | DONE | 2 | 1 |
| 9 | 03-gateway-deep.md | DONE | 2 | 4 |
| 10 | 04-channels.md | DONE | 2 | 4 |
| 11 | 04-providers.md | DONE | 2 | 1 |
| 12 | 05-agents-deep.md (voice) | DONE | 2 | 5 |
| 13 | 05-tools-sandbox.md | DONE | 2 | 2 |
| 14 | 06-memory.md | DONE | 2 | 5 |
| 15 | 06-providers-deep.md | DONE | 2 | 5 |
| 16 | 07-native-apps.md | DONE | 2 | 5 |
| 17 | 08-security.md | DONE | 2 | 5 |
| 18 | 09-tools-deep.md | DONE | 2 | 5 |
| 19 | HTML generation | DONE | -- | -- |
| 20 | Grandfather review | DONE | -- | -- |

**Total: 18 docs, ~25,000 words, 36 mermaid diagrams, 56 Aha moments.**

## Grandfather Review Findings

All names, numbers, and flows verified against source code:

- **Names match:** `ChannelType` enum, `SttProvider`/`TtsProvider` traits, `ToolCallFingerprint`, `ChannelPlugin`, `LlmProvider` — all match source.
- **Numbers match:** 11 STT providers (whisper, whisper_cli, whisper_local, deepgram, elevenlabs, google, groq, mistral, sherpa_onnx, voxtral_local, openai_compat), 6 TTS providers (elevenlabs, openai, google, coqui, piper, mock), `CACHE_MAX_ROWS = 50_000`, `lazy mode * 3` iteration multiplier, 9 channel types, 4 `TtsAutoMode` variants.
- **Flows match:** Agent loop (stream → parse → execute → compact → repeat), webhook middleware (verify → dedup → rate-limit → handler), memory sync (mtime check → hash check → chunk → embed → store → FTS5).
- **Coverage:** All public API surfaces documented. 68 crates + 3 apps in workspace. 1038 Rust files confirmed.

## Build System

```bash
cd "/home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Moltis"
python3 build.py .
```

## Expected Outcome

A developer reading these docs understands Moltis's architecture at the crate level, how channels and tools work, and how the native apps connect to the server.
