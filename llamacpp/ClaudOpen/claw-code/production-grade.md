# Claw Code Production Implementation Guide

A comprehensive guide for building production-ready features on top of the Claw Code Rust implementation.

## Table of Contents

1. [Production Readiness Assessment](#production-readiness-assessment)
2. [Architecture Principles](#architecture-principles)
3. [Feature Implementation Patterns](#feature-implementation-patterns)
4. [Performance Optimization](#performance-optimization)
5. [Error Handling and Resilience](#error-handling-and-resilience)
6. [Security Considerations](#security-considerations)
7. [Observability and Monitoring](#observability-and-monitoring)
8. [Deployment Strategies](#deployment-strategies)

---

## Production Readiness Assessment

### Current State (as of 2026-04-02)

| Component | Status | Production Ready? |
|-----------|--------|-------------------|
| **Core API Client** | ✅ Complete | Yes |
| **OAuth Flow** | ✅ Complete | Yes |
| **Tool System (MVP)** | ✅ 15 tools | Yes for MVP |
| **Session Persistence** | ✅ Complete | Yes |
| **Permission System** | ✅ Complete | Yes |
| **MCP Support** | ✅ Stdio/SSE | Yes |
| **Configuration Loading** | ✅ Complete | Yes |
| **Hooks** | ⚠️ Config only | No - needs runtime |
| **Plugins** | ❌ Missing | No |
| **Skills Registry** | ⚠️ Local only | Partial |
| **TUI** | ⚠️ Inline only | Partial |
| **Tests** | ✅ Good coverage | Yes |

### Gaps to Address

1. **Hook Runtime Execution** - Currently parsed but not executed
2. **Plugin System** - Not implemented
3. **Full Skills Registry** - Only local file loading
4. **Enhanced TUI** - No full-screen mode, limited visual features
5. **Tool Parity** - Missing 25+ tools from TypeScript source

---

## Architecture Principles

### 1. Separation of Concerns

The workspace is organized by responsibility:

```
┌─────────────────────────────────────────────────────────────┐
│                    rusty-claude-cli                         │
│                  (CLI Entrypoint)                           │
├─────────────┬─────────────┬─────────────┬───────────────────┤
│  commands   │    api      │   runtime   │      tools        │
│  (slash)    │  (HTTP)     │   (core)    │  (executors)      │
└─────────────┴─────────────┴─────────────┴───────────────────┘
                          │
                   compat-harness
                  (TS extraction)
```

**Implementation Pattern:**
```rust
// Keep crates decoupled
// api crate knows nothing about tools
// tools crate depends on runtime types only
// runtime is the core - minimal external dependencies

// Example: Tool execution is isolated
pub fn execute_tool(
    tool_name: &str,
    input: &str,
    cwd: &Path,
) -> Result<String, ToolError> {
    // No direct API calls
    // No session mutation
    // Pure execution logic
}
```

### 2. Trait-Based Abstraction

Enables testing and future extensibility:

```rust
// Define traits for external boundaries
pub trait ApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError>;
}

pub trait ToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError>;
}

pub trait PermissionPrompter {
    fn prompt(&mut self, request: &PermissionRequest) -> PermissionPromptDecision;
}

// Implement for production types
impl ApiClient for AnthropicClient { /* ... */ }
impl ToolExecutor for StaticToolExecutor { /* ... */ }

// Implement for test mocks
impl ApiClient for MockApiClient { /* ... */ }
impl ToolExecutor for MockToolExecutor { /* ... */ }

// Runtime is generic over traits
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

### 3. Configuration Hierarchy

Multi-source configuration with clear precedence:

```rust
// Precedence (lowest to highest):
// 1. User global (~/.claude/settings.json)
// 2. Project (.claude.json)
// 3. Local (.claude/settings.local.json)

pub struct ConfigLoader {
    cwd: PathBuf,
    config_home: PathBuf,
}

impl ConfigLoader {
    pub fn discover(&self) -> Vec<ConfigEntry> {
        vec![
            ConfigEntry { source: ConfigSource::User, path: ... },
            ConfigEntry { source: ConfigSource::Project, path: ... },
            ConfigEntry { source: ConfigSource::Local, path: ... },
        ]
    }

    pub fn load(&self) -> Result<RuntimeConfig, ConfigError> {
        let entries = self.discover();
        let mut merged = BTreeMap::new();

        // Merge in order (later overrides earlier)
        for entry in entries {
            if entry.path.exists() {
                merge_json(&mut merged, load_file(&entry.path)?);
            }
        }

        Ok(RuntimeConfig { merged, .. })
    }
}
```

### 4. Session as Source of Truth

All conversation state flows through the Session type:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub messages: Vec<ConversationMessage>,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// Session is:
// - Serialized after each turn
// - Resumable across restarts
// - Compactable for long conversations
// - Exportable for analysis

// Usage tracking is derived from session
impl UsageTracker {
    pub fn from_session(session: &Session) -> Self {
        // Calculate from message history
    }

    pub fn record(&mut self, usage: TokenUsage) {
        // Update running totals
    }

    pub fn total(&self) -> TokenUsage {
        // Return cumulative usage
    }
}
```

---

## Feature Implementation Patterns

### Adding a New Tool

**Step 1: Define the tool spec**

```rust
// rust/crates/tools/src/lib.rs

ToolSpec {
    name: "my_new_tool",
    description: "Performs a useful action",
    input_schema: json!({
        "type": "object",
        "properties": {
            "param1": {
                "type": "string",
                "description": "First parameter"
            },
            "param2": {
                "type": "integer",
                "minimum": 0,
                "description": "Second parameter"
            }
        },
        "required": ["param1"],
        "additionalProperties": false
    }),
    required_permission: PermissionMode::WorkspaceWrite,
}
```

**Step 2: Implement execution**

```rust
// rust/crates/tools/src/lib.rs - execute_tool function

"my_new_tool" => {
    let input: MyToolInput = serde_json::from_value(input)?;
    my_new_tool_execute(input, cwd)
}

// Separate function for clarity
fn my_new_tool_execute(input: MyToolInput, cwd: &Path) -> Result<String, ToolError> {
    // Validate input
    if input.param1.is_empty() {
        return Err(ToolError::new("param1 cannot be empty"));
    }

    // Execute logic
    let result = do_something(&input.param1, input.param2);

    // Format output
    Ok(format!("Success: {result}"))
}
```

**Step 3: Add tests**

```rust
// rust/crates/tools/src/lib.rs - tests module

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn my_new_tool_validates_input() {
        let input = json!({"param1": ""});
        let result = execute_tool("my_new_tool", &input.to_string(), Path::new("."));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn my_new_tool_executes_successfully() {
        let input = json!({"param1": "test", "param2": 42});
        let result = execute_tool("my_new_tool", &input.to_string(), Path::new("."));
        assert!(result.is_ok());
    }
}
```

### Adding a Slash Command

**Step 1: Add specification**

```rust
// rust/crates/commands/src/lib.rs

const SLASH_COMMAND_SPECS: &[SlashCommandSpec] = &[
    // ... existing specs
    SlashCommandSpec {
        name: "mycommand",
        summary: "Does something useful",
        argument_hint: Some("[argument]"),
        resume_supported: true,
    },
];
```

**Step 2: Add enum variant**

```rust
pub enum SlashCommand {
    // ... existing variants
    MyCommand { argument: Option<String> },
    // ...
}
```

**Step 3: Implement parsing**

```rust
impl SlashCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        let mut parts = trimmed.trim_start_matches('/').split_whitespace();
        let command = parts.next().unwrap_or_default();

        Some(match command {
            // ... existing commands
            "mycommand" => Self::MyCommand {
                argument: remainder_after_command(trimmed, command),
            },
            other => Self::Unknown(other.to_string()),
        })
    }
}
```

**Step 4: Implement handling**

```rust
pub fn handle_slash_command(
    input: &str,
    session: &Session,
    compaction: CompactionConfig,
) -> Option<SlashCommandResult> {
    match SlashCommand::parse(input)? {
        // ... existing handlers
        SlashCommand::MyCommand { argument } => {
            let message = execute_my_command(argument, session);
            Some(SlashCommandResult {
                message,
                session: session.clone(),
            })
        }
        // ... more handlers
    }
}

fn execute_my_command(argument: Option<String>, session: &Session) -> String {
    match argument {
        Some(arg) => format!("Executing with argument: {arg}"),
        None => "Executing without arguments".to_string(),
    }
}
```

### Adding a Configuration Option

**Step 1: Add to config struct**

```rust
// rust/crates/runtime/src/config.rs

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeFeatureConfig {
    hooks: RuntimeHookConfig,
    mcp: McpConfigCollection,
    oauth: Option<OAuthConfig>,
    model: Option<String>,
    permission_mode: Option<ResolvedPermissionMode>,
    sandbox: SandboxConfig,
    // New field:
    my_feature: Option<MyFeatureConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MyFeatureConfig {
    enabled: bool,
    setting: String,
}
```

**Step 2: Add extraction logic**

```rust
impl ConfigLoader {
    fn extract_features(&self, merged: &BTreeMap<String, JsonValue>) -> RuntimeFeatureConfig {
        RuntimeFeatureConfig {
            hooks: self.extract_hooks(merged),
            mcp: self.extract_mcp_config(merged),
            // ...
            my_feature: merged.get("myFeature").and_then(|v| {
                Some(MyFeatureConfig {
                    enabled: v.get("enabled")?.as_bool()?,
                    setting: v.get("setting")?.as_str()?.to_string(),
                })
            }),
        }
    }
}
```

**Step 3: Document in schema**

```json
// Example .claude.json
{
  "myFeature": {
    "enabled": true,
    "setting": "value"
  }
}
```

---

## Performance Optimization

### Current Bottlenecks

From `TUI-ENHANCEMENT-PLAN.md`:

1. **Artificial streaming delay** - 8ms per chunk in `stream_markdown`
2. **Monolithic main.rs** - 3,159 lines, harder to optimize
3. **No incremental rendering** - Full markdown reparse on each update
4. **No lazy tool output** - Large outputs block rendering

### Optimization Strategies

#### 1. Remove Artificial Delays

```rust
// Current (rusty-claude-cli/src/main.rs)
fn stream_markdown(text: &str, renderer: &mut TerminalRenderer) -> io::Result<()> {
    for chunk in text.split_whitespace() {
        renderer.render(chunk)?;
        std::thread::sleep(Duration::from_millis(8));  // REMOVE THIS
    }
    Ok(())
}

// Optimized
fn stream_markdown(text: &str, renderer: &mut TerminalRenderer) -> io::Result<()> {
    // Stream immediately, let terminal handle refresh rate
    renderer.render(text)?;
    Ok(())
}
```

#### 2. Incremental Markdown Rendering

```rust
// Current: Full reparse
pub fn render_markdown(&mut self, markdown: &str) -> io::Result<()> {
    let parser = Parser::new_ext(markdown, Options::all());
    // Process all events...
}

// Optimized: Track state, render deltas
pub struct IncrementalMarkdownRenderer {
    state: RenderState,
    buffer: String,
    last_complete_paragraph: String,
}

impl IncrementalMarkdownRenderer {
    pub fn render_delta(&mut self, new_text: &str) -> io::Result<()> {
        self.buffer.push_str(new_text);

        // Find complete paragraphs (ending with blank line)
        if let Some((complete, remaining)) = split_at_paragraph_boundary(&self.buffer) {
            // Render complete paragraph
            self.render_paragraph(complete)?;
            self.buffer = remaining.to_string();
        }

        // Render partial paragraph without final formatting
        self.render_partial(&self.buffer)?;

        Ok(())
    }
}
```

#### 3. Lazy Tool Output

```rust
// For tool outputs longer than N lines
pub struct LazyToolOutput {
    summary: String,
    full_output: Lazy<Vec<String>>,
    expanded: Cell<bool>,
}

impl LazyToolOutput {
    pub fn new(lines: Vec<String>) -> Self {
        let summary = if lines.len() > 15 {
            format!("{} lines (use <expand> to show all)", lines.len())
        } else {
            lines.join("\n")
        };

        Self {
            summary,
            full_output: Lazy::new(|| lines),
            expanded: Cell::new(false),
        }
    }

    pub fn expand(&self) {
        self.expanded.set(true);
        // Trigger re-render with full output
    }

    pub fn display(&self) -> &str {
        if self.expanded.get() {
            &self.full_output.join("\n")
        } else {
            &self.summary
        }
    }
}
```

#### 4. Parallel Tool Execution (Safe Subset)

```rust
// For independent tool calls
pub async fn execute_tools_parallel(
    tools: Vec<(String, String)>,
) -> Vec<(String, Result<String, ToolError>)> {
    let futures: Vec<_> = tools
        .into_iter()
        .map(|(name, input)| tokio::spawn(async move {
            (name.clone(), execute_tool(&name, &input))
        }))
        .collect();

    let mut results = Vec::new();
    for future in futures {
        results.push(future.await.expect("Task panicked"));
    }
    results
}

// Only use for tools that are:
// - Read-only (no file writes)
// - Don't share state
// - Idempotent
```

### Memory Management

```rust
// Auto-compaction to bound session size
pub fn should_compact(session: &Session, config: &CompactionConfig) -> bool {
    let estimated_tokens = estimate_session_tokens(session);
    estimated_tokens >= config.max_estimated_tokens
}

pub fn compact_session(
    session: &Session,
    config: CompactionConfig,
) -> CompactionResult {
    // Keep recent messages
    let recent = session.messages
        .iter()
        .rev()
        .take(config.preserve_recent_messages)
        .rev()
        .cloned()
        .collect::<Vec<_>>();

    // Summarize old messages
    let old_summary = summarize_messages(&session.messages[..session.messages.len() - recent.len()]);

    // Create compacted session
    let mut compacted = Session::new();
    compacted.messages.push(ConversationMessage::system(old_summary));
    compacted.messages.extend(recent);

    CompactionResult {
        compacted_session: compacted,
        removed_message_count: session.messages.len() - compacted.messages.len(),
    }
}
```

---

## Error Handling and Resilience

### Error Categories

```rust
// api/src/error.rs
#[derive(Debug)]
pub enum ApiError {
    // Network errors
    Http(reqwest::Error),

    // Parsing errors
    Parse(serde_json::Error),

    // Auth errors
    Unauthorized(String),

    // Rate limiting
    RateLimited { retry_after: Option<u64> },

    // Server errors
    ServerError { status: u16, message: String },
}

// runtime/src/conversation.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    message: String,
}

// tools/src/lib.rs
#[derive(Debug, Clone)]
pub struct ToolError {
    message: String,
}
```

### Retry Logic

```rust
// For transient failures
pub async fn stream_with_retry(
    client: &AnthropicClient,
    request: MessageRequest,
) -> Result<Vec<AssistantEvent>, ApiError> {
    let mut attempts = 0;
    let max_attempts = 3;

    loop {
        match client.stream(request.clone()).await {
            Ok(events) => return Ok(events),
            Err(ApiError::RateLimited { retry_after }) => {
                attempts += 1;
                if attempts >= max_attempts {
                    return Err(ApiError::RateLimited { retry_after });
                }

                let delay = retry_after.unwrap_or(2_u64.pow(attempts as u32));
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
            Err(ApiError::Http(e)) if e.is_timeout() => {
                attempts += 1;
                if attempts >= max_attempts {
                    return Err(ApiError::Http(e));
                }
                tokio::time::sleep(Duration::from_secs(2_u64.pow(attempts as u32))).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Graceful Degradation

```rust
// Fall back to API key if OAuth fails
pub fn resolve_auth_source() -> AuthSource {
    // 1. Try explicit API key
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            return AuthSource::ApiKey(key);
        }
    }

    // 2. Try OAuth token
    if let Ok(token) = load_oauth_credentials() {
        if !oauth_token_is_expired(&token) {
            return AuthSource::OAuthBearer(token.access_token);
        }
        // Token expired - try refresh
        if let Ok(refreshed) = refresh_oauth_token(&token) {
            save_oauth_credentials(&refreshed).ok();
            return AuthSource::OAuthBearer(refreshed.access_token);
        }
    }

    // 3. Fall back to empty API key (will fail with clear error)
    AuthSource::ApiKey(String::new())
}

// Clear error message for auth failure
impl AnthropicClient {
    pub fn new(auth_source: AuthSource) -> Result<Self, ApiError> {
        if let AuthSource::ApiKey(key) = &auth_source {
            if key.is_empty() {
                eprintln!("Warning: No API key found.");
                eprintln!("Set ANTHROPIC_API_KEY or run 'claw login' for OAuth.");
            }
        }
        // ... continue with client creation
    }
}
```

### Session Recovery

```rust
// Handle corrupted session files
impl Session {
    pub fn load(path: &Path) -> Result<Self, SessionError> {
        let content = fs::read_to_string(path)
            .map_err(|e| SessionError::Io(e.to_string()))?;

        serde_json::from_str(&content).map_err(|e| {
            // Offer to backup corrupted session
            let backup_path = path.with_extension("json.corrupted");
            fs::rename(path, &backup_path).ok();
            SessionError::Corrupted {
                path: path.to_path_buf(),
                backup: backup_path,
                parse_error: e.to_string(),
            }
        })
    }

    // Create minimal valid session if all else fails
    pub fn load_or_create(path: &Path) -> Self {
        Self::load(path).unwrap_or_else(|e| {
            eprintln!("Warning: Could not load session: {e}");
            eprintln!("Starting fresh session.");
            Session::new()
        })
    }
}
```

---

## Security Considerations

### Permission Enforcement

```rust
// Three-tier permission system
pub enum PermissionMode {
    ReadOnly,        // Inspection only
    WorkspaceWrite,  // File modifications
    DangerFullAccess, // Full system access
}

// Tool permission requirements
fn get_required_permission(tool_name: &str) -> PermissionMode {
    match tool_name {
        "read_file" | "glob_search" | "grep_search" | "WebFetch" | "WebSearch" => {
            PermissionMode::ReadOnly
        }
        "write_file" | "edit_file" | "NotebookEdit" | "TodoWrite" => {
            PermissionMode::WorkspaceWrite
        }
        "bash" | "Agent" | "REPL" => {
            PermissionMode::DangerFullAccess
        }
        _ => PermissionMode::DangerFullAccess, // Default to restrictive
    }
}

// Authorization check
impl PermissionPolicy {
    pub fn authorize(
        &self,
        tool_name: &str,
        input: &str,
        mut prompter: Option<&mut dyn PermissionPrompter>,
    ) -> PermissionOutcome {
        // 1. Check explicit allow list
        if let Some(allowed) = &self.allowed_tools {
            if !allowed.contains(tool_name) {
                return PermissionOutcome::Deny;
            }
        }

        // 2. Check permission mode
        let required = get_required_permission(tool_name);
        if self.mode >= required {
            return PermissionOutcome::Allow;
        }

        // 3. Interactive prompt if available
        if let Some(prompter) = prompter.as_mut() {
            match prompter.prompt(&PermissionRequest {
                tool_name: tool_name.to_string(),
                input: input.to_string(),
                required_mode: required,
            }) {
                PermissionPromptDecision::AllowOnce => PermissionOutcome::Allow,
                PermissionPromptDecision::Deny => PermissionOutcome::Deny,
            }
        } else {
            PermissionOutcome::Deny
        }
    }
}
```

### Input Validation

```rust
// Validate file paths (prevent directory traversal)
pub fn validate_path(path: &str, cwd: &Path) -> Result<PathBuf, ToolError> {
    let path = Path::new(path);

    // Reject absolute paths outside workspace
    if path.is_absolute() {
        return Err(ToolError::new("Absolute paths not allowed"));
    }

    // Reject directory traversal
    if path.components().any(|c| c.as_os_str() == "..") {
        return Err(ToolError::new("Directory traversal not allowed"));
    }

    // Resolve to absolute path within workspace
    let resolved = cwd.join(path);

    // Verify resolved path is within workspace
    let workspace = cwd.canonicalize().map_err(|e| {
        ToolError::new(format!("Cannot resolve workspace: {e}"))
    })?;

    if !resolved.starts_with(&workspace) {
        return Err(ToolError::new("Path escapes workspace boundary"));
    }

    Ok(resolved)
}

// Validate bash commands (basic sanitization)
pub fn validate_bash_command(command: &str) -> Result<(), ToolError> {
    // Reject commands that try to escape the shell
    if command.contains("$(") && command.contains(")") {
        // Allow command substitution but warn
        eprintln!("Warning: Command contains command substitution");
    }

    // Reject obvious injection attempts
    if command.contains("\n") || command.contains("\r") {
        return Err(ToolError::new("Newlines not allowed in commands"));
    }

    Ok(())
}
```

### Credential Handling

```rust
// Secure credential storage
pub fn save_oauth_credentials(tokens: &OAuthTokenSet) -> std::io::Result<()> {
    let credentials_path = credentials_path();

    // Create directory with restrictive permissions
    if let Some(parent) = credentials_path.parent() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::create_dir_all(parent)?;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        }
    }

    // Write with restrictive permissions
    let json = serde_json::to_string_pretty(tokens)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&credentials_path, json)?;
        std::fs::set_permissions(&credentials_path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&credentials_path, json)?;
    }

    Ok(())
}

// Never log credentials
impl std::fmt::Debug for OAuthTokenSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthTokenSet")
            .field("expires_at", &self.expires_at)
            .field("access_token", &"<redacted>")
            .field("refresh_token", &"<redacted>")
            .finish()
    }
}
```

---

## Observability and Monitoring

### Token Usage Tracking

```rust
// runtime/src/usage.rs

pub struct UsageTracker {
    input_tokens: u32,
    output_tokens: u32,
    cache_read_input_tokens: u32,
    cache_write_input_tokens: u32,
    turns: u32,
}

impl UsageTracker {
    pub fn record(&mut self, usage: TokenUsage) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.cache_read_input_tokens += usage.cache_read_input_tokens.unwrap_or(0);
        self.cache_write_input_tokens += usage.cache_write_input_tokens.unwrap_or(0);
        self.turns += 1;
    }

    pub fn total(&self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cache_read_input_tokens: Some(self.cache_read_input_tokens),
            cache_write_input_tokens: Some(self.cache_write_input_tokens),
        }
    }

    pub fn estimate_cost(&self, model: &str) -> UsageCostEstimate {
        let pricing = pricing_for_model(model);
        pricing.estimate_cost(&self.total())
    }
}

pub struct UsageCostEstimate {
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub currency: String,
}

pub fn format_usd(cost: f64) -> String {
    format!("${:.4}", cost)
}
```

### Session Status Reporting

```rust
// Comprehensive status display
pub fn format_session_status(session: &Session, usage: &UsageTracker, model: &str) -> String {
    let mut lines = vec![
        format!("Model: {model}"),
        format!("Session ID: {}", session_id_display(session)),
        format!("Messages: {}", session.messages.len()),
        format!("Turns: {}", usage.turns),
        "".to_string(),
        "Token Usage:".to_string(),
        format!("  Input:  {}", usage.total().input_tokens),
        format!("  Output: {}", usage.total().output_tokens),
        format!("  Total:  {}", usage.total().total_tokens()),
        "".to_string(),
        "Estimated Cost:".to_string(),
        format!("  Input:  {}", format_usd(usage.estimate_cost(model).input_cost)),
        format!("  Output: {}", format_usd(usage.estimate_cost(model).output_cost)),
        format!("  Total:  {}", format_usd(usage.estimate_cost(model).total_cost)),
    ];

    lines.join("\n")
}
```

### Structured Logging

```rust
// For production deployments
pub struct TurnLog {
    pub timestamp: u64,
    pub session_id: String,
    pub turn_number: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub tools_executed: Vec<String>,
    pub duration_ms: u64,
}

impl TurnLog {
    pub fn emit(&self) {
        eprintln!("{}", serde_json::to_string(self).unwrap());
    }
}

// Usage in conversation loop
pub fn run_turn(&mut self, user_input: impl Into<String>) -> Result<TurnSummary, RuntimeError> {
    let start = Instant::now();
    // ... turn execution ...
    let duration = start.elapsed().as_millis() as u64;

    let log = TurnLog {
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        session_id: session_id(&self.session),
        turn_number: self.usage_tracker.turns + 1,
        input_tokens: summary.usage.input_tokens,
        output_tokens: summary.usage.output_tokens,
        tools_executed: summary.tool_results.iter().map(|r| r.tool_name()).collect(),
        duration_ms: duration,
    };
    log.emit();

    Ok(summary)
}
```

---

## Deployment Strategies

### Binary Distribution

```bash
# Build optimized release
cd rust/
cargo build --release

# Verify binary
./target/release/claw --version
./target/release/claw doctor

# Strip debug symbols (if not done in Cargo.toml)
strip target/release/claw

# Package
tar -czf claw-linux-x86_64.tar.gz -C target/release claw
```

### System Installation

```bash
# Linux (system-wide)
sudo install target/release/claw /usr/local/bin/

# macOS (Homebrew formula)
# Formula/claw.rb:
class Claw < Formula
  desc "AI agent harness CLI"
  homepage "https://github.com/instructkr/claw-code"
  url "..."
  sha256 "..."

  def install
    system "cargo", "install", *std_cargo_args
  end
end

# Windows (Scoop manifest)
# buckets/main/claw.json:
{
  "version": "0.1.0",
  "description": "AI agent harness CLI",
  "homepage": "https://github.com/instructkr/claw-code",
  "license": "MIT",
  "url": "...",
  "hash": "...",
  "bin": "claw.exe"
}
```

### Docker Container

```dockerfile
# Dockerfile
FROM rust:1.76 as builder

WORKDIR /app
COPY rust/Cargo.toml rust/Cargo.lock ./
COPY rust/crates ./crates

RUN cargo build --release --manifest-path crates/rusty-claude-cli/Cargo.toml

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/claw /usr/local/bin/

ENTRYPOINT ["claw"]
```

### Environment Configuration

```bash
# Production environment variables
export ANTHROPIC_API_KEY="sk-ant-..."
export CLAUDE_CONFIG_HOME="/etc/claw"
export CLAUDE_CODE_AUTO_COMPACT_INPUT_TOKENS="200000"

# Optional
export RUST_LOG="info"  # For logging
export ANTHROPIC_BASE_URL="https://custom-proxy.example.com"
```

### Health Checks

```rust
// doctor command implementation
pub fn run_doctor() -> Result<(), Box<dyn std::error::Error>> {
    println!("Claw Code Health Check");
    println!("=====================\n");

    // Check API key
    match std::env::var("ANTHROPIC_API_KEY") {
        Ok(key) if !key.is_empty() => println!("✓ API key configured"),
        _ => println!("✗ API key not configured"),
    }

    // Check OAuth
    match load_oauth_credentials() {
        Ok(token) if !oauth_token_is_expired(&token) => {
            println!("✓ OAuth token valid");
        }
        Ok(_) => println!("⚠ OAuth token expired"),
        Err(_) => println!("- No OAuth token"),
    }

    // Check config
    let loader = ConfigLoader::default_for(std::env::current_dir()?);
    let entries = loader.discover();
    for entry in entries {
        if entry.path.exists() {
            println!("✓ Config found: {:?}", entry.path);
        }
    }

    // Test API connectivity
    println!("\nTesting API connectivity...");
    // ... make test request ...

    Ok(())
}
```

---

## Migration Path from TypeScript

### Phase 1: Core Parity (Current)

- [x] API client with streaming
- [x] Basic tool set (15 MVP tools)
- [x] Session management
- [x] Permission system
- [ ] Hook runtime execution

### Phase 2: Extended Features

- [ ] Full hook system (PreToolUse/PostToolUse)
- [ ] Skills registry with bundled skills
- [ ] Enhanced TUI (status bar, themes)
- [ ] Additional tools (LSP, AskUserQuestion, etc.)

### Phase 3: Advanced Features

- [ ] Plugin system
- [ ] Full-screen TUI mode (ratatui)
- [ ] Remote/structured transports
- [ ] Analytics and team sync

### Compatibility Layer

```rust
// Use compat-harness to track TypeScript features
pub fn extract_manifest(paths: &UpstreamPaths) -> Result<ExtractedManifest, io::Error> {
    let commands_source = fs::read_to_string(paths.commands_path())?;
    let tools_source = fs::read_to_string(paths.tools_path())?;

    Ok(ExtractedManifest {
        commands: extract_commands(&commands_source),
        tools: extract_tools(&tools_source),
        bootstrap: extract_bootstrap_plan(&cli_source),
    })
}

// Generate parity reports
pub fn parity_report(rust_features: &[Feature], ts_features: &[Feature]) -> ParityReport {
    let mut missing = Vec::new();
    let mut implemented = Vec::new();

    for ts_feature in ts_features {
        if rust_features.contains(ts_feature) {
            implemented.push(ts_feature.name.clone());
        } else {
            missing.push(ts_feature.name.clone());
        }
    }

    ParityReport {
        total_ts: ts_features.len(),
        total_rust: rust_features.len(),
        implemented,
        missing,
    }
}
```

---

*Last updated: 2026-04-02*
*Target version: 0.2.0 (Production Ready)*
