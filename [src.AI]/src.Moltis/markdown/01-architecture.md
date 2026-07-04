# Architecture: Workspace Structure and Crate Map

Moltis is a Rust workspace with 68 crates, an iOS Swift app, a macOS Swift app, and a courier server app. The workspace is organized into modular crates that can be audited independently.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/`

## High-Level Architecture

```mermaid
flowchart TD
    subgraph "Native Apps"
        IOS["iOS App (Swift)
        + Live Activity widget"]
        MACOS["macOS App (Swift)
        Multi-pane UI"]
    end

    subgraph "Server"
        COURIER["courier app
        Rust server entry point"]
        GATEWAY["moltis-gateway
        HTTP/WS server, RPC, auth"]
    end

    subgraph "Agent Core"
        AGENTS["moltis-agents
        Agent loop, streaming"]
        CHAT["moltis-chat
        Chat engine"]
        PROVIDERS["moltis-providers
        LLM provider implementations"]
    end

    subgraph "Tools & Sandbox"
        TOOLS["moltis-tools
        Tool execution, WASM"]
        SANDBOX["Docker/Podman/
        Apple Container/WASM"]
    end

    subgraph "Channels"
        TELEGRAM["Telegram"]
        SIGNAL["Signal"]
        DISCORD["Discord"]
        SLACK["Slack"]
        WHATSAPP["WhatsApp"]
        MATRIX["Matrix"]
        MSTEAMS["MS Teams"]
        NOSTR["Nostr"]
    end

    subgraph "Infrastructure"
        MEMORY["moltis-memory
        SQLite + FTS + vector"]
        AUTH["moltis-auth
        Password + Passkey + API keys"]
        VAULT["moltis-vault
        Secret storage"]
    end

    IOS -->|swift-bridge| COURIER
    MACOS -->|swift-bridge| COURIER
    COURIER --> GATEWAY
    GATEWAY --> AGENTS
    AGENTS --> CHAT
    CHAT --> PROVIDERS
    AGENTS --> TOOLS
    TOOLS --> SANDBOX
    GATEWAY --> TELEGRAM
    GATEWAY --> SIGNAL
    GATEWAY --> DISCORD
    GATEWAY --> SLACK
    GATEWAY --> WHATSAPP
    GATEWAY --> MATRIX
    GATEWAY --> MSTEAMS
    GATEWAY --> NOSTR
    GATEWAY --> MEMORY
    GATEWAY --> AUTH
    AUTH --> VAULT
```

## Core Runtime Crates (by size)

| Crate | LoC | Role |
|-------|-----|------|
| `moltis-gateway` | 37.4K | HTTP/WS server, RPC, auth, startup wiring |
| `moltis-tools` | 37.0K | Tool execution, sandboxing, WASM tools |
| `moltis-providers` | 18.9K | LLM provider implementations |
| `moltis-agents` | 14.5K | Agent loop, streaming, prompt assembly |
| `moltis-chat` | 14.2K | Chat engine, agent orchestration |

**Source:** `README.md` architecture table.

## Crate Dependency Graph

```mermaid
flowchart LR
    COURIER["apps/courier
    Entry point"] --> GATEWAY["gateway
    HTTP/WS server"]
    GATEWAY --> AGENTS["agents
    Agent loop"]
    GATEWAY --> CHANNELS["channels
    Channel routing"]
    GATEWAY --> AUTH["auth
    Authentication"]
    AGENTS --> CHAT["chat
    Chat engine"]
    AGENTS --> TOOLS["tools
    Tool execution"]
    AGENTS --> PROVIDERS["providers
    LLM providers"]
    CHAT --> MEMORY["memory
    SQLite + FTS"]
    TOOLS --> SANDBOX["sandbox
    Docker/WASM/container"]
    CHANNELS --> TELEGRAM["telegram"]
    CHANNELS --> SIGNAL["signal"]
    CHANNELS --> DISCORD["discord"]
    CHANNELS --> SLACK["slack"]
    CHANNELS --> WHATSAPP["whatsapp"]
    CHANNELS --> MATRIX["matrix"]
    CHANNELS --> MSTEAMS["msteams"]
    CHANNELS --> NOSTR["nostr"]
```

## Key Crate Groups

### Agent & Chat

| Crate | Purpose |
|-------|---------|
| `moltis-agents` | Agent loop, streaming, prompt assembly |
| `moltis-chat` | Chat engine, agent orchestration |
| `moltis-sessions` | Session management |

### Channels

| Crate | Purpose |
|-------|---------|
| `moltis-channels` | Channel abstractions |
| `moltis-telegram` | Telegram bot integration |
| `moltis-signal` | Signal messaging |
| `moltis-discord` | Discord bot |
| `moltis-slack` | Slack bot |
| `moltis-whatsapp` | WhatsApp integration |
| `moltis-matrix` | Matrix protocol |
| `moltis-msteams` | MS Teams integration |
| `moltis-nostr` | Nostr protocol |
| `moltis-caldav` | CalDAV calendar integration |
| `moltis-webhooks` | Webhook handling |

### Infrastructure

| Crate | Purpose |
|-------|---------|
| `moltis-gateway` | HTTP/WS server, RPC, auth |
| `moltis-auth` | Authentication (password, passkey, API keys) |
| `moltis-vault` | Secret vault |
| `moltis-config` | Configuration management |
| `moltis-common` | Shared types and utilities |
| `moltis-protocol` | Protocol definitions |

### Tools & Execution

| Crate | Purpose |
|-------|---------|
| `moltis-tools` | Tool execution framework |
| `moltis-skills` | Skill definitions |
| `moltis-browser` | Browser automation |
| `moltis-voice` | Voice I/O (STT/TTS) |
| `moltis-media` | Media handling |

### Memory & Data

| Crate | Purpose |
|-------|---------|
| `moltis-memory` | SQLite + FTS + vector memory |
| `moltis-code-index` | Code indexing |
| `moltis-projects` | Project management |

### Providers & MCP

| Crate | Purpose |
|-------|---------|
| `moltis-providers` | LLM provider implementations |
| `moltis-mcp` | MCP protocol support |
| `moltis-mcp-agent-bridge` | MCP-to-agent bridge |
| `moltis-routing` | Message routing |

### Import Crates

| Crate | Purpose |
|-------|---------|
| `moltis-import-core` | Core import logic |
| `moltis-claude-import` | Import from Claude |
| `moltis-codex-import` | Import from Codex |
| `moltis-hermes-import` | Import from Hermes |
| `moltis-openclaw-import` | Import from OpenClaw |

### Native App Bridge

| Crate | Purpose |
|-------|---------|
| `moltis-swift-bridge` | Rust-Swift FFI layer |

### WASM

| Crate | Purpose |
|-------|---------|
| `moltis-wasm-precompile` | WASM precompilation |
| `moltis-wasm-tools/calc` | WASM calculator tool |
| `moltis-wasm-tools/web-fetch` | WASM web fetch tool |
| `moltis-wasm-tools/web-search` | WASM web search tool |

## Native Apps

### iOS App

Swift iOS app with:
- Live Activity widget
- GraphQL client
- MoltisApp.swift entry point
- Widget extension

**Source:** `apps/ios/`.

### macOS App

Swift macOS app with:
- Multi-pane UI (Chat, Settings, Httpd, Logs)
- Configuration management
- Provider configuration
- Network audit pane
- Onboarding flow

**Source:** `apps/macos/`.

## Courier App

The `apps/courier/` is the Rust server entry point:

```
apps/courier/
├── Cargo.toml
├── src/main.rs          ← Server entry point
├── README.md
└── deploy/
    ├── ansible.cfg
    └── playbook.yml     ← Ansible deployment
```

**Source:** `apps/courier/`.

## Next

Continue to [02-gateway](02-gateway.md) for the HTTP/WS server, RPC, and MCP support.
