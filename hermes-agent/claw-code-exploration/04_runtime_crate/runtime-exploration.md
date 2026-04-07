# Runtime Crate — Module Exploration

**Crate:** `runtime`  
**Status:** Most significantly enhanced crate in claw-code-latest (~4,500 → ~12,000 lines, +167%)  
**Purpose:** Core runtime functionality, session management, permissions, MCP, policy engine  
**Total Files:** 
- claw-code: ~25 files
- claw-code-latest: 39 files

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [File Inventory](#file-inventory)
3. [Core Modules](#core-modules)
4. [New Modules in claw-code-latest](#new-modules-in-claw-code-latest)
5. [Key Structures](#key-structures)
6. [Integration Points](#integration-points)

---

## Module Overview

The runtime crate is the **largest and most complex** crate in claw-code, providing:

- **Session management** - Conversation history, state persistence
- **Permission enforcement** - Tool gating, workspace boundaries
- **MCP lifecycle** - Model Context Protocol server management
- **Policy engine** - Executable automation rules
- **Lane events** - Typed events for clawhip integration
- **Worker boot** - Worker status state machine, trust resolution
- **Branch awareness** - Lock detection, stale branch checking
- **Task registry** - In-memory task lifecycle
- **Team/cron registry** - Scheduled task management
- **LSP client** - Language Server Protocol integration
- **OAuth flows** - PKCE-based authentication
- **Prompt caching** - File-based prompt/response caching

---

## File Inventory

### Core Files (Both Versions)

| File | Purpose |
|------|---------|
| `lib.rs` | Crate root, re-exports |
| `session.rs` | Session state, persistence |
| `conversation.rs` | Conversation message types |
| `permissions.rs` | Permission mode definitions |
| `permission_enforcer.rs` | Tool gating logic |
| `config.rs` | Configuration loading |
| `prompt.rs` | System prompt generation |
| `sse.rs` | Server-Sent Events parsing |
| `file_ops.rs` | Read/write file operations |
| `bash.rs` | Bash execution |
| `compact.rs` | Session compaction |
| `usage.rs` | Token usage tracking |
| `oauth.rs` | OAuth authentication |

### New Files in claw-code-latest

| File | Purpose |
|------|---------|
| `bash_validation.rs` | Enhanced bash sandbox validation |
| `bootstrap.rs` | Bootstrap phase detection |
| `branch_lock.rs` | Parallel work collision detection |
| `green_contract.rs` | Test status contracts |
| `hooks.rs` | Hook execution framework |
| `json.rs` | JSON utilities |
| `lane_events.rs` | Typed lane events |
| `lsp_client.rs` | LSP client implementation |
| `mcp.rs` | MCP server management |
| `mcp_client.rs` | MCP client protocol |
| `mcp_lifecycle_hardened.rs` | Degraded startup reporting |
| `mcp_stdio.rs` | MCP stdio transport |
| `mcp_tool_bridge.rs` | MCP tool registry bridge |
| `plugin_lifecycle.rs` | Plugin init/shutdown |
| `policy_engine.rs` | Automation rule engine |
| `recovery_recipes.rs` | Automatic failure recovery |
| `remote.rs` | Remote execution support |
| `sandbox.rs` | Sandbox status |
| `session_control.rs` | Session lifecycle control |
| `stale_branch.rs` | Branch freshness detection |
| `summary_compression.rs` | Context summarization |
| `task_packet.rs` | Structured task format |
| `task_registry.rs` | Task lifecycle tracking |
| `team_cron_registry.rs` | Team/cron job management |
| `trust_resolver.rs` | Trust resolution for workers |
| `worker_boot.rs` | Worker state machine |

---

## Core Modules

### Session Management (`session.rs`)

```rust
pub struct Session {
    pub version: u32,
    pub messages: Vec<ConversationMessage>,
    // ... more fields
}
```

**Responsibilities:**
- Store conversation history
- Track token usage
- Persist to JSONL/JSON
- Support resume functionality

### Permission System (`permissions.rs`, `permission_enforcer.rs`)

```rust
pub enum PermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

pub struct PermissionEnforcer {
    workspace_root: PathBuf,
    mode: PermissionMode,
}
```

**Responsibilities:**
- Gate tool execution by permission level
- Validate file paths within workspace
- Handle permission prompts

### MCP Lifecycle (`mcp.rs`, `mcp_client.rs`, `mcp_lifecycle_hardened.rs`)

```rust
pub struct McpServerManager {
    servers: BTreeMap<String, McpServer>,
}

pub struct McpDegradedReport {
    pub servers_pending: Vec<String>,
    pub servers_degraded: Vec<String>,
}
```

**Responsibilities:**
- Discover MCP servers
- Report degraded startup states
- Bridge MCP tools to claw-code

### Worker Boot (`worker_boot.rs`)

```rust
pub enum WorkerStatus {
    Untrusted,
    Trusted,
    Ready,
}

pub struct WorkerRegistry {
    workers: BTreeMap<String, WorkerStatus>,
}
```

**Responsibilities:**
- Track worker trust state
- Handle ready handshake
- Resolve trust for background workers

---

## New Modules in claw-code-latest

### Lane Events (`lane_events.rs`)

```rust
pub struct LaneContext {
    pub lane_id: String,
    pub green_level: u8,
    pub blocker: LaneBlocker,
    pub review_status: ReviewStatus,
    pub completed: bool,
    // ... more fields
}
```

**Purpose:** Typed lane events for clawhip integration.

### Policy Engine (`policy_engine.rs`)

```rust
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

pub struct PolicyRule {
    pub name: &'static str,
    pub condition: PolicyCondition,
    pub action: PolicyAction,
    pub priority: u8,
}
```

**Purpose:** Executable automation rules for lane lifecycle.

### Branch Lock Detection (`branch_lock.rs`)

```rust
pub fn detect_parallel_work_collision() -> Result<bool, RuntimeError>
```

**Purpose:** Detect when multiple contributors are working on the same branch.

### Stale Branch Detection (`stale_branch.rs`)

```rust
pub fn check_branch_freshness() -> Result<BranchFreshness, RuntimeError>
```

**Purpose:** Detect if local branch is behind main.

### Task Registry (`task_registry.rs`)

```rust
pub struct TaskRegistry {
    tasks: BTreeMap<String, TaskState>,
}
```

**Purpose:** In-memory task lifecycle tracking.

### Team Cron Registry (`team_cron_registry.rs`)

```rust
pub struct TeamRegistry {
    teams: BTreeMap<String, TeamConfig>,
}

pub struct CronRegistry {
    cron_jobs: Vec<CronJob>,
}
```

**Purpose:** Team and scheduled job management.

### Recovery Recipes (`recovery_recipes.rs`)

```rust
pub fn suggest_recovery(error: &RuntimeError) -> Option<RecoveryAction>
```

**Purpose:** Automatic failure recovery suggestions.

---

## Key Structures

### PermissionMode

```rust
pub enum PermissionMode {
    ReadOnly,        // Read files, search, web tools
    WorkspaceWrite,  // Write within workspace
    DangerFullAccess,// Unrestricted access
}
```

### Session

```rust
pub struct Session {
    pub version: u32,
    pub messages: Vec<ConversationMessage>,
    pub usage: TokenUsage,
    // ... more fields
}
```

### TokenUsage

```rust
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}
```

### ConversationMessage

```rust
pub struct ConversationMessage {
    pub role: MessageRole,
    pub content: Vec<ContentBlock>,
}

pub enum MessageRole {
    User,
    Assistant,
    System,
    ToolResult,
}
```

---

## Integration Points

### Upstream Dependencies

| Crate | Usage |
|-------|-------|
| `api` | API types, streaming, OAuth |
| `serde`, `serde_json` | Serialization |
| `tokio` | Async runtime |
| `reqwest` | HTTP client |

### Downstream Dependents

| Crate | How it uses runtime |
|-------|---------------------|
| `api` | Prompt caching, session state |
| `commands` | Session compaction |
| `tools` | Tool execution functions |
| `rusty-claude-cli` | Session management, permissions |
| `plugins` | Plugin lifecycle hooks |

---

## Summary

The runtime crate is the **backbone of claw-code** with:

| Metric | claw-code | claw-code-latest | Delta |
|--------|-----------|------------------|-------|
| Files | ~25 | 39 | +14 |
| Lines | ~4,500 | ~12,000 | +167% |
| Modules | Core only | +20 new modules | |

**Major additions in claw-code-latest:**

1. **Lane event system** - Structured lane lifecycle events
2. **Policy engine** - Rule-based automation
3. **MCP hardening** - Degraded startup, stdio transport
4. **Worker management** - Trust resolution, boot state machine
5. **Branch awareness** - Lock detection, stale checking
6. **Task registry** - In-memory task tracking
7. **Team/cron system** - Scheduled job management
8. **Recovery recipes** - Automatic failure recovery
9. **LSP client** - Language server integration
10. **Enhanced sandbox** - Bash validation improvements

The runtime crate grew by **167%** making it the most significantly enhanced crate in claw-code-latest.
