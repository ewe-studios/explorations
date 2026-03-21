---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/ironclaw
explored_at: 2026-03-22
---

# IronClaw Architecture Deep Dive

## System Architecture Overview

This document provides a detailed analysis of IronClaw's architectural patterns, component interactions, and design decisions.

---

## 1. Architectural Philosophy

### 1.1 Design Principles

**1. Defense in Depth**
Multiple overlapping security layers ensure that a failure in one layer doesn't compromise the system:
- WASM sandbox with fuel metering
- Credential injection at host boundary
- Network allowlisting
- Leak detection on all I/O
- Prompt injection defense

**2. Capability-Based Security**
Tools and channels declare their requirements in `.capabilities.json` files:
```json
{
  "http": {
    "allowlist": [
      { "host": "slack.com", "path_prefix": "/api/" }
    ],
    "credentials": {
      "slack_bot_token": {
        "secret_name": "slack_bot_token",
        "location": { "type": "bearer" }
      }
    }
  }
}
```

**3. Generic/Extensible Architectures**
Prefer traits and abstractions over hardcoded implementations:
- `Database` trait for backend abstraction
- `Channel` trait for input sources
- `Tool` trait for capabilities
- `LlmProvider` trait for model providers

**4. Persistence Over Volatility**
"Memory is database, not RAM" - all state is persisted:
- Conversations in PostgreSQL/libSQL
- Workspace documents with chunking
- Job history with action logs
- Tool failure tracking for self-repair

---

## 2. Component Architecture

### 2.1 Agent Loop Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                           Agent Loop                                 │
│                                                                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐              │
│  │   Channel   │───▶│   Router    │───▶│  Scheduler  │              │
│  │   Message   │    │  (Intent)   │    │  (Queue)    │              │
│  └─────────────┘    └─────────────┘    └──────┬──────┘              │
│                                               │                      │
│                                               ▼                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐              │
│  │   Channel   │◀───│   Worker    │◀───│   Session   │              │
│  │  Response   │    │  (Execute)  │    │  (Context)  │              │
│  └─────────────┘    └──────┬──────┘    └─────────────┘              │
│                            │                                         │
│              ┌─────────────┴─────────────┐                          │
│              ▼                           ▼                          │
│     ┌─────────────┐            ┌─────────────┐                      │
│     │ToolRegistry │            │  Workspace  │                      │
│     │ - Built-in  │            │ - Read/Write│                      │
│     │ - WASM      │            │ - Search    │                      │
│     │ - MCP       │            │ - Memory    │                      │
│     └─────────────┘            └─────────────┘                      │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    Cross-Cutting Concerns                    │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │    │
│  │  │  Safety  │  │  Self-   │  │Heartbeat │  │Routine   │     │    │
│  │  │  Layer   │  │  Repair  │  │ Monitor  │  │ Engine   │     │    │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

#### Message Flow

1. **Receive**: Channel receives message from external source
2. **Route**: Router classifies intent (Command/Query/Task/Conversation)
3. **Schedule**: Scheduler assigns to job queue with priority
4. **Session**: SessionManager creates/retrieves session context
5. **Execute**: Worker processes with LLM reasoning
6. **Tool Call**: ToolRegistry executes requested tools
7. **Sanitize**: SafetyLayer processes tool output
8. **Respond**: Channel sends response back

#### State Machine

```
JobState:
  Pending → InProgress → Completed → Submitted → Accepted
                      ↓
                    Failed
                      ↓
                    Stuck → InProgress (recovery)
                         ↓
                       Failed
```

### 2.2 Channel Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         ChannelManager                              │
│                                                                      │
│  ┌──────────────┐   ┌─────────────┐   ┌─────────────┐              │
│  │ ReplChannel  │   │HttpChannel  │   │WasmChannel  │   ...        │
│  │   (TUI)      │   │ (Webhook)   │   │  (Telegram) │              │
│  └──────┬───────┘   └──────┬──────┘   └──────┬──────┘              │
│         │                 │                 │                       │
│         └─────────────────┴─────────────────┘                       │
│                           │                                         │
│                    select_all (streams)                             │
│                           │                                         │
│                           ▼                                         │
│                   MessageStream                                     │
│                           │                                         │
│                           ▼                                         │
│                    Agent Loop                                       │
└─────────────────────────────────────────────────────────────────────┘
```

#### Channel Trait Contract

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    /// Human-readable name for logging/identification
    fn name(&self) -> &str;

    /// Receive messages as async stream
    fn receive(&self) -> MessageStream;

    /// Send response back through channel
    async fn send(&self, response: OutgoingResponse) -> Result<()>;

    /// Report channel health status
    async fn status(&self) -> ChannelStatus;

    /// Broadcast message to all connected users (optional)
    async fn broadcast(&self, message: &str) -> Result<()> {
        Err(ChannelError::NotImplemented)
    }
}
```

#### WASM Channel Runtime

```
┌─────────────────────────────────────────────────────────────────────┐
│                       WASM Channel Runtime                          │
│                                                                      │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐             │
│  │   WASM      │───▶│   Channel   │───▶│   Router    │             │
│  │   Module    │    │   Wrapper   │    │ (Dispatch)  │             │
│  └─────────────┘    └──────┬──────┘    └──────┬──────┘             │
│                            │                 │                      │
│                            ▼                 ▼                      │
│                   ┌────────────────────────────────┐                │
│                   │        Host Functions          │                │
│                   │  ┌──────┐ ┌──────┐ ┌────────┐ │                │
│                   │  │ HTTP │ │ Log  │ │Channel │ │                │
│                   │  │Proxy │ │      │ │ Events │ │                │
│                   │  └──────┘ └──────┘ └────────┘ │                │
│                   └────────────────────────────────┘                │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.3 Tool Execution Architecture

#### Tool Registry

```rust
pub struct ToolRegistry {
    builtin: HashMap<String, Arc<dyn Tool>>,
    wasm: HashMap<String, WasmToolWrapper>,
    mcp: HashMap<String, McpTool>,
    dynamic: HashMap<String, Arc<dyn Tool>>,
}
```

#### Execution Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Tool Execution                               │
│                                                                      │
│   LLM Request                                                        │
│       │                                                              │
│       ▼                                                              │
│  ┌─────────┐     ┌─────────────┐     ┌─────────────┐               │
│  │  Parse  │────▶│  Domain     │────▶│  Executor   │               │
│  │  Tool   │     │  Check      │     │  Selection  │               │
│  │  Call   │     │             │     │             │               │
│  └─────────┘     └──────┬──────┘     └──────┬──────┘               │
│                         │                   │                       │
│         ┌───────────────┴────────┐          │                       │
│         │                        │          │                       │
│         ▼                        ▼          ▼                       │
│  ┌─────────────┐         ┌─────────────┐ ┌────────┐                │
│  │Orchestrator │         │  Docker     │ │  WASM  │                │
│  │  (in-proc)  │         │  Container  │ │Sandbox │                │
│  │             │         │             │ │        │                │
│  │ - Memory    │         │ - Shell     │ │- Plugin│                │
│  │ - Job mgmt  │         │ - File ops  │ │- Tool  │                │
│  │ - Safe ops  │         │ - Code exec │ │        │                │
│  └──────┬──────┘         └──────┬──────┘ └───┬────┘                │
│         │                      │             │                      │
│         └──────────────────────┴─────────────┘                      │
│                                │                                     │
│                                ▼                                     │
│                       ┌─────────────┐                               │
│                       │   Safety    │                               │
│                       │   Layer     │                               │
│                       │             │                               │
│                       │ - Sanitize  │                               │
│                       │ - Validate  │                               │
│                       │ - Leak Scan │                               │
│                       └──────┬──────┘                               │
│                              │                                       │
│                              ▼                                       │
│                        LLM Context                                   │
└─────────────────────────────────────────────────────────────────────┘
```

#### WASM Tool Security Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                    WASM Tool Security Stack                         │
│                                                                      │
│  Layer 1: Compilation                                                │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  • BLAKE3 hash verification                                 │   │
│  │  • WIT interface validation                                 │   │
│  │  • Capabilities schema parsing                              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  Layer 2: Runtime Limits                                             │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  • Fuel metering (CPU)                                      │   │
│  │  • Memory limits (10MB default)                             │   │
│  │  • Epoch interruption (timeout)                             │   │
│  │  • Fresh instance per execution                             │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  Layer 3: Host Function Controls                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  • Network allowlist validation                             │   │
│  │  • Credential injection at boundary                         │   │
│  │  • Rate limiting per tool                                   │   │
│  │  • Path traversal prevention                                │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  Layer 4: I/O Scanning                                               │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  • Request leak detection                                   │   │
│  │  • Response leak detection                                  │   │
│  │  • Log sanitization                                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.4 Database Architecture

#### Dual-Backend Abstraction

```
┌─────────────────────────────────────────────────────────────────────┐
│                       Database Abstraction                          │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                      Database Trait                         │   │
│  │  (~60 async methods for all persistence operations)         │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │                                       │
│           ┌────────────────┴────────────────┐                     │
│           │                                 │                      │
│           ▼                                 ▼                      │
│  ┌─────────────────┐              ┌─────────────────┐             │
│  │   PostgreSQL    │              │     libSQL      │             │
│  │    Backend      │              │     Backend     │             │
│  │                 │              │                 │             │
│  │ • Connection    │              │ • Connection    │             │
│  │   Pool          │              │   (per-op)      │             │
│  │ • Store +       │              │ • Native SQL    │             │
│  │   Repository    │              │ • SQLite        │             │
│  │ • pgvector      │              │   dialect       │             │
│  │ • refinery      │              │ • FTS5          │             │
│  │   migrations    │              │ • Vector via    │             │
│  │                 │              │   libsql_       │             │
│  │                 │              │   vector_idx    │             │
│  └─────────────────┘              └─────────────────┘             │
│                                                                      │
│  Type Translation:                                                   │
│  ┌────────────────────────┬──────────────────────────────┐         │
│  │  PostgreSQL            │  libSQL                      │         │
│  ├────────────────────────┼──────────────────────────────┤         │
│  │  UUID                  │  TEXT                        │         │
│  │  TIMESTAMPTZ           │  TEXT (ISO-8601)             │         │
│  │  JSONB                 │  TEXT (JSON)                 │         │
│  │  VECTOR(1536)          │  F32_BLOB(1536)              │         │
│  │  tsvector              │  FTS5 virtual table          │         │
│  └────────────────────────┴──────────────────────────────┘         │
└─────────────────────────────────────────────────────────────────────┘
```

#### Repository Pattern (PostgreSQL)

```rust
pub struct Repository {
    pool: Pool,  // deadpool-postgres
}

impl Repository {
    pub async fn get_document_by_path(
        &self,
        user_id: &str,
        agent_id: Option<Uuid>,
        path: &str,
    ) -> Result<MemoryDocument, WorkspaceError>;

    pub async fn hybrid_search(
        &self,
        user_id: &str,
        agent_id: Option<Uuid>,
        query: &str,
        embedding: Option<&[f32]>,
        config: &SearchConfig,
    ) -> Result<Vec<SearchResult>, WorkspaceError>;
}
```

#### Schema Overview

**Core Tables (351 lines PostgreSQL, ~480 lines libSQL):**

| Table | Purpose |
|-------|---------|
| `conversations` | Multi-channel conversation tracking |
| `agent_jobs` | Job metadata and status |
| `job_actions` | Event-sourced tool executions |
| `dynamic_tools` | Agent-built tools |
| `llm_calls` | Cost tracking |
| `estimation_snapshots` | Learning data |
| `memory_documents` | Flexible path-based files |
| `memory_chunks` | Chunked content with indexes |
| `routines`, `routine_runs` | Scheduled execution |
| `settings` | Per-user key-value |
| `secrets`, `wasm_tools` | Extension infrastructure |

### 2.5 Workspace Architecture

#### Document Model

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Workspace Document Model                        │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    MemoryDocument                           │   │
│  │  - id: Uuid                                                 │   │
│  │  - user_id: String                                          │   │
│  │  - agent_id: Option<Uuid>                                   │   │
│  │  - path: String  (e.g., "projects/alpha/notes.md")          │   │
│  │  - content: String                                          │   │
│  │  - created_at, updated_at: DateTime                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                              │                                       │
│                              │ 1:N                                   │
│                              ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     MemoryChunk                             │   │
│  │  - id: Uuid                                                 │   │
│  │  - document_id: Uuid                                        │   │
│  │  - chunk_index: i32                                         │   │
│  │  - content: String                                          │   │
│  │  - embedding: Option<Vec<f32]>  (1536 dim)                  │   │
│  │  - fts: String  (for full-text search)                      │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

#### Chunking Algorithm

```rust
pub fn chunk_document(content: &str, config: ChunkConfig) -> Vec<String> {
    // Default: 800 words, 15% overlap
    let words: Vec<&str> = content.split_whitespace().collect();
    let step_size = (config.chunk_size as f32 * (1.0 - config.overlap)) as usize;

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < words.len() {
        let end = (start + config.chunk_size).min(words.len());
        let chunk = words[start..end].join(" ");
        chunks.push(chunk);
        start += step_size;

        if end == words.len() {
            break;
        }
    }

    chunks
}
```

#### Hybrid Search (RRF)

```rust
pub fn reciprocal_rank_fusion(
    fts_results: Vec<RankedResult>,
    vector_results: Vec<RankedResult>,
    k: usize,
) -> Vec<SearchResult> {
    let mut scores: HashMap<Uuid, f64> = HashMap::new();

    // Score from FTS
    for (rank, result) in fts_results.iter().enumerate() {
        *scores.entry(result.id).or_insert(0.0) += 1.0 / (k + rank + 1) as f64;
    }

    // Score from Vector
    for (rank, result) in vector_results.iter().enumerate() {
        *scores.entry(result.id).or_insert(0.0) += 1.0 / (k + rank + 1) as f64;
    }

    // Sort by combined score
    let mut results: Vec<_> = scores.into_iter().collect();
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    results
        .into_iter()
        .map(|(id, score)| SearchResult { id, score })
        .collect()
}
```

### 2.6 Safety Layer Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Safety Layer Pipeline                          │
│                                                                      │
│  Input: Tool Output String                                          │
│       │                                                              │
│       ▼                                                              │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  1. Length Check                                             │   │
│  │     • Truncate if > max_output_length                        │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │                                       │
│                            ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  2. Leak Detection & Redaction                               │   │
│  │     • Pattern matching for secrets                           │   │
│  │     • Redact or block                                        │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │                                       │
│                            ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  3. Policy Enforcement                                       │   │
│  │     • Check against PolicyRule list                          │   │
│  │     • Actions: Block, Warn, Review, Sanitize                 │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │                                       │
│                            ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  4. Sanitization (if enabled or policy requires)             │   │
│  │     • Pattern-based injection detection                      │   │
│  │     • Content escaping                                       │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │                                       │
│                            ▼                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  5. Wrap for LLM                                             │   │
│  │     <tool_output name="x" sanitized="true">...</tool_output> │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                            │                                       │
│                            ▼                                       │
│  Output: SanitizedOutput                                           │
└─────────────────────────────────────────────────────────────────────┘
```

#### Leak Detection Patterns

```rust
pub struct LeakPattern {
    pub name: &'static str,
    pub pattern: Regex,
    pub severity: LeakSeverity,
    pub action: LeakAction,
}

// Built-in patterns:
// - API keys (sk-, ghp_, etc.)
// - Bearer tokens
// - Password patterns
// - Private keys (-----BEGIN)
// - AWS credentials (AKIA...)
```

### 2.7 Orchestrator/Worker Architecture

#### Container Lifecycle

```
┌─────────────────────────────────────────────────────────────────────┐
│                   Orchestrator/Worker Flow                          │
│                                                                      │
│  ┌─────────────┐                                                    │
│  │  Job Queue  │                                                    │
│  └──────┬──────┘                                                    │
│         │                                                          │
│         ▼                                                          │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   Orchestrator                              │   │
│  │                                                             │   │
│  │  1. Create Container                                        │   │
│  │     • Generate per-job token                                │   │
│  │     • Mount workspace volume                                │   │
│  │     • Set environment variables                             │   │
│  │                                                             │   │
│  │  2. Start Container with Worker                             │   │
│  │     • ironclaw worker --job-id <uuid>                       │   │
│  │     • IRONCLAW_WORKER_TOKEN=<token>                         │   │
│  │                                                             │   │
│  │  3. Monitor Container                                       │   │
│  │     • Health checks                                         │   │
│  │     • Log streaming                                         │   │
│  │     • Timeout handling                                      │   │
│  │                                                             │   │
│  │  4. Cleanup                                                 │   │
│  │     • Stop container                                        │   │
│  │     • Revoke token                                          │   │
│  │     • Archive logs                                          │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   Worker (in container)                     │   │
│  │                                                             │   │
│  │  1. Connect to Orchestrator                                 │   │
│  │     • Authenticate with token                               │   │
│  │                                                             │   │
│  │  2. Execute Job Loop                                        │   │
│  │     • Request LLM completion (via proxy)                    │   │
│  │     • Execute tools locally                                 │   │
│  │     • Report progress                                       │   │
│  │                                                             │   │
│  │  3. Claude Code Mode (optional)                             │   │
│  │     • Spawn claude CLI inside container                     │   │
│  │     • Bridge tool calls                                     │   │
│  │     • Stream output                                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

#### Per-Job Auth

```rust
pub struct TokenStore {
    tokens: Arc<RwLock<HashMap<String, JobToken>>>,
}

pub struct JobToken {
    job_id: Uuid,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    permissions: Vec<Permission>,
}

impl TokenStore {
    pub fn create(&self, job_id: Uuid) -> String;
    pub fn validate(&self, token: &str) -> Option<&JobToken>;
    pub fn revoke(&self, token: &str);
}
```

---

## 3. Concurrency Model

### 3.1 Async Runtime

Built on tokio with full async/await:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Bootstrap
    let (config, db, secrets) = bootstrap::bootstrap().await?;

    // Spawn channels as concurrent tasks
    let channels = vec![
        tokio::spawn(repl_channel.run()),
        tokio::spawn(http_channel.run()),
        tokio::spawn(gateway_channel.run()),
    ];

    // Agent loop runs in main task
    agent.run().await?;

    Ok(())
}
```

### 3.2 Shared State

```rust
pub struct AgentDeps {
    llm: Arc<dyn LlmProvider>,
    tool_registry: Arc<ToolRegistry>,
    workspace: Workspace,
    safety: Arc<SafetyLayer>,
    db: Arc<dyn Database>,
    secrets: Arc<SecretsStore>,
    // ...
}
```

All shared state uses `Arc<T>` with interior mutability where needed:
- `Arc<RwLock<T>>` for read-heavy maps (sessions, tokens)
- `Arc<Mutex<T>>` for exclusive access (job queue)
- `Arc<AtomicUsize>` for counters

### 3.3 Channel Communication

```rust
// Response channel from agent to channels
let (response_tx, mut response_rx) = tokio::sync::mpsc::channel(100);

// Spawn agent loop
tokio::spawn(async move {
    while let Some(response) = response_rx.recv().await {
        channel_manager.send(response).await?;
    }
});
```

---

## 4. Error Handling Patterns

### 4.1 Error Type Hierarchy

```rust
pub enum Error {
    Config(#[from] ConfigError),
    Database(#[from] DatabaseError),
    Channel(#[from] ChannelError),
    Llm(#[from] LlmError),
    Tool(#[from] ToolError),
    Safety(#[from] SafetyError),
    Job(#[from] JobError),
    // ...
}
```

### 4.2 Error Context

```rust
// Map errors with context
.some_operation()
    .map_err(|e| DatabaseError::Query {
        reason: e.to_string()
    })?;

// Use anyhow for application-level errors
anyhow::bail!("Worker init failed: {}", e);
```

### 4.3 No Panic Policy

```rust
// Production code: NO unwrap() or expect()
let value = result.map_err(|e| Error::from(e))?;

// Tests: unwrap() is acceptable
#[test]
fn test_something() {
    let result = some_function().unwrap();
}
```

---

## 5. Extension Points

### 5.1 Adding New Tools

**Built-in (Rust):**
1. Create `src/tools/builtin/my_tool.rs`
2. Implement `Tool` trait
3. Register in `ToolRegistry::register_builtin_tools()`

**WASM:**
1. Create crate in `tools-src/<name>/`
2. Implement WIT interface
3. Create `<name>.capabilities.json`
4. Build with `cargo component build --release`
5. Install with `ironclaw tool install`

### 5.2 Adding New Channels

1. Implement `Channel` trait
2. Add to `ChannelManager`
3. Wire up in `main.rs`

### 5.3 Adding New Database Backends

1. Implement `Database` trait (~60 methods)
2. Add feature flag
3. Handle type translations

---

## 6. Performance Considerations

### 6.1 Database Connection Strategy

**PostgreSQL:** Connection pool (deadpool)
```rust
let pool = Pool::builder()
    .max_size(config.pool_size)
    .build(client)?;
```

**libSQL:** Connection per operation
```rust
// libSQL Connection is Send + cheap to clone
let conn = db.connect()?;
conn.execute(...).await?;
```

### 6.2 Caching Strategy

- WASM modules: Compile once, cache `Module`, instantiate fresh
- Embeddings: Cached in database chunks
- Tool schemas: Cached in registry

### 6.3 Memory Management

- WASM: 10MB default limit per tool
- Job contexts: Isolated, dropped after completion
- Workspace documents: Chunked for search (800 words each)

---

## 7. Security Deep Dive

### 7.1 Credential Injection

```rust
// Credentials NEVER exposed to WASM code
pub struct CredentialInjector {
    secrets: Arc<SecretsStore>,
}

impl CredentialInjector {
    pub async fn inject(
        &self,
        request: &mut Request,
        credentials: &Vec<CredentialConfig>,
    ) -> Result<()> {
        for cred in credentials {
            let secret = self.secrets.get(&cred.secret_name).await?;
            match cred.location {
                Location::Bearer => {
                    request.headers.insert(
                        AUTHORIZATION,
                        format!("Bearer {}", secret).parse().unwrap(),
                    );
                }
                Location::Header { name } => {
                    request.headers.insert(
                        HeaderName::from_str(&name)?,
                        secret.parse().unwrap(),
                    );
                }
                // ...
            }
        }
        Ok(())
    }
}
```

### 7.2 Prompt Injection Defense

```rust
// Wrap all tool outputs
pub fn wrap_for_llm(&self, tool_name: &str, content: &str, sanitized: bool) -> String {
    format!(
        "<tool_output name=\"{}\" sanitized=\"{}\">\n{}\n</tool_output>",
        escape_xml_attr(tool_name),
        sanitized,
        escape_xml_content(content)
    )
}

// Injection pattern detection
const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous instructions",
    "you are now",
    "system prompt",
    "new instructions",
    // ...
];
```

---

## 8. Testing Architecture

### 8.1 Test Organization

Tests in `mod tests {}` blocks at end of each file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Unit test
    }

    #[tokio::test]
    async fn test_async_something() {
        // Async test
    }
}
```

### 8.2 Integration Testing

```bash
# Requires test database
createdb ironclaw_test
cargo test --features integration
```

---

## 9. Build System

### 9.1 Feature Flags

```bash
# Default (both backends)
cargo build

# PostgreSQL only
cargo build --no-default-features --features postgres

# libSQL only
cargo build --no-default-features --features libsql
```

### 9.2 Distribution

Uses `cargo-dist` for cross-platform releases with installers:
- Shell script (Unix)
- PowerShell (Windows)
- NPM package
- MSI (Windows)

---

## Related Documents

- [`exploration.md`](./exploration.md) - Main exploration overview
- [`production-grade.md`](./production-grade.md) - Production deployment
- [`rust-revision.md`](./rust-revision.md) - Rust implementation details
