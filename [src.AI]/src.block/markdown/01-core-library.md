# Goose Core Library: 40+ AI Providers, Agent Framework, MCP Integration

The `goose` core crate is the heart of the system — a Rust library providing 40+ AI provider adapters, an agent framework with tool execution, session management, and MCP utilities.

Source: `crates/goose/src/` — 26 modules, 175 Rust files.

## Module Map

```
goose/src/
├── agents/             # Agent framework (32 files)
├── providers/          # 40+ AI provider adapters (43 files)
├── config/             # Configuration system
├── context_mgmt/       # Context management
├── conversation/       # Conversation handling
├── execution/          # Tool execution engine
├── model/              # Model types
├── session/            # Session management
├── scheduler/          # Task scheduler
├── security/           # Security inspection
├── recipe/             # Recipe system
├── recipe_deeplink/    # Recipe deeplinks
├── token_counter/      # Token counting
├── tool_inspection/    # Tool inspection
├── tool_monitor/       # Tool monitoring
├── permission/         # Permission system
├── mcp_utils/          # MCP utilities
├── prompt_template/    # Prompt templates
├── oauth/              # OAuth support
├── subprocess/         # Subprocess management
├── hints/              # AI hints
├── logging/            # Logging
├── tracing/            # Tracing
└── utils/              # Utilities
```

## Provider Registry

Source: `providers/factory.rs`

40+ providers registered in a global `OnceCell<RwLock<ProviderRegistry>>`:

```rust
static REGISTRY: OnceCell<RwLock<ProviderRegistry>> = OnceCell::const_new();

async fn init_registry() -> RwLock<ProviderRegistry> {
    let mut registry = ProviderRegistry::new().with_providers(|registry| {
        registry.register::<AnthropicProvider, _>(|m| Box::pin(AnthropicProvider::from_env(m)), true);
        registry.register::<AzureProvider, _>(|m| Box::pin(AzureProvider::from_env(m)), false);
        registry.register::<BedrockProvider, _>(|m| Box::pin(BedrockProvider::from_env(m)), false);
        registry.register::<ClaudeCodeProvider, _>(|m| Box::pin(ClaudeCodeProvider::from_env(m)), true);
        registry.register::<CursorAgentProvider, _>(|m| Box::pin(CursorAgentProvider::from_env(m)), false);
        registry.register::<DatabricksProvider, _>(|m| Box::pin(DatabricksProvider::from_env(m)), true);
        registry.register::<GcpVertexAIProvider, _>(|m| Box::pin(GcpVertexAIProvider::from_env(m)), false);
        registry.register::<GeminiCliProvider, _>(|m| Box::pin(GeminiCliProvider::from_env(m)), false);
        registry.register::<GithubCopilotProvider, _>(|m| Box::pin(GithubCopilotProvider::from_env(m)), false);
        registry.register::<GoogleProvider, _>(|m| Box::pin(GoogleProvider::from_env(m)), true);
        registry.register::<LiteLLMProvider, _>(|m| Box::pin(LiteLLMProvider::from_env(m)), false);
        registry.register::<OllamaProvider, _>(|m| Box::pin(OllamaProvider::from_env(m)), true);
        registry.register::<OpenAiProvider, _>(|m| Box::pin(OpenAiProvider::from_env(m)), true);
        registry.register::<OpenRouterProvider, _>(|m| Box::pin(OpenRouterProvider::from_env(m)), true);
        registry.register::<SageMakerTgiProvider, _>(|m| Box::pin(SageMakerTgiProvider::from_env(m)), false);
        registry.register::<SnowflakeProvider, _>(|m| Box::pin(SnowflakeProvider::from_env(m)), false);
        registry.register::<TetrateProvider, _>(|m| Box::pin(TetrateProvider::from_env(m)), true);
        registry.register::<VeniceProvider, _>(|m| Box::pin(VeniceProvider::from_env(m)), false);
        registry.register::<XaiProvider, _>(|m| Box::pin(XaiProvider::from_env(m)), false);
        // ... and more: lead_worker, declarative, sagemaker_tgi, tetrate, toolshim, usage_estimator
    });
    register_declarative_providers(&mut registry);
    RwLock::new(registry)
}
```

Aha: The second boolean parameter indicates whether the provider is "primary" (listed by default in configuration). Anthropic, Claude Code, Databricks, Google, Ollama, OpenAI, OpenRouter, and Tetrate are primary; others require explicit selection.

### Provider Trait

Source: `providers/base.rs`

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider metadata (name, description, models)
    fn metadata(&self) -> ProviderMetadata;

    /// Complete a chat conversation (non-streaming)
    async fn complete(&self, messages: Vec<Message>, tools: Vec<Tool>) -> Result<Message>;

    /// Complete a chat conversation (streaming)
    async fn complete_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<Tool>,
    ) -> Result<MessageStream>;

    /// Get model info
    fn model_config(&self) -> &ModelConfig;
}
```

### OpenAI Provider

Source: `providers/openai.rs`

```rust
pub const OPEN_AI_DEFAULT_MODEL: &str = "gpt-4o";
pub const OPEN_AI_DEFAULT_FAST_MODEL: &str = "gpt-4o-mini";
pub const OPEN_AI_KNOWN_MODELS: &[(&str, usize)] = &[
    ("gpt-4o", 128_000),
    ("gpt-4o-mini", 128_000),
    ("gpt-4.1", 128_000),
    ("gpt-4.1-mini", 128_000),
    ("o1", 200_000),
    ("o3", 200_000),
    ("gpt-3.5-turbo", 16_385),
    ("gpt-4-turbo", 128_000),
    ("o4-mini", 128_000),
];

pub struct OpenAiProvider {
    api_client: ApiClient,
    base_path: String,
    organization: Option<String>,
    project: Option<String>,
    model: ModelConfig,
    custom_headers: Option<HashMap<String, String>>,
    supports_streaming: bool,
    name: String,
}

impl OpenAiProvider {
    pub async fn from_env(model: ModelConfig) -> Result<Self> {
        let model = model.with_fast(OPEN_AI_DEFAULT_FAST_MODEL.to_string());
        let config = crate::config::Config::global();
        let api_key: String = config.get_secret("OPENAI_API_KEY")?;
        let host: String = config.get_param("OPENAI_HOST")
            .unwrap_or_else(|_| "https://api.openai.com".to_string());
        let base_path: String = config.get_param("OPENAI_BASE_PATH")
            .unwrap_or_else(|_| "v1/chat/completions".to_string());
        // ... handles OPENAI_ORGANIZATION, OPENAI_PROJECT, OPENAI_CUSTOM_HEADERS, OPENAI_TIMEOUT
    }
}
```

Environment variables: `OPENAI_API_KEY`, `OPENAI_HOST`, `OPENAI_BASE_PATH`, `OPENAI_ORGANIZATION`, `OPENAI_PROJECT`, `OPENAI_CUSTOM_HEADERS`, `OPENAI_TIMEOUT`.

### Global Model Store

```rust
pub static CURRENT_MODEL: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

pub fn set_current_model(model: &str) {
    if let Ok(mut current_model) = CURRENT_MODEL.lock() {
        *current_model = Some(model.to_string());
    }
}

pub fn get_current_model() -> Option<String> {
    CURRENT_MODEL.lock().ok().and_then(|model| model.clone())
}
```

Aha: The global `CURRENT_MODEL` stores the real model name returned by the provider (not the alias the user configured). When a provider responds with the actual model used, it's stored here so downstream code can report the real model name.

### ModelInfo

```rust
pub struct ModelInfo {
    pub name: String,
    pub context_limit: usize,
    pub input_token_cost: Option<f64>,
    pub output_token_cost: Option<f64>,
    pub currency: Option<String>,
    pub supports_cache_control: Option<bool>,
}
```

## Agent Framework

Source: `agents/` — 32 files.

### Agent

Source: `agents/agent.rs`

```rust
const DEFAULT_MAX_TURNS: u32 = 1000;
const COMPACTION_THINKING_TEXT: &str = "goose is compacting the conversation...";
pub const MANUAL_COMPACT_TRIGGER: &str = "Please compact this conversation";

pub struct ReplyContext {
    pub conversation: Conversation,
    pub tools: Vec<Tool>,
    pub toolshim_tools: Vec<Tool>,
    pub system_prompt: String,
    pub goose_mode: GooseMode,
    pub initial_messages: Vec<Message>,
}
```

### ReplyContext

The context needed for the agent's reply function:
- **conversation** — The conversation state
- **tools** — Available tools
- **toolshim_tools** — Shimming tools for specific providers
- **system_prompt** — System prompt text
- **goose_mode** — Operating mode
- **initial_messages** — Messages before the reply

### Types

Source: `agents/types.rs`

```rust
pub type ToolResultReceiver = Arc<Mutex<mpsc::Receiver<(String, ToolResult<Vec<Content>>)>>>;
pub type SharedProvider = Arc<Mutex<Option<Arc<dyn Provider>>>>;

pub const DEFAULT_RETRY_TIMEOUT_SECONDS: u64 = 300;      // 5 minutes
pub const DEFAULT_ON_FAILURE_TIMEOUT_SECONDS: u64 = 600; // 10 minutes
```

### RetryConfig

```rust
pub struct RetryConfig {
    pub max_retries: u32,
    pub checks: Vec<SuccessCheck>,
    pub on_failure: Option<String>,           // Shell command on failure
    pub timeout_seconds: Option<u64>,         // Default: 300s
    pub on_failure_timeout_seconds: Option<u64>, // Default: 600s
}
```

### SuccessCheck

```rust
pub enum SuccessCheck {
    Shell { command: String },  // Execute shell command, check exit status
}
```

## Session Management

Source: `session/` — 6 files.

```
session/
├── mod.rs
├── session_manager.rs    # SessionManager
├── chat_history_search.rs # Chat history search
├── diagnostics.rs        # Session diagnostics
├── extension_data.rs     # Extension state
└── legacy.rs             # Legacy session format
```

## MCP Developer Server

Source: `../goose-mcp/src/developer/` — 8 files.

```
developer/
├── mod.rs
├── rmcp_developer.rs     # MCP developer server runner
├── shell.rs              # Shell command execution
├── text_editor.rs        # Text editor operations
├── lang.rs               # Language detection
├── analyze/              # Code analysis
├── editor_models/        # Editor model definitions
└── prompts/              # Developer prompts
```

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [CLI](02-cli.md) — CLI commands, session management
- [MCP Server](03-mcp-server.md) — MCP servers
- [HTTP Server](04-http-server.md) — REST API

[Next → CLI](02-cli.md)
