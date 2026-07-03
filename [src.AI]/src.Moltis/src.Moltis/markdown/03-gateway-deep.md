# Gateway Deep Dive: Startup, Services, and State

The gateway crate is the server's core — it loads configuration, runs migrations, wires all services, and returns a `PreparedGatewayCore` that can be consumed by HTTP or non-HTTP consumers.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/gateway/src/`

## prepare_gateway_core()

**Source:** `server/prepare_core.rs:47+`

```rust
pub async fn prepare_gateway_core(
    bind: &str,
    port: u16,
    no_tls: bool,
    log_buffer: Option<LogBuffer>,
    config_dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    tailscale_mode_override: Option<String>,
    tailscale_reset_on_exit_override: Option<bool>,
    session_event_bus: Option<SessionEventBus>,
) -> anyhow::Result<PreparedGatewayCore>
```

### Startup Sequence

1. **Directory overrides** (lines 63-68):
   ```rust
   if let Some(dir) = config_dir { moltis_config::set_config_dir(dir); }
   if let Some(dir) = data_dir { moltis_config::set_data_dir(dir.clone()); }
   ```

2. **Auth resolution** (lines 71-76):
   ```rust
   let token = std::env::var("MOLTIS_TOKEN").ok();
   let password = std::env::var("MOLTIS_PASSWORD").ok();
   let resolved_auth = auth::resolve_auth(token, password.clone());
   ```

3. **Config loading** (line 81):
   ```rust
   let mut config = moltis_config::discover_and_load();
   ```
   Loads from `moltis.toml`, `.yaml`, or `.json`.

4. **Memory probe** (lines 90-91):
   ```rust
   let mut startup_mem_probe = StartupMemProbe::new();
   startup_mem_probe.checkpoint("prepare_gateway.start");
   ```

5. **TLS configuration** (lines 94-100):
   - `--no-tls` flag overrides config
   - `MOLTIS_BEHIND_PROXY` env flag
   - `MOLTIS_ALLOW_TLS_BEHIND_PROXY` env flag

### Service Initialization

After `prepare_gateway_core()` completes, the following services are wired:

| Service | Source | Purpose |
|---------|--------|---------|
| `LiveModelService` | `chat.rs` | Model/chat management |
| `LiveSessionService` | `session.rs` | Session lifecycle |
| `LiveProviderSetupService` | `provider_setup.rs` | LLM provider setup |
| `LiveExecApprovalService` | `approval.rs` | Execution approval workflow |
| `Broadcast` | `broadcast.rs` | Event broadcasting |
| `ProviderRegistry` | `moltis_providers` | LLM provider registry |
| `ProjectStore` | `moltis_projects` | Project management |
| `SessionStore` | `moltis_sessions` | Session storage |
| `SessionEventBus` | `session_events` | Session event bus |

### Channel Initialization

**Source:** `server/init_channels.rs`

Initializes all configured channels from `config.channels.offered`:

```rust
info!(offered_channels = ?config.channels.offered, "loaded offered channels");
```

Each channel is initialized with its specific configuration (Telegram bot token, Discord bot token, etc.).

### Memory Initialization

**Source:** `server/init_memory.rs`

Initializes the memory/RAG system with SQLite + FTS + vector search.

### Code Index Initialization

**Source:** `server/init_code_index.rs`

Initializes the code indexing system for repository analysis.

## GatewayState

**Source:** `state.rs`

The `GatewayState` holds all core services:

```rust
pub struct GatewayState {
    pub config: GatewayConfig,
    pub services: GatewayServices,
    pub broadcast: BroadcastHandle,
    pub session_event_bus: SessionEventBus,
    // ...
}
```

## PreparedGatewayCore

**Source:** `server/prepared.rs`

```rust
pub struct PreparedGatewayCore {
    pub state: Arc<GatewayState>,
    // HTTP router is NOT yet attached — this is transport-agnostic
}
```

HTTP consumers call `prepare_gateway()` which delegates to `prepare_gateway_core()` and then adds the router + middleware.

## Auth System

**Source:** `auth.rs`

### Auth Resolution

```rust
// auth.rs
pub fn resolve_auth(token: Option<String>, password: Option<String>) -> ResolvedAuth {
    // MOLTIS_TOKEN takes precedence over MOLTIS_PASSWORD
    // Returns token-based or password-based auth
}
```

### WebAuthn

**Source:** `auth_webauthn.rs`

WebAuthn/FIDO2 passkey authentication:
- Registration flow
- Authentication flow
- Credential management

## Broadcast

**Source:** `broadcast.rs`

Event broadcasting to connected WebSocket clients:

```rust
pub struct BroadcastOpts {
    pub channel_capacity: usize,
}

pub fn broadcast(opts: BroadcastOpts) -> BroadcastHandle;
```

## Channel Events

**Source:** `channel_events/`

### Dispatch

**Source:** `channel_events/dispatch.rs`

Dispatches incoming channel events to the appropriate handler based on channel type.

### Commands

**Source:** `channel_events/commands/`

| File | Purpose |
|------|---------|
| `mod.rs` | Command module entry |
| `dispatch.rs` | Command dispatch |
| `formatting.rs` | Message formatting |
| `attachments.rs` | Attachment handling |
| `media.rs` | Media handling |
| `location.rs` | Location data |
| `control_handlers.rs` | Control commands |
| `session_handlers.rs` | Session commands |
| `quick_actions.rs` | Quick actions |

## Webhook Middleware

**Source:** `channel_webhook_*.rs`

| File | Purpose |
|------|---------|
| `channel_webhook_dedup.rs` | Deduplicate webhook events |
| `channel_webhook_middleware.rs` | Webhook middleware chain |
| `channel_webhook_rate_limit.rs` | Rate limiting |

## MCP Service

**Source:** `mcp_service.rs`

Model Context Protocol server:
- stdio transport
- HTTP/SSE transport
- Tool listing and execution

## Network Audit

**Source:** `network_audit.rs`

Network audit logging for security.

## Logs

**Source:** `logs.rs`

Logging infrastructure with buffer support for TUI display.

## Next

Continue to [06-providers-deep](06-providers-deep.md) for the LLM provider implementations.
