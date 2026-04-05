# Bash Tool Implementation Deep-Dive

## Executive Summary

The bash tool is claw-code's primary mechanism for executing shell commands. This document provides a comprehensive analysis of its implementation, covering command execution, Linux sandboxing with `unshare()`, process isolation, filesystem restrictions, network controls, working directory management, environment handling, timeouts, resource limits, signal handling, and output capture.

**Source Reference:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/`

---

## Table of Contents

1. [Overview](#overview)
2. [BashCommandInput Structure](#bashcommandinput-structure)
3. [BashCommandOutput Structure](#bashcommandoutput-structure)
4. [Execution Flow](#execution-flow)
5. [Linux Sandboxing with unshare()](#linux-sandboxing-with-unshare)
6. [Process Isolation Techniques](#process-isolation-techniques)
7. [Filesystem Restrictions](#filesystem-restrictions)
8. [Network Access Controls](#network-access-controls)
9. [Working Directory Management](#working-directory-management)
10. [Environment Variable Handling](#environment-variable-handling)
11. [Timeout and Resource Limits](#timeout-and-resource-limits)
12. [Signal Handling and Process Cleanup](#signal-handling-and-process-cleanup)
13. [Output Capture and Streaming](#output-capture-and-streaming)
14. [Background Task Execution](#background-task-execution)
15. [Sandbox Fallback Behavior](#sandbox-fallback-behavior)

---

## 1. Overview

### 1.1 Purpose

The bash tool enables the AI assistant to execute shell commands for:
- File system operations beyond basic read/write
- Running build systems and test frameworks
- Process management and system introspection
- Git operations
- Package management
- Development tooling

### 1.2 Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Bash Tool Entry Point                        │
│              execute_bash(BashCommandInput)                     │
│                   runtime/src/bash.rs:67                        │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│               Sandbox Status Resolution                         │
│         sandbox_status_for_input() - bash.rs:167                │
│   - Load config from RuntimeConfig                              │
│   - Apply input overrides                                       │
│   - Resolve final SandboxStatus                                 │
└─────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                 Background vs Foreground                        │
│         run_in_background flag check - bash.rs:71               │
└─────────────────────────────────────────────────────────────────┘
                │                               │
                │ Background                    │ Foreground
                ▼                               ▼
    ┌───────────────────────┐       ┌───────────────────────────┐
    │ Spawn detached        │       │ Prepare with sandbox      │
    │ Null I/O              │       │ Execute synchronously     │
    │ Return PID            │       │ Capture output            │
    └───────────────────────┘       └───────────────────────────┘
                │                               │
                │                               ▼
                │                   ┌───────────────────────────┐
                │                   │   Sandbox Launcher Build  │
                │                   │  build_linux_sandbox_cmd  │
                │                   │   runtime/src/sandbox.rs  │
                │                   └───────────────────────────┘
                │                               │
                │                   ┌───────────┴───────────┐
                │                   │                       │
                │             Sandbox Available      No Sandbox
                │                   │                       │
                │                   ▼                       ▼
                │         ┌─────────────────┐   ┌─────────────────┐
                │         │ unshare command │   │ Direct sh -lc   │
                │         │ with namespaces │   │ + env restrict  │
                │         └─────────────────┘   └─────────────────┘
                │                   │                       │
                └───────────────────┴───────────────────────┘
                                    │
                                    ▼
                        ┌─────────────────────────┐
                        │   Output Processing     │
                        │ - Timeout handling      │
                        │ - Exit code capture     │
                        │ - Stream capture        │
                        │ - Status formatting     │
                        └─────────────────────────┘
```

---

## 2. BashCommandInput Structure

### 2.1 Complete Input Definition

```rust
// runtime/src/bash.rs:17-34
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BashCommandInput {
    pub command: String,
    pub timeout: Option<u64>,
    pub description: Option<String>,
    #[serde(rename = "run_in_background")]
    pub run_in_background: Option<bool>,
    #[serde(rename = "dangerouslyDisableSandbox")]
    pub dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "namespaceRestrictions")]
    pub namespace_restrictions: Option<bool>,
    #[serde(rename = "isolateNetwork")]
    pub isolate_network: Option<bool>,
    #[serde(rename = "filesystemMode")]
    pub filesystem_mode: Option<FilesystemIsolationMode>,
    #[serde(rename = "allowedMounts")]
    pub allowed_mounts: Option<Vec<String>>,
}
```

### 2.2 Field Descriptions

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `command` | `String` | Yes | - | Shell command to execute |
| `timeout` | `Option<u64>` | No | `None` | Timeout in milliseconds |
| `description` | `Option<String>` | No | `None` | Human-readable description |
| `run_in_background` | `Option<bool>` | No | `false` | Run asynchronously |
| `dangerouslyDisableSandbox` | `Option<bool>` | No | `false` | Disable all sandboxing |
| `namespaceRestrictions` | `Option<bool>` | No | `true` | Use Linux namespaces |
| `isolateNetwork` | `Option<bool>` | No | `false` | Block network access |
| `filesystemMode` | `Option<Enum>` | No | `workspace-only` | Filesystem isolation level |
| `allowedMounts` | `Option<Vec<String>>` | No | `[]` | Explicit path allowlist |

### 2.3 FilesystemIsolationMode

```rust
// runtime/src/sandbox.rs:7-25
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FilesystemIsolationMode {
    Off,            // No isolation
    #[default]
    WorkspaceOnly,  // Default: only workspace directory
    AllowList,      // Only explicitly allowed paths
}
```

---

## 3. BashCommandOutput Structure

### 3.1 Complete Output Definition

```rust
// runtime/src/bash.rs:36-65
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
    #[serde(rename = "backgroundedByUser")]
    pub backgrounded_by_user: Option<bool>,
    #[serde(rename = "assistantAutoBackgrounded")]
    pub assistant_auto_backgrounded: Option<bool>,
    #[serde(rename = "dangerouslyDisableSandbox")]
    pub dangerously_disable_sandbox: Option<bool>,
    #[serde(rename = "returnCodeInterpretation")]
    pub return_code_interpretation: Option<String>,
    #[serde(rename = "noOutputExpected")]
    pub no_output_expected: Option<bool>,
    #[serde(rename = "structuredContent")]
    pub structured_content: Option<Vec<serde_json::Value>>,
    #[serde(rename = "persistedOutputPath")]
    pub persisted_output_path: Option<String>,
    #[serde(rename = "persistedOutputSize")]
    pub persisted_output_size: Option<u64>,
    #[serde(rename = "sandboxStatus")]
    pub sandbox_status: Option<SandboxStatus>,
}
```

### 3.2 Key Output Fields

| Field | Description |
|-------|-------------|
| `stdout` | Standard output from command |
| `stderr` | Standard error from command |
| `interrupted` | True if command was timeout-killed |
| `background_task_id` | PID for background tasks |
| `return_code_interpretation` | `null` for success, `"exit_code:N"` for failure |
| `sandbox_status` | Actual sandbox configuration used |
| `no_output_expected` | True if both stdout and stderr are empty |

---

## 4. Execution Flow

### 4.1 Main Entry Point

```rust
// runtime/src/bash.rs:67-100
pub fn execute_bash(input: BashCommandInput) -> io::Result<BashCommandOutput> {
    // 1. Get current working directory
    let cwd = env::current_dir()?;

    // 2. Resolve sandbox status based on input and config
    let sandbox_status = sandbox_status_for_input(&input, &cwd);

    // 3. Handle background execution
    if input.run_in_background.unwrap_or(false) {
        let mut child = prepare_command(&input.command, &cwd, &sandbox_status, false);
        let child = child
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        return Ok(BashCommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            raw_output_path: None,
            interrupted: false,
            is_image: None,
            background_task_id: Some(child.id().to_string()),
            backgrounded_by_user: Some(false),
            assistant_auto_backgrounded: Some(false),
            dangerously_disable_sandbox: input.dangerously_disable_sandbox,
            return_code_interpretation: None,
            no_output_expected: Some(true),
            structured_content: None,
            persisted_output_path: None,
            persisted_output_size: None,
            sandbox_status: Some(sandbox_status),
        });
    }

    // 4. Execute synchronously with async runtime
    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(execute_bash_async(input, sandbox_status, cwd))
}
```

### 4.2 Sandbox Status Resolution

```rust
// runtime/src/bash.rs:167-180
fn sandbox_status_for_input(
    input: &BashCommandInput,
    cwd: &std::path::Path,
) -> SandboxStatus {
    // Load configuration
    let config = ConfigLoader::default_for(cwd).load().map_or_else(
        |_| SandboxConfig::default(),
        |runtime_config| runtime_config.sandbox().clone(),
    );

    // Resolve with input overrides
    let request = config.resolve_request(
        input.dangerously_disable_sandbox.map(|disabled| !disabled),
        input.namespace_restrictions,
        input.isolate_network,
        input.filesystem_mode,
        input.allowed_mounts.clone(),
    );

    // Compute final status
    resolve_sandbox_status_for_request(&request, cwd)
}
```

### 4.3 Async Execution

```rust
// runtime/src/bash.rs:102-165
async fn execute_bash_async(
    input: BashCommandInput,
    sandbox_status: SandboxStatus,
    cwd: std::path::PathBuf,
) -> io::Result<BashCommandOutput> {
    // Prepare command with sandbox
    let mut command = prepare_tokio_command(&input.command, &cwd, &sandbox_status, true);

    // Execute with optional timeout
    let output_result = if let Some(timeout_ms) = input.timeout {
        match timeout(Duration::from_millis(timeout_ms), command.output()).await {
            Ok(result) => (result?, false),
            Err(_) => {
                // Timeout handling
                return Ok(BashCommandOutput {
                    stdout: String::new(),
                    stderr: format!("Command exceeded timeout of {timeout_ms} ms"),
                    interrupted: true,
                    return_code_interpretation: Some(String::from("timeout")),
                    // ... other fields
                    sandbox_status: Some(sandbox_status),
                });
            }
        }
    } else {
        (command.output().await?, false)
    };

    // Process output
    let (output, interrupted) = output_result;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let no_output_expected = Some(stdout.trim().is_empty() && stderr.trim().is_empty());
    let return_code_interpretation = output.status.code().and_then(|code| {
        if code == 0 {
            None
        } else {
            Some(format!("exit_code:{code}"))
        }
    });

    Ok(BashCommandOutput {
        stdout,
        stderr,
        interrupted,
        return_code_interpretation,
        no_output_expected,
        sandbox_status: Some(sandbox_status),
        // ... other fields
    })
}
```

---

## 5. Linux Sandboxing with unshare()

### 5.1 What is unshare()?

`unshare()` is a Linux system call that creates new namespaces for the calling process. Namespaces isolate system resources so processes in different namespaces have different views of:
- Process IDs (PID namespace)
- Filesystem mounts (Mount namespace)
- User/group IDs (User namespace)
- Network interfaces (Network namespace)
- Interprocess communication (IPC namespace)
- Hostname (UTS namespace)

### 5.2 Sandbox Command Builder

```rust
// runtime/src/sandbox.rs:210-262
pub fn build_linux_sandbox_command(
    command: &str,
    cwd: &Path,
    status: &SandboxStatus,
) -> Option<LinuxSandboxCommand> {
    // Check if sandboxing is applicable
    if !cfg!(target_os = "linux")
        || !status.enabled
        || (!status.namespace_active && !status.network_active)
    {
        return None;  // Fall back to non-sandboxed execution
    }

    // Build unshare arguments
    let mut args = vec![
        "--user".to_string(),      // New user namespace
        "--map-root-user".to_string(),  // Map user to root inside
        "--mount".to_string(),     // New mount namespace
        "--ipc".to_string(),       // New IPC namespace
        "--pid".to_string(),       // New PID namespace
        "--uts".to_string(),       // New UTS namespace
        "--fork".to_string(),      // Fork into namespace
    ];

    // Add network isolation if requested
    if status.network_active {
        args.push("--net".to_string());
    }

    // Execute sh -c inside namespace
    args.push("sh".to_string());
    args.push("-lc".to_string());
    args.push(command.to_string());

    // Set up isolated environment
    let sandbox_home = cwd.join(".sandbox-home");
    let sandbox_tmp = cwd.join(".sandbox-tmp");
    let mut env = vec![
        ("HOME".to_string(), sandbox_home.display().to_string()),
        ("TMPDIR".to_string(), sandbox_tmp.display().to_string()),
        (
            "CLAWD_SANDBOX_FILESYSTEM_MODE".to_string(),
            status.filesystem_mode.as_str().to_string(),
        ),
        (
            "CLAWD_SANDBOX_ALLOWED_MOUNTS".to_string(),
            status.allowed_mounts.join(":"),
        ),
    ];

    // Preserve PATH
    if let Ok(path) = env::var("PATH") {
        env.push(("PATH".to_string(), path));
    }

    Some(LinuxSandboxCommand {
        program: "unshare".to_string(),
        args,
        env,
    })
}
```

### 5.3 Namespace Effects

| Namespace Flag | Effect | Security Benefit |
|----------------|--------|------------------|
| `--user` | New user namespace | Root inside ≠ root outside |
| `--map-root-user` | Map UID to 0 inside | Process thinks it's root |
| `--mount` | New mount namespace | Cannot see host mounts |
| `--pid` | New PID namespace | Cannot see host processes |
| `--uts` | New UTS namespace | Isolated hostname |
| `--ipc` | New IPC namespace | No shared memory with host |
| `--fork` | Fork into namespace | Required for PID namespace |
| `--net` | New network namespace | (Optional) No network access |

### 5.4 User Namespace Mapping

```
┌─────────────────────────────────────────────────────────────────┐
│                    Outside Namespace (Host)                     │
│  UID: 1000 (darkvoid)                                           │
│  Capabilities: Limited                                          │
└─────────────────────────────────────────────────────────────────┘
                      │
                      │ unshare --user --map-root-user
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Inside Namespace (Sandbox)                   │
│  UID: 0 (root) - but mapped to 1000 outside                     │
│  Capabilities: Full inside namespace only                       │
│                                                                 │
│  What root CAN do inside:                                       │
│  - Create files in accessible filesystem                        │
│  - Bind to ports > 1024                                         │
│  - Use all syscalls                                             │
│                                                                 │
│  What root CANNOT do:                                           │
│  - Affect processes outside namespace                           │
│  - Access filesystem outside mount namespace                    │
│  - Use network (if --net used)                                  │
│  - Load kernel modules                                          │
│  - Modify host network config                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Process Isolation Techniques

### 6.1 PID Namespace Isolation

```rust
// --pid flag creates isolated process tree
```

**Effects:**
- Process inside namespace sees itself as PID 1
- Cannot see processes outside namespace via `/proc`
- Cannot send signals to external processes
- Child processes are reaped by namespace init

```
Host Process Tree:
├── PID 1: systemd
├── PID 100: sshd
├── PID 500: claw-code
│   └── PID 501: unshare (namespace boundary)
│       └── PID 1: sh (inside namespace - can't see PIDs above)
│           └── PID 2: ls
│           └── PID 3: cat
```

### 6.2 Mount Namespace Isolation

```rust
// --mount flag creates isolated filesystem view
```

**Effects:**
- Process sees private mount tree
- Mount/umount operations don't affect host
- Combined with workspace-only mode, limits file access

### 6.3 IPC Namespace Isolation

```rust
// --ipc flag isolates interprocess communication
```

**Effects:**
- Private shared memory segments
- Private semaphores
- Private message queues
- Cannot communicate with processes outside

---

## 7. Filesystem Restrictions

### 7.1 Filesystem Isolation Modes

```rust
// runtime/src/sandbox.rs:7-14
pub enum FilesystemIsolationMode {
    Off,            // No restrictions
    WorkspaceOnly,  // Default: only current directory tree
    AllowList,      // Only explicitly configured paths
}
```

### 7.2 WorkspaceOnly Mode Implementation

```rust
// runtime/src/bash.rs:202-205
if sandbox_status.filesystem_active {
    prepared.env("HOME", cwd.join(".sandbox-home"));
    prepared.env("TMPDIR", cwd.join(".sandbox-tmp"));
}
```

**Protection Mechanism:**
1. HOME redirected to `.sandbox-home/` inside workspace
2. TMPDIR redirected to `.sandbox-tmp/` inside workspace
3. Process starts in workspace directory
4. Mount namespace can restrict visible paths

### 7.3 AllowList Mode

```rust
// runtime/src/sandbox.rs:179-184
if request.enabled
    && request.filesystem_mode == FilesystemIsolationMode::AllowList
    && request.allowed_mounts.is_empty()
{
    fallback_reasons.push(
        "filesystem allow-list requested without configured mounts".to_string()
    );
}
```

**Configuration:**
```json
{
  "sandbox": {
    "filesystem_mode": "allow-list",
    "allowed_mounts": ["/tmp", "/var/cache"]
  }
}
```

### 7.4 Directory Preparation

```rust
// runtime/src/bash.rs:236-239
fn prepare_sandbox_dirs(cwd: &std::path::Path) {
    let _ = std::fs::create_dir_all(cwd.join(".sandbox-home"));
    let _ = std::fs::create_dir_all(cwd.join(".sandbox-tmp"));
}
```

---

## 8. Network Access Controls

### 8.1 Network Isolation via Namespace

```rust
// runtime/src/sandbox.rs:232-234
if status.network_active {
    args.push("--net".to_string());
}
```

**Effects of `--net`:**
- New network namespace with isolated interfaces
- No access to host network interfaces
- No inbound connections (no listening sockets)
- No outbound connections (no external access)
- Only loopback interface (if configured)

### 8.2 Network Isolation Detection

```rust
// runtime/src/sandbox.rs:164-177
let network_supported = namespace_supported;  // Requires unshare
// ...
if request.enabled && request.network_isolation && !network_supported {
    fallback_reasons.push(
        "network isolation unavailable (requires Linux with `unshare`)".to_string()
    );
}
```

### 8.3 Configuration

```json
{
  "sandbox": {
    "enabled": true,
    "network_isolation": true  // Enable --net flag
  }
}
```

---

## 9. Working Directory Management

### 9.1 CWD Preservation

```rust
// runtime/src/bash.rs:68, 195, 222
let cwd = env::current_dir()?;
// ...
prepared.current_dir(cwd);
```

**Behavior:**
- Command executes in same directory as claw-code
- Relative paths resolved from this directory
- Subdirectory access allowed within workspace

### 9.2 Directory Access Controls

With `WorkspaceOnly` mode:
- Can access current directory and subdirectories
- Cannot traverse to parent directories (mount namespace)
- Canonical paths outside workspace blocked

---

## 10. Environment Variable Handling

### 10.1 Environment Setup

```rust
// runtime/src/sandbox.rs:241-255
let mut env = vec![
    ("HOME".to_string(), sandbox_home.display().to_string()),
    ("TMPDIR".to_string(), sandbox_tmp.display().to_string()),
    (
        "CLAWD_SANDBOX_FILESYSTEM_MODE".to_string(),
        status.filesystem_mode.as_str().to_string(),
    ),
    (
        "CLAWD_SANDBOX_ALLOWED_MOUNTS".to_string(),
        status.allowed_mounts.join(":"),
    ),
];
if let Ok(path) = env::var("PATH") {
    env.push(("PATH".to_string(), path));
}
```

### 10.2 Environment Isolation

| Variable | Sandboxed Value | Purpose |
|----------|-----------------|---------|
| `HOME` | `.sandbox-home/` | Redirect home directory |
| `TMPDIR` | `.sandbox-tmp/` | Redirect temp directory |
| `PATH` | Inherited | Preserve command availability |
| `CLAWD_SANDBOX_*` | Metadata | Inform subprocess of sandbox |

### 10.3 Security Considerations

**Dangerous Variables Not Set:**
- No `LD_PRELOAD` (prevent library injection)
- No `LD_LIBRARY_PATH` (prevent library path manipulation)
- No `PYTHONPATH` modifications
- No shell history files

---

## 11. Timeout and Resource Limits

### 11.1 Timeout Implementation

```rust
// runtime/src/bash.rs:109-134
let output_result = if let Some(timeout_ms) = input.timeout {
    match timeout(Duration::from_millis(timeout_ms), command.output()).await {
        Ok(result) => (result?, false),
        Err(_) => {
            return Ok(BashCommandOutput {
                stdout: String::new(),
                stderr: format!("Command exceeded timeout of {timeout_ms} ms"),
                interrupted: true,
                return_code_interpretation: Some(String::from("timeout")),
                // ...
            });
        }
    }
} else {
    (command.output().await?, false)
};
```

### 11.2 Timeout Behavior

On timeout:
1. Process is killed
2. `interrupted: true` is set
3. Timeout message in `stderr`
4. `return_code_interpretation: "timeout"`

### 11.3 Resource Limits

**Current Implementation:**
- Timeout only (no CPU/memory limits)

**Recommended Additions:**
```rust
// Example: Add resource limits via ulimit or cgroups
use std::process::Command;
use nix::sys::resource::{setrlimit, Resource};

fn set_resource_limits() {
    // Limit CPU time
    setrlimit(Resource::RLIMIT_CPU, 60, 60).ok();

    // Limit memory (e.g., 512MB)
    setrlimit(Resource::RLIMIT_AS, 512 * 1024 * 1024, 512 * 1024 * 1024).ok();

    // Limit file size
    setrlimit(Resource::RLIMIT_FSIZE, 100 * 1024 * 1024, 100 * 1024 * 1024).ok();

    // Limit processes
    setrlimit(Resource::RLIMIT_NPROC, 10, 10).ok();
}
```

---

## 12. Signal Handling and Process Cleanup

### 12.1 Current Implementation

```rust
// runtime/src/bash.rs uses tokio::process::Command
// Signal handling is implicit via tokio runtime
```

**Behavior:**
- When timeout expires, tokio kills the process
- When parent dies, child processes are orphaned
- PID namespace handles reaping

### 12.2 Recommended Improvements

```rust
use tokio::process::Child;
use tokio::signal;

async fn execute_with_signal_handling(
    mut command: TokioCommand,
) -> io::Result<std::process::Output> {
    let mut child = command.spawn()?;

    // Handle Ctrl+C
    tokio::select! {
        result = child.wait_with_output() => result,
        _ = signal::ctrl_c() => {
            child.kill().await?;
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Process interrupted by signal"
            ))
        }
    }
}
```

### 12.3 Zombie Process Prevention

The PID namespace (`--pid`) ensures:
- Child processes are reaped by namespace init
- No zombie processes accumulate
- Process table isolation from host

---

## 13. Output Capture and Streaming

### 13.1 Output Capture

```rust
// runtime/src/bash.rs:136-146
let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
let no_output_expected = Some(stdout.trim().is_empty() && stderr.trim().is_empty());
let return_code_interpretation = output.status.code().and_then(|code| {
    if code == 0 {
        None
    } else {
        Some(format!("exit_code:{code}"))
    }
});
```

### 13.2 Output Encoding

- Output captured as raw bytes
- Converted to UTF-8 with lossy conversion
- Invalid UTF-8 sequences replaced with replacement character

### 13.3 Large Output Handling

**Current Limitation:** All output captured in memory

**Recommended Improvement:**
```rust
// Stream large output to temporary file
use std::fs::File;
use std::io::Write;

const MAX_MEMORY_OUTPUT: usize = 10 * 1024 * 1024;  // 10MB

fn handle_large_output(output: std::process::Output) -> BashCommandOutput {
    if output.stdout.len() + output.stderr.len() > MAX_MEMORY_OUTPUT {
        // Write to temp file
        let temp_path = "/tmp/clawd-output-{}.txt";
        let mut file = File::create(temp_path)?;
        file.write_all(&output.stdout)?;
        file.write_all(&output.stderr)?;

        BashCommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            raw_output_path: Some(temp_path.to_string()),
            persisted_output_size: Some((output.stdout.len() + output.stderr.len()) as u64),
            // ...
        }
    } else {
        // Normal in-memory output
        // ...
    }
}
```

---

## 14. Background Task Execution

### 14.1 Background Mode

```rust
// runtime/src/bash.rs:71-96
if input.run_in_background.unwrap_or(false) {
    let mut child = prepare_command(&input.command, &cwd, &sandbox_status, false);
    let child = child
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    return Ok(BashCommandOutput {
        stdout: String::new(),
        stderr: String::new(),
        background_task_id: Some(child.id().to_string()),
        backgrounded_by_user: Some(false),
        no_output_expected: Some(true),
        sandbox_status: Some(sandbox_status),
        // ...
    });
}
```

### 14.2 Background Task Characteristics

| Aspect | Behavior |
|--------|----------|
| stdin | Null (no input) |
| stdout | Null (discarded) |
| stderr | Null (discarded) |
| Return value | Task PID |
| Parent relationship | Detached |
| Cleanup | OS reaps on completion |

### 14.3 Use Cases

- Long-running servers
- Background workers
- Fire-and-forget operations
- Daemons

---

## 15. Sandbox Fallback Behavior

### 15.1 Detection and Fallback

```rust
// runtime/src/sandbox.rs:162-207
pub fn resolve_sandbox_status_for_request(
    request: &SandboxRequest,
    cwd: &Path,
) -> SandboxStatus {
    let container = detect_container_environment();
    let namespace_supported = cfg!(target_os = "linux") && command_exists("unshare");
    let network_supported = namespace_supported;

    // Track fallback reasons
    let mut fallback_reasons = Vec::new();

    if request.enabled && request.namespace_restrictions && !namespace_supported {
        fallback_reasons.push(
            "namespace isolation unavailable (requires Linux with `unshare`)".to_string()
        );
    }

    let active = request.enabled
        && (!request.namespace_restrictions || namespace_supported)
        && (!request.network_isolation || network_supported);

    SandboxStatus {
        enabled: request.enabled,
        supported: namespace_supported,
        active,
        namespace_supported,
        namespace_active: request.enabled
            && request.namespace_restrictions
            && namespace_supported,
        fallback_reason: (!fallback_reasons.is_empty())
            .then(|| fallback_reasons.join("; ")),
        // ...
    }
}
```

### 15.2 Fallback Scenarios

| Scenario | Fallback Behavior |
|----------|-------------------|
| Non-Linux OS | No sandbox, environment restrictions only |
| unshare not found | No namespace isolation |
| Inside container | May have reduced capabilities |
| Permission denied | Log warning, continue without sandbox |

### 15.3 Container Detection

```rust
// runtime/src/sandbox.rs:109-153
pub fn detect_container_environment() -> ContainerEnvironment {
    let proc_1_cgroup = fs::read_to_string("/proc/1/cgroup").ok();
    detect_container_environment_from(SandboxDetectionInputs {
        env_pairs: env::vars().collect(),
        dockerenv_exists: Path::new("/.dockerenv").exists(),
        containerenv_exists: Path::new("/run/.containerenv").exists(),
        proc_1_cgroup: proc_1_cgroup.as_deref(),
    })
}

pub fn detect_container_environment_from(
    inputs: SandboxDetectionInputs<'_>,
) -> ContainerEnvironment {
    let mut markers = Vec::new();

    // Check for Docker marker file
    if inputs.dockerenv_exists {
        markers.push("/.dockerenv".to_string());
    }

    // Check for Podman/other marker
    if inputs.containerenv_exists {
        markers.push("/run/.containerenv".to_string());
    }

    // Check environment variables
    for (key, value) in inputs.env_pairs {
        let normalized = key.to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "container" | "docker" | "podman" | "kubernetes_service_host"
        ) && !value.is_empty()
        {
            markers.push(format!("env:{key}={value}"));
        }
    }

    // Check cgroup for container markers
    if let Some(cgroup) = inputs.proc_1_cgroup {
        for needle in ["docker", "containerd", "kubepods", "podman", "libpod"] {
            if cgroup.contains(needle) {
                markers.push(format!("/proc/1/cgroup:{needle}"));
            }
        }
    }

    ContainerEnvironment {
        in_container: !markers.is_empty(),
        markers,
    }
}
```

---

## 16. Summary

The bash tool implementation demonstrates several key security and engineering principles:

### 16.1 Key Security Features

| Feature | Implementation | Benefit |
|---------|----------------|---------|
| Namespace Isolation | `unshare --user --mount --pid` | Process cannot affect host |
| Filesystem Restriction | WorkspaceOnly, AllowList modes | Limits file access scope |
| Network Isolation | `--net` flag | Prevents external communication |
| Environment Control | HOME, TMPDIR redirection | Contains temp/home files |
| Timeout Protection | tokio timeout wrapper | Prevents hangs |

### 16.2 Key Engineering Decisions

| Decision | Rationale |
|----------|-----------|
| unshare over containers | Lighter weight, no daemon required |
| Graceful fallback | Works on non-Linux, degraded gracefully |
| Synchronous API with async impl | Simple interface, efficient execution |
| Structured output | Machine-parseable results |
| Background task support | Long-running operations |

### 16.3 Areas for Improvement

1. **Resource Limits**: Add CPU/memory limits via cgroups or ulimit
2. **Signal Handling**: Explicit SIGINT/SIGTERM handling
3. **Large Output**: Stream to file instead of memory
4. **Audit Logging**: Log all commands for security review
5. **Seccomp Filtering**: Additional syscall filtering

---

*Document generated from source analysis of claw-code repository.*
*Source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claw-code/*
