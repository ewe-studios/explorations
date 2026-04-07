# Tools Crate — Line-by-Line Exploration

**Crate:** `tools`  
**Status:** Enhanced in claw-code-latest (1 file → 2 files)  
**Purpose:** Tool execution engine, tool registry, and lane completion detection  
**Total Lines:** ~3,000+ (lib.rs: ~2,800 + lane_completion.rs: 182)  
**Files:** 
- claw-code: `src/lib.rs` (~1,100 lines)
- claw-code-latest: `src/lib.rs` (~2,800 lines), `src/lane_completion.rs` (182 lines)

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [Tool Registry Types (Lines 71-97)](#tool-registry-types)
3. [ToolSpec Definitions (Lines 99-700+)](#toolspec-definitions)
4. [GlobalToolRegistry (Lines 107-367)](#globaltoolregistry)
5. [Tool Execution (Lines 368-2000+)](#tool-execution)
6. [Lane Completion Detection (lane_completion.rs)](#lane-completion-detection)
7. [Integration Points](#integration-points)

---

## Module Overview

The tools crate is the **tool execution engine** for claw-code. It provides:

- **Tool specifications** - JSON schemas for all tools
- **Tool execution** - Dispatch and execute tool calls
- **Plugin tool integration** - Merge plugin tools with builtins
- **Permission enforcement** - Gate tools by permission mode
- **Lane completion detection** - Auto-complete lanes when done

### Key Differences: claw-code vs claw-code-latest

| Feature | claw-code | claw-code-latest |
|---------|-----------|------------------|
| Files | 1 (lib.rs) | 2 (lib.rs + lane_completion.rs) |
| Tool count | ~18 tools | ~20+ tools |
| Plugin support | None | Full plugin tool integration |
| Runtime tools | Basic | Extended with LSP, MCP, workers |
| Permission enforcement | Basic | Full PermissionEnforcer integration |

---

## Tool Registry Types (Lines 71-97)

### ToolManifestEntry (Lines 71-74)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifestEntry {
    pub name: String,
    pub source: ToolSource,
}
```

### ToolSource Enum (Lines 76-80)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    Base,
    Conditional,
}
```

**Variants:**
| Variant | Description |
|---------|-------------|
| `Base` | Always available |
| `Conditional` | Behind feature flag |

### ToolRegistry (Lines 82-97)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolRegistry {
    entries: Vec<ToolManifestEntry>,
}
```

**Methods:**
- `new()` - Create from entries
- `entries()` - Get slice of entries

---

## ToolSpec Definitions (Lines 99-700+)

### ToolSpec Struct (Lines 100-105)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
}
```

### MVP Tool Specs (Lines 383-700+)

The `mvp_tool_specs()` function defines all built-in tools:

#### Core File Operations

| Tool | Permission | Description |
|------|------------|-------------|
| `bash` | DangerFullAccess | Execute shell commands |
| `read_file` | ReadOnly | Read file contents |
| `write_file` | WorkspaceWrite | Write file contents |
| `edit_file` | WorkspaceWrite | Replace text in files |

#### Search Operations

| Tool | Permission | Description |
|------|------------|-------------|
| `glob_search` | ReadOnly | Find files by glob pattern |
| `grep_search` | ReadOnly | Search file contents with regex |

#### Web Operations

| Tool | Permission | Description |
|------|------------|-------------|
| `WebFetch` | ReadOnly | Fetch URL and answer prompt |
| `WebSearch` | ReadOnly | Search web for information |

#### Session Management

| Tool | Permission | Description |
|------|------------|-------------|
| `TodoWrite` | WorkspaceWrite | Update task list |
| `Skill` | ReadOnly | Load skill definitions |
| `Agent` | DangerFullAccess | Launch subagent |

#### Additional Tools (claw-code-latest)

| Tool | Permission | Description |
|------|------------|-------------|
| `ToolSearch` | ReadOnly | Search for tools by name |
| `NotebookEdit` | WorkspaceWrite | Edit Jupyter notebooks |
| `Sleep` | ReadOnly | Wait without holding shell |
| `SendUserMessage`/`Brief` | ReadOnly | Send message to user |
| `Config` | WorkspaceWrite | Get/set settings |
| `StructuredOutput` | ReadOnly | Return structured output |
| `REPL` | DangerFullAccess | Execute code in REPL |
| `PowerShell` | DangerFullAccess | Execute PowerShell commands |

### Tool Input Schemas

Each tool has a JSON schema defining its inputs:

```rust
ToolSpec {
    name: "read_file",
    description: "Read a text file from the workspace.",
    input_schema: json!({
        "type": "object",
        "properties": {
            "path": { "type": "string" },
            "offset": { "type": "integer", "minimum": 0 },
            "limit": { "type": "integer", "minimum": 1 }
        },
        "required": ["path"],
        "additionalProperties": false
    }),
    required_permission: PermissionMode::ReadOnly,
}
```

---

## GlobalToolRegistry (Lines 107-367)

### Structure (Lines 107-112)

```rust
#[derive(Debug, Clone)]
pub struct GlobalToolRegistry {
    plugin_tools: Vec<PluginTool>,
    runtime_tools: Vec<RuntimeToolDefinition>,
    enforcer: Option<PermissionEnforcer>,
}
```

**Fields:**
| Field | Type | Purpose |
|-------|------|---------|
| `plugin_tools` | `Vec<PluginTool>` | Plugin-provided tools |
| `runtime_tools` | `Vec<RuntimeToolDefinition>` | Runtime-added tools |
| `enforcer` | `Option<PermissionEnforcer>` | Permission gate |

### Builder Methods

#### `builtin()` (Lines 124-130)
```rust
pub fn builtin() -> Self {
    Self {
        plugin_tools: Vec::new(),
        runtime_tools: Vec::new(),
        enforcer: None,
    }
}
```
Creates empty registry.

#### `with_plugin_tools()` (Lines 132-156)
```rust
pub fn with_plugin_tools(plugin_tools: Vec<PluginTool>) -> Result<Self, String> {
    let builtin_names = mvp_tool_specs()
        .into_iter()
        .map(|spec| spec.name.to_string())
        .collect::<BTreeSet<_>>();
    let mut seen_plugin_names = BTreeSet::new();

    for tool in &plugin_tools {
        let name = tool.definition().name.clone();
        if builtin_names.contains(&name) {
            return Err(format!(
                "plugin tool `{name}` conflicts with a built-in tool name"
            ));
        }
        if !seen_plugin_names.insert(name.clone()) {
            return Err(format!("duplicate plugin tool name `{name}`"));
        }
    }

    Ok(Self {
        plugin_tools,
        runtime_tools: Vec::new(),
        enforcer: None,
    })
}
```

**Validates:**
1. No conflict with builtin tool names
2. No duplicate plugin tool names

#### `with_runtime_tools()` (Lines 158-183)
```rust
pub fn with_runtime_tools(
    mut self,
    runtime_tools: Vec<RuntimeToolDefinition>,
) -> Result<Self, String> {
    let mut seen_names = mvp_tool_specs()
        .into_iter()
        .map(|spec| spec.name.to_string())
        .chain(
            self.plugin_tools
                .iter()
                .map(|tool| tool.definition().name.clone()),
        )
        .collect::<BTreeSet<_>>();

    for tool in &runtime_tools {
        if !seen_names.insert(tool.name.clone()) {
            return Err(format!(
                "runtime tool `{}` conflicts with an existing tool name",
                tool.name
            ));
        }
    }

    self.runtime_tools = runtime_tools;
    Ok(self)
}
```

**Validates:** No conflict with existing tool names.

#### `with_enforcer()` (Lines 185-189)
```rust
pub fn with_enforcer(mut self, enforcer: PermissionEnforcer) -> Self {
    self.set_enforcer(enforcer);
    self
}
```

### Tool Dispatch Methods

#### `definitions()` (Lines 245-277)
```rust
pub fn definitions(&self, allowed_tools: Option<&BTreeSet<String>>) -> Vec<ToolDefinition> {
    let builtin = mvp_tool_specs()
        .into_iter()
        .filter(|spec| allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)))
        .map(|spec| ToolDefinition {
            name: spec.name.to_string(),
            description: Some(spec.description.to_string()),
            input_schema: spec.input_schema,
        });
    let runtime = self.runtime_tools.iter()...;
    let plugin = self.plugin_tools.iter()...;
    builtin.chain(runtime).chain(plugin).collect()
}
```

Returns tool definitions filtered by allowed tools list.

#### `permission_specs()` (Lines 279-305)
```rust
pub fn permission_specs(
    &self,
    allowed_tools: Option<&BTreeSet<String>>,
) -> Result<Vec<(String, PermissionMode)>, String> {
    // Returns (tool_name, required_permission) pairs
}
```

#### `execute()` (Lines 338-348)
```rust
pub fn execute(&self, name: &str, input: &Value) -> Result<String, String> {
    if mvp_tool_specs().iter().any(|spec| spec.name == name) {
        return execute_tool_with_enforcer(self.enforcer.as_ref(), name, input);
    }
    self.plugin_tools
        .iter()
        .find(|tool| tool.definition().name == name)
        .ok_or_else(|| format!("unsupported tool: {name}"))?
        .execute(input)
        .map_err(|error| error.to_string())
}
```

**Dispatch logic:**
1. Check if builtin tool → execute with enforcer
2. Check if plugin tool → execute plugin tool
3. Return error if not found

### Helper Functions

#### `normalize_tool_name()` (Lines 369-371)
```rust
fn normalize_tool_name(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}
```
Normalizes tool names for comparison (handles `read-file` vs `read_file`).

#### `permission_mode_from_plugin()` (Lines 373-380)
```rust
fn permission_mode_from_plugin(value: &str) -> Result<PermissionMode, String> {
    match value {
        "read-only" => Ok(PermissionMode::ReadOnly),
        "workspace-write" => Ok(PermissionMode::WorkspaceWrite),
        "danger-full-access" => Ok(PermissionMode::DangerFullAccess),
        other => Err(format!("unsupported plugin permission: {other}")),
    }
}
```
Converts plugin permission strings to PermissionMode.

---

## Tool Execution (Lines 368-2000+)

### Execute Function Pattern

Each tool has:
1. Input struct with serde Deserialize
2. Execution function
3. JSON output formatting

### Example: Bash Tool

```rust
fn run_bash(input: BashCommandInput) -> Result<String, String> {
    serde_json::to_string_pretty(
        execute_bash(input).map_err(|error| error.to_string())?
    ).map_err(|error| error.to_string())
}
```

### Example: Read/Write File Tools

```rust
fn run_read_file(input: ReadFileInput) -> Result<String, String> {
    to_pretty_json(read_file(&input.path, input.offset, input.limit).map_err(io_to_string)?)
}

fn run_write_file(input: WriteFileInput) -> Result<String, String> {
    to_pretty_json(write_file(&input.path, &input.content).map_err(io_to_string)?)
}
```

### Global Registries (Lines 34-68)

claw-code-latest adds static registries for runtime features:

```rust
fn global_lsp_registry() -> &'static LspRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<LspRegistry> = OnceLock::new();
    REGISTRY.get_or_init(LspRegistry::new)
}

fn global_mcp_registry() -> &'static McpToolRegistry {
    static REGISTRY: OnceLock<McpToolRegistry> = OnceLock::new();
    REGISTRY.get_or_init(McpToolRegistry::new)
}

fn global_team_registry() -> &'static TeamRegistry { ... }
fn global_cron_registry() -> &'static CronRegistry { ... }
fn global_task_registry() -> &'static TaskRegistry { ... }
fn global_worker_registry() -> &'static WorkerRegistry { ... }
```

**Runtime tools available in claw-code-latest:**
- LSP (Language Server Protocol)
- MCP (Model Context Protocol)
- Team registry (cron jobs)
- Task registry (task tracking)
- Worker registry (background workers)

---

## Lane Completion Detection (lane_completion.rs)

**NEW in claw-code-latest** - Automatic lane completion when sessions finish successfully.

### Module Purpose (Lines 1-10)

```rust
//! Lane completion detector — automatically marks lanes as completed when
//! session finishes successfully with green tests and pushed code.
```

### detect_lane_completion() (Lines 23-66)

```rust
pub(crate) fn detect_lane_completion(
    output: &AgentOutput,
    test_green: bool,
    has_pushed: bool,
) -> Option<LaneContext> {
    // Must be finished without errors
    if output.error.is_some() {
        return None;
    }

    // Must have finished status
    if !output.status.eq_ignore_ascii_case("completed")
        && !output.status.eq_ignore_ascii_case("finished")
    {
        return None;
    }

    // Must have no current blocker
    if output.current_blocker.is_some() {
        return None;
    }

    // Must have green tests
    if !test_green {
        return None;
    }

    // Must have pushed code
    if !has_pushed {
        return None;
    }

    // All conditions met — create completed context
    Some(LaneContext {
        lane_id: output.agent_id.clone(),
        green_level: 3, // Workspace green
        branch_freshness: std::time::Duration::from_secs(0),
        blocker: LaneBlocker::None,
        review_status: ReviewStatus::Approved,
        diff_scope: runtime::DiffScope::Scoped,
        completed: true,
        reconciled: false,
    })
}
```

**Completion conditions:**
1. No errors
2. Status is "Finished" or "Completed"
3. No current blocker
4. Tests are green
5. Code has been pushed

### evaluate_completed_lane() (Lines 70-90)

```rust
pub(crate) fn evaluate_completed_lane(context: &LaneContext) -> Vec<PolicyAction> {
    let engine = PolicyEngine::new(vec![
        PolicyRule::new(
            "closeout-completed-lane",
            PolicyCondition::And(vec![
                PolicyCondition::LaneCompleted,
                PolicyCondition::GreenAt { level: 3 },
            ]),
            PolicyAction::CloseoutLane,
            10,
        ),
        PolicyRule::new(
            "cleanup-completed-session",
            PolicyCondition::LaneCompleted,
            PolicyAction::CleanupSession,
            5,
        ),
    ]);

    evaluate(&engine, context)
}
```

**Policy actions triggered:**
1. `CloseoutLane` - Mark lane as complete
2. `CleanupSession` - Clean up session resources

### Unit Tests (Lines 92-181)

Tests verify:
- Completion detected when all conditions met
- No completion when error present
- No completion when not finished
- No completion when tests not green
- No completion when not pushed
- Policy evaluation triggers correct actions

---

## Integration Points

### Upstream Dependencies

| Crate | Usage |
|-------|-------|
| `api` | ToolDefinition, MessageRequest, ProviderClient |
| `runtime` | Tool execution functions, PermissionEnforcer |
| `plugins` | PluginTool (claw-code-latest only) |
| `reqwest` | HTTP client for web tools |

### Downstream Dependents

| Crate | How it uses tools |
|-------|-------------------|
| `rusty-claude-cli` | Tool execution in REPL |
| `runtime` | ToolExecutor trait implementation |
| `compat-harness` | Tool manifest extraction |

### Tool Execution Flow

```
User input → Claude API → Tool call → GlobalToolRegistry::execute()
                                            ↓
                              ┌─────────────┼─────────────┐
                              ↓             ↓             ↓
                        Builtin tool  Plugin tool   Runtime tool
                              ↓             ↓             ↓
                        Permission    Plugin binary   LSP/MCP/Worker
                          check         execution       registry
```

---

## Summary

The tools crate is the **tool execution backbone** with:

| Component | claw-code | claw-code-latest |
|-----------|-----------|------------------|
| Tool specs | ~18 tools | ~20+ tools |
| Plugin support | None | Full integration |
| Runtime tools | Basic | LSP, MCP, Teams, Workers |
| Lane completion | N/A | Auto-detection |
| Permission enforcement | Basic | Full enforcer |

**Key additions in claw-code-latest:**

1. **Plugin tool integration** - External tools from plugins
2. **Runtime tool extensibility** - Dynamic tool registration
3. **Lane completion detection** - Automatic lane lifecycle
4. **Enhanced permission enforcement** - PermissionEnforcer integration
5. **Global registries** - LSP, MCP, Teams, Cron, Tasks, Workers
