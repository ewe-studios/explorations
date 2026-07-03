# What Is src.Moltis?

Moltis is a secure, persistent personal agent server written in Rust. One binary serves as the complete agent platform — no Node.js, no npm, no external runtime. It connects to multiple communication channels, runs tools in sandboxes, manages memory, and provides native iOS/macOS apps.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/`

## Key Differentiators

| Feature | Moltis |
|---------|--------|
| Stack | **Rust** — single binary, no runtime |
| Size | ~270K Rust lines across 68 crates |
| Sandbox | Docker/Podman + Apple Container + WASM |
| Auth | Password + Passkey + API keys + Vault |
| Memory | SQLite + FTS + vector search |
| Voice | Built-in STT + TTS providers |
| MCP | stdio + HTTP/SSE |
| Channels | Telegram, Signal, Discord, Slack, WhatsApp, Matrix, MS Teams, Nostr, CalDAV |

## Comparison

| | OpenClaw | Hermes Agent | **Moltis** |
|---|---|---|---|
| Primary stack | TypeScript + Swift/Kotlin | Python + TypeScript | **Rust** |
| Runtime | Node.js + npm/pnpm/bun | Python + uv/pip | **Single Rust binary** |
| Local checkout size | ~1.1M app LoC | ~152K app LoC | **~270K Rust LoC** |
| Architecture | Broad gateway ecosystem | CLI/gateway with learning loop | **Persistent personal agent server** |
| Sandbox | App-level permissions | Local, Docker, SSH, Daytona | **Docker/Podman + Apple Container + WASM** |

**Source:** `README.md` comparison table.

## Installation

```bash
# One-liner (macOS/Linux)
curl -fsSL https://www.moltis.org/install.sh | sh

# Homebrew
brew install moltis-org/tap/moltis

# Docker
docker pull ghcr.io/moltis-org/moltis:latest

# Build from source
cargo install moltis --git https://github.com/moltis-org/moltis
```

## Project Structure

```
moltis/                          ← Rust workspace root
├── apps/
│   ├── courier/                 ← Rust server app (entry point)
│   ├── ios/                     ← iOS Swift app + Live Activity widget
│   └── macos/                   ← macOS Swift app
├── crates/                      ← 68 Rust crates
│   ├── agents/                  ← Agent loop, streaming
│   ├── auth/                    ← Authentication
│   ├── browser/                 ← Browser automation
│   ├── caldav/                  ← Calendar integration
│   ├── canvas/                  ← Canvas rendering
│   ├── channels/                ← Channel abstractions
│   ├── chat/                    ← Chat engine
│   ├── cli/                     ← CLI interface
│   ├── code-index/              ← Code indexing
│   ├── config/                  ← Configuration
│   ├── cron/                    ← Scheduled tasks
│   ├── ctl/                     ← Control interface
│   ├── discord/                 ← Discord channel
│   ├── gateway/                 ← HTTP/WS server, RPC
│   ├── graphql/                 ← GraphQL API
│   ├── httpd/                   ← HTTP server
│   ├── matrix/                  ← Matrix channel
│   ├── mcp/                     ← MCP protocol
│   ├── mcp-agent-bridge/        ← MCP-to-agent bridge
│   ├── media/                   ← Media handling
│   ├── memory/                  ← Memory/RAG
│   ├── metrics/                 ← Observability
│   ├── msteams/                 ← MS Teams channel
│   ├── nostr/                   ← Nostr channel
│   ├── oauth/                   ← OAuth flows
│   ├── plugins/                 ← Plugin system
│   ├── providers/               ← LLM providers
│   ├── routing/                 ← Message routing
│   ├── secret-store/            ← Secret management
│   ├── sessions/                ← Session management
│   ├── signal/                  ← Signal channel
│   ├── skills/                  ← Skill system
│   ├── slack/                   ← Slack channel
│   ├── swift-bridge/            ← Rust-Swift FFI
│   ├── telegram/                ← Telegram channel
│   ├── telephony/               ← Voice/telephony
│   ├── tls/                     ← TLS handling
│   ├── tools/                   ← Tool execution
│   ├── vault/                   ← Secret vault
│   ├── voice/                   ← Voice I/O
│   ├── wasm-precompile/         ← WASM precompilation
│   ├── web/                     ← Web UI
│   ├── webhooks/                ← Webhook handling
│   └── whatsapp/                ← WhatsApp channel
├── moltis-website/              ← Documentation website
└── Cargo.toml                   ← Workspace manifest
```

## Next

Continue to [01-architecture](01-architecture.md) for the workspace structure and crate dependency graph.
