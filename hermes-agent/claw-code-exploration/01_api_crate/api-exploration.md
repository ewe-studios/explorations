# API Crate — Comprehensive Exploration

**Repository:** claw-code / claw-code-latest
**Path:** `rust/crates/api/`
**Purpose:** Provider abstraction layer for Anthropic, xAI (Grok), and OpenAI APIs with streaming support

---

## Table of Contents

1. [Crate Overview](#crate-overview)
2. [Module Structure](#module-structure)
3. [File-by-File Analysis](#file-by-file-analysis)
4. [Key Types and Data Structures](#key-types-and-data-structures)
5. [Differences: Original vs Latest](#differences-original-vs-latest)

---

## Crate Overview

### Purpose
The `api` crate provides a unified abstraction layer for communicating with LLM providers. It handles:
- HTTP client management
- Request/response serialization
- Server-Sent Events (SSE) streaming
- Authentication (API key, OAuth, bearer tokens)
- Retry logic with exponential backoff
- Error classification and handling

### Dependencies (Latest)
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
runtime = { path = "../runtime" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
telemetry = { path = "../telemetry" }  # NEW in latest
tokio = { version = "1", features = ["io-util", "macros", "net", "rt-multi-thread", "time"] }
```

### Changes from Original
- **Added:** `telemetry` dependency for session tracing
- **Added:** `prompt_cache.rs` module
- **Added:** `providers/` directory with multi-provider support
- **Added:** `prompt_cache.rs` - File-based caching with fingerprint invalidation

---

## Module Structure

### Original claw-code
```
src/
├── client.rs      # AnthropicClient, AuthSource, OAuth flows
├── error.rs       # ApiError enum
├── lib.rs         # Module declarations and exports
├── sse.rs         # SSE parser for streaming
└── types.rs       # Request/response types
```

### Latest claw-code-latest
```
src/
├── client.rs              # ProviderClient enum (multi-provider)
├── error.rs               # ApiError with context-window errors
├── lib.rs                 # Extended exports
├── prompt_cache.rs        # NEW: File-based prompt caching
├── providers/
│   ├── mod.rs             # Provider trait, ProviderKind enum
│   ├── anthropic.rs       # AnthropicClient (extracted)
│   └── openai_compat.rs   # NEW: OpenAI/xAI compatible client
├── sse.rs                 # Enhanced SSE parser
└── types.rs               # Extended with Thinking blocks
```

---

## File-by-File Analysis

### lib.rs

#### Original (17 lines)
```rust
mod client;
mod error;
mod sse;
mod types;

pub use client::{
    oauth_token_is_expired, read_base_url, resolve_saved_oauth_token, resolve_startup_auth_source,
    AnthropicClient, AuthSource, MessageStream, OAuthTokenSet,
};
pub use error::ApiError;
pub use sse::{parse_frame, SseParser};
pub use types::{...};
```

**Line-by-Line:**
- **Line 1-4:** Module declarations - defines the four core modules
- **Line 6-9:** Client exports - re-exports OAuth utilities and AnthropicClient
- **Line 10:** Error export - single error type for all API operations
- **Line 11:** SSE export - streaming parser utilities
- **Line 12-17:** Type exports - all request/response types

#### Latest (34 lines)
```rust
mod client;
mod error;
mod prompt_cache;           // NEW
mod providers;              // NEW
mod sse;
mod types;

pub use client::{
    oauth_token_is_expired, read_base_url, read_xai_base_url,  // +read_xai_base_url
    resolve_saved_oauth_token, resolve_startup_auth_source,
    MessageStream, OAuthTokenSet, ProviderClient,              // +ProviderClient
};
pub use error::ApiError;
pub use prompt_cache::{                                       // NEW
    CacheBreakEvent, PromptCache, PromptCacheConfig,
    PromptCachePaths, PromptCacheRecord, PromptCacheStats,
};
pub use providers::anthropic::{AnthropicClient, AnthropicClient as ApiClient, AuthSource};
pub use providers::openai_compat::{OpenAiCompatClient, OpenAiCompatConfig};  // NEW
pub use providers::{                                          // NEW
    detect_provider_kind, max_tokens_for_model,
    resolve_model_alias, ProviderKind,
};
pub use sse::{parse_frame, SseParser};
pub use types::{...};

pub use telemetry::{                                          // NEW
    AnalyticsEvent, AnthropicRequestProfile, ClientIdentity,
    JsonlTelemetrySink, MemoryTelemetrySink, SessionTraceRecord,
    SessionTracer, TelemetryEvent, TelemetrySink,
    DEFAULT_ANTHROPIC_VERSION,
};
```

**New Exports:**
- `ProviderClient` - Unified enum for all provider backends
- `PromptCache*` types - Complete caching system
- `ProviderKind` - Anthropic/xAI/OpenAI discrimination
- `OpenAiCompatClient` - OpenAI-compatible API client
- Telemetry types - Session tracing integration

---

### types.rs

#### Original (212 lines)

**MessageRequest (Lines 4-25)**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<InputMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stream: bool,
}
```

**Line-by-Line Analysis:**
- **Line 5:** Derives - Debug (formatting), Clone (copying), PartialEq (comparison), Serialize/Deserialize (JSON)
- **Line 6:** `model` - Model identifier (e.g., "claude-opus-4-6")
- **Line 7:** `max_tokens` - Maximum output tokens to generate
- **Line 8:** `messages` - Conversation history
- **Line 9-10:** `system` - Optional system prompt (skipped if None)
- **Line 11-12:** `tools` - Tool definitions for function calling
- **Line 13-14:** `tool_choice` - How to select tools (Auto/Any/Specific)
- **Line 15-16:** `stream` - Whether to use SSE streaming (default false)

**InputMessage (Lines 27-58)**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMessage {
    pub role: String,
    pub content: Vec<InputContentBlock>,
}
```

**Key Methods:**
- `user_text(text)` - Constructor for user text messages
- `user_tool_result(tool_use_id, content, is_error)` - Tool response messages

**InputContentBlock (Lines 61-78)**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    ToolResult {
        tool_use_id: String,
        content: Vec<ToolResultContentBlock>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
    },
}
```

**Serde Attribute Explanation:**
- `tag = "type"` - Uses "type" field for enum variant discrimination
- `rename_all = "snake_case"` - Converts variant names (Text → "text")

**OutputContentBlock (Lines 127-138)**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
}
```

**Usage (Lines 140-155)**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    pub output_tokens: u32,
}

impl Usage {
    #[must_use]
    pub const fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}
```

**Note:** Original does NOT include cache tokens in total. Latest does.

**StreamEvent (Lines 203-212)**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart(MessageStartEvent),
    MessageDelta(MessageDeltaEvent),
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
    MessageStop(MessageStopEvent),
}
```

#### Latest (290 lines)

**Major Additions:**

1. **Cost Estimation (Lines 179-186)**
```rust
impl Usage {
    #[must_use]
    pub fn estimated_cost_usd(&self, model: &str) -> UsageCostEstimate {
        let usage = self.token_usage();
        pricing_for_model(model).map_or_else(
            || usage.estimate_cost_usd(),
            |pricing| usage.estimate_cost_usd_with_pricing(pricing),
        )
    }
}
```

2. **Thinking Content Blocks (Lines 139-147)**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    Thinking {                                    // NEW
        #[serde(default)]
        thinking: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    RedactedThinking {                           // NEW
        data: Value,
    },
}
```

3. **Enhanced Usage total_tokens (Lines 160-177)**
```rust
impl Usage {
    #[must_use]
    pub const fn total_tokens(&self) -> u32 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens   // NOW INCLUDED
            + self.cache_read_input_tokens       // NOW INCLUDED
    }

    #[must_use]
    pub const fn token_usage(&self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cache_creation_input_tokens: self.cache_creation_input_tokens,
            cache_read_input_tokens: self.cache_read_input_tokens,
        }
    }
}
```

4. **New Delta Types (Lines 220-227)**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },      // NEW
    SignatureDelta { signature: String },    // NEW
}
```

5. **New Tests (Lines 248-289)**
- `usage_total_tokens_includes_cache_tokens` - Verifies cache tokens counted
- `message_response_estimates_cost_from_model_usage` - Cost calculation test

---

### error.rs

#### Original (134 lines)

**ApiError Enum (Lines 5-30)**
```rust
#[derive(Debug)]
pub enum ApiError {
    MissingApiKey,
    ExpiredOAuthToken,
    Auth(String),
    InvalidApiKeyEnv(VarError),
    Http(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Api {
        status: reqwest::StatusCode,
        error_type: Option<String>,
        message: Option<String>,
        body: String,
        retryable: bool,
    },
    RetriesExhausted {
        attempts: u32,
        last_error: Box<ApiError>,
    },
    InvalidSseFrame(&'static str),
    BackoffOverflow {
        attempt: u32,
        base_delay: Duration,
    },
}
```

**is_retryable (Lines 32-48)**
```rust
impl ApiError {
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(error) => error.is_connect() || error.is_timeout() || error.is_request(),
            Self::Api { retryable, .. } => *retryable,
            Self::RetriesExhausted { last_error, .. } => last_error.is_retryable(),
            Self::MissingApiKey | Self::ExpiredOAuthToken | Self::Auth(_)
            | Self::InvalidApiKeyEnv(_) | Self::Io(_) | Self::Json(_)
            | Self::InvalidSseFrame(_) | Self::BackoffOverflow { .. } => false,
        }
    }
}
```

#### Latest (353 lines)

**New Error Variants:**
```rust
#[derive(Debug)]
pub enum ApiError {
    MissingCredentials {                    // CHANGED
        provider: &'static str,
        env_vars: &'static [&'static str],
    },
    ContextWindowExceeded {                 // NEW
        model: String,
        estimated_input_tokens: u32,
        requested_output_tokens: u32,
        estimated_total_tokens: u32,
        context_window_tokens: u32,
    },
    ExpiredOAuthToken,
    Auth(String),
    InvalidApiKeyEnv(VarError),
    Http(reqwest::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Api {
        status: reqwest::StatusCode,
        error_type: Option<String>,
        message: Option<String>,
        request_id: Option<String>,        // NEW
        body: String,
        retryable: bool,
    },
    RetriesExhausted {
        attempts: u32,
        last_error: Box<ApiError>,
    },
    InvalidSseFrame(&'static str),
    BackoffOverflow {
        attempt: u32,
        base_delay: Duration,
    },
}
```

**New Methods:**

1. **request_id (Lines 85-101)**
```rust
#[must_use]
pub fn request_id(&self) -> Option<&str> {
    match self {
        Self::Api { request_id, .. } => request_id.as_deref(),
        Self::RetriesExhausted { last_error, .. } => last_error.request_id(),
        _ => None,
    }
}
```

2. **safe_failure_class (Lines 103-125)**
```rust
#[must_use]
pub fn safe_failure_class(&self) -> &'static str {
    match self {
        Self::RetriesExhausted { .. } if self.is_context_window_failure() => "context_window",
        Self::RetriesExhausted { .. } if self.is_generic_fatal_wrapper() => "provider_retry_exhausted",
        Self::RetriesExhausted { last_error, .. } => last_error.safe_failure_class(),
        Self::MissingCredentials { .. } | Self::ExpiredOAuthToken | Self::Auth(_) => "provider_auth",
        Self::Api { status, .. } if matches!(status.as_u16(), 401 | 403) => "provider_auth",
        Self::ContextWindowExceeded { .. } => "context_window",
        Self::Api { .. } if self.is_context_window_failure() => "context_window",
        Self::Api { status, .. } if status.as_u16() == 429 => "provider_rate_limit",
        Self::Api { .. } if self.is_generic_fatal_wrapper() => "provider_internal",
        Self::Api { .. } => "provider_error",
        Self::Http(_) | Self::InvalidSseFrame(_) | Self::BackoffOverflow { .. } => "provider_transport",
        Self::InvalidApiKeyEnv(_) | Self::Io(_) | Self::Json(_) => "runtime_io",
    }
}
```

3. **is_generic_fatal_wrapper (Lines 127-148)**
```rust
const GENERIC_FATAL_WRAPPER_MARKERS: &[&str] = &[
    "something went wrong while processing your request",
    "please try again, or use /new to start a fresh session",
];

#[must_use]
pub fn is_generic_fatal_wrapper(&self) -> bool {
    match self {
        Self::Api { message, body, .. } => {
            message.as_deref().is_some_and(looks_like_generic_fatal_wrapper)
                || looks_like_generic_fatal_wrapper(body)
        }
        Self::RetriesExhausted { last_error, .. } => last_error.is_generic_fatal_wrapper(),
        _ => false,
    }
}
```

4. **is_context_window_failure (Lines 150-177)**
```rust
const CONTEXT_WINDOW_ERROR_MARKERS: &[&str] = &[
    "maximum context length",
    "context window",
    "context length",
    "too many tokens",
    "prompt is too long",
    "input is too long",
    "request is too large",
];

#[must_use]
pub fn is_context_window_failure(&self) -> bool {
    match self {
        Self::ContextWindowExceeded { .. } => true,
        Self::Api { status, message, body, .. } => {
            matches!(status.as_u16(), 400 | 413 | 422)
                && (message.as_deref().is_some_and(looks_like_context_window_error)
                    || looks_like_context_window_error(body))
        }
        Self::RetriesExhausted { last_error, .. } => last_error.is_context_window_failure(),
        _ => false,
    }
}
```

**Display Implementation - Key Changes:**
```rust
Self::ContextWindowExceeded { model, estimated_input_tokens, requested_output_tokens, 
    estimated_total_tokens, context_window_tokens } => write!(
    f,
    "context_window_blocked for {model}: estimated input {estimated_input_tokens} + requested output {requested_output_tokens} = {estimated_total_tokens} tokens exceeds the {context_window_tokens}-token context window; compact the session or reduce request size before retrying"
),
```

---

### client.rs

#### Original (994 lines)

**AuthSource Enum (Lines 22-86)**
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthSource {
    None,
    ApiKey(String),
    BearerToken(String),
    ApiKeyAndBearer { api_key: String, bearer_token: String },
}
```

**Key Methods:**
- `from_env()` - Reads ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN
- `api_key()` / `bearer_token()` - Extract credentials
- `apply(request_builder)` - Apply auth headers to request

**AnthropicClient (Lines 103-337)**
```rust
pub struct AnthropicClient {
    http: reqwest::Client,
    auth: AuthSource,
    base_url: String,
    max_retries: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
}
```

**Retry Logic (Lines 273-336):**
```rust
async fn send_with_retry(&self, request: &MessageRequest) -> Result<reqwest::Response, ApiError> {
    let mut attempts = 0;
    let mut last_error: Option<ApiError>;
    
    loop {
        attempts += 1;
        match self.send_raw_request(request).await {
            Ok(response) => match expect_success(response).await {
                Ok(response) => return Ok(response),
                Err(error) if error.is_retryable() && attempts <= self.max_retries + 1 => {
                    last_error = Some(error);
                }
                Err(error) => return Err(error),
            },
            Err(error) if error.is_retryable() && attempts <= self.max_retries + 1 => {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
        
        if attempts > self.max_retries {
            break;
        }
        
        tokio::time::sleep(self.backoff_for_attempt(attempts)?).await;
    }
    
    Err(ApiError::RetriesExhausted { attempts, last_error: Box::new(last_error.expect(...)) })
}
```

**Exponential Backoff (Lines 325-336):**
```rust
fn backoff_for_attempt(&self, attempt: u32) -> Result<Duration, ApiError> {
    let Some(multiplier) = 1_u32.checked_shl(attempt.saturating_sub(1)) else {
        return Err(ApiError::BackoffOverflow { attempt, base_delay: self.initial_backoff });
    };
    Ok(self.initial_backoff.checked_mul(multiplier)
        .map_or(self.max_backoff, |delay| delay.min(self.max_backoff)))
}
```

#### Latest (155 lines for main client.rs, with providers extracted)

**ProviderClient Enum (Lines 8-14)**
```rust
#[derive(Debug, Clone)]
pub enum ProviderClient {
    Anthropic(AnthropicClient),
    Xai(OpenAiCompatClient),
    OpenAi(OpenAiCompatClient),
}
```

**Multi-Provider Factory (Lines 16-98):**
```rust
impl ProviderClient {
    pub fn from_model(model: &str) -> Result<Self, ApiError> {
        Self::from_model_with_anthropic_auth(model, None)
    }
    
    pub fn from_model_with_anthropic_auth(
        model: &str,
        anthropic_auth: Option<AuthSource>,
    ) -> Result<Self, ApiError> {
        let resolved_model = providers::resolve_model_alias(model);
        match providers::detect_provider_kind(&resolved_model) {
            ProviderKind::Anthropic => Ok(Self::Anthropic(match anthropic_auth {
                Some(auth) => AnthropicClient::from_auth(auth),
                None => AnthropicClient::from_env()?,
            })),
            ProviderKind::Xai => Ok(Self::Xai(OpenAiCompatClient::from_env(
                OpenAiCompatConfig::xai(),
            )?)),
            ProviderKind::OpenAi => Ok(Self::OpenAi(OpenAiCompatClient::from_env(
                OpenAiCompatConfig::openai(),
            )?)),
        }
    }
    
    pub async fn send_message(&self, request: &MessageRequest) -> Result<MessageResponse, ApiError> {
        match self {
            Self::Anthropic(client) => client.send_message(request).await,
            Self::Xai(client) | Self::OpenAi(client) => client.send_message(request).await,
        }
    }
    
    pub async fn stream_message(&self, request: &MessageRequest) -> Result<MessageStream, ApiError> {
        match self {
            Self::Anthropic(client) => client.stream_message(request)
                .await.map(MessageStream::Anthropic),
            Self::Xai(client) | Self::OpenAi(client) => client.stream_message(request)
                .await.map(MessageStream::OpenAiCompat),
        }
    }
}
```

**MessageStream Enum (Lines 100-121):**
```rust
#[derive(Debug)]
pub enum MessageStream {
    Anthropic(anthropic::MessageStream),
    OpenAiCompat(openai_compat::MessageStream),
}

impl MessageStream {
    pub fn request_id(&self) -> Option<&str> { ... }
    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        match self {
            Self::Anthropic(stream) => stream.next_event().await,
            Self::OpenAiCompat(stream) => stream.next_event().await,
        }
    }
}
```

---

### providers/mod.rs (NEW in Latest - 378 lines)

**Provider Trait (Lines 16-29):**
```rust
pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, ApiError>> + Send + 'a>>;

pub trait Provider {
    type Stream;
    
    fn send_message<'a>(&'a self, request: &'a MessageRequest) -> ProviderFuture<'a, MessageResponse>;
    fn stream_message<'a>(&'a self, request: &'a MessageRequest) -> ProviderFuture<'a, Self::Stream>;
}
```

**ProviderKind Enum (Lines 31-36):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Anthropic,
    Xai,
    OpenAi,
}
```

**Model Registry (Lines 52-125):**
```rust
const MODEL_REGISTRY: &[(&str, ProviderMetadata)] = &[
    ("opus", ProviderMetadata { provider: Anthropic, ... }),
    ("sonnet", ProviderMetadata { provider: Anthropic, ... }),
    ("haiku", ProviderMetadata { provider: Anthropic, ... }),
    ("grok", ProviderMetadata { provider: Xai, ... }),
    ("grok-3", ProviderMetadata { provider: Xai, ... }),
    ("grok-mini", ProviderMetadata { provider: Xai, ... }),
    ("grok-3-mini", ProviderMetadata { provider: Xai, ... }),
    ("grok-2", ProviderMetadata { provider: Xai, ... }),
];
```

**resolve_model_alias (Lines 127-151):**
```rust
#[must_use]
pub fn resolve_model_alias(model: &str) -> String {
    let trimmed = model.trim();
    let lower = trimmed.to_ascii_lowercase();
    MODEL_REGISTRY.iter().find_map(|(alias, metadata)| {
        (*alias == lower).then_some(match metadata.provider {
            ProviderKind::Anthropic => match *alias {
                "opus" => "claude-opus-4-6",
                "sonnet" => "claude-sonnet-4-6",
                "haiku" => "claude-haiku-4-5-20251213",
                _ => trimmed,
            },
            ProviderKind::Xai => match *alias {
                "grok" | "grok-3" => "grok-3",
                "grok-mini" | "grok-3-mini" => "grok-3-mini",
                "grok-2" => "grok-2",
                _ => trimmed,
            },
            ProviderKind::OpenAi => trimmed,
        })
    })
    .map_or_else(|| trimmed.to_string(), ToOwned::to_owned)
}
```

**detect_provider_kind (Lines 175-190):**
```rust
#[must_use]
pub fn detect_provider_kind(model: &str) -> ProviderKind {
    if let Some(metadata) = metadata_for_model(model) {
        return metadata.provider;
    }
    if anthropic::has_auth_from_env_or_saved().unwrap_or(false) {
        return ProviderKind::Anthropic;
    }
    if openai_compat::has_api_key("OPENAI_API_KEY") {
        return ProviderKind::OpenAi;
    }
    if openai_compat::has_api_key("XAI_API_KEY") {
        return ProviderKind::Xai;
    }
    ProviderKind::Anthropic  // Default fallback
}
```

**Context Window Preflight (Lines 227-259):**
```rust
pub fn preflight_message_request(request: &MessageRequest) -> Result<(), ApiError> {
    let Some(limit) = model_token_limit(&request.model) else {
        return Ok(());
    };
    
    let estimated_input_tokens = estimate_message_request_input_tokens(request);
    let estimated_total_tokens = estimated_input_tokens.saturating_add(request.max_tokens);
    if estimated_total_tokens > limit.context_window_tokens {
        return Err(ApiError::ContextWindowExceeded {
            model: resolve_model_alias(&request.model),
            estimated_input_tokens,
            requested_output_tokens: request.max_tokens,
            estimated_total_tokens,
            context_window_tokens: limit.context_window_tokens,
        });
    }
    
    Ok(())
}
```

---

### providers/openai_compat.rs (NEW in Latest - 1108 lines)

**Purpose:** Provides OpenAI API-compatible client for xAI (Grok) and OpenAI models.

**OpenAiCompatConfig (Lines 25-64):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenAiCompatConfig {
    pub provider_name: &'static str,
    pub api_key_env: &'static str,
    pub base_url_env: &'static str,
    pub default_base_url: &'static str,
}

impl OpenAiCompatConfig {
    #[must_use]
    pub const fn xai() -> Self {
        Self {
            provider_name: "xAI",
            api_key_env: "XAI_API_KEY",
            base_url_env: "XAI_BASE_URL",
            default_base_url: DEFAULT_XAI_BASE_URL,
        }
    }
    
    #[must_use]
    pub const fn openai() -> Self {
        Self {
            provider_name: "OpenAI",
            api_key_env: "OPENAI_API_KEY",
            base_url_env: "OPENAI_BASE_URL",
            default_base_url: DEFAULT_OPENAI_BASE_URL,
        }
    }
}
```

**OpenAiCompatClient (Lines 66-218):**
```rust
pub struct OpenAiCompatClient {
    http: reqwest::Client,
    api_key: String,
    config: OpenAiCompatConfig,
    base_url: String,
    max_retries: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
}
```

**Chat Completion Request Building (Lines 643-675):**
```rust
fn build_chat_completion_request(request: &MessageRequest, config: OpenAiCompatConfig) -> Value {
    let mut messages = Vec::new();
    if let Some(system) = request.system.as_ref().filter(|value| !value.is_empty()) {
        messages.push(json!({ "role": "system", "content": system }));
    }
    for message in &request.messages {
        messages.extend(translate_message(message));
    }
    
    let mut payload = json!({
        "model": request.model,
        "max_tokens": request.max_tokens,
        "messages": messages,
        "stream": request.stream,
    });
    
    if request.stream && should_request_stream_usage(config) {
        payload["stream_options"] = json!({ "include_usage": true });
    }
    
    if let Some(tools) = &request.tools {
        payload["tools"] = Value::Array(tools.iter().map(openai_tool_definition).collect());
    }
    if let Some(tool_choice) = &request.tool_choice {
        payload["tool_choice"] = openai_tool_choice(tool_choice);
    }
    
    payload
}
```

**Message Translation (Lines 677-728):**
```rust
fn translate_message(message: &InputMessage) -> Vec<Value> {
    match message.role.as_str() {
        "assistant" => {
            let mut text = String::new();
            let mut tool_calls = Vec::new();
            for block in &message.content {
                match block {
                    InputContentBlock::Text { text: value } => text.push_str(value),
                    InputContentBlock::ToolUse { id, name, input } => {
                        tool_calls.push(json!({
                            "id": id,
                            "type": "function",
                            "function": { "name": name, "arguments": input.to_string() }
                        }))
                    }
                    InputContentBlock::ToolResult { .. } => {}
                }
            }
            if text.is_empty() && tool_calls.is_empty() {
                Vec::new()
            } else {
                vec![json!({
                    "role": "assistant",
                    "content": (!text.is_empty()).then_some(text),
                    "tool_calls": tool_calls,
                })]
            }
        }
        _ => message.content.iter().filter_map(|block| match block {
            InputContentBlock::Text { text } => Some(json!({ "role": "user", "content": text })),
            InputContentBlock::ToolResult { tool_use_id, content, is_error } => Some(json!({
                "role": "tool",
                "tool_call_id": tool_use_id,
                "content": flatten_tool_result_content(content),
                "is_error": is_error,
            })),
            InputContentBlock::ToolUse { .. } => None,
        }).collect(),
    }
}
```

**Response Normalization (Lines 767-814):**
```rust
fn normalize_response(model: &str, response: ChatCompletionResponse) -> Result<MessageResponse, ApiError> {
    let choice = response.choices.into_iter().next()
        .ok_or(ApiError::InvalidSseFrame("chat completion response missing choices"))?;
    
    let mut content = Vec::new();
    if let Some(text) = choice.message.content.filter(|value| !value.is_empty()) {
        content.push(OutputContentBlock::Text { text });
    }
    for tool_call in choice.message.tool_calls {
        content.push(OutputContentBlock::ToolUse {
            id: tool_call.id,
            name: tool_call.function.name,
            input: parse_tool_arguments(&tool_call.function.arguments),
        });
    }
    
    Ok(MessageResponse {
        id: response.id,
        kind: "message".to_string(),
        role: choice.message.role,
        content,
        model: response.model.if_empty_then(model.to_string()),
        stop_reason: choice.finish_reason.map(|v| normalize_finish_reason(&v)),
        stop_sequence: None,
        usage: Usage {
            input_tokens: response.usage.as_ref().map_or(0, |u| u.prompt_tokens),
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: response.usage.as_ref().map_or(0, |u| u.completion_tokens),
        },
        request_id: None,
    })
}
```

---

### sse.rs

#### Original (219 lines)

**SseParser (Lines 4-61):**
```rust
#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
}

impl SseParser {
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<StreamEvent>, ApiError> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();
        
        while let Some(frame) = self.next_frame() {
            if let Some(event) = parse_frame(&frame)? {
                events.push(event);
            }
        }
        
        Ok(events)
    }
    
    fn next_frame(&mut self) -> Option<String> {
        let separator = self.buffer.windows(2)
            .position(|window| window == b"\n\n")
            .map(|p| (p, 2))
            .or_else(|| {
                self.buffer.windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|p| (p, 4))
            })?;
        
        let (position, separator_len) = separator;
        let frame = self.buffer.drain(..position + separator_len).collect::<Vec<_>>();
        let frame_len = frame.len().saturating_sub(separator_len);
        Some(String::from_utf8_lossy(&frame[..frame_len]).into_owned())
    }
}
```

**parse_frame (Lines 63-101):**
```rust
pub fn parse_frame(frame: &str) -> Result<Option<StreamEvent>, ApiError> {
    let trimmed = frame.trim();
    if trimmed.is_empty() { return Ok(None); }
    
    let mut data_lines = Vec::new();
    let mut event_name: Option<&str> = None;
    
    for line in trimmed.lines() {
        if line.starts_with(':') { continue; }  // Comment line
        if let Some(name) = line.strip_prefix("event:") {
            event_name = Some(name.trim());
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            data_lines.push(data.trim_start());
        }
    }
    
    if matches!(event_name, Some("ping")) { return Ok(None); }
    if data_lines.is_empty() { return Ok(None); }
    
    let payload = data_lines.join("\n");
    if payload == "[DONE]" { return Ok(None); }
    
    serde_json::from_str::<StreamEvent>(&payload)
        .map(Some)
        .map_err(ApiError::from)
}
```

#### Latest (279 lines)

**Changes:**
- Added support for thinking/signature deltas
- Tests expanded to cover new delta types
- No structural changes to parser logic

---

### prompt_cache.rs (NEW in Latest - 734 lines)

**Purpose:** File-based prompt/response caching with fingerprint-based invalidation for reducing API costs.

**PromptCacheConfig (Lines 19-43):**
```rust
#[derive(Debug, Clone)]
pub struct PromptCacheConfig {
    pub session_id: String,
    pub completion_ttl: Duration,      // Default: 30 seconds
    pub prompt_ttl: Duration,          // Default: 5 minutes
    pub cache_break_min_drop: u32,     // Default: 2000 tokens
}

impl PromptCacheConfig {
    #[must_use]
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            completion_ttl: Duration::from_secs(DEFAULT_COMPLETION_TTL_SECS),
            prompt_ttl: Duration::from_secs(DEFAULT_PROMPT_TTL_SECS),
            cache_break_min_drop: DEFAULT_BREAK_MIN_DROP,
        }
    }
}
```

**PromptCachePaths (Lines 45-73):**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptCachePaths {
    pub root: PathBuf,
    pub session_dir: PathBuf,
    pub completion_dir: PathBuf,
    pub session_state_path: PathBuf,
    pub stats_path: PathBuf,
}

impl PromptCachePaths {
    #[must_use]
    pub fn for_session(session_id: &str) -> Self {
        let root = base_cache_root();
        let session_dir = root.join(sanitize_path_segment(session_id));
        let completion_dir = session_dir.join("completions");
        Self {
            root,
            session_state_path: session_dir.join("session-state.json"),
            stats_path: session_dir.join("stats.json"),
            session_dir,
            completion_dir,
        }
    }
}
```

**Cache Break Detection (Lines 314-382):**
```rust
fn detect_cache_break(
    config: &PromptCacheConfig,
    previous: Option<&TrackedPromptState>,
    current: &TrackedPromptState,
) -> Option<CacheBreakEvent> {
    let previous = previous?;
    
    // Fingerprint version change
    if previous.fingerprint_version != current.fingerprint_version {
        return Some(CacheBreakEvent {
            unexpected: false,
            reason: format!("fingerprint version changed (v{} -> v{})", 
                previous.fingerprint_version, current.fingerprint_version),
            previous_cache_read_input_tokens: previous.cache_read_input_tokens,
            current_cache_read_input_tokens: current.cache_read_input_tokens,
            token_drop: previous.cache_read_input_tokens
                .saturating_sub(current.cache_read_input_tokens),
        });
    }
    
    let token_drop = previous.cache_read_input_tokens
        .saturating_sub(current.cache_read_input_tokens);
    if token_drop < config.cache_break_min_drop {
        return None;  // Not enough change to matter
    }
    
    // Check what changed
    let mut reasons = Vec::new();
    if previous.model_hash != current.model_hash { reasons.push("model changed"); }
    if previous.system_hash != current.system_hash { reasons.push("system prompt changed"); }
    if previous.tools_hash != current.tools_hash { reasons.push("tool definitions changed"); }
    if previous.messages_hash != current.messages_hash { reasons.push("message payload changed"); }
    
    // Determine if expected or unexpected
    let (unexpected, reason) = if reasons.is_empty() {
        if elapsed > config.prompt_ttl.as_secs() {
            (false, format!("possible prompt cache TTL expiry after {elapsed}s"))
        } else {
            (true, "cache read tokens dropped while prompt fingerprint remained stable".to_string())
        }
    } else {
        (false, reasons.join(", "))
    };
    
    Some(CacheBreakEvent { unexpected, reason, ... })
}
```

---

## Key Types Summary

### Authentication

| Type | Original | Latest |
|------|----------|--------|
| AuthSource | ✓ | ✓ (in providers/anthropic.rs) |
| OAuthTokenSet | ✓ | ✓ |

### Providers

| Type | Original | Latest |
|------|----------|--------|
| AnthropicClient | ✓ | ✓ (moved to providers/) |
| ProviderClient | ✗ | ✓ (unified enum) |
| OpenAiCompatClient | ✗ | ✓ |
| ProviderKind | ✗ | ✓ |

### Caching

| Type | Original | Latest |
|------|----------|--------|
| PromptCache | ✗ | ✓ |
| PromptCacheConfig | ✗ | ✓ |
| PromptCacheStats | ✗ | ✓ |
| CacheBreakEvent | ✗ | ✓ |

---

## Differences: Original vs Latest

### File Count
| Category | Original | Latest | Delta |
|----------|----------|--------|-------|
| Source files | 5 | 8 | +3 |
| Lines of code | ~1,350 | ~2,800 | +1,450 |
| Test cases | ~20 | ~35 | +15 |

### New Features in Latest

1. **Multi-Provider Support**
   - Unified `ProviderClient` enum
   - xAI (Grok) support via `grok`, `grok-mini` aliases
   - OpenAI compatibility layer

2. **Prompt Caching**
   - File-based completion cache
   - Fingerprint-based invalidation
   - Cache break detection and reporting

3. **Enhanced Error Handling**
   - Context window preflight checks
   - Failure classification (`safe_failure_class()`)
   - Request ID tracking

4. **Thinking Content Blocks**
   - Support for Claude's thinking/reasoning output
   - Signature verification

5. **Telemetry Integration**
   - Session tracing exports
   - Analytics event types

### Breaking Changes

1. `ApiError::MissingApiKey` → `ApiError::MissingCredentials { provider, env_vars }`
2. `Usage::total_tokens()` now INCLUDES cache tokens
3. Client construction now uses `ProviderClient::from_model()` instead of `AnthropicClient::new()`

---

**End of API Crate Exploration**

See also:
- [../04_runtime_crate/runtime-exploration.md](../04_runtime_crate/runtime-exploration.md)
- [../05_rusty-claude-cli_crate/rusty-claude-cli-exploration.md](../05_rusty-claude-cli_crate/rusty-claude-cli-exploration.md)
