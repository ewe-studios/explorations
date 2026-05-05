# yoke -- Tools

## AgentTool Trait

**File**: `yoagent/src/tools/mod.rs`

```rust
#[async_trait]
pub trait AgentTool: Send + Sync {
    fn name(&self) -> &str;
    fn label(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value, ctx: ToolContext) -> Result<ToolResult, ToolError>;
}
```

### ToolContext

```rust
pub struct ToolContext {
    pub cancel: CancellationToken,  // For cancellation support
}
```

### ToolResult

```rust
pub struct ToolResult {
    pub content: Vec<Content>,           // Output content (text, images)
    pub details: serde_json::Value,      // Machine-readable metadata
}
```

### ToolError

```rust
pub enum ToolError {
    InvalidArgs(String),
    Failed(String),
    Cancelled,
}
```

## Built-in Tools

### bash

**File**: `yoagent/src/tools/bash.rs`

Executes shell commands via `sh -c`.

```json
{"command": "ls -la src/", "timeout_ms": 30000}
```

- Default timeout: 30 seconds
- Captures stdout + stderr
- Returns exit code in details

### read_file

**File**: `yoagent/src/tools/file.rs`

Reads file content with line numbers.

```json
{"path": "src/main.rs", "offset": 0, "limit": 100}
```

- Adds line numbers (`1: content`)
- Supports offset/limit for partial reads
- Returns file size in details

### write_file

**File**: `yoagent/src/tools/file.rs`

Creates or overwrites a file.

```json
{"path": "src/new_file.rs", "content": "fn main() {}"}
```

- Creates parent directories if needed
- Returns bytes written

### edit_file

**File**: `yoagent/src/tools/edit.rs`

Search/replace editing.

```json
{"path": "src/main.rs", "old_string": "fn old()", "new_string": "fn new()"}
```

- Finds exact string match
- Replaces first occurrence
- Fails if old_string not found or ambiguous

### list_files

**File**: `yoagent/src/tools/list.rs`

Lists directory contents.

```json
{"path": "src/", "recursive": false}
```

- Optionally recursive
- Returns file names with type indicators

### search

**File**: `yoagent/src/tools/search.rs`

Pattern search (grep/ripgrep).

```json
{"pattern": "fn main", "path": "src/", "include": "*.rs"}
```

- Uses ripgrep if available, falls back to grep
- Supports file pattern filtering
- Returns matches with file:line:content format

### web_search

**File**: `yoagent/src/tools/web_search.rs`

Provider-side web search capability.

```json
{}
```

This is a marker tool — the actual search is executed server-side by the provider. yoke just signals to the provider that web search should be enabled.

## Tool Groups

The `--tools` flag accepts comma-separated tool names or groups:

| Group | Tools Included |
|-------|---------------|
| `all` | bash, read_file, write_file, edit_file, list_files, search, web_search, nu |
| `code` | bash, read_file, write_file, edit_file, list_files, search |
| `none` | (empty) |

Individual tools can be mixed: `--tools nu,read_file,search`

## Tool Selection Logic

**File**: `src/main.rs`

```rust
fn build_tools(spec: &str) -> Vec<Box<dyn AgentTool>> {
    let parts: Vec<&str> = spec.split(',').map(|s| s.trim()).collect();
    for part in &parts {
        match *part {
            "all" => { /* all tools + web_search + nu */ }
            "none" => { return Vec::new(); }
            "code" => { /* default_tools() */ }
            "bash" | "nu" | "read_file" | ... => { /* individual */ }
        }
    }
}
```

## MCP Tool Adapter

**File**: `yoagent/src/mcp/tool_adapter.rs`

Wraps MCP (Model Context Protocol) tools as `AgentTool`:

```rust
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool_name: String,
    description: String,
    schema: serde_json::Value,
}
```

This allows connecting to external MCP servers and using their tools as native yoagent tools.

## Sub-Agent Tool

**File**: `yoagent/src/sub_agent.rs`

Wraps another Agent as a tool:

```rust
pub struct SubAgentTool {
    name: String,
    description: String,
    agent: Agent,
}
```

When invoked, runs a complete agent loop as a nested tool call. Useful for delegation ("ask the research agent to find...").

## OpenAPI Tool Adapter

**File**: `yoagent/src/openapi/adapter.rs` (feature-gated: `openapi`)

Generates tools from OpenAPI specs:

```rust
pub fn tools_from_openapi(spec: &str) -> Result<Vec<Box<dyn AgentTool>>>
```

Parses OpenAPI v3 specs and creates tool definitions for each endpoint. The LLM can then call API endpoints as tools.

## Tool Definition (for providers)

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
}
```

Each provider translates `ToolDefinition` into its native format:
- Anthropic: `tools` array with `input_schema`
- Google: `functionDeclarations`
- OpenAI: `tools` array with `function` type
