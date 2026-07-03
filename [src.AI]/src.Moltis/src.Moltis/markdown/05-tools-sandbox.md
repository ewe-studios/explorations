# Tools and Sandboxing: How Tool Execution Works

This document explains how Moltis executes tools: the tool registry, individual tool implementations, sandbox routing, and WASM tools.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/tools/src/`

---

## Tool Execution Flow

```
Agent loop calls tool.execute(args)
    ↓
ToolDispatcher.dispatch(tool_name, args)
    ↓
1. Schema validation (pre-dispatch)
2. BeforeToolCall hook (can block/modify)
3. tool.execute(args)
    ↓
SandboxRouter.route(session_key)
    ↓
Backend: Docker | Podman | Apple Container | WASM | Vercel | Daytona | Firecracker
    ↓
Command execution → stdout, stderr, exit_code
    ↓
AfterToolCall hook
    ↓
Result returned to agent loop
```

---

## ToolRegistry

**Source:** `tool_registry.rs`

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn AgentTool>>,
}

#[async_trait]
pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> anyhow::Result<serde_json::Value>;
}
```

Registration:

```rust
registry.register(Box::new(ExecTool::default()
    .with_sandbox(sandbox, id)
    .with_approval(approval_mgr, broadcaster)
    .with_sandbox_router(router)
    .with_env_provider(env_provider)
));
```

---

## Exec Tool: Shell Command Execution

**Source:** `exec.rs`

### Core Function

```rust
pub async fn exec_command(command: &str, opts: &ExecOpts) -> Result<ExecResult> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::null());  // Child cannot read stdin

    if let Some(ref dir) = opts.working_dir { cmd.current_dir(dir); }
    for (k, v) in &opts.env { cmd.env(k, v); }

    let child = cmd.spawn()?;
    let result = tokio::time::timeout(opts.timeout, child.wait_with_output()).await;

    match result {
        Ok(Ok(output)) => {
            let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            truncate_output_for_display(&mut stdout, opts.max_output_bytes);
            truncate_output_for_display(&mut stderr, opts.max_output_bytes);
            Ok(ExecResult { stdout, stderr, exit_code: output.status.code().unwrap_or(-1) })
        },
        Ok(Err(e)) => Err(Error::message(format!("failed to run command: {e}"))),
        Err(_) => Err(Error::message(format!("command timed out after {}s", opts.timeout.as_secs()))),
    }
}
```

**Security measures:**
- `stdin = null` — child processes cannot read from stdin
- Output truncated at 200KB with `... [output truncated]` appended
- Timeout enforced via `tokio::time::timeout`

### ExecTool Builder Pattern

```rust
pub struct ExecTool {
    pub default_timeout: Duration,          // Default: 30 seconds
    pub max_output_bytes: usize,            // Default: 200KB
    pub working_dir: Option<PathBuf>,
    approval_manager: Option<Arc<ApprovalManager>>,
    broadcaster: Option<Arc<dyn ApprovalBroadcaster>>,
    sandbox: Arc<dyn Sandbox>,
    sandbox_router: Option<Arc<SandboxRouter>>,
    env_provider: Option<Arc<dyn EnvVarProvider>>,
    node_provider: Option<Arc<dyn NodeExecProvider>>,
    default_node: Option<String>,
}
```

### Node Routing

When remote nodes are connected, commands can be forwarded:

```rust
// Determine effective node reference
let node_ref = if let Some(provider) = &self.node_provider {
    if provider.has_connected_nodes() {
        match model_node.or_else(|| self.default_node.clone()) {
            Some(node_ref) => Some(node_ref),   // Model-supplied or configured default
            None => provider.default_node_ref().await,  // Use provider's default
        }
    } else if let Some(ref dn) = self.default_node {
        return Err(Error::message("default node '{dn}' is configured but no nodes connected"));
    } else {
        None  // Fall through to local execution
    }
} else { None };

// Forward to remote node
if let (Some(provider), Some(node_ref)) = (&self.node_provider, &node_ref) {
    let node_id = provider.resolve_node_id(node_ref).await?;
    let result = provider.exec_on_node(&node_id, command, timeout_secs, cwd, None).await?;
    return Ok(serde_json::to_value(&result)?);
}
```

**Key detail:** Model-hallucinated node values are silently dropped when no nodes are connected. A configured `default_node` produces a clear error so the admin knows the intended remote host is unavailable.

---

## Sandbox Routing

**Source:** `sandbox.rs`

### SandboxRouter

```rust
pub struct SandboxRouter {
    config: SandboxConfig,
    backends: Vec<Box<dyn Sandbox>>,
    primary_index: usize,
}

impl SandboxRouter {
    pub async fn route(&self, session_key: &str) -> &dyn Sandbox {
        // Determine which backend to use for this session
        // 1. Check agent-scoped override
        // 2. Check session-scoped override
        // 3. Fall back to primary backend
    }

    pub async fn is_sandboxed(&self, session_key: &str) -> bool {
        // Returns true if this session should run in sandbox
    }

    pub fn resolve_backend(&self, session_key: &str) -> SandboxBackend {
        // Returns the backend type (Docker, Podman, AppleContainer, etc.)
    }
}
```

### Sandbox Trait

```rust
#[async_trait]
pub trait Sandbox: Send + Sync {
    async fn execute(&self, command: &str, opts: ExecOpts) -> Result<ExecResult>;
    fn provides_fs_isolation(&self) -> bool;
    async fn cleanup(&self, id: &SandboxId) -> Result<()>;
}
```

### Backend Types

| Backend | Platform | FS Isolation | Config |
|---------|----------|--------------|--------|
| Docker | Linux, macOS | Yes | `docker.enabled` |
| Podman | Linux, macOS | Yes | `podman.enabled` |
| Apple Container | macOS | Yes | macOS only |
| Firecracker | Linux | Yes | `firecracker_bin` available |
| Vercel | Cloud | Yes | `vercel_token` set |
| Daytona | Cloud | Yes | `daytona_api_key` set |
| WASM | Cross-platform | Yes | `wasm.enabled` |
| RestrictedHost | Any | No | Fallback |
| Cgroup | Linux | No | Fallback |
| None | Any | No | Fallback |

### Filesystem Isolation Check

```rust
let has_container_backend = if let Some(ref router) = self.sandbox_router {
    router.resolve_backend(session_key).await.provides_fs_isolation()
} else {
    self.sandbox.provides_fs_isolation()
};
```

When the backend doesn't provide filesystem isolation (`RestrictedHost`, `Cgroup`, `None`), commands run directly on the host even when the session mode says "sandboxed."

---

## WASM Tools

**Source:** `crates/wasm-tools/`

### Calc Tool

**Source:** `crates/wasm-tools/calc/`

```rust
// Precompiled WASM calculator
// Tool name: "calc"
// Parameters: { "expression": "2 + 2" }
// Returns: { "result": 4 }
```

### Web Fetch Tool

**Source:** `crates/wasm-tools/web-fetch/`

```rust
// Precompiled WASM web fetcher
// Tool name: "web_fetch"
// Parameters: { "url": "https://example.com" }
// Returns: { "content": "...", "status": 200 }
```

### Web Search Tool

**Source:** `crates/wasm-tools/web-search/`

```rust
// Precompiled WASM web searcher
// Tool name: "web_search"
// Parameters: { "query": "Rust programming" }
// Returns: { "results": [...] }
```

---

## Other Tools

### File Read Tool

**Source:** `fs/read/tool.rs`

```json
{
  "name": "read",
  "parameters": {
    "path": { "type": "string" },
    "offset": { "type": "integer", "description": "Start line (0-based)" },
    "limit": { "type": "integer", "description": "Max lines" }
  }
}
```

### File Write Tool

**Source:** `fs/write.rs`

```json
{
  "name": "write",
  "parameters": {
    "path": { "type": "string" },
    "content": { "type": "string" },
    "append": { "type": "boolean", "default": false }
  }
}
```

### Glob Tool

**Source:** `fs/glob.rs`

```json
{
  "name": "glob",
  "parameters": {
    "pattern": { "type": "string" },
    "path": { "type": "string", "description": "Search root" }
  }
}
```

### Grep Tool

**Source:** `fs/grep.rs`

```json
{
  "name": "grep",
  "parameters": {
    "pattern": { "type": "string" },
    "path": { "type": "string" },
    "case_sensitive": { "type": "boolean" },
    "max_results": { "type": "integer", "default": 100 }
  }
}
```

---

## Approval Gating

**Source:** `approval.rs`

```rust
pub struct ApprovalManager { ... }

pub enum ApprovalDecision {
    Approve,
    Deny,
    DenyAndBlock,  // Block future similar commands
}
```

When a command requires approval:
1. `create_request()` stores the pending approval
2. `broadcast_request()` notifies connected clients
3. Client approves/denies via REST API
4. `await_response()` waits for decision (with timeout)

---

## Next

Continue to [06-memory](06-memory.md) for the memory system.
