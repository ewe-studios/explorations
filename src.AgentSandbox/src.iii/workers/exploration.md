---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/
repository: git@github.com:iii-hq/workers
explored_at: 2026-06-03T00:00:00Z
language: Rust (primary), TypeScript/JavaScript (harness), Python
---

# Project Exploration: iii-workers

## Overview

The iii-workers repository is a comprehensive collection of worker processes for the [iii engine](https://github.com/iii-hq/iii), an AI agent orchestration platform. Workers are self-contained processes that connect to the iii engine over WebSocket, register functions and triggers, and provide specific capabilities ranging from database access to LLM provider integrations.

**Key insight:** This is not a monolithic application but a polyglot ecosystem of specialized workers. Each worker is independently deployable, versioned, and replaceable. The architecture follows a microservices pattern where workers communicate through the iii engine's message bus using a unified function/trigger protocol.

The repository contains 17 distinct worker implementations spanning three languages: Rust (for performance-critical and system-level workers), TypeScript/JavaScript (for the harness stack and UI components), and Python (for quickstart examples and tooling). Workers are distributed as pre-compiled binaries (Rust), bundled JavaScript (Node.js), or container images.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/`
- **Remote:** `git@github.com:iii-hq/workers`
- **Primary Language:** Rust (10 workers), TypeScript/JavaScript (3 workers), Python (2 workers)
- **License:** Apache 2.0
- **SDK Version:** `iii-sdk = "=0.16.0-next.2"` (pinned across Rust workers)

### Recent Commits

```
8e90d57 feat: modifications for provider configuration (#209)
28fb49a fix(turn-orchestrator): update TURN_STEP_QUEUE constant to 'default' (#208)
17e02df feat: removing mock server (#207)
2194ab8 perf: harness telemetry/span-volume reductions (#205)
b9e314c ci: fix fork PR checkout + add recheck-on-comment listener (#200)
```

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/
├── acp/                        # Rust - Agent Client Protocol (ACP) worker
│   ├── Cargo.toml              # iii-sdk = "=0.16.0-next.2"
│   ├── iii.worker.yaml         # Worker manifest
│   └── src/
│       ├── main.rs             # Entry: stdio JSON-RPC bridge
│       ├── lib.rs              # Public API
│       ├── handler.rs          # ACP session handling
│       ├── transport.rs        # stdio transport
│       ├── session.rs          # Session management
│       └── types.rs            # ACP protocol types
├── coder/                      # Rust - Path-jailed code worker
│   ├── Cargo.toml
│   ├── config.yaml             # Runtime configuration
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.rs             # Entry
│       ├── lib.rs
│       ├── config.rs           # Config loading
│       ├── error.rs            # Error types
│       ├── manifest.rs         # --manifest output
│       └── functions/          # File operations
│           ├── mod.rs
│           ├── create_file.rs
│           ├── delete_file.rs
│           ├── list_folder.rs
│           ├── read_file.rs
│           ├── search.rs
│           ├── tree.rs
│           └── update_file.rs
├── console/                    # Rust - Web console for iii
│   ├── Cargo.toml
│   ├── config.yaml
│   ├── iii.worker.yaml
│   ├── DESIGN.md               # Architecture documentation
│   └── src/
│       ├── main.rs             # Entry: HTTP proxy + embedded assets
│       ├── lib.rs
│       ├── server.rs           # Web server
│       ├── proxy.rs            # WebSocket proxy
│       ├── assets.rs           # Embedded web assets
│       └── web/                # React/Vite SPA
├── database/                   # Rust - PostgreSQL/MySQL/SQLite client
│   ├── Cargo.toml
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.rs             # Entry: 11 functions + trigger type
│       ├── lib.rs
│       ├── config.rs           # WorkerConfig
│       ├── configuration.rs    # Dynamic config reloading
│       ├── error.rs
│       ├── handle.rs           # Prepared statement handles
│       ├── transaction.rs      # TxRegistry
│       ├── value.rs            # DB value coercion
│       ├── driver/             # DB drivers
│       │   ├── mod.rs
│       │   ├── postgres.rs     # tokio-postgres
│       │   ├── mysql.rs        # mysql_async
│       │   └── sqlite.rs       # rusqlite (blocking)
│       ├── handlers/           # Function handlers
│       │   ├── mod.rs
│       │   ├── query.rs
│       │   ├── execute.rs
│       │   ├── prepare.rs
│       │   ├── run_statement.rs
│       │   ├── transaction.rs
│       │   └── ...
│       ├── pool/               # Connection pools
│       │   ├── mod.rs
│       │   ├── postgres.rs
│       │   ├── mysql.rs
│       │   └── sqlite.rs
│       └── triggers/           # Row change triggers
├── harness/                    # TS/Node - Meta-worker bundle
│   ├── package.json            # pnpm monorepo, version 0.4.7
│   ├── iii.worker.yaml
│   ├── tsconfig.json
│   └── src/
│       ├── index.ts            # Composite entry (all workers)
│       ├── harness/            # Core harness worker
│       ├── approval-gate/      # Approval resolution
│       ├── turn-orchestrator/  # Agent turn FSM
│       ├── session/            # Session storage
│       ├── llm-budget/         # Spend caps
│       ├── hook-fanout/        # Pub-sub primitive
│       ├── models-catalog/     # Model registry
│       ├── context-compaction/ # History compaction
│       ├── provider-anthropic/ # Anthropic API
│       ├── provider-openai/    # OpenAI API
│       ├── provider-kimi/      # Kimi (Moonshot)
│       ├── provider-lmstudio/  # LM Studio local
│       ├── provider-llamacpp/  # llama.cpp server
│       └── web/                # HTTP client worker
├── iii-directory/              # Rust - Engine introspection
│   ├── Cargo.toml
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.rs
│       └── ...                 # Functions/triggers discovery
├── iii-lsp/                    # Rust - Language Server Protocol
│   ├── Cargo.toml
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.rs
│       ├── analyzer.rs         # Tree-sitter analysis
│       ├── completions.rs      # Auto-completion
│       ├── diagnostics.rs        # Error detection
│       └── ...
├── iii-lsp-vscode/             # Node - VS Code extension
│   ├── extension.js
│   └── installer.js
├── image-resize/               # Rust - Image processing
│   ├── Cargo.toml
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.rs
│       ├── processing.rs       # Image manipulation
│       └── handler.rs
├── mcp/                        # Rust - MCP 2025-06-18 bridge
│   ├── Cargo.toml
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.rs
│       ├── handler.rs          # HTTP bridge handler
│       ├── transport.rs        # Streamable HTTP
│       ├── jsonrpc.rs          # JSON-RPC protocol
│       ├── manifest.rs
│       └── skills_bridge.rs    # Skills integration
├── shell/                      # Rust - Unix shell + filesystem
│   ├── Cargo.toml
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.rs             # Entry: 15 functions
│       ├── config.rs           # ShellConfig + FS jail
│       ├── exec.rs             # Foreground execution
│       ├── exec_dispatch.rs    # Target dispatch
│       ├── jobs.rs             # Background jobs
│       ├── fs/                 # Filesystem backends
│       └── ...
├── storage/                    # Rust - S3-compatible storage
│   ├── Cargo.toml
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── backend/            # S3, GCS, R2, local
│       ├── handlers/           # Object operations
│       └── triggers/           # Object change triggers
├── todo-worker/                # TS - Quickstart example
│   ├── package.json
│   ├── iii.worker.yaml
│   └── src/
│       ├── index.ts
│       ├── handlers.ts
│       └── hooks.ts
├── todo-worker-python/         # Python - Quickstart example
│   ├── pyproject.toml
│   ├── iii.worker.yaml
│   └── src/
│       ├── main.py
│       └── handlers.py
├── .github/
│   ├── workflows/              # CI/CD pipelines
│   │   ├── ci.yml              # Per-worker lint + test
│   │   ├── release.yml         # Unified release dispatcher
│   │   ├── create-tag.yml      # Version bump workflow
│   │   └── _*.yml              # Reusable workflows
│   └── scripts/                # Python automation scripts
├── binary-worker.md            # Rust worker scaffold guide
├── worker-readme.md            # README contract for workers
├── iii-permissions.yaml        # Default permission rules
└── README.md                   # Main repository documentation
```

## Architecture

### High-Level Component Diagram

```mermaid
flowchart TB
    subgraph Engine["iii Engine"]
        E1["WebSocket Bus"]
        E2["Function Registry"]
        E3["Trigger Router"]
        E4["State/Stream Store"]
    end

    subgraph RustWorkers["Rust Binary Workers"]
        R1["shell"]
        R2["database"]
        R3["storage"]
        R4["coder"]
        R5["mcp"]
        R6["acp"]
        R7["iii-directory"]
        R8["iii-lsp"]
        R9["image-resize"]
        R10["console"]
    end

    subgraph NodeWorkers["Node.js Workers (Bundle)"]
        N1["harness"]
        N2["approval-gate"]
        N3["turn-orchestrator"]
        N4["session"]
        N5["providers"]
        N6["llm-budget"]
        N7["context-compaction"]
    end

    subgraph External["External Systems"]
        X1["PostgreSQL/MySQL/SQLite"]
        X2["S3/GCS/R2"]
        X3["Anthropic/OpenAI/Kimi"]
        X4["LM Studio/llama.cpp"]
        X5["Filesystem"]
    end

    E1 <-->|"WebSocket"| RustWorkers
    E1 <-->|"WebSocket"| NodeWorkers
    R2 --> X1
    R3 --> X2
    N5 --> X3
    N5 --> X4
    R1 --> X5
    R4 --> X5
```

### Worker Communication Pattern

```mermaid
sequenceDiagram
    participant Client as SDK Client
    participant Engine as iii Engine
    participant Worker as Worker Process
    participant External as External Resource

    Client->>Engine: register_worker(ws://...)
    Engine->>Worker: WebSocket upgrade
    Worker->>Engine: register_function("shell::exec", ...)
    Engine->>Worker: ACK
    Worker->>Engine: register_trigger("http", ...)
    Engine->>Worker: ACK

    Note over Client,Worker: Function Invocation Flow

    Client->>Engine: trigger("shell::exec", payload)
    Engine->>Worker: Invoke function
    Worker->>External: Execute command
    External-->>Worker: Result
    Worker-->>Engine: Return value
    Engine-->>Client: Response

    Note over Client,Worker: Trigger Subscription Flow

    Worker->>Engine: subscribe("state::change", callback)
    External->>Worker: Event
    Worker->>Engine: state::set(key, value)
    Engine->>Worker: Trigger callback
```

### Worker Registration Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Starting: cargo run
    Starting --> Connecting: Parse CLI args
    Connecting --> Registering: WebSocket upgrade
    Registering --> Running: Functions + triggers registered
    Running --> [*]: SIGINT/SIGTERM

    Registering --> Failed: Connection refused
    Failed --> [*]: Exit with error

    state Running {
        [*] --> Idle: Awaiting invocations
        Idle --> Executing: Function call received
        Executing --> Idle: Return result
        Idle --> Triggered: Trigger event received
        Triggered --> Idle: Callback complete
    }
```

## Component Breakdown

### 1. acp (Agent Client Protocol)

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/acp/`

**Purpose:** Exposes iii agents as ACP (Agent Client Protocol) sessions over stdio JSON-RPC. Acts as a bridge between ACP-compatible clients and the iii engine.

**Key files:**
- `src/main.rs:80-128` - Entry point with brain function routing
- `src/handler.rs` - ACP session state machine
- `src/transport.rs` - stdio transport layer

**Configuration via CLI:**
```rust
// src/main.rs:19-78
#[derive(Parser, Debug)]
#[command(name = "iii-acp")]
struct Args {
    #[arg(long, short = 'e', env = "IIIACP_ENGINE_URL", default_value = "ws://localhost:49134")]
    engine_url: String,
    #[arg(long, short = 'd')]
    debug: bool,
    #[arg(long, env = "IIIACP_BRAIN_FN")]
    brain_fn: Option<String>,  // Function ID for turn processing
    #[arg(long, env = "IIIACP_USE_CANONICAL_BRAIN")]
    use_canonical_brain: bool,  // Shortcut for run::start_and_wait
    #[arg(long, env = "IIIACP_MODEL")]
    model: Option<String>,
    #[arg(long, env = "IIIACP_PROVIDER")]
    provider: Option<String>,
}
```

**Dependencies:** `iii-sdk = "=0.16.0-next.2"`, `tokio`, `serde`, `uuid`, `dashmap`

---

### 2. coder

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/coder/`

**Purpose:** Path-jailed file system operations for code manipulation. Provides read, write, create, delete, search, tree listing with glob protection.

**Key insight:** The coder worker implements defense-in-depth path security. It uses a jail root directory combined with denylist patterns to prevent accessing sensitive paths. The `path/` module contains the jailing logic that normalizes and validates all paths before operations.

**Function surface (8 functions):**
- `coder::create_file` - Create new file
- `coder::read_file` - Read file contents
- `coder::update_file` - Patch file with operations
- `coder::delete_file` - Delete file
- `coder::list_folder` - Paginated directory listing
- `coder::tree` - Recursive directory tree
- `coder::search` - Content search

**Configuration (`config.yaml`):**
```yaml
jail_root: "${WORKER_JAIL}"  # Required: base directory for all operations
denylist_patterns:            # Glob patterns to exclude
  - "*.secret"
  - "**/.git/**"
max_read_bytes: 1048576      # 1MB read limit
max_write_bytes: 1048576     # 1MB write limit
```

**Testing:** Uses Cucumber BDD with Gherkin feature files in `tests/features/`

---

### 3. console

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/console/`

**Purpose:** Web console for iii that bundles a React UI and proxies the engine WebSocket on a single HTTP port. Embeds the web SPA directly into the binary using `rust-embed`.

**Architecture:**
- `src/server.rs` - HTTP server with Axum
- `src/proxy.rs` - WebSocket proxy to engine
- `src/assets.rs` - Embedded static assets via `rust-embed`
- `web/` - Vite + React SPA

**Key files:**
- `src/main.rs` - Entry with HTTP server + WS proxy
- `DESIGN.md` - Console architecture documentation
- `PRODUCT.md` - Product requirements

---

### 4. database

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/database/`

**Purpose:** Multi-database SQL client supporting PostgreSQL, MySQL, and SQLite with connection pooling, transactions, prepared statements, and change feeds.

**Key insight:** The database worker uses a handle registry pattern for prepared statements. Handles are reference-counted and auto-evicted after TTL. This allows long-lived prepared statements without leaking resources.

**Function surface (11 functions):**
- `database::query` - Read-only SQL
- `database::execute` - Write statements (INSERT/UPDATE/DELETE/DDL)
- `database::prepareStatement` - Create parameterized statement
- `database::runStatement` - Execute prepared handle
- `database::transaction` - Atomic multi-statement block
- `database::beginTransaction` - Start interactive transaction
- `database::transactionQuery` - Query within transaction
- `database::transactionExecute` - Execute within transaction
- `database::commitTransaction` - Commit transaction
- `database::rollbackTransaction` - Rollback transaction

**Trigger type:**
- `database::row-change` - PostgreSQL logical replication (stubbed pending tokio-postgres replication API)

**Architecture:**
```mermaid
flowchart LR
    subgraph Handlers
        H1[query]
        H2[execute]
        H3[transaction]
    end

    subgraph State
        S1[PoolRegistry]
        S2[HandleRegistry]
        S3[TxRegistry]
    end

    subgraph Drivers
        D1[PostgreSQL]
        D2[MySQL]
        D3[SQLite]
    end

    Handlers --> State
    State --> Drivers
    D1 --> Postgres[(PostgreSQL)]
    D2 --> MySQL[(MySQL)]
    D3 --> SQLite[(SQLite)]
```

**Source highlights:**
- `src/main.rs:42-285` - Entry with configuration loading and function registration
- `src/pool/` - Connection pooling per driver
- `src/handlers/` - Function implementations
- `src/transaction.rs` - Transaction registry with timeout watcher

**Configuration via `configuration` worker:**
```yaml
database:
  connections:
    - name: "main-postgres"
      driver: "postgres"
      url: "postgresql://user:pass@localhost/db"
    - name: "analytics-mysql"
      driver: "mysql"
      url: "mysql://user:pass@localhost/db"
    - name: "local-cache"
      driver: "sqlite"
      path: "~/.iii/cache.db"
```

---

### 5. harness

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/harness/`

**Purpose:** Meta-worker that composes multiple Node.js workers into a single bundle. This is the core chat/AI surface of iii, containing the turn orchestration, approval gating, LLM providers, and session management.

**Key insight:** The harness is architected as a composite worker. While each sub-worker can run independently, `src/index.ts` provides a unified entry point that spins up all 13 workers in one process. This reduces operational complexity while maintaining logical separation.

**Workers bundled:**
| Worker | Function Prefix | Purpose |
|--------|-----------------|---------|
| harness | `harness::`, `ui::`, `policy::` | Meta-worker, permissions, provider registry |
| turn-orchestrator | `run::`, `turn::` | Agent turn state machine |
| approval-gate | `approval::` | Human-in-the-loop approvals |
| session | `session-tree::`, `session-inbox::` | Session storage |
| llm-budget | `budget::` | Spend tracking and caps |
| hook-fanout | `hook-fanout::` | Pub-sub primitives |
| models-catalog | `models::` | Model capability registry |
| provider-anthropic | `provider::anthropic::` | Anthropic API integration |
| provider-openai | `provider::openai::` | OpenAI API integration |
| provider-kimi | `provider::kimi::` | Kimi (Moonshot) API |
| provider-lmstudio | `provider::lmstudio::` | LM Studio local models |
| provider-llamacpp | `provider::llamacpp::` | llama.cpp server |
| context-compaction | (side-car) | Session history compaction |
| web | `web::` | Outbound HTTP client |

**Entry point (`src/index.ts:37-123`):**
```typescript
const WORKERS: readonly WorkerDefinition[] = [
  { name: 'harness', register: (iii, ctx) => registerHarness(iii, ctx) },
  { name: 'turn-orchestrator', register: (iii, ctx) => registerTurnOrchestrator(iii, ctx) },
  { name: 'approval-gate', register: async (iii) => { ... } },
  // ... 13 workers total
];
```

**Runtime (`src/runtime/`):**
- `worker.ts` - Bootstrap and lifecycle management
- `otel.ts` - OpenTelemetry integration
- `state.ts` - State wrapper helpers
- `stream.ts` - Stream wrapper helpers

---

### 6. shell

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/shell/`

**Purpose:** Unix shell execution with filesystem operations. Supports both foreground commands with output capture and background jobs with status polling.

**Key insight:** The shell worker implements a defense-in-depth security model combining multiple layers: (1) an allowlist of permitted commands, (2) a denylist of forbidden patterns, (3) filesystem jail via `host_root`, and (4) configurable timeouts and output size limits.

**Function surface (15 functions):**

**Execution:**
- `shell::exec` - Run command, wait for completion, return output
- `shell::exec_bg` - Spawn background job, return job_id
- `shell::kill` - Terminate background job
- `shell::status` - Poll job status
- `shell::list` - List all background jobs

**Filesystem:**
- `shell::fs::ls` - List directory
- `shell::fs::stat` - File metadata
- `shell::fs::mkdir` - Create directory
- `shell::fs::rm` - Remove file/directory
- `shell::fs::chmod` - Change permissions
- `shell::fs::mv` - Move/rename
- `shell::fs::grep` - Recursive regex search
- `shell::fs::sed` - Find and replace
- `shell::fs::read` - Read file (stream via channel)
- `shell::fs::write` - Write file (stream via channel)

**Configuration (`config.yaml`):**
```yaml
# Security: commands must be in this list
allowlist:
  - "git"
  - "cargo"
  - "npm"
  - "pnpm"

# Security: patterns to reject (regex)
denylist_patterns:
  - "rm\s+-rf\s+/"

# Filesystem jail
fs:
  host_root: "${HOME}/projects"
  max_read_bytes: 10485760   # 10MB
  max_write_bytes: 10485760  # 10MB
  denylist_paths:
    - "**/.ssh/**"
    - "**/.aws/**"

# Resource limits
timeout_ms: 30000
max_timeout_ms: 60000
max_output_bytes: 1048576
max_concurrent_jobs: 10
```

**Source highlights:**
- `src/main.rs:33-367` - Entry with 15 function registrations
- `src/exec.rs` - Foreground execution handler
- `src/jobs.rs` - Background job management
- `src/fs/host.rs` - Host filesystem backend with jail

---

### 7. storage

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/storage/`

**Purpose:** S3-compatible object storage across multiple backends: AWS S3, Google Cloud Storage (GCS), Cloudflare R2, and a managed local rustfs backend.

**Architecture:**
```mermaid
flowchart TD
    subgraph API
        A1[put_object]
        A2[get_object]
        A3[delete_object]
        A4[presign_url]
    end

    subgraph BackendFactory
        F1[Backend::S3]
        F2[Backend::Gcs]
        F3[Backend::R2]
        F4[Backend::Local]
    end

    subgraph Backends
        B1[aws-sdk-s3]
        B2[gcloud-storage]
        B3[s3-compatible]
        B4[rustfs]
    end

    API --> BackendFactory
    BackendFactory --> Backends
```

**Trigger types:**
- `storage::object-created` - Fires when object is uploaded
- `storage::object-deleted` - Fires when object is removed

---

### 8. mcp

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/mcp/`

**Purpose:** MCP (Model Context Protocol) 2025-06-18 Streamable HTTP bridge. Exposes iii functions tagged with `mcp.expose` as MCP tools, enabling integration with MCP-compatible clients like Claude Desktop.

**Key insight:** The MCP worker acts as a protocol adapter. It translates between iii's function/trigger protocol and the MCP specification, allowing iii workers to serve as MCP servers without code changes.

**Configuration (`config.yaml`):**
```yaml
api_path: "/mcp/v1"      # HTTP endpoint path
listen_addr: "0.0.0.0:3000"  # Optional: dedicated HTTP server
```

---

### 9. iii-directory

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/iii-directory/`

**Purpose:** Engine introspection worker. Provides discovery of functions, triggers, and workers; proxies the workers registry API; reads skills and prompts from the filesystem.

**Function surface:**
- `directory::engine::functions::list` - List all registered functions
- `directory::engine::functions::info` - Get function metadata
- `directory::engine::triggers::list` - List trigger types
- `directory::engine::triggers::info` - Get trigger metadata
- `directory::engine::workers::list` - List connected workers
- `directory::engine::workers::info` - Get worker metadata
- `directory::skills::list` - List available skills
- `directory::skills::get` - Get skill definition
- `directory::skills::download` - Download skill archive
- `directory::prompts::list` - List prompt templates
- `directory::prompts::get` - Get prompt template
- `directory::prompts::download` - Download prompt

---

### 10. iii-lsp

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/workers/iii-lsp/`

**Purpose:** Language Server Protocol implementation for iii. Provides autocompletion and hover information for iii function IDs and trigger configurations across JS/TS, Python, and Rust.

**Architecture:**
- `src/analyzer.rs` - Tree-sitter based code analysis
- `src/completions.rs` - Auto-completion provider
- `src/hover.rs` - Hover information provider
- `src/diagnostics.rs` - Error detection

**Supported languages:** TypeScript, Python, Rust (via tree-sitter parsers)

---

## Entry Points

### Rust Worker Entry Pattern

All Rust workers follow the same entry pattern from `binary-worker.md`:

**File:** `src/main.rs` (all Rust workers)

```rust
// shell/src/main.rs:32-367 (representative)
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(...)
        .init();

    // 2. Parse CLI
    let cli = Cli::parse();

    // 3. Handle --manifest (registry publish)
    if cli.manifest {
        let m = manifest::build_manifest();
        println!("{}", serde_json::to_string_pretty(&m)?);
        return Ok(());
    }

    // 4. Load config
    let cfg = match config::load_config(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("failed to load config, using defaults");
            Config::default()
        }
    };

    // 5. Connect to iii engine
    let iii = register_worker(&cli.url, InitOptions::default());

    // 6. Register functions
    functions::register_all(&iii, &cfg);

    // 7. Register triggers (optional)
    iii.register_trigger(RegisterTriggerInput { ... });

    // 8. Wait for shutdown
    tokio::signal::ctrl_c().await?;
    iii.shutdown_async().await;
    Ok(())
}
```

### Harness Entry Pattern

**File:** `harness/src/index.ts:125-183`

```typescript
async function main(): Promise<void> {
  const cli = program.opts();

  // Handle --manifest
  if (cli.manifest) {
    const manifest = WORKERS.map(({ name, description }) => ({ name, description }));
    process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
    return;
  }

  // Spin up all workers
  const handles: WorkerHandle[] = [];
  for (const def of WORKERS) {
    const handle = await runWorker({
      name: def.name,
      description: def.description,
      register: def.register,
      configPath: cli.config,
      url: cli.url,
    });
    handles.push(handle);
  }

  // Wait for shutdown
  await waitForShutdown();
  await shutdownAll(handles);
}
```

## Data Flow

### Function Invocation Flow

```mermaid
sequenceDiagram
    actor User
    participant SDK as iii-sdk
    participant Engine as iii Engine
    participant Worker as Worker Process
    participant Resource as External Resource

    User->>SDK: trigger(function_id, payload)
    SDK->>Engine: WebSocket: trigger message
    Engine->>Engine: Lookup function registry
    Engine->>Worker: Route to registered handler
    Worker->>Worker: Deserialize payload
    Worker->>Resource: Execute operation
    Resource-->>Worker: Result
    Worker-->>Engine: Return value
    Engine-->>SDK: Response
    SDK-->>User: Result
```

### Configuration Hot-Reload Flow

```mermaid
sequenceDiagram
    participant FS as Filesystem
    participant Worker as Worker Process
    participant Engine as iii Engine
    participant State as State Store

    FS->>Worker: File change event
    Worker->>Worker: Reload config.yaml
    Worker->>State: state::set("config", new_value)
    State-->>Engine: Update notification
    Engine->>Engine: Notify subscribers
    Engine->>Worker: Trigger callback (if subscribed)
```

## External Dependencies

### Core SDK

| Dependency | Version | Purpose |
|------------|---------|---------|
| iii-sdk (Rust) | `=0.16.0-next.2` | Core worker SDK |
| iii-observability | `=0.16.0-next.2` | OpenTelemetry integration |
| iii-sdk (Node) | `^0.16.1` | Node.js SDK |

### Rust Workers

| Worker | Key Dependencies |
|--------|------------------|
| database | tokio-postgres, mysql_async, rusqlite, deadpool-postgres |
| storage | aws-sdk-s3, gcloud-storage, aws-sigv4 |
| shell | libc, walkdir, regex, shell-words |
| console | axum, tower, tokio-tungstenite, rust-embed |
| image-resize | image, kamadak-exif |
| mcp | (iii-sdk only) |

### Node.js Workers (harness)

| Dependency | Purpose |
|------------|---------|
| @anthropic-ai/sdk | Anthropic API |
| openai | OpenAI API |
| zod | Schema validation |
| @opentelemetry/api | Telemetry |

## Configuration

### Worker Manifest (`iii.worker.yaml`)

Each worker defines its metadata in `iii.worker.yaml`:

```yaml
iii: v1
name: <worker>              # Must match folder name
language: rust | javascript | python
deploy: binary | bundle | image
manifest: Cargo.toml | package.json | pyproject.toml
bin: <binary_name>          # For binary deploy
description: "One-line description"

targets:                    # Optional: supported architectures
  - x86_64-unknown-linux-gnu

runtime:                    # Optional: runtime config
  kind: rust | javascript | python
  entry: src/main.py      # For image deploy

resources:                  # Optional: container resources
  memory: 256
  cpus: 1

env:                        # Optional: environment variables
  III_URL: "ws://localhost:49134"

dependencies:               # Optional: other worker dependencies
  skills: "^0.2.1"
```

### Permission System (`iii-permissions.yaml`)

Default permissions loaded by the harness worker:

```yaml
version: 1
rules:
  # Deny dangerous operations
  - '!approval::resolve'
  - '!state::set'
  - '!state::delete'
  - '!harness::provider::resolve'
  - '!configuration::get'
  - '!configuration::set'

  # Allow read-only introspection
  - state::get
  - state::list
  - models::list
  - directory::engine::functions::list
  - ...
```

**Rule precedence:** First match wins. Patterns support:
- Exact match: `shell::exec`
- Wildcard: `'*'` (allow all)
- Prefix wildcard: `'*::list'`
- Suffix wildcard: `'worker::*'`
- Negation: `'!function_id'`

## Testing

### Rust Workers

Each Rust worker has tests under `tests/`:

```bash
cd <worker>
cargo test --all-features        # Unit + integration
cargo test --test bdd              # BDD tests (if using Cucumber)
```

**Test patterns:**
1. **Unit tests** - In `#[cfg(test)]` modules
2. **Integration tests** - Spawn engine + worker via `which::which("iii")`
3. **BDD tests** - Gherkin `.feature` files with Cucumber

### Harness (Node.js)

```bash
cd harness
pnpm install
pnpm test          # vitest
```

## CI/CD

### Continuous Integration (`.github/workflows/ci.yml`)

1. **Discover** - Find changed workers from PR diff
2. **PR Checks** - Validate `iii.worker.yaml`, README.md, version bump
3. **Rust Lint** - `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
4. **Node Lint** - `biome ci`, `npm test`
5. **Python Lint** - `ruff check`, `ruff format --check`, `pytest`

### Release Flow

1. **Create Tag** workflow - Pick worker, bump type (patch/minor/major), registry tag
2. Tag format: `<worker>/v<X.Y.Z>`
3. **Release** workflow fires on tag:
   - Cross-compile for 9 targets (Linux gnu/musl, macOS x86_64/aarch64, Windows)
   - Upload to GitHub Release with SHA-256 checksums
   - Call `POST /publish` on workers registry API

**Supported targets:**
```
aarch64-apple-darwin
x86_64-apple-darwin
x86_64-pc-windows-msvc
i686-pc-windows-msvc
aarch64-pc-windows-msvc
x86_64-unknown-linux-gnu
x86_64-unknown-linux-musl
aarch64-unknown-linux-gnu
armv7-unknown-linux-gnueabihf
```

## Key Insights

### 1. Worker Isolation Pattern

Each worker is an independent process with its own lifecycle. This provides:
- **Fault isolation** - A crash in one worker doesn't affect others
- **Language flexibility** - Use the best language for each domain
- **Independent scaling** - Scale providers separately from storage
- **Zero-downtime updates** - Replace workers individually

### 2. The iii SDK Contract

All workers use a unified SDK interface:

```rust
// Register with engine
let iii = register_worker("ws://127.0.0.1:49134", InitOptions::default());

// Register a function
iii.register_function(RegisterFunction::new_async("worker::verb", handler));

// Call another function
let result = iii.trigger(TriggerRequest {
    function_id: "other::function".into(),
    payload: json!({ ... }),
    timeout_ms: Some(5000),
}).await?;
```

**Aha:** The SDK abstracts the WebSocket protocol, function routing, and serialization. Workers only concern themselves with business logic.

### 3. Configuration as Code

Workers use filesystem-backed configuration with hot-reload:
- `config.yaml` - Worker-specific settings
- `iii-permissions.yaml` - Access control rules
- Workers watch files and reload on change

This enables runtime tuning without restarts.

### 4. Registry-Driven Distribution

Workers are discovered via the workers registry API at `https://api.workers.iii.dev`:

```bash
# Install a worker
iii worker add shell

# The CLI:
# 1. Queries registry for worker metadata
# 2. Downloads binary for host architecture
# 3. Verifies SHA-256 checksum
# 4. Writes config to ~/.iii/config.yaml
```

### 5. Defense in Depth Security

The shell worker exemplifies the security model:
1. **Allowlist** - Only permitted commands
2. **Denylist** - Forbidden patterns (regex)
3. **Jail** - Filesystem root restriction
4. **Resource caps** - Timeouts, output limits
5. **Size limits** - Read/write byte caps

## Open Questions

1. **Scaling strategy** - How does the engine handle hundreds of concurrent workers? Is there a connection pool limit?

2. **State consistency** - The database worker supports transactions, but what about cross-worker atomic operations?

3. **Provider failover** - If `provider-anthropic` fails, does the turn-orchestrator automatically retry with `provider-openai`?

4. **Custom trigger lifecycle** - How are trigger subscriptions cleaned up when a worker disconnects unexpectedly?

5. **Worker dependencies** - The `mcp` worker declares `skills: "^0.2.1"` - is there a startup order guarantee?

## File References

Key source locations for understanding the system:

| Concept | File | Lines |
|---------|------|-------|
| Worker entry pattern | `shell/src/main.rs` | 32-367 |
| Function registration | `database/src/main.rs` | 112-267 |
| Harness composite | `harness/src/index.ts` | 1-183 |
| SDK connection | `acp/src/main.rs` | 80-128 |
| Config loading | `shell/src/config.rs` | 1-200 |
| BDD test example | `coder/tests/bdd.rs` | 1-50 |
| CI workflow | `.github/workflows/ci.yml` | 1-263 |
| Worker manifest schema | `shell/iii.worker.yaml` | 1-30 |
| Permission rules | `iii-permissions.yaml` | 1-58 |
| Binary scaffold | `binary-worker.md` | 1-1339 |

---

*This exploration was generated on 2026-06-03 following the Exploration Agent guidelines from /home/darkvoid/Boxxed/@dev/repo-expolorations/.agents/exploration-agent.md*
