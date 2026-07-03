# src.Moltis Documentation

A secure, persistent personal agent server in Rust. One binary — sandboxed, secure, yours.

| Attribute | Value |
|-----------|-------|
| Language | Rust (+ Swift for native apps) |
| Workspace | 68 crates, ~270K lines |
| License | MIT |
| Runtime | Single Rust binary |

## Navigation

Start here:

- **[00-overview](00-overview.md)** — What Moltis is, installation, comparison table
- **[01-architecture](01-architecture.md)** — Workspace structure, crate map, dependency graph

Core systems:

- **[02-gateway](02-gateway.md)** — HTTP/WS server, RPC, auth, MCP
- **[03-agents-chat](03-agents-chat.md)** — Agent loop, chat engine, LLM providers
- **[04-channels](04-channels.md)** — Telegram, Signal, Discord, Slack, WhatsApp, Matrix, MS Teams, Nostr

Advanced:

- **[05-tools-sandbox](05-tools-sandbox.md)** — Tool execution, sandboxing, WASM, skills
- **[06-memory](06-memory.md)** — SQLite + FTS + vector memory

Native:

- **[07-native-apps](07-native-apps.md)** — iOS and macOS Swift apps
- **[08-security](08-security.md)** — Sandboxing, vault, auth, security model
