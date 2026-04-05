# Claw-Code Tool System Architecture

## Executive Summary

Claw-Code implements a sophisticated, security-first tool system that enables AI assistants to interact with the external world through a carefully controlled set of operations. This document provides a comprehensive deep-dive into the architecture, covering tool definition, registration, execution pipelines, result processing, agentic loop integration, and the permission/authorization model.

**Source Reference:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/`

---

## Table of Contents

1. [Overview](#overview)
2. [Tool Definition and Schema](#tool-definition-and-schema)
3. [Tool Registration and Discovery](#tool-registration-and-discovery)
4. [Tool Execution Pipeline](#tool-execution-pipeline)
5. [Tool Result Processing and Formatting](#tool-result-processing-and-formatting)
6. [Agentic Loop Integration](#agentic-loop-integration)
7. [Tool Permissions and Authorization](#tool-permissions-and-authorization)
8. [MCP Tool Integration](#mcp-tool-integration)
9. [Architecture Diagrams](#architecture-diagrams)

---

## 1. Overview

### 1.1 The Tool System's Role

The tool system serves as the bridge between the AI model and the external environment. It transforms natural language intentions into concrete actions while maintaining strict security boundaries. The architecture follows a layered approach:

```
┌─────────────────────────────────────────────────────────────────┐
│                    AI Model (Anthropic API)                     │
│                    Generates Tool Use Requests                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Conversation Runtime                         │
│  conversation.rs: Tool use extraction, permission checking,     │
│                   hook execution, result injection              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Tool Executor Layer                          │
│  tools/lib.rs: Tool dispatch, input validation, handler calls  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Tool Implementation Layer                    │
│  - bash.rs: Shell execution with sandboxing                     │
│  - file_ops.rs: File operations (read/write/edit/glob/grep)     │
│  - mcp_stdio.rs: MCP protocol handling                          │
│  - Other specialized tools                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Operating System                             │
│  Linux with unshare() sandboxing, namespace isolation           │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Design Principles

The tool system is built on several core principles:

| Principle | Description |
|-----------|-------------|
| **Defense in Depth** | Multiple layers of security (permissions, hooks, sandboxing, input validation) |
| **Explicit Permission** | Tools require explicit permission modes; escalation requires approval |
| **Fail-Safe Defaults** | Unknown tools are rejected; sandboxing enabled by default |
| **Auditability** | All tool uses are logged with full input/output capture |
| **Graceful Degradation** | Sandboxing falls back gracefully when unavailable |

---

## 2. Tool Definition and Schema

### 2.1 Tool Specification Structure

Tools in claw-code are defined using the `ToolSpec` structure (tools/lib.rs:50-56):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,  // JSON Schema
    pub required_permission: PermissionMode,
}
```

### 2.2 Tool Registration via `mvp_tool_specs()`

The central tool registry is defined in `mvp_tool_specs()` (tools/lib.rs:60-380), which returns a vector of all available tools. Each tool specifies:

1. **Name**: The identifier used in tool use requests
2. **Description**: Human-readable purpose
3. **Input Schema**: JSON Schema defining valid inputs
4. **Required Permission**: Minimum permission mode needed

**Example Tool Definition:**

```rust
ToolSpec {
    name: "bash",
    description: "Execute a shell command in the current workspace.",
    input_schema: json!({
        "type": "object",
        "properties": {
            "command": { "type": "string" },
            "timeout": { "type": "integer", "minimum": 1 },
            "description": { "type": "string" },
            "run_in_background": { "type": "boolean" },
            "dangerouslyDisableSandbox": { "type": "boolean" }
        },
        "required": ["command"],
        "additionalProperties": false
    }),
    required_permission: PermissionMode::DangerFullAccess,
}
```

### 2.3 Permission Mode Hierarchy

Tools declare their required permission level:

```rust
pub enum PermissionMode {
    ReadOnly,           // Read-only operations (read_file, glob_search, grep_search)
    WorkspaceWrite,     // Workspace modifications (write_file, edit_file)
    DangerFullAccess,   // Full system access (bash, REPL, PowerShell)
    Prompt,             // Requires explicit user prompt
    Allow,              // Always allowed (bypasses checks)
}
```

**Permission Ordering:**
```
Allow > DangerFullAccess > WorkspaceWrite > ReadOnly > Prompt
```

### 2.4 Complete Tool Inventory

| Tool | Permission | Description |
|------|------------|-------------|
| `bash` | DangerFullAccess | Shell command execution with sandboxing |
| `read_file` | ReadOnly | Read text files with offset/limit |
| `write_file` | WorkspaceWrite | Create/overwrite files |
| `edit_file` | WorkspaceWrite | Replace text in files |
| `glob_search` | ReadOnly | Find files by glob pattern |
| `grep_search` | ReadOnly | Regex search across files |
| `WebFetch` | ReadOnly | Fetch URL content with summarization |
| `WebSearch` | ReadOnly | Web search with domain filtering |
| `TodoWrite` | WorkspaceWrite | Task list management |
| `Skill` | ReadOnly | Load local skill definitions |
| `Agent` | DangerFullAccess | Spawn sub-agents |
| `ToolSearch` | ReadOnly | Find tools by name/keywords |
| `NotebookEdit` | WorkspaceWrite | Jupyter notebook cell editing |
| `Sleep` | ReadOnly | Timed delay without holding process |
| `SendUserMessage` | ReadOnly | Send messages to user |
| `Config` | WorkspaceWrite | Get/set configuration |
| `StructuredOutput` | ReadOnly | Return structured JSON output |
| `REPL` | DangerFullAccess | Execute code in interpreted languages |
| `PowerShell` | DangerFullAccess | PowerShell command execution |

---

## 3. Tool Registration and Discovery

### 3.1 Static Tool Registry

The primary tool registry is static and defined at compile time in `tools/lib.rs`. The `ToolRegistry` struct (lines 33-48) wraps the entries:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolRegistry {
    entries: Vec<ToolManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifestEntry {
    pub name: String,
    pub source: ToolSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    Base,       // Built-in tools
    Conditional // Platform-specific or feature-gated tools
}
```

### 3.2 Dynamic Tool Discovery via MCP

MCP (Model Context Protocol) servers provide dynamic tool discovery:

```rust
// mcp_stdio.rs:92-118
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpListToolsResult {
    pub tools: Vec<McpTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpTool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "inputSchema", skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<JsonValue>,
}
```

**MCP Tool Naming Convention:**

```rust
// mcp.rs:25-36
pub fn mcp_tool_prefix(server_name: &str) -> String {
    format!("mcp__{}__", normalize_name_for_mcp(server_name))
}

pub fn mcp_tool_name(server_name: &str, tool_name: &str) -> String {
    format!(
        "{}{}",
        mcp_tool_prefix(server_name),
        normalize_name_for_mcp(tool_name)
    )
}
```

This produces names like: `mcp__filesystem__read_file`

### 3.3 Tool Filtering by Permission Context

Tools can be filtered based on the current permission context:

```python
# src/permissions.py:6-20
@dataclass(frozen=True)
class ToolPermissionContext:
    deny_names: frozenset[str] = field(default_factory=frozenset)
    deny_prefixes: tuple[str, ...] = ()

    def blocks(self, tool_name: str) -> bool:
        lowered = tool_name.lower()
        return lowered in self.deny_names or any(
            lowered.startswith(prefix) for prefix in self.deny_prefixes
        )
```

---

## 4. Tool Execution Pipeline

### 4.1 High-Level Execution Flow

The tool execution pipeline flows through multiple stages:

```
┌──────────────────────────────────────────────────────────────────┐
│ Stage 1: Tool Use Extraction (conversation.rs:200-209)           │
│ - Parse assistant response for ToolUse blocks                    │
│ - Extract (id, name, input) tuples                               │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│ Stage 2: Permission Authorization (conversation.rs:219-224)      │
│ - Check tool against PermissionPolicy                            │
│ - Prompt user if escalation required                             │
│ - Allow or deny                                                  │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│ Stage 3: Pre-Tool Hook (conversation.rs:228-237)                 │
│ - Execute PreToolUse hooks                                       │
│ - Hooks can deny, warn, or allow                                 │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│ Stage 4: Tool Execution (conversation.rs:238-242)                │
│ - Dispatch to appropriate handler via execute_tool()             │
│ - Catch and handle errors                                        │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│ Stage 5: Post-Tool Hook (conversation.rs:245-255)                │
│ - Execute PostToolUse hooks                                      │
│ - Modify output if needed                                        │
│ - Hooks can mark as error                                        │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│ Stage 6: Result Injection (conversation.rs:257-262)              │
│ - Create tool_result message                                     │
│ - Append to session history                                      │
│ - Continue or break loop                                         │
└──────────────────────────────────────────────────────────────────┘
```

### 4.2 Tool Dispatch Mechanism

The `execute_tool()` function (tools/lib.rs:383-406) dispatches to the appropriate handler:

```rust
pub fn execute_tool(name: &str, input: &Value) -> Result<String, String> {
    match name {
        "bash" => from_value::<BashCommandInput>(input).and_then(run_bash),
        "read_file" => from_value::<ReadFileInput>(input).and_then(run_read_file),
        "write_file" => from_value::<WriteFileInput>(input).and_then(run_write_file),
        "edit_file" => from_value::<EditFileInput>(input).and_then(run_edit_file),
        "glob_search" => from_value::<GlobSearchInputValue>(input).and_then(run_glob_search),
        "grep_search" => from_value::<GrepSearchInput>(input).and_then(run_grep_search),
        "WebFetch" => from_value::<WebFetchInput>(input).and_then(run_web_fetch),
        "WebSearch" => from_value::<WebSearchInput>(input).and_then(run_web_search),
        "TodoWrite" => from_value::<TodoWriteInput>(input).and_then(run_todo_write),
        "Skill" => from_value::<SkillInput>(input).and_then(run_skill),
        "Agent" => from_value::<AgentInput>(input).and_then(run_agent),
        "ToolSearch" => from_value::<ToolSearchInput>(input).and_then(run_tool_search),
        "NotebookEdit" => from_value::<NotebookEditInput>(input).and_then(run_notebook_edit),
        "Sleep" => from_value::<SleepInput>(input).and_then(run_sleep),
        "SendUserMessage" | "Brief" => from_value::<BriefInput>(input).and_then(run_brief),
        "Config" => from_value::<ConfigInput>(input).and_then(run_config),
        // ... more tools
        other => Err(format!("Unknown tool: {}", other)),
    }
}
```

### 4.3 Input Deserialization and Validation

Each tool handler uses type-safe deserialization:

```rust
#[derive(Debug, Deserialize)]
struct BashCommandInput {
    command: String,
    timeout: Option<u64>,
    description: Option<String>,
    #[serde(rename = "run_in_background")]
    run_in_background: Option<bool>,
    #[serde(rename = "dangerouslyDisableSandbox")]
    dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "namespaceRestrictions")]
    namespace_restrictions: Option<bool>,
    #[serde(rename = "isolateNetwork")]
    isolate_network: Option<bool>,
    #[serde(rename = "filesystemMode")]
    filesystem_mode: Option<FilesystemIsolationMode>,
    #[serde(rename = "allowedMounts")]
    allowed_mounts: Option<Vec<String>>,
}
```

**Validation Pattern:**
```rust
fn run_bash(input: BashCommandInput) -> Result<String, String> {
    // Input already validated by serde against the struct
    let output = execute_bash(input)
        .map_err(|e| e.to_string())?;
    to_pretty_json(&output)
}
```

### 4.4 The ToolExecutor Trait

The `ToolExecutor` trait (conversation.rs:38-40) abstracts tool execution:

```rust
pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}
```

**StaticToolExecutor Implementation (conversation.rs:428-456):**
```rust
#[derive(Default)]
pub struct StaticToolExecutor {
    handlers: BTreeMap<String, ToolHandler>,
}

type ToolHandler = Box<dyn FnMut(&str) -> Result<String, ToolError>>;

impl ToolExecutor for StaticToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        self.handlers
            .get_mut(tool_name)
            .ok_or_else(|| ToolError::new(format!("unknown tool: {tool_name}")))?(input)
    }
}
```

---

## 5. Tool Result Processing and Formatting

### 5.1 Result Structure

Tool results are returned as formatted JSON strings:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BashCommandOutput {
    pub stdout: String,
    pub stderr: String,
    #[serde(rename = "rawOutputPath")]
    pub raw_output_path: Option<String>,
    pub interrupted: bool,
    #[serde(rename = "isImage")]
    pub is_image: Option<bool>,
    #[serde(rename = "backgroundTaskId")]
    pub background_task_id: Option<String>,
    #[serde(rename = "returnCodeInterpretation")]
    pub return_code_interpretation: Option<String>,
    #[serde(rename = "noOutputExpected")]
    pub no_output_expected: Option<bool>,
    #[serde(rename = "sandboxStatus")]
    pub sandbox_status: Option<SandboxStatus>,
    // ... more fields
}
```

### 5.2 JSON Serialization

Results are serialized with consistent formatting:

```rust
fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| e.to_string())
}
```

### 5.3 Hook Feedback Integration

Hook feedback is merged into tool output:

```rust
// conversation.rs:408-424
fn merge_hook_feedback(messages: &[String], output: String, denied: bool) -> String {
    if messages.is_empty() {
        return output;
    }

    let mut sections = Vec::new();
    if !output.trim().is_empty() {
        sections.push(output);
    }
    let label = if denied {
        "Hook feedback (denied)"
    } else {
        "Hook feedback"
    };
    sections.push(format!("{label}:\n{}", messages.join("\n")));
    sections.join("\n\n")
}
```

### 5.4 Error Handling

Errors are captured and formatted consistently:

```rust
// conversation.rs:42-62
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError {
    message: String,
}

impl ToolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}
```

---

## 6. Agentic Loop Integration

### 6.1 The ConversationRuntime

The `ConversationRuntime` (conversation.rs:100-110) orchestrates the agentic loop:

```rust
pub struct ConversationRuntime<C, T> {
    session: Session,
    api_client: C,              // API client for model communication
    tool_executor: T,           // Tool execution handler
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    max_iterations: usize,
    usage_tracker: UsageTracker,
    hook_runner: HookRunner,
    auto_compaction_input_tokens_threshold: u32,
}
```

### 6.2 The Run Turn Loop

The main execution loop (conversation.rs:170-283):

```rust
pub fn run_turn(
    &mut self,
    user_input: impl Into<String>,
    mut prompter: Option<&mut dyn PermissionPrompter>,
) -> Result<TurnSummary, RuntimeError> {
    // 1. Add user message to session
    self.session.messages.push(ConversationMessage::user_text(user_input.into()));

    let mut assistant_messages = Vec::new();
    let mut tool_results = Vec::new();
    let mut iterations = 0;

    loop {
        iterations += 1;
        if iterations > self.max_iterations {
            return Err(RuntimeError::new("conversation loop exceeded maximum iterations"));
        }

        // 2. Build API request with current session state
        let request = ApiRequest {
            system_prompt: self.system_prompt.clone(),
            messages: self.session.messages.clone(),
        };

        // 3. Stream response from model
        let events = self.api_client.stream(request)?;
        let (assistant_message, usage) = build_assistant_message(events)?;

        // 4. Track token usage
        if let Some(usage) = usage {
            self.usage_tracker.record(usage);
        }

        // 5. Extract pending tool uses
        let pending_tool_uses = assistant_message
            .blocks
            .iter()
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.clone(), name.clone(), input.clone()))
                }
                _ => None,
            })
            .collect();

        // 6. Add assistant message to session
        self.session.messages.push(assistant_message.clone());
        assistant_messages.push(assistant_message);

        // 7. Break if no tools to execute
        if pending_tool_uses.is_empty() {
            break;
        }

        // 8. Execute each tool
        for (tool_use_id, tool_name, input) in pending_tool_uses {
            // ... permission check, hooks, execution, result injection
        }
    }

    // 9. Auto-compaction if needed
    let auto_compaction = self.maybe_auto_compact();

    Ok(TurnSummary {
        assistant_messages,
        tool_results,
        iterations,
        usage: self.usage_tracker.cumulative_usage(),
        auto_compaction,
    })
}
```

### 6.3 Tool Use Content Blocks

Tool uses are represented as content blocks:

```rust
// session.rs (via conversation.rs imports)
pub enum ContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        tool_use_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },
}
```

### 6.4 Turn Summary

Each turn produces a summary:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSummary {
    pub assistant_messages: Vec<ConversationMessage>,
    pub tool_results: Vec<ConversationMessage>,
    pub iterations: usize,
    pub usage: TokenUsage,
    pub auto_compaction: Option<AutoCompactionEvent>,
}
```

---

## 7. Tool Permissions and Authorization

### 7.1 PermissionPolicy Structure

The permission system (permissions.rs:49-135) enforces access control:

```rust
pub struct PermissionPolicy {
    active_mode: PermissionMode,
    tool_requirements: BTreeMap<String, PermissionMode>,
}
```

### 7.2 Authorization Flow

```rust
impl PermissionPolicy {
    pub fn authorize(
        &self,
        tool_name: &str,
        input: &str,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> PermissionOutcome {
        let current_mode = self.active_mode();
        let required_mode = self.required_mode_for(tool_name);

        // Allow if mode is Allow or meets requirement
        if current_mode == PermissionMode::Allow || current_mode >= required_mode {
            return PermissionOutcome::Allow;
        }

        // Prompt for specific escalation scenarios
        if current_mode == PermissionMode::Prompt
            || (current_mode == PermissionMode::WorkspaceWrite
                && required_mode == PermissionMode::DangerFullAccess)
        {
            return match prompter.as_mut() {
                Some(prompter) => match prompter.decide(&PermissionRequest {
                    tool_name: tool_name.to_string(),
                    input: input.to_string(),
                    current_mode,
                    required_mode,
                }) {
                    PermissionPromptDecision::Allow => PermissionOutcome::Allow,
                    PermissionPromptDecision::Deny { reason } => PermissionOutcome::Deny { reason },
                },
                None => PermissionOutcome::Deny {
                    reason: format!("tool '{tool_name}' requires approval"),
                },
            };
        }

        PermissionOutcome::Deny {
            reason: format!(
                "tool '{tool_name}' requires {} permission; current mode is {}",
                required_mode.as_str(),
                current_mode.as_str()
            ),
        }
    }
}
```

### 7.3 Permission Modes Explained

| Mode | Behavior | Use Case |
|------|----------|----------|
| `ReadOnly` | Allows read-only tools | Safe exploration, code review |
| `WorkspaceWrite` | Allows file modifications | Code generation, refactoring |
| `DangerFullAccess` | Allows shell execution | DevOps, testing, deployment |
| `Prompt` | Requires explicit approval | High-risk operations |
| `Allow` | Bypasses all checks | Trusted automation |

### 7.4 Permission Prompter Trait

```rust
pub trait PermissionPrompter {
    fn decide(&mut self, request: &PermissionRequest) -> PermissionPromptDecision;
}

pub enum PermissionPromptDecision {
    Allow,
    Deny { reason: String },
}
```

---

## 8. MCP Tool Integration

### 8.1 MCP Transport Types

Claw-Code supports multiple MCP transports (config.rs:65-73):

```rust
pub enum McpTransport {
    Stdio,          // Standard I/O (process-based)
    Sse,            // Server-Sent Events
    Http,           // HTTP polling
    Ws,             // WebSocket
    Sdk,            // Native SDK
    ClaudeAiProxy,  // Claude.ai proxy
}
```

### 8.2 MCP Server Configuration

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpRemoteServerConfig),
    Http(McpRemoteServerConfig),
    Ws(McpWebSocketServerConfig),
    Sdk(McpSdkServerConfig),
    ClaudeAiProxy(McpClaudeAiProxyServerConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpStdioServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}
```

### 8.3 MCP Tool Call Flow

```rust
// mcp_stdio.rs:120-149
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpToolCallResult {
    #[serde(default)]
    pub content: Vec<McpToolCallContent>,
    #[serde(default)]
    pub structured_content: Option<JsonValue>,
    #[serde(default)]
    pub is_error: Option<bool>,
}
```

### 8.4 MCP Client Implementation

The MCP client (mcp_client.rs) handles the JSON-RPC protocol:

```rust
pub trait McpClientTransport {
    fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpClientError>;
}
```

---

## 9. Architecture Diagrams

### 9.1 Complete Tool Execution Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           ANTHROPIC API                                │
│                    (Model generates tool use)                          │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ SSE Stream
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         ApiClient Trait                                │
│                    AnthropicClient implementation                      │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ AssistantEvent::ToolUse
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       ConversationRuntime                              │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │ 1. Extract Tool Uses from ContentBlocks                         │   │
│  │ 2. Check PermissionPolicy.authorize()                           │   │
│  │ 3. Run HookRunner.run_pre_tool_use()                            │   │
│  │ 4. Execute via ToolExecutor.execute()                           │   │
│  │ 5. Run HookRunner.run_post_tool_use()                           │   │
│  │ 6. Create ToolResult ContentBlock                               │   │
│  └─────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
                    ▼               ▼               ▼
        ┌───────────────┐ ┌───────────────┐ ┌───────────────┐
        │    Built-in   │ │      MCP      │ │    Custom     │
        │     Tools     │ │     Tools     │ │    Handlers   │
        │               │ │               │ │               │
        │ - bash        │ │ - stdio       │ │ - Registered  │
        │ - read_file   │ │ - sse         │ │   via         │
        │ - write_file  │ │ - http        │ │   register()  │
        │ - edit_file   │ │ - ws          │ │               │
        │ - glob_search │ │ - sdk         │ │               │
        │ - grep_search │ │ - claude.ai   │ │               │
        │ - WebFetch    │ │   proxy       │ │               │
        │ - WebSearch   │ │               │ │               │
        │ - TodoWrite   │ │               │ │               │
        │ - Agent       │ │               │ │               │
        │ - ...         │ │               │ │               │
        └───────────────┘ └───────────────┘ └───────────────┘
                │                 │                 │
                │                 │                 │
                ▼                 ▼                 ▼
        ┌─────────────────────────────────────────────────────────┐
        │                    SECURITY LAYER                       │
        │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐   │
        │  │  Sandbox    │ │   Input     │ │    Permission   │   │
        │  │  (unshare)  │ │ Validation  │ │    Modes        │   │
        │  └─────────────┘ └─────────────┘ └─────────────────┘   │
        └─────────────────────────────────────────────────────────┘
                                    │
                                    ▼
        ┌─────────────────────────────────────────────────────────┐
        │                    OPERATING SYSTEM                     │
        │              Linux with namespace isolation             │
        └─────────────────────────────────────────────────────────┘
```

### 9.2 Permission Authorization Flow

```
                         ┌──────────────────────┐
                         │   Tool Use Request   │
                         │  (name, input JSON)  │
                         └──────────────────────┘
                                    │
                                    ▼
                         ┌──────────────────────┐
                         │  Get Required Mode   │
                         │  from ToolSpec       │
                         └──────────────────────┘
                                    │
                                    ▼
                         ┌──────────────────────┐
                         │  Compare with Active │
                         │     PermissionMode   │
                         └──────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    │                               │
           Active >= Required              Active < Required
                    │                               │
                    ▼                               ▼
         ┌──────────────────┐            ┌──────────────────┐
         │  ALLOW           │            │  Check if Prompt │
         │  Proceed to Hook │            │  Mode or Escalation
         └──────────────────┘            └──────────────────┘
                    │                               │
                    │                    ┌──────────┴──────────┐
                    │                    │                     │
                    │             Needs Prompt         Hard Deny
                    │                    │                     │
                    │                    ▼                     ▼
                    │         ┌──────────────────┐   ┌──────────────────┐
                    │         │  Invoke Prompter │   │  Return Deny     │
                    │         │  User Decision   │   │  Reason          │
                    │         └──────────────────┘   └──────────────────┘
                    │                    │
                    │         ┌──────────┴──────────┐
                    │         │                     │
                    │      Allow                  Deny
                    │         │                     │
                    └─────────┘                     │
                              │                     │
                              ▼                     │
                     ┌──────────────────────────────┘
                     │
                     ▼
          ┌─────────────────────┐
          │  Run Pre-Tool Hook  │
          └─────────────────────┘
```

### 9.3 MCP Tool Registration Flow

```
┌──────────────────────┐      ┌──────────────────────┐
│  MCP Server Config   │      │   MCP Client         │
│  (command, args)     │─────▶│   (stdio transport)  │
└──────────────────────┘      └──────────────────────┘
                                       │
                                       │ Initialize
                                       ▼
                              ┌──────────────────────┐
                              │  tools/list          │
                              │  JSON-RPC Request    │
                              └──────────────────────┘
                                       │
                                       │ Response
                                       ▼
                              ┌──────────────────────┐
                              │  McpListToolsResult  │
                              │  [McpTool, ...]      │
                              └──────────────────────┘
                                       │
                                       │ Prefix + Normalize
                                       ▼
                              ┌──────────────────────┐
                              │  mcp__{server}__{tool}
                              │  Registered in Tool  │
                              │  Executor            │
                              └──────────────────────┘
```

---

## 10. Key Source Files

| File | Purpose | Lines |
|------|---------|-------|
| `rust/crates/tools/src/lib.rs` | Main tool registry and dispatch | 3800+ |
| `rust/crates/runtime/src/conversation.rs` | Agentic loop, tool orchestration | 650+ |
| `rust/crates/runtime/src/permissions.rs` | Permission policy, authorization | 232 |
| `rust/crates/runtime/src/hooks.rs` | Pre/post tool hooks | 349 |
| `rust/crates/runtime/src/bash.rs` | Bash tool with sandboxing | 283 |
| `rust/crates/runtime/src/sandbox.rs` | Linux sandbox with unshare | 364 |
| `rust/crates/runtime/src/file_ops.rs` | File operations | 550 |
| `rust/crates/runtime/src/mcp_stdio.rs` | MCP protocol handling | 500+ |
| `rust/crates/runtime/src/mcp.rs` | MCP naming, signatures | 200+ |
| `rust/crates/runtime/src/config.rs` | Configuration, MCP server config | 400+ |
| `src/tools.py` | Python tool mirror definitions | 96 |
| `src/permissions.py` | Python permission context | 20 |

---

## 11. Summary

Claw-Code's tool system represents a mature, security-conscious approach to AI tool integration. Key architectural decisions include:

1. **Static typing for tool inputs** - All tools use strongly-typed input structs with serde validation
2. **Permission tiers** - Clear hierarchy from ReadOnly to DangerFullAccess
3. **Hook-based extensibility** - Pre and post hooks for audit/modify/deny
4. **Sandboxed execution** - Linux unshare() for bash isolation
5. **MCP extensibility** - Dynamic tool discovery via Model Context Protocol
6. **Graceful degradation** - Fallback behavior when sandboxing unavailable

The system is designed to be extended safely, with clear boundaries between the model's intent and system execution.

---

*Document generated from source analysis of claw-code repository.*
*Source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/*
