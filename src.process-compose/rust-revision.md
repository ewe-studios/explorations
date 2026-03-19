---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.process-compose/process-compose
repository: https://github.com/f1bonacc1/process-compose
revised_at: 2026-03-19T00:00:00Z
workspace: process-compose-rs
---

# Rust Revision: process-compose

## Overview

This document details the complete redesign of process-compose in Rust. Process-compose is a process orchestrator similar to docker-compose but for non-containerized processes. The Rust implementation will be called `process-compose-rs` and will provide:

- Process dependency management with topological sorting
- Multiple restart policies (always, on_failure, exit_on_failure, no)
- Health checks (exec and HTTP probes)
- TUI using ratatui
- REST API with OpenAPI documentation
- Process scaling and replication
- Hot-reload project updates
- PTY support for interactive processes

The implementation uses a multi-crate workspace with tokio for async runtime, emphasizing type safety, proper error handling, and idiomatic Rust patterns.

## Workspace Structure

```
process-compose-rs/
├── Cargo.toml                      # Workspace root
├── Cargo.lock
├── rust-revision.md
├── crates/
│   ├── process-compose-core/       # Core orchestration engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── project.rs
│   │       ├── process.rs
│   │       ├── config.rs
│   │       ├── state.rs
│   │       └── error.rs
│   ├── process-compose-exec/       # Process execution with PTY
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── executor.rs
│   │       ├── pty.rs
│   │       └── signal.rs
│   ├── process-compose-health/     # Health check framework
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── probe.rs
│   │       ├── exec_checker.rs
│   │       └── http_checker.rs
│   ├── process-compose-api/        # REST API server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs
│   │       ├── routes.rs
│   │       ├── handlers.rs
│   │       └── types.rs
│   ├── process-compose-tui/        # Terminal UI
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── app.rs
│   │       ├── ui.rs
│   │       ├── components/
│   │       │   ├── mod.rs
│   │       │   ├── process_table.rs
│   │       │   ├── log_view.rs
│   │       │   └── status_bar.rs
│   │       └── events.rs
│   ├── process-compose-loader/     # Configuration loading
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── loader.rs
│   │       ├── merger.rs
│   │       ├── validator.rs
│   │       └── templater.rs
│   └── process-compose/            # Main binary CLI
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── cli.rs
│           └── commands/
│               ├── mod.rs
│               ├── up.rs
│               ├── down.rs
│               ├── list.rs
│               ├── logs.rs
│               └── scale.rs
└── tests/
    └── integration/
        ├── Cargo.toml
        └── src/
            └── main.rs
```

### Crate Breakdown

#### process-compose-core

- **Purpose:** Core orchestration engine, project and process state management
- **Type:** library
- **Public API:** `Project`, `ProjectRunner`, `Process`, `ProcessConfig`, `ProcessState`, `RestartPolicy`, dependency resolution
- **Dependencies:** tokio, serde, serde_yaml, thiserror, tracing, uuid

#### process-compose-exec

- **Purpose:** Process execution abstraction with PTY support
- **Type:** library
- **Public API:** `CommandExecutor`, `PtyExecutor`, `ProcessHandle`, `Signal`
- **Dependencies:** tokio, portable-pty, nix (Unix), windows (Windows), thiserror

#### process-compose-health

- **Purpose:** Health check framework for liveness and readiness probes
- **Type:** library
- **Public API:** `Probe`, `ProbeResult`, `ExecProbe`, `HttpProbe`, `HealthChecker`
- **Dependencies:** tokio, reqwest, thiserror, tracing

#### process-compose-api

- **Purpose:** REST API server with WebSocket support for log streaming
- **Type:** library
- **Public API:** `ApiServer`, `ApiConfig`, route handlers
- **Dependencies:** axum, tokio, tower, tower-http, serde, serde_json, utoipa, utoipa-swagger-ui, tokio-tungstenite

#### process-compose-tui

- **Purpose:** Terminal user interface using ratatui
- **Type:** library
- **Public API:** `TuiApp`, `TuiConfig`, UI components
- **Dependencies:** ratatui, crossterm, tokio, tracing

#### process-compose-loader

- **Purpose:** YAML configuration loading, merging, validation, and templating
- **Type:** library
- **Public API:** `Loader`, `LoaderOptions`, `ProjectMerger`, `Validator`, `Templater`
- **Dependencies:** serde, serde_yaml, tera (templating), thiserror, tracing

#### process-compose

- **Purpose:** Main CLI binary using clap
- **Type:** binary
- **Public API:** N/A (binary crate)
- **Dependencies:** clap, tokio, tracing-subscriber, all workspace crates

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Async runtime | tokio | 1.0 | Full-featured async runtime with process management |
| CLI parsing | clap | 4.0 | Industry standard CLI parser with derive macros |
| Serialization | serde + serde_yaml | 1.0 | Standard serialization framework |
| JSON handling | serde_json | 1.0 | API JSON serialization |
| HTTP server | axum | 0.7 | Ergonomic, type-safe web framework |
| HTTP client | reqwest | 0.12 | Async HTTP client for health checks |
| TUI framework | ratatui | 0.29 | Modern tui-rs fork, actively maintained |
| Terminal backend | crossterm | 0.28 | Cross-platform terminal manipulation |
| PTY handling | portable-pty | 0.8 | Cross-platform PTY abstraction |
| Unix APIs | nix | 0.29 | Unix-specific system calls |
| Error handling | thiserror | 2.0 | Derive macro for error types |
| Logging | tracing | 0.1 | Async-aware logging/tracing |
| Logging subscriber | tracing-subscriber | 0.3 | Configurable tracing subscriber |
| OpenAPI docs | utoipa + utoipa-swagger-ui | 5.0 | Auto-generated OpenAPI docs |
| WebSocket | tokio-tungstenite | 0.24 | Async WebSocket client/server |
| UUIDs | uuid | 1.0 | Unique process identifiers |
| Time | chrono | 0.4 | Time handling and formatting |
| Process info | sysinfo | 0.32 | Process CPU/memory monitoring |
| Tower middleware | tower + tower-http | 0.5 | HTTP middleware for axum |
| dotenv | dotenvy | 0.15 | .env file loading |
| Template engine | tera | 1.0 | Jinja2-like templating for configs |

## Type System Design

### Core Types

```rust
// ============ PROJECT CONFIGURATION ============

/// Main project configuration loaded from YAML
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub version: Option<String>,
    pub log_location: Option<String>,
    pub log_level: Option<LogLevel>,
    pub log_length: usize,
    pub logger_config: Option<LoggerConfig>,
    pub log_format: Option<LogFormat>,
    pub processes: Processes,
    pub environment: Vec<String>,
    pub shell_config: Option<ShellConfig>,
    pub is_strict: bool,
    pub vars: Vars,
    pub disable_env_expansion: bool,
    pub is_tui_disabled: bool,
    pub extends_project: Option<String>,
    pub env_commands: EnvCommands,
    #[serde(skip)]
    pub file_names: Vec<PathBuf>,
    #[serde(skip)]
    pub env_file_names: Vec<PathBuf>,
}

/// Map of process name to process configuration
pub type Processes = HashMap<String, ProcessConfig>;
pub type Vars = HashMap<String, serde_yaml::Value>;
pub type EnvCommands = HashMap<String, String>;

/// Process configuration from YAML
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessConfig {
    pub name: String,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub is_daemon: bool,
    pub command: Option<String>,
    #[serde(default)]
    pub entrypoint: Vec<String>,
    pub log_location: Option<String>,
    pub logger_config: Option<LoggerConfig>,
    #[serde(default)]
    pub environment: Vec<String>,
    #[serde(default, rename = "availability")]
    pub restart_policy: RestartPolicyConfig,
    #[serde(default)]
    pub depends_on: DependsOnConfig,
    pub liveness_probe: Option<Probe>,
    pub readiness_probe: Option<Probe>,
    pub ready_log_line: Option<String>,
    #[serde(default)]
    pub shutdown_params: ShutdownParams,
    #[serde(default)]
    pub disable_ansi_colors: bool,
    pub working_dir: Option<PathBuf>,
    pub namespace: Option<String>,
    #[serde(default)]
    pub replicas: u32,
    pub description: Option<String>,
    #[serde(default)]
    pub vars: Vars,
    #[serde(default)]
    pub is_foreground: bool,
    #[serde(default)]
    pub is_tty: bool,
    #[serde(default)]
    pub is_elevated: bool,
    pub launch_timeout: Option<u32>,

    // Runtime-computed fields (not from YAML)
    #[serde(skip)]
    pub replica_num: u32,
    #[serde(skip)]
    pub replica_name: String,
    #[serde(skip)]
    pub executable: Option<String>,
    #[serde(skip)]
    pub args: Vec<String>,
    #[serde(skip)]
    pub original_config: Option<String>,
}

/// Restart/availability policy
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RestartPolicyConfig {
    #[serde(default, rename = "restart")]
    pub restart: RestartPolicy,
    #[serde(default)]
    pub backoff_seconds: u32,
    #[serde(default)]
    pub max_restarts: Option<u32>,
    #[serde(default)]
    pub exit_on_end: bool,
    #[serde(default)]
    pub exit_on_skipped: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    Always,
    OnFailure,
    ExitOnFailure,
    #[default]
    No,
}

/// Process dependencies with conditions
pub type DependsOnConfig = HashMap<String, ProcessDependency>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessDependency {
    #[serde(default)]
    pub condition: ProcessCondition,
    #[serde(flatten)]
    pub extensions: HashMap<String, serde_yaml::Value>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessCondition {
    ProcessCompleted,
    ProcessCompletedSuccessfully,
    ProcessHealthy,
    ProcessLogReady,
    #[default]
    ProcessStarted,
}

// ============ PROCESS STATE ============

/// Runtime state of a process
#[derive(Debug, Clone)]
pub struct ProcessState {
    pub name: String,
    pub namespace: Option<String>,
    pub status: ProcessStatus,
    pub system_time: Duration,
    pub health: ProcessHealth,
    pub restarts: u32,
    pub exit_code: Option<i32>,
    pub pid: Option<u32>,
    pub is_elevated: bool,
    pub password_provided: bool,
    pub memory_bytes: u64,
    pub cpu_percent: f64,
    pub is_running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProcessStatus {
    Disabled,
    Foreground,
    Pending,
    Running,
    Launching,
    Launched,
    Restarting,
    Terminating,
    Completed,
    Skipped,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessHealth {
    Ready,
    NotReady,
    Unknown,
}

/// Project runtime state
#[derive(Debug, Clone)]
pub struct ProjectState {
    pub file_names: Vec<PathBuf>,
    pub start_time: SystemTime,
    pub up_time: Duration,
    pub process_num: usize,
    pub running_process_num: usize,
    pub user_name: String,
    pub host_name: String,
    pub version: &'static str,
    pub memory_state: Option<MemoryState>,
}

#[derive(Debug, Clone)]
pub struct MemoryState {
    pub allocated_mb: u64,
    pub total_allocated_mb: u64,
    pub system_memory_mb: u64,
    pub gc_cycles: u64, // N/A in Rust, kept for API compatibility
}

// ============ HEALTH PROBES ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Probe {
    pub exec: Option<ExecProbe>,
    #[serde(rename = "http_get")]
    pub http_get: Option<HttpProbe>,
    #[serde(default)]
    pub initial_delay_seconds: u32,
    #[serde(default)]
    pub period_seconds: u32,
    #[serde(default)]
    pub timeout_seconds: u32,
    #[serde(default)]
    pub success_threshold: u32,
    #[serde(default)]
    pub failure_threshold: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecProbe {
    pub command: String,
    pub working_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpProbe {
    #[serde(default = "default_localhost")]
    pub host: String,
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_http_scheme")]
    pub scheme: String,
    pub port: Option<String>,
    #[serde(skip)]
    pub num_port: Option<u16>,
}

fn default_localhost() -> String { "127.0.0.1".to_string() }
fn default_http_scheme() -> String { "http".to_string() }

#[derive(Debug, Clone)]
pub enum ProbeResult {
    Success,
    Failure(String),
    Timeout,
    Aborted,
}

// ============ SHELL CONFIG ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShellConfig {
    pub shell_command: String,
    pub shell_argument: String,
    pub elevated_shell_cmd: Option<String>,
    pub elevated_shell_arg: Option<String>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            shell_command: default_shell(),
            shell_argument: default_shell_arg(),
            elevated_shell_cmd: default_elevated_cmd(),
            elevated_shell_arg: default_elevated_arg(),
        }
    }
}

#[cfg(unix)]
fn default_shell() -> String { "bash".to_string() }
#[cfg(unix)]
fn default_shell_arg() -> String { "-c".to_string() }
#[cfg(unix)]
fn default_elevated_cmd() -> Option<String> { Some("sudo".to_string()) }
#[cfg(unix)]
fn default_elevated_arg() -> Option<String> { Some("-S".to_string()) }

#[cfg(windows)]
fn default_shell() -> String { "cmd".to_string() }
#[cfg(windows)]
fn default_shell_arg() -> String { "/C".to_string() }
#[cfg(windows)]
fn default_elevated_cmd() -> Option<String> { Some("runas".to_string()) }
#[cfg(windows)]
fn default_elevated_arg() -> Option<String> { Some("/user:Administrator".to_string()) }

// ============ LOGGING CONFIG ============

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoggerConfig {
    pub rotation: Option<LogRotation>,
    #[serde(default)]
    pub fields_order: Vec<String>,
    #[serde(default)]
    pub disable_json: bool,
    #[serde(default)]
    pub timestamp_format: String,
    #[serde(default)]
    pub no_metadata: bool,
    #[serde(default)]
    pub add_timestamp: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogRotation {
    pub max_size_mb: u64,
    pub max_age_days: u32,
    pub max_backups: u32,
    #[serde(default)]
    pub compress: bool,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    #[default]
    Off,
}

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    #[default]
    Text,
}
```

### Error Types

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessComposeError {
    #[error("Project loading failed: {0}")]
    ProjectLoad(#[from] ProjectLoadError),

    #[error("Process execution failed: {0}")]
    ProcessExecution(#[from] ExecutionError),

    #[error("Health check failed: {0}")]
    HealthCheck(#[from] HealthCheckError),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("Dependency error: {0}")]
    Dependency(String),

    #[error("Process not found: {0}")]
    ProcessNotFound(String),

    #[error("Process already running: {0}")]
    ProcessAlreadyRunning(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_yaml::Error),

    #[error("Template error: {0}")]
    Template(#[from] tera::Error),
}

pub type Result<T> = std::result::Result<T, ProcessComposeError>;

#[derive(Debug, Error)]
pub enum ProjectLoadError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Failed to read file: {0}")]
    ReadError(#[source] std::io::Error),

    #[error("Failed to parse YAML: {0}")]
    ParseError(#[source] serde_yaml::Error),

    #[error("Failed to extend project: {0}")]
    ExtendError(String),

    #[error("No config files found in {0}")]
    NoConfigFound(PathBuf),
}

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    #[error("Process not found: {0}")]
    NotFound(u32), // PID

    #[error("Process terminated with signal: {0}")]
    Signaled(i32),

    #[error("Process exited with code: {0}")]
    Exited(i32),

    #[error("PTY error: {0}")]
    Pty(#[from] portable_pty::Error),

    #[error("Timeout waiting for process")]
    Timeout,
}

#[derive(Debug, Error)]
pub enum HealthCheckError {
    #[error("Exec probe failed: {0}")]
    ExecFailed(String),

    #[error("HTTP probe failed: {0}")]
    HttpFailed(#[from] reqwest::Error),

    #[error("Probe timeout")]
    Timeout,

    #[error("Probe configuration error: {0}")]
    Configuration(String),
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Server failed to start: {0}")]
    ServerStart(#[source] std::io::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
}
```

### Traits

```rust
use std::future::Future;

/// Trait for project-level operations
pub trait ProjectInterface: Send + Sync {
    fn get_lexicographic_process_names(&self) -> Result<Vec<String>>;
    fn get_process_state(&self, name: &str) -> Result<ProcessState>;
    fn get_processes_state(&self) -> Result<Vec<ProcessState>>;
    fn get_process_info(&self, name: &str) -> Result<ProcessConfig>;
    fn get_process_ports(&self, name: &str) -> Result<ProcessPorts>;

    fn start_process(&self, name: &str) -> Result<()>;
    fn stop_process(&self, name: &str) -> Result<()>;
    fn stop_processes(&self, names: &[&str]) -> Result<HashMap<String, StopResult>>;
    fn restart_process(&self, name: &str) -> Result<()>;
    fn scale_process(&self, name: &str, scale: u32) -> Result<()>;

    fn get_process_log(
        &self,
        name: &str,
        offset_from_end: usize,
        limit: usize
    ) -> Result<Vec<String>>;

    fn get_process_log_length(&self, name: &str) -> usize;

    fn shut_down_project(&self) -> Result<()>;

    fn is_remote(&self) -> bool;
    fn error_for_secs(&self) -> u32;
    fn get_host_name(&self) -> Result<String>;

    fn reload_project(&self) -> Result<HashMap<String, ProcessUpdateStatus>>;
    fn update_project(
        &self,
        project: Project
    ) -> Result<HashMap<String, ProcessUpdateStatus>>;
}

/// Trait for process log streaming
pub trait LogObserver: Send {
    fn on_log(&self, message: LogMessage);
}

#[derive(Debug, Clone)]
pub struct LogMessage {
    pub message: String,
    pub process_name: String,
    pub replica_num: Option<u32>,
}

/// Trait for health probe execution
pub trait ProbeChecker: Send + Sync {
    fn check(&self) -> impl Future<Output = Result<ProbeResult>> + Send;
}

/// Trait for process execution
pub trait Executor: Send + Sync {
    fn spawn(&self, config: &ProcessConfig) -> Result<Box<dyn ProcessHandle>>;
    fn kill(&self, pid: u32, signal: i32) -> Result<()>;
}

/// Handle to a running process
pub trait ProcessHandle: Send {
    fn pid(&self) -> u32;
    fn wait(&self) -> impl Future<Output = Result<ExitStatus>> + Send;
    fn try_wait(&self) -> Result<Option<ExitStatus>>;
    fn kill(&self) -> Result<()>;
    fn kill_with_signal(&self, signal: i32) -> Result<()>;
    fn stdout(&self) -> Option<Box<dyn std::io::Read + Send>>;
    fn stderr(&self) -> Option<Box<dyn std::io::Read + Send>>;
    fn stdin(&self) -> Option<Box<dyn std::io::Write + Send>>;
}

#[derive(Debug, Clone)]
pub struct ExitStatus {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}
```

## Key Rust-Specific Changes

### 1. Ownership and State Management

**Source Pattern:** Go uses shared memory with mutex locks (`sync.Mutex`, `sync.RWMutex`) to protect state. Multiple goroutines access the same `Process` and `ProjectRunner` structs.

**Rust Translation:** Use `Arc<Mutex<T>>` for shared mutable state across async tasks, combined with message passing via `tokio::sync::mpsc` channels for inter-process communication.

**Rationale:** Rust's ownership system prevents data races at compile time. Using `Arc<Mutex<T>>` makes the sharing and locking explicit. Channels provide a cleaner abstraction for event-driven updates.

```rust
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

pub struct ProjectRunner {
    project: Arc<Project>,
    processes: Arc<Mutex<HashMap<String, ProcessHandle>>>,
    states: Arc<Mutex<HashMap<String, ProcessState>>>,
    logs: Arc<Mutex<HashMap<String, LogBuffer>>>,
    event_tx: mpsc::Sender<ProjectEvent>,
    event_rx: Arc<Mutex<mpsc::Receiver<ProjectEvent>>>,
}

pub enum ProjectEvent {
    ProcessStarted(String),
    ProcessExited(String, ExitStatus),
    ProcessRestarting(String),
    LogMessage(LogMessage),
    HealthCheck(String, ProbeResult),
}
```

### 2. Async Runtime for Concurrency

**Source Pattern:** Go uses goroutines (green threads) with channels. Each process runs in its own goroutine, and the `WaitGroup` coordinates shutdown.

**Rust Translation:** Use tokio for async runtime. Each process is spawned as an async task, with `JoinHandle` for coordination.

**Rationale:** Tokio provides a mature async runtime with excellent ecosystem support. Async tasks are lighter weight than OS threads and integrate well with I/O operations.

```rust
use tokio::task::JoinHandle;
use tokio::sync::mpsc;

impl ProjectRunner {
    pub async fn run(&self) -> Result<()> {
        let run_order = self.build_run_order()?;
        let mut handles = Vec::new();

        for proc_config in run_order {
            let handle = self.spawn_process_task(proc_config.clone());
            handles.push(handle);
        }

        // Wait for all processes
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    fn spawn_process_task(
        &self,
        config: ProcessConfig
    ) -> JoinHandle<Result<ExitStatus>> {
        let self_clone = self.clone();
        tokio::spawn(async move {
            self_clone.run_process(config).await
        })
    }
}
```

### 3. Error Handling with thiserror

**Source Pattern:** Go returns `(value, error)` tuples. Errors are often logged and ignored, or checked with `if err != nil`.

**Rust Translation:** Use `Result<T, E>` with `thiserror` for error type derivation. Use `?` operator for propagation.

**Rationale:** Rust's type system forces error handling. `thiserror` provides ergonomic error type creation with automatic `Display` and `Error` trait implementations.

```rust
// Instead of:
// err := proc.shutDownNoRestart()
// if err != nil {
//     log.Error().Err(err).Msgf("failed to stop process %s", name)
//     return err
// }

// Use:
proc.shut_down_no_restart().await?;  // Propagates with context
```

### 4. Dependency Resolution with petgraph

**Source Pattern:** Go code uses recursive DFS with a `done` map to track visited nodes.

**Rust Translation:** Use the `petgraph` crate for graph operations including cycle detection and topological sorting.

**Rationale:** `petgraph` is a well-tested graph library that handles edge cases like cycle detection efficiently.

```rust
use petgraph::graph::DiGraph;
use petgraph::algo::toposort;
use petgraph::algo::is_cyclic_directed;

pub fn resolve_dependencies(
    processes: &Processes
) -> Result<Vec<String>, DependencyError> {
    let mut graph = DiGraph::<&str, ()>::new();
    let mut name_to_node = HashMap::new();

    // Build graph
    for (name, config) in processes.iter() {
        let node = graph.add_node(name);
        name_to_node.insert(name, node);
    }

    for (name, config) in processes.iter() {
        let from = name_to_node[name];
        for dep_name in config.depends_on.keys() {
            if let Some(&to) = name_to_node.get(dep_name) {
                graph.add_edge(from, to, ());
            }
        }
    }

    // Check for cycles
    if let Some(cycle) = is_cyclic_directed(&graph) {
        return Err(DependencyError::CircularDependency(
            "Circular dependency detected".to_string()
        ));
    }

    // Topological sort
    let sorted = toposort(&graph, None)
        .map_err(|_| DependencyError::SortFailed)?;

    Ok(sorted
        .into_iter()
        .map(|node| graph[node].to_string())
        .collect())
}
```

### 5. Configuration with Serde Untagged Enums

**Source Pattern:** Go uses `yaml:",inline"` and `map[string]interface{}` for flexible YAML parsing.

**Rust Translation:** Use serde's untagged enums and flatten attributes for flexible parsing.

**Rationale:** Serde provides type-safe deserialization with excellent error messages. Untagged enums allow parsing multiple formats without explicit tags.

```rust
use serde::de::{self, Deserializer, Visitor};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessConfig {
    // ... other fields

    #[serde(flatten)]
    pub extensions: HashMap<String, serde_yaml::Value>,
}

impl ProcessConfig {
    pub fn validate(&self) -> Result<()> {
        for key in self.extensions.keys() {
            if !key.starts_with("x-") {
                return Err(ProcessComposeError::Validation(
                    format!("Unknown key '{}' in process '{}'", key, self.name)
                ));
            }
        }
        Ok(())
    }
}
```

### 6. Hot Reload with File Watching

**Source Pattern:** Manual polling or external triggers for configuration reload.

**Rust Translation:** Use `notify` crate for cross-platform file watching combined with debounced events.

**Rationale:** `notify` provides efficient, native file system notifications on all platforms.

```rust
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

pub struct HotReloader {
    watcher: RecommendedWatcher,
    reload_tx: mpsc::Sender<()>,
}

impl HotReloader {
    pub fn new<F>(files: &[PathBuf], reload_fn: F) -> Result<Self>
    where
        F: Fn() -> Result<()> + Send + 'static,
    {
        let (reload_tx, mut reload_rx) = mpsc::channel(1);

        let watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() || event.kind.is_create() {
                        let _ = reload_tx.try_send(());
                    }
                }
            },
            Config::default(),
        )?;

        for file in files {
            watcher.watch(file, RecursiveMode::NonRecursive)?;
        }

        Ok(Self { watcher, reload_tx })
    }

    pub async fn run(&self, reload_fn: impl Fn() -> Result<()> + Send) {
        // Debounced reload logic
    }
}
```

## Ownership & Borrowing Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                        ProjectRunner                            │
│  Arc<Project> (shared, immutable config)                        │
│  Arc<Mutex<HashMap<String, ProcessHandle>>> (running procs)     │
│  Arc<Mutex<HashMap<String, ProcessState>>> (mutable state)      │
│  mpsc::Sender<ProjectEvent> (event emission)                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Arc clones (cheap, atomic)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Process Task (async)                       │
│  Arc<Project> (read config)                                     │
│  mpsc::Sender<LogMessage> (log streaming)                       │
│  HealthChecker (owned, runs probes)                             │
│  CommandExecutor (owned, spawns process)                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ Channels for communication
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         TUI / API                               │
│  mpsc::Receiver<ProjectEvent> (receives updates)                │
│  ProjectInterface trait object (operations)                     │
└─────────────────────────────────────────────────────────────────┘
```

Key principles:

1. **Configuration is immutable after load** - `Arc<Project>` is shared read-only
2. **State is behind Mutex** - `Arc<Mutex<HashMap>>` for process states
3. **Events flow one way** - `mpsc::channel` from processes to UI/API
4. **Handles are boxed traits** - `Box<dyn ProcessHandle>` for flexibility
5. **Logs are ring buffers** - Owned by `ProjectRunner`, accessed via references

## Concurrency Model

**Approach:** Async (tokio) with message passing

**Rationale:**
- Process I/O (logs, health checks) is naturally async
- Tokio provides excellent ecosystem support (axum, reqwest, etc.)
- Message passing avoids lock contention
- PTY operations need blocking threads (use `spawn_blocking`)

```rust
// Concurrency pattern for process execution
use tokio::task::JoinSet;
use tokio::sync::mpsc;

pub struct ProjectRunner {
    event_tx: mpsc::Sender<ProjectEvent>,
}

impl ProjectRunner {
    pub async fn run_all(&self, configs: Vec<ProcessConfig>) -> Result<()> {
        let mut set = JoinSet::new();

        for config in configs {
            let tx = self.event_tx.clone();
            set.spawn(async move {
                // Each process runs in its own task
                run_process(config, tx).await
            });
        }

        // Collect results
        while let Some(result) = set.join_next().await {
            match result {
                Ok(Ok(exit_status)) => { /* handled */ }
                Ok(Err(e)) => tracing::error!(?e),
                Err(e) => tracing::error!(?e, "task panicked"),
            }
        }

        Ok(())
    }
}

async fn run_process(
    config: ProcessConfig,
    event_tx: mpsc::Sender<ProjectEvent>,
) -> Result<ExitStatus> {
    // Wait for dependencies
    wait_for_dependencies(&config, &event_tx).await?;

    // Spawn the process (blocking operation for PTY)
    let handle = tokio::task::spawn_blocking(move || {
        spawn_process(&config)
    }).await??;

    _ = event_tx.send(ProjectEvent::ProcessStarted(config.name.clone())).await;

    // Stream logs in background
    let (log_tx, mut log_rx) = mpsc::channel(100);
    let log_handle = tokio::spawn(stream_logs(handle.stdout(), log_tx));

    // Run health probes concurrently
    let health_handle = tokio::spawn(run_health_probes(config.clone()));

    // Wait for process exit
    let exit_status = handle.wait().await?;

    _ = event_tx.send(
        ProjectEvent::ProcessExited(config.name, exit_status.clone())
    ).await;

    Ok(exit_status)
}
```

## Memory Considerations

- **Stack vs. Heap:** Large structs (`Project`, `ProcessConfig`) are boxed or Arc'd
- **Ring buffers:** Log buffers use `VecDeque` with max size, dropping oldest entries
- **Arc for shared config:** Configuration is cloned as `Arc`, not deep copied
- **Zero-copy log streaming:** Logs streamed via channels, not collected

```rust
// Log buffer with bounded capacity
use std::collections::VecDeque;

pub struct LogBuffer {
    messages: VecDeque<String>,
    max_size: usize,
    subscribers: Vec<mpsc::Sender<LogMessage>>,
}

impl LogBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            messages: VecDeque::with_capacity(max_size),
            max_size,
            subscribers: Vec::new(),
        }
    }

    pub fn push(&mut self, message: String) {
        if self.messages.len() >= self.max_size {
            self.messages.pop_front();
        }
        self.messages.push_back(message.clone());

        // Notify subscribers (non-blocking)
        self.subscribers.retain(|tx| {
            tx.try_send(LogMessage { message: message.clone() }).is_ok()
        });
    }
}
```

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Circular dependencies | `petgraph::algo::is_cyclic_directed` detects at load time |
| Process dies unexpectedly | `ProcessHandle::wait()` returns error, triggers restart policy |
| Log buffer overflow | `VecDeque::pop_front()` drops oldest entries |
| Health check timeout | `tokio::time::timeout()` aborts probe after deadline |
| Concurrent process start/stop | `Arc<Mutex<HashMap>>` ensures exclusive access |
| Configuration hot-reload | Atomic swap of `Arc<Project>` after validation |
| PTY cleanup on panic | `Drop` impl for `PtyProcess` kills child and closes FD |
| Signal handling during shutdown | `tokio::signal` with graceful shutdown timeout |
| Missing environment variables | `std::env::var` returns `Result`, validation catches early |
| Dependency not found | Graph building fails with `ProcessNotFound` error |

## Code Examples

### Example: Project Runner Core

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{info, error, debug};

use process_compose_core::{
    Project, ProcessConfig, ProcessState, ProcessStatus,
    ProjectInterface, Result, ProcessComposeError,
};
use process_compose_exec::{Executor, ProcessHandle, ExitStatus};
use process_compose_health::{Probe, ProbeChecker, ProbeResult};

/// Main orchestrator for running processes
#[derive(Clone)]
pub struct ProjectRunner {
    project: Arc<Project>,
    processes: Arc<Mutex<HashMap<String, Arc<Mutex<dyn ProcessHandle>>>>>,
    states: Arc<Mutex<HashMap<String, ProcessState>>>,
    event_tx: mpsc::Sender<ProjectEvent>,
    shutdown_tx: mpsc::Sender<()>,
}

#[derive(Debug, Clone)]
pub enum ProjectEvent {
    ProcessStarted(String),
    ProcessExited(String, ExitStatus),
    ProcessRestarting(String),
    LogMessage(LogMessage),
    HealthCheck(String, ProbeResult),
}

impl ProjectRunner {
    /// Create a new project runner
    pub fn new(project: Project, buffer_size: usize) -> Self {
        let (event_tx, _event_rx) = mpsc::channel(buffer_size);
        let (shutdown_tx, _shutdown_rx) = mpsc::channel(1);

        Self {
            project: Arc::new(project),
            processes: Arc::new(Mutex::new(HashMap::new())),
            states: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            shutdown_tx,
        }
    }

    /// Build execution order using topological sort
    pub fn build_run_order(&self) -> Result<Vec<String>> {
        use petgraph::graph::DiGraph;
        use petgraph::algo::{toposort, is_cyclic_directed};

        let mut graph = DiGraph::<&str, ()>::new();
        let mut name_to_node = HashMap::new();

        // Add nodes
        for (name, _config) in self.project.processes.iter() {
            let node = graph.add_node(name.as_str());
            name_to_node.insert(name.as_str(), node);
        }

        // Add edges (dependencies point to dependents)
        for (name, config) in self.project.processes.iter() {
            let from = name_to_node[name.as_str()];
            for dep_name in config.depends_on.keys() {
                if let Some(&to) = name_to_node.get(dep_name.as_str()) {
                    graph.add_edge(from, to, ());
                }
            }
        }

        // Check for cycles
        if is_cyclic_directed(&graph) {
            return Err(ProcessComposeError::CircularDependency(
                "Circular dependency detected in project configuration".to_string()
            ));
        }

        // Topological sort
        let sorted = toposort(&graph, None)
            .map_err(|_| ProcessComposeError::Dependency(
                "Failed to sort processes".to_string()
            ))?;

        Ok(sorted.into_iter()
            .map(|node| graph[node].to_string())
            .collect())
    }

    /// Run all processes in dependency order
    pub async fn run(&self) -> Result<()> {
        let run_order = self.build_run_order()?;
        info!("Starting {} processes", run_order.len());
        debug!("Run order: {:?}", run_order);

        let mut handles = Vec::new();

        for name in run_order {
            if let Some(config) = self.project.processes.get(&name) {
                if config.disabled || config.is_foreground {
                    continue;
                }
                let handle = self.spawn_process(config.clone()).await?;
                handles.push(handle);
            }
        }

        // Wait for all processes to complete
        for handle in handles {
            let _ = handle.await;
        }

        info!("Project completed");
        Ok(())
    }

    /// Spawn a single process task
    async fn spawn_process(
        &self,
        config: ProcessConfig,
    ) -> Result<tokio::task::JoinHandle<Result<ExitStatus>>> {
        let self_clone = self.clone();

        Ok(tokio::spawn(async move {
            self_clone.run_process_lifecycle(config).await
        }))
    }

    /// Run the full lifecycle of a process
    async fn run_process_lifecycle(&self, mut config: ProcessConfig) -> Result<ExitStatus> {
        // Wait for dependencies
        self.wait_for_dependencies(&config).await?;

        // Initialize state
        self.set_process_status(&config.name, ProcessStatus::Pending).await;

        // Main run loop (handles restarts)
        loop {
            match self.run_process_once(&config).await {
                Ok(exit_status) => {
                    // Check restart policy
                    if !self.should_restart(&config, &exit_status).await {
                        self.set_process_status(&config.name, ProcessStatus::Completed).await;
                        return Ok(exit_status);
                    }

                    // Restart
                    self.set_process_status(&config.name, ProcessStatus::Restarting).await;
                    let backoff = std::time::Duration::from_secs(
                        config.restart_policy.backoff_seconds as u64
                    );
                    tokio::time::sleep(backoff).await;
                    config.restart_policy.max_restarts =
                        config.restart_policy.max_restarts.map(|n| n.saturating_sub(1));
                }
                Err(e) => {
                    error!("Process {} failed: {}", config.name, e);
                    if !self.should_restart_on_error(&config).await {
                        self.set_process_status(&config.name, ProcessStatus::Error).await;
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Run a process once (without restart logic)
    async fn run_process_once(&self, config: &ProcessConfig) -> Result<ExitStatus> {
        use process_compose_exec::CommandExecutor;

        self.set_process_status(&config.name, ProcessStatus::Running).await;

        // Spawn the process
        let executor = CommandExecutor::default();
        let handle = executor.spawn(config).await?;
        let pid = handle.pid();

        info!(process = %config.name, pid = %pid, "Process started");

        // Update state with PID
        self.update_process_state(&config.name, |state| {
            state.pid = Some(pid);
            state.is_running = true;
        }).await;

        // Start health probes in background
        let probe_handle = self.spawn_health_probes(config, handle.as_ref()).await;

        // Wait for process exit
        let exit_status = handle.wait().await?;

        info!(
            process = %config.name,
            pid = %pid,
            exit_code = ?exit_status.code,
            "Process exited"
        );

        // Stop probes
        if let Some(probe) = probe_handle {
            probe.abort();
        }

        self.update_process_state(&config.name, |state| {
            state.is_running = false;
            state.exit_code = exit_status.code;
        }).await;

        Ok(exit_status)
    }

    /// Wait for process dependencies to satisfy conditions
    async fn wait_for_dependencies(&self, config: &ProcessConfig) -> Result<()> {
        for (dep_name, dependency) in &config.depends_on {
            info!("Process {} waiting for dependency: {}", config.name, dep_name);

            loop {
                let state = self.get_process_state(dep_name).await?;

                match dependency.condition {
                    ProcessCondition::ProcessCompleted => {
                        if state.status == ProcessStatus::Completed {
                            break;
                        }
                    }
                    ProcessCondition::ProcessCompletedSuccessfully => {
                        if state.status == ProcessStatus::Completed && state.exit_code == Some(0) {
                            break;
                        }
                        if state.status == ProcessStatus::Error && state.exit_code != Some(0) {
                            return Err(ProcessComposeError::Dependency(
                                format!("Dependency {} failed", dep_name)
                            ));
                        }
                    }
                    ProcessCondition::ProcessHealthy => {
                        if state.health == ProcessHealth::Ready {
                            break;
                        }
                        if state.status == ProcessStatus::Error {
                            return Err(ProcessComposeError::Dependency(
                                format!("Dependency {} became unhealthy", dep_name)
                            ));
                        }
                    }
                    ProcessCondition::ProcessStarted => {
                        if state.status == ProcessStatus::Running
                            || state.status == ProcessStatus::Completed {
                            break;
                        }
                    }
                    ProcessCondition::ProcessLogReady => {
                        // Handled by log monitoring
                        break;
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }

        Ok(())
    }

    /// Determine if a process should restart based on exit status and policy
    async fn should_restart(&self, config: &ProcessConfig, exit_status: &ExitStatus) -> bool {
        match config.restart_policy.restart {
            RestartPolicy::Always => {
                config.restart_policy.max_restarts.map_or(true, |n| n > 0)
            }
            RestartPolicy::OnFailure => {
                exit_status.code != Some(0) &&
                config.restart_policy.max_restarts.map_or(true, |n| n > 0)
            }
            RestartPolicy::ExitOnFailure | RestartPolicy::No => false,
        }
    }

    async fn should_restart_on_error(&self, config: &ProcessConfig) -> bool {
        matches!(config.restart_policy.restart, RestartPolicy::Always | RestartPolicy::OnFailure)
    }

    // State management helpers
    async fn set_process_status(&self, name: &str, status: ProcessStatus) {
        let mut states = self.states.lock().await;
        if let Some(state) = states.get_mut(name) {
            state.status = status;
        } else {
            states.insert(name.to_string(), ProcessState {
                name: name.to_string(),
                status,
                ..Default::default()
            });
        }
    }

    async fn get_process_state(&self, name: &str) -> Result<ProcessState> {
        let states = self.states.lock().await;
        states.get(name).cloned().ok_or_else(|| {
            ProcessComposeError::ProcessNotFound(name.to_string())
        })
    }

    async fn update_process_state<F>(&self, name: &str, f: F)
    where
        F: FnOnce(&mut ProcessState),
    {
        let mut states = self.states.lock().await;
        if let Some(state) = states.get_mut(name) {
            f(state);
        }
    }
}

// Implement ProjectInterface trait
impl ProjectInterface for ProjectRunner {
    fn get_lexicographic_process_names(&self) -> Result<Vec<String>> {
        let mut names: Vec<_> = self.project.processes.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    // ... other trait methods
}
```

### Example: Process Execution with PTY

```rust
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use tokio::sync::mpsc;
use tracing::{debug, error};

#[cfg(unix)]
use portable_pty::{CommandBuilder, NativePtySystem, PtySize};

use process_compose_core::{ProcessConfig, Result, ProcessComposeError};
use crate::{ProcessHandle, ExitStatus};

/// Executor for spawning processes
pub struct CommandExecutor {
    #[cfg(unix)]
    pty_system: NativePtySystem,
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self {
            #[cfg(unix)]
            pty_system: NativePtySystem::default(),
        }
    }
}

impl CommandExecutor {
    /// Spawn a process based on configuration
    pub async fn spawn(&self, config: &ProcessConfig) -> Result<Box<dyn ProcessHandle>> {
        if config.is_tty {
            self.spawn_pty(config).await
        } else {
            self.spawn_standard(config).await
        }
    }

    /// Spawn a process with standard pipes
    async fn spawn_standard(&self, config: &ProcessConfig) -> Result<Box<dyn ProcessHandle>> {
        let mut cmd = Command::new(
            config.executable.as_ref().ok_or_else(|| {
                ProcessComposeError::Configuration("No executable specified".to_string())
            })?
        );

        cmd.args(&config.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory
        if let Some(working_dir) = &config.working_dir {
            cmd.current_dir(working_dir);
        }

        // Set environment
        for env in &config.environment {
            if let Some((key, value)) = env.split_once('=') {
                cmd.env(key, value);
            }
        }

        let child = cmd.spawn()
            .map_err(|e| ProcessComposeError::ProcessExecution(
                crate::ExecutionError::SpawnFailed(format!("{}: {}", config.name, e))
            ))?;

        Ok(Box::new(StandardProcessHandle {
            child: tokio::process::Child::from(child),
            name: config.name.clone(),
        }))
    }

    /// Spawn a process with PTY for TTY processes
    #[cfg(unix)]
    async fn spawn_pty(&self, config: &ProcessConfig) -> Result<Box<dyn ProcessHandle>> {
        let mut cmd = CommandBuilder::new(
            config.executable.as_ref().ok_or_else(|| {
                ProcessComposeError::Configuration("No executable specified".to_string())
            })?
        );

        cmd.args(&config.args);

        if let Some(working_dir) = &config.working_dir {
            cmd.cwd(working_dir);
        }

        for env in &config.environment {
            if let Some((key, value)) = env.split_once('=') {
                cmd.env(key, value);
            }
        }

        let pair = self.pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave); // Close slave side

        let master = pair.master;

        Ok(Box::new(PtyProcessHandle {
            child,
            master: Some(master),
            name: config.name.clone(),
        }))
    }

    /// Kill a process by PID
    pub fn kill(&self, pid: u32, signal: i32) -> Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            let signal = Signal::try_from(signal)
                .map_err(|_| ProcessComposeError::Configuration(
                    format!("Invalid signal: {}", signal)
                ))?;
            kill(nix::unistd::Pid::from_raw(pid as i32), signal)?;
        }

        #[cfg(windows)]
        {
            use windows::Win32::System::Threading::{
                OpenProcess, TerminateProcess, PROCESS_ACCESS_RIGHTS
            };
            let handle = unsafe {
                OpenProcess(PROCESS_ACCESS_RIGHTS::PROCESS_TERMINATE, false, pid)?
            };
            unsafe { TerminateProcess(handle, 1)? };
        }

        Ok(())
    }
}

/// Standard process handle (non-PTY)
pub struct StandardProcessHandle {
    child: tokio::process::Child,
    name: String,
}

impl ProcessHandle for StandardProcessHandle {
    fn pid(&self) -> u32 {
        self.child.id().unwrap_or(0)
    }

    async fn wait(&mut self) -> Result<ExitStatus> {
        let status = self.child.wait().await?;
        Ok(ExitStatus {
            code: status.code(),
            signal: None, // Unix signals not available in std::process::ExitStatus
        })
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        Ok(self.child.try_wait()?.map(|status| ExitStatus {
            code: status.code(),
            signal: None,
        }))
    }

    fn kill(&mut self) -> Result<()> {
        self.child.start_kill()?;
        Ok(())
    }

    fn kill_with_signal(&mut self, _signal: i32) -> Result<()> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            let signal = Signal::try_from(_signal)?;
            kill(nix::unistd::Pid::from_raw(self.pid() as i32), signal)?;
        }
        #[cfg(not(unix))]
        {
            self.kill()?;
        }
        Ok(())
    }

    fn stdout(&self) -> Option<Box<dyn Read + Send>> {
        self.child.stdout.as_ref().map(|s| Box::new(s) as Box<dyn Read + Send>)
    }

    fn stderr(&self) -> Option<Box<dyn Read + Send>> {
        self.child.stderr.as_ref().map(|s| Box::new(s) as Box<dyn Read + Send>)
    }

    fn stdin(&self) -> Option<Box<dyn Write + Send>> {
        self.child.stdin.as_ref().map(|s| Box::new(s) as Box<dyn Write + Send>)
    }
}

/// PTY process handle for TTY processes
#[cfg(unix)]
pub struct PtyProcessHandle {
    child: portable_pty::Child,
    master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    name: String,
}

#[cfg(unix)]
impl ProcessHandle for PtyProcessHandle {
    fn pid(&self) -> u32 {
        self.child.process_id() as u32
    }

    async fn wait(&mut self) -> Result<ExitStatus> {
        // PTY wait is blocking, use spawn_blocking
        let mut child = std::mem::replace(&mut self.child, unsafe {
            // Create a dummy child - we're about to replace it
            use std::mem;
            mem::zeroed()
        });

        let exit_status = tokio::task::spawn_blocking(move || {
            child.wait()
        }).await??;

        Ok(ExitStatus {
            code: exit_status.exit_code().map(|c| c as i32),
            signal: None,
        })
    }

    fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        // portable-pty doesn't support try_wait, always return None
        Ok(None)
    }

    fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        Ok(())
    }

    fn kill_with_signal(&mut self, _signal: i32) -> Result<()> {
        // PTY doesn't support signals directly
        self.kill()
    }

    fn stdout(&self) -> Option<Box<dyn Read + Send>> {
        self.master.as_ref().and_then(|m| {
            m.try_clone_reader().ok().map(|r| Box::new(r) as Box<dyn Read + Send>)
        })
    }

    fn stderr(&self) -> Option<Box<dyn Read + Send>> {
        None // PTY typically doesn't separate stderr
    }

    fn stdin(&self) -> Option<Box<dyn Write + Send>> {
        self.master.as_ref().and_then(|m| {
            m.take_writer().ok().map(|w| Box::new(w) as Box<dyn Write + Send>)
        })
    }
}

impl Drop for PtyProcessHandle {
    fn drop(&mut self) {
        // Ensure PTY is closed
        drop(self.master.take());
        let _ = self.kill();
    }
}
```

### Example: Dependency Resolution

```rust
use petgraph::graph::DiGraph;
use petgraph::algo::{toposort, is_cyclic_directed};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

use process_compose_core::{Processes, ProcessConfig, ProcessCondition, Result};
use process_compose_core::ProcessComposeError;

/// Dependency resolver using graph algorithms
pub struct DependencyResolver<'a> {
    processes: &'a Processes,
    graph: DiGraph<&'a str, DependencyType>,
    name_to_node: HashMap<&'a str, petgraph::graph::NodeIndex>,
}

#[derive(Debug, Clone, Copy)]
enum DependencyType {
    DependsOn,  // A depends on B
}

impl<'a> DependencyResolver<'a> {
    /// Create a new dependency resolver
    pub fn new(processes: &'a Processes) -> Self {
        let mut graph = DiGraph::<&str, DependencyType>::new();
        let mut name_to_node = HashMap::new();

        // Add all processes as nodes
        for (name, _config) in processes.iter() {
            let node = graph.add_node(name.as_str());
            name_to_node.insert(name.as_str(), node);
        }

        // Add dependency edges
        for (name, config) in processes.iter() {
            let from = name_to_node[name.as_str()];
            for dep_name in config.depends_on.keys() {
                if let Some(&to) = name_to_node.get(dep_name.as_str()) {
                    // Edge from dependent to dependency
                    graph.add_edge(from, to, DependencyType::DependsOn);
                }
            }
        }

        Self {
            processes,
            graph,
            name_to_node,
        }
    }

    /// Check for circular dependencies
    pub fn has_circular_dependency(&self) -> Option<Vec<String>> {
        use petgraph::visit::VisitMap;

        // Find cycle using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut cycle_path = Vec::new();

        fn dfs<'a>(
            node: petgraph::graph::NodeIndex,
            graph: &DiGraph<&'a str, DependencyType>,
            visited: &mut HashSet<petgraph::graph::NodeIndex>,
            rec_stack: &mut HashSet<petgraph::graph::NodeIndex>,
            path: &mut Vec<&'a str>,
        ) -> Option<Vec<String>> {
            visited.insert(node);
            rec_stack.insert(node);
            path.push(graph[node]);

            for edge in graph.edges(node) {
                let neighbor = edge.target();
                if !visited.contains(&neighbor) {
                    if let Some(cycle) = dfs(neighbor, graph, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&neighbor) {
                    // Found cycle
                    let cycle_start = path.iter().position(|&n| n == graph[neighbor]).unwrap();
                    let mut cycle: Vec<String> = path[cycle_start..].iter().map(|s| s.to_string()).collect();
                    cycle.push(graph[neighbor].to_string());
                    return Some(cycle);
                }
            }

            path.pop();
            rec_stack.remove(&node);
            None
        }

        for node in self.graph.node_indices() {
            if !visited.contains(&node) {
                if let Some(cycle) = dfs(node, &self.graph, &mut visited, &mut rec_stack, &mut cycle_path) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// Get topologically sorted process names
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        // Check for cycles first
        if let Some(cycle) = self.has_circular_dependency() {
            return Err(ProcessComposeError::CircularDependency(
                format!("Circular dependency: {}", cycle.join(" -> "))
            ));
        }

        let sorted = toposort(&self.graph, None)
            .map_err(|e| ProcessComposeError::Dependency(
                format!("Topological sort failed: {:?}", e)
            ))?;

        Ok(sorted.into_iter()
            .map(|node| self.graph[node].to_string())
            .collect())
    }

    /// Get all transitive dependencies of a process
    pub fn get_all_dependencies(&self, process_name: &str) -> HashSet<String> {
        let mut deps = HashSet::new();
        let Some(&node) = self.name_to_node.get(process_name) else {
            return deps;
        };

        // DFS to find all dependencies
        let mut stack = vec![node];
        while let Some(current) = stack.pop() {
            for edge in self.graph.edges(current) {
                let neighbor = edge.target();
                if deps.insert(self.graph[neighbor].to_string()) {
                    stack.push(neighbor);
                }
            }
        }

        deps
    }

    /// Get reverse dependencies (what depends on this process)
    pub fn get_dependents(&self, process_name: &str) -> HashSet<String> {
        let mut dependents = HashSet::new();
        let Some(&node) = self.name_to_node.get(process_name) else {
            return dependents;
        };

        // Search in reverse direction
        for edge in self.graph.edges_directed(node, petgraph::Direction::Incoming) {
            dependents.insert(self.graph[edge.source()].to_string());
        }

        dependents
    }

    /// Validate that all dependencies exist
    pub fn validate_dependencies(&self) -> Vec<ProcessComposeError> {
        let mut errors = Vec::new();

        for (name, config) in self.processes.iter() {
            for dep_name in config.depends_on.keys() {
                if !self.processes.contains_key(dep_name) {
                    errors.push(ProcessComposeError::Dependency(
                        format!("Process '{}' depends on '{}', which does not exist",
                                name, dep_name)
                    ));
                }
            }
        }

        errors
    }
}

/// Build and run processes in dependency order
pub async fn run_project_with_dependencies(
    processes: &Processes,
) -> Result<()> {
    let resolver = DependencyResolver::new(processes);

    // Validate first
    let errors = resolver.validate_dependencies();
    if !errors.is_empty() {
        return Err(ProcessComposeError::Dependency(
            errors.into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    // Get run order
    let run_order = resolver.topological_sort()?;

    // Run processes
    for name in run_order {
        if let Some(config) = processes.get(&name) {
            println!("Starting process: {}", name);
            // spawn_and_run(config).await?;
        }
    }

    Ok(())
}
```

## Migration Path

### Phase 1: Foundation (Week 1-2)
1. Set up workspace structure with all crates
2. Implement core types (`Project`, `ProcessConfig`, `ProcessState`)
3. Implement YAML loader with serde
4. Create basic error types with thiserror

### Phase 2: Process Execution (Week 3-4)
1. Implement `CommandExecutor` for standard processes
2. Add PTY support for TTY processes
3. Implement process state tracking
4. Write unit tests for execution logic

### Phase 3: Orchestration (Week 5-6)
1. Implement dependency resolution with petgraph
2. Build `ProjectRunner` with async task spawning
3. Add restart policy logic
4. Implement shutdown ordering

### Phase 4: Health Checks (Week 7)
1. Implement `ExecProbe` checker
2. Implement `HttpProbe` checker with reqwest
3. Add health check scheduling
4. Integrate with process lifecycle

### Phase 5: REST API (Week 8-9)
1. Set up axum server
2. Implement all route handlers
3. Add OpenAPI docs with utoipa
4. Implement WebSocket log streaming

### Phase 6: TUI (Week 10-12)
1. Set up ratatui application structure
2. Implement process table view
3. Implement log viewer
4. Add keyboard shortcuts and commands
5. Theme support

### Phase 7: Polish (Week 13-14)
1. Integration tests
2. Performance optimization
3. Documentation
4. Release preparation

## Performance Considerations

1. **Async I/O for logs:** Log streaming uses async channels, avoiding blocking
2. **Bounded ring buffers:** Log buffers have fixed capacity, preventing memory growth
3. **Lazy health probes:** Probes only run when process is in appropriate state
4. **Arc for shared config:** Configuration is shared, not cloned
5. **Batched UI updates:** TUI refreshes at fixed interval, not on every event
6. **Process info caching:** CPU/memory stats cached between refresh cycles

```rust
// Example: Rate-limited process stats collection
use std::time::{Duration, Instant};

pub struct ProcessStatsCollector {
    last_update: Instant,
    update_interval: Duration,
}

impl ProcessStatsCollector {
    pub fn new(update_interval_secs: u64) -> Self {
        Self {
            last_update: Instant::now(),
            update_interval: Duration::from_secs(update_interval_secs),
        }
    }

    pub async fn collect(&mut self, pid: u32) -> Option<ProcessStats> {
        if self.last_update.elapsed() < self.update_interval {
            return None; // Rate limited
        }

        self.last_update = Instant::now();

        // Collect stats using sysinfo
        let mut system = sysinfo::System::new();
        system.refresh_process(sysinfo::Pid::from_u32(pid));

        system.process(sysinfo::Pid::from_u32(pid))
            .map(|p| ProcessStats {
                memory_bytes: p.memory(),
                cpu_percent: p.cpu_usage(),
            })
    }
}
```

## Testing Strategy

```rust
// Unit tests in each crate
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_config_deserialize() {
        let yaml = r#"
name: test_process
command: echo hello
availability:
  restart: always
  backoff_seconds: 5
depends_on:
  dep_process:
    condition: process_healthy
"#;

        let config: ProcessConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "test_process");
        assert_eq!(config.restart_policy.restart, RestartPolicy::Always);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut processes = Processes::new();
        processes.insert("a".to_string(), ProcessConfig {
            name: "a".to_string(),
            depends_on: HashMap::from([("b".to_string(), ProcessDependency::default())]),
            ..Default::default()
        });
        processes.insert("b".to_string(), ProcessConfig {
            name: "b".to_string(),
            depends_on: HashMap::from([("a".to_string(), ProcessDependency::default())]),
            ..Default::default()
        });

        let resolver = DependencyResolver::new(&processes);
        assert!(resolver.has_circular_dependency().is_some());
    }
}

// Integration tests
#[cfg(test)]
mod integration {
    use process_compose_core::*;
    use process_compose_loader::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_full_project_run() {
        let project = Loader::new()
            .load(Path::new("tests/fixtures/simple-project.yaml"))
            .await
            .unwrap();

        let runner = ProjectRunner::new(project, 100);
        let result = runner.run().await;
        assert!(result.is_ok());
    }
}
```

## Open Considerations

1. **Windows PTY support:** `portable-pty` has limited Windows support. May need ConPTY-specific handling.

2. **Signal handling on Windows:** Windows signals differ from Unix. The `nix` crate doesn't work on Windows; need `windows` crate alternatives.

3. **Elevated process support:** The Go version supports `sudo`/`runas` for elevated processes. Rust implementation needs careful password handling (consider using `secrets` crate or platform-specific secure storage).

4. **Log rotation:** Consider using `tracing-appender` with rolling file appender instead of custom implementation.

5. **Plugin system:** The Go version has `x-` extension fields. Consider a plugin trait for custom process types.

6. **Metrics/observability:** Consider adding `metrics` crate for Prometheus-compatible metrics export.
