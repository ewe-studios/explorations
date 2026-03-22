# src.openclaw Exploration

## Executive Summary

**src.openclaw** is the main source repository for **OpenClaw**, a multi-channel AI gateway with extensible messaging integrations. The project provides a comprehensive platform for building AI assistants that can operate across various communication channels (Telegram, Discord, Slack, Signal, WhatsApp, iMessage, etc.) with a focus on security, privacy, and self-hosting capabilities.

**Repository Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/src.openclaw/`

**Primary Codebase:** `openclaw/` subdirectory (TypeScript/Node.js)
**Rust Components:** Scattered throughout `skills/skills/` as independent skill packages

---

## Table of Contents

1. [Repository Structure](#repository-structure)
2. [Core Architecture](#core-architecture)
3. [Key Components](#key-components)
4. [Rust Implementation Details](#rust-implementation-details)
5. [Design Patterns](#design-patterns)
6. [Production Considerations](#production-considerations)
7. [Related Projects](#related-projects)

---

## Repository Structure

```
src.openclaw/
├── openclaw/                          # Main TypeScript/Node.js codebase
│   ├── src/
│   │   ├── cli/                       # CLI implementation (157 files)
│   │   ├── gateway/                   # Gateway server (231 files)
│   │   ├── config/                    # Configuration system (204 files)
│   │   ├── agents/                    # Agent system (4480 files)
│   │   ├── plugin-sdk/                # Plugin SDK (110 files)
│   │   ├── channels/                  # Channel implementations
│   │   ├── security/                  # Security modules
│   │   └── [50+ more directories]
│   ├── extensions/                    # Extension packages
│   ├── packages/
│   │   ├── clawdbot/                  # Core bot package
│   │   └── moltbot/                   # Moltbot package
│   ├── apps/                          # Application builds
│   ├── docs/                          # Documentation (Mintlify)
│   ├── test/                          # Test suites
│   └── ui/                            # UI components
│
├── barnacle/                          # TypeScript project
├── clawgo/                            # Go project (bot framework)
├── lobster/                           # TypeScript project
├── clawhub/                           # Hub/clawbot project
├── clawdinators/                      # Automation tools
├── butter.bot/                        # Bot implementation
├── casa/                              # [TBD]
├── flawd-bot/                         # Bot implementation
├── nix-openclaw/                      # Nix package definitions
├── nix-steipete-tools/                # Nix tools
├── openclaw.ai/                       # Website/installers
├── openclaw-ansible/                  # Ansible deployment
└── skills/skills/                     # Skill marketplace
    ├── 0xnyk/xint-rs/                 # X/Twitter intelligence (Rust)
    ├── apollostreetcompany/clauditor/ # Security watchdog (Rust)
    ├── bowen31337/clawchain/          # Blockchain RPC client (Rust)
    ├── dendisuhubdy/clawnet/          # Network tools (Rust)
    └── [100+ more skills]
```

---

## Core Architecture

### High-Level Architecture

OpenClaw follows a **hub-and-spoke architecture** with the following layers:

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Layer                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐            │
│  │Telegram │  │ Discord │  │  Slack  │  │ WhatsApp│  ...       │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘            │
└───────┼────────────┼────────────┼────────────┼──────────────────┘
        │            │            │            │
┌───────▼────────────▼────────────▼────────────▼──────────────────┐
│                    Channel Adapter Layer                         │
│  (src/telegram, src/discord, src/slack, src/whatsapp, etc.)     │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                      Gateway Core                                │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Session Management  │  Message Routing  │  Auth         │    │
│  │  Chat Processing     │  Event Bus        │  Rate Limit   │    │
│  │  Tool Invocation     │  Hook System      │  Audit Log    │    │
│  └─────────────────────────────────────────────────────────┘    │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                      Agent Layer                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Agent Scope       │  Memory System    │  Skills        │    │
│  │  Context Engine    │  Model Providers  │  Tools         │    │
│  └─────────────────────────────────────────────────────────┘    │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                   Extension/Skill Layer                          │
│  (Plugin SDK, Rust skills, external integrations)               │
└─────────────────────────────────────────────────────────────────┘
```

### Gateway Architecture

The Gateway is the central orchestrator:

| Component | Files | Purpose |
|-----------|-------|---------|
| `server.impl.ts` | 36.9K lines | Main server implementation |
| `server-http.ts` | 26.9K lines | HTTP API server |
| `server-cron.ts` | 16.9K lines | Scheduled job system |
| `server-channels.ts` | 15.5K lines | Channel management |
| `server-chat.ts` | 19.6K lines | Chat/session handling |
| `server-node-events.ts` | 19.5K lines | Node event system |
| `hooks.ts` | 12.9K lines | Hook execution engine |
| `auth.ts` | 15.6K lines | Authentication system |

### Session Management

OpenClaw uses a sophisticated session system:

- **Session Keys:** Format `agent:<agentId>:<mainKey>` or `acp:<uuid>`
- **DM Scoping Modes:**
  - `main` - All DMs share one session
  - `per-peer` - Isolate by sender ID
  - `per-channel-peer` - Isolate by channel + sender
  - `per-account-channel-peer` - Isolate by account + channel + sender

- **Session Storage:** JSON files with HMAC integrity checks
- **Compaction:** Automatic history compaction for long sessions

---

## Key Components

### 1. CLI System (`src/cli/`)

The CLI provides 150+ commands across these categories:

| Category | Commands |
|----------|----------|
| Configuration | `config`, `secrets`, `memory` |
| Channels | `channels`, `pairing`, `webhooks` |
| Gateway | `gateway`, `nodes`, `models` |
| Agents | `agents`, `skills`, `plugins` |
| Security | `security`, `auth`, `exec-approvals` |
| Monitoring | `logs`, `system`, `health` |
| Tools | `browser`, `directory`, `dns`, `qr` |

Key files:
- `program.ts` - Command program wiring
- `argv.ts` - Argument parsing (8.1K lines)
- `config-cli.ts` - Configuration management
- `gateway-cli.ts` - Gateway control
- `acp-cli.ts` - Agent Client Protocol bridge

### 2. Plugin SDK (`src/plugin-sdk/`)

110 files providing:
- Channel plugin interfaces
- Authentication helpers
- Message formatting
- Rate limiting utilities
- SSRF protection
- Webhook handling

### 3. Configuration System (`src/config/`)

204 files handling:
- Schema validation (Zod-based)
- Environment variable substitution
- Multi-agent configuration
- Secret management
- Plugin validation
- Legacy migration

### 4. Agent System (`src/agents/`)

Core agent capabilities:
- Agent scope isolation
- Apply-patch tool implementation
- Auth profiles with cooldowns
- MCP (Model Context Protocol) support
- Tool invocation with approvals

---

## Rust Implementation Details

### X Intelligence CLI (`skills/skills/0xnyk/xint-rs/`)

A high-performance X/Twitter intelligence tool written in Rust.

**Architecture:**
```
src/
├── main.rs          # Entry point, CLI parsing
├── cli.rs           # Command definitions (18.7K lines)
├── client.rs        # X API client (9.2K lines)
├── config.rs        # Configuration loading
├── api/
│   ├── mod.rs       # API module
│   ├── grok.rs      # Grok AI integration
│   ├── twitter.rs   # Twitter API
│   └── xai.rs       # X.AI integration
├── auth/
│   ├── mod.rs       # Auth module
│   └── oauth.rs     # OAuth handling
├── commands/        # 28 command modules
│   ├── search.rs    # Tweet search
│   ├── watch.rs     # Real-time monitoring
│   ├── stream.rs    # Filtered stream
│   ├── analyze.rs   # AI analysis
│   └── ...
├── costs.rs         # Token cost tracking
├── format.rs        # Output formatting
├── mcp.rs           # MCP integration (26.5K lines)
├── models.rs        # Data models (9.9K lines)
├── policy.rs        # Policy/permission system
├── reliability.rs   # Retry/fallback logic
└── sentiment.rs     # Sentiment analysis
```

**Key Design Patterns:**
- Policy-based command allowlisting (`ReadOnly`, `Engagement`, `Moderation`)
- Async-first with Tokio runtime
- Zero runtime dependencies (static binary)
- Sub-5ms startup, 2.5MB binary size

**Cargo.toml Dependencies:**
```toml
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls", "multipart"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
# ... plus sha2, md-5, base64, csv, etc.
```

### Clauditor (`skills/skills/apollostreetcompany/clauditor/`)

A tamper-resistant security audit watchdog for Clawdbot.

**Workspace Structure:**
```
clauditor/
├── Cargo.toml (workspace)
└── crates/
    ├── schema/        # Log schema, HMAC chain verification
    ├── detector/      # Rule-based detection engine
    ├── collector/     # Event collection (fanotify)
    ├── alerter/       # Alert dispatch
    ├── writer/        # Append-only log writer
    └── clauditor-cli/ # CLI interface
```

**Security Model:**
| Component | Owner | Permissions | Clawdbot Access |
|-----------|-------|-------------|-----------------|
| Daemon | sysaudit | runs as sysaudit user | Cannot kill |
| HMAC Key | root:sysaudit | 640 | Cannot read |
| Log Dir | sysaudit:sysaudit | 750 | Cannot write |
| Logs | sysaudit | 640 | Can read (tamper-evident) |

**Detection Capabilities:**
- Exec-only monitoring via FAN_OPEN_EXEC
- Sequence detection (credential read → network exfil)
- Command baseline tracking
- Orphan execution detection
- Tamper attempt alerts

### Other Rust Projects

| Project | Path | Purpose |
|---------|------|---------|
| clawchain-rpc-client | `skills/skills/bowen31337/clawchain/` | Blockchain RPC client |
| clawnet | `skills/skills/dendisuhubdy/clawnet/` | Network utilities |
| xint-rs | `skills/skills/0xnyk/xint-rs/` | X/Twitter intelligence |
| clauditor | `skills/skills/apollostreetcompany/clauditor/` | Security watchdog |

---

## Design Patterns

### 1. Channel Abstraction Pattern

All channels implement a common interface:
```typescript
interface ChannelPlugin {
  send(): Promise<SendResult>;
  poll(): AsyncIterable<InboundMessage>;
  getCapabilities(): ChannelCapabilities;
  handleWebhook(req: Request): Promise<Response>;
}
```

### 2. Gateway Session Pattern

Sessions are isolated by key with configurable scoping:
```typescript
// Session key derivation
const sessionKey = `agent:${agentId}:${mainKey}`;

// DM scoping policies
enum DMScope {
  Main = 'main',                    // All DMs share session
  PerPeer = 'per-peer',             // One session per sender
  PerChannelPeer = 'per-channel',   // One session per channel+sender
  PerAccountChannelPeer = 'per-account-channel-peer'
}
```

### 3. Hook System Pattern

Hooks are user-defined JavaScript/TypeScript functions:
```typescript
// Hook registration in config
{
  hooks: {
    'pre-send': './hooks/pre-send.ts',
    'post-receive': './hooks/post-receive.ts'
  }
}
```

### 4. Plugin SDK Boundary

Plugins run in a sandboxed context with:
- Isolated module resolution
- Capability-based permissions
- SSRF protection for webhooks
- Rate limiting enforcement

### 5. Policy-Based Access Control

Used in Rust projects:
```rust
#[derive(ValueEnum, Clone)]
enum PolicyMode {
    ReadOnly,    // No write operations
    Engagement,  // Allow replies/interactions
    Moderation,  // Allow moderation actions
}

// Policy check before command execution
if !policy::is_allowed(cli.policy, required) {
    policy::emit_policy_denied(cmd, cli.policy, required);
    std::process::exit(2);
}
```

---

## Production Considerations

### Security

1. **Threat Model** (from SECURITY.md):
   - Assumes gateway may be compromised
   - Protects against credential exfiltration
   - Tamper-evident logging required

2. **Auth Modes:**
   - Device pairing (QR code)
   - Token-based API auth
   - Origin-based CORS for web UI

3. **Secret Management:**
   - Encrypted at rest
   - In-memory only during runtime
   - Support for external secret stores

### Deployment Options

| Method | Description |
|--------|-------------|
| Docker | Official images with sandbox support |
| Nix | NixOS/home-manager modules |
| Ansible | `openclaw-ansible` roles |
| Manual | Binary + config files |

### Observability

- Structured JSON logging
- OpenTelemetry integration (diagnostics-otel.ts)
- Health check endpoints
- Prometheus metrics endpoint

### Performance

- Session compaction for long-running chats
- Message chunking for large payloads
- Rate limiting at multiple layers
- Async I/O throughout

---

## Related Projects

### In This Repository

| Project | Type | Status |
|---------|------|--------|
| openclaw | TypeScript | Main codebase |
| barnacle | TypeScript | [TBD] |
| clawgo | Go | Bot framework |
| lobster | TypeScript | [TBD] |
| clawhub | Unknown | Hub system |
| clawdinators | Unknown | Automation |
| butter.bot | Unknown | Bot impl |
| casa | Unknown | [TBD] |
| flawd-bot | Unknown | Bot impl |

### External Repositories

| Repository | Purpose |
|------------|---------|
| openclaw/openclaw | Main repository |
| openclaw/clawhub | Hub/management |
| openclaw/trust | Trust model documentation |
| openclaw.ai | Website/installers |

---

## Key Documentation

| Document | Path |
|----------|------|
| README | `openclaw/README.md` (120K lines) |
| AGENTS.md | `openclaw/AGENTS.md` (25K lines) |
| VISION.md | `openclaw/VISION.md` |
| SECURITY.md | `openclaw/SECURITY.md` (20K lines) |
| CHANGELOG | `openclaw/CHANGELOG.md` (649K lines) |
| ACP Bridge | `openclaw/docs.acp.md` |
| Session Management | `openclaw/docs/concepts/session.md` |

---

## Files of Interest

### Core Implementation Files
- `openclaw/src/index.ts` - Main entry point
- `openclaw/src/gateway/server.impl.ts` - Gateway core
- `openclaw/src/gateway/server-http.ts` - HTTP server
- `openclaw/src/cli/program.ts` - CLI wiring
- `openclaw/src/config/io.ts` - Config I/O (45K lines)
- `openclaw/src/config/schema.help.ts` - Config schema (152K lines)

### Rust Implementation Files
- `skills/skills/0xnyk/xint-rs/src/main.rs` - X Intelligence entry
- `skills/skills/0xnyk/xint-rs/src/mcp.rs` - MCP integration
- `skills/skills/apollostreetcompany/clauditor/crates/clauditor-cli/src/main.rs` - Clauditor CLI
- `skills/skills/apollostreetcompany/clauditor/crates/detector/src/lib.rs` - Detection engine

---

## Summary

**src.openclaw** is a comprehensive, production-grade AI messaging platform with:

1. **Multi-channel support** - 15+ messaging platforms
2. **Extensible architecture** - Plugin SDK and skill system
3. **Security-first design** - Tamper-evident logging, secret management
4. **Hybrid implementation** - TypeScript core with Rust skills
5. **Production ready** - Docker, Nix, Ansible deployment options

The codebase demonstrates sophisticated patterns for:
- Session management and isolation
- Event-driven architecture
- Policy-based access control
- Audit logging and compliance
- Plugin sandboxing

Total codebase: ~500K+ lines across TypeScript, Rust, and Go implementations.
