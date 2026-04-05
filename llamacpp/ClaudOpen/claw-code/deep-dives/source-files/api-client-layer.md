# API Client Layer Deep-Dive

A comprehensive analysis of the Anthropic API client implementation, including authentication, streaming, retry logic, and error handling.

## Table of Contents

1. [Overview](#overview)
2. [Client Structure](#client-structure)
3. [Authentication Flow](#authentication-flow)
4. [Request/Response Types](#requestresponse-types)
5. [Message Sending](#message-sending)
6. [SSE Streaming Parser](#sse-streaming-parser)
7. [Retry Logic](#retry-logic)
8. [Error Handling](#error-handling)
9. [Token Exchange and Refresh](#token-exchange-and-refresh)
10. [Stream Event Types](#stream-event-types)

---

## Overview

The `api` crate provides the HTTP client layer for communicating with Anthropic's Messages API. It handles:

- OAuth 2.0 authentication with PKCE
- Server-Sent Events (SSE) streaming
- Exponential backoff retry logic
- Comprehensive error categorization
- Token usage tracking

**Location**: `rust/crates/api/`

**Key Files**:
- `src/client.rs` - Main client implementation
- `src/types.rs` - Request/response type definitions
- `src/sse.rs` - SSE frame parser
- `src/error.rs` - Error types and traits
- `src/lib.rs` - Public API exports

---

## Client Structure

### AnthropicClient

```rust
pub struct AnthropicClient {
    http: reqwest::Client,
    base_url: String,
    auth: AuthMode,
    model: String,
    max_tokens: u32,
}

enum AuthMode {
    OAuth { tokens: OAuthTokenSet },
    ApiKey { key: String },
}
```

### Fields

| Field | Type | Purpose |
|-------|------|---------|
| `http` | `reqwest::Client` | HTTP client with connection pooling |
| `base_url` | `String` | API base URL (default: `https://api.anthropic.com`) |
| `auth` | `AuthMode` | Authentication credentials |
| `model` | `String` | Default model for requests |
| `max_tokens` | `u32` | Default max tokens per response |

### Constructor

```rust
impl AnthropicClient {
    pub fn new(auth: OAuthTokenSet) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(600))  // 10 minute timeout
                .build()
                .expect("failed to build HTTP client"),
            base_url: String::from("https://api.anthropic.com"),
            auth: AuthMode::OAuth { tokens: auth },
            model: String::from("claude-opus-4-6"),
            max_tokens: 4096,
        }
    }

    pub fn with_api_key(api_key: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(600))
                .build()
                .expect("failed to build HTTP client"),
            base_url: String::from("https://api.anthropic.com"),
            auth: AuthMode::ApiKey { key: api_key },
            model: String::from("claude-opus-4-6"),
            max_tokens: 4096,
        }
    }
}
```

---

## Authentication Flow

### OAuth Token Set

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,  // Unix timestamp
    pub scopes: Vec<String>,
}

impl OAuthTokenSet {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Consider expired 5 minutes early to avoid edge cases
        now >= self.expires_at.saturating_sub(300)
    }
}
```

### Authorization Header

```rust
fn build_auth_header(&self) -> Result<String, ApiError> {
    match &self.auth {
        AuthMode::OAuth { tokens } => {
            if tokens.is_expired() {
                // Trigger refresh before building header
                self.refresh_tokens()?;
            }
            Ok(format!("Bearer {}", tokens.access_token))
        }
        AuthMode::ApiKey { key } => {
            Ok(format!("Bearer {}", key))
        }
    }
}
```

### OAuth Headers for Token Exchange

```rust
async fn exchange_authorization_code(
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<OAuthTokenSet, ApiError> {
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];

    let response = CLIENT
        .post("https://auth.anthropic.com/oauth2/token")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(ApiError::OAuthError {
            message: format!("token exchange failed: {}", response.status()),
        });
    }

    let tokens: OAuthTokenSet = response.json().await?;
    Ok(tokens)
}
```

---

## Request/Response Types

### MessageRequest

```rust
#[derive(Debug, Clone, Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub messages: Vec<InputMessage>,
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    #[serde(rename = "max_tokens")]
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputMessage {
    pub role: String,  // "user" or "assistant"
    pub content: Vec<InputContentBlock>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ToolResultContent>,
    },
}
```

### MessageResponse

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub role: String,
    pub model: String,
    pub content: Vec<OutputContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
}
```

### ToolDefinition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
```

---

## Message Sending

### Non-Streaming Request

```rust
pub async fn send_message(&self, request: &MessageRequest) -> Result<MessageResponse, ApiError> {
    let url = format!("{}/v1/messages", self.base_url);

    let mut builder = self.http
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-api-key", self.get_api_key()?)
        .header("anthropic-version", "2024-01-01");

    if let AuthMode::OAuth { tokens } = &self.auth {
        builder = builder.header("Authorization", format!("Bearer {}", tokens.access_token));
    }

    let response = builder
        .json(request)
        .send()
        .await
        .map_err(|e| ApiError::NetworkError(e.to_string()))?;

    self.handle_response(response).await
}
```

### Streaming Request

```rust
pub async fn send_message_streaming(
    &self,
    request: &MessageRequest,
) -> Result<impl Stream<Item = Result<StreamEvent, ApiError>>, ApiError> {
    let mut streaming_request = request.clone();
    streaming_request.stream = Some(true);

    let url = format!("{}/v1/messages", self.base_url);

    let response = self.http
        .post(&url)
        .header("Content-Type", "application/json")
        .header("x-api-key", self.get_api_key()?)
        .header("anthropic-version", "2024-01-01")
        .header("Accept", "text/event-stream")
        .json(&streaming_request)
        .send()
        .await
        .map_err(|e| ApiError::NetworkError(e.to_string()))?;

    if !response.status().is_success() {
        return Err(self.handle_error_response(response).await);
    }

    let stream = response
        .bytes_stream()
        .map_err(|e| ApiError::NetworkError(e.to_string()))
        .and_then(|bytes| self.parse_sse_frame(bytes));

    Ok(stream)
}
```

---

## SSE Streaming Parser

### SseParser

```rust
pub struct SseParser {
    buffer: String,
}

impl SseParser {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Parse incoming bytes into SSE frames
    /// SSE frames are delimited by double newlines (\n\n or \r\n\r\n)
    pub fn parse(&mut self, bytes: bytes::Bytes) -> Vec<SseFrame> {
        let chunk = String::from_utf8_lossy(&bytes);
        self.buffer.push_str(&chunk);

        let mut frames = Vec::new();

        loop {
            // Look for frame delimiter
            if let Some(pos) = self.find_frame_boundary() {
                let frame_data = self.buffer[..pos].to_string();
                self.buffer = self.buffer[pos + 2..].to_string();  // Skip \n\n

                if let Some(frame) = self.parse_frame(&frame_data) {
                    frames.push(frame);
                }
            } else {
                break;
            }
        }

        frames
    }

    fn find_frame_boundary(&self) -> Option<usize> {
        // Check for \r\n\r\n first
        if let Some(pos) = self.buffer.find("\r\n\r\n") {
            return Some(pos);
        }
        // Then check for \n\n
        if let Some(pos) = self.buffer.find("\n\n") {
            return Some(pos);
        }
        None
    }

    fn parse_frame(&self, data: &str) -> Option<SseFrame> {
        let mut event_type = None;
        let mut event_data = None;

        for line in data.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim();
                match key {
                    "event" => event_type = Some(value.to_string()),
                    "data" => event_data = Some(value.to_string()),
                    _ => {}  // Ignore unknown fields
                }
            }
        }

        Some(SseFrame {
            event_type: event_type?,
            data: event_data?,
        })
    }
}
```

### SseFrame

```rust
#[derive(Debug, Clone)]
pub struct SseFrame {
    pub event_type: String,
    pub data: String,
}
```

### Parsing to StreamEvent

```rust
impl AnthropicClient {
    fn parse_sse_frame(&self, frame: SseFrame) -> Result<StreamEvent, ApiError> {
        match frame.event_type.as_str() {
            "message_start" => {
                let data: MessageStartData = serde_json::from_str(&frame.data)
                    .map_err(|e| ApiError::ParseError(e.to_string()))?;
                Ok(StreamEvent::MessageStart { message: data.message })
            }
            "content_block_start" => {
                let data: ContentBlockStartData = serde_json::from_str(&frame.data)
                    .map_err(|e| ApiError::ParseError(e.to_string()))?;
                Ok(StreamEvent::ContentBlockStart {
                    index: data.index,
                    content_block: data.content_block,
                })
            }
            "content_block_delta" => {
                let data: ContentBlockDeltaData = serde_json::from_str(&frame.data)
                    .map_err(|e| ApiError::ParseError(e.to_string()))?;
                Ok(StreamEvent::ContentBlockDelta {
                    index: data.index,
                    delta: data.delta,
                })
            }
            "content_block_stop" => {
                let data: ContentBlockStopData = serde_json::from_str(&frame.data)
                    .map_err(|e| ApiError::ParseError(e.to_string()))?;
                Ok(StreamEvent::ContentBlockStop { index: data.index })
            }
            "message_delta" => {
                let data: MessageDeltaData = serde_json::from_str(&frame.data)
                    .map_err(|e| ApiError::ParseError(e.to_string()))?;
                Ok(StreamEvent::MessageDelta {
                    delta: data.delta,
                    usage: data.usage,
                })
            }
            "message_stop" => Ok(StreamEvent::MessageStop),
            "error" => {
                let data: ErrorData = serde_json::from_str(&frame.data)
                    .map_err(|e| ApiError::ParseError(e.to_string()))?;
                Err(ApiError::StreamError {
                    message: data.error.message,
                })
            }
            other => Err(ApiError::ParseError(format!("unknown event type: {}", other))),
        }
    }
}
```

---

## Retry Logic

### Exponential Backoff

```rust
pub async fn send_message_with_retry(
    &self,
    request: &MessageRequest,
    max_retries: u32,
) -> Result<MessageResponse, ApiError> {
    let mut attempt = 0;
    let mut last_error: Option<ApiError> = None;

    while attempt < max_retries {
        match self.send_message(request).await {
            Ok(response) => return Ok(response),
            Err(error) => {
                if !error.is_retryable() {
                    return Err(error);
                }

                last_error = Some(error);
                attempt += 1;

                if attempt < max_retries {
                    // Exponential backoff: 2^(attempt-1) seconds
                    let delay_secs = 1_u64 << (attempt - 1);
                    tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
```

### Retryable Errors

```rust
impl ApiError {
    pub fn is_retryable(&self) -> bool {
        match self {
            // Network errors are always retryable
            ApiError::NetworkError(_) => true,

            // Rate limits are retryable with backoff
            ApiError::RateLimitExceeded { .. } => true,

            // API errors with 5xx status are retryable
            ApiError::ApiError { status, .. } if *status >= 500 => true,

            // Token expiration - retry after refresh
            ApiError::AuthenticationError { .. } => true,

            // All other errors are not retryable
            _ => false,
        }
    }
}
```

### Retry Configuration

```rust
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 60000,
            jitter: true,
        }
    }
}
```

---

## Error Handling

### ApiError Enum

```rust
#[derive(Debug, Clone)]
pub enum ApiError {
    /// Network connectivity issues
    NetworkError(String),

    /// HTTP response with error status
    ApiError {
        status: u16,
        message: String,
        error_type: Option<String>,
    },

    /// Invalid or missing authentication
    AuthenticationError {
        message: String,
    },

    /// Rate limit exceeded
    RateLimitExceeded {
        message: String,
        retry_after: Option<u64>,
    },

    /// Invalid request payload
    InvalidRequest {
        message: String,
        field: Option<String>,
    },

    /// Parsing SSE or JSON failed
    ParseError(String),

    /// Stream error from server
    StreamError {
        message: String,
    },

    /// OAuth token exchange failed
    OAuthError {
        message: String,
    },

    /// Token refresh failed
    TokenRefreshError {
        message: String,
    },

    /// Request timeout
    Timeout,
}
```

### Error Display Implementation

```rust
impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ApiError::ApiError { status, message, .. } => {
                write!(f, "API error ({}): {}", status, message)
            }
            ApiError::AuthenticationError { message } => {
                write!(f, "Authentication failed: {}", message)
            }
            ApiError::RateLimitExceeded { message, retry_after } => {
                write!(f, "Rate limit exceeded: {}", message)?;
                if let Some(secs) = retry_after {
                    write!(f, " (retry after {}s)", secs)?;
                }
                Ok(())
            }
            ApiError::InvalidRequest { message, field } => {
                write!(f, "Invalid request: {}", message)?;
                if let Some(field) = field {
                    write!(f, " (field: {})", field)?;
                }
                Ok(())
            }
            ApiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ApiError::StreamError { message } => write!(f, "Stream error: {}", message),
            ApiError::OAuthError { message } => write!(f, "OAuth error: {}", message),
            ApiError::TokenRefreshError { message } => {
                write!(f, "Token refresh failed: {}", message)
            }
            ApiError::Timeout => write!(f, "Request timed out"),
        }
    }
}
```

### Error Response Handling

```rust
async fn handle_error_response(&self, response: reqwest::Response) -> ApiError {
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();

    // Try to parse as Anthropic error format
    #[derive(Deserialize)]
    struct AnthropicError {
        error: ErrorDetails,
    }

    #[derive(Deserialize)]
    struct ErrorDetails {
        #[serde(rename = "type")]
        error_type: String,
        message: String,
    }

    if let Ok(error) = serde_json::from_str::<AnthropicError>(&body) {
        match status {
            401 | 403 => ApiError::AuthenticationError {
                message: error.error.message,
            },
            429 => ApiError::RateLimitExceeded {
                message: error.error.message,
                retry_after: None,  // Would need to parse Retry-After header
            },
            400 => ApiError::InvalidRequest {
                message: error.error.message,
                field: None,
            },
            _ => ApiError::ApiError {
                status,
                message: error.error.message,
                error_type: Some(error.error.error_type),
            },
        }
    } else {
        ApiError::ApiError {
            status,
            message: body,
            error_type: None,
        }
    }
}
```

---

## Token Exchange and Refresh

### Authorization Code Flow

```rust
pub struct OAuthFlow {
    client_id: String,
    redirect_uri: String,
}

impl OAuthFlow {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            redirect_uri: String::from("http://localhost:8477/callback"),
        }
    }

    /// Generate authorization URL with PKCE
    pub fn generate_auth_url(&self) -> (String, PkceCodePair) {
        let pkce = PkceCodePair::generate();

        let params = [
            ("client_id", &self.client_id),
            ("redirect_uri", &self.redirect_uri),
            ("response_type", "code"),
            ("code_challenge", &pkce.challenge),
            ("code_challenge_method", "S256"),
            ("scope", "org:read org:write"),
        ];

        let query = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(&params)
            .finish();

        (format!("https://auth.anthropic.com/oauth2/auth?{}", query), pkce)
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, code: &str, verifier: &str) -> Result<OAuthTokenSet, ApiError> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.redirect_uri),
            ("code_verifier", verifier),
            ("client_id", &self.client_id),
        ];

        let response = CLIENT
            .post("https://auth.anthropic.com/oauth2/token")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ApiError::OAuthError {
                message: format!("token exchange failed: {}", response.status()),
            });
        }

        let tokens: OAuthTokenSet = response.json().await?;
        Ok(tokens)
    }

    /// Refresh expired tokens
    pub async fn refresh_tokens(&self, refresh_token: &str) -> Result<OAuthTokenSet, ApiError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
        ];

        let response = CLIENT
            .post("https://auth.anthropic.com/oauth2/token")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ApiError::TokenRefreshError {
                message: format!("token refresh failed: {}", response.status()),
            });
        }

        let tokens: OAuthTokenSet = response.json().await?;
        Ok(tokens)
    }
}
```

### PKCE Implementation

```rust
#[derive(Debug, Clone)]
pub struct PkceCodePair {
    pub verifier: String,
    pub challenge: String,
}

impl PkceCodePair {
    pub fn generate() -> Self {
        // Generate 32-byte random verifier
        let mut verifier_bytes = [0u8; 32];
        getrandom::getrandom(&mut verifier_bytes).expect("failed to generate random bytes");

        // Base64url encode
        let verifier = base64_url::encode(&verifier_bytes);

        // SHA256 hash for challenge
        let mut hasher = sha2::Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finish();
        let challenge = base64_url::encode(&hash);

        Self { verifier, challenge }
    }
}
```

---

## Stream Event Types

### StreamEvent Enum

```rust
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Initial message metadata
    MessageStart {
        message: MessageStartData,
    },

    /// Content block beginning
    ContentBlockStart {
        index: usize,
        content_block: OutputContentBlock,
    },

    /// Delta within content block
    ContentBlockDelta {
        index: usize,
        delta: ContentBlockDelta,
    },

    /// Content block complete
    ContentBlockStop {
        index: usize,
    },

    /// Message delta (stop reason, final usage)
    MessageDelta {
        delta: MessageDelta,
        usage: Usage,
    },

    /// Message complete
    MessageStop,
}
```

### Supporting Types

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MessageStartData {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub role: String,
    pub model: String,
    pub content: Vec<OutputContentBlock>,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageDelta {
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
}
```

### Stream Event Iterator

```rust
pub struct StreamEventIterator {
    parser: SseParser,
    stream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: Vec<StreamEvent>,
    done: bool,
}

impl StreamEventIterator {
    pub fn new(stream: impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static) -> Self {
        Self {
            parser: SseParser::new(),
            stream: Box::pin(stream),
            buffer: Vec::new(),
            done: false,
        }
    }

    pub async fn next(&mut self) -> Option<Result<StreamEvent, ApiError>> {
        // Return buffered events first
        if let Some(event) = self.buffer.pop(0) {
            return Some(Ok(event));
        }

        if self.done {
            return None;
        }

        // Read next chunk from stream
        while let Some(chunk_result) = self.stream.next().await {
            let chunk = match chunk_result {
                Ok(bytes) => bytes,
                Err(e) => return Some(Err(ApiError::NetworkError(e.to_string()))),
            };

            // Parse SSE frames
            let frames = self.parser.parse(chunk);
            for frame in frames {
                match self.parse_sse_frame(frame) {
                    Ok(event) => {
                        if let StreamEvent::MessageStop = event {
                            self.done = true;
                            return Some(Ok(event));
                        }
                        self.buffer.push(event);
                    }
                    Err(e) => return Some(Err(e)),
                }
            }

            // Return first parsed event
            if let Some(event) = self.buffer.pop(0) {
                return Some(Ok(event));
            }
        }

        self.done = true;
        None
    }
}
```

---

## Related Files

| File | Purpose |
|------|---------|
| `rust/crates/api/src/client.rs` | Main client implementation |
| `rust/crates/api/src/types.rs` | Request/response type definitions |
| `rust/crates/api/src/sse.rs` | SSE frame parser |
| `rust/crates/api/src/error.rs` | Error types |
| `rust/crates/api/src/lib.rs` | Public exports |
| `rust/crates/runtime/src/oauth.rs` | OAuth flow integration |

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sse_frame_correctly() {
        let mut parser = SseParser::new();
        let frames = parser.parse(bytes::Bytes::from("event: message_start\ndata: {\"id\":\"123\"}\n\n"));

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event_type, "message_start");
        assert_eq!(frames[0].data, "{\"id\":\"123\"}");
    }

    #[test]
    fn handles_multiple_frames() {
        let mut parser = SseParser::new();
        let frames = parser.parse(bytes::Bytes::from(
            "event: start\ndata: first\n\n\nevent: stop\ndata: second\n\n"
        ));

        assert_eq!(frames.len(), 2);
    }

    #[tokio::test]
    async fn retry_logic_works() {
        // Mock server setup
        let mut server = mockito::Server::new_async().await;
        let mock = server.mock("POST", "/v1/messages")
            .expect(3)
            .respond_with_response(
                Response::new(503, "", "")
            )
            .create();

        let client = AnthropicClient::with_api_key("test".to_string());
        let request = MessageRequest { /* ... */ };

        let result = client.send_message_with_retry(&request, 3).await;
        assert!(result.is_err());
        mock.assert();
    }
}
```
