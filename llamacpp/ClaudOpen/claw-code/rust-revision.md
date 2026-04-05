# Claw Code Rust Revision Guide

Rust-specific patterns, crate structure, and implementation details for the Claw Code Rust port.

## Table of Contents

1. [Workspace Overview](#workspace-overview)
2. [Crate-by-Crate Deep Dive](#crate-by-crate-deep-dive)
3. [Key Rust Patterns](#key-rust-patterns)
4. [Async Runtime Design](#async-runtime-design)
5. [Error Handling Strategy](#error-handling-strategy)
6. [Memory Management](#memory-management)
7. [Testing Patterns](#testing-patterns)
8. [Build and Release](#build-and-release)

---

## Workspace Overview

### Workspace Structure

```
rust/
├── Cargo.toml              # Workspace root
├── Cargo.lock              # Dependency lockfile
├── README.md               # Rust-specific documentation
├── TUI-ENHANCEMENT-PLAN.md # Terminal UI enhancement roadmap
└── crates/
    ├── api/                # Anthropic API client
    ├── commands/           # Slash command registry
    ├── compat-harness/     # TypeScript manifest extraction
    ├── runtime/            # Core agentic loop
    ├── rusty-claude-cli/   # Main CLI binary
    └── tools/              # Tool implementations
```

### Workspace Configuration

```toml
# rust/Cargo.toml
[workspace]
members = ["crates/*"]
resolver = "2"  # Use edition 2021 resolver

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
publish = false  # Private workspace

[workspace.lints.rust]
unsafe_code = "forbid"  # No unsafe code allowed

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"  # Common pattern in Rust
missing_panics_doc = "allow"       # Acceptable for internal code
missing_errors_doc = "allow"       # Acceptable for internal code
```

### Crate Statistics

| Crate | LOC | Files | Primary Dependencies |
|-------|-----|-------|---------------------|
| `api` | ~500 | 5 | reqwest, tokio, serde |
| `commands` | ~620 | 1 | runtime |
| `compat-harness` | ~300 | 1 | commands, tools, runtime |
| `runtime` | ~5,300 | 20 | tokio, serde, regex, glob |
| `rusty-claude-cli` | ~4,500 | 4 | crossterm, rustyline, syntect |
| `tools` | ~800 | 1 | api, runtime, reqwest |
| **Total** | **~12,020** | **32** | - |

---

## Crate-by-Crate Deep Dive

### `api` Crate

**Path**: `rust/crates/api/`

**Purpose**: HTTP client for Anthropic API with SSE streaming support

#### Module Structure

```
api/src/
├── lib.rs      # Public API exports
├── client.rs   # AnthropicClient implementation
├── error.rs    # ApiError type
├── sse.rs      # SSE parser
└── types.rs    # Request/response types
```

#### Key Types

```rust
// src/types.rs

/// Anthropic API message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRequest {
    pub model: String,
    pub messages: Vec<InputMessage>,
    pub system: Option<Vec<String>>,
    pub max_tokens: u32,
    pub stream: bool,
    pub tools: Option<Vec<ToolDefinition>>,
    pub tool_choice: Option<ToolChoice>,
}

/// Input message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMessage {
    pub role: String,  // "user" or "assistant"
    pub content: Vec<InputContentBlock>,
}

/// Content block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InputContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: Vec<ToolResultContentBlock>,
    },
}

/// Streaming event types
#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart(MessageStartEvent),
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
    MessageDelta(MessageDeltaEvent),
    MessageStop(MessageStopEvent),
}

/// Token usage tracking
#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_write_input_tokens: Option<u32>,
}
```

#### Client Implementation

```rust
// src/client.rs

pub struct AnthropicClient {
    http_client: reqwest::Client,
    base_url: String,
    auth_source: AuthSource,
}

#[derive(Debug, Clone)]
pub enum AuthSource {
    ApiKey(String),
    OAuthBearer(String),
}

impl AnthropicClient {
    pub fn new(auth_source: AuthSource) -> Result<Self, ApiError> {
        let client = reqwest::Client::builder()
            .default_headers(Self::build_headers(&auth_source)?)
            .build()?;

        Ok(Self {
            http_client: client,
            base_url: read_base_url(),
            auth_source,
        })
    }

    pub async fn stream(
        &self,
        request: MessageRequest,
    ) -> Result<impl Stream<Item = Result<StreamEvent, ApiError>>, ApiError> {
        let response = self.http_client
            .post(format!("{}/v1/messages", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ApiError::from_response(response).await);
        }

        Ok(parse_sse_stream(response))
    }
}

/// OAuth token management
pub fn oauth_token_is_expired(token: &OAuthTokenSet) -> bool {
    match token.expires_at {
        Some(expires_at) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            // Refresh 5 minutes before expiration
            expires_at <= now + 300
        }
        None => false,
    }
}

/// Resolve auth source from environment
pub fn resolve_startup_auth_source() -> AuthSource {
    // 1. Check API key
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        return AuthSource::ApiKey(key);
    }

    // 2. Check OAuth token
    if let Ok(token) = load_oauth_credentials() {
        if !oauth_token_is_expired(&token) {
            return AuthSource::OAuthBearer(token.access_token);
        }
    }

    // 3. Fall back to API key (will fail if not set)
    AuthSource::ApiKey(String::new())
}
```

#### SSE Parser

```rust
// src/sse.rs

pub struct SseParser {
    buffer: String,
}

impl SseParser {
    pub fn new() -> Self {
        Self { buffer: String::new() }
    }

    pub fn parse_chunk(&mut self, chunk: &str) -> Vec<Result<StreamEvent, ApiError>> {
        self.buffer.push_str(chunk);

        let mut events = Vec::new();
        let mut start = 0;

        // Split on double newlines (SSE event separator)
        while let Some(end) = self.buffer[start..].find("\n\n") {
            let event_end = start + end;
            let event_data = &self.buffer[start..event_end];

            if let Ok(event) = parse_frame(event_data) {
                events.push(Ok(event));
            }

            start = event_end + 2;
        }

        // Remove processed data from buffer
        self.buffer = self.buffer[start..].to_string();

        events
    }
}

/// Parse a single SSE frame
pub fn parse_frame(frame: &str) -> Result<StreamEvent, ApiError> {
    // SSE format:
    // event: message_start
    // data: {"type": "message_start", ...}

    let mut event_type = None;
    let mut data = None;

    for line in frame.lines() {
        if let Some(rest) = line.strip_prefix("event: ") {
            event_type = Some(rest);
        } else if let Some(rest) = line.strip_prefix("data: ") {
            data = Some(rest);
        }
    }

    match (event_type, data) {
        (Some("message_start"), Some(json)) => {
            Ok(StreamEvent::MessageStart(serde_json::from_str(json)?))
        }
        (Some("content_block_start"), Some(json)) => {
            Ok(StreamEvent::ContentBlockStart(serde_json::from_str(json)?))
        }
        (Some("content_block_delta"), Some(json)) => {
            Ok(StreamEvent::ContentBlockDelta(serde_json::from_str(json)?))
        }
        // ... more event types
        _ => Err(ApiError::ParseError(format!("Unknown SSE event: {frame}"))),
    }
}
```

---

### `runtime` Crate

**Path**: `rust/crates/runtime/`

**Purpose**: Core agentic loop, configuration, sessions, MCP, permissions

#### Module Structure

```
runtime/src/
├── lib.rs              # Public exports
├── conversation.rs     # Core agentic loop (~800 lines)
├── config.rs           # Configuration loading (~900 lines)
├── session.rs          # Session management
├── permissions.rs      # Permission system
├── hooks.rs            # Hook execution
├── mcp.rs              # MCP configuration
├── mcp_client.rs       # MCP client trait
├── mcp_stdio.rs        # MCP stdio transport (~1,500 lines)
├── oauth.rs            # OAuth flow (~500 lines)
├── prompt.rs           # System prompts (~700 lines)
├── compact.rs          # Conversation compaction
├── file_ops.rs         # File operations
├── bash.rs             # Bash execution
├── usage.rs            # Token tracking
├── remote.rs           # Remote sessions
├── sandbox.rs          # Sandbox config
├── json.rs             # JSON utilities
└── bootstrap.rs        # Bootstrap phases
```

#### Conversation Runtime

```rust
// src/conversation.rs

/// Core runtime for managing conversations
pub struct ConversationRuntime<C, T> {
    session: Session,
    api_client: C,
    tool_executor: T,
    permission_policy: PermissionPolicy,
    system_prompt: Vec<String>,
    max_iterations: usize,
    usage_tracker: UsageTracker,
    hook_runner: HookRunner,
    auto_compaction_input_tokens_threshold: u32,
}

/// Trait for API clients (enables testing with mocks)
pub trait ApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError>;
}

/// Trait for tool executors (enables testing with mocks)
pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}

impl<C, T> ConversationRuntime<C, T>
where
    C: ApiClient,
    T: ToolExecutor,
{
    pub fn run_turn(
        &mut self,
        user_input: impl Into<String>,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> Result<TurnSummary, RuntimeError> {
        // Add user message
        self.session.messages.push(ConversationMessage::user_text(user_input.into()));

        let mut assistant_messages = Vec::new();
        let mut tool_results = Vec::new();
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(RuntimeError::new("Conversation loop exceeded maximum iterations"));
            }

            // Build and send API request
            let request = ApiRequest {
                system_prompt: self.system_prompt.clone(),
                messages: self.session.messages.clone(),
            };
            let events = self.api_client.stream(request)?;

            // Build assistant message from events
            let (assistant_message, usage) = build_assistant_message(events)?;
            if let Some(usage) = usage {
                self.usage_tracker.record(usage);
            }

            // Extract tool calls
            let pending_tool_uses = assistant_message.blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some((id.clone(), name.clone(), input.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();

            self.session.messages.push(assistant_message.clone());
            assistant_messages.push(assistant_message);

            // Exit if no tool calls
            if pending_tool_uses.is_empty() {
                break;
            }

            // Execute each tool
            for (tool_use_id, tool_name, input) in pending_tool_uses {
                let permission_outcome = if let Some(prompt) = prompter.as_mut() {
                    self.permission_policy.authorize(&tool_name, &input, Some(*prompt))
                } else {
                    self.permission_policy.authorize(&tool_name, &input, None)
                };

                let result_message = match permission_outcome {
                    PermissionOutcome::Allow => {
                        // Run pre-tool hooks
                        let pre_hook_result = self.hook_runner.run_pre_tool_use(&tool_name, &input);
                        if pre_hook_result.is_denied() {
                            // Hook denied the tool
                            ConversationMessage::tool_result(
                                tool_use_id,
                                tool_name,
                                format!("PreToolUse hook denied tool `{tool_name}`"),
                                true,
                            )
                        } else {
                            // Execute tool
                            let (mut output, mut is_error) = match self.tool_executor.execute(&tool_name, &input) {
                                Ok(output) => (output, false),
                                Err(error) => (error.to_string(), true),
                            };

                            // Merge hook feedback
                            output = merge_hook_feedback(pre_hook_result.messages(), output, false);

                            // Run post-tool hooks
                            let post_hook_result = self.hook_runner.run_post_tool_use(
                                &tool_name, &input, &output, is_error
                            );

                            ConversationMessage::tool_result(tool_use_id, tool_name, output, is_error)
                        }
                    }
                    PermissionOutcome::Deny => {
                        ConversationMessage::tool_result(
                            tool_use_id,
                            tool_name,
                            "Tool use was denied by permission policy".to_string(),
                            true,
                        )
                    }
                    PermissionOutcome::Skip => {
                        // Tool doesn't require permission
                        continue;
                    }
                };

                self.session.messages.push(result_message.clone());
                tool_results.push(result_message);
            }

            // Check for auto-compaction
            let auto_compaction = self.maybe_auto_compact()?;
        }

        Ok(TurnSummary {
            assistant_messages,
            tool_results,
            iterations,
            usage: self.usage_tracker.total(),
            auto_compaction,
        })
    }

    fn maybe_auto_compact(&mut self) -> Result<Option<AutoCompactionEvent>, RuntimeError> {
        let current_tokens = estimate_session_tokens(&self.session);
        if current_tokens >= self.auto_compaction_input_tokens_threshold {
            let result = compact_session(&self.session, CompactionConfig::default());
            self.session = result.compacted_session;
            Ok(Some(AutoCompactionEvent {
                removed_message_count: result.removed_message_count,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Environment variable for auto-compaction threshold
pub fn auto_compaction_threshold_from_env() -> u32 {
    std::env::var(AUTO_COMPACTION_THRESHOLD_ENV_VAR)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_AUTO_COMPACTION_INPUT_TOKENS_THRESHOLD)
}
```

#### Configuration Loading

```rust
// src/config.rs

pub struct ConfigLoader {
    cwd: PathBuf,
    config_home: PathBuf,
}

impl ConfigLoader {
    pub fn default_for(cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();
        let config_home = std::env::var_os("CLAUDE_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude"))
            })
            .unwrap_or_else(|| PathBuf::from(".claude"));
        Self { cwd, config_home }
    }

    pub fn discover(&self) -> Vec<ConfigEntry> {
        vec![
            // Legacy user config
            ConfigEntry {
                source: ConfigSource::User,
                path: self.config_home.parent().join(".claude.json"),
            },
            // User settings
            ConfigEntry {
                source: ConfigSource::User,
                path: self.config_home.join("settings.json"),
            },
            // Project settings
            ConfigEntry {
                source: ConfigSource::Project,
                path: self.cwd.join(".claude.json"),
            },
            // Local overrides (highest precedence)
            ConfigEntry {
                source: ConfigSource::Local,
                path: self.cwd.join(".claude/settings.local.json"),
            },
        ]
    }

    pub fn load(&self) -> Result<RuntimeConfig, ConfigError> {
        let entries = self.discover();
        let mut merged = BTreeMap::new();
        let mut loaded_entries = Vec::new();

        for entry in entries {
            if entry.path.exists() {
                let content = fs::read_to_string(&entry.path)?;
                let json: JsonValue = serde_json::from_str(&content)
                    .map_err(|e| ConfigError::Parse(e.to_string()))?;
                merge_json(&mut merged, &json);
                loaded_entries.push(entry);
            }
        }

        Ok(RuntimeConfig {
            merged,
            loaded_entries,
            feature_config: self.extract_features(&merged),
        })
    }

    fn extract_features(&self, merged: &BTreeMap<String, JsonValue>) -> RuntimeFeatureConfig {
        RuntimeFeatureConfig {
            hooks: self.extract_hooks(merged),
            mcp: self.extract_mcp_config(merged),
            oauth: self.extract_oauth_config(merged),
            model: self.extract_model(merged),
            permission_mode: self.extract_permission_mode(merged),
            sandbox: self.extract_sandbox_config(merged),
        }
    }
}

/// Configuration merging (later values override earlier)
fn merge_json(target: &mut BTreeMap<String, JsonValue>, source: &JsonValue) {
    if let JsonValue::Object(obj) = source {
        for (key, value) in obj {
            target.insert(key.clone(), value.clone());
        }
    }
}
```

#### MCP Stdio Transport

```rust
// src/mcp_stdio.rs

/// Manages MCP server processes
pub struct McpServerManager {
    servers: BTreeMap<String, ManagedMcpServer>,
}

struct ManagedMcpServer {
    process: McpStdioProcess,
    tools: Vec<McpTool>,
    initialized: bool,
}

/// MCP process with stdin/stdout handles
pub struct McpStdioProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    pending_requests: BTreeMap<JsonRpcId, oneshot::Sender<JsonRpcResponse>>,
}

impl McpServerManager {
    pub fn spawn_stdio_process(
        command: &str,
        args: &[String],
        env: &BTreeMap<String, String>,
    ) -> Result<McpStdioProcess, McpError> {
        let mut child = Command::new(command)
            .args(args)
            .envs(env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().ok_or(McpError::NoStdin)?;
        let stdout = BufReader::new(child.stdout.take().ok_or(McpError::NoStdout)?);

        Ok(McpStdioProcess {
            child,
            stdin,
            stdout,
            pending_requests: BTreeMap::new(),
        })
    }

    pub async fn initialize(&mut self, process: &mut McpStdioProcess) -> Result<McpInitializeResult, McpError> {
        let params = McpInitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: McpInitializeClientInfo {
                roots: Some(ClientRootsCapability { list_changed: Some(true) }),
                sampling: None,
            },
            client_info: ClientInfo {
                name: "claw-code".to_string(),
                version: VERSION.to_string(),
            },
        };

        let request = JsonRpcRequest::initialize(params);
        let (tx, rx) = oneshot::channel();
        process.pending_requests.insert(request.id.clone(), tx);

        // Send request
        let json = serde_json::to_string(&request)?;
        writeln!(process.stdin, "{json}")?;

        // Read response
        let response = read_json_rpc_response(&mut process.stdout).await?;
        let result: McpInitializeResult = serde_json::from_value(response.result.ok_or(McpError::NoResult)?)?;

        Ok(result)
    }

    pub async fn list_tools(&mut self, process: &mut McpStdioProcess) -> Result<Vec<McpTool>, McpError> {
        let request = JsonRpcRequest::list_tools(McpListToolsParams {});
        let (tx, rx) = oneshot::channel();
        process.pending_requests.insert(request.id.clone(), tx);

        let json = serde_json::to_string(&request)?;
        writeln!(process.stdin, "{json}")?;

        let response = read_json_rpc_response(&mut process.stdout).await?;
        let result: McpListToolsResult = serde_json::from_value(response.result.ok_or(McpError::NoResult)?)?;

        Ok(result.tools)
    }

    pub async fn call_tool(
        &mut self,
        process: &mut McpStdioProcess,
        params: McpToolCallParams,
    ) -> Result<McpToolCallResult, McpError> {
        let request = JsonRpcRequest::call_tool(params);
        let (tx, rx) = oneshot::channel();
        process.pending_requests.insert(request.id.clone(), tx);

        let json = serde_json::to_string(&request)?;
        writeln!(process.stdin, "{json}")?;

        let response = read_json_rpc_response(&mut process.stdout).await?;

        if let Some(error) = response.error {
            return Err(McpError::ServerError(error.message));
        }

        let result: McpToolCallResult = serde_json::from_value(response.result.ok_or(McpError::NoResult)?)?;
        Ok(result)
    }
}

/// JSON-RPC message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonRpcId {
    Integer(u64),
    String(String),
    Null,
}
```

---

### `tools` Crate

**Path**: `rust/crates/tools/`

**Purpose**: Built-in tool specifications and execution

#### Tool Specifications

```rust
// src/lib.rs

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: serde_json::Value,
    pub required_permission: PermissionMode,
}

pub fn mvp_tool_specs() -> Vec<ToolSpec> {
    vec![
        // Bash execution
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
        },

        // File reading
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
        },

        // File writing
        ToolSpec {
            name: "write_file",
            description: "Write a text file in the workspace.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
        },

        // File editing with string replacement
        ToolSpec {
            name: "edit_file",
            description: "Replace text in a workspace file.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old_string": { "type": "string" },
                    "new_string": { "type": "string" },
                    "replace_all": { "type": "boolean" }
                },
                "required": ["path", "old_string", "new_string"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
        },

        // ... 11 more tools
    ]
}
```

#### Tool Execution

```rust
pub fn execute_tool(
    tool_name: &str,
    input: &str,
    cwd: &Path,
    session: &Session,
) -> Result<String, ToolError> {
    let input: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| ToolError::new(format!("Invalid JSON input: {e}")))?;

    match tool_name {
        "bash" => execute_bash(input, cwd),
        "read_file" => read_file(input, cwd),
        "write_file" => write_file(input, cwd),
        "edit_file" => edit_file(input, cwd),
        "glob_search" => glob_search(input, cwd),
        "grep_search" => grep_search(input, cwd),
        "WebFetch" => web_fetch(input),
        "WebSearch" => web_search(input),
        "TodoWrite" => todo_write(input, session),
        "Skill" => load_skill(input, cwd),
        "Agent" => spawn_agent(input, cwd),
        "ToolSearch" => search_tools(input),
        "NotebookEdit" => notebook_edit(input, cwd),
        "Sleep" => sleep(input),
        other => Err(ToolError::new(format!("Unknown tool: {other}"))),
    }
}

/// Bash execution example
pub fn execute_bash(input: serde_json::Value, cwd: &Path) -> Result<String, ToolError> {
    let BashCommandInput { command, timeout, .. } = serde_json::from_value(input)?;

    let mut child = Command::new("bash")
        .arg("-c")
        .arg(&command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Wait with timeout
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let stdout = read_pipe_output(child.stdout.take())?;
            let stderr = read_pipe_output(child.stderr.take())?;

            return Ok(format!("Exit code: {}\n\nStdout:\n{}\nStderr:\n{}",
                status.code().unwrap_or(-1), stdout, stderr));
        }

        if start.elapsed() > Duration::from_secs(timeout.unwrap_or(60)) {
            child.kill()?;
            return Err(ToolError::new("Command timed out"));
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
```

---

### `commands` Crate

**Path**: `rust/crates/commands/`

**Purpose**: Slash command registry, parsing, and handling

#### Command Specifications

```rust
// src/lib.rs

#[derive(Debug, Clone, Copy)]
pub struct SlashCommandSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub argument_hint: Option<&'static str>,
    pub resume_supported: bool,
}

const SLASH_COMMAND_SPECS: &[SlashCommandSpec] = &[
    SlashCommandSpec {
        name: "help",
        summary: "Show available slash commands",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "status",
        summary: "Show current session status",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "compact",
        summary: "Compact local session history",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "model",
        summary: "Show or switch the active model",
        argument_hint: Some("[model]"),
        resume_supported: false,
    },
    // ... 18 more commands
];
```

#### Command Parsing

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommand {
    Help,
    Status,
    Compact,
    Bughunter { scope: Option<String> },
    Commit,
    Pr { context: Option<String> },
    Issue { context: Option<String> },
    Ultraplan { task: Option<String> },
    Teleport { target: Option<String> },
    DebugToolCall,
    Model { model: Option<String> },
    Permissions { mode: Option<String> },
    Clear { confirm: bool },
    Cost,
    Resume { session_path: Option<String> },
    Config { section: Option<String> },
    Memory,
    Init,
    Diff,
    Version,
    Export { path: Option<String> },
    Session { action: Option<String>, target: Option<String> },
    Unknown(String),
}

impl SlashCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        let mut parts = trimmed.trim_start_matches('/').split_whitespace();
        let command = parts.next().unwrap_or_default();

        Some(match command {
            "help" => Self::Help,
            "status" => Self::Status,
            "compact" => Self::Compact,
            "bughunter" => Self::Bughunter {
                scope: remainder_after_command(trimmed, command),
            },
            "commit" => Self::Commit,
            "pr" => Self::Pr {
                context: remainder_after_command(trimmed, command),
            },
            "model" => Self::Model {
                model: parts.next().map(ToOwned::to_owned),
            },
            // ... more commands
            other => Self::Unknown(other.to_string()),
        })
    }
}

fn remainder_after_command(input: &str, command: &str) -> Option<String> {
    input
        .trim()
        .strip_prefix(&format!("/{command}"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}
```

---

## Key Rust Patterns

### 1. Trait-Based Abstraction

The codebase uses traits extensively for testability and flexibility:

```rust
// Abstraction for API clients
pub trait ApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError>;
}

// Abstraction for tool executors
pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}

// Abstraction for permission prompting
pub trait PermissionPrompter {
    fn prompt(&mut self, request: &PermissionRequest) -> PermissionPromptDecision;
}

// Usage in ConversationRuntime (generic over traits)
pub struct ConversationRuntime<C, T>
where
    C: ApiClient,
    T: ToolExecutor,
{
    api_client: C,
    tool_executor: T,
    // ...
}
```

### 2. Builder Pattern

Used for constructing complex objects:

```rust
impl<C, T> ConversationRuntime<C, T> {
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub fn with_auto_compaction_input_tokens_threshold(mut self, threshold: u32) -> Self {
        self.auto_compaction_input_tokens_threshold = threshold;
        self
    }
}

// Usage
let runtime = ConversationRuntime::new(session, client, executor, policy, prompt)
    .with_max_iterations(100)
    .with_auto_compaction_input_tokens_threshold(200_000);
```

### 3. Newtype Pattern

Type-safe wrappers around primitive types:

```rust
// Type-safe JSON-RPC ID
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonRpcId {
    Integer(u64),
    String(String),
    Null,
}

// Type-safe config sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigSource {
    User,
    Project,
    Local,
}
```

### 4. Result Type Aliases

Consistent error handling:

```rust
pub type Result<T, E> = std::result::Result<T, E>;

// Specific result types
pub type ApiResult<T> = Result<T, ApiError>;
pub type RuntimeError = crate::error::RuntimeError;
pub type ToolError = crate::error::ToolError;
pub type McpError = crate::mcp::McpError;
pub type SessionError = crate::session::SessionError;
```

### 5. Enum-Based State Machines

For representing protocol states:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionOutcome {
    Allow,
    Deny,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionPromptDecision {
    AllowOnce,
    AllowAlways,
    Deny,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart(MessageStartEvent),
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
    MessageDelta(MessageDeltaEvent),
    MessageStop(MessageStopEvent),
}
```

### 6. Tagged Serde Enums

For JSON serialization with type discrimination:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerConfig {
    #[serde(rename = "stdio")]
    Stdio(McpStdioServerConfig),
    #[serde(rename = "sse")]
    Sse(McpRemoteServerConfig),
    #[serde(rename = "http")]
    Http(McpRemoteServerConfig),
    #[serde(rename = "ws")]
    Ws(McpWebSocketServerConfig),
}
```

---

## Async Runtime Design

### Tokio Usage

The codebase uses Tokio for async operations:

```rust
// Tokio features used (from Cargo.toml):
# runtime/Cargo.toml
tokio = { version = "1", features = [
    "io-util",      # AsyncRead/AsyncWrite utilities
    "process",      # Child process management
    "rt",           # Basic runtime
    "rt-multi-thread",  # Multi-threaded scheduler
    "time",         # Sleep, timeout, interval
]}

# api/Cargo.toml
tokio = { version = "1", features = [
    "io-util",
    "macros",       # #[tokio::main], #[tokio::test]
    "net",          # TCP/UDP networking
    "rt-multi-thread",
    "time",
]}
```

### Async Patterns

```rust
// Async function with Result
pub async fn stream(
    &self,
    request: MessageRequest,
) -> Result<impl Stream<Item = Result<StreamEvent, ApiError>>, ApiError> {
    let response = self.http_client
        .post(format!("{}/v1/messages", self.base_url))
        .json(&request)
        .send()
        .await?;
    // ...
}

// Async process I/O
pub async fn read_json_rpc_response(
    stdout: &mut BufReader<ChildStdout>,
) -> Result<JsonRpcResponse, McpError> {
    let mut line = String::new();
    stdout.read_line(&mut line).await?;
    serde_json::from_str(&line).map_err(McpError::from)
}

// Tokio spawn for background tasks
tokio::spawn(async move {
    // Background processing
});
```

### Blocking Operations

Some operations remain synchronous:

```rust
// File I/O (using std::fs, not tokio::fs)
pub fn save(&self, path: &Path) -> Result<(), SessionError> {
    let json = serde_json::to_string_pretty(self)?;
    fs::write(path, json)?;  // Blocking, but acceptable for CLI
    Ok(())
}

// Process spawning (using std::process::Command)
pub fn spawn_stdio_process(command: &str, args: &[String]) -> Result<McpStdioProcess, McpError> {
    let child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;  // Synchronous spawn
    // ...
}
```

---

## Error Handling Strategy

### Custom Error Types

Each crate defines its own error types:

```rust
// api/src/error.rs
#[derive(Debug)]
pub enum ApiError {
    Http(reqwest::Error),
    Parse(serde_json::Error),
    Unauthorized(String),
    RateLimited { retry_after: Option<u64> },
    ServerError { status: u16, message: String },
}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parse(error)
    }
}

// runtime/src/conversation.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl Display for RuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RuntimeError {}
```

### Error Propagation

Using the `?` operator extensively:

```rust
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match parse_args(&args)? {  // Propagate parse error
        CliAction::Prompt { prompt, .. } => {
            LiveCli::new(model, true, ..)?.run_turn_with_output(&prompt, ..)?
        }
        CliAction::Login => run_login()?,  // Propagate login error
        // ...
    }
    Ok(())
}
```

### Boxed Errors for CLI

The main entry point uses boxed errors for simplicity:

```rust
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Any error type can be returned with ?
}
```

---

## Memory Management

### Ownership Model

The codebase follows standard Rust ownership:

```rust
// Borrowing for read-only access
pub fn execute_tool(
    tool_name: &str,      // Borrowed string slice
    input: &str,          // Borrowed string slice
    cwd: &Path,           // Borrowed path reference
    session: &Session,    // Borrowed session reference
) -> Result<String, ToolError> {
    // ...
}

// Taking ownership for configuration
pub fn new(
    session: Session,     // Takes ownership
    api_client: C,        // Takes ownership
    tool_executor: T,     // Takes ownership
) -> Self {
    Self { session, api_client, tool_executor, .. }
}

// Cloning when needed
pub fn run_turn(&mut self, user_input: impl Into<String>) -> Result<TurnSummary, RuntimeError> {
    let request = ApiRequest {
        system_prompt: self.system_prompt.clone(),  // Clone for API request
        messages: self.session.messages.clone(),    // Clone for API request
    };
    // ...
}
```

### Smart Pointers

Used where needed:

```rust
// Rc for shared read-only data
use std::rc::Rc;

// Arc would be used for multi-threaded sharing (not needed in this single-threaded async code)
use std::sync::Arc;

// RefCell for interior mutability (rarely used in this codebase)
use std::cell::RefCell;
```

### Memory-Efficient Patterns

```rust
// Using &str instead of String where possible
pub fn resolve_model_alias(model: &str) -> &str {
    match model {
        "opus" => "claude-opus-4-6",
        "sonnet" => "claude-sonnet-4-6",
        _ => model,
    }
}

// Using Cow for zero-copy when possible
use std::borrow::Cow;

pub fn highlight_line<'a>(line: &'a str, theme: &Theme) -> Cow<'a, str> {
    if needs_highlighting(line) {
        Cow::Owned(do_highlight(line, theme))
    } else {
        Cow::Borrowed(line)  // Zero copy
    }
}
```

---

## Testing Patterns

### Unit Tests

Located in `#[cfg(test)]` modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slash_commands() {
        assert_eq!(SlashCommand::parse("/help"), Some(SlashCommand::Help));
        assert_eq!(SlashCommand::parse("/status"), Some(SlashCommand::Status));
        assert_eq!(SlashCommand::parse("/compact"), Some(SlashCommand::Compact));
    }

    #[test]
    fn renders_help_from_shared_specs() {
        let help = render_slash_command_help();
        assert!(help.contains("Slash commands"));
        assert!(help.contains("/help"));
        assert!(help.contains("/status"));
        assert_eq!(slash_command_specs().len(), 22);
    }

    #[test]
    fn compacts_sessions_via_slash_command() {
        let session = Session {
            messages: vec![
                ConversationMessage::user_text("a".repeat(200)),
                ConversationMessage::assistant(vec![ContentBlock::Text { text: "b".repeat(200) }]),
            ],
        };

        let result = handle_slash_command("/compact", &session, CompactionConfig::default())
            .expect("slash command should be handled");

        assert!(result.message.contains("Compacted"));
        assert_eq!(result.session.messages[0].role, MessageRole::System);
    }
}
```

### Integration Tests

In `tests/` directory:

```rust
// crates/api/tests/client_integration.rs

#[tokio::test]
async fn test_streaming_response() {
    let client = AnthropicClient::new(AuthSource::ApiKey(test_key())).unwrap();

    let request = MessageRequest {
        model: "claude-haiku-4-5-20251213".to_string(),
        messages: vec![InputMessage::user_text("Hello")],
        max_tokens: 100,
        stream: true,
        ..Default::default()
    };

    let events = client.stream(request).await.unwrap();
    // Verify streaming events
}
```

### Test Utilities

```rust
// runtime/src/lib.rs

#[cfg(test)]
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

// Usage in tests to serialize access to environment variables
#[test]
fn test_with_env() {
    let _lock = test_env_lock();
    std::env::set_var("TEST_VAR", "value");
    // Test code
    std::env::remove_var("TEST_VAR");
}
```

### Mocking Traits

```rust
// Mock API client for testing
struct MockApiClient {
    response: Vec<AssistantEvent>,
}

impl ApiClient for MockApiClient {
    fn stream(&mut self, _request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        Ok(self.response.clone())
    }
}

// Mock tool executor for testing
struct MockToolExecutor {
    responses: BTreeMap<String, String>,
}

impl ToolExecutor for MockToolExecutor {
    fn execute(&mut self, tool_name: &str, _input: &str) -> Result<String, ToolError> {
        self.responses
            .get(tool_name)
            .cloned()
            .ok_or_else(|| ToolError::new(format!("Unknown tool: {tool_name}")))
    }
}

// Usage in tests
#[test]
fn test_conversation_turn() {
    let mock_client = MockApiClient {
        response: vec![AssistantEvent::TextDelta("Hello".to_string())],
    };

    let mock_executor = MockToolExecutor {
        responses: BTreeMap::new(),
    };

    let mut runtime = ConversationRuntime::new(
        Session::new(),
        mock_client,
        mock_executor,
        PermissionPolicy::default(),
        vec![],
    );

    let result = runtime.run_turn("Test prompt");
    assert!(result.is_ok());
}
```

---

## Build and Release

### Development Build

```bash
cd rust/
cargo build           # Debug build
cargo build --release # Release build
```

### Running Tests

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p runtime
cargo test -p commands

# With output
cargo test --workspace -- --nocapture

# Specific test
cargo test -p commands parses_slash_commands
```

### Formatting and Linting

```bash
# Format all code
cargo fmt --all

# Run clippy
cargo clippy --workspace --all-targets -- -D warnings

# Check without building
cargo check --workspace
```

### Release Build Configuration

```toml
# rust/Cargo.toml
[profile.release]
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
panic = "abort"      # Abort on panic (smaller binary)
strip = true         # Strip debug symbols
```

### Binary Output

```bash
# After release build
./target/release/claw --version

# Binary size
ls -lh target/release/claw
# Typical: 2-5 MB (stripped)
```

### Cross-Compilation

```bash
# Install target
rustup target add x86_64-unknown-linux-musl

# Build for target
cargo build --release --target x86_64-unknown-linux-musl
```

---

## Performance Considerations

### Optimization Targets

1. **Streaming latency**: Minimize time to first token
2. **Memory usage**: Keep session state bounded
3. **Tool execution**: Parallel where safe
4. **Binary size**: Optimize for distribution

### Current Optimizations

```rust
// Unlimited max iterations (usize::MAX) - no artificial limit
max_iterations: usize::MAX

// Token-based auto-compaction to bound memory
auto_compaction_input_tokens_threshold: 200_000

// Efficient SSE parsing (incremental, not buffering entire response)
pub fn parse_chunk(&mut self, chunk: &str) -> Vec<Result<StreamEvent, ApiError>> {
    // Process incrementally
}
```

### Potential Improvements (from TUI-ENHANCEMENT-PLAN.md)

1. Remove artificial streaming delays (8ms per chunk)
2. Incremental markdown rendering
3. Lazy tool output loading
4. Background session persistence

---

*Last updated: 2026-04-02*
*Based on Rust workspace revision: 0.1.0*
