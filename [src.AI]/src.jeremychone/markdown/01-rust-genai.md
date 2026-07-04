# rust-genai: AI Provider Client Library

`genai` is a Rust client library for 19+ AI providers with a unified API for chat, embeddings, and streaming.

Source: `rust-genai/src/` — 10+ modules.

## Module Structure

```
rust-genai/src/
├── lib.rs            # Module exports: Client, Error, Result
├── error.rs          # Unified error type
├── support.rs        # Shared utilities
├── client/           # Client construction and configuration
│   ├── mod.rs
│   ├── builder.rs    # ClientBuilder pattern
│   ├── client_types.rs  # Client struct (Arc-wrapped)
│   ├── client_impl.rs   # Client methods
│   ├── config.rs     # ClientConfig
│   ├── headers.rs    # HTTP headers
│   ├── model_spec.rs # Model specifications
│   ├── service_target.rs  # Service targets
│   └── web_config.rs # Web client config
├── chat/             # Chat request/response types
│   ├── mod.rs
│   ├── chat_request.rs    # ChatRequest struct
│   ├── chat_response.rs   # ChatResponse struct
│   ├── chat_message.rs    # ChatMessage, ChatRole
│   ├── chat_stream.rs     # Streaming support
│   ├── chat_options.rs    # ChatOptions
│   ├── message_content.rs # MessageContent types
│   ├── content_part/      # Content parts (text, image, etc.)
│   ├── tool/              # Tool definitions, ToolCall, ToolResponse
│   ├── usage.rs           # Token usage
│   ├── printer.rs         # Output formatting
│   └── binary.rs          # Binary content
├── adapter/          # Provider adapters
├── resolver/         # Model resolution
├── embed/            # Embedding support
└── webc/             # HTTP client (reqwest wrapper)
```

## Client

Source: `client/client_types.rs`

```rust
#[derive(Debug, Clone)]
pub struct Client {
    pub(super) inner: Arc<ClientInner>,
}

pub(super) struct ClientInner {
    pub(super) web_client: WebClient,
    pub(super) config: ClientConfig,
}
```

Arc-wrapped for cheap cloning. Built via `ClientBuilder`:

```rust
impl Default for Client {
    fn default() -> Self { Client::builder().build() }
}

impl Client {
    pub fn builder() -> ClientBuilder { ClientBuilder::default() }
}
```

## ChatRequest

Source: `chat/chat_request.rs`

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatRequest {
    pub system: Option<String>,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    pub tools: Option<Vec<Tool>>,
    pub previous_response_id: Option<String>,  // OpenAI stateful sessions
    pub store: Option<bool>,                     // OpenAI stateful sessions
}
```

### Constructors

```rust
impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self
    pub fn from_system(content: impl Into<String>) -> Self
    pub fn from_user(content: impl Into<String>) -> Self
    pub fn from_messages(messages: Vec<ChatMessage>) -> Self
}
```

Aha: `previous_response_id` and `store` fields enable OpenAI's stateful Responses API — the server caches conversation state so only new messages need to be sent, reducing token costs for multi-turn conversations.

## ChatMessage

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: MessageContent,
}

pub enum ChatRole {
    System, User, Assistant, Tool,
}
```

## Error Type

Source: `error.rs`

```rust
pub enum Error {
    // Provider-specific errors
    // Network errors
    // Serialization errors
}

pub type Result<T> = std::result::Result<T, Error>;
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
```

## Provider Adapters

Source: `adapter/`

Each provider (OpenAI, Anthropic, Google, Ollama, etc.) has an adapter that translates the unified `ChatRequest`/`ChatResponse` to/from the provider's native format.

## Cross-References

- [Overview](00-overview.md) — All 21 projects
- [rust-agentic](06-rust-agentic.md) — MCP agent framework (uses genai)

[Next → rust-rpc-router](02-rust-rpc-router.md)
