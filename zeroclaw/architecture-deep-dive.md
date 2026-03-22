# ZeroClaw Architecture Deep Dive

**Document Type:** Architecture Reference
**Last Updated:** 2026-03-22
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/zeroclaw`

---

## Table of Contents

1. [Architectural Principles](#architectural-principles)
2. [System Overview](#system-overview)
3. [Core Traits](#core-traits)
4. [Module Dependencies](#module-dependencies)
5. [Data Flow](#data-flow)
6. [Concurrency Model](#concurrency-model)
7. [Error Handling Strategy](#error-handling-strategy)
8. [Configuration System](#configuration-system)
9. [Extension Points](#extension-points)
10. [Security Boundaries](#security-boundaries)

---

## Architectural Principles

### 1. Trait-Driven Design

Every subsystem is defined as a trait, enabling **implementation swappability without code changes**:

```rust
// Any memory backend implements the same trait
#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;
    async fn store(&self, ...) -> Result<()>;
    async fn recall(&self, ...) -> Result<Vec<MemoryEntry>>;
    // ...
}

// Usage is identical regardless of backend
let memory: Arc<dyn Memory> = Arc::new(SqliteMemory::new(...)?);
// or
let memory: Arc<dyn Memory> = Arc::new(PostgresMemory::new(...)?);
```

### 2. Factory Pattern for Registration

All implementations are registered in factory modules:

```rust
// src/memory/mod.rs
pub fn create_memory(
    config: &MemoryConfig,
    workspace_dir: &Path,
    api_key: Option<&str>,
) -> Result<Box<dyn Memory>> {
    match config.backend.as_str() {
        "sqlite" => Ok(Box::new(SqliteMemory::new(...)?)),
        "postgres" => Ok(Box::new(PostgresMemory::new(...)?)),
        "markdown" => Ok(Box::new(MarkdownMemory::new(...)?)),
        "none" => Ok(Box::new(NoopMemory)),
        _ => bail!("Unknown memory backend: {}", config.backend),
    }
}
```

### 3. Dependency Direction Inward

Concrete implementations depend on traits, not on other implementations:

```
traits.rs (core contract)
    ↑
sqlite.rs | postgres.rs | markdown.rs (implementations)
    ↑
mod.rs (factory)
    ↑
agent.rs (orchestration - uses factory)
```

### 4. Interface Segregation

Traits are **narrow and focused** - each handles one concern:

| Trait | Responsibility |
|-------|---------------|
| `Memory` | Persistence and retrieval |
| `Tool` | Single capability execution |
| `Channel` | Messaging transport |
| `Provider` | LLM communication |
| `Observer` | Event/metric recording |
| `RuntimeAdapter` | Platform abstraction |
| `Peripheral` | Hardware board interface |

### 5. Fail Fast, Explicit Errors

No silent fallbacks - unsupported states error explicitly:

```rust
if unsupported_runtime_kind(config.runtime.kind) {
    bail!("Unsupported runtime kind: {}. Supported: native, docker",
          config.runtime.kind);
}
```

---

## System Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI / Gateway                            │
│  (main.rs - clap commands, axum HTTP server)                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Agent Core                               │
│  (agent.rs - orchestration loop, tool dispatcher)                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │  Provider   │  │    Tools    │  │   Memory    │              │
│  │   (trait)   │  │   (trait)   │  │   (trait)   │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌───────────────┐   ┌─────────────────┐   ┌───────────────┐
│   Channels    │   │    Runtime      │   │ Observability │
│   (trait)     │   │   (trait)       │   │   (trait)     │
└───────────────┘   └─────────────────┘   └───────────────┘
        │                     │                     │
        ▼                     ▼                     ▼
  Telegram, Discord     Native, Docker       Prometheus, OTel
  Slack, WhatsApp,
  Email, IRC, etc.
```

### Module Graph

```
main.rs
  ├── lib.rs (exports)
  │
  ├── config/
  │   ├── schema.rs        # Config structs
  │   └── mod.rs           # Loading, merging
  │
  ├── agent/
  │   ├── agent.rs         # Core agent
  │   ├── dispatcher.rs    # Tool dispatch (native/XML)
  │   ├── memory_loader.rs # Context loading
  │   ├── prompt.rs        # System prompt builder
  │   └── classifier.rs    # Query classification
  │
  ├── providers/
  │   ├── traits.rs        # Provider trait
  │   ├── mod.rs           # Factory
  │   ├── openai.rs
  │   ├── anthropic.rs
  │   ├── ollama.rs
  │   └── ... (28+ providers)
  │
  ├── tools/
  │   ├── traits.rs        # Tool trait
  │   ├── mod.rs           # Factory (all_tools_with_runtime)
  │   ├── shell.rs
  │   ├── file.rs
  │   ├── memory.rs
  │   ├── browser.rs
  │   └── ...
  │
  ├── memory/
  │   ├── traits.rs        # Memory trait
  │   ├── mod.rs           # Factory (create_memory)
  │   ├── sqlite.rs        # Hybrid search (vector + FTS5)
  │   ├── postgres.rs
  │   ├── vector.rs        # Hybrid merge algorithm
  │   ├── embeddings.rs    # EmbeddingProvider trait
  │   └── chunker.rs       # Markdown chunking
  │
  ├── channels/
  │   ├── traits.rs        # Channel trait
  │   ├── mod.rs           # Factory, start_channels
  │   ├── telegram.rs
  │   ├── discord.rs
  │   ├── slack.rs
  │   └── ...
  │
  ├── security/
  │   ├── policy.rs        # SecurityPolicy, AutonomyLevel
  │   ├── secrets.rs       # SecretStore (AEAD encryption)
  │   ├── pairing.rs       # PairingGuard
  │   ├── traits.rs        # Sandbox trait
  │   ├── docker.rs
  │   └── landlock.rs      # Linux sandboxing
  │
  ├── runtime/
  │   ├── traits.rs        # RuntimeAdapter trait
  │   ├── mod.rs           # Factory
  │   ├── native.rs
  │   └── docker.rs
  │
  ├── observability/
  │   ├── traits.rs        # Observer trait
  │   ├── mod.rs           # Factory
  │   ├── prometheus.rs
  │   ├── otel.rs
  │   └── log.rs
  │
  ├── auth/
  │   ├── profiles.rs      # Multi-profile auth storage
  │   ├── openai_oauth.rs  # OAuth device code flow
  │   └── anthropic_token.rs
  │
  ├── gateway/
  │   └── mod.rs           # axum HTTP server, WS
  │
  ├── daemon/
  │   └── mod.rs           # Long-running runtime
  │
  ├── cron/
  │   ├── scheduler.rs
  │   ├── store.rs
  │   └── types.rs
  │
  ├── heartbeat/
  │   └── engine.rs        # Periodic tasks
  │
  ├── hardware/
  │   ├── discover.rs      # USB enumeration (nusb)
  │   ├── introspect.rs
  │   └── registry.rs
  │
  └── peripherals/
      ├── traits.rs        # Peripheral trait
      ├── mod.rs
      ├── nucleo.rs
      ├── rpi_gpio.rs
      └── esp32.rs
```

---

## Core Traits

### Provider Trait

**File:** `src/providers/traits.rs`

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    /// Chat with optional system prompt
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> Result<String>;

    /// Chat with conversation history and optional tools
    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> Result<ChatResponse>;

    /// Whether provider supports native function calling
    fn supports_native_tools(&self) -> bool;

    /// Provider capabilities
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }
}
```

**Key Implementations:**
- `OpenAiProvider` - OpenAI API, GPT-4, o1, o3
- `AnthropicProvider` - Anthropic API, Claude 3/4
- `OllamaProvider` - Local Ollama instances
- `RoutedProvider` - Fallback chain with reliability
- `CustomProvider` - OpenAI-compatible endpoints

### Memory Trait

**File:** `src/memory/traits.rs`

```rust
#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()>;

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>>;

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>>;

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>>;

    async fn forget(&self, key: &str) -> Result<bool>;

    async fn count(&self) -> Result<usize>;

    async fn health_check(&self) -> bool;
}
```

**Key Implementations:**
- `SqliteMemory` - Hybrid search (vector + FTS5)
- `PostgresMemory` - PostgreSQL backend
- `MarkdownMemory` - File-based markdown storage
- `LucidMemory` - Bridge to Lucid external memory
- `NoopMemory` - No persistence

### Tool Trait

**File:** `src/tools/traits.rs`

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>;

    // Default method - full spec
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}
```

### Channel Trait

**File:** `src/channels/traits.rs`

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;

    async fn send(&self, message: &SendMessage) -> Result<()>;

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()>;

    async fn health_check(&self) -> bool { true }

    async fn start_typing(&self, _recipient: &str) -> Result<()> { Ok(()) }

    async fn stop_typing(&self, _recipient: &str) -> Result<()> { Ok(()) }

    fn supports_draft_updates(&self) -> bool { false }

    async fn send_draft(&self, _message: &SendMessage) -> Result<Option<String>> {
        Ok(None)
    }

    async fn update_draft(&self, _recipient: &str, _message_id: &str, _text: &str) -> Result<()> {
        Ok(())
    }

    async fn finalize_draft(&self, _recipient: &str, _message_id: &str, _text: &str) -> Result<()> {
        Ok(())
    }
}
```

### Observer Trait

**File:** `src/observability/traits.rs`

```rust
pub trait Observer: Send + Sync + 'static {
    fn record_event(&self, event: &ObserverEvent);
    fn record_metric(&self, metric: &ObserverMetric);
    fn flush(&self) {}
    fn name(&self) -> &str;
    fn as_any(&self) -> &dyn Any;  // Downcast for backend-specific ops
}
```

**Events:**
```rust
pub enum ObserverEvent {
    AgentStart { provider: String, model: String },
    LlmRequest { provider: String, model: String, messages_count: usize },
    LlmResponse { provider: String, model: String, duration: Duration, success: bool, error_message: Option<String> },
    AgentEnd { provider: String, model: String, duration: Duration, tokens_used: Option<u64>, cost_usd: Option<f64> },
    ToolCallStart { tool: String },
    ToolCall { tool: String, duration: Duration, success: bool },
    TurnComplete,
    ChannelMessage { channel: String, direction: String },
    HeartbeatTick,
    Error { component: String, message: String },
}
```

### RuntimeAdapter Trait

**File:** `src/runtime/traits.rs`

```rust
pub trait RuntimeAdapter: Send + Sync {
    fn name(&self) -> &str;
    fn has_shell_access(&self) -> bool;
    fn has_filesystem_access(&self) -> bool;
    fn storage_path(&self) -> PathBuf;
    fn supports_long_running(&self) -> bool;
    fn memory_budget(&self) -> u64 { 0 }

    fn build_shell_command(
        &self,
        command: &str,
        workspace_dir: &Path,
    ) -> Result<tokio::process::Command>;
}
```

---

## Module Dependencies

### Dependency Rules

1. **Traits are leaf nodes** - No dependencies on concrete implementations
2. **Implementations depend only on traits + utils** - Never on other implementations
3. **Factories depend on traits + all implementations** - For match dispatch
4. **Orchestration depends on factories** - Never on implementations directly

### Dependency Graph

```
                 traits.rs (no deps)
                      ▲
                      │
         ┌────────────┼────────────┐
         │            │            │
   sqlite.rs     postgres.rs   markdown.rs  (deps: traits, utils)
         │            │            │
         └────────────┼────────────┘
                      │
                 mod.rs (factory)
                      │
                      ▼
                 agent.rs (orchestration)
```

### Circular Dependency Prevention

ZeroClaw enforces **strict dependency direction**:

```rust
// ✅ CORRECT: agent uses factory
use crate::memory::create_memory;
let memory = create_memory(&config.memory, ...)?;

// ❌ WRONG: agent imports concrete implementation
use crate::memory::sqlite::SqliteMemory;  // Don't do this
```

---

## Data Flow

### Request Flow (Single Turn)

```
User Input (CLI/Channel)
         │
         ▼
┌─────────────────────────┐
│  Agent::turn()          │
│  - Build system prompt  │
│  - Load memory context  │
│  - Enrich user message  │
└─────────────────────────┘
         │
         ▼
┌─────────────────────────┐
│  Provider::chat()       │
│  - Format request       │
│  - Send to LLM API      │
│  - Parse response       │
└─────────────────────────┘
         │
         ├─── Has tool calls? ───┐
         │                       │
         ▼ No                    ▼ Yes
┌─────────────────┐     ┌─────────────────────┐
│ Return text     │     │ ToolDispatcher::    │
│ to user         │     │   parse_response()  │
└─────────────────┘     └─────────────────────┘
                                │
                                ▼
                        ┌─────────────────────┐
                        │ For each tool call: │
                        │  - Find tool        │
                        │  - Execute          │
                        │  - Record result    │
                        └─────────────────────┘
                                │
                                ▼
                        ┌─────────────────────┐
                        │ Format results as   │
                        │ tool message        │
                        └─────────────────────┘
                                │
                                ▼
                        ┌─────────────────────┐
                        │ Recurse (max N      │
                        │ iterations)         │
                        └─────────────────────┘
                                │
                                ▼
                        Back to Provider::chat()
```

### Memory Flow (Hybrid Search)

```
User Query
    │
    ▼
┌─────────────────────────────┐
│  MemoryLoader::load_context │
│  - Keyword extraction       │
│  - Generate embedding       │
└─────────────────────────────┘
    │
    ├────────────────────┐
    ▼                    ▼
┌─────────────┐   ┌─────────────┐
│ FTS5 Search │   │   Vector    │
│ (BM25)      │   │  Search     │
│             │   │ (Cosine)    │
└─────────────┘   └─────────────┘
    │                    │
    └──────────┬─────────┘
               ▼
    ┌─────────────────────┐
    │  Hybrid Merge       │
    │  weighted_score =   │
    │    vector_weight * vector_score +
    │    keyword_weight * keyword_score
    └─────────────────────┘
               │
               ▼
    ┌─────────────────────┐
    │  Return top-K       │
    │  sorted by score    │
    └─────────────────────┘
```

### Authentication Flow (OAuth)

```
User: auth login --provider openai-codex
              │
              ▼
    ┌───────────────────┐
    │ Generate PKCE     │
    │ - code_verifier   │
    │ - code_challenge  │
    │ - state           │
    └───────────────────┘
              │
              ▼
    ┌───────────────────┐
    │ Build authorize   │
    │ URL with PKCE     │
    └───────────────────┘
              │
              ▼
    ┌───────────────────┐
    │ Start local HTTP  │
    │ server (port 1455)│
    └───────────────────┘
              │
              ▼
    User authorizes in browser
              │
              ▼
    ┌───────────────────┐
    │ Receive callback  │
    │ Extract auth code │
    └───────────────────┘
              │
              ▼
    ┌───────────────────┐
    │ Exchange code for │
    │ tokens (access,   │
    │ refresh)          │
    └───────────────────┘
              │
              ▼
    ┌───────────────────┐
    │ Extract account   │
    │ ID from JWT       │
    └───────────────────┘
              │
              ▼
    ┌───────────────────┐
    │ Store in          │
    │ ~/.zeroclaw/      │
    │   auth-profiles   │
    │   .secret_key     │
    └───────────────────┘
```

---

## Concurrency Model

### Tokio-Based Async

ZeroClaw uses **tokio** with the multi-threaded runtime:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // ...
}
```

### Channel-Based Communication

**Message channels use `tokio::sync::mpsc`:**

```rust
// In agent.rs - interactive mode
let (tx, mut rx) = tokio::sync::mpsc::channel(32);

// Channel listener spawns task
let listen_handle = tokio::spawn(async move {
    let _ = crate::channels::Channel::listen(&cli, tx).await;
});

// Agent receives messages
while let Some(msg) = rx.recv().await {
    let response = self.turn(&msg.content).await?;
    println!("\n{response}\n");
}
```

### Shared State with Arc

**Immutable shared state wrapped in `Arc`:**

```rust
pub struct Agent {
    memory: Arc<dyn Memory>,
    observer: Arc<dyn Observer>,
    // ...
}

// Thread-safe interior mutability when needed
use parking_lot::Mutex;  // Faster than std::sync::Mutex

pub struct ActionTracker {
    actions: Mutex<Vec<Instant>>,
}
```

### Concurrency Patterns

| Pattern | Use Case | Implementation |
|---------|----------|----------------|
| **Task Spawning** | Channel listeners, heartbeat | `tokio::spawn()` |
| **Message Passing** | CLI ↔ Agent, Channel ↔ Agent | `mpsc::channel()` |
| **Shared Read-Only** | Memory, Observer | `Arc<T>` |
| **Interior Mutability** | Rate limiting, counters | `Arc<Mutex<T>>` |
| **Blocking Tasks** | Onboarding wizard | `spawn_blocking()` |

---

## Error Handling Strategy

### Result Type

```rust
use anyhow::Result;  // Primary error type

// Explicit errors for public APIs
use thiserror::Error;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid SSE format: {0}")]
    InvalidSse(String),
}
```

### Error Propagation

**Use `?` operator for propagation:**

```rust
async fn turn(&mut self, user_message: &str) -> Result<String> {
    if self.history.is_empty() {
        let system_prompt = self.build_system_prompt()?;  // ? propagates
        self.history.push(ConversationMessage::Chat(
            ChatMessage::system(system_prompt)
        ));
    }
    // ...
}
```

### Explicit bail! for Invalid States

```rust
if interactive && channels_only {
    bail!("Use either --interactive or --channels-only, not both");
}
```

### Error Context

```rust
let config = Config::load()
    .with_context(|| format!("Failed to load config from {}", config_path))?;
```

---

## Configuration System

### Schema Definition

**File:** `src/config/schema.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub workspace_dir: PathBuf,
    #[serde(skip)]
    pub config_path: PathBuf,

    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: f64,

    #[serde(default)]
    pub observability: ObservabilityConfig,

    #[serde(default)]
    pub autonomy: AutonomyConfig,

    #[serde(default)]
    pub runtime: RuntimeConfig,

    // ... 20+ config sections
}
```

### Default Values

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            workspace_dir: default_workspace_dir(),
            config_path: default_config_path(),
            api_key: None,
            default_provider: Some("openrouter".into()),
            default_model: Some("anthropic/claude-sonnet-4-20250514".into()),
            default_temperature: 0.7,
            // ... uses #[serde(default)] for nested structs
        }
    }
}
```

### Config Loading

```rust
impl Config {
    pub fn load() -> Result<Self> {
        let config_path = default_config_path();
        let config_toml = std::fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&config_toml)?;
        config.config_path = config_path;
        config.workspace_dir = default_workspace_dir();
        Ok(config)
    }

    pub fn apply_env_overrides(&mut self) {
        if let Ok(key) = std::env::var("ZEROCLAW_API_KEY") {
            self.api_key = Some(key);
        }
        // ...
    }
}
```

---

## Extension Points

### Adding a Provider

1. **Implement `Provider` trait:**
```rust
// src/providers/my_provider.rs
#[async_trait]
impl Provider for MyProvider {
    async fn chat_with_system(...) -> Result<String> {
        // ...
    }

    async fn chat(...) -> Result<ChatResponse> {
        // ...
    }

    fn supports_native_tools(&self) -> bool { true }
}
```

2. **Register in factory:**
```rust
// src/providers/mod.rs
pub fn create_provider(name: &str, ...) -> Result<Box<dyn Provider>> {
    match name {
        "my_provider" => Ok(Box::new(MyProvider::new(...)?)),
        // ...
    }
}
```

### Adding a Tool

1. **Implement `Tool` trait:**
```rust
// src/tools/my_tool.rs
#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Does amazing things" }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            }
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        // ...
    }
}
```

### Adding a Channel

1. **Implement `Channel` trait:**
```rust
// src/channels/my_channel.rs
#[async_trait]
impl Channel for MyChannel {
    fn name(&self) -> &str { "my_channel" }
    async fn send(&self, message: &SendMessage) -> Result<()> {
        // ...
    }
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        // ...
    }
}
```

### Adding a Peripheral

1. **Implement `Peripheral` trait:**
```rust
// src/peripherals/my_board.rs
pub struct MyBoard { ... }

#[async_trait]
impl Peripheral for MyBoard {
    fn name(&self) -> &str { "my_board" }
    fn board_type(&self) -> &str { "my-board-v1" }
    fn tools(&self) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(GpioReadTool::new(self)),
            Box::new(GpioWriteTool::new(self)),
        ]
    }
}
```

---

## Security Boundaries

### Trust Boundaries

```
┌────────────────────────────────────────────────────────────┐
│                    Untrusted Input                          │
│  (User messages, channel inputs, webhook payloads)          │
└────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────┐
│                  Input Validation                           │
│  - Allowlist checks (channels)                             │
│  - Path canonicalization (files)                           │
│  - Command allowlist (shell)                               │
│  - Rate limiting (actions/hour)                            │
└────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────┐
│                   Security Policy                           │
│  - Workspace scoping                                       │
│  - Forbidden path checks                                   │
│  - Risk assessment (low/medium/high)                       │
│  - Approval workflow (supervised mode)                     │
└────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌────────────────────────────────────────────────────────────┐
│                 Sandbox Execution                           │
│  - Docker container (optional)                             │
│  - Landlock LSM (Linux, optional)                          │
│  - Native with restrictions (default)                      │
└────────────────────────────────────────────────────────────┘
```

### Security Modules

| Module | Responsibility |
|--------|---------------|
| `policy.rs` | AutonomyLevel, SecurityPolicy, risk assessment |
| `secrets.rs` | SecretStore with ChaCha20-Poly1305 encryption |
| `pairing.rs` | PairingGuard for gateway authentication |
| `docker.rs` | Docker sandbox execution |
| `landlock.rs` | Linux Landlock LSM sandboxing |
| `detect.rs` | Sandbox factory |

### Risk Assessment

```rust
pub fn command_risk_level(&self, command: &str) -> CommandRiskLevel {
    // Split on &&, ||, ;, |, &
    // Check each segment:
    // - High: rm -rf, sudo, curl|bash, etc.
    // - Medium: git push, npm publish, etc.
    // - Low: ls, cat, grep, etc.

    // Any high-risk segment marks whole command high
}
```

---

## Conclusion

ZeroClaw's architecture is built on **trait-driven design, explicit error handling, and secure-by-default principles**. The codebase demonstrates:

1. **Clear separation of concerns** - Each module handles one responsibility
2. **Swappable implementations** - All subsystems are traits with multiple backends
3. **Strict dependency direction** - Implementations depend on traits, not each other
4. **Comprehensive security** - Multiple layers from input validation to sandboxing
5. **Production-grade concurrency** - Tokio-based async with proper error handling
6. **Extensibility** - Clear patterns for adding providers, tools, channels, peripherals

This architecture enables ZeroClaw to achieve its goals of **zero overhead, zero compromise, deploy anywhere**.
