# ZeroClaw Exploration

**Explored:** 2026-03-22
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/zeroclaw`
**Version:** 0.1.0

## Executive Summary

ZeroClaw is a **zero-overhead, zero-compromise, 100% Rust** autonomous AI assistant infrastructure designed for:
- **< 5MB RAM** runtime footprint (99% less than OpenClaw)
- **< 10ms** cold start on 0.8GHz hardware
- **~3.4MB** static binary size
- Runs on $10 hardware (Raspberry Pi, STM32, ESP32)

Built by students and members of Harvard, MIT, and Sundai.Club communities.

---

## Quick Facts

| Metric | Value |
|--------|-------|
| **Language** | 100% Rust (2021 edition) |
| **Binary Size** | 3.4MB (release build) |
| **RAM Usage** | < 5MB peak |
| **Startup Time** | < 10ms (0.8GHz) |
| **Source Files** | 190+ Rust files |
| **License** | MIT / Apache-2.0 |
| **Contributors** | 27+ |

---

## Architecture Overview

### Core Design Philosophy

ZeroClaw follows a **trait-driven, fully swappable architecture** where every subsystem is defined as a trait:

```
Every subsystem is a TRAIT → Swap implementations with config change → Zero code changes
```

### Subsystem Matrix

| Subsystem | Trait | Built-in Implementations |
|-----------|-------|-------------------------|
| **AI Models** | `Provider` | 28+ providers (OpenAI, Anthropic, Ollama, OpenRouter, Zhipu/GLM, custom endpoints) |
| **Channels** | `Channel` | CLI, Telegram, Discord, Slack, Mattermost, iMessage, Matrix, Signal, WhatsApp, Email, IRC, Lark, DingTalk, QQ, Webhook |
| **Memory** | `Memory` | SQLite (hybrid search), PostgreSQL, Lucid bridge, Markdown files, No-op |
| **Tools** | `Tool` | shell, file, memory, cron, git, browser, http_request, screenshot, hardware, delegate |
| **Observability** | `Observer` | Noop, Log, Multi, Prometheus, OpenTelemetry |
| **Runtime** | `RuntimeAdapter` | Native, Docker (sandboxed) |
| **Security** | `SecurityPolicy` | Gateway pairing, sandbox, allowlists, rate limits, filesystem scoping |
| **Identity** | `IdentityConfig` | OpenClaw (markdown), AIEOS v1.1 (JSON) |
| **Tunnel** | `Tunnel` | None, Cloudflare, Tailscale, ngrok, Custom |
| **Peripherals** | `Peripheral` | STM32 Nucleo, Raspberry Pi GPIO, ESP32 |

---

## Directory Structure

```
zeroclaw/
├── src/
│   ├── main.rs              # CLI entrypoint (400+ lines)
│   ├── lib.rs               # Module exports, shared enums
│   ├── agent/               # Core agent loop, dispatcher, memory loader
│   ├── auth/                # OAuth (OpenAI Codex), Anthropic token, profiles
│   ├── approval/            # Action approval workflows
│   ├── channels/            # Messaging platform integrations (15+)
│   ├── config/              # Schema, loading, merging
│   ├── cost/                # Cost tracking, budget enforcement
│   ├── cron/                # Scheduled tasks, cron expression parser
│   ├── daemon/              # Long-running autonomous runtime
│   ├── doctor/              # System diagnostics
│   ├── gateway/             # Webhook server, pairing, WS
│   ├── hardware/            # USB discovery, introspection
│   ├── health/              # Health check endpoints
│   ├── heartbeat/           # Periodic background tasks
│   ├── identity.rs          # Identity loading (OpenClaw/AIEOS)
│   ├── integrations/        # 70+ integration registry
│   ├── memory/              # Vector DB, FTS5, hybrid search, embeddings
│   ├── observability/       # Prometheus, OTel, logging
│   ├── onboard/             # First-time setup wizard
│   ├── peripherals/         # Hardware board management
│   ├── providers/           # LLM provider implementations
│   ├── rag/                 # Retrieval-augmented generation
│   ├── runtime/             # Runtime adapter factory
│   ├── security/            # Policy, sandbox, secrets, pairing
│   ├── service/             # OS service management (systemd/launchd)
│   ├── skillforge/          # Skill loader
│   ├── skills/              # Community skill packs
│   ├── tools/               # Tool implementations
│   ├── tunnel/              # Tunnel adapters
│   └── util/                # Utilities, truncation, helpers
├── crates/
│   └── robot-kit/           # Hardware robotics crate (GPIO, sensors)
├── firmware/
│   ├── zeroclaw-arduino/    # Arduino firmware
│   ├── zeroclaw-esp32/      # ESP32 firmware
│   ├── zeroclaw-esp32-ui/   # ESP32 with UI
│   └── zeroclaw-nucleo/     # STM32 Nucleo firmware
├── python/
│   └── zeroclaw_tools/      # Python companion package (LangGraph)
├── docs/                    # Comprehensive documentation (50+ files)
├── .github/                 # CI/CD workflows (20+ workflows)
├── benches/                 # Criterion benchmarks
├── fuzz/                    # AFL/LibFuzzer fuzzing targets
└── examples/                # Example integrations
```

---

## Key Components Deep Dive

### 1. Agent System (`src/agent/`)

The agent is the **orchestration core** that manages:
- Provider communication
- Tool execution loop
- Memory context loading
- System prompt building
- Conversation history management

**Key Types:**
```rust
pub struct Agent {
    provider: Box<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
    memory: Arc<dyn Memory>,
    observer: Arc<dyn Observer>,
    tool_dispatcher: Box<dyn ToolDispatcher>,  // "native" or "xml"
    memory_loader: Box<dyn MemoryLoader>,
    history: Vec<ConversationMessage>,
    // ...
}
```

**Tool Dispatch Strategies:**
- **Native:** Uses provider's native function calling (OpenAI, Anthropic)
- **XML:** Wraps tool calls in XML tags for providers without native support

**Key Methods:**
- `turn(&mut self, user_message: &str)` - Single conversation turn
- `run_single(&mut self, message: &str)` - One-shot execution
- `run_interactive(&mut self)` - REPL mode

---

### 2. Memory System (`src/memory/`)

**Full-stack search engine with ZERO external dependencies** - no Pinecone, no Elasticsearch:

| Layer | Implementation |
|-------|---------------|
| **Vector DB** | Embeddings stored as BLOB in SQLite, cosine similarity |
| **Keyword Search** | FTS5 virtual tables with BM25 scoring |
| **Hybrid Merge** | Custom weighted merge function (`vector.rs`) |
| **Embeddings** | `EmbeddingProvider` trait - OpenAI, custom URL, or noop |
| **Chunking** | Line-based markdown chunker with heading preservation |
| **Caching** | SQLite `embedding_cache` table with LRU eviction |

**Memory Trait:**
```rust
#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;
    async fn store(&self, key: &str, content: &str, category: MemoryCategory, session_id: Option<&str>) -> Result<()>;
    async fn recall(&self, query: &str, limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>>;
    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>>;
    async fn list(&self, category: Option<&MemoryCategory>, session_id: Option<&str>) -> Result<Vec<MemoryEntry>>;
    async fn forget(&self, key: &str) -> Result<bool>;
    async fn count(&self) -> Result<usize>;
    async fn health_check(&self) -> bool;
}
```

**Memory Categories:**
- `Core` - Long-term facts, preferences, decisions
- `Daily` - Daily session logs
- `Conversation` - Conversation context
- `Custom(String)` - User-defined

---

### 3. Provider System (`src/providers/`)

**28+ built-in providers** with automatic routing:

**Major Providers:**
- OpenAI (GPT-4, GPT-4o, o1, o3)
- Anthropic (Claude 3/4 family)
- Google (Gemini 2.0)
- Meta (Llama 3.1/3.2/3.3 via OpenRouter)
- Ollama (local models)
- Zhipu/GLM (Chinese models)
- DeepSeek
- Groq
- xAI (Grok)

**Custom Endpoints:**
- `custom:https://your-api.com` - OpenAI-compatible
- `anthropic-custom:https://your-api.com` - Anthropic-compatible

**Provider Trait:**
```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat_with_system(&self, system_prompt: Option<&str>, message: &str, model: &str, temperature: f64) -> Result<String>;
    async fn chat(&self, request: ChatRequest<'_>, model: &str, temperature: f64) -> Result<ChatResponse>;
    fn supports_native_tools(&self) -> bool;
}
```

**ProviderCapabilities:**
```rust
pub struct ProviderCapabilities {
    pub supports_native_tools: bool,
    pub supports_system_messages: bool,
    pub prefers_xml_tool_calls: bool,
    // ...
}
```

---

### 4. Channel System (`src/channels/`)

**15+ messaging platform integrations:**

| Channel | Protocol | Notes |
|---------|----------|-------|
| CLI | stdin/stdout | Interactive mode |
| Telegram | Polling | Bot API, allowlist support |
| Discord | WebSocket | Gateway API |
| Slack | HTTP | Events API |
| WhatsApp | Webhook | Meta Cloud API |
| Email | IMAP/SMTP | lettre crate |
| Signal | CLI bridge | signal-cli |
| Matrix | Client-Server | Ruma crate |
| iMessage | AppleScript/DB | macOS only |
| IRC | TCP | irc crate |
| Lark | HTTP | Feishu/Lark API |
| DingTalk | HTTP | Alibaba API |
| QQ | HTTP | Tencent API |
| Mattermost | API v4 | Self-hosted Slack alternative |
| Webhook | HTTP | Generic webhook receiver |

**Channel Trait:**
```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn send(&self, message: &SendMessage) -> Result<()>;
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()>;
    async fn health_check(&self) -> bool;
    async fn start_typing(&self, recipient: &str) -> Result<()>;
    fn supports_draft_updates(&self) -> bool;
}
```

---

### 5. Security System (`src/security/`)

**Secure-by-default at every layer:**

| Security Feature | Implementation |
|-----------------|----------------|
| **Gateway Pairing** | 6-digit one-time code, bearer token exchange |
| **Filesystem Scoping** | `workspace_only = true` by default, 14 system dirs blocked |
| **Tunnel Requirement** | Refuses `0.0.0.0` bind without active tunnel |
| **Channel Allowlists** | Deny-by-default, explicit opt-in |
| **Command Rate Limiting** | 20 actions/hour default |
| **Cost Budgets** | $5/day default, $100/month |
| **Secret Encryption** | ChaCha20-Poly1305 (AEAD) |
| **Sandboxing** | Docker, Landlock (Linux), Firejail |

**Autonomy Levels:**
```rust
pub enum AutonomyLevel {
    ReadOnly,      // Observe only
    Supervised,    // Default - approval for risky ops
    Full,          // Autonomous within bounds
}
```

**SecurityPolicy:**
```rust
pub struct SecurityPolicy {
    pub autonomy: AutonomyLevel,
    pub workspace_only: bool,
    pub allowed_commands: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub max_actions_per_hour: u32,
    pub max_cost_per_day_cents: u32,
    pub block_high_risk_commands: bool,
    // ...
}
```

**Forbidden Paths (default):**
- System: `/etc`, `/root`, `/usr`, `/bin`, `/sbin`, `/lib`, `/boot`, `/dev`, `/proc`, `/sys`, `/var`, `/tmp`
- Sensitive: `~/.ssh`, `~/.gnupg`, `~/.aws`, `~/.config`

---

### 6. Tool System (`src/tools/`)

**Built-in Tools:**

| Tool | Category | Description |
|------|----------|-------------|
| `shell` | Execution | Run shell commands with security policy |
| `file_read` | I/O | Read files (workspace-scoped) |
| `file_write` | I/O | Write files (workspace-scoped) |
| `memory_store` | Memory | Store long-term memories |
| `memory_recall` | Memory | Search/retrieve memories |
| `browser_open` | Browser | Open URLs (agent_browser or rust_native backend) |
| `screenshot` | Browser | Capture screen |
| `http_request` | Network | HTTP GET/POST/PUT/DELETE |
| `cron_schedule` | Automation | Schedule tasks |
| `git` | VCS | Git operations |
| `delegate` | Multi-agent | Delegate to sub-agents |
| `hardware_*` | Hardware | GPIO, sensors, peripherals |

**Tool Trait:**
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>;
}
```

---

### 7. Identity System

**Dual-format identity support:**

**OpenClaw (Default):**
- `IDENTITY.md` - Who the agent is
- `SOUL.md` - Core personality and values
- `USER.md` - Who the agent is helping
- `AGENTS.md` - Behavior guidelines

**AIEOS v1.1 (JSON):**
Portable AI Entity Object Specification for cross-platform identity:

```json
{
  "identity": {
    "names": { "first": "Nova", "nickname": "N" },
    "bio": { "gender": "Non-binary", "age_biological": 3 },
    "origin": { "nationality": "Digital", "birthplace": { "city": "Cloud" } }
  },
  "psychology": {
    "neural_matrix": { "creativity": 0.9, "logic": 0.8 },
    "traits": { "mbti": "ENTP", "ocean": { "openness": 0.8 } }
  },
  "linguistics": {
    "text_style": { "formality_level": 0.2 },
    "idiolect": { "catchphrases": ["Let's test this"] }
  }
}
```

---

### 8. Hardware/Peripherals (`src/peripherals/`, `src/hardware/`)

**Supported Boards:**

| Board | Transport | Capabilities |
|-------|-----------|--------------|
| STM32 Nucleo-F401RE | USB/Serial, probe-rs | GPIO, I2C, SPI, ADC |
| Raspberry Pi | Native GPIO (rppal) | GPIO, I2C, SPI, PWM |
| ESP32 | Serial | WiFi, Bluetooth, GPIO |
| Arduino Uno | Serial | GPIO, Analog, PWM |

**Hardware Transports:**
```rust
pub enum HardwareTransport {
    None,
    Native,     // Direct GPIO (RPi)
    Serial,     // USB serial
    Probe,      // probe-rs (ST-Link)
}
```

**Peripheral Trait:**
```rust
pub trait Peripheral: Send + Sync {
    fn name(&self) -> &str;
    fn board_type(&self) -> &str;
    fn tools(&self) -> Vec<Box<dyn Tool>>;
}
```

---

## CLI Commands

| Command | Description |
|---------|-------------|
| `onboard` | Quick setup / interactive wizard |
| `agent` | Interactive or single-message chat |
| `gateway` | Start webhook server (default: `127.0.0.1:3000`) |
| `daemon` | Start autonomous runtime |
| `service` | OS service management (install/start/stop/status) |
| `doctor` | System diagnostics |
| `status` | Full system status |
| `cron` | Scheduled task management |
| `models` | Provider model catalog refresh |
| `providers` | List supported providers |
| `channel` | Channel management (list/start/doctor/bind-telegram) |
| `integrations` | Integration details |
| `skills` | Skill pack management |
| `migrate` | Import from OpenClaw |
| `hardware` | USB discover/introspect/info |
| `peripheral` | Hardware management (list/add/flash) |
| `auth` | OAuth/token auth profiles |

---

## Configuration

**Location:** `~/.zeroclaw/config.toml`

**Key Sections:**
```toml
api_key = "sk-..."
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4-6"
default_temperature = 0.7

[memory]
backend = "sqlite"
auto_save = true
embedding_provider = "none"

[gateway]
port = 3000
host = "127.0.0.1"
require_pairing = true

[autonomy]
level = "supervised"
workspace_only = true
allowed_commands = ["git", "npm", "cargo", "ls", "cat"]

[runtime]
kind = "native"  # or "docker"

[browser]
enabled = false
backend = "agent_browser"  # or "rust_native"

[identity]
format = "openclaw"  # or "aieos"
```

---

## Gateway API

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/health` | GET | None | Health check |
| `/pair` | POST | `X-Pairing-Code` | Exchange code for bearer token |
| `/webhook` | POST | `Authorization: Bearer` | Send message |
| `/whatsapp` | GET/POST | Meta signature | WhatsApp webhook |

---

## Observability

**Supported Backends:**

1. **Noop** - Disabled
2. **Log** - Tracing subscriber
3. **Multi** - Multiple backends
4. **Prometheus** - Metrics export
5. **OpenTelemetry** - OTLP trace/metrics export

**Observer Events:**
- `AgentStart` / `AgentEnd`
- `LlmRequest` / `LlmResponse`
- `ToolCallStart` / `ToolCall`
- `TurnComplete`
- `ChannelMessage`
- `HeartbeatTick`
- `Error`

**Metrics:**
- Request latency
- Tokens used
- Active sessions
- Queue depth

---

## Build System

**Cargo.toml Highlights:**

```toml
[profile.release]
opt-level = "z"       # Optimize for size
lto = "thin"          # Link-time optimization
codegen-units = 1     # Single codegen unit (low RAM)
strip = true          # Remove debug symbols
panic = "abort"       # Smaller binary
```

**Features:**
- `hardware` (default) - USB/serial hardware support
- `browser-native` - Rust-native browser automation
- `sandbox-landlock` - Linux Landlock sandboxing
- `probe` - probe-rs for STM32
- `rag-pdf` - PDF extraction for datasheets

**Dependencies (Key):**
- `tokio` - Async runtime
- `reqwest` - HTTP client (rustls)
- `axum` - HTTP server (gateway)
- `clap` - CLI parsing
- `serde` - Serialization
- `rusqlite` - SQLite memory backend
- `postgres` - PostgreSQL memory backend
- `prometheus` - Metrics
- `opentelemetry-otlp` - OTel export
- `chacha20poly1305` - Secret encryption
- `nusb` - USB enumeration
- `tokio-serial` - Serial port

---

## Testing

**Test Strategy:**
- Unit tests in each module
- Integration tests in `tests/`
- Fuzzing targets in `fuzz/`
- Benchmarks in `benches/`

**Validation Commands:**
```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

---

## Python Companion Package

**`zeroclaw-tools`** - LangGraph-based tool calling for providers with inconsistent native support:

```bash
pip install zeroclaw-tools
```

```python
from zeroclaw_tools import create_agent, shell, file_read

agent = create_agent(
    tools=[shell, file_read],
    model="glm-5",
    api_key="your-key",
    base_url="https://api.z.ai/api/coding/paas/v4"
)
```

---

## CI/CD

**20+ GitHub Workflows:**

| Workflow | Purpose |
|----------|---------|
| `ci-run.yml` | Main CI (fmt, clippy, test) |
| `sec-audit.yml` | Security audit (`cargo audit`) |
| `sec-codeql.yml` | CodeQL analysis |
| `test-e2e.yml` | End-to-end tests |
| `test-fuzz.yml` | Fuzzing tests |
| `pub-release.yml` | Release publishing |
| `pub-docker-img.yml` | Docker image publishing |
| `pr-labeler.yml` | Automatic PR labeling |
| `pr-intake-checks.yml` | PR validation |

---

## Documentation System

**50+ documentation files** organized into collections:

- `docs/getting-started/` - Installation, quickstart
- `docs/reference/` - Commands, config, providers, channels
- `docs/operations/` - Runbook, deployment, troubleshooting
- `docs/security/` - Security architecture, roadmap
- `docs/hardware/` - Board setup, peripherals
- `docs/contributing/` - PR workflow, CI map, reviewer guide

**Multilingual:**
- English (primary)
- 简体中文 (Chinese)
- 日本語 (Japanese)
- Русский (Russian)

---

## Key Design Decisions

1. **Trait-Driven Architecture** - Every subsystem is a trait for maximum swappability
2. **Zero External Dependencies for Core** - Memory system has no external DB dependencies
3. **Secure by Default** - Deny-by-default access control, workspace scoping
4. **Size-Optimized Builds** - `opt-level = "z"`, `codegen-units = 1`
5. **Deterministic Behavior** - Locked dependencies, reproducible builds
6. **Explicit Error Handling** - Fail fast, no silent fallbacks
7. **Documentation as Product** - Comprehensive docs with multilingual support
8. **Hardware Native** - First-class support for STM32, RPi, ESP32

---

## Comparison with Alternatives

| Feature | ZeroClaw | OpenClaw | NanoBot |
|---------|----------|----------|---------|
| Language | Rust | TypeScript | Python |
| RAM | < 5MB | > 1GB | > 100MB |
| Binary | 3.4MB | ~28MB | N/A |
| Startup | < 10ms | > 500s | > 30s |
| Hardware | $10 boards | Mac mini ($599) | Linux SBC ($50) |

---

## Files of Interest

**Core Architecture:**
- `src/lib.rs` - Module exports, command enums
- `src/main.rs` - CLI entrypoint (1100+ lines)
- `src/agent/agent.rs` - Agent loop (750+ lines)
- `src/config/schema.rs` - Configuration schema

**Traits:**
- `src/memory/traits.rs` - Memory trait
- `src/tools/traits.rs` - Tool trait
- `src/channels/traits.rs` - Channel trait
- `src/providers/traits.rs` - Provider trait
- `src/runtime/traits.rs` - RuntimeAdapter trait
- `src/observability/traits.rs` - Observer trait
- `src/peripherals/traits.rs` - Peripheral trait

**Security:**
- `src/security/policy.rs` - SecurityPolicy, AutonomyLevel
- `src/security/secrets.rs` - SecretStore (ChaCha20-Poly1305)
- `src/security/pairing.rs` - PairingGuard

**Engineering Protocol:**
- `AGENTS.md` - Agent engineering protocol (470+ lines)

---

## Conclusion

ZeroClaw represents a **production-grade, trait-driven AI assistant runtime** built with:
- **Performance** - < 5MB RAM, < 10ms startup
- **Security** - Secure by default, sandboxed execution
- **Extensibility** - Everything is a trait, swap with config
- **Portability** - Single binary, cross-platform, hardware support
- **Observability** - Prometheus, OpenTelemetry integration
- **Documentation** - 50+ files, multilingual

The codebase demonstrates **mature Rust engineering** with:
- Comprehensive test coverage
- Fuzzing infrastructure
- CI/CD automation
- Security-first design
- Clear module boundaries
- Explicit error handling
