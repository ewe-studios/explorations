# qwen-code -- Rust Revision Plan

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.QwenCode/qwen-code/`

**Target:** `qwen-code-rs` -- Production-grade Rust implementation

**Explored At:** 2026-04-11

---

## Table of Contents

1. [Overview](#overview)
2. [Workspace Structure](#workspace-structure)
3. [Key Crates & Dependencies](#key-crates--dependencies)
4. [Core Subsystems](#core-subsystems)
5. [Testing Strategy](#testing-strategy)
6. [Deployment & Operations](#deployment--operations)
7. [Migration Guide](#migration-guide)
8. [Performance Expectations](#performance-expectations)

---

## Overview

This document provides a complete plan for translating qwen-code from TypeScript/Node.js to Rust. The goal is to produce a production-grade implementation that:

- Matches or exceeds all functionality of the TypeScript version
- Provides better performance (lower latency, less memory)
- Offers stronger type safety and compile-time guarantees
- Delivers a single binary with no runtime dependencies

### Design Principles

1. **Correctness First**: Use Rust's type system to prevent errors at compile time
2. **Async by Default**: Use tokio for async runtime with structured concurrency
3. **Composable Architecture**: Traits for abstractions, concrete types for implementations
4. **Observability Built-in**: Tracing spans, metrics, structured logging from day one
5. **Progressive Enhancement**: Start with core CLI, add channels and extensions later

---

## Workspace Structure

```
qwen-code-rs/
├── Cargo.toml                  # Workspace root
├── Cargo.lock                  # Dependency lock file
├── README.md                   # Project overview
├── LICENSE                     # Apache-2.0
├── .github/
│   └── workflows/
│       ├── ci.yml              # CI pipeline
│       ├── release.yml         # Release automation
│       └── benchmark.yml       # Performance benchmarks
│
├── crates/
│   ├── qwen-cli/               # CLI entry point (interactive/headless)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs         # Entry point
│   │       ├── app.rs          # Application state
│   │       ├── ui/             # Terminal UI (Ratatui-based)
│   │       │   ├── mod.rs
│   │       │   ├── components.rs
│   │       │   └── theme.rs
│   │       ├── commands/       # Slash commands
│   │       │   ├── mod.rs
│   │       │   ├── help.rs
│   │       │   ├── auth.rs
│   │       │   └── model.rs
│   │       └── repl.rs         # Read-eval-print loop
│   │
│   ├── qwen-core/              # Core engine (main loop, turn management)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs       # GeminiClient (main orchestration)
│   │       ├── turn.rs         # Turn event stream
│   │       ├── message.rs      # Message types
│   │       └── error.rs        # Error types
│   │
│   ├── qwen-config/            # Configuration system (DI container)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs       # Config builder
│   │       ├── settings.rs     # Settings loading
│   │       ├── storage.rs      # Storage paths
│   │       └── builder.rs      # Typed builder
│   │
│   ├── qwen-provider/          # LLM provider abstraction
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── trait.rs        # ContentGenerator trait
│   │       ├── openai.rs       # OpenAI provider
│   │       ├── anthropic.rs    # Anthropic provider
│   │       ├── gemini.rs       # Google Gemini provider
│   │       ├── qwen.rs         # Qwen OAuth provider
│   │       └── logging.rs      # Logging wrapper
│   │
│   ├── qwen-tools/             # Tool registry and implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs     # ToolRegistry
│   │       ├── trait.rs        # Tool trait
│   │       ├── shell.rs        # Shell execution
│   │       ├── edit.rs         # File editing
│   │       ├── read.rs         # File reading
│   │       ├── write.rs        # File writing
│   │       ├── glob.rs         # Glob pattern matching
│   │       ├── grep.rs         # Grep/ripgrep
│   │       ├── web.rs          # Web fetch/search
│   │       ├── agent.rs        # SubAgent tool
│   │       ├── skill.rs        # Skill execution
│   │       ├── memory.rs       # Memory tool
│   │       ├── todo.rs         # TodoWrite tool
│   │       └── mcp.rs          # MCP tools
│   │
│   ├── qwen-services/          # Services layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── session.rs      # Session management
│   │       ├── compression.rs  # Chat compression
│   │       ├── recording.rs    # Chat recording
│   │       ├── git.rs          # Git operations
│   │       ├── fs.rs           # File system service
│   │       ├── shell.rs        # Shell execution service
│   │       ├── loop_detect.rs  # Loop detection
│   │       └── worktree.rs     # Git worktree
│   │
│   ├── qwen-auth/              # Authentication (OAuth, API keys)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── oauth.rs        # OAuth 2.0 Device Flow
│   │       ├── pkce.rs         # PKCE generation
│   │       ├── tokens.rs       # Token storage
│   │       └── api_key.rs      # API key management
│   │
│   ├── qwen-permissions/       # Permission system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs      # PermissionManager
│   │       ├── rules.rs        # Rule parser
│   │       └── shell_semantics.rs  # Shell analysis
│   │
│   ├── qwen-hooks/             # Hook system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs     # HookRegistry
│   │       ├── planner.rs      # HookPlanner
│   │       ├── runner.rs       # HookRunner
│   │       └── aggregator.rs   # HookAggregator
│   │
│   ├── qwen-skills/            # Skill system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs      # SkillManager
│   │       ├── bundled/        # Bundled skills
│   │       └── loader.rs       # Dynamic skill loading
│   │
│   ├── qwen-subagents/         # SubAgent system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs      # SubAgentManager
│   │       ├── builtin.rs      # Built-in agents
│   │       └── validation.rs   # Output validation
│   │
│   ├── qwen-mcp/               # MCP (Model Context Protocol)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs       # MCP client
│   │       ├── oauth.rs        # MCP OAuth
│   │       └── tokens.rs       # Token storage
│   │
│   ├── qwen-extensions/        # Extension system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── manager.rs      # ExtensionManager
│   │       ├── marketplace.rs  # npm registry
│   │       └── converters.rs   # Claude/Gemini converters
│   │
│   ├── qwen-ide/               # IDE integration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── detector.rs     # IDE detector
│   │       ├── client.rs       # IDE client
│   │       ├── context.rs      # IDE context store
│   │       └── vscode.rs       # VS Code extension
│   │
│   ├── qwen-channels/          # Channel adapters (Telegram, WeChat, etc.)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── base.rs         # ChannelBase trait
│   │       ├── acp.rs          # AcpBridge
│   │       ├── session.rs      # SessionRouter
│   │       ├── pairing.rs      # PairingStore
│   │       └── telegram/       # Telegram adapter
│   │
│   ├── qwen-network/           # HTTP client, streaming
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── http.rs         # HTTP client
│   │       ├── streaming.rs    # HTTP streaming
│   │       ├── tls.rs          # TLS configuration
│   │       └── proxy.rs        # Proxy support
│   │
│   ├── qwen-stream-parser/     # JSON stream parser
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── json_stream.rs  # JSON stream parser
│   │       ├── sse.rs          # SSE parser
│   │       └── tool_call.rs    # Tool call parsing
│   │
│   ├── qwen-retry/             # Retry logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── backoff.rs      # Exponential backoff
│   │       ├── retry_after.rs  # Retry-After header
│   │       └── rate_limit.rs   # Rate limiting
│   │
│   ├── qwen-telemetry/         # OpenTelemetry
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── tracing.rs      # Tracing setup
│   │       ├── metrics.rs      # Metrics
│   │       └── logging.rs      # Structured logging
│   │
│   └── qwen-types/             # Shared types
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── content.rs      # Content types
│           ├── message.rs      # Message types
│           ├── tool.rs         # Tool types
│           └── response.rs     # Response types
```

---

## Key Crates & Dependencies

| Crate | Dependencies | Purpose |
|-------|--------------|---------|
| `qwen-cli` | `clap`, `ratatui`, `crossterm`, `tokio`, `serde_json` | CLI entry point, terminal UI |
| `qwen-core` | `tokio`, `tokio-stream`, `async-trait`, `thiserror` | Main orchestration loop |
| `qwen-config` | `serde`, `serde_json`, `toml`, `dirs`, `buildstructor` | Configuration builder |
| `qwen-provider` | `async-trait`, `serde`, `reqwest` | LLM provider trait + implementations |
| `qwen-tools` | `tokio`, `serde_json`, `globe`, `regex` | Tool registry and implementations |
| `qwen-services` | `tokio`, `simple-git`, `notify`, `chrono` | Supporting services |
| `qwen-auth` | `oauth2`, `reqwest`, `serde_json`, `keyring` | OAuth 2.0, API keys |
| `qwen-permissions` | `serde`, `regex`, `thiserror` | Permission rules and shell analysis |
| `qwen-hooks` | `tokio`, `serde_json`, `async-trait` | Hook system |
| `qwen-skills` | `tokio`, `serde`, `include_dir` | Skill loading and execution |
| `qwen-subagents` | `tokio`, `serde_json`, `jsonschema` | SubAgent management |
| `qwen-mcp` | `tokio`, `serde_json`, `oauth2` | MCP client |
| `qwen-extensions` | `tokio`, `serde`, `semver` | Extension system |
| `qwen-ide` | `tokio`, `serde_json`, `uds_windows` | IDE integration |
| `qwen-channels` | `tokio`, `serde_json`, `reqwest` | Channel adapters |
| `qwen-network` | `reqwest`, `hyper-util`, `tokio`, `native-tls` | HTTP client |
| `qwen-stream-parser` | `nom`, `tokio-stream`, `serde_json` | Stream parsing |
| `qwen-retry` | `tokio`, `rand`, `futures` | Retry logic |
| `qwen-telemetry` | `opentelemetry`, `tracing`, `tracing-opentelemetry` | Observability |
| `qwen-types` | `serde`, `serde_json`, `chrono` | Shared types |

---

## Core Subsystems

### 1. Config as Typed Builder

```rust
use buildstructor::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub model_providers: Vec<ModelProvider>,
    pub default_model: String,
    pub auth_type: Option<AuthType>,
    pub proxy: Option<String>,
    pub telemetry_enabled: bool,
    pub telemetry_endpoint: Option<String>,
}

#[derive(Debug, Clone, Builder)]
pub struct Config {
    settings: Settings,
    content_generator: Box<dyn ContentGenerator>,
    tool_registry: ToolRegistry,
    permission_manager: PermissionManager,
    hook_system: HookSystem,
    skill_manager: SkillManager,
    subagent_manager: SubAgentManager,
    file_system: Arc<dyn FileSystem>,
    git_service: GitService,
    storage: Storage,
    telemetry: Option<Telemetry>,
}

impl Config {
    pub async fn builder() -> Result<ConfigBuilder, ConfigError> {
        // Load settings from:
        // 1. ~/.qwen/settings.json
        // 2. .qwen/settings.json
        // 3. Environment variables
        // 4. CLI flags
        let settings = Settings::load()?;
        
        Ok(ConfigBuilder {
            settings,
            content_generator: None,
            tool_registry: None,
            // ...
        })
    }
    
    pub async fn build(mut builder: ConfigBuilder) -> Result<Self, ConfigError> {
        // 1. Create content generator based on auth type
        builder.content_generator = Some(
            ContentGeneratorFactory::create(&builder.settings).await?
        );
        
        // 2. Create tool registry
        builder.tool_registry = Some(ToolRegistry::new());
        
        // 3. Register all tools
        builder.tool_registry.as_mut().unwrap().register_all();
        
        // 4. Create services
        builder.file_system = Some(Arc::new(TokioFileSystem));
        builder.git_service = GitService::new()?;
        builder.storage = Storage::new(&builder.settings)?;
        
        // 5. Initialize telemetry
        if builder.settings.telemetry_enabled {
            builder.telemetry = Some(Telemetry::init(&builder.settings).await?);
        }
        
        Ok(Config {
            settings: builder.settings,
            content_generator: builder.content_generator.unwrap(),
            tool_registry: builder.tool_registry.unwrap(),
            permission_manager: PermissionManager::load()?,
            hook_system: HookSystem::new(),
            skill_manager: SkillManager::load()?,
            subagent_manager: SubAgentManager::load()?,
            file_system: builder.file_system.unwrap(),
            git_service: builder.git_service,
            storage: builder.storage,
            telemetry: builder.telemetry,
        })
    }
}
```

### 2. ContentGenerator Trait

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait ContentGenerator: Send + Sync {
    async fn generate_content(
        &self,
        request: GenerateContentRequest,
        prompt_id: &str,
    ) -> Result<GenerateContentResponse, GenerateError>;
    
    fn generate_content_stream(
        &self,
        request: GenerateContentRequest,
        prompt_id: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<GenerateContentResponse, GenerateError>> + Send>>;
    
    async fn count_tokens(
        &self,
        request: CountTokensRequest,
    ) -> Result<CountTokensResponse, GenerateError>;
    
    async fn embed_content(
        &self,
        request: EmbedContentRequest,
    ) -> Result<EmbedContentResponse, GenerateError>;
}

pub struct GenerateContentRequest {
    pub messages: Vec<Content>,
    pub tools: Vec<FunctionDeclaration>,
    pub system_instruction: Option<String>,
    pub generation_config: GenerationConfig,
}

pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: Option<UsageMetadata>,
}

#[async_trait]
pub trait ContentGeneratorFactory {
    async fn create(settings: &Settings) -> Result<Box<dyn ContentGenerator>, GenerateError>;
}
```

### 3. Tool Trait

```rust
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn schema(&self) -> FunctionDeclaration;
    fn description(&self) -> &str;
    
    async fn execute(
        &self,
        params: Value,
        cancel: CancellationToken,
    ) -> Result<ToolResult, ToolError>;
    
    /// Optional: Check if tool requires user confirmation
    fn requires_confirmation(&self, params: &Value) -> bool {
        false
    }
}

pub struct ToolResult {
    pub output: String,
    pub display: Option<ToolResultDisplay>,
    pub error: bool,
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }
    
    pub fn register_all(&mut self) {
        self.register(Box::new(ShellTool::new()));
        self.register(Box::new(EditTool::new()));
        self.register(Box::new(ReadFileTool::new()));
        self.register(Box::new(WriteFileTool::new()));
        self.register(Box::new(GlobTool::new()));
        self.register(Box::new(GrepTool::new()));
        self.register(Box::new(RipGrepTool::new()));
        self.register(Box::new(WebFetchTool::new()));
        self.register(Box::new(WebSearchTool::new()));
        self.register(Box::new(AgentTool::new()));
        self.register(Box::new(SkillTool::new()));
        self.register(Box::new(MemoryTool::new()));
        self.register(Box::new(TodoWriteTool::new()));
        self.register(Box::new(LspTool::new()));
        self.register(Box::new(McpTool::new()));
        self.register(Box::new(CronCreateTool::new()));
        self.register(Box::new(CronListTool::new()));
        self.register(Box::new(CronDeleteTool::new()));
        self.register(Box::new(AskUserQuestionTool::new()));
        self.register(Box::new(ExitPlanModeTool::new()));
    }
}
```

### 4. HTTP Client with reqwest

```rust
use reqwest::{Client, Response, StatusCode};
use tokio::time::{timeout, Duration};
use std::collections::HashMap;

pub struct HttpClient {
    client: Client,
    proxy: Option<String>,
    timeout_ms: u64,
}

impl HttpClient {
    pub fn new(proxy: Option<String>, timeout_ms: u64) -> Result<Self, Error> {
        let mut builder = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .tcp_keepalive(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10);
        
        if let Some(proxy_url) = proxy {
            builder = builder.proxy(reqwest::Proxy::all(&proxy_url)?);
        }
        
        // Disable default undici-style timeouts, let SDK control
        builder = builder
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_millis(timeout_ms));
        
        Ok(Self {
            client: builder.build()?,
            proxy,
            timeout_ms,
        })
    }
    
    pub async fn post_stream(
        &self,
        url: &str,
        headers: HashMap<String, String>,
        body: serde_json::Value,
    ) -> Result<impl Stream<Item = Result<Bytes, Error>>, Error> {
        let mut request_builder = self.client.post(url);
        
        for (key, value) in headers {
            request_builder = request_builder.header(&key, &value);
        }
        
        let response = request_builder
            .json(&body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(ApiError::from_response(response).await);
        }
        
        Ok(response.bytes_stream()
            .map_err(|e| Error::Network(e.to_string())))
    }
    
    /// Respect Retry-After header
    pub fn get_retry_after_ms(&self, response: &Response) -> Option<u64> {
        response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|s| {
                // Parse as seconds or HTTP date
                if let Ok(secs) = s.parse::<u64>() {
                    Some(secs * 1000)
                } else {
                    // Parse HTTP date
                    chrono::DateTime::parse_from_rfc2822(s)
                        .ok()
                        .map(|dt| {
                            let now = chrono::Utc::now();
                            (dt.with_timezone(&now.timezone()) - now)
                                .num_milliseconds() as u64
                        })
                }
            })
    }
}
```

### 5. OAuth 2.0 Device Flow

```rust
use oauth2::basic::BasicDeviceAuthorizationResponse;
use oauth2::{DeviceAuthorizationUrl, ClientId, Scope};
use rand::RngCore;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

pub struct OAuthClient {
    client_id: ClientId,
    device_auth_url: DeviceAuthorizationUrl,
    token_url: Url,
    scopes: Vec<Scope>,
}

impl OAuthClient {
    pub fn generate_pkce_pair() -> (String, String) {
        // Generate 32-byte random verifier
        let mut verifier_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut verifier_bytes);
        let code_verifier = base64_url::encode(&verifier_bytes);
        
        // Generate SHA-256 challenge
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        let code_challenge = base64_url::encode(&hash);
        
        (code_verifier, code_challenge)
    }
    
    pub async fn request_device_code(
        &self,
        code_challenge: &str,
    ) -> Result<DeviceAuthorizationResponse, Error> {
        let response = self.client
            .post(self.device_auth_url.as_str())
            .form(&[
                ("client_id", &self.client_id),
                ("scope", "openid profile email model.completion"),
                ("code_challenge", code_challenge),
                ("code_challenge_method", "S256"),
            ])
            .send()
            .await?;
        
        Ok(response.json::<DeviceAuthorizationResponse>().await?)
    }
    
    pub async fn poll_device_token(
        &self,
        device_code: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse, PollError> {
        let response = self.client
            .post(self.token_url.as_str())
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", &self.client_id),
                ("device_code", device_code),
                ("code_verifier", code_verifier),
            ])
            .send()
            .await?;
        
        match response.status() {
            StatusCode::OK => Ok(response.json::<TokenResponse>().await?),
            StatusCode::BAD_REQUEST => {
                let error = response.json::<OAuthError>().await?;
                match error.error.as_str() {
                    "authorization_pending" => Err(PollError::Pending),
                    "slow_down" => Err(PollError::SlowDown),
                    _ => Err(PollError::Failed(error)),
                }
            }
            _ => Err(PollError::Http(response.status())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStorage {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: Option<String>,
    pub token_type: String,
    pub expiry_date: u64,  // Unix timestamp in milliseconds
    pub resource_url: String,
}

impl TokenStorage {
    pub fn load(path: &Path) -> Result<Self, TokenError> {
        // Load from ~/.qwen/oauth_creds.json
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }
    
    pub fn save(&self, path: &Path) -> Result<(), TokenError> {
        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("json.tmp");
        std::fs::write(&temp_path, serde_json::to_string_pretty(self)?)?;
        
        // Set permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&temp_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&temp_path, perms)?;
        }
        
        std::fs::rename(&temp_path, path)?;
        Ok(())
    }
    
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now >= self.expiry_date
    }
}
```

### 6. JSON Stream Parser

```rust
use futures::stream::Stream;
use serde_json::Value;
use std::collections::HashMap;

pub struct JsonStreamParser {
    buffers: HashMap<usize, String>,  // Per-index JSON accumulation
    depths: HashMap<usize, usize>,    // JSON nesting depth
    in_strings: HashMap<usize, bool>, // Inside string literal?
    escapes: HashMap<usize, bool>,    // Next char escaped?
}

impl JsonStreamParser {
    pub fn new() -> Self {
        Self {
            buffers: HashMap::new(),
            depths: HashMap::new(),
            in_strings: HashMap::new(),
            escapes: HashMap::new(),
        }
    }
    
    pub fn push_chunk(
        &mut self,
        index: usize,
        chunk: &str,
    ) -> ParseResult {
        let buffer = self.buffers.entry(index).or_insert_with(String::new);
        let depth = self.depths.entry(index).or_insert(0);
        let in_string = self.in_strings.entry(index).or_insert(false);
        let escape = self.escapes.entry(index).or_insert(false);
        
        buffer.push_str(chunk);
        
        // Track JSON structure
        for char in chunk.chars() {
            if !*in_string {
                match char {
                    '{' | '[' => *depth += 1,
                    '}' | ']' => *depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
            
            if char == '"' && !*escape {
                *in_string = !*in_string;
            }
            *escape = char == '\\' && !*escape;
        }
        
        // Parse if complete (depth == 0)
        if *depth == 0 && !buffer.trim().is_empty() {
            match serde_json::from_str::<Value>(buffer) {
                Ok(value) => {
                    let result = ParseResult {
                        complete: true,
                        value: Some(value),
                        repaired: false,
                    };
                    buffer.clear();
                    *depth = 0;
                    *in_string = false;
                    *escape = false;
                    result
                }
                Err(_) if *in_string => {
                    // Try repair: auto-close unclosed strings
                    let repaired = format!("{}\"", buffer);
                    match serde_json::from_str::<Value>(&repaired) {
                        Ok(value) => {
                            let result = ParseResult {
                                complete: true,
                                value: Some(value),
                                repaired: true,
                            };
                            buffer.clear();
                            *depth = 0;
                            *in_string = false;
                            *escape = false;
                            result
                        }
                        Err(_) => ParseResult {
                            complete: false,
                            value: None,
                            repaired: false,
                        },
                    }
                }
                Err(_) => ParseResult {
                    complete: false,
                    value: None,
                    repaired: false,
                },
            }
        } else {
            ParseResult {
                complete: false,
                value: None,
                repaired: false,
            }
        }
    }
}

pub struct ParseResult {
    pub complete: bool,
    pub value: Option<Value>,
    pub repaired: bool,
}
```

### 7. Retry Logic with Exponential Backoff

```rust
use tokio::time::{sleep, Duration};
use rand::Rng;

pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter_percent: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 7,
            initial_delay_ms: 1500,
            max_delay_ms: 30000,
            jitter_percent: 0.3,
        }
    }
}

pub async fn retry_with_backoff<F, T, E, Fut>(
    config: &RetryConfig,
    mut operation: F,
    should_retry: impl Fn(&E) -> bool,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut delay = config.initial_delay_ms;
    
    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt == config.max_attempts => return Err(e),
            Err(e) if !should_retry(&e) => return Err(e),
            Err(_) => {
                // Add jitter: delay ± jitter%
                let jitter = (delay as f64 * config.jitter_percent) 
                    * (rand::thread_rng().gen::<f64>() * 2.0 - 1.0);
                let delay_with_jitter = (delay as f64 + jitter).max(0.0) as u64;
                
                sleep(Duration::from_millis(delay_with_jitter)).await;
                delay = (delay * 2).min(config.max_delay_ms);
            }
        }
    }
    
    unreachable!()
}

/// Should we retry on this error?
pub fn default_should_retry(error: &dyn std::error::Error) -> bool {
    // Check for HTTP status codes
    if let Some(http_error) = error.downcast_ref::<reqwest::Error>() {
        if let Some(status) = http_error.status() {
            return status.as_u16() == 429 || (status.as_u16() >= 500 && status.as_u16() < 600);
        }
    }
    
    // Check for TLS errors, connection errors
    let error_str = error.to_string();
    error_str.contains("timeout")
        || error_str.contains("connection")
        || error_str.contains("tls")
}
```

---

## Testing Strategy

### Testing Philosophy

The TypeScript version uses Vitest for unit tests co-located with source files. The Rust version uses the standard approach:

1. **Unit tests** in each module using `#[cfg(test)]`
2. **Integration tests** in `tests/` directory
3. **Property-based tests** using `proptest` for parsers
4. **Mock LLM providers** for deterministic testing

### Test Structure

```
qwen-code-rs/
├── crates/
│   └── qwen-core/
│       ├── src/
│       │   ├── client.rs
│       │   ├── client_test.rs      # Test module
│       │   └── ...
│       └── tests/
│           ├── integration_test.rs  # Integration tests
│           └── fixtures/            # Test fixtures
├── tests/
│   ├── common/                      # Shared test utilities
│   │   ├── mod.rs
│   │   ├── mock_server.rs           # Mock HTTP server
│   │   └── temp_dir.rs              # Temp directory helper
│   ├── cli_test.rs                  # CLI integration tests
│   ├── tool_execution_test.rs       # Tool execution tests
│   ├── oauth_test.rs                # OAuth flow tests
│   └── streaming_test.rs            # Streaming parser tests
```

### Unit Testing Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_shell_tool_execute() {
        let tool = ShellTool::new();
        let params = serde_json::json!({
            "command": "echo hello",
            "description": "Test echo",
        });
        
        let cancel = CancellationToken::new();
        let result = tool.execute(params, cancel).await.unwrap();
        
        assert!(!result.error);
        assert!(result.output.contains("hello"));
    }
    
    #[tokio::test]
    async fn test_oauth_pkce_generation() {
        let (verifier, challenge) = OAuthClient::generate_pkce_pair();
        
        // Verify format
        assert!(verifier.len() > 43);
        assert!(challenge.len() > 43);
        
        // Verify deterministic: same verifier = same challenge
        let (_, challenge2) = OAuthClient::generate_pkce_pair();
        assert_ne!(challenge, challenge2);  // Random, should differ
    }
    
    #[test]
    fn test_json_stream_parser_complete() {
        let mut parser = JsonStreamParser::new();
        
        // Simulate chunks arriving
        parser.push_chunk(0, r#"{"choices":[{"delta":{"content":"Hello"}}"#);
        let result = parser.push_chunk(0, r#"]}"#);
        
        assert!(result.complete);
        assert!(result.value.is_some());
        assert!(!result.repaired);
    }
    
    #[test]
    fn test_json_stream_parser_repair() {
        let mut parser = JsonStreamParser::new();
        
        // Simulate incomplete JSON (unclosed string)
        parser.push_chunk(0, r#"{"content": "hello"#);
        let result = parser.push_chunk(0, r#" world}"#);
        
        // Parser should repair and complete
        assert!(result.complete);
        assert!(result.repaired);
    }
    
    #[tokio::test]
    async fn test_retry_with_backoff() {
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let config = RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 10,
            max_delay_ms: 100,
            jitter_percent: 0.1,
        };
        
        let result = retry_with_backoff(&config, || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err::<(), _>("transient error".into())
                } else {
                    Ok(())
                }
            }
        }, |_| true).await;
        
        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
```

### Integration Testing with Mock Server

```rust
// tests/common/mock_server.rs
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, header};

pub async fn create_mock_openai_server() -> MockServer {
    let server = MockServer::start().await;
    
    // Mock chat completions endpoint
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("Authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "id": "chatcmpl-test",
                "object": "chat.completion.chunk",
                "choices": [{
                    "delta": { "content": "Hello from mock!" },
                    "index": 0
                }]
            }))
        )
        .mount(&server)
        .await;
    
    server
}

#[tokio::test]
async fn test_openai_provider_streaming() {
    let mock_server = create_mock_openai_server().await;
    
    let provider = OpenAIProvider::new(
        "test-key",
        &mock_server.uri(),
        "gpt-4o",
    );
    
    let request = GenerateContentRequest {
        messages: vec![Content::user("Hello")],
        tools: vec![],
        system_instruction: None,
        generation_config: GenerationConfig::default(),
    };
    
    let mut stream = provider.generate_content_stream(request, "test-id");
    
    let mut content = String::new();
    while let Some(event) = stream.next().await {
        if let Some(text) = event?.content {
            content.push_str(&text);
        }
    }
    
    assert_eq!(content, "Hello from mock!");
}
```

### Property-Based Testing for Stream Parser

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_parser_handles_any_valid_json(
        json_string in "\\{[^}]*\\}",  // Simple JSON objects
    ) {
        let mut parser = JsonStreamParser::new();
        
        // Feed character by character
        for (i, ch) in json_string.chars().enumerate() {
            let result = parser.push_chunk(0, &ch.to_string());
            
            // Should only complete at the end
            if i < json_string.len() - 1 {
                assert!(!result.complete);
            }
        }
        
        // Final character should complete
        // (This is a simplification - real test would be more nuanced)
    }
}
```

### Test Fixtures

```rust
// tests/fixtures/mod.rs
use std::path::PathBuf;

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn load_fixture(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

pub fn load_fixture_json<T: serde::de::DeserializeOwned>(name: &str) -> T {
    serde_json::from_str(&load_fixture(name))
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {}", name, e))
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_oauth_pkce_generation

# Run integration tests only
cargo test --test integration_test

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html

# Run tests in release mode (for performance-sensitive tests)
cargo test --release
```

---

## Deployment & Operations

### CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable
        with:
          components: clippy, rustfmt
      
      - name: Cache cargo registry
        uses: Swatinem/rust-cache@v2
      
      - name: Check formatting
        run: cargo fmt --all --check
      
      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
  
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable
      
      - name: Cache cargo registry
        uses: Swatinem/rust-cache@v2
      
      - name: Run tests
        run: cargo test --all-features
      
      - name: Run integration tests
        run: cargo test --test '*'
  
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Cache cargo registry
        uses: Swatinem/rust-cache@v2
      
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: qwen-code-${{ matrix.os }}
          path: target/${{ matrix.target }}/release/qwen
```

### Release Automation

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable
      
      - name: Build for all platforms
        run: |
          cargo build --release
      
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          generate_release_notes: true
          files: |
            target/release/qwen
            target/release/qwen-code
```

### Cross-Compilation

```bash
# Install cross-rs for easy cross-compilation
cargo install cross

# Build for Linux (musl for static linking)
cross build --release --target x86_64-unknown-linux-musl

# Build for macOS ARM
cross build --release --target aarch64-apple-darwin

# Build for Windows
cross build --release --target x86_64-pc-windows-gnu
```

### Binary Distribution

```toml
# Cargo.toml - Release configuration
[package]
name = "qwen-code"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"

# Binary targets
[[bin]]
name = "qwen"
path = "crates/qwen-cli/src/main.rs"

# Optimize for binary size in release
[profile.release]
lto = true
codegen-units = 1
strip = true
```

### Docker Image

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Copy workspace
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/qwen /usr/local/bin/

ENTRYPOINT ["qwen"]
```

### Version Management

```rust
// crates/qwen-cli/src/version.rs
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_TIME: &str = build_time::build_time_utc!();
pub const RUST_VERSION: &str = build_time::rustc_version!();

pub fn version_string() -> String {
    format!(
        "qwen-code {}\nbuilt at: {}\nrustc: {}",
        VERSION, BUILD_TIME, RUST_VERSION
    )
}
```

---

## Migration Guide

### Phase 1: Core CLI (Weeks 1-4)

**Goal**: Working CLI with basic tool support

**Scope**:
- `qwen-types` - Shared types
- `qwen-config` - Configuration loading
- `qwen-network` - HTTP client
- `qwen-provider` - OpenAI provider only
- `qwen-tools` - Shell, Read, Write, Edit, Glob, Grep
- `qwen-cli` - Interactive REPL

**Milestones**:
1. Week 1: Project setup, types, config loading
2. Week 2: HTTP client, OpenAI provider
3. Week 3: Basic tools (Shell, Read, Write, Edit)
4. Week 4: REPL, slash commands, testing

**Deliverable**: `qwen rs` can answer questions and edit files

### Phase 2: Full Tool Parity (Weeks 5-8)

**Goal**: All tools from TypeScript version

**Scope**:
- Remaining tools: Web, Agent, Skill, Memory, Todo, LSP, MCP, Cron
- `qwen-hooks` - Hook system
- `qwen-skills` - Skill loading
- `qwen-subagents` - SubAgent management

**Milestones**:
1. Week 5: Search tools (Glob, Grep, RipGrep, Web)
2. Week 6: Agent, Skill, Memory tools
3. Week 7: Hook system, permissions
4. Week 8: Testing, bug fixes

**Deliverable**: Feature parity with TypeScript qwen-code

### Phase 3: Advanced Features (Weeks 9-12)

**Goal**: Channels, IDE integration, extensions

**Scope**:
- `qwen-channels` - Telegram, WeChat adapters
- `qwen-ide` - VS Code extension
- `qwen-extensions` - Extension system
- `qwen-mcp` - MCP client

**Milestones**:
1. Week 9: AcpBridge, channel base
2. Week 10: IDE detection, VS Code extension
3. Week 11: Extension system, MCP
4. Week 12: Integration testing

**Deliverable**: Full feature set matching TypeScript

### Phase 4: Production Hardening (Weeks 13-16)

**Goal**: Production-ready implementation

**Scope**:
- Performance optimization
- Memory profiling
- Benchmark suite
- Documentation
- Security audit

**Milestones**:
1. Week 13: Performance profiling, optimizations
2. Week 14: Memory profiling, leak detection
3. Week 15: Documentation, examples
4. Week 16: Security audit, release prep

**Deliverable**: Production release v0.1.0

### TypeScript ↔ Rust Interop (During Migration)

During the migration, you may want to run both versions side by side:

```bash
# Install both versions
npm install -g @qwen-code/qwen-code@latest
cargo install qwen-code

# Use Rust version for performance-critical tasks
qwen-rs "refactor this function"

# Use TypeScript version for unimplemented features
qwen-ts "use experimental feature X"
```

### Gradual Migration Strategy

1. **Start with non-critical paths**: Use Rust version for file operations, search
2. **Validate outputs**: Compare Rust vs TypeScript outputs for same inputs
3. **Switch primary**: Once confident, make Rust the default
4. **Deprecate TypeScript**: Keep TypeScript as fallback during transition

---

## Performance Expectations

### Expected Improvements

| Metric | TypeScript (Node.js) | Rust | Improvement |
|--------|---------------------|------|-------------|
| **Startup time** | ~500ms | ~50ms | 10x faster |
| **Memory usage** | ~150MB | ~30MB | 5x less |
| **HTTP latency** | ~50ms overhead | ~5ms overhead | 10x faster |
| **JSON parsing** | ~10ms/file | ~1ms/file | 10x faster |
| **Binary size** | ~100MB (Node + deps) | ~15MB | 6x smaller |

### Benchmarking

```rust
// benches/streaming_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut parser = JsonStreamParser::new();
    let chunk = r#"{"choices":[{"delta":{"content":"test"}}]}"#;
    
    c.bench_function("parse_chunk", |b| {
        b.iter(|| {
            parser.push_chunk(0, black_box(chunk))
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

Run benchmarks:

```bash
cargo bench
```

### Memory Profiling

```bash
# Using heaptrack
heaptrack target/release/qwen

# Using massif (valgrind)
valgrind --tool=massif target/release/qwen

# Analyze with massif-visualizer
massif-visualizer massif.out.*
```

---

## Open Questions

1. **Tauri for desktop GUI?** Could provide optional GUI using Tauri instead of Ratatui terminal UI
2. **GPU acceleration for embeddings?** Could use `candle` or `burn` for local embeddings
3. **WASM for browser extension?** Could compile core to WASM for web-based IDE extensions
4. **Distributed tracing backend?** Should we bundle Jaeger/Tempo for local tracing?

---

*Rust revision plan completed on 2026-04-11.*
       