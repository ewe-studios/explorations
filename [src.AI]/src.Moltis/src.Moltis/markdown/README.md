# src.Moltis Documentation

A secure, persistent personal agent server in Rust. One binary — sandboxed, secure, yours.

| Attribute | Value |
|-----------|-------|
| Language | Rust (+ Swift for native apps) |
| Workspace | 68 crates, ~270K lines, 1038 Rust files |
| License | MIT |
| Runtime | Single Rust binary |
| Native Apps | iOS (51 Swift files), macOS (31 Swift files) |

## Navigation

Start here:

- **[00-overview](00-overview.md)** — What Moltis is, installation, comparison table
- **[01-architecture](01-architecture.md)** — Workspace structure, crate map, dependency graph

Core systems:

- **[02-gateway](02-gateway.md)** — HTTP/WS server, RPC, auth, MCP
- **[03-gateway-deep](03-gateway-deep.md)** — Gateway internals: 129 files, 55.6K LoC
- **[04-agents-chat](04-agents-chat.md)** — Agent loop, chat engine, providers
- **[05-agents-deep](05-agents-deep.md)** — Agent internals: 46 files, 20.3K LoC
- **[06-providers-deep](06-providers-deep.md)** — Provider internals: 72 files, 33.2K LoC
- **[07-channels](07-channels.md)** — Telegram, Signal, Discord, Slack, WhatsApp, Matrix, MS Teams, Nostr

Advanced:

- **[08-tools-sandbox](08-tools-sandbox.md)** — Tool execution, sandboxing, WASM, skills
- **[09-tools-deep](09-tools-deep.md)** — Tools internals: 90 files, 52.9K LoC
- **[10-memory](10-memory.md)** — SQLite + FTS + vector memory

Native:

- **[11-native-apps](11-native-apps.md)** — iOS and macOS Swift apps
- **[12-security](12-security.md)** — Sandboxing, vault, auth, security model
