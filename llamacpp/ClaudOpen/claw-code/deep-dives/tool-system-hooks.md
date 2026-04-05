# Tool System and Hooks Deep-Dive

A comprehensive analysis of how Claw Code executes tools, handles sandboxing, and implements pre/post execution hooks.

## Table of Contents

1. [Overview](#overview)
2. [Tool Specifications](#tool-specifications)
3. [Bash Execution](#bash-execution)
4. [Sandbox Implementation](#sandbox-implementation)
5. [File Operations](#file-operations)
6. [Hooks System](#hooks-system)
7. [Tool Execution Flow](#tool-execution-flow)
8. [Testing](#testing)

---

## Overview

Claw Code's tool system enables the AI to interact with the real world by:

- Executing bash commands with optional sandboxing
- Reading and writing files with structured patches
- Searching codebases with glob and grep
- Making web requests
- Running MCP tool servers

**Location**: `rust/crates/runtime/src/bash.rs`, `rust/crates/runtime/src/file_ops.rs`, `rust/crates/runtime/src/hooks.rs`, `rust/crates/runtime/src/sandbox.rs`

**MVP Tools** (15 built-in):
- `bash` - Command execution
- `read_file` - Read file contents
- `write_file` - Write/create files
- `edit_file` - Edit file contents
- `glob` - Pattern-based file search
- `grep` - Regex-based content search
- `curl` - Web requests
- `todo_write` - Task tracking
- `think` - Internal reasoning

---

## Tool Specifications

### Tool Definition Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
```

### MVP Tool Specs

```rust
pub fn mvp_tool_specs() -> Vec<ToolDefinition> {
    vec![
        // Bash
        ToolDefinition {
            name: String::from("bash"),
            description: String::from("Execute a bash command"),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The bash command to execute"
                    },
                    "background": {
                        "type": "boolean",
                        "description": "Run in background (default: false)"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 300)"
                    }
                },
                "required": ["command"]
            }),
        },

        // Read File
        ToolDefinition {
            name: String::from("read_file"),
            description: String::from("Read contents of a file"),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Start line (0-indexed)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum lines to read"
                    }
                },
                "required": ["path"]
            }),
        },

        // Write File
        ToolDefinition {
            name: String::from("write_file"),
            description: String::from("Write content to a file"),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write"
                    }
                },
                "required": ["path", "content"]
            }),
        },

        // Edit File
        ToolDefinition {
            name: String::from("edit_file"),
            description: String::from("Edit content of a file"),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "String to find and replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement string"
                    },
                    "replace_all": {
                        "type": "boolean",
                        "description": "Replace all occurrences (default: false)"
                    }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        },

        // Glob
        ToolDefinition {
            name: String::from("glob"),
            description: String::from("Search for files matching a pattern"),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern (e.g., **/*.rs)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory to search"
                    }
                },
                "required": ["pattern"]
            }),
        },

        // Grep
        ToolDefinition {
            name: String::from("grep"),
            description: String::from("Search for content using regex"),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search"
                    },
                    "path": {
                        "type": "string",
                        "description": "Base directory to search"
                    },
                    "glob": {
                        "type": "string",
                        "description": "Glob filter for file paths"
                    },
                    "output_mode": {
                        "type": "string",
                        "enum": ["files_with_matches", "content", "count"],
                        "description": "Output format"
                    },
                    "-n": {
                        "type": "boolean",
                        "description": "Show line numbers"
                    },
                    "-i": {
                        "type": "boolean",
                        "description": "Case insensitive"
                    },
                    "-C": {
                        "type": "integer",
                        "description": "Context lines"
                    }
                },
                "required": ["pattern"]
            }),
        },
    ]
}
```

---

## Bash Execution

### execute_bash Function

```rust
/// Execute a bash command
///
/// # Arguments
/// * `command` - The bash command to execute
/// * `background` - Whether to run in background
/// * `timeout` - Timeout in seconds (default: 300)
/// * `sandbox` - Whether to use sandbox
///
/// # Returns
/// Command output or error
pub fn execute_bash(
    command: &str,
    background: bool,
    timeout: Option<u64>,
    sandbox: bool,
) -> io::Result<BashOutput> {
    let timeout = timeout.unwrap_or(300);

    if sandbox {
        execute_in_sandbox(command, timeout)
    } else {
        execute_native(command, background, timeout)
    }
}

fn execute_native(
    command: &str,
    background: bool,
    timeout: u64,
) -> io::Result<BashOutput> {
    let mut child = std::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if background {
        // Return immediately with PID
        return Ok(BashOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            pid: Some(child.id()),
            background: true,
        });
    }

    // Wait with timeout
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout {
        if let Some(status) = child.try_wait()? {
            let output = child.wait_with_output()?;
            return Ok(BashOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: status.code(),
                pid: None,
                background: false,
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Timeout - kill process
    child.kill()?;
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("command timed out after {}s", timeout),
    ))
}
```

### BashOutput Structure

```rust
#[derive(Debug, Clone)]
pub struct BashOutput {
    /// Standard output
    pub stdout: String,

    /// Standard error
    pub stderr: String,

    /// Exit code (None if background)
    pub exit_code: Option<i32>,

    /// Process ID (for background processes)
    pub pid: Option<u32>,

    /// Whether running in background
    pub background: bool,
}

impl BashOutput {
    pub fn is_success(&self) -> bool {
        self.exit_code == Some(0)
    }

    pub fn is_error(&self) -> bool {
        self.exit_code.map_or(false, |code| code != 0)
    }

    pub fn combined_output(&self) -> String {
        let mut output = String::new();
        if !self.stdout.is_empty() {
            output.push_str(&self.stdout);
        }
        if !self.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&self.stderr);
        }
        output
    }
}
```

---

## Sandbox Implementation

### SandboxConfig

```rust
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Whether sandboxing is enabled
    pub enabled: bool,

    /// Namespace restrictions
    pub namespace_restrictions: NamespaceConfig,

    /// Network isolation
    pub network_isolation: NetworkMode,

    /// Filesystem access mode
    pub filesystem_mode: FilesystemIsolationMode,

    /// Allowed mount points
    pub allowed_mounts: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct NamespaceConfig {
    pub mount: bool,
    pub pid: bool,
    pub network: bool,
    pub uts: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    /// Full network access
    Enabled,
    /// No network access
    Disabled,
    /// Restricted to specific hosts
    Restricted(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemIsolationMode {
    /// Full filesystem access
    Full,
    /// Only workspace directory
    Workspace,
    /// Read-only access
    ReadOnly,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,  // Disabled by default
            namespace_restrictions: NamespaceConfig {
                mount: true,
                pid: false,
                network: false,
                uts: true,
            },
            network_isolation: NetworkMode::Enabled,
            filesystem_mode: FilesystemIsolationMode::Workspace,
            allowed_mounts: Vec::new(),
        }
    }
}
```

### Linux Sandbox Command

```rust
/// Build sandbox command using unshare
pub fn build_linux_sandbox_command(
    command: &str,
    config: &SandboxConfig,
    workspace_dir: &Path,
) -> std::process::Command {
    let mut sandbox = std::process::Command::new("unshare");

    // Namespace flags
    if config.namespace_restrictions.mount {
        sandbox.arg("--mount");
    }
    if config.namespace_restrictions.pid {
        sandbox.arg("--pid");
        sandbox.arg("--fork");  // Required for PID namespace
    }
    if config.namespace_restrictions.network {
        sandbox.arg("--net");
    }
    if config.namespace_restrictions.uts {
        sandbox.arg("--uts");
    }

    // Filesystem isolation
    match config.filesystem_mode {
        FilesystemIsolationMode::Workspace => {
            // Bind mount workspace to /workspace
            sandbox.arg("--bind");
            sandbox.arg(workspace_dir);
            sandbox.arg("/workspace");
        }
        FilesystemIsolationMode::ReadOnly => {
            sandbox.arg("--bind-ro");
            sandbox.arg("/");
            sandbox.arg("/");
        }
        FilesystemIsolationMode::Full => {}
    }

    // Network isolation
    if matches!(config.network_isolation, NetworkMode::Disabled) {
        // Use network namespace without any interfaces
        sandbox.arg("--net");
    }

    // Allowed mounts
    for mount in &config.allowed_mounts {
        sandbox.arg("--bind");
        sandbox.arg(mount);
        sandbox.arg(mount);
    }

    // Run bash command
    sandbox.arg("bash");
    sandbox.arg("-c");
    sandbox.arg(command);

    sandbox
}

fn execute_in_sandbox(command: &str, timeout: u64) -> io::Result<BashOutput> {
    let workspace_dir = std::env::current_dir()?;
    let config = SandboxConfig::default();

    let mut child = build_linux_sandbox_command(command, &config, &workspace_dir)
        .spawn()?;

    // Wait with timeout (same as native)
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout {
        if let Some(status) = child.try_wait()? {
            let output = child.wait_with_output()?;
            return Ok(BashOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: status.code(),
                pid: None,
                background: false,
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    child.kill()?;
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("command timed out after {}s", timeout),
    ))
}
```

---

## File Operations

### read_file

```rust
/// Read file contents with optional line range
pub fn read_file(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
) -> io::Result<ReadFileOutput> {
    let absolute_path = normalize_path(path)?;
    let content = fs::read_to_string(&absolute_path)?;
    let lines: Vec<&str> = content.lines().collect();

    // Calculate range
    let start_index = offset.unwrap_or(0).min(lines.len());
    let end_index = limit.map_or(lines.len(), |limit| {
        start_index.saturating_add(limit).min(lines.len())
    });

    let selected = lines[start_index..end_index].join("\n");

    Ok(ReadFileOutput {
        kind: String::from("text"),
        file: TextFilePayload {
            file_path: absolute_path.to_string_lossy().into_owned(),
            content: selected,
            num_lines: end_index.saturating_sub(start_index),
            start_line: start_index.saturating_add(1),
            total_lines: lines.len(),
        },
    })
}
```

### write_file

```rust
/// Write content to a file
pub fn write_file(path: &str, content: &str) -> io::Result<WriteFileOutput> {
    let absolute_path = normalize_path_allow_missing(path)?;
    let original_file = fs::read_to_string(&absolute_path).ok();

    // Create parent directories
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write file
    fs::write(&absolute_path, content)?;

    Ok(WriteFileOutput {
        kind: if original_file.is_some() {
            String::from("update")
        } else {
            String::from("create")
        },
        file_path: absolute_path.to_string_lossy().into_owned(),
        content: content.to_owned(),
        structured_patch: make_patch(original_file.as_deref().unwrap_or(""), content),
        original_file,
        git_diff: None,
    })
}
```

### edit_file

```rust
/// Edit file by replacing a string
pub fn edit_file(
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> io::Result<EditFileOutput> {
    let absolute_path = normalize_path(path)?;
    let original_file = fs::read_to_string(&absolute_path)?;

    // Validate input
    if old_string == new_string {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "old_string and new_string must differ",
        ));
    }

    if !original_file.contains(old_string) {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "old_string not found in file",
        ));
    }

    // Perform replacement
    let updated = if replace_all {
        original_file.replace(old_string, new_string)
    } else {
        original_file.replacen(old_string, new_string, 1)
    };

    // Write updated content
    fs::write(&absolute_path, &updated)?;

    Ok(EditFileOutput {
        file_path: absolute_path.to_string_lossy().into_owned(),
        old_string: old_string.to_owned(),
        new_string: new_string.to_owned(),
        original_file: original_file.clone(),
        structured_patch: make_patch(&original_file, &updated),
        user_modified: false,
        replace_all,
        git_diff: None,
    })
}
```

### Structured Patch

```rust
/// Create a simple structured patch (diff format)
fn make_patch(original: &str, updated: &str) -> Vec<StructuredPatchHunk> {
    let mut lines = Vec::new();

    // Removed lines
    for line in original.lines() {
        lines.push(format!("-{line}"));
    }

    // Added lines
    for line in updated.lines() {
        lines.push(format!("+{line}"));
    }

    vec![StructuredPatchHunk {
        old_start: 1,
        old_lines: original.lines().count(),
        new_start: 1,
        new_lines: updated.lines().count(),
        lines,
    }]
}
```

### Path Normalization

```rust
/// Normalize and canonicalize a path (must exist)
fn normalize_path(path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };
    candidate.canonicalize()
}

/// Normalize path, allowing missing final component
fn normalize_path_allow_missing(path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };

    // Try to canonicalize
    if let Ok(canonical) = candidate.canonicalize() {
        return Ok(canonical);
    }

    // If file doesn't exist, canonicalize parent
    if let Some(parent) = candidate.parent() {
        let canonical_parent = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        if let Some(name) = candidate.file_name() {
            return Ok(canonical_parent.join(name));
        }
    }

    Ok(candidate)
}
```

---

## Hooks System

### HookEvent Enum

```rust
#[derive(Debug, Clone)]
pub enum HookEvent {
    /// Before tool execution
    PreToolUse {
        tool_name: String,
        tool_input: String,
    },
    /// After tool execution
    PostToolUse {
        tool_name: String,
        tool_input: String,
        tool_output: String,
        is_error: bool,
    },
}
```

### HookRunner

```rust
#[derive(Debug, Clone)]
pub struct HookRunner {
    /// Pre-use hook command
    pub pre_hook: Option<String>,

    /// Post-use hook command
    pub post_hook: Option<String>,
}

impl HookRunner {
    pub fn new(pre_hook: Option<String>, post_hook: Option<String>) -> Self {
        Self { pre_hook, post_hook }
    }

    /// Run pre-tool-use hook
    pub fn run_pre_tool_use(
        &self,
        tool_name: &str,
        tool_input: &str,
    ) -> io::Result<HookResult> {
        let Some(command) = &self.pre_hook else {
            return Ok(HookResult::Continue);
        };

        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .env("HOOK_EVENT", "pre_tool_use")
            .env("HOOK_TOOL_NAME", tool_name)
            .env("HOOK_TOOL_INPUT", tool_input)
            .output()?;

        if !output.status.success() {
            return Ok(HookResult::Deny {
                reason: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        Ok(HookResult::Continue)
    }

    /// Run post-tool-use hook
    pub fn run_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &str,
        tool_output: &str,
        is_error: bool,
    ) -> io::Result<HookResult> {
        let Some(command) = &self.post_hook else {
            return Ok(HookResult::Continue);
        };

        std::process::Command::new("bash")
            .arg("-c")
            .arg(command)
            .env("HOOK_EVENT", "post_tool_use")
            .env("HOOK_TOOL_NAME", tool_name)
            .env("HOOK_TOOL_INPUT", tool_input)
            .env("HOOK_TOOL_OUTPUT", tool_output)
            .env("HOOK_TOOL_IS_ERROR", is_error.to_string())
            .output()?;

        Ok(HookResult::Continue)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookResult {
    /// Continue with tool execution
    Continue,
    /// Deny tool execution
    Deny { reason: String },
}
```

### Hook Configuration

Hooks are configured in `.claude/settings.json`:

```json
{
  "hooks": {
    "pre_use_tool": "~/.claude/hooks/pre_use_tool.sh",
    "post_use_tool": "~/.claude/hooks/post_use_tool.sh"
  }
}
```

### Environment Variables for Hooks

| Variable | Event | Description |
|----------|-------|-------------|
| `HOOK_EVENT` | Both | `pre_tool_use` or `post_tool_use` |
| `HOOK_TOOL_NAME` | Both | Name of the tool |
| `HOOK_TOOL_INPUT` | Both | JSON input to the tool |
| `HOOK_TOOL_OUTPUT` | Post | Tool output |
| `HOOK_TOOL_IS_ERROR` | Post | Whether tool failed |

### Example Hook Scripts

**Pre-use hook** (log all tool calls):

```bash
#!/bin/bash
echo "[$(date)] Tool: $HOOK_TOOL_NAME" >> ~/.claude/tool_log.txt
echo "Input: $HOOK_TOOL_INPUT" >> ~/.claude/tool_log.txt
```

**Post-use hook** (alert on destructive commands):

```bash
#!/bin/bash
if [ "$HOOK_TOOL_NAME" = "bash" ] && [ "$HOOK_TOOL_IS_ERROR" = "false" ]; then
    if echo "$HOOK_TOOL_INPUT" | grep -qE "(rm|sudo|chmod|chown)"; then
        echo "Destructive command executed:" >> ~/.claude/alerts.txt
        echo "$HOOK_TOOL_INPUT" >> ~/.claude/alerts.txt
    fi
fi
```

---

## Tool Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    ConversationRuntime::run_turn()              │
│                                                                 │
│  1. Send user message + session to API                          │
│  2. Receive response with tool_use blocks                       │
│  └─────────────────────────────────────────────────────────────┘
│                              │
│                              ▼
│  ┌─────────────────────────────────────────────────────────────┐
│  │              For each tool_use block:                       │
│  │                                                             │
│  │  ┌───────────────────────────────────────────────────────┐ │
│  │  │ 1. Pre-hook Execution (HookRunner::run_pre_tool_use)  │ │
│  │  │    - Set environment variables                        │ │
│  │  │    - Run pre_hook command if configured               │ │
│  │  │    - Check for Deny result                            │ │
│  │  └───────────────────────────────────────────────────────┘ │
│  │                            │                                │
│  │                    ┌───────┴───────┐                       │
│  │                    │               │                       │
│  │                    ▼               ▼                       │
│  │              ┌──────────┐   ┌──────────────┐              │
│  │              │ Continue │   │    Deny      │              │
│  │              └────┬─────┘   └──────────────┘              │
│  │                   │                                        │
│  │                   ▼                                        │
│  │  ┌───────────────────────────────────────────────────────┐ │
│  │  │ 2. Tool Execution (StaticToolExecutor::execute)       │ │
│  │  │    - Parse tool name and input                        │ │
│  │  │    - Dispatch to appropriate handler:                 │ │
│  │  │      * bash → execute_bash()                          │ │
│  │  │      * read_file → read_file()                        │ │
│  │  │      * write_file → write_file()                      │ │
│  │  │      * edit_file → edit_file()                        │ │
│  │  │      * glob → glob_search()                           │ │
│  │  │      * grep → grep_search()                           │ │
│  │  │      * etc.                                           │ │
│  │  └───────────────────────────────────────────────────────┘ │
│  │                            │                                │
│  │                            ▼                                │
│  │  ┌───────────────────────────────────────────────────────┐ │
│  │  │ 3. Post-hook Execution (HookRunner::run_post_tool_use)│ │
│  │  │    - Set environment variables                        │ │
│  │  │    - Run post_hook command if configured              │ │
│  │  └───────────────────────────────────────────────────────┘ │
│  │                            │                                │
│  │                            ▼                                │
│  │  ┌───────────────────────────────────────────────────────┐ │
│  │  │ 4. Add Tool Result to Session                         │ │
│  │  │    - Create ToolResult content block                  │ │
│  │  │    - Include output or error message                  │ │
│  │  └───────────────────────────────────────────────────────┘ │
│  └─────────────────────────────────────────────────────────────┘
│                              │
│                              ▼
│  ┌─────────────────────────────────────────────────────────────┐
│  │ 3. Continue conversation turn with tool results            │
│  │    - Send tool results back to API                          │
│  │    - Receive final response                                 │
│  └─────────────────────────────────────────────────────────────┘
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_files() {
        let path = std::env::temp_dir().join("test-read-write.txt");
        let write_output = write_file(path.to_string_lossy().as_ref(), "one\ntwo\nthree")
            .expect("write should succeed");
        assert_eq!(write_output.kind, "create");

        let read_output = read_file(path.to_string_lossy().as_ref(), Some(1), Some(1))
            .expect("read should succeed");
        assert_eq!(read_output.file.content, "two");

        fs::remove_file(path).ok();
    }

    #[test]
    fn edits_file_contents() {
        let path = std::env::temp_dir().join("test-edit.txt");
        write_file(path.to_string_lossy().as_ref(), "alpha beta alpha")
            .expect("initial write should succeed");

        let output = edit_file(
            path.to_string_lossy().as_ref(),
            "alpha",
            "omega",
            true,
        ).expect("edit should succeed");

        assert!(output.replace_all);
        assert!(output.original_file.contains("alpha"));
        assert!(output.new_string.contains("omega"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn globs_and_greps_directory() {
        let dir = std::env::temp_dir().join("test-search-dir");
        fs::create_dir_all(&dir).expect("directory should be created");

        let file = dir.join("demo.rs");
        write_file(
            file.to_string_lossy().as_ref(),
            "fn main() {\n println!(\"hello\");\n}\n",
        ).expect("file write should succeed");

        let globbed = glob_search("**/*.rs", Some(dir.to_string_lossy().as_ref()))
            .expect("glob should succeed");
        assert_eq!(globbed.num_files, 1);

        let grep_output = grep_search(&GrepSearchInput {
            pattern: String::from("hello"),
            path: Some(dir.to_string_lossy().into_owned()),
            glob: Some(String::from("**/*.rs")),
            output_mode: Some(String::from("content")),
            before: None,
            after: None,
            context_short: None,
            context: None,
            line_numbers: Some(true),
            case_insensitive: Some(false),
            file_type: None,
            head_limit: Some(10),
            offset: Some(0),
            multiline: Some(false),
        }).expect("grep should succeed");

        assert!(grep_output.content.unwrap_or_default().contains("hello"));

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn hook_runner_respects_deny() {
        let runner = HookRunner::new(
            Some("exit 1".to_string()),  // Always fail
            None,
        );

        let result = runner.run_pre_tool_use("bash", "{\"command\": \"ls\"}")
            .expect("hook should run");

        assert!(matches!(result, HookResult::Deny { .. }));
    }
}
```

---

## Related Files

| File | Purpose |
|------|---------|
| `rust/crates/runtime/src/bash.rs` | Bash execution with sandbox |
| `rust/crates/runtime/src/file_ops.rs` | File read/write/edit operations |
| `rust/crates/runtime/src/hooks.rs` | Pre/post hook execution |
| `rust/crates/runtime/src/sandbox.rs` | Sandbox configuration |
| `rust/crates/runtime/src/tools.rs` | Tool specifications |
| `rust/crates/runtime/src/conversation.rs` | Tool execution integration |

---

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAWD_SANDBOX_ENABLED` | Enable sandboxing | false |
| `CLAWD_BASH_TIMEOUT` | Default bash timeout (seconds) | 300 |
| `CLAWD_WORKSPACE` | Workspace directory | current dir |
| `CLAWD_PRE_HOOK` | Pre-tool hook command | None |
| `CLAWD_POST_HOOK` | Post-tool hook command | None |
