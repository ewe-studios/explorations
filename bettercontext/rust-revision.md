# BetterContext - Rust Revision Guide

## Goal

Reproduce BetterContext (BTCA) functionality in Rust at production level with focus on:
- Context management for AI coding
- Improving AI code understanding
- Efficient resource loading and caching
- Tool execution (read, grep, glob, list)
- Streaming responses

---

## Architecture Overview

```
btca-rs/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── btca-core/          # Core agent logic and types
│   ├── btca-vfs/           # Virtual filesystem
│   ├── btca-tools/         # Tool implementations
│   ├── btca-resources/     # Git/local resource management
│   ├── btca-server/        # HTTP server with SSE
│   ├── btca-cli/           # CLI application
│   └── btca-auth/          # Provider authentication
└── apps/
    └── btca/               # Binary crate
```

---

## Core Dependencies

```toml
# Workspace Cargo.toml
[workspace]
members = ["crates/*", "apps/*"]
resolver = "2"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"

# HTTP server
axum = { version = "0.8", features = ["macros"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# HTTP client
reqwest = { version = "0.12", features = ["json", "stream"] }
hyper = "1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Git operations
git2 = "0.19"

# Filesystem
tokio-util = { version = "0.7", features = ["io"] }
notify = "7"       # File watching
ignore = "0.4"     # .gitignore parsing

# Search
regex = "1"
globset = "0.4"
ripgrep = "0.19"   # For fast grep

# Error handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# AI SDK compatibility
async-stream = "0.3"
futures = "0.3"

# Configuration
config = "0.14"
dotenvy = "0.15"

# CLI
clap = { version = "4", features = ["derive"] }
crossterm = "0.28"  # TUI support
ratatui = "0.29"    # TUI rendering

# Auth
oauth2 = "5"
reqwest-cookie = "1"
```

---

## Virtual Filesystem (btca-vfs)

### Design

```rust
// crates/btca-vfs/src/lib.rs
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

pub type VfsId = String;

#[derive(Debug, Clone)]
pub enum VfsEntry {
    File { content: Vec<u8>, mtime: u64 },
    Directory { entries: HashMap<String, VfsEntry> },
}

pub struct VirtualFs {
    instances: RwLock<HashMap<VfsId, VfsEntry>>,
}

impl VirtualFs {
    pub fn new() -> Self {
        let mut instances = HashMap::new();
        instances.insert("default".to_string(), VfsEntry::Directory { entries: HashMap::new() });
        Self {
            instances: RwLock::new(instances),
        }
    }

    pub async fn create(&self) -> VfsId {
        let id = uuid::Uuid::new_v4().to_string();
        self.instances.write().await.insert(
            id.clone(),
            VfsEntry::Directory { entries: HashMap::new() },
        );
        id
    }

    pub async fn import_from_disk(
        &self,
        vfs_id: &str,
        source: &Path,
        dest: &Path,
        ignore: Option<&dyn Fn(&Path) -> bool>,
    ) -> anyhow::Result<()> {
        // Recursively import directory into VFS
        self.import_recursive(vfs_id, source, source, dest, ignore.as_ref()).await
    }

    pub async fn read_file(&self, vfs_id: &str, path: &Path) -> anyhow::Result<Vec<u8>> {
        let instances = self.instances.read().await;
        let root = instances.get(vfs_id)
            .ok_or_else(|| anyhow::anyhow!("VFS instance not found"))?;
        self.get_file_content(root, path)
    }

    pub async fn list_files_recursive(&self, vfs_id: &str, root: &Path) -> Vec<PathBuf> {
        // BFS traversal
    }

    pub async fn dispose(&self, vfs_id: &str) {
        self.instances.write().await.remove(vfs_id);
    }
}
```

### Key Operations

```rust
// crates/btca-vfs/src/ops.rs
pub trait VfsOperations {
    async fn mkdir(&self, vfs_id: &str, path: &Path, recursive: bool) -> anyhow::Result<()>;
    async fn writeFile(&self, vfs_id: &str, path: &Path, content: &[u8]) -> anyhow::Result<()>;
    async fn readFile(&self, vfs_id: &str, path: &Path) -> anyhow::Result<Vec<u8>>;
    async fn readdir(&self, vfs_id: &str, path: &Path) -> anyhow::Result<Vec<String>>;
    async fn stat(&self, vfs_id: &str, path: &Path) -> anyhow::Result<VfsStat>;
    async fn exists(&self, vfs_id: &str, path: &Path) -> bool;
    async fn remove(&self, vfs_id: &str, path: &Path, recursive: bool) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct VfsStat {
    pub is_file: bool,
    pub is_directory: bool,
    pub size: u64,
    pub mtime: u64,
}
```

---

## Tool Implementations (btca-tools)

### Read Tool

```rust
// crates/btca-tools/src/read.rs
use btca_vfs::{VfsId, VirtualFs};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ReadParams {
    pub path: String,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ReadResult {
    pub title: String,
    pub output: String,
    pub metadata: ReadMetadata,
    pub attachments: Option<Vec<FileAttachment>>,
}

#[derive(Debug, Serialize)]
pub struct ReadMetadata {
    pub lines: u64,
    pub truncated: bool,
    pub truncated_by_lines: bool,
    pub truncated_by_bytes: bool,
    pub is_image: bool,
    pub is_pdf: bool,
    pub is_binary: bool,
}

#[derive(Debug, Serialize)]
pub struct FileAttachment {
    #[serde(rename = "type")]
    pub attachment_type: String,
    pub mime: String,
    pub data: String, // base64
}

const MAX_LINES: u64 = 2000;
const MAX_BYTES: u64 = 50 * 1024;
const MAX_LINE_LENGTH: usize = 2000;

pub async fn execute_read(
    vfs: &VirtualFs,
    vfs_id: &VfsId,
    base_path: &Path,
    params: ReadParams,
) -> anyhow::Result<ReadResult> {
    let full_path = base_path.join(&params.path);

    // Check file exists
    if !vfs.exists(vfs_id, &full_path).await {
        return Ok(ReadResult {
            title: params.path,
            output: format!("File not found: {}", params.path),
            metadata: ReadMetadata { lines: 0, truncated: false, ..default() },
            attachments: None,
        });
    }

    let content = vfs.read_file(vfs_id, &full_path).await?;

    // Check for binary/image/PDF
    if is_image(&params.path) {
        return Ok(ReadResult {
            title: params.path,
            output: format!("[Image file: {}]", Path::new(&params.path).file_name().unwrap()),
            metadata: ReadMetadata { is_image: true, ..default() },
            attachments: Some(vec![FileAttachment {
                attachment_type: "file".to_string(),
                mime: image_mime(&params.path),
                data: base64::encode(&content),
            }]),
        });
    }

    if is_binary(&content) {
        return Ok(ReadResult {
            title: params.path,
            output: format!("[Binary file: {}]", Path::new(&params.path).file_name().unwrap()),
            metadata: ReadMetadata { is_binary: true, ..default() },
            attachments: None,
        });
    }

    // Read text file with truncation
    let text = String::from_utf8_lossy(&content);
    let lines: Vec<&str> = text.lines().collect();

    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(MAX_LINES);

    let mut output_lines = Vec::new();
    let mut total_bytes = 0u64;
    let mut truncated_by_lines = false;
    let mut truncated_by_bytes = false;

    for (i, line) in lines.iter().enumerate().skip(offset as usize).take(limit as usize) {
        let line = if line.len() > MAX_LINE_LENGTH {
            format!("{}...", &line[..MAX_LINE_LENGTH])
        } else {
            line.to_string()
        };

        let line_bytes = line.len() as u64;
        if total_bytes + line_bytes > MAX_BYTES {
            truncated_by_bytes = true;
            break;
        }

        output_lines.push(format!("{:5}\t{}", i + 1, line));
        total_bytes += line_bytes;
    }

    truncated_by_lines = !truncated_by_bytes && output_lines.len() as u64 >= limit;

    Ok(ReadResult {
        title: params.path,
        output: output_lines.join("\n"),
        metadata: ReadMetadata {
            lines: output_lines.len() as u64,
            truncated: truncated_by_lines || truncated_by_bytes,
            truncated_by_lines,
            truncated_by_bytes,
            ..default()
        },
        attachments: None,
    })
}
```

### Grep Tool

```rust
// crates/btca-tools/src/grep.rs
use regex::Regex;
use btca_vfs::{VfsId, VirtualFs};

const MAX_RESULTS: usize = 100;

#[derive(Debug, Deserialize)]
pub struct GrepParams {
    pub pattern: String,
    pub path: Option<String>,
    pub include: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GrepResult {
    pub title: String,
    pub output: String,
    pub metadata: GrepMetadata,
}

#[derive(Debug, Serialize)]
pub struct GrepMetadata {
    pub match_count: usize,
    pub file_count: usize,
    pub truncated: bool,
}

#[derive(Debug)]
struct GrepMatch {
    path: PathBuf,
    line_number: usize,
    line_text: String,
    mtime: u64,
}

pub async fn execute_grep(
    vfs: &VirtualFs,
    vfs_id: &VfsId,
    base_path: &Path,
    params: GrepParams,
) -> anyhow::Result<GrepResult> {
    let search_path = params.path
        .map(|p| base_path.join(p))
        .unwrap_or_else(|| base_path.to_path_buf());

    let regex = match Regex::new(&params.pattern) {
        Ok(r) => r,
        Err(_) => return Ok(GrepResult {
            title: params.pattern,
            output: "Invalid regex pattern.".to_string(),
            metadata: GrepMetadata { match_count: 0, file_count: 0, truncated: false },
        }),
    };

    let include_glob = params.include.as_ref()
        .map(|p| globset::Glob::new(p).map(|g| g.compile_matcher()));

    let all_files = vfs.list_files_recursive(vfs_id, &search_path).await;
    let mut results = Vec::new();

    for file_path in all_files {
        if results.len() >= MAX_RESULTS {
            break;
        }

        // Check include pattern
        if let Some(ref matcher) = include_glob {
            let rel_path = file_path.strip_prefix(&search_path).unwrap_or(&file_path);
            if !matcher.is_match(rel_path) {
                continue;
            }
        }

        // Read file
        let content = match vfs.read_file(vfs_id, &file_path).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Skip binary
        if is_binary(&content) {
            continue;
        }

        let text = String::from_utf8_lossy(&content);
        for (i, line) in text.lines().enumerate() {
            if regex.is_match(line) {
                results.push(GrepMatch {
                    path: file_path.clone(),
                    line_number: i + 1,
                    line_text: line[..line.len().min(200)].to_string(),
                    mtime: 0, // Get from stat
                });

                if results.len() >= MAX_RESULTS {
                    break;
                }
            }
        }
    }

    // Sort by mtime, group by file
    results.sort_by(|a, b| b.mtime.cmp(&a.mtime));

    let mut file_groups: std::collections::HashMap<PathBuf, Vec<&GrepMatch>> =
        std::collections::HashMap::new();

    for result in &results {
        let rel_path = result.path.strip_prefix(base_path)
            .unwrap_or(&result.path)
            .to_path_buf();
        file_groups.entry(rel_path).or_default().push(result);
    }

    let mut output_lines = Vec::new();
    for (file_path, matches) in &file_groups {
        output_lines.push(format!("{}:", file_path.display()));
        for m in matches {
            output_lines.push(format!("  {}: {}", m.line_number, m.line_text));
        }
        output_lines.push(String::new());
    }

    let truncated = results.len() > MAX_RESULTS;
    if truncated {
        output_lines.push(format!(
            "[Truncated: Results limited to {} matches]",
            MAX_RESULTS
        ));
    }

    Ok(GrepResult {
        title: params.pattern,
        output: output_lines.join("\n"),
        metadata: GrepMetadata {
            match_count: results.len(),
            file_count: file_groups.len(),
            truncated,
        },
    })
}
```

---

## Resource Management (btca-resources)

```rust
// crates/btca-resources/src/lib.rs
use git2::{Repository, RepositoryInitOptions};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResourceDefinition {
    #[serde(rename = "git")]
    Git(GitResource),
    #[serde(rename = "local")]
    Local(LocalResource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitResource {
    pub name: String,
    pub url: String,
    pub branch: String,
    #[serde(default)]
    pub search_paths: Vec<String>,
    #[serde(default, rename = "searchPath")]
    pub search_path: Option<String>,
    #[serde(default, rename = "specialNotes")]
    pub special_notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalResource {
    pub name: String,
    pub path: String,
    #[serde(default, rename = "specialNotes")]
    pub special_notes: Option<String>,
}

pub struct ResourceManager {
    resources_dir: PathBuf,
}

impl ResourceManager {
    pub fn new(resources_dir: PathBuf) -> Self {
        Self { resources_dir }
    }

    pub async fn load_resource(&self, def: &ResourceDefinition) -> anyhow::Result<LoadedResource> {
        match def {
            ResourceDefinition::Git(git) => self.load_git_resource(git).await,
            ResourceDefinition::Local(local) => self.load_local_resource(local),
        }
    }

    async fn load_git_resource(&self, git: &GitResource) -> anyhow::Result<LoadedResource> {
        let repo_path = self.resources_dir.join(&git.name);

        if !repo_path.exists() {
            // Clone repository
            tracing::info!("Cloning {} to {}", git.url, repo_path.display());
            Repository::clone(&git.url, &repo_path)?;
        } else {
            // Fetch latest
            let repo = Repository::open(&repo_path)?;
            tracing::info!("Fetching {}", git.name);

            let mut remote = repo.find_remote("origin")?;
            remote.fetch(&[&git.branch], None, None)?;

            // Checkout branch
            let (object, _) = repo.revparse_ext(&format!("origin/{}", git.branch))?;
            repo.checkout_tree(&object, None)?;
        }

        Ok(LoadedResource::Git {
            name: git.name.clone(),
            path: repo_path,
            search_paths: self.normalize_search_paths(git),
            special_notes: git.special_notes.clone(),
        })
    }

    fn load_local_resource(&self, local: &LocalResource) -> anyhow::Result<LoadedResource> {
        let path = PathBuf::from(&local.path);
        if !path.exists() {
            anyhow::bail!("Local resource path not found: {}", local.path);
        }

        Ok(LoadedResource::Local {
            name: local.name.clone(),
            path,
            special_notes: local.special_notes.clone(),
        })
    }

    fn normalize_search_paths(&self, git: &GitResource) -> Vec<String> {
        let mut paths = git.search_paths.clone();
        if let Some(ref p) = git.search_path {
            paths.push(p.clone());
        }
        paths.into_iter().filter(|p| !p.trim().is_empty()).collect()
    }
}

pub enum LoadedResource {
    Git {
        name: String,
        path: PathBuf,
        search_paths: Vec<String>,
        special_notes: Option<String>,
    },
    Local {
        name: String,
        path: PathBuf,
        special_notes: Option<String>,
    },
}

impl LoadedResource {
    pub fn get_path(&self) -> &Path {
        match self {
            LoadedResource::Git { path, .. } | LoadedResource::Local { path, .. } => path,
        }
    }

    pub fn get_search_paths(&self) -> Vec<PathBuf> {
        match self {
            LoadedResource::Git { search_paths, path } => {
                search_paths.iter().map(|p| path.join(p)).collect()
            }
            LoadedResource::Local { path, .. } => vec![path.clone()],
        }
    }
}
```

---

## Agent Loop (btca-core)

```rust
// crates/btca-core/src/agent.rs
use async_stream::stream;
use futures::stream::Stream;
use serde_json::json;

pub enum AgentEvent {
    TextDelta { text: String },
    ToolCall { tool_name: String, input: serde_json::Value },
    ToolResult { tool_name: String, output: String },
    Finish { usage: TokenUsage },
    Error { message: String },
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

pub struct AgentConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub max_steps: u32,
}

pub async fn run_agent_loop(
    config: &AgentConfig,
    question: &str,
    collection_path: &Path,
    vfs_id: &str,
) -> impl Stream<Item = AgentEvent> {
    stream! {
        // Build system prompt
        let system_prompt = build_system_prompt();

        // Initialize tools
        let tools = create_tools(collection_path, vfs_id);

        // Track conversation
        let mut messages = vec![json!({
            "role": "user",
            "content": question
        })];

        let mut total_input_tokens = 0u32;
        let mut total_output_tokens = 0u32;

        for step in 0..config.max_steps {
            // Call LLM
            let response = call_llm(
                &config.provider,
                &config.model,
                &config.api_key,
                &system_prompt,
                &messages,
                &tools,
            ).await;

            match response {
                Ok(mut stream) => {
                    let mut full_response = String::new();
                    let mut tool_calls = Vec::new();

                    while let Some(part) = stream.next().await {
                        match part {
                            LlmStreamPart::TextDelta { text } => {
                                full_response.push_str(&text);
                                yield AgentEvent::TextDelta { text };
                            }
                            LlmStreamPart::ToolCall { name, input } => {
                                tool_calls.push((name.clone(), input.clone()));
                                yield AgentEvent::ToolCall {
                                    tool_name: name,
                                    input,
                                };
                            }
                            LlmStreamPart::Usage { input, output } => {
                                total_input_tokens += input;
                                total_output_tokens += output;
                            }
                        }
                    }

                    // Execute tool calls
                    if tool_calls.is_empty() {
                        // No more tools, we're done
                        yield AgentEvent::Finish {
                            usage: TokenUsage {
                                input_tokens: total_input_tokens,
                                output_tokens: total_output_tokens,
                            },
                        };
                        break;
                    }

                    // Execute tools and collect results
                    for (tool_name, input) in tool_calls {
                        let output = execute_tool(&tool_name, input, collection_path, vfs_id).await;
                        yield AgentEvent::ToolResult {
                            tool_name: tool_name.clone(),
                            output: output.clone(),
                        };

                        messages.push(json!({
                            "role": "assistant",
                            "content": null,
                            "tool_calls": [{
                                "id": format!("call_{}", step),
                                "type": "function",
                                "function": {
                                    "name": tool_name,
                                    "arguments": input,
                                }
                            }]
                        }));

                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": format!("call_{}", step),
                            "content": output,
                        }));
                    }
                }
                Err(e) => {
                    yield AgentEvent::Error { message: e.to_string() };
                    break;
                }
            }
        }
    }
}

fn build_system_prompt() -> String {
    r#"You are btca, an expert documentation search agent.
Your job is to answer questions by searching through the collection of resources.

You have access to the following tools:
- read: Read file contents with line numbers
- grep: Search file contents using regex patterns
- glob: Find files matching glob patterns
- list: List directory contents

Guidelines:
- Use glob to find relevant files first, then read them
- Use grep to search for specific code patterns or text
- Always cite the source files in your answers
- Be concise but thorough in your responses
- If you cannot find the answer, say so clearly
"#.to_string()
}
```

---

## HTTP Server with SSE (btca-server)

```rust
// crates/btca-server/src/lib.rs
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
    response::sse::{Event, KeepAlive, Sse},
};
use axum_macros::debug_handler;
use futures::Stream;
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;

pub struct ServerState {
    pub config: ServerConfig,
    pub agent: AgentHandle,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct QuestionRequest {
    pub question: String,
    pub resources: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct QuestionResponse {
    pub answer: String,
    pub model: ModelInfo,
}

#[derive(Debug, serde::Serialize)]
pub struct ModelInfo {
    pub provider: String,
    pub model: String,
}

pub fn create_router(state: ServerState) -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/config", get(get_config))
        .route("/resources", get(list_resources))
        .route("/question", post(ask_question))
        .route("/question/stream", post(ask_question_stream))
        .with_state(state)
}

#[debug_handler]
async fn health_check() -> &'static str {
    "ok"
}

#[debug_handler]
async fn get_config(State(state): State<ServerState>) -> Json<ServerConfig> {
    Json(state.config)
}

#[debug_handler]
async fn ask_question(
    State(state): State<ServerState>,
    Json(req): Json<QuestionRequest>,
) -> Result<Json<QuestionResponse>, ServerError> {
    let answer = state.agent.answer(&req.question, &req.resources).await?;

    Ok(Json(QuestionResponse {
        answer,
        model: ModelInfo {
            provider: state.config.provider.clone(),
            model: state.config.model.clone(),
        },
    }))
}

#[debug_handler]
async fn ask_question_stream(
    State(state): State<ServerState>,
    Json(req): Json<QuestionRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    // Spawn agent loop in background
    tokio::spawn(async move {
        let stream = state.agent.answer_stream(&req.question, &req.resources).await;

        tokio::pin!(stream);

        while let Some(event) = stream.next().await {
            let sse_event = match event {
                AgentEvent::TextDelta { text } => {
                    Event::default().data(serde_json::json!({
                        "type": "text.delta",
                        "delta": text
                    }).to_string())
                }
                AgentEvent::ToolCall { tool_name, input } => {
                    Event::default().data(serde_json::json!({
                        "type": "tool.updated",
                        "tool": tool_name,
                        "state": { "status": "running" }
                    }).to_string())
                }
                AgentEvent::Finish { usage } => {
                    Event::default().data(serde_json::json!({
                        "type": "done",
                        "usage": {
                            "inputTokens": usage.input_tokens,
                            "outputTokens": usage.output_tokens,
                        }
                    }).to_string())
                }
                AgentEvent::Error { message } => {
                    Event::default().data(serde_json::json!({
                        "type": "error",
                        "message": message
                    }).to_string())
                }
                _ => Event::default().data("{}"),
            };

            if tx.send(Ok(sse_event)).await.is_err() {
                break;
            }
        }
    });

    Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default())
}
```

---

## CLI Application (btca-cli)

```rust
// crates/btca-cli/src/main.rs
use clap::{Parser, Subcommand};
use crossterm::event;
use std::io;

#[derive(Parser)]
#[command(name = "btca")]
#[command(about = "Better Context to AI - AI-powered codebase exploration")]
struct Cli {
    #[arg(long, global = true)]
    server: Option<String>,

    #[arg(long, global = true, default_value = "8080")]
    port: u16,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch interactive TUI
    Tui,

    /// Ask a question
    Ask {
        #[arg(short, long)]
        question: String,

        #[arg(short, long)]
        resource: Vec<String>,
    },

    /// Add a resource
    Add {
        url: String,

        #[arg(short, long)]
        name: Option<String>,

        #[arg(short, long)]
        branch: Option<String>,

        #[arg(short, long)]
        search_path: Vec<String>,
    },

    /// Remove a resource
    Remove {
        name: String,
    },

    /// Configure provider/model
    Connect {
        #[arg(short, long)]
        provider: Option<String>,

        #[arg(short, long)]
        model: Option<String>,
    },

    /// Start standalone server
    Serve {
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) => run_tui().await?,
        Some(Commands::Ask { question, resource }) => {
            run_ask(&question, &resource, cli.server.as_deref(), cli.port).await?;
        }
        Some(Commands::Add { url, name, branch, search_path }) => {
            run_add(&url, name.as_deref(), branch.as_deref(), &search_path).await?;
        }
        Some(Commands::Remove { name }) => {
            run_remove(&name).await?;
        }
        Some(Commands::Connect { provider, model }) => {
            run_connect(provider.as_deref(), model.as_deref()).await?;
        }
        Some(Commands::Serve { port }) => {
            run_server(port).await?;
        }
        None => {
            // Default: launch TUI
            run_tui().await?;
        }
    }

    Ok(())
}

async fn run_ask(
    question: &str,
    resources: &[String],
    server_url: Option<&str>,
    port: u16,
) -> anyhow::Result<()> {
    let base_url = server_url.unwrap_or(&format!("http://localhost:{}", port));

    // Stream response
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/question/stream", base_url))
        .json(&serde_json::json!({
            "question": question,
            "resources": resources,
        }))
        .send()
        .await?;

    let mut stream = response.bytes_stream();
    let mut in_reasoning = false;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);

        // Parse SSE
        for line in text.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                    match event {
                        StreamEvent::ReasoningDelta { delta } => {
                            if !in_reasoning {
                                print!("<thinking>\n");
                                in_reasoning = true;
                            }
                            print!("{}", delta);
                        }
                        StreamEvent::TextDelta { delta } => {
                            if in_reasoning {
                                print!("\n</thinking>\n\n");
                                in_reasoning = false;
                            }
                            print!("{}", delta);
                        }
                        StreamEvent::ToolUpdated { tool } => {
                            if in_reasoning {
                                print!("\n</thinking>\n\n");
                                in_reasoning = false;
                            }
                            println!("[{}]", tool);
                        }
                        StreamEvent::Done { .. } => {
                            println!();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
```

---

## Configuration System

```rust
// crates/btca-core/src/config.rs
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BtcaConfig {
    #[serde(default = "default_data_directory")]
    pub data_directory: String,

    #[serde(default)]
    pub provider: Option<String>,

    #[serde(default)]
    pub model: Option<String>,

    pub resources: Vec<ResourceDefinition>,
}

fn default_data_directory() -> String {
    ".btca".to_string()
}

impl BtcaConfig {
    pub fn load_project(path: &Path) -> anyhow::Result<Self> {
        let config_path = path.join("btca.config.jsonc");
        let content = std::fs::read_to_string(&config_path)?;
        let config: BtcaConfig = serde_jsonc::from_str(&content)?;
        Ok(config)
    }

    pub fn load_global() -> anyhow::Result<Self> {
        let config_dir = config_dir()?;
        let config_path = config_dir.join("btca.config.jsonc");
        Self::load_project(&config_path)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let content = serde_jsonc::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

fn config_dir() -> anyhow::Result<PathBuf> {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").expect("HOME not set");
            PathBuf::from(home).join(".config")
        });

    Ok(config_dir.join("btca"))
}
```

---

## Production Considerations

### Performance Optimizations

1. **Use memory-mapped files** with `memmap2` for large file reads
2. **Parallel grep** using `rayon` for multi-core search
3. **Caching layer** with `moka` for frequently accessed files
4. **Incremental git fetch** to avoid full clones
5. **ripgrep integration** for fastest possible grep

### Error Handling

```rust
// Use thiserror for structured errors
#[derive(Debug, thiserror::Error)]
pub enum BtcaError {
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Invalid provider: {0}")]
    InvalidProvider(String),

    #[error("Authentication required for {0}")]
    AuthRequired(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Git operation failed: {0}")]
    GitError(#[from] git2::Error),
}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_tool_truncation() {
        let vfs = VirtualFs::new();
        let vfs_id = vfs.create().await;

        // Create large file
        let content = "line\n".repeat(3000);
        vfs.write_file(&vfs_id, Path::new("/test.txt"), content.as_bytes()).await.unwrap();

        let result = execute_read(&vfs, &vfs_id, Path::new("/"), ReadParams {
            path: "/test.txt".to_string(),
            offset: Some(0),
            limit: Some(2000),
        }).await.unwrap();

        assert!(result.metadata.truncated);
        assert!(result.metadata.truncated_by_lines);
    }
}
```

---

## Summary

This Rust implementation provides:

1. **Type safety** - Strong typing for all tool inputs/outputs
2. **Performance** - Native speed for grep, glob, file operations
3. **Concurrency** - Async runtime for parallel operations
4. **Streaming** - SSE support for real-time responses
5. **CLI** - Full-featured command line interface
6. **Server** - HTTP API compatible with existing BTCA clients

The architecture mirrors the TypeScript implementation while leveraging Rust's strengths in performance and safety.
