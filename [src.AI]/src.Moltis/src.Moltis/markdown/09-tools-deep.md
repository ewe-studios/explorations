# Tools Deep Dive: Exec, Sandboxing, and File Operations

The tools crate implements all agent tools — exec, file read/write/edit, glob, grep, directory listing, and more. This document covers the actual implementation.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/tools/src/`

## Exec Tool: Shell Command Execution

**Source:** `exec.rs` (~600 lines)

### Core Function: `exec_command`

```rust
// exec.rs:125-196
pub async fn exec_command(command: &str, opts: &ExecOpts) -> Result<ExecResult>
```

The implementation:

```rust
let mut cmd = Command::new("sh");
cmd.arg("-c").arg(command);

if let Some(ref dir) = opts.working_dir {
    cmd.current_dir(dir);
}
for (k, v) in &opts.env {
    cmd.env(k, v);
}

cmd.stdout(std::process::Stdio::piped());
cmd.stderr(std::process::Stdio::piped());
cmd.stdin(std::process::Stdio::null());  // Child doesn't inherit stdin

let child = cmd.spawn().map_err(|e| { ... })?;
let result = tokio::time::timeout(opts.timeout, child.wait_with_output()).await;
```

**Key behavior:**
- Runs via `sh -c` — the shell interprets the command
- **stdin is null** — child processes cannot read from stdin (security measure)
- **Timeout** via `tokio::time::timeout` — default 30 seconds, max 1800 seconds (30 minutes)
- **Output truncated** at `max_output_bytes` (default 200KB) with `... [output truncated]` appended

### ExecResult

```rust
// exec.rs:88-93
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,  // -1 if signal terminated
}
```

### ExecOpts Defaults

```rust
// exec.rs:104-113
impl Default for ExecOpts {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_output_bytes: 200 * 1024,  // 200KB
            working_dir: None,
            env: Vec::new(),
        }
    }
}
```

### ExecTool: Agent Tool Implementation

```rust
// exec.rs:199-214
pub struct ExecTool {
    pub default_timeout: Duration,
    pub max_output_bytes: usize,
    pub working_dir: Option<PathBuf>,
    approval_manager: Option<Arc<ApprovalManager>>,
    broadcaster: Option<Arc<dyn ApprovalBroadcaster>>,
    sandbox: Arc<dyn Sandbox>,
    sandbox_id: Option<SandboxId>,
    sandbox_router: Option<Arc<SandboxRouter>>,
    env_provider: Option<Arc<dyn EnvVarProvider>>,
    completion_callback: Option<ExecCompletionFn>,
    node_provider: Option<Arc<dyn NodeExecProvider>>,
    default_node: Option<String>,
}
```

The builder pattern:

```rust
ExecTool::default()
    .with_approval(manager, broadcaster)
    .with_sandbox(sandbox, id)
    .with_sandbox_router(router)
    .with_env_provider(provider)
    .with_completion_callback(cb)
    .with_default_timeout(timeout)
    .with_max_output_bytes(bytes)
    .with_node_provider(provider, default_node)
```

### Tool Schema

```rust
// exec.rs:325-359
fn parameters_schema(&self) -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "command": { "type": "string", "description": "The shell command to execute" },
            "timeout": { "type": "integer", "description": "Timeout in seconds (default 30, max 1800)" },
            "working_dir": { "type": "string", "description": "Working directory" },
            "node": { "type": "string", "description": "Node name or ID (if remote nodes connected)" }
        },
        "required": ["command"]
    })
}
```

The `node` parameter is **conditionally added** only when remote nodes are connected.

### Execute Method Logic

The execute flow:

1. **Parse command** — required, errors if missing
2. **Parse timeout** — capped at 1800 seconds (30 minutes)
3. **Node routing** — if `node_provider` is set and nodes connected:
   - Model-supplied node values are resolved via `provider.resolve_node_id()`
   - If no nodes connected but `default_node` is configured → clear error
   - If model hallucinates a node value with no nodes connected → silently drops it
4. **Sandbox check** — `router.is_sandboxed(session_key)` or `sandbox_id.is_some()`
5. **Backend isolation check** — `provides_fs_isolation()` determines if working in container or host
6. **Working directory resolution**:
   - Sandbox with container backend → default `/home/sandbox`
   - Host execution → default `$HOME` (via `moltis_config::home_dir()`)
   - Explicit `working_dir` validated — if it doesn't exist on host, falls back to default
   - Relative paths resolved under `/home/sandbox`
7. **Approval gating** — if `approval_manager` is set, broadcasts request and waits for approval
8. **Environment injection** — from `env_provider` (API keys, etc.)
9. **Execute** — either `exec_command()` locally or `provider.exec_on_node()` remotely
10. **Completion callback** — fires `ExecCompletionEvent` with command, exit code, previews

### Approval Gating

```rust
// exec.rs:565-592
if let Some(ref mgr) = self.approval_manager {
    let request_id = format!("exec-{}", uuid::Uuid::new_v4());
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();
    
    mgr.create_request(&request_id, command, session_key, response_tx).await?;
    self.broadcaster.as_ref().unwrap()
        .broadcast_request(&request_id, command, session_key).await?;
    
    let response = response_rx.await?;
    match response.decision {
        ApprovalDecision::Approve => { /* continue */ },
        ApprovalDecision::Deny => {
            return Ok(serde_json::json!({
                "error": "command denied by user",
                "command": command
            }));
        },
        ApprovalDecision::DenyAndBlock => { /* block future similar commands */ },
    }
}
```

### Sandbox Recovery

```rust
// exec.rs:600-630
if let Some(ref router) = self.sandbox_router {
    let session_key = session_key.unwrap_or("main");
    match router.resolve_backend(session_key).await {
        SandboxBackend::Docker | SandboxBackend::Podman | SandboxBackend::AppleContainer => {
            // Execute in container
        },
        SandboxBackend::RestrictedHost | SandboxBackend::Cgroup | SandboxBackend::None => {
            // Execute directly on host
        },
    }
}
```

**Aha:** When the sandbox backend doesn't provide filesystem isolation (RestrictedHost, Cgroup, None), commands run directly on the host even when the session mode says "sandboxed." Using `/home/sandbox` as the working directory would fail with ENOENT on the host, so the code falls back to the host data directory.

### Node Execution

When remote nodes are connected:

```rust
let result = provider
    .exec_on_node(&node_id, command, timeout_secs, cwd, None)
    .await
    .map_err(|e| Error::message(format!("node exec failed: {e}")))?;
```

The `NodeExecProvider` trait (defined in `exec.rs:60-85`) bridges the tools crate with the gateway crate's `node_exec` module without a direct dependency.

---

## File Read Tool

**Source:** `fs/read/tool.rs`

### Schema

```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string", "description": "File path" },
    "offset": { "type": "integer", "description": "Start line (0-based)" },
    "limit": { "type": "integer", "description": "Max lines to read" }
  },
  "required": ["path"]
}
```

### Implementation

```rust
// fs/read/tool.rs
async fn execute(&self, params: Value) -> Result<Value> {
    let path = params["path"].as_str().unwrap();
    let offset = params["offset"].as_u64().unwrap_or(0);
    let limit = params["limit"].as_u64().unwrap_or(1000);
    
    // Path validation
    // Read file
    // Return JSON: { path, content, total_lines, lines_read, offset }
}
```

---

## File Write Tool

**Source:** `file_write.rs`

### Schema

```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string" },
    "content": { "type": "string" },
    "append": { "type": "boolean", "default": false }
  },
  "required": ["path", "content"]
}
```

### Implementation

```rust
// file_write.rs
async fn execute(&self, params: Value) -> Result<Value> {
    let path = params["path"].as_str().unwrap();
    let content = params["content"].as_str().unwrap();
    let append = params["append"].as_bool().unwrap_or(false);
    
    // Path validation (no escaping workspace)
    // Create parent directories if needed
    // Write (or append)
    // Return: { path, bytes_written }
}
```

---

## File Edit Tool

**Source:** `file_edit.rs`

### Schema

```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string" },
    "old_text": { "type": "string", "description": "Text to find" },
    "new_text": { "type": "string", "description": "Replacement text" },
    "replace_all": { "type": "boolean", "default": false }
  },
  "required": ["path", "old_text", "new_text"]
}
```

### Implementation

```rust
// file_edit.rs
async fn execute(&self, params: Value) -> Result<Value> {
    let path = params["path"].as_str().unwrap();
    let old_text = params["old_text"].as_str().unwrap();
    let new_text = params["new_text"].as_str().unwrap();
    let replace_all = params["replace_all"].as_bool().unwrap_or(false);
    
    // Read file
    // Find old_text
    // If not found → error with "text not found"
    // If found multiple times and !replace_all → error "multiple matches"
    // Replace and write
    // Return: { path, replacements }
}
```

---

## Glob Tool

**Source:** `glob.rs`

### Schema

```json
{
  "type": "object",
  "properties": {
    "pattern": { "type": "string", "description": "Glob pattern (*.rs, src/**/*.rs)" },
    "path": { "type": "string", "description": "Search root (default: workspace)" }
  },
  "required": ["pattern"]
}
```

---

## Grep Tool

**Source:** `grep.rs`

### Schema

```json
{
  "type": "object",
  "properties": {
    "pattern": { "type": "string", "description": "Regex pattern" },
    "path": { "type": "string", "description": "File or directory" },
    "case_sensitive": { "type": "boolean" },
    "max_results": { "type": "integer", "default": 100 }
  },
  "required": ["pattern"]
}
```

---

## List Directory Tool

**Source:** `list_directory.rs`

### Schema

```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string", "description": "Directory path (default: workspace root)" }
  }
}
```

### Return Format

```json
{
  "path": "/workspace/src",
  "entries": [
    { "name": "main.rs", "type": "file", "size": 1234 },
    { "name": "utils", "type": "directory" }
  ]
}
```

---

## ToolBase and ToolDispatcher

**Source:** `tool_base.rs`, `tool_dispatcher.rs`

### ToolBase

Abstract base for all tools:

```rust
#[async_trait]
pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value>;
}
```

### ToolDispatcher

Routes tool calls to the correct tool:

```rust
// tool_dispatcher.rs
pub async fn dispatch(&self, tool_name: &str, params: Value) -> Result<ToolResult> {
    let tool = self.registry.get(tool_name)
        .ok_or_else(|| Error::message(format!("unknown tool: {tool_name}")))?;
    
    // Validate params against schema
    let schema = tool.parameters_schema();
    validate_tool_args(&schema, &params)?;
    
    // Execute
    let result = tool.execute(params).await?;
    
    // Truncate output if too large
    truncate_if_needed(&result, self.max_output_bytes);
    
    Ok(result)
}
```

---

## ToolResult

**Source:** `tool_result.rs`

```rust
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}
```

---

## Sanity Checks

**Source:** `sanity_check.rs`

Pre-execution checks:
- Path validation (no escaping workspace)
- Command safety (no destructive commands without approval)
- Resource limits

---

## Next

Return to [03-gateway-deep](03-gateway-deep.md) for the gateway internals.
