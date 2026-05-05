# yoke -- Providers

## Provider Trait

**File**: `yoagent/src/provider/traits.rs`

```rust
pub trait StreamProvider: Send + Sync {
    async fn stream(
        &self,
        config: StreamConfig,
        tx: mpsc::UnboundedSender<StreamEvent>,
        cancel: CancellationToken,
    ) -> Result<Message, ProviderError>;
}
```

All providers implement this single trait. They receive a `StreamConfig`, an unbounded channel sender for streaming `StreamEvent`s, and a cancellation token. They return the final complete assistant `Message` on success.

## Available Providers

| Provider | File | API Protocol | Auth |
|----------|------|--------------|------|
| anthropic | `anthropic.rs` | `AnthropicMessages` | `ANTHROPIC_API_KEY` |
| google | `google.rs` | `GoogleGenerativeAi` | `GEMINI_API_KEY` |
| google_vertex | `google_vertex.rs` | `GoogleVertex` | GCP credentials |
| openai (compat) | `openai_compat.rs` | `OpenAiCompletions` | `OPENAI_API_KEY` |
| openai (responses) | `openai_responses.rs` | `OpenAiResponses` | `OPENAI_API_KEY` |
| azure | `azure_openai.rs` | `AzureOpenAiResponses` | Azure credentials |
| bedrock | `bedrock.rs` | `BedrockConverseStream` | AWS credentials |

OpenAI-compatible providers (OpenRouter, Ollama, xAI, etc.) use the `openai_compat` implementation with different base URLs and API keys.

## ModelConfig

**File**: `yoagent/src/provider/model.rs`

```rust
pub struct ModelConfig {
    pub id: String,              // Model identifier sent to API
    pub name: String,            // Human-friendly name
    pub api: ApiProtocol,        // Which API protocol to use
    pub provider: String,        // Provider name (e.g. "openai", "anthropic")
    pub base_url: String,        // Base URL for API requests
    pub reasoning: bool,         // Whether model supports thinking/reasoning
    pub context_window: u32,     // Context window size in tokens
    pub max_tokens: u32,         // Default max output tokens
    pub cost: CostConfig,        // Cost per million tokens
    pub headers: HashMap<String, String>,  // Additional request headers
    pub compat: Option<OpenAiCompat>,      // OpenAI-compat quirk flags
}

pub struct CompatConfig {
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_system_prompt: bool,
    pub thinking_format: ThinkingFormat,
    pub tool_call_format: ToolCallFormat,
}
```

### Factory Methods

```rust
ModelConfig::openai(model_id, display_name)      // api.openai.com
ModelConfig::openrouter(model_id, display_name)  // openrouter.ai/api/v1
ModelConfig::local(base_url, model_id)           // Custom base URL
ModelConfig::google(model_id, display_name)      // Google Generative AI
```

## StreamConfig

What the agent loop sends to the provider:

```rust
pub struct StreamConfig {
    pub model: String,
    pub api_key: String,
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDefinition>,
    pub thinking_level: ThinkingLevel,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub model_config: Option<ModelConfig>,
    pub cache_config: CacheConfig,
    pub web_search: bool,
}
```

## StreamEvent

What providers yield back:

```rust
pub enum StreamEvent {
    Delta(StreamDelta),
    MessageComplete(Message),
    Error(String),
}

pub enum StreamDelta {
    Text { delta: String },
    Thinking { delta: String },
    ToolCallDelta { delta: String },
}
```

## AnthropicProvider

**File**: `yoagent/src/provider/anthropic.rs`

- Uses SSE streaming (`POST /v1/messages` with `stream: true`)
- Native tool use support
- Extended thinking via `thinking` parameter with `budget_tokens`
- Prompt caching via `cache_control` blocks
- Handles `overloaded_error` and context overflow gracefully

### Thinking Budget Mapping

| ThinkingLevel | Budget Tokens |
|--------------|---------------|
| Off | (disabled) |
| Minimal | 1024 |
| Low | 4096 |
| Medium | 10000 |
| High | 32000 |

## GoogleProvider

**File**: `yoagent/src/provider/google.rs`

- Uses SSE streaming (`POST /v1beta/models/{model}:streamGenerateContent`)
- Native tool use with `functionDeclarations`
- Google Search grounding tool (web_search capability)
- Thinking via `thinkingConfig` with `thinkingBudget`
- Returns grounding metadata (search results, sources)

### Thinking Budget Mapping

| ThinkingLevel | Budget Tokens |
|--------------|---------------|
| Off | 0 (thinking disabled) |
| Minimal | 1024 |
| Low | 4096 |
| Medium | 8192 |
| High | 24576 |

## OpenAiCompatProvider

**File**: `yoagent/src/provider/openai_compat.rs`

Generic OpenAI Chat Completions client. Used for OpenAI, OpenRouter, Ollama, and any compatible API.

- SSE streaming (`POST /v1/chat/completions` with `stream: true`)
- Tool use via `tools` array with `function` type
- Supports multiple thinking formats:
  - `ThinkingFormat::OpenAi` — reasoning_effort parameter
  - `ThinkingFormat::Xai` — Separate thinking content blocks
  - `ThinkingFormat::None` — No thinking support

### ThinkingFormat

```rust
pub enum ThinkingFormat {
    None,    // No thinking support
    OpenAi,  // reasoning_effort: "low"/"medium"/"high"
    Xai,     // Separate content blocks (for compatible local models)
}
```

## Provider Registry

**File**: `yoagent/src/provider/registry.rs`

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn StreamProvider>>,
}
```

A registry for managing multiple providers. Used in applications that support dynamic provider switching.

## Web Search Support

| Provider | Mechanism | Works with tools? |
|----------|-----------|-------------------|
| Anthropic | Server-side tool (model invokes `web_search`) | Yes |
| OpenAI | Dedicated search models | No (model-level) |
| Gemini | Google Search grounding tool | Yes |

## Error Handling

```rust
pub enum ProviderError {
    RateLimit { retry_after: Option<Duration> },
    ContextOverflow { message: String },
    ServerError { status: u16, body: String },
    ConnectionError(String),
    ParseError(String),
}
```

Context overflow detection (cross-provider):

```rust
pub fn is_context_overflow_message(msg: &str) -> bool {
    // Checks for known patterns:
    // "prompt is too long", "context_length_exceeded",
    // "maximum context length", "token limit", etc.
}
```
