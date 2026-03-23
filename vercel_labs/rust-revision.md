# Vercel Labs AI Tools - Rust Implementation Guide

## Overview

This guide provides a comprehensive roadmap for reproducing Vercel Labs AI tools and patterns in Rust at production level.

**Target Projects:**
- AI SDK (unified LLM interface)
- Agent Browser (browser automation)
- Bash Tool (sandboxed execution)
- Workflow Engine (durable execution)
- RAG Systems (embeddings + vector search)

---

## Table of Contents

1. [AI SDK Equivalent](#1-ai-sdk-equivalent)
2. [Browser Automation](#2-browser-automation)
3. [Sandbox Execution](#3-sandbox-execution)
4. [Workflow Engine](#4-workflow-engine)
5. [RAG Implementation](#5-rag-implementation)
6. [Reasoning Models](#6-reasoning-models)
7. [Production Considerations](#7-production-considerations)

---

## 1. AI SDK Equivalent

### Core Abstractions

The Vercel AI SDK provides a unified interface for multiple LLM providers. In Rust, we need similar abstractions.

### Provider Trait

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use futures::stream::BoxStream;

#[async_trait]
pub trait LanguageModel: Send + Sync {
    /// Stream text completion
    async fn stream_text(
        &self,
        messages: Vec<Message>,
        options: GenerationOptions,
    ) -> Result<BoxStream<'_, Result<Token>>>;

    /// Generate structured output
    async fn generate_object<T: DeserializeOwned>(
        &self,
        messages: Vec<Message>,
        schema: &JsonSchema,
        options: GenerationOptions,
    ) -> Result<T>;

    /// Execute tools and return final result
    async fn generate_with_tools(
        &self,
        messages: Vec<Message>,
        tools: &[ToolDefinition],
        options: GenerationOptions,
    ) -> Result<GenerationResponse>;
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone)]
pub struct MessageContent {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub tool_result: Option<ToolResult>,
}

#[derive(Debug, Clone)]
pub struct GenerationOptions {
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub top_p: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub provider_options: serde_json::Value,  // Provider-specific options
}
```

### Anthropic Provider Implementation

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    system: Option<String>,
    max_tokens: usize,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<AnthropicThinking>,
}

#[derive(Serialize)]
struct AnthropicThinking {
    r#type: String,
    budget_tokens: usize,
}

impl AnthropicProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    pub fn with_thinking(
        &self,
        messages: Vec<Message>,
        thinking_enabled: bool,
        thinking_budget: usize,
    ) -> AnthropicRequest {
        AnthropicRequest {
            model: "claude-3-7-sonnet-20250219".to_string(),
            messages: convert_messages(messages),
            system: extract_system_message(&messages),
            max_tokens: 4096,
            stream: true,
            thinking: if thinking_enabled {
                Some(AnthropicThinking {
                    r#type: "enabled".to_string(),
                    budget_tokens: thinking_budget,
                })
            } else {
                None
            },
        }
    }
}

#[async_trait]
impl LanguageModel for AnthropicProvider {
    async fn stream_text(
        &self,
        messages: Vec<Message>,
        options: GenerationOptions,
    ) -> Result<BoxStream<'_, Result<Token>>> {
        let request = AnthropicRequest {
            model: "claude-3-7-sonnet-20250219".to_string(),
            messages: convert_messages(messages),
            system: extract_system_message(&messages),
            max_tokens: options.max_tokens.unwrap_or(4096),
            stream: true,
            thinking: None,
        };

        let (tx, rx) = tokio::sync::mpsc::channel(32);

        // Spawn SSE streaming
        tokio::spawn({
            let client = self.client.clone();
            let api_key = self.api_key.clone();
            async move {
                let response = client
                    .post(&format!("{}/messages", self.base_url))
                    .header("x-api-key", &api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&request)
                    .send()
                    .await?;

                let mut stream = response.bytes_stream();

                while let Some(chunk) = stream.next().await {
                    let chunk = chunk?;
                    let text = String::from_utf8_lossy(&chunk);

                    // Parse SSE: data: {"type": "content_block_delta", "delta": {"text": "..."}}
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(event) = serde_json::from_str::<AnthropicEvent>(data) {
                                if let Some(text) = event.delta.and_then(|d| d.text) {
                                    tx.send(Ok(Token { text })).await.ok();
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    // ... implement other methods
}
```

### Provider Registry

```rust
use std::collections::HashMap;
use std::sync::Arc;

pub type ModelProvider = Arc<dyn LanguageModel>;

pub struct ProviderRegistry {
    providers: HashMap<String, ModelProvider>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            providers: HashMap::new(),
        };

        // Register default providers
        registry.register("anthropic", Arc::new(AnthropicProvider::new(
            &std::env::var("ANTHROPIC_API_KEY").unwrap()
        )));

        registry.register("openai", Arc::new(OpenAIProvider::new(
            &std::env::var("OPENAI_API_KEY").unwrap()
        )));

        registry.register("groq", Arc::new(GroqProvider::new(
            &std::env::var("GROQ_API_KEY").unwrap()
        )));

        registry
    }

    pub fn register(&mut self, name: &str, provider: ModelProvider) {
        self.providers.insert(name.to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<&ModelProvider> {
        self.providers.get(name)
    }

    pub fn get_model(&self, model_spec: &str) -> Result<ModelProvider> {
        // Parse "provider:model" format (e.g., "anthropic:claude-3-7-sonnet")
        let parts: Vec<&str> = model_spec.split(':').collect();
        if parts.len() == 2 {
            self.get(parts[0]).cloned().ok_or_else(|| {
                anyhow!("Unknown provider: {}", parts[0])
            })
        } else {
            // Default to OpenAI
            self.get("openai").cloned().ok_or_else(|| {
                anyhow!("No default provider configured")
            })
        }
    }
}
```

### Structured Output with JsonSchema

```rust
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

// Use schemars for JSON schema generation from Rust types
#[derive(JsonSchema, Deserialize, Serialize)]
struct LeadQualification {
    category: QualificationCategory,
    reason: String,
    priority: Option<Priority>,
}

#[derive(JsonSchema, Deserialize, Serialize)]
enum QualificationCategory {
    Qualified,
    FollowUp,
    Support,
    NotQualified,
}

#[derive(JsonSchema, Deserialize, Serialize)]
enum Priority {
    High,
    Medium,
    Low,
}

// Generate JSON schema for LLM
fn generate_schema<T: JsonSchema>() -> serde_json::Value {
    let schema = schema_for!(T);
    serde_json::to_value(schema).unwrap()
}

// Use in generate_object
async fn qualify_lead(
    model: &dyn LanguageModel,
    lead_data: &LeadData,
    research: &str,
) -> Result<LeadQualification> {
    let schema = generate_schema::<LeadQualification>();

    model
        .generate_object(
            vec![Message::user(format!(
                "Qualify this lead:\nLead: {:?}\nResearch: {}",
                lead_data, research
            ))],
            &schema,
            GenerationOptions::default(),
        )
        .await
}
```

---

## 2. Browser Automation

### Playwright Alternative

For browser automation, use `playwright-rs` or direct CDP implementation:

```rust
use futures::channel::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use serde::{Deserialize, Serialize};

pub struct BrowserManager {
    cdp_port: Option<u16>,
    ws_url: String,
}

impl BrowserManager {
    pub async fn launch(options: BrowserOptions) -> Result<Self> {
        // For now, use CDP with existing Chrome/Chromium
        // Future: embed chromium via rust-chromium or similar

        let cdp_port = options.cdp_port.unwrap_or(9222);

        Ok(Self {
            cdp_port: Some(cdp_port),
            ws_url: format!("ws://localhost:{}/devtools/browser", cdp_port),
        })
    }

    pub async fn connect(cdp_port: u16) -> Result<Self> {
        // Connect to existing browser via CDP
        Ok(Self {
            cdp_port: Some(cdp_port),
            ws_url: format!("ws://localhost:{}/devtools/browser", cdp_port),
        })
    }

    pub async fn navigate(&self, url: &str) -> Result<()> {
        let cdp = self.get_cdp_session().await?;

        cdp.send("Page.navigate", json!({
            "url": url,
        })).await?;

        Ok(())
    }

    pub async fn get_snapshot(&self) -> Result<AccessibilitySnapshot> {
        let cdp = self.get_cdp_session().await?;

        // Get accessibility tree
        let response: serde_json::Value = cdp.send(
            "Accessibility.getFullAXTree",
            json!({})
        ).await?;

        // Parse and enhance with refs
        let snapshot = parse_ax_tree(response)?;
        Ok(snapshot)
    }

    pub async fn click(&self, selector: &str) -> Result<()> {
        let cdp = self.get_cdp_session().await?;

        // Runtime.evaluate to find element
        let result: serde_json::Value = cdp.send(
            "Runtime.evaluate",
            json!({
                "expression": format!("document.querySelector('{}')", selector),
            })
        ).await?;

        // Dispatch click event
        cdp.send("Input.dispatchMouseEvent", json!({
            "type": "mousePressed",
            "x": /* element coordinates */,
            "y": /* element coordinates */,
            "button": "left",
            "clickCount": 1,
        })).await?;

        Ok(())
    }

    async fn get_cdp_session(&self) -> Result<CDPSession> {
        // Connect to CDP WebSocket
        let (ws, _) = connect_async(&self.ws_url).await?;
        Ok(CDPSession::new(ws))
    }
}

// Accessibility snapshot with refs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilitySnapshot {
    pub tree: String,
    pub refs: RefMap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefMap {
    #[serde(flatten)]
    pub refs: HashMap<String, RefEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefEntry {
    pub role: String,
    pub name: Option<String>,
    pub selector: String,
    pub nth: Option<usize>,
}
```

### Ref-Based Element Selection

```rust
use regex::Regex;

pub struct SnapshotProcessor {
    ref_counter: std::cell::RefCell<usize>,
}

impl SnapshotProcessor {
    pub fn new() -> Self {
        Self {
            ref_counter: std::cell::RefCell::new(0),
        }
    }

    pub fn process_tree(&self, ax_tree: &serde_json::Value) -> AccessibilitySnapshot {
        *self.ref_counter.borrow_mut() = 0;

        let mut refs = HashMap::new();
        let tree = self.process_node(ax_tree, &mut refs, None);

        AccessibilitySnapshot {
            tree,
            refs: RefMap { refs },
        }
    }

    fn process_node(
        &self,
        node: &serde_json::Value,
        refs: &mut HashMap<String, RefEntry>,
        parent_indent: Option<usize>,
    ) -> String {
        let role = node["role"]["value"].as_str().unwrap_or("generic");
        let name = node["name"].and_then(|n| n["value"].as_str());

        let indent = parent_indent.unwrap_or(0) + 2;
        let prefix = " ".repeat(indent);

        // Check if this element should get a ref
        let should_have_ref = self.is_interactive_role(role) || (name.is_some() && self.is_content_role(role));

        let ref_id = if should_have_ref {
            let id = self.next_ref();
            refs.insert(id.clone(), RefEntry {
                role: role.to_string(),
                name: name.map(String::from),
                selector: build_selector(role, name),
                nth: None,
            });
            format!(" [ref={}]", id)
        } else {
            String::new()
        };

        let mut line = format!("{}- {}", prefix, role);
        if let Some(name) = name {
            line.push_str(&format!(" \"{}\"", name));
        }
        line.push_str(&ref_id);

        // Process children
        if let Some(children) = node["children"].as_array() {
            for child in children {
                line.push('\n');
                line.push_str(&self.process_node(child, refs, Some(indent)));
            }
        }

        line
    }

    fn is_interactive_role(&self, role: &str) -> bool {
        matches!(role,
            "button" | "link" | "textbox" | "checkbox" | "radio" |
            "combobox" | "listbox" | "menuitem" | "slider" | "tab"
        )
    }

    fn is_content_role(&self, role: &str) -> bool {
        matches!(role,
            "heading" | "cell" | "listitem" | "article" | "region"
        )
    }

    fn next_ref(&self) -> String {
        let mut counter = self.ref_counter.borrow_mut();
        *counter += 1;
        format!("e{}", counter)
    }
}
```

### CLI Interface

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agent-browser")]
#[command(about = "Headless browser automation for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long)]
    session: Option<String>,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    cdp: Option<u16>,
}

#[derive(Subcommand)]
enum Commands {
    /// Navigate to URL
    Open { url: String },
    /// Get accessibility snapshot with refs
    Snapshot {
        #[arg(short, long)]
        interactive: bool,
        #[arg(short, long)]
        compact: bool,
    },
    /// Click element
    Click { selector: String },
    /// Fill input
    Fill { selector: String, text: String },
    /// Get text content
    Get {
        #[command(subcommand)]
        action: GetAction,
    },
    /// Close browser
    Close,
}

#[derive(Subcommand)]
enum GetAction {
    Text { selector: String },
    Html { selector: String },
    Value { selector: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Connect to daemon or start new browser
    let browser = connect_to_daemon(cli.session.as_deref()).await?;

    // Execute command
    match cli.command {
        Some(Commands::Open { url }) => {
            browser.navigate(&url).await?;
            println!("Navigated to {}", url);
        }
        Some(Commands::Snapshot { interactive, compact }) => {
            let snapshot = browser.get_snapshot().await?;
            let filtered = filter_snapshot(snapshot, interactive, compact);
            if cli.json {
                println!("{}", serde_json::to_string(&filtered)?);
            } else {
                println!("{}", filtered.tree);
            }
        }
        Some(Commands::Click { selector }) => {
            let locator = parse_selector(&selector, browser.get_ref_map());
            browser.click(&locator).await?;
            println!("Clicked {}", selector);
        }
        // ... handle other commands
        None => {
            println!("Usage: agent-browser <command> [options]");
            println!("Run 'agent-browser --help' for more info");
        }
    }

    Ok(())
}
```

---

## 3. Sandbox Execution

### In-Memory Filesystem

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct InMemoryFs {
    files: Arc<RwLock<HashMap<String, String>>>,
    directories: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl InMemoryFs {
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            directories: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn write(&self, path: &str, content: &str) -> Result<()> {
        let mut files = self.files.write().await;

        // Create parent directories
        if let Some(parent) = std::path::Path::new(path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            let file_name = std::path::Path::new(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let mut dirs = self.directories.write().await;
            dirs.entry(parent_str)
                .or_insert_with(Vec::new)
                .push(file_name);
        }

        files.insert(path.to_string(), content.to_string());
        Ok(())
    }

    pub async fn read(&self, path: &str) -> Result<String> {
        let files = self.files.read().await;
        files
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow!("File not found: {}", path))
    }

    pub async fn list(&self, path: &str) -> Result<Vec<String>> {
        let dirs = self.directories.read().await;
        dirs.get(path)
            .cloned()
            .ok_or_else(|| anyhow!("Directory not found: {}", path))
    }

    pub async fn exists(&self, path: &str) -> bool {
        let files = self.files.read().await;
        let dirs = self.directories.read().await;
        files.contains_key(path) || dirs.contains_key(path)
    }
}
```

### Command Execution with Custom FS

```rust
use std::process::Stdio;
use tokio::process::Command;

pub struct Sandbox {
    fs: InMemoryFs,
    cwd: String,
    env: HashMap<String, String>,
}

impl Sandbox {
    pub fn new(cwd: &str, initial_files: HashMap<String, String>) -> Result<Self> {
        let fs = InMemoryFs::new();

        // Write initial files
        for (path, content) in initial_files {
            fs.write(&path, &content).await?;
        }

        Ok(Self {
            fs,
            cwd: cwd.to_string(),
            env: std::env::vars().collect(),
        })
    }

    pub async fn execute(&self, command: &str) -> Result<CommandOutput> {
        // Parse command and handle special cases
        let parts = shell_words::split(command)?;
        let cmd = parts.first().map(|s| s.as_str()).unwrap_or("");

        match cmd {
            "cat" => self.handle_cat(&parts[1..]).await,
            "ls" => self.handle_ls(&parts[1..]).await,
            "echo" => self.handle_echo(&parts[1..]).await,
            "pwd" => Ok(CommandOutput {
                stdout: format!("{}\n", self.cwd),
                stderr: String::new(),
                exit_code: 0,
            }),
            // For other commands, use actual shell execution
            _ => self.execute_shell(command).await,
        }
    }

    async fn handle_cat(&self, args: &[String]) -> Result<CommandOutput> {
        let mut output = String::new();
        let mut errors = String::new();

        for arg in args {
            let path = if arg.starts_with('/') {
                arg.clone()
            } else {
                format!("{}/{}", self.cwd, arg)
            };

            match self.fs.read(&path).await {
                Ok(content) => {
                    output.push_str(&content);
                    output.push('\n');
                }
                Err(e) => {
                    errors.push_str(&format!("cat: {}: {}\n", arg, e));
                }
            }
        }

        Ok(CommandOutput {
            stdout: output,
            stderr: errors,
            exit_code: if errors.is_empty() { 0 } else { 1 },
        })
    }

    async fn execute_shell(&self, command: &str) -> Result<CommandOutput> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.cwd)
            .envs(&self.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(1),
        })
    }
}
```

### Container-Based Sandbox (Docker)

```rust
use bollard::{Docker, container::*};
use tokio::io::AsyncReadExt;

pub struct ContainerSandbox {
    docker: Docker,
    container_id: String,
    image: String,
}

impl ContainerSandbox {
    pub async fn create(image: &str) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;

        // Pull image if not present
        docker.create_image(image, None, None).await.ok();

        // Create container
        let config = Config {
            image: Some(image.to_string()),
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            open_stdin: Some(true),
            tty: Some(true),
            host_config: Some(HostConfig {
                // Resource limits
                memory: Some(512 * 1024 * 1024),  // 512MB
                cpu_quota: Some(50000),  // 50% CPU
                ..Default::default()
            }),
            ..Default::default()
        };

        let container = docker.create_container::<&str, &str>(None, config).await?;

        // Start container
        docker.start_container(&container.id).await?;

        Ok(Self {
            docker,
            container_id: container.id,
            image: image.to_string(),
        })
    }

    pub async fn execute(&self, command: &str) -> Result<CommandOutput> {
        // Create exec instance
        let config = ExecCreateContainerOptions {
            cmd: Some(vec!["sh", "-c", command]),
            attach_stdout: true,
            attach_stderr: true,
            ..Default::default()
        };

        let exec = self.docker.create_exec(&self.container_id, config).await?;

        // Start exec and capture output
        let mut output = self.docker.start_exec(&exec.id, None).await?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        // Parse Docker exec output format
        // First 8 bytes are header: stream type (1 byte) + length (4 bytes)
        // ... (parsing logic)

        Ok(CommandOutput {
            stdout,
            stderr,
            exit_code: 0,  // Would need to inspect exec inspect for actual exit code
        })
    }

    pub async fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        // Create tar archive
        let mut tar = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar);
            let mut header = tar::Header::new_gnu();
            header.set_path(path)?;
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, content)?;
            builder.finish()?;
        }

        // Copy to container
        self.docker
            .copy_to_container(&self.container_id, path, &tar, None)
            .await?;

        Ok(())
    }

    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        let mut archive = self.docker.copy_from_container(&self.container_id, path).await?;

        // Extract from tar
        let mut tar = tar::Archive::new(&mut archive);
        for entry in tar.entries()? {
            let mut entry = entry?;
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents)?;
            return Ok(contents);
        }

        Err(anyhow!("File not found: {}", path))
    }

    pub async fn stop(&self) -> Result<()> {
        self.docker.stop_container(&self.container_id, None).await?;
        self.docker.remove_container(&self.container_id, None).await?;
        Ok(())
    }
}
```

---

## 4. Workflow Engine

### Workflow Trait and Context

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait]
pub trait Workflow: Send + Sync {
    type Input: serde::de::DeserializeOwned + Send;
    type Output: serde::Serialize + Send;

    async fn run(&self, ctx: WorkflowContext, input: Self::Input)
        -> Result<Self::Output>;
}

pub struct WorkflowContext {
    pub workflow_id: String,
    pub checkpoint_store: Arc<dyn CheckpointStore>,
    pub state: Arc<Mutex<WorkflowState>>,
}

impl WorkflowContext {
    pub async fn checkpoint(&self, state: WorkflowState) -> Result<()> {
        self.checkpoint_store
            .save(&self.workflow_id, &state)
            .await?;
        *self.state.lock().await = state;
        Ok(())
    }

    pub async fn wait_for_callback<T: serde::de::DeserializeOwned>(
        &self,
        callback_id: String,
    ) -> Result<T> {
        // Save state and yield
        self.checkpoint(WorkflowState::WaitingForCallback {
            callback_id: callback_id.clone(),
        }).await?;

        // Return error to signal yield - engine will resume on callback
        Err(Error::YieldedForCallback(callback_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowState {
    Running { current_step: String },
    WaitingForCallback { callback_id: String },
    Completed { output: serde_json::Value },
    Failed { error: String },
}
```

### Step Abstraction

```rust
#[async_trait]
pub trait Step: Send + Sync {
    type Input: serde::de::DeserializeOwned + Send;
    type Output: serde::Serialize + Send;

    async fn execute(&self, input: Self::Input) -> Result<Self::Output>;

    fn name(&self) -> &'static str;
}

// Workflow step wrapper
pub struct StepExecutor<S: Step> {
    step: S,
}

impl<S: Step> StepExecutor<S> {
    pub fn new(step: S) -> Self {
        Self { step }
    }

    pub async fn execute_and_checkpoint(
        &self,
        ctx: &mut WorkflowContext,
        input: S::Input,
    ) -> Result<S::Output> {
        // Update state
        ctx.checkpoint(WorkflowState::Running {
            current_step: self.step.name().to_string(),
        }).await?;

        // Execute step
        let output = self.step.execute(input).await?;

        // Checkpoint output
        ctx.checkpoint(WorkflowState::Running {
            current_step: format!("{} (completed)", self.step.name()),
        }).await?;

        Ok(output)
    }
}
```

### Example: Lead Qualification Workflow

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct FormSchema {
    name: String,
    email: String,
    company: String,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Qualification {
    category: String,
    reason: String,
}

struct InboundLeadWorkflow {
    research_agent: Arc<ResearchAgent>,
    llm: Arc<dyn LanguageModel>,
}

#[async_trait]
impl Workflow for InboundLeadWorkflow {
    type Input = FormSchema;
    type Output = Qualification;

    async fn run(&self, mut ctx: WorkflowContext, input: FormSchema) -> Result<Qualification> {
        // Step 1: Research
        let research = self.step_research(&mut ctx, &input).await?;

        // Step 2: Qualify
        let qualification = self.step_qualify(&mut ctx, &input, &research).await?;

        // Step 3: Conditional - write email if qualified
        if qualification.category == "QUALIFIED" || qualification.category == "FOLLOW_UP" {
            let email = self.step_write_email(&mut ctx, &research, &qualification).await?;

            // Step 4: Human feedback (yields for callback)
            let callback_id = format!("approval-{}", ctx.workflow_id);
            let decision: ApprovalDecision = ctx.wait_for_callback(callback_id).await?;

            if decision.approved {
                self.step_send_email(&email).await?;
            }
        }

        Ok(qualification)
    }
}

impl InboundLeadWorkflow {
    async fn step_research(&self, ctx: &mut WorkflowContext, input: &FormSchema) -> Result<String> {
        ctx.checkpoint(WorkflowState::Running {
            current_step: "research".to_string(),
        }).await?;

        let result = self.research_agent
            .generate(&format!("Research this lead: {:?}", input))
            .await?;

        Ok(result)
    }

    async fn step_qualify(
        &self,
        ctx: &mut WorkflowContext,
        input: &FormSchema,
        research: &str,
    ) -> Result<Qualification> {
        ctx.checkpoint(WorkflowState::Running {
            current_step: "qualify".to_string(),
        }).await?;

        // Use generate_object for structured output
        // ... (implementation)

        Ok(qualification)
    }

    // ... other steps
}
```

### Workflow Engine with Persistence

```rust
use sqlx::{PgPool, FromRow};
use tokio::sync::mpsc;

pub struct WorkflowEngine {
    pool: PgPool,
    checkpoint_store: Arc<dyn CheckpointStore>,
    event_tx: mpsc::Sender<WorkflowEvent>,
}

impl WorkflowEngine {
    pub fn new(pool: PgPool) -> Self {
        let checkpoint_store = Arc::new(DatabaseCheckpointStore::new(pool.clone()));

        let (event_tx, event_rx) = mpsc::channel(100);

        // Spawn event processor
        tokio::spawn({
            let engine = Self {
                pool: pool.clone(),
                checkpoint_store: checkpoint_store.clone(),
                event_tx: event_tx.clone(),
            };
            async move {
                engine.process_events(event_rx).await;
            }
        });

        Self {
            pool,
            checkpoint_store,
            event_tx,
        }
    }

    pub async fn start<W: Workflow + 'static>(&self, workflow: W, input: W::Input) -> Result<String> {
        let workflow_id = uuid::Uuid::new_v4().to_string();

        // Save initial state
        sqlx::query(
            "INSERT INTO workflows (id, type, status, input, created_at)
             VALUES ($1, $2, 'running', $3, NOW())"
        )
        .bind(&workflow_id)
        .bind(std::any::type_name::<W>())
        .bind(serde_json::to_value(&input)?)
        .execute(&self.pool)
        .await?;

        // Spawn workflow execution
        tokio::spawn({
            let ctx = WorkflowContext {
                workflow_id: workflow_id.clone(),
                checkpoint_store: self.checkpoint_store.clone(),
                state: Arc::new(Mutex::new(WorkflowState::Running {
                    current_step: "init".to_string(),
                })),
            };
            async move {
                match workflow.run(ctx, input).await {
                    Ok(output) => {
                        // Mark as completed
                        sqlx::query(
                            "UPDATE workflows SET status = 'completed', output = $1 WHERE id = $2"
                        )
                        .bind(serde_json::to_value(&output).unwrap())
                        .bind(&workflow_id)
                        .execute(&self.pool)
                        .await
                        .ok();
                    }
                    Err(e) => {
                        // Mark as failed (unless yielded for callback)
                        if !matches!(e, Error::YieldedForCallback(_)) {
                            sqlx::query(
                                "UPDATE workflows SET status = 'failed', error = $1 WHERE id = $2"
                            )
                            .bind(&e.to_string())
                            .bind(&workflow_id)
                            .execute(&self.pool)
                            .await
                            .ok();
                        }
                    }
                }
            }
        });

        Ok(workflow_id)
    }

    pub async fn resume(&self, workflow_id: &str, callback_data: serde_json::Value) -> Result<()> {
        // Load workflow state
        let state = self.checkpoint_store.load(workflow_id).await?;

        if let WorkflowState::WaitingForCallback { callback_id } = state {
            // Resume workflow with callback data
            // ... (reconstruct and continue execution)
        }

        Ok(())
    }
}
```

---

## 5. RAG Implementation

### Embedding Client

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

pub struct EmbeddingClient {
    client: Client,
    api_key: String,
}

impl EmbeddingClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&EmbeddingRequest {
                model: "text-embedding-ada-002".to_string(),
                input: vec![text.to_string()],
            })
            .send()
            .await?
            .json::<EmbeddingResponse>()
            .await?;

        Ok(response.data[0].embedding.clone())
    }

    pub async fn embed_many(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&EmbeddingRequest {
                model: "text-embedding-ada-002".to_string(),
                input: texts,
            })
            .send()
            .await?
            .json::<EmbeddingResponse>()
            .await?;

        let mut embeddings = vec![Vec::new(); response.data.len()];
        for item in response.data {
            embeddings[item.index] = item.embedding;
        }

        Ok(embeddings)
    }
}
```

### Vector Search with pgvector

```rust
use sqlx::{PgPool, FromRow};

#[derive(FromRow)]
struct EmbeddingRow {
    id: String,
    content: String,
    similarity: f32,
}

pub struct VectorStore {
    pool: PgPool,
}

impl VectorStore {
    pub async fn insert(&self, id: &str, content: &str, embedding: &[f32]) -> Result<()> {
        // Convert Vec<f32> to pgvector format
        let embedding_str = format!(
            "[{}]",
            embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")
        );

        sqlx::query(
            "INSERT INTO embeddings (id, content, embedding)
             VALUES ($1, $2, $3::vector)"
        )
        .bind(id)
        .bind(content)
        .bind(&embedding_str)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<EmbeddingRow>> {
        let embedding_str = format!(
            "[{}]",
            query_embedding.iter().map(|f| f.to_string()).collect::<Vec<_>>().join(",")
        );

        let results = sqlx::query_as::<_, EmbeddingRow>(
            r#"
            SELECT id, content, 1 - (embedding <=> $1::vector) AS similarity
            FROM embeddings
            WHERE 1 - (embedding <=> $1::vector) > $2
            ORDER BY similarity DESC
            LIMIT $3
            "#
        )
        .bind(&embedding_str)
        .bind(threshold)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }
}
```

### Text Chunking

```rust
/// Split text into chunks for embedding
pub fn chunk_text(text: &str, options: ChunkingOptions) -> Vec<String> {
    match options.strategy {
        ChunkingStrategy::BySentence => chunk_by_sentence(text),
        ChunkingStrategy::ByToken { max_tokens } => chunk_by_tokens(text, max_tokens),
        ChunkingStrategy::ByOverlap { chunk_size, overlap } => {
            chunk_with_overlap(text, chunk_size, overlap)
        }
    }
}

fn chunk_by_sentence(text: &str) -> Vec<String> {
    // Handle common abbreviations
    let abbreviations = ["Mr.", "Mrs.", "Dr.", "Ph.D.", "M.D.", "e.g.", "i.e.", "vs.", "etc."];

    let mut chunks = Vec::new();
    let mut current = String::new();

    for sentence in text.split('.') {
        let sentence = sentence.trim();
        if sentence.is_empty() {
            continue;
        }

        // Check if this is an abbreviation
        let is_abbrev = abbreviations.iter().any(|abbr| {
            current.ends_with(abbr) || sentence.starts_with(abbr)
        });

        if is_abbrev {
            current.push_str(sentence);
            current.push('.');
        } else {
            if !current.is_empty() {
                current.push('.');
                chunks.push(current.clone());
                current.clear();
            }
            current.push_str(sentence);
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn chunk_by_tokens(text: &str, max_tokens: usize) -> Vec<String> {
    // Use tiktoken for accurate tokenization
    let bpe = tiktoken_rs::get_bpe_from_model("gpt-3.5-turbo").unwrap();
    let tokens = bpe.encode_with_special_tokens(text);

    tokens
        .chunks(max_tokens)
        .map(|chunk| {
            bpe.decode(chunk).unwrap_or_default()
        })
        .collect()
}

fn chunk_with_overlap(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();

    for i in (0..words.len()).step_by(chunk_size - overlap) {
        let end = (i + chunk_size).min(words.len());
        chunks.push(words[i..end].join(" "));
    }

    chunks
}

#[derive(Debug, Clone)]
pub struct ChunkingOptions {
    pub strategy: ChunkingStrategy,
}

#[derive(Debug, Clone)]
pub enum ChunkingStrategy {
    BySentence,
    ByToken { max_tokens: usize },
    ByOverlap { chunk_size: usize, overlap: usize },
}
```

### Cosine Similarity

```rust
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}
```

---

## 6. Reasoning Models

### Reasoning Extraction Middleware

```rust
use regex::Regex;

/// Extract reasoning from models that use <think> tags
pub fn extract_reasoning(content: &str) -> (Option<String>, String) {
    let think_pattern = Regex::new(r"(?s)<think>(.*?)</think>(.*)").unwrap();

    if let Some(caps) = think_pattern.captures(content) {
        let reasoning = caps[1].trim().to_string();
        let answer = caps[2].trim().to_string();
        (Some(reasoning), answer)
    } else {
        (None, content.to_string())
    }
}

/// Stream handler that separates reasoning from answer
pub struct ReasoningStreamHandler {
    buffer: String,
    reasoning_emitted: bool,
}

impl ReasoningStreamHandler {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            reasoning_emitted: false,
        }
    }

    pub fn process_chunk(&mut self, chunk: &str) -> Option<StreamEvent> {
        self.buffer.push_str(chunk);

        // Check for complete <think> tag
        if !self.reasoning_emitted {
            if let Some(start) = self.buffer.find("<think>") {
                if let Some(end) = self.buffer.find("</think>") {
                    let reasoning = self.buffer[start + 7..end].to_string();
                    let remaining = self.buffer[end + 9..].to_string();
                    self.buffer = remaining;
                    self.reasoning_emitted = true;

                    return Some(StreamEvent::Reasoning(reasoning));
                }
            }
        }

        // Emit any complete content
        if !self.buffer.is_empty() {
            let content = std::mem::take(&mut self.buffer);
            Some(StreamEvent::Content(content))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum StreamEvent {
    Reasoning(String),
    Content(String),
    Done,
}
```

### Anthropic Thinking Support

```rust
#[derive(Serialize)]
struct AnthropicThinkingConfig {
    #[serde(rename = "type")]
    thinking_type: String,
    budget_tokens: usize,
}

impl AnthropicProvider {
    pub fn with_thinking_config(
        model: &str,
        thinking_enabled: bool,
        budget_tokens: usize,
    ) -> AnthropicRequest {
        AnthropicRequest {
            model: model.to_string(),
            max_tokens: 4096,
            thinking: if thinking_enabled {
                Some(AnthropicThinkingConfig {
                    thinking_type: "enabled".to_string(),
                    budget_tokens,
                })
            } else {
                Some(AnthropicThinkingConfig {
                    thinking_type: "disabled".to_string(),
                    budget_tokens,
                })
            },
            ..Default::default()
        }
    }
}
```

---

## 7. Production Considerations

### Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AISError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Rate limited: retry after {0} seconds")]
    RateLimited(u64),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timeout: {0}")]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

// Retry with exponential backoff
pub async fn with_retry<T, F, Fut>(
    f: F,
    max_retries: u32,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempts = 0;
    let mut delay = 1000u64;  // Start with 1 second

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(AISError::RateLimited(retry_after)) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(AISError::RateLimited(retry_after).into());
                }
                tokio::time::sleep(std::time::Duration::from_millis(retry_after * 1000)).await;
            }
            Err(e) if is_retryable(&e) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(e);
                }
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                delay *= 2;  // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }
}

fn is_retryable(error: &anyhow::Error) -> bool {
    error.downcast_ref::<AISError>()
        .map_or(false, |e| matches!(e, AISError::Provider(_) | AISError::Timeout(_)))
}
```

### Observability

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(self, messages), fields(model = %options.model))]
async fn stream_text_with_tracing(
    &self,
    messages: Vec<Message>,
    options: GenerationOptions,
) -> Result<BoxStream<'_, Result<Token>>> {
    info!("Starting text generation");

    let start = std::time::Instant::now();
    let result = self.stream_text(messages, options).await;

    match &result {
        Ok(_) => {
            info!(duration_ms = %start.elapsed().as_millis(), "Text generation completed");
        }
        Err(e) => {
            error!(error = %e, "Text generation failed");
        }
    }

    result
}
```

### Configuration

```rust
use config::{Config, ConfigError, File};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AIConfig {
    pub providers: ProviderConfig,
    pub defaults: GenerationDefaults,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProviderConfig {
    pub anthropic: AnthropicConfig,
    pub openai: OpenAIConfig,
    pub groq: GroqConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub base_url: Option<String>,
    pub default_model: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GenerationDefaults {
    pub temperature: f32,
    pub max_tokens: usize,
    pub thinking_budget: usize,
}

impl AIConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config/ai").required(false))
            .add_source(config::Environment::with_prefix("AI").separator("__"))
            .build()?;

        config.try_deserialize()
    }
}
```

---

## Cargo Dependencies

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# HTTP client
reqwest = { version = "0.11", features = ["json", "stream"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Streaming
futures = "0.3"
tokio-stream = "0.1"

# Error handling
anyhow = "1"
thiserror = "1"

# JSON Schema
schemars = "0.8"

# CDP/Browser
tokio-tungstenite = "0.20"

# Container sandbox (optional)
bollard = "0.14"
tar = "0.4"

# Database
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio"] }

# Tokenization
tiktoken-rs = "0.5"

# Configuration
config = "0.13"

# Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
uuid = { version = "1", features = ["v4"] }
regex = "1"
shell-words = "1"
```

---

## Summary

Reproducing Vercel Labs AI tools in Rust requires:

1. **Provider Abstraction** - Trait-based LLM interface with streaming support
2. **Structured Output** - JSON Schema generation from Rust types
3. **Browser Automation** - CDP integration with ref-based element selection
4. **Sandbox Execution** - In-memory or container-based isolation
5. **Workflow Engine** - Durable execution with checkpointing
6. **Vector Search** - pgvector integration for RAG
7. **Reasoning Support** - <think> tag extraction and thinking budget configuration

The Rust ecosystem has all the necessary primitives - the key is building the right abstractions to match the ergonomics of the AI SDK while leveraging Rust's performance and safety guarantees.
