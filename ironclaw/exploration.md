---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/ironclaw
repository: https://github.com/nearai/ironclaw
explored_at: 2026-03-22
---

# IronClaw Exploration

## Project Overview

**IronClaw** is a secure personal AI assistant framework written in Rust that protects user data and expands its capabilities dynamically. It's a Rust reimplementation inspired by OpenClaw (TypeScript reference), with key architectural differences emphasizing security, performance, and extensibility.

### Core Philosophy

- **Your data stays yours** - All information is stored locally, encrypted, and never leaves user control
- **Transparency by design** - Open source, auditable, no hidden telemetry or data harvesting
- **Self-expanding capabilities** - Build new tools on the fly without waiting for vendor updates
- **Defense in depth** - Multiple security layers protect against prompt injection and data exfiltration

### Key Statistics

| Metric | Value |
|--------|-------|
| Version | 0.1.3 |
| Rust Edition | 2024 |
| Minimum Rust | 1.92 |
| Lines of Code | ~50,000+ |
| Main Dependencies | tokio, axum, wasmtime, rig-core, deadpool-postgres, libsql |
| License | MIT OR Apache-2.0 |

---

## Architecture Overview

### High-Level System Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                          Channels                              │
│  ┌──────┐  ┌──────┐   ┌─────────────┐  ┌─────────────┐         │
│  │ REPL │  │ HTTP │   │WASM Channels│  │ Web Gateway │         │
│  └──┬───┘  └──┬───┘   └──────┬──────┘  │ (SSE + WS)  │         │
│     │         │              │         └──────┬──────┘         │
│     └─────────┴──────────────┴────────────────┘                │
│                              │                                 │
│                    ┌─────────▼─────────┐                       │
│                    │    Agent Loop     │  Intent routing       │
│                    └────┬──────────┬───┘                       │
│                         │          │                           │
│              ┌──────────▼────┐  ┌──▼───────────────┐           │
│              │  Scheduler    │  │ Routines Engine  │           │
│              │(parallel jobs)│  │(cron, event, wh) │           │
│              └──────┬────────┘  └────────┬─────────┘           │
│                     │                    │                     │
│       ┌─────────────┼────────────────────┘                     │
│       │             │                                          │
│   ┌───▼─────┐  ┌────▼────────────────┐                         │
│   │ Local   │  │    Orchestrator     │                         │
│   │Workers  │  │  ┌───────────────┐  │                         │
│   │(in-proc)│  │  │ Docker Sandbox│  │                         │
│   └───┬─────┘  │  │   Containers  │  │                         │
│       │        │  │ ┌───────────┐ │  │                         │
│       │        │  │ │Worker / CC│ │  │                         │
│       │        │  │ └───────────┘ │  │                         │
│       │        │  └───────────────┘  │                         │
│       │        └─────────┬───────────┘                         │
│       └──────────────────┤                                     │
│                          │                                     │
│              ┌───────────▼──────────┐                          │
│              │    Tool Registry     │                          │
│              │  Built-in, MCP, WASM │                          │
│              └──────────────────────┘                          │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
                    ┌─────────────────┐
                    │   PostgreSQL    │
                    │   / libSQL      │
                    │   + pgvector    │
                    └─────────────────┘
```

### Security Architecture (Defense in Depth)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              WASM Tool Execution                             │
│                                                                              │
│   WASM Tool ──▶ Host Function ──▶ Allowlist ──▶ Credential ──▶ Execute     │
│   (untrusted)   (boundary)        Validator     Injector       Request      │
│                                                                    │        │
│                                                                    ▼        │
│                              ◀────── Leak Detector ◀────── Response        │
│                          (sanitized, no secrets)                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Security Layers:**

| Layer | Purpose | Implementation |
|-------|---------|----------------|
| WASM Sandbox | Untrusted code isolation | wasmtime with fuel metering, memory limits |
| Credential Injection | Secret protection | Injected at host boundary, never exposed to WASM |
| Leak Detection | Prevent exfiltration | Pattern scanning on all I/O |
| Allowlist Validator | Network control | Endpoint/path allowlisting |
| Rate Limiter | Abuse prevention | Per-tool rate limiting |
| Prompt Injection Defense | LLM protection | Pattern detection, sanitization, policy rules |

---

## Directory Structure

```
ironclaw/
├── src/                          # Main Rust source code
│   ├── lib.rs                    # Library root, module declarations
│   ├── main.rs                   # Entry point, CLI args, startup
│   ├── config.rs                 # Configuration from env vars (51KB)
│   ├── error.rs                  # Error types using thiserror
│   ├── bootstrap.rs              # Application bootstrap logic
│   ├── settings.rs               # User settings management
│   ├── tracing_fmt.rs            # Tracing/telemetry setup
│   ├── util.rs                   # Utility functions
│   │
│   ├── agent/                    # Core agent logic (16 files)
│   │   ├── agent_loop.rs         # Main Agent struct, message handling (101KB)
│   │   ├── router.rs             # MessageIntent classification
│   │   ├── scheduler.rs          # Parallel job scheduling
│   │   ├── worker.rs             # Per-job execution with LLM
│   │   ├── session.rs            # Session/thread/turn model
│   │   ├── session_manager.rs    # Thread/session lifecycle
│   │   ├── compaction.rs         # Context window management
│   │   ├── context_monitor.rs    # Memory pressure detection
│   │   ├── heartbeat.rs          # Proactive periodic execution
│   │   ├── routine.rs            # Routine types
│   │   ├── routine_engine.rs     # Routine execution engine
│   │   ├── self_repair.rs        # Stuck job detection/recovery
│   │   ├── undo.rs               # Turn-based undo/redo
│   │   ├── submission.rs         # Submission parsing
│   │   └── task.rs               # Sub-task execution framework
│   │
│   ├── channels/                 # Multi-channel input system
│   │   ├── channel.rs            # Channel trait definitions
│   │   ├── manager.rs            # ChannelManager for stream merging
│   │   ├── http.rs               # HTTP webhook channel
│   │   ├── repl.rs               # Simple REPL for testing
│   │   ├── webhook_server.rs     # Webhook server
│   │   ├── wasm/                 # WASM channel runtime
│   │   │   ├── mod.rs
│   │   │   ├── bundled.rs        # Bundled channel discovery
│   │   │   ├── capabilities.rs   # Channel capabilities
│   │   │   ├── host.rs           # Host functions for WASM
│   │   │   ├── loader.rs         # WASM loading
│   │   │   ├── router.rs         # WASM channel router
│   │   │   ├── runtime.rs        # WASM runtime
│   │   │   ├── schema.rs         # WASM schema
│   │   │   └── wrapper.rs        # Channel trait wrapper
│   │   └── web/                  # Web gateway (browser UI)
│   │       ├── mod.rs
│   │       ├── server.rs         # Axum router, 40+ endpoints
│   │       ├── sse.rs            # SSE broadcast manager
│   │       ├── ws.rs             # WebSocket gateway
│   │       ├── types.rs          # Request/response types
│   │       ├── auth.rs           # Bearer token auth
│   │       └── log_layer.rs      # Tracing layer for log streaming
│   │
│   ├── orchestrator/             # Internal HTTP API for sandbox
│   │   ├── mod.rs
│   │   ├── api.rs                # Axum endpoints
│   │   ├── auth.rs               # Per-job bearer token store
│   │   └── job_manager.rs        # Container lifecycle management
│   │
│   ├── worker/                   # Runs inside Docker containers
│   │   ├── mod.rs
│   │   ├── runtime.rs            # Worker execution loop
│   │   ├── claude_bridge.rs      # Claude Code bridge (26KB)
│   │   ├── api.rs                # HTTP client to orchestrator
│   │   └── proxy_llm.rs          # LLM proxy through orchestrator
│   │
│   ├── tools/                    # Extensible tool system
│   │   ├── mod.rs
│   │   ├── tool.rs               # Tool trait, ToolOutput, ToolError
│   │   ├── registry.rs           # ToolRegistry for discovery
│   │   ├── sandbox.rs            # Process-based sandbox (stub)
│   │   ├── builtin/              # Built-in Rust tools
│   │   │   ├── echo.rs, time.rs, json.rs, http.rs
│   │   │   ├── file.rs           # ReadFile, WriteFile, ApplyPatch
│   │   │   ├── shell.rs          # Shell command execution
│   │   │   ├── memory.rs         # Memory tools (search, write, read, tree)
│   │   │   ├── job.rs            # Job management tools
│   │   │   └── routine.rs        # Routine CRUD tools
│   │   ├── builder/              # Dynamic tool building
│   │   │   ├── core.rs
│   │   │   ├── templates.rs
│   │   │   ├── testing.rs
│   │   │   └── validation.rs
│   │   ├── mcp/                  # Model Context Protocol
│   │   │   ├── client.rs
│   │   │   └── protocol.rs
│   │   └── wasm/                 # Full WASM sandbox (wasmtime)
│   │       ├── mod.rs
│   │       ├── runtime.rs
│   │       ├── wrapper.rs
│   │       ├── host.rs
│   │       ├── limits.rs
│   │       ├── allowlist.rs
│   │       ├── credential_injector.rs
│   │       ├── loader.rs
│   │       ├── rate_limiter.rs
│   │       ├── storage.rs
│   │       ├── capabilities.rs
│   │       └── capabilities_schema.rs
│   │
│   ├── db/                       # Database abstraction layer
│   │   ├── mod.rs                # Database trait (~60 methods)
│   │   ├── postgres.rs           # PostgreSQL backend
│   │   ├── libsql_backend.rs     # libSQL/Turso backend
│   │   └── libsql_migrations.rs  # SQLite-dialect schema
│   │
│   ├── workspace/                # Persistent memory system
│   │   ├── mod.rs                # Workspace struct
│   │   ├── document.rs           # MemoryDocument, MemoryChunk
│   │   ├── chunker.rs            # Document chunking (800 tokens, 15% overlap)
│   │   ├── embeddings.rs         # EmbeddingProvider trait
│   │   ├── search.rs             # Hybrid search with RRF
│   │   └── repository.rs         # PostgreSQL CRUD operations
│   │
│   ├── context/                  # Job context isolation
│   │   ├── state.rs              # JobState enum, state machine
│   │   ├── memory.rs             # ActionRecord, ConversationMemory
│   │   └── manager.rs            # ContextManager for concurrent jobs
│   │
│   ├── llm/                      # LLM integration
│   │   ├── provider.rs           # LlmProvider trait
│   │   ├── nearai.rs             # NEAR AI implementation
│   │   ├── nearai_chat.rs        # NEAR AI chat-api
│   │   ├── failover.rs           # FailoverProvider
│   │   ├── reasoning.rs          # Planning, tool selection
│   │   ├── session.rs            # Session token management
│   │   └── rig_adapter.rs        # rig-core adapter
│   │
│   ├── safety/                   # Prompt injection defense
│   │   ├── mod.rs
│   │   ├── sanitizer.rs          # Pattern detection, escaping
│   │   ├── validator.rs          # Input validation
│   │   ├── policy.rs             # PolicyRule system
│   │   └── leak_detector.rs      # Secret detection
│   │
│   ├── secrets/                  # Secrets management
│   │   ├── mod.rs
│   │   ├── crypto.rs             # AES-256-GCM encryption
│   │   ├── keychain.rs           # macOS keychain, Linux secret-service
│   │   ├── store.rs              # Secret storage
│   │   └── types.rs              # Credential types
│   │
│   ├── estimation/               # Cost/time/value estimation
│   │   ├── cost.rs
│   │   ├── time.rs
│   │   ├── value.rs
│   │   └── learner.rs            # Exponential moving average
│   │
│   ├── evaluation/               # Success evaluation
│   │   ├── success.rs
│   │   └── metrics.rs
│   │
│   ├── extensions/               # Extension management
│   │   ├── mod.rs
│   │   ├── manager.rs
│   │   ├── registry.rs
│   │   └── discovery.rs
│   │
│   ├── history/                  # Persistence layer
│   │   ├── mod.rs
│   │   ├── store.rs
│   │   └── analytics.rs
│   │
│   ├── sandbox/                  # Sandbox infrastructure
│   ├── pairing/                  # Channel DM pairing
│   ├── setup/                    # Onboarding wizard
│   └── cli/                      # CLI subcommands
│       ├── mod.rs
│       ├── config.rs
│       ├── mcp.rs
│       ├── memory.rs
│       ├── oauth_defaults.rs
│       ├── pairing.rs
│       ├── status.rs
│       └── tool.rs
│
├── channels-src/                 # External channel sources (WASM)
│   ├── slack/
│   ├── telegram/
│   └── whatsapp/
│
├── tools-src/                    # External tool sources (WASM)
│   ├── gmail/
│   ├── google-calendar/
│   ├── google-docs/
│   ├── google-drive/
│   ├── google-sheets/
│   ├── google-slides/
│   ├── okta/
│   └── slack/
│
├── migrations/                   # PostgreSQL migrations
│   └── V1__initial.sql
│
├── wit/                          # WIT interface definitions
│
├── deploy/                       # Deployment configurations
├── docker/                       # Docker configurations
├── docs/                         # Documentation
├── examples/                     # Code examples
├── scripts/                      # Build/deployment scripts
└── tests/                        # Integration tests
```

---

## Key Components Deep Dive

### 1. Agent System (`src/agent/`)

The agent system is the brain of IronClaw, handling message routing, job scheduling, and LLM coordination.

#### Agent Loop (`agent_loop.rs` - 101KB)

The `Agent` struct is the main coordinator:

```rust
pub struct Agent {
    deps: AgentDeps,
    session_manager: SessionManager,
    // ... internal state
}

pub struct AgentDeps {
    llm: Arc<dyn LlmProvider>,
    tool_registry: Arc<ToolRegistry>,
    workspace: Workspace,
    safety: SafetyLayer,
    // ... other dependencies
}
```

**Key Responsibilities:**
- Message routing from channels
- Job scheduling via `Scheduler`
- Worker coordination for job execution
- Self-repair for stuck jobs
- Heartbeat for proactive execution
- Turn-based session management with undo support

#### Router (`router.rs`)

Classifies incoming messages into intents:

```rust
pub enum MessageIntent {
    Command,      // Direct commands like "/help"
    Query,        // Information-seeking questions
    Task,         // Action requiring tool use
    Conversation, // Casual chat
}
```

#### Scheduler (`scheduler.rs`)

Manages parallel job execution with priorities:

- Supports up to `MAX_PARALLEL_JOBS` concurrent jobs
- Priority-based scheduling
- Job state machine management

#### Worker (`worker.rs` - 30KB)

Executes individual jobs with LLM reasoning:

```rust
pub struct Worker {
    deps: WorkerDeps,
    context: JobContext,
}

impl Worker {
    async fn execute(&self, intent: MessageIntent) -> Result<JobOutput>;
}
```

**Execution Flow:**
1. Parse user intent
2. Retrieve relevant context from workspace
3. Call LLM with system prompt + context
4. Parse tool calls from LLM response
5. Execute tools (with safety checks)
6. Loop until completion or max iterations

#### Session Management (`session.rs`, `session_manager.rs`)

Turn-based session model with state machine:

```
ThreadState:
  Active → Compacting → Active
       → Completed
       → Failed
```

**Key Features:**
- Per-sender session isolation
- Context compaction when window exceeded
- Undo/redo with checkpoints
- Turn summarization for memory efficiency

#### Heartbeat System (`heartbeat.rs`)

Proactive periodic execution:

- Default: 30-minute intervals
- Reads `HEARTBEAT.md` checklist
- Runs agent turn with checklist prompt
- Notifies via channel if findings

#### Routines Engine (`routine.rs`, `routine_engine.rs`)

Scheduled and reactive background tasks:

```rust
pub enum Trigger {
    Cron(String),           // Cron expression
    Event(EventMatcher),    // Event-based trigger
    Webhook(WebhookConfig), // HTTP webhook
    Manual,                 // Manual trigger
}

pub struct Routine {
    name: String,
    trigger: Trigger,
    action: RoutineAction,
    guardrails: RoutineGuardrails,
}
```

### 2. Channel System (`src/channels/`)

Multi-channel input with unified message format.

#### Channel Trait

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    fn receive(&self) -> MessageStream;
    async fn send(&self, response: OutgoingResponse) -> Result<()>;
    async fn status(&self) -> ChannelStatus;
}
```

#### Channel Types

| Channel | Description | Status |
|---------|-------------|--------|
| `ReplChannel` | Simple REPL for testing | ✅ |
| `HttpChannel` | HTTP webhook with secret validation | ✅ |
| `GatewayChannel` | Web gateway with SSE/WebSocket | ✅ |
| `WasmChannel` | Dynamically loaded WASM channels | ✅ |
| `TelegramChannel` | WASM-based Telegram (MTProto) | ✅ |
| `SlackChannel` | WASM-based Slack tool | ✅ |

#### Web Gateway (`web/`)

Browser-based control plane with 40+ API endpoints:

**Endpoints:**
- `/api/chat` - Send messages
- `/api/jobs` - Job management
- `/api/memory` - Memory operations
- `/api/logs` - Real-time log streaming
- `/api/extensions` - Extension management
- `/api/routines` - Routine management
- `/api/health` - Health checks
- `/api/gateway/status` - Gateway status

**Real-time Features:**
- SSE (Server-Sent Events) for server→client streaming
- WebSocket for bidirectional communication
- Log streaming with level/target filters

### 3. Tool System (`src/tools/`)

Extensible tool architecture with multiple execution domains.

#### Tool Trait

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value, ctx: &JobContext)
        -> Result<ToolOutput, ToolError>;
    fn requires_sanitization(&self) -> bool { true }
    fn requires_approval(&self) -> bool { false }
    fn execution_timeout(&self) -> Duration { Duration::from_secs(60) }
    fn domain(&self) -> ToolDomain { ToolDomain::Orchestrator }
}
```

#### Tool Domains

```rust
pub enum ToolDomain {
    Orchestrator,  // In-process (memory, job mgmt)
    Container,     // Docker sandbox (shell, file ops)
}
```

#### Built-in Tools

| Tool | Purpose | Domain |
|------|---------|--------|
| `echo` | Testing | Orchestrator |
| `time` | Current time | Orchestrator |
| `http` | HTTP requests | Container |
| `file` | Read/Write/ApplyPatch | Container |
| `shell` | Shell commands | Container |
| `memory_search` | Hybrid memory search | Orchestrator |
| `memory_write` | Write to workspace | Orchestrator |
| `memory_read` | Read from workspace | Orchestrator |
| `memory_tree` | List directory structure | Orchestrator |
| `job_create` | Create sub-job | Orchestrator |
| `routine_*` | Routine CRUD | Orchestrator |

#### WASM Tools

Full WASM sandbox with capability-based security:

**Security Features:**
- Fuel metering (CPU limits)
- Memory limits (10MB default)
- Network allowlisting
- Credential injection at boundary
- Leak detection on I/O
- Rate limiting per tool

**Host Functions (V2 API):**
- `log` - Structured logging
- `time` - Time operations
- `workspace_read` / `workspace_write` - Filesystem access
- `http_request` - Network access (allowlisted)
- `tool_invoke` - Call other tools
- `secret_exists` - Check secret availability

### 4. Database System (`src/db/`)

Dual-backend database abstraction with ~60 async methods.

#### Database Trait

```rust
#[async_trait]
pub trait Database: Send + Sync {
    // Conversations
    async fn create_conversation(&self, ...) -> Result<Uuid>;
    async fn get_conversation(&self, ...) -> Result<Conversation>;
    // ... many more

    // Jobs
    async fn create_job(&self, ...) -> Result<Uuid>;
    async fn update_job_state(&self, ...) -> Result<()>;
    // ... many more

    // Workspace
    async fn get_document_by_path(&self, ...) -> Result<MemoryDocument>;
    async fn hybrid_search(&self, ...) -> Result<Vec<SearchResult>>;
    // ... many more
}
```

#### Backends

| Backend | Feature Flag | Use Case |
|---------|-------------|----------|
| PostgreSQL | `postgres` (default) | Production deployments |
| libSQL/Turso | `libsql` | Zero-dependency local mode |

**Schema Translation:**

| PostgreSQL | libSQL |
|------------|--------|
| `UUID` | `TEXT` |
| `TIMESTAMPTZ` | `TEXT` (ISO-8601) |
| `JSONB` | `TEXT` (JSON) |
| `VECTOR(1536)` | `F32_BLOB(1536)` |
| `tsvector` | FTS5 virtual table |

### 5. Workspace/Memory System (`src/workspace/`)

Persistent memory inspired by OpenClaw with hybrid search.

#### Filesystem-like API

```
workspace/
├── README.md              # Root runbook
├── MEMORY.md              # Long-term curated memory
├── HEARTBEAT.md           # Periodic checklist
├── IDENTITY.md            # Agent name, nature
├── SOUL.md                # Core values
├── AGENTS.md              # Behavior instructions
├── USER.md                # User context
├── context/               # Identity documents
├── daily/                 # Daily logs
└── projects/              # Arbitrary structure
```

#### Hybrid Search (RRF Algorithm)

Combines full-text search (BM25) with vector similarity:

```rust
score(d) = Σ 1/(k + rank(d)) for each method where d appears
```

**Default k=60** - Documents appearing in both results get boosted scores.

#### Chunking Strategy

- Default: 800 words per chunk (~800 tokens for English)
- 15% overlap between chunks for context preservation
- Minimum chunk size: 50 words

### 6. Safety System (`src/safety/`)

Multi-layer defense against prompt injection and data exfiltration.

#### Components

| Component | Purpose |
|-----------|---------|
| `Sanitizer` | Pattern detection, content escaping |
| `Validator` | Length, encoding, pattern validation |
| `Policy` | Rule-based severity/actions |
| `LeakDetector` | Secret exfiltration detection |

#### Policy System

```rust
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

pub enum PolicyAction {
    Block,
    Warn,
    Review,
    Sanitize,
}
```

#### Tool Output Wrapping

```xml
<tool_output name="search" sanitized="true">
[escaped content]
</tool_output>
```

### 7. Orchestrator/Worker System

Docker-based sandbox for untrusted operations.

#### Orchestrator (`src/orchestrator/`)

Internal HTTP API for container management:

- `/api/jobs/{id}/start` - Start job
- `/api/jobs/{id}/complete` - Complete job
- `/api/llm/chat` - LLM proxy
- `/api/events` - Event streaming

**Auth:** Per-job bearer tokens with automatic cleanup.

#### Worker (`src/worker/`)

Runs inside Docker containers:

```rust
pub struct WorkerRuntime {
    config: WorkerConfig,
    llm: Arc<dyn LlmProvider>,  // Proxies through orchestrator
}
```

**Claude Code Mode:** Bridge to spawn Claude CLI inside containers for complex tasks.

### 8. LLM Integration (`src/llm/`)

Multi-provider LLM support via NEAR AI.

#### Provider Types

| Provider | Status | Notes |
|----------|--------|-------|
| NEAR AI | ✅ | Primary provider |
| Anthropic (via NEAR AI) | 🚧 | Proxy access |
| OpenAI (via NEAR AI) | 🚧 | Proxy access |
| Failover | ✅ | Sequential retry on errors |

#### Session Management

```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

pub struct Session {
    token: String,
    expires_at: DateTime<Utc>,
    // ...
}
```

Auto-renewal when sessions expire.

### 9. Secrets Management (`src/secrets/`)

Secure credential storage with platform-specific backends.

#### Crypto (`crypto.rs`)

- AES-256-GCM encryption
- Blake3 for hashing
- HKDF for key derivation
- Constant-time comparisons

#### Platform Backends

| Platform | Backend |
|----------|---------|
| macOS | Security Framework (keychain) |
| Linux | Secret Service (GNOME Keyring, KWallet) |
| Windows | (TODO) |

### 10. Extensions System (`src/extensions/`)

Dynamic extension management for MCP and WASM extensions.

```rust
pub struct ExtensionManager {
    registry: ExtensionRegistry,
    wasm_runtime: Arc<WasmToolRuntime>,
    mcp_manager: Arc<McpSessionManager>,
}
```

**Commands:**
- `ironclaw tool install` - Install WASM/MCP extension
- `ironclaw tool auth` - Authenticate extension
- `ironclaw tool activate` - Activate extension
- `ironclaw tool remove` - Remove extension

---

## Entry Points and Execution Flow

### Main Entry Point (`src/main.rs`)

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Command::Tool(cmd)) => run_tool_command(cmd).await,
        Some(Command::Mcp(cmd)) => run_mcp_command(cmd).await,
        Some(Command::Worker { job_id, .. }) => {
            // Worker mode inside Docker container
            let runtime = WorkerRuntime::new(config)?;
            runtime.run().await?;
        }
        Some(Command::Onboard) => {
            // Interactive setup wizard
            let wizard = SetupWizard::new(config)?;
            wizard.run().await?;
        }
        None => {
            // Default: start agent with channels
            let (config, db, secrets) = bootstrap::bootstrap().await?;
            let agent = Agent::new(deps).await?;
            agent.run().await?;
        }
    }
}
```

### CLI Commands

```
ironclaw
├── (default)     # Start REPL with agent
├── onboard       # Interactive setup wizard
├── tool
│   ├── install   # Install WASM/MCP extension
│   ├── list      # List installed extensions
│   ├── auth      # Authenticate extension
│   ├── remove    # Remove extension
│   └── build     # Build WASM tool
├── mcp
│   ├── list      # List MCP servers
│   ├── add       # Add MCP server
│   └── remove    # Remove MCP server
├── pairing
│   ├── list      # List pending pairings
│   └── approve   # Approve pairing
├── status        # System status
├── memory        # Memory CLI
│   ├── search    # Search memory
│   └── index     # Rebuild index
├── config        # Configuration management
└── worker        # Worker mode (Docker)
```

### Bootstrap Flow (`bootstrap.rs`)

1. Load configuration from `.env` and `~/.ironclaw/settings.toml`
2. Connect to database (PostgreSQL or libSQL)
3. Initialize secrets store (platform-specific)
4. Run database migrations
5. Create tool registry with built-in tools
6. Load WASM tools from `~/.ironclaw/tools/`
7. Load MCP servers from database
8. Initialize workspace with embeddings
9. Create safety layer
10. Initialize channels
11. Spawn agent loop

---

## Dependencies Analysis

### Core Dependencies

| Category | Dependencies |
|----------|-------------|
| Async Runtime | `tokio` (full), `tokio-stream`, `futures` |
| HTTP | `reqwest`, `axum`, `tower`, `tower-http`, `hyper` |
| Serialization | `serde`, `serde_json` |
| Database | `deadpool-postgres`, `tokio-postgres`, `libsql`, `refinery`, `pgvector` |
| Error Handling | `thiserror`, `anyhow` |
| Logging | `tracing`, `tracing-subscriber` |
| WASM | `wasmtime`, `wasmtime-wasi`, `wasmparser` |
| CLI | `clap`, `crossterm`, `rustyline`, `termimad` |
| Crypto | `aes-gcm`, `hkdf`, `sha2`, `blake3`, `rand`, `subtle` |
| LLM | `rig-core` |
| Docker | `bollard` |
| Config | `dotenvy`, `uuid`, `chrono`, `rust_decimal` |

### Feature Flags

```toml
[features]
default = ["postgres", "libsql"]
postgres = [
    "dep:deadpool-postgres",
    "dep:tokio-postgres",
    "dep:postgres-types",
    "dep:refinery",
    "dep:pgvector",
]
libsql = ["dep:libsql"]
integration = []
```

---

## Configuration

### Environment Variables

```bash
# Database backend (default: postgres)
DATABASE_BACKEND=postgres               # or "libsql"
DATABASE_URL=postgres://user:pass@localhost/ironclaw
LIBSQL_PATH=~/.ironclaw/ironclaw.db

# NEAR AI (required)
NEARAI_SESSION_TOKEN=sess_...
NEARAI_MODEL=claude-3-5-sonnet-20241022
NEARAI_BASE_URL=https://private.near.ai

# Agent settings
AGENT_NAME=ironclaw
MAX_PARALLEL_JOBS=5

# Embeddings
OPENAI_API_KEY=sk-...
EMBEDDING_PROVIDER=nearai
EMBEDDING_MODEL=text-embedding-3-small

# Heartbeat
HEARTBEAT_ENABLED=true
HEARTBEAT_INTERVAL_SECS=1800

# Web gateway
GATEWAY_ENABLED=true
GATEWAY_HOST=127.0.0.1
GATEWAY_PORT=3001
GATEWAY_AUTH_TOKEN=changeme

# Docker sandbox
SANDBOX_ENABLED=true
SANDBOX_IMAGE=ironclaw-worker:latest
SANDBOX_MEMORY_LIMIT_MB=512
SANDBOX_TIMEOUT_SECS=1800

# Claude Code mode
CLAUDE_CODE_ENABLED=false
CLAUDE_CODE_MODEL=claude-sonnet-4-20250514
CLAUDE_CODE_MAX_TURNS=50
```

### Configuration File (`~/.ironclaw/settings.toml`)

Generated by `ironclaw onboard` wizard:
- Database connection
- NEAR AI authentication (OAuth)
- Secrets encryption key (system keychain)

---

## Testing Strategy

### Test Organization

Tests are in `mod tests {}` blocks at the bottom of each file:

```bash
cargo test safety::sanitizer::tests
cargo test tools::registry::tests
cargo test workspace::tests::test_normalize_path
```

### Test Patterns

- Unit tests for pure functions
- Async tests with `#[tokio::test]`
- No mocks, prefer real implementations or stubs
- Integration tests require `--features integration`

### Database Testing

```bash
# Create test database
createdb ironclaw_test

# Run tests
cargo test
```

---

## Build System

### Standard Build

```bash
# Format code
cargo fmt

# Lint
cargo clippy --all --benches --tests --examples --all-features

# Build
cargo build --release

# Run tests
cargo test
```

### Full Release (with channel rebuilds)

```bash
# Rebuild WASM channels first
./scripts/build-all.sh

# Then build main binary
cargo build --release
```

### Distribution

Uses `cargo-dist` for cross-platform releases:

```toml
[workspace.metadata.dist]
installers = ["shell", "powershell", "npm", "msi"]
targets = [
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
]
```

---

## OpenClaw Heritage

IronClaw is a Rust reimplementation inspired by OpenClaw with these key differences:

| Aspect | OpenClaw | IronClaw |
|--------|----------|----------|
| Language | TypeScript | Rust |
| Sandbox | Docker | WASM + Docker |
| Database | SQLite | PostgreSQL + libSQL |
| Mobile Apps | iOS/Android | N/A (out of scope) |
| Provider | Multiple direct | NEAR AI focused |
| Channels | Native | WASM-extensible |

See `FEATURE_PARITY.md` for detailed feature tracking.

---

## Current Limitations / TODOs

### P1 - High Priority

- ❌ WhatsApp channel
- ❌ Hooks system (beforeInbound, beforeToolCall)
- ❌ Configuration hot-reload

### P2 - Medium Priority

- ❌ Media handling (caption support only)
- ❌ Ollama/local model support
- ❌ Webhook trigger endpoint in web gateway

### P3 - Lower Priority

- ❌ Discord, Signal, Matrix channels
- ❌ TTS/audio features
- ❌ Video support
- ❌ Skills system

### Completed

- ✅ TUI with approval overlays
- ✅ HTTP webhook channel
- ✅ DM pairing
- ✅ WASM tool sandbox
- ✅ Workspace with hybrid search
- ✅ Heartbeat system
- ✅ Gateway control plane + WebSocket
- ✅ Web Control UI
- ✅ Telegram/Slack WASM channels
- ✅ Docker sandbox (orchestrator/worker)
- ✅ Routines (cron, event, webhook)
- ✅ libSQL/Turso backend

---

## Related Documents

- [`architecture-deep-dive.md`](./architecture-deep-dive.md) - Detailed architecture analysis
- [`production-grade.md`](./production-grade.md) - Production deployment considerations
- [`rust-revision.md`](./rust-revision.md) - Rust implementation details
