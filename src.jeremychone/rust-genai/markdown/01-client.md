---
title: Client Builder and Configuration
---

# Client Builder and Configuration

**Client construction, configuration, and WebClient management.**

Source: `src/client/` — 7 files, builder.rs, client_types.rs, client_impl.rs, config.rs

## ClientBuilder

Source: `src/client/builder.rs:16-100`

```rust
#[derive(Debug, Default)]
pub struct ClientBuilder {
    web_client: Option<WebClient>,
    config: Option<ClientConfig>,
}

impl ClientBuilder {
    // -- Core setters
    pub fn with_reqwest(mut self, reqwest_client: reqwest::Client) -> Self
    pub fn with_config(mut self, config: ClientConfig) -> Self
    pub fn with_web_config(mut self, req_options: WebConfig) -> Self
    
    // -- ChatOptions passthrough
    pub fn with_chat_options(mut self, options: ChatOptions) -> Self
    
    // -- Resolver setters
    pub fn with_auth_resolver(mut self, auth_resolver: AuthResolver) -> Self
    pub fn with_auth_resolver_fn(mut self, auth_resolver_fn: impl IntoAuthResolverFn) -> Self
    pub fn with_service_target_resolver(mut self, target_resolver: ServiceTargetResolver) -> Self
    pub fn with_service_target_resolver_fn(mut self, target_resolver_fn: impl IntoServiceTargetResolverFn) -> Self
    pub fn with_model_mapper(mut self, model_mapper: ModelMapper) -> Self
    pub fn with_model_mapper_fn(mut self, model_mapper_fn: impl IntoModelMapperFn) -> Self
    
    pub fn build(self) -> Client
}
```

### Usage Example

```rust
let client = Client::builder()
    .with_chat_options(ChatOptions {
        max_tokens: Some(1024),
        temperature: Some(0.7),
    })
    .with_auth_resolver_fn(|adapter_kind, _model| {
        match adapter_kind {
            AdapterKind::OpenAI => env::var("OPENAI_API_KEY").ok(),
            AdapterKind::Anthropic => env::var("ANTHROPIC_API_KEY").ok(),
            _ => None,
        }
    })
    .build();
```

## Client and ClientInner

Source: `src/client/client_types.rs:12-60`

```rust
#[derive(Debug, Clone)]
pub struct Client {
    pub(super) inner: Arc<ClientInner>,
}

#[derive(Debug)]
pub(super) struct ClientInner {
    pub(super) web_client: WebClient,
    pub(super) config: ClientConfig,
}

impl Default for Client {
    fn default() -> Self {
        Client::builder().build()
    }
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }
    
    pub(crate) fn web_client(&self) -> &WebClient {
        &self.inner.web_client
    }
    
    pub(crate) fn config(&self) -> &ClientConfig {
        &self.inner.config
    }
}
```

**Aha:** Client is Arc-wrapped so cloning is cheap - `Arc::clone` just increments a reference count. This enables passing the client to multiple async tasks without ownership complexity.

## ClientConfig

Source: `src/client/config.rs`

Contains:

```rust
pub struct ClientConfig {
    pub web_config: Option<WebConfig>,
    pub chat_options: Option<ChatOptions>,
    pub auth_resolver: Option<AuthResolver>,
    pub service_target_resolver: Option<ServiceTargetResolver>,
    pub model_mapper: Option<ModelMapper>,
}
```

### ChatOptions

Per-request options that can be set at client level:

```rust
pub struct ChatOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
    pub capture_raw_body: Option<bool>,
}
```

### WebConfig

HTTP client configuration:

```rust
pub struct WebConfig {
    pub timeout: Option<Duration>,
    pub connect_timeout: Option<Duration>,
}
```

## WebClient

Source: `src/webc/` — reqwest wrapper for HTTP requests

```rust
pub struct WebClient {
    // wraps reqwest::Client
}
```

**Aha:** WebClient abstracts reqwest, allowing custom client configuration (timeouts, proxies, connection pooling). Shared across all requests via Arc<ClientInner>.

## Resolver System

Three resolver types for dynamic configuration:

### AuthResolver

Provides API keys per adapter:

```rust
pub struct AuthResolver {
    // Returns API key for adapter_kind
}
```

### ServiceTargetResolver

Maps models to endpoints:

```rust
pub struct ServiceTargetResolver {
    // Returns ServiceTarget (base URL + headers) for model
}
```

**Aha:** ServiceTargetResolver allows routing requests to custom endpoints - e.g., `openai::custom-model` → your own OpenAI-compatible server, or `vertex::gemini-2.5-flash` → Vertex AI endpoint with different auth.

### ModelMapper

Transforms model names:

```rust
pub struct ModelMapper {
    // Maps "gpt-4o" → "gpt-4o-2024-08-06" etc.
}
```

## Next

→ [02-chat-types.md](02-chat-types.md) — ChatMessage, MessageContent, Tools
