# Gateway Deep Dive: HTTP/WS Server, RPC, MCP, Auth, Channels

The gateway crate (`moltis-gateway`) is the largest in the workspace at 129 files and ~55,619 lines of Rust. It handles HTTP/WebSocket serving, RPC dispatch, authentication, MCP protocol support, channel management, and startup wiring.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/gateway/src/`

## Crate Structure (68 modules)

### Core Modules

| Module | Files | Purpose |
|--------|-------|---------|
| `lib.rs` | 1 | Crate entry point, module declarations, `run_migrations()` |
| `server/` | 10 | Startup, initialization, seeding, workspace management |
| `state.rs` | 1 | `GatewayState` — the core application state object |
| `methods.rs` | 1 | RPC method handler registration |
| `services/` | — | Service implementations (domain logic) |

### Authentication

| Module | Purpose |
|--------|---------|
| `auth.rs` | Password authentication, session management |
| `auth_webauthn.rs` | WebAuthn/FIDO2 passkey authentication |
| `api_keys.rs` | API key management (if exists) |
| `pairing.rs` | Device pairing protocol |

### Channels

| Module | Purpose |
|--------|---------|
| `channel.rs` | Channel abstraction trait |
| `channel_store.rs` | Channel storage and lookup |
| `channel_events/` | Event dispatch, command handling, formatting |
| `channel_events/commands/` | Attachments, control handlers, dispatch, formatting, location, media, quick actions, session handlers |
| `channel_webhook_dedup.rs` | Webhook deduplication |
| `channel_webhook_middleware.rs` | Webhook middleware chain |
| `channel_webhook_rate_limit.rs` | Webhook rate limiting |
| `channel_agent_tools.rs` | Agent tools for channels |

### Chat & Agent

| Module | Purpose |
|--------|---------|
| `chat.rs` | Chat session management |
| `chat_error.rs` | Chat error types |
| `agent_persona.rs` | Agent persona configuration |
| `mcp_agent_tools.rs` | MCP agent tool definitions |
| `project_aware_tools.rs` | Project-aware tool execution |

### MCP (Model Context Protocol)

| Module | Purpose |
|--------|---------|
| `mcp_service.rs` | MCP server implementation |
| `mcp_health.rs` | MCP health checking |

### Infrastructure

| Module | Purpose |
|--------|---------|
| `broadcast.rs` | Event broadcasting to connected clients |
| `cron.rs` | Cron/scheduled task management |
| `logs.rs` | Logging infrastructure |
| `network_audit.rs` | Network audit logging |
| `update_check.rs` | Update checking |
| `updater.rs` | Self-update mechanism |

### Voice

| Module | Purpose |
|--------|---------|
| `voice.rs` | Voice I/O management |
| `voice_agent_tools.rs` | Voice-specific agent tools |
| `voice_persona.rs` | Voice persona configuration |
| `tts_phrases.rs` | Text-to-speech phrase management |

### Features (optional, feature-gated)

| Module | Feature | Purpose |
|--------|---------|---------|
| `local_llm_setup/` | `local-llm` | Local LLM management |
| `netbird/` | `netbird` | Netbird VPN integration |
| `mdns/` | `mdns` | mDNS discovery |
| `push/` | `push-notifications` | Push notifications |
| `tailscale/` | `tailscale` | Tailscale integration |
| `teams_agent_tools/` | `msteams` | MS Teams tools |
| `vault_lifecycle/` | `vault` | Secret vault lifecycle |

## lib.rs: Entry Point

**Source:** `crates/gateway/src/lib.rs` (91 lines)

### Module Declarations

The gateway declares 68 public modules covering all aspects of the server. Optional modules are feature-gated (`#[cfg(feature = "...")]`):

```rust
// crates/gateway/src/lib.rs
pub mod agent_persona;
pub mod auth;
pub mod auth_webauthn;
pub mod channel;
pub mod channel_events;
pub mod chat;
pub mod cron;
pub mod mcp_service;
pub mod session;
pub mod server;
// ... and 50+ more
```

### run_migrations()

```rust
// crates/gateway/src/lib.rs:84-90
pub async fn run_migrations(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .set_ignore_missing(true)
        .run(pool)
        .await?;
    Ok(())
}
```

Runs database migrations for:
- Auth tables (`auth_password`, `passkeys`, `api_keys`, `auth_sessions`)
- `env_variables`
- `message_log`
- `channels`

Should be called at application startup after other crate migrations (projects, sessions, cron).

### config_override_test_lock()

```rust
// crates/gateway/src/lib.rs:72-77
pub(crate) fn config_override_test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}
```

Test-only lock for config override isolation. Prevents test interference when multiple tests modify config concurrently.

## server/ Module: Startup and Initialization

### server/startup.rs

**Source:** `crates/gateway/src/server/startup.rs` (~300 lines)

Handles application startup sequence:

1. **OpenClaw detection** — Detects existing OpenClaw installations and spawns background import workers
2. **Channel initialization** — Sets up configured channels (Telegram, Signal, Discord, etc.)
3. **Memory initialization** — Initializes the memory/RAG system
4. **Code index initialization** — Sets up code indexing
5. **Content seeding** — Seeds default content (skills, templates, etc.)
6. **Workspace setup** — Configures the workspace directory

#### OpenClaw Background Import

```rust
// crates/gateway/src/server/startup.rs:52-120
fn spawn_openclaw_background_init(data_dir: PathBuf) {
    tokio::spawn(async move {
        let detection = tokio::task::spawn_blocking(detect_openclaw_with_startup_logs).await;
        // If detected, spawn file watcher for incremental session imports
        // Runs every 60 seconds to check for new OpenClaw sessions
    });
}
```

Spawns a background task that:
1. Detects OpenClaw installation at startup
2. If found, runs initial import of sessions, agents, config
3. Spawns file watcher for incremental imports every 60 seconds

### server/prepare_core.rs

**Source:** `crates/gateway/src/server/prepare_core.rs` (~1400 lines)

The core startup preparation logic. Handles:
- Config loading and validation
- Auth system initialization (password, passkeys, API keys)
- Database connection setup
- Service registration
- Background task spawning

This is the largest single file in the gateway crate and the heart of the startup sequence.

### server/init_channels.rs

**Source:** `crates/gateway/src/server/init_channels.rs` (~330 lines)

Initializes all configured channels:

```rust
// For each configured channel:
// 1. Create channel instance
// 2. Register with channel store
// 3. Start channel event listener
// 4. Register webhook endpoints (if applicable)
```

### server/init_memory.rs

**Source:** `crates/gateway/src/server/init_memory.rs` (~500 lines)

Initializes the memory/RAG system:
- Creates or connects to SQLite memory database
- Sets up FTS (Full-Text Search) indices
- Initializes vector embedding store
- Loads existing memory entries

### server/init_code_index.rs

**Source:** `crates/gateway/src/server/init_code_index.rs` (~120 lines)

Initializes the code indexing system:
- Scans workspace directories
- Builds code index
- Sets up file watchers for incremental updates

### server/seed_content.rs

**Source:** `crates/gateway/src/server/seed_content.rs` (~300 lines)

Seeds default content:
- Default skills
- Default templates
- Default agent personas
- Default voice phrases

## auth.rs: Authentication

**Source:** `crates/gateway/src/auth.rs`

### Password Authentication

- Password hashing (bcrypt/argon2)
- Session creation and validation
- Password reset flow

### Session Management

- Session tokens with expiry
- Session invalidation
- Concurrent session limits

## auth_webauthn.rs: Passkey Authentication

**Source:** `crates/gateway/src/auth_webauthn.rs`

### WebAuthn Registration

1. Client requests registration options
2. Server generates challenge
3. Client creates passkey
4. Server verifies and stores credential

### WebAuthn Authentication

1. Client requests authentication options
2. Server generates challenge
3. Client signs challenge with passkey
4. Server verifies signature

## channel_events/ Module: Event Dispatch

### channel_events/dispatch.rs

Dispatches incoming channel events to the appropriate handler:

```rust
// For each incoming event:
// 1. Parse event into canonical format
// 2. Route to appropriate command handler
// 3. Execute command
// 4. Format response
// 5. Send back through channel
```

### channel_events/commands/

| File | Purpose |
|------|---------|
| `attachments.rs` | Handle message attachments |
| `control_handlers.rs` | Handle control commands (start, stop, pause) |
| `dispatch.rs` | Command dispatch logic |
| `formatting.rs` | Message formatting |
| `location.rs` | Location data handling |
| `media.rs` | Media message handling |
| `quick_actions.rs` | Quick action commands |
| `session_handlers.rs` | Session management commands |

## mcp_service.rs: MCP Server

**Source:** `crates/gateway/src/mcp_service.rs`

Implements the Model Context Protocol:
- **stdio transport** — for local agent connections
- **HTTP/SSE transport** — for remote agent connections
- Tool listing and execution
- Resource access

## broadcast.rs: Event Broadcasting

Broadcasts events to connected WebSocket clients:
- Chat messages
- Agent status updates
- Tool execution progress
- Channel events

## Next

Continue to [05-agents-deep](05-agents-deep.md) for the agent loop internals.
