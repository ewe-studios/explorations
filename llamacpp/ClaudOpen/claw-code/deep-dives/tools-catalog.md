# Tools Catalog: Detailed Analysis of Each Built-in Tool

## Executive Summary

This document provides comprehensive documentation for each built-in tool in claw-code. For each tool, we cover its purpose, use cases, implementation details, security considerations, edge cases, failure modes, and code examples.

**Source Reference:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/`

---

## Table of Contents

1. [bash](#bash)
2. [read_file](#read_file)
3. [write_file](#write_file)
4. [edit_file](#edit_file)
5. [glob_search](#glob_search)
6. [grep_search](#grep_search)
7. [WebFetch](#webfetch)
8. [WebSearch](#websearch)
9. [TodoWrite](#todowrite)
10. [Skill](#skill)
11. [Agent](#agent)
12. [ToolSearch](#toolsearch)
13. [NotebookEdit](#notebookedit)
14. [Sleep](#sleep)
15. [SendUserMessage/Brief](#sendusermessagebrief)
16. [Config](#config)
17. [StructuredOutput](#structuredoutput)
18. [REPL](#repl)
19. [PowerShell](#powershell)

---

## bash

### Purpose

Execute shell commands in the current workspace with Linux sandboxing.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "command": { "type": "string" },
    "timeout": { "type": "integer", "minimum": 1 },
    "description": { "type": "string" },
    "run_in_background": { "type": "boolean" },
    "dangerouslyDisableSandbox": { "type": "boolean" },
    "namespaceRestrictions": { "type": "boolean" },
    "isolateNetwork": { "type": "boolean" },
    "filesystemMode": { "type": "string", "enum": ["off", "workspace-only", "allow-list"] },
    "allowedMounts": { "type": "array", "items": { "type": "string" } }
  },
  "required": ["command"]
}
```

### Implementation Details

**Source Files:**
- `rust/crates/runtime/src/bash.rs` - Main execution logic
- `rust/crates/runtime/src/sandbox.rs` - Sandbox implementation

**Key Functions:**
```rust
// Main entry point
pub fn execute_bash(input: BashCommandInput) -> io::Result<BashCommandOutput>

// Sandbox resolution
fn sandbox_status_for_input(input: &BashCommandInput, cwd: &Path) -> SandboxStatus

// Command preparation
fn prepare_command(command: &str, cwd: &Path, sandbox_status: &SandboxStatus, create_dirs: bool) -> Command
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Command injection | Sandboxed via unshare() namespaces |
| Path traversal | WorkspaceOnly filesystem mode |
| Resource exhaustion | Timeout option, recommended cgroups |
| Network abuse | Optional --net namespace isolation |
| Privilege escalation | User namespace maps to unprivileged |

### Edge Cases

1. **Empty command**: Valid but produces no output
2. **Timeout**: Returns `interrupted: true` with timeout message
3. **Background mode**: Returns PID, no output captured
4. **Sandbox unavailable**: Falls back to environment restrictions

### Failure Modes

| Failure | Response |
|---------|----------|
| Command not found | stderr contains error, exit_code != 0 |
| Permission denied | stderr contains error, exit_code != 0 |
| Timeout | `interrupted: true`, stderr has timeout message |
| Sandbox setup fails | Fallback to non-sandboxed execution |

### Code Examples

```rust
// Basic command
BashCommandInput {
    command: "ls -la".to_string(),
    timeout: Some(5000),
    ..Default::default()
}

// With sandbox disabled (dangerous!)
BashCommandInput {
    command: "systemctl status".to_string(),
    dangerously_disable_sandbox: Some(true),
    ..Default::default()
}

// Background task
BashCommandInput {
    command: "cargo build --release".to_string(),
    run_in_background: Some(true),
    ..Default::default()
}
```

---

## read_file

### Purpose

Read content from text files with optional line offset and limit.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string" },
    "offset": { "type": "integer", "minimum": 0 },
    "limit": { "type": "integer", "minimum": 1 }
  },
  "required": ["path"]
}
```

### Implementation Details

**Source File:** `rust/crates/runtime/src/file_ops.rs`

**Key Function:**
```rust
pub fn read_file(
    path: &str,
    offset: Option<usize>,
    limit: Option<usize>,
) -> io::Result<ReadFileOutput>
```

**Output Structure:**
```rust
pub struct ReadFileOutput {
    pub kind: String,      // "text"
    pub file: TextFilePayload,
}

pub struct TextFilePayload {
    pub file_path: String,
    pub content: String,
    pub num_lines: usize,
    pub start_line: usize,
    pub total_lines: usize,
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Path traversal | canonicalize() resolves .., sandbox blocks |
| Symlink attacks | canonicalize() resolves symlinks |
| Large files | offset/limit for pagination |
| Binary files | UTF-8 conversion (may fail) |

### Edge Cases

1. **File not found**: Returns `io::ErrorKind::NotFound`
2. **Permission denied**: Returns `io::ErrorKind::PermissionDenied`
3. **Binary file**: UTF-8 conversion may produce replacement chars
4. **Offset beyond EOF**: Returns empty content
5. **Empty file**: Returns content with 0 lines

### Failure Modes

| Failure | Response |
|---------|----------|
| Path doesn't exist | Error: "No such file or directory" |
| Not a file | Error on canonicalize |
| Permission denied | Error: "Permission denied" |
| Too large | May cause memory pressure |

### Code Examples

```rust
// Read entire file
read_file("README.md", None, None)

// Read lines 10-20 (offset 9, limit 10)
read_file("src/main.rs", Some(9), Some(10))

// Read first 5 lines
read_file("config.yaml", Some(0), Some(5))
```

---

## write_file

### Purpose

Create or overwrite files in the workspace.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string" },
    "content": { "type": "string" }
  },
  "required": ["path", "content"]
}
```

### Implementation Details

**Source File:** `rust/crates/runtime/src/file_ops.rs`

**Key Function:**
```rust
pub fn write_file(path: &str, content: &str) -> io::Result<WriteFileOutput>
```

**Output Structure:**
```rust
pub struct WriteFileOutput {
    pub kind: String,           // "create" or "update"
    pub file_path: String,
    pub content: String,
    pub structured_patch: Vec<StructuredPatchHunk>,
    pub original_file: Option<String>,
    pub git_diff: Option<JsonValue>,
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Overwriting sensitive files | WorkspaceWrite permission, sandbox |
| Path traversal | canonicalize() + sandbox |
| Directory creation | Creates parent dirs automatically |
| Large writes | No explicit limit (memory bound) |

### Edge Cases

1. **Parent directory doesn't exist**: Auto-created
2. **File exists**: Overwritten, original saved in output
3. **Symbolic link**: Resolved before write
4. **Read-only filesystem**: Returns error

### Failure Modes

| Failure | Response |
|---------|----------|
| Parent path is file | Error creating directory |
| No write permission | Error: "Permission denied" |
| Disk full | Error: "No space left on device" |
| Path is directory | Error |

### Code Examples

```rust
// Create new file
write_file("output.txt", "Hello, World!")

// Overwrite existing file
write_file("config.json", r#"{"key": "value"}"#)

// Create nested path
write_file("src/new/file.rs", "fn main() {}")
```

---

## edit_file

### Purpose

Replace text in existing files with optional replace-all.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "path": { "type": "string" },
    "old_string": { "type": "string" },
    "new_string": { "type": "string" },
    "replace_all": { "type": "boolean" }
  },
  "required": ["path", "old_string", "new_string"]
}
```

### Implementation Details

**Source File:** `rust/crates/runtime/src/file_ops.rs`

**Key Function:**
```rust
pub fn edit_file(
    path: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> io::Result<EditFileOutput>
```

**Validation:**
```rust
// old_string and new_string must differ
if old_string == new_string {
    return Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "old_string and new_string must differ",
    ));
}

// old_string must exist in file
if !original_file.contains(old_string) {
    return Err(io::Error::new(
        io::ErrorKind::NotFound,
        "old_string not found in file",
    ));
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Arbitrary file modification | WorkspaceWrite permission |
| Path traversal | canonicalize() + sandbox |
| Unintended replacements | Single replacement by default |
| Large replacements | No limit (memory bound) |

### Edge Cases

1. **Multiple occurrences**: `replace_all: true` replaces all
2. **String not found**: Returns error
3. **Same old/new**: Returns error
4. **Empty old_string**: Would match at every position (error)

### Failure Modes

| Failure | Response |
|---------|----------|
| File not found | Error: "No such file" |
| String not found | Error: "old_string not found" |
| Same strings | Error: "must differ" |
| Write permission denied | Error on fs::write |

### Code Examples

```rust
// Single replacement
edit_file("main.rs", "fn old()", "fn new()", false)

// Replace all occurrences
edit_file("config.py", "DEBUG = True", "DEBUG = False", true)

// Multi-line replacement
edit_file("README.md", "# Old\n## Section", "# New\n## Section", false)
```

---

## glob_search

### Purpose

Find files matching glob patterns.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "pattern": { "type": "string" },
    "path": { "type": "string" }
  },
  "required": ["pattern"]
}
```

### Implementation Details

**Source File:** `rust/crates/runtime/src/file_ops.rs`

**Key Function:**
```rust
pub fn glob_search(pattern: &str, path: Option<&str>) -> io::Result<GlobSearchOutput>
```

**Output Structure:**
```rust
pub struct GlobSearchOutput {
    pub duration_ms: u128,
    pub num_files: usize,
    pub filenames: Vec<String>,
    pub truncated: bool,  // true if > 100 results
}
```

**Sorting:** Results sorted by modification time (newest first)

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Directory traversal in pattern | Pattern resolved relative to base |
| Glob injection | Standard glob crate, no shell expansion |
| Large result sets | Truncated to 100 files |
| Symlink loops | glob crate handles loops |

### Edge Cases

1. **Absolute pattern**: Used as-is
2. **No matches**: Returns empty list
3. **More than 100 results**: Truncated, `truncated: true`
4. **Invalid glob pattern**: Error from glob crate

### Failure Modes

| Failure | Response |
|---------|----------|
| Invalid pattern | Error: "Pattern syntax error" |
| Base path not found | Error: "No such directory" |
| Permission denied | Skips inaccessible directories |

### Code Examples

```rust
// Find all Rust files
glob_search("**/*.rs", None)

// Find Python files in specific directory
glob_search("src/**/*.py", Some("/project"))

// Find all JSON files
glob_search("**/*.json", None)
```

---

## grep_search

### Purpose

Search file contents using regex patterns.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "pattern": { "type": "string" },
    "path": { "type": "string" },
    "glob": { "type": "string" },
    "output_mode": { "type": "string", "enum": ["files_with_matches", "content", "count"] },
    "-B": { "type": "integer", "minimum": 0 },
    "-A": { "type": "integer", "minimum": 0 },
    "-C": { "type": "integer", "minimum": 0 },
    "context": { "type": "integer", "minimum": 0 },
    "-n": { "type": "boolean" },
    "-i": { "type": "boolean" },
    "type": { "type": "string" },
    "head_limit": { "type": "integer", "minimum": 1 },
    "offset": { "type": "integer", "minimum": 0 },
    "multiline": { "type": "boolean" }
  },
  "required": ["pattern"]
}
```

### Implementation Details

**Source File:** `rust/crates/runtime/src/file_ops.rs`

**Key Function:**
```rust
pub fn grep_search(input: &GrepSearchInput) -> io::Result<GrepSearchOutput>
```

**Output Modes:**
- `files_with_matches`: List of files containing matches
- `content`: Matching lines with context
- `count`: Match count per file

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| ReDoS (regex DoS) | regex crate has safety limits |
| Large file scanning | Default limit 250 results |
| Path traversal | Normalized paths |
| Binary file reading | May produce garbage output |

### Edge Cases

1. **Invalid regex**: Returns error
2. **No matches**: Empty results
3. **Very large files**: May cause memory pressure
4. **Binary files**: May match binary patterns

### Failure Modes

| Failure | Response |
|---------|----------|
| Invalid regex | Error: "Regex syntax error" |
| Path not found | Error from walkdir |
| Permission denied | Skips inaccessible files |

### Code Examples

```rust
// Find TODO comments
grep_search(&GrepSearchInput {
    pattern: "TODO".to_string(),
    output_mode: Some("content".to_string()),
    line_numbers: Some(true),
    ..Default::default()
})

// Case-insensitive search for function definitions
grep_search(&GrepSearchInput {
    pattern: "fn \\w+".to_string(),
    case_insensitive: Some(false),
    file_type: Some("rs".to_string()),
    ..Default::default()
})

// Count matches
grep_search(&GrepSearchInput {
    pattern: "error".to_string(),
    output_mode: Some("count".to_string()),
    ..Default::default()
})
```

---

## WebFetch

### Purpose

Fetch URL content, convert to readable text, and answer prompts about it.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "url": { "type": "string", "format": "uri" },
    "prompt": { "type": "string" }
  },
  "required": ["url", "prompt"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Key Functions:**
```rust
fn execute_web_fetch(input: &WebFetchInput) -> Result<WebFetchOutput, String>
fn normalize_fetch_url(url: &str) -> Result<String, String>
fn html_to_text(html: &str) -> String
fn summarize_web_fetch(url: &str, prompt: &str, content: &str, body: &str, content_type: &str) -> String
```

**Output Structure:**
```rust
pub struct WebFetchOutput {
    pub bytes: usize,
    pub code: u16,
    pub code_text: String,
    pub result: String,
    pub duration_ms: u128,
    pub url: String,
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| SSRF (Server-Side Request Forgery) | URL scheme validation (http/https only) |
| Internal network access | Depends on network sandbox |
| Large responses | No explicit limit |
| Redirect loops | reqwest handles redirects |

### Edge Cases

1. **Non-HTTP URL**: Rejected (only http/https)
2. **Missing scheme**: Prepends https://
3. **HTML content**: Converted to plain text
4. **Non-text content**: May produce garbage

### Failure Modes

| Failure | Response |
|---------|----------|
| Invalid URL | Error: "Invalid URL" |
| Connection failed | Error from reqwest |
| Timeout | reqwest timeout error |
| Non-200 response | Returns with status code |

### Code Examples

```rust
// Fetch documentation
WebFetchInput {
    url: "https://doc.rust-lang.org/book/".to_string(),
    prompt: "Summarize the main topics covered".to_string(),
}

// Fetch API response
WebFetchInput {
    url: "https://api.example.com/data".to_string(),
    prompt: "Extract the user names".to_string(),
}
```

---

## WebSearch

### Purpose

Search the web and return cited results.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "query": { "type": "string", "minLength": 2 },
    "allowed_domains": { "type": "array", "items": { "type": "string" } },
    "blocked_domains": { "type": "array", "items": { "type": "string" } }
  },
  "required": ["query"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Key Functions:**
```rust
fn execute_web_search(input: &WebSearchInput) -> Result<WebSearchOutput, String>
fn extract_search_hits(html: &str) -> Vec<SearchHit>
fn host_matches_list(url: &str, domains: &[String]) -> bool
```

**Search Backend:** DuckDuckGo HTML search

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Domain filtering | allowed_domains/blocked_domains lists |
| Search query injection | Minimal validation |
| Result parsing | HTML parsing, no script execution |

### Edge Cases

1. **Empty query**: Rejected (minLength: 2)
2. **No results**: Empty results list
3. **All domains blocked**: Empty results

### Code Examples

```rust
// Basic search
WebSearchInput {
    query: "Rust programming language".to_string(),
    allowed_domains: None,
    blocked_domains: None,
}

// Domain-restricted search
WebSearchInput {
    query: "documentation".to_string(),
    allowed_domains: Some(vec!["doc.rust-lang.org".to_string()]),
    blocked_domains: None,
}
```

---

## TodoWrite

### Purpose

Manage task lists for the current session.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "todos": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "content": { "type": "string" },
          "activeForm": { "type": "string" },
          "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] }
        },
        "required": ["content", "activeForm", "status"]
      }
    }
  },
  "required": ["todos"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Storage:** `.clawd-todos.json` in current directory

**Validation:**
```rust
fn validate_todos(todos: &[TodoItem]) -> Result<(), String> {
    if todos.is_empty() {
        return Err("todos must not be empty".to_string());
    }
    if todos.iter().any(|todo| todo.content.trim().is_empty()) {
        return Err("todo content must not be empty".to_string());
    }
    if todos.iter().any(|todo| todo.active_form.trim().is_empty()) {
        return Err("todo activeForm must not be empty".to_string());
    }
    Ok(())
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| File system writes | WorkspaceWrite permission |
| Large todo lists | No explicit limit |

### Edge Cases

1. **All completed**: Store clears, verification nudge shown
2. **Empty todos**: Rejected by validation
3. **Multiple in_progress**: Allowed for parallel workflows

### Code Examples

```rust
// Set up task list
TodoWriteInput {
    todos: vec![
        TodoItem {
            content: "Implement feature X".to_string(),
            active_form: "Implementing feature X".to_string(),
            status: TodoStatus::InProgress,
        },
        TodoItem {
            content: "Write tests".to_string(),
            active_form: "Writing tests".to_string(),
            status: TodoStatus::Pending,
        },
    ],
}

// Mark all complete
TodoWriteInput {
    todos: vec![
        TodoItem {
            content: "Task 1".to_string(),
            active_form: "Completed task 1".to_string(),
            status: TodoStatus::Completed,
        },
    ],
}
```

---

## Skill

### Purpose

Load local skill definitions and their instructions.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "skill": { "type": "string" },
    "args": { "type": "string" }
  },
  "required": ["skill"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Search Locations:**
1. `$CODEX_HOME/skills/{skill}/SKILL.md`
2. `/home/bellman/.codex/skills/{skill}/SKILL.md`

**Output:**
```rust
pub struct SkillOutput {
    pub skill: String,
    pub path: String,
    pub args: Option<String>,
    pub description: Option<String>,
    pub prompt: String,
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Arbitrary file read | Restricted to skill directories |
| Path traversal | Skill name is sanitized |

### Edge Cases

1. **Skill not found**: Returns error
2. **Empty skill file**: Returns empty prompt
3. **Case sensitivity**: Case-insensitive matching

### Code Examples

```rust
// Load a skill
SkillInput {
    skill: "code-review".to_string(),
    args: None,
}

// Load with arguments
SkillInput {
    skill: "refactor".to_string(),
    args: Some("--aggressive".to_string()),
}
```

---

## Agent

### Purpose

Spawn specialized sub-agents for delegated tasks.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "description": { "type": "string" },
    "prompt": { "type": "string" },
    "subagent_type": { "type": "string" },
    "name": { "type": "string" },
    "model": { "type": "string" }
  },
  "required": ["description", "prompt"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Subagent Types:**
- `Explore` - Read-only exploration
- `Plan` - Planning and task management
- `Verification` - Testing and validation
- `claw-code-guide` - Documentation help
- `statusline-setup` - Configuration tasks
- Default - Full tool access

**Tool Restrictions by Type:**
```rust
fn allowed_tools_for_subagent(subagent_type: &str) -> BTreeSet<String> {
    match subagent_type {
        "Explore" => vec!["read_file", "glob_search", "grep_search", "WebFetch", ...],
        "Plan" => vec!["read_file", "glob_search", "TodoWrite", ...],
        "Verification" => vec!["bash", "read_file", "PowerShell", ...],
        // ...
    }
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Privilege escalation | Subagents have restricted tool sets |
| Resource exhaustion | Max iterations (32) |
| Infinite delegation | No recursive agent spawning check |

### Edge Cases

1. **Unknown subagent_type**: Falls back to default (full access)
2. **Empty description**: Rejected
3. **Empty prompt**: Rejected

### Code Examples

```rust
// Exploration agent
AgentInput {
    description: "Explore the codebase structure".to_string(),
    prompt: "Find all Python files and summarize their purpose".to_string(),
    subagent_type: Some("Explore".to_string()),
    model: None,
    name: None,
}

// Verification agent
AgentInput {
    description: "Run tests and verify functionality".to_string(),
    prompt: "Run the test suite and report any failures".to_string(),
    subagent_type: Some("Verification".to_string()),
    ..Default::default()
}
```

---

## ToolSearch

### Purpose

Find available tools by name or keywords.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "query": { "type": "string" },
    "max_results": { "type": "integer", "minimum": 1 }
  },
  "required": ["query"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Search Scope:**
- Built-in tool names
- MCP tool names
- Deferred tools

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Information disclosure | Only shows available tools |

### Code Examples

```rust
// Search for file tools
ToolSearchInput {
    query: "file".to_string(),
    max_results: Some(10),
}
```

---

## NotebookEdit

### Purpose

Edit Jupyter notebook cells.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "notebook_path": { "type": "string" },
    "cell_id": { "type": "string" },
    "new_source": { "type": "string" },
    "cell_type": { "type": "string", "enum": ["code", "markdown"] },
    "edit_mode": { "type": "string", "enum": ["replace", "insert", "delete"] }
  },
  "required": ["notebook_path"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Validation:**
```rust
// Must be .ipynb file
if path.extension().and_then(|ext| ext.to_str()) != Some("ipynb") {
    return Err("notebook_path must point to a .ipynb file".to_string());
}

// cell_id required for non-insert operations
if edit_mode != Some(Insert) && cell_id.is_none() {
    return Err("cell_id is required".to_string());
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Arbitrary file write | WorkspaceWrite permission, .ipynb extension check |
| Path traversal | canonicalize() + sandbox |

### Code Examples

```rust
// Replace cell content
NotebookEditInput {
    notebook_path: "analysis.ipynb".to_string(),
    cell_id: Some("abc123".to_string()),
    new_source: Some("print('hello')".to_string()),
    edit_mode: Some(NotebookEditMode::Replace),
    cell_type: Some(NotebookCellType::Code),
}

// Delete cell
NotebookEditInput {
    notebook_path: "analysis.ipynb".to_string(),
    cell_id: Some("xyz789".to_string()),
    edit_mode: Some(NotebookEditMode::Delete),
    ..Default::default()
}
```

---

## Sleep

### Purpose

Wait for a duration without holding a shell process.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "duration_ms": { "type": "integer", "minimum": 0 }
  },
  "required": ["duration_ms"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Implementation:**
```rust
fn run_sleep(input: SleepInput) -> Result<String, String> {
    std::thread::sleep(Duration::from_millis(input.duration_ms));
    to_pretty_json(&SleepOutput {
        duration_ms: input.duration_ms,
        message: format!("Slept for {} ms", input.duration_ms),
    })
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| DoS via long sleep | No explicit limit (could be added) |

### Code Examples

```rust
// Wait 5 seconds
SleepInput {
    duration_ms: 5000,
}
```

---

## SendUserMessage/Brief

### Purpose

Send messages to the user with optional attachments.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "message": { "type": "string" },
    "attachments": { "type": "array", "items": { "type": "string" } },
    "status": { "type": "string", "enum": ["normal", "proactive"] }
  },
  "required": ["message", "status"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Attachment Handling:**
```rust
fn resolve_attachments(paths: &[String]) -> Vec<ResolvedAttachment> {
    paths.iter().filter_map(|path| {
        let metadata = std::fs::metadata(path).ok()?;
        Some(ResolvedAttachment {
            path: path.clone(),
            size: metadata.len(),
            is_image: path.ends_with(".png") || path.ends_with(".jpg"),
        })
    }).collect()
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Path disclosure | Only shows file size and type |
| Large attachments | No explicit limit |

### Code Examples

```rust
// Normal message
BriefInput {
    message: "Task completed successfully".to_string(),
    attachments: None,
    status: BriefStatus::Normal,
}

// With attachment
BriefInput {
    message: "Here's the generated image".to_string(),
    attachments: Some(vec!["output.png".to_string()]),
    status: BriefStatus::Normal,
}
```

---

## Config

### Purpose

Get or set claw-code configuration settings.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "setting": { "type": "string" },
    "value": { "type": ["string", "boolean", "number"] }
  },
  "required": ["setting"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Supported Settings:**
- `theme`, `editorMode`, `verbose`
- `autoCompactEnabled`, `autoMemoryEnabled`
- `fileCheckpointingEnabled`, `showTurnDuration`
- `terminalProgressBarEnabled`, `todoFeatureEnabled`
- `model`, `alwaysThinkingEnabled`
- `permissions.defaultMode`, `language`, `teammateMode`

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Arbitrary config modification | Whitelist of supported settings |
| Invalid values | Type validation, enum validation |

### Code Examples

```rust
// Get a setting
ConfigInput {
    setting: "model".to_string(),
    value: None,
}

// Set a setting
ConfigInput {
    setting: "permissions.defaultMode".to_string(),
    value: Some(ConfigValue::String("plan".to_string())),
}
```

---

## StructuredOutput

### Purpose

Return structured output in the requested format.

### Input Schema

```json
{
  "type": "object",
  "additionalProperties": true
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Usage:** Pass any JSON object as the desired output structure.

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Large output | No explicit limit |

### Code Examples

```rust
// Request specific structure
StructuredOutputInput(json!({
    "summary": "",
    "files_changed": [],
    "tests_passed": true
}))
```

---

## REPL

### Purpose

Execute code in interpreted languages (Python, JavaScript, Shell).

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "code": { "type": "string" },
    "language": { "type": "string" },
    "timeout_ms": { "type": "integer", "minimum": 1 }
  },
  "required": ["code", "language"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Supported Languages:**
- `python`/`py` → python3/python -c
- `javascript`/`js`/`node` → node -e
- `sh`/`shell`/`bash` → bash/sh -lc

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Code injection | DangerFullAccess permission required |
| Resource exhaustion | Timeout option |
| Arbitrary code | Sandboxed execution |

### Code Examples

```rust
// Python code
ReplInput {
    code: "print(2 + 2)".to_string(),
    language: "python".to_string(),
    timeout_ms: Some(5000),
}

// JavaScript code
ReplInput {
    code: "console.log('hello')".to_string(),
    language: "javascript".to_string(),
    ..Default::default()
}
```

---

## PowerShell

### Purpose

Execute PowerShell commands with optional timeout.

### Input Schema

```json
{
  "type": "object",
  "properties": {
    "command": { "type": "string" },
    "timeout": { "type": "integer", "minimum": 1 },
    "description": { "type": "string" },
    "run_in_background": { "type": "boolean" }
  },
  "required": ["command"]
}
```

### Implementation Details

**Source File:** `rust/crates/tools/src/lib.rs`

**Shell Detection:**
```rust
fn detect_powershell_shell() -> io::Result<&'static str> {
    if command_exists("pwsh") {
        Ok("pwsh")  // PowerShell Core
    } else if command_exists("powershell") {
        Ok("powershell")  // Windows PowerShell
    } else {
        Err("PowerShell executable not found")
    }
}
```

### Security Considerations

| Concern | Mitigation |
|---------|------------|
| Arbitrary command execution | DangerFullAccess permission |
| Windows-specific | Only available on Windows |

### Code Examples

```rust
// Basic PowerShell command
PowerShellInput {
    command: "Get-Process".to_string(),
    timeout: Some(5000),
    ..Default::default()
}
```

---

## Summary Table

| Tool | Permission | Sandbox | Primary Use |
|------|------------|---------|-------------|
| bash | DangerFullAccess | Yes | Shell commands |
| read_file | ReadOnly | N/A | Read files |
| write_file | WorkspaceWrite | N/A | Create files |
| edit_file | WorkspaceWrite | N/A | Modify files |
| glob_search | ReadOnly | N/A | Find files |
| grep_search | ReadOnly | N/A | Search contents |
| WebFetch | ReadOnly | N/A | Fetch URLs |
| WebSearch | ReadOnly | N/A | Web search |
| TodoWrite | WorkspaceWrite | N/A | Task management |
| Skill | ReadOnly | N/A | Load skills |
| Agent | DangerFullAccess | Sub-agent | Delegate tasks |
| ToolSearch | ReadOnly | N/A | Find tools |
| NotebookEdit | WorkspaceWrite | N/A | Edit notebooks |
| Sleep | ReadOnly | N/A | Wait |
| SendUserMessage | ReadOnly | N/A | User communication |
| Config | WorkspaceWrite | N/A | Settings |
| StructuredOutput | ReadOnly | N/A | JSON output |
| REPL | DangerFullAccess | Yes | Code execution |
| PowerShell | DangerFullAccess | Yes | PowerShell commands |

---

*Document generated from source analysis of claw-code repository.*
*Source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/*
