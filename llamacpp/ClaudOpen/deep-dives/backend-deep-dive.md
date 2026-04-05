# Backend Deep Dive: API Client, Streaming, and MCP

**Purpose:** Understanding how ClaudOpen communicates with the Anthropic API, handles streaming responses, and integrates MCP servers.

---

## Table of Contents

1. [Overview](#overview)
2. [HTTP Client Architecture](#http-client-architecture)
3. [Authentication System](#authentication-system)
4. [SSE Streaming Protocol](#sse-streaming-protocol)
5. [Request/Response Types](#requestresponse-types)
6. [MCP Protocol Implementation](#mcp-protocol-implementation)
7. [Session Persistence](#session-persistence)
8. [Token Tracking and Cost Estimation](#token-tracking-and-cost-estimation)
9. [Rust Implementation Details](#rust-implementation-details)
10. [Production Considerations](#production-considerations)

---

## Overview

The backend layer handles:

- **HTTP communication** with Anthropic's API
- **SSE streaming** for real-time response rendering
- **OAuth/API key authentication**
- **MCP server management**
- **Session persistence and compaction**
- **Token usage tracking**

### Core Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        Backend Layer                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ HTTP Client  │  │ SSE Parser   │  │ Auth Manager         │  │
│  │ (reqwest)    │  │ (custom)     │  │ (API key / OAuth)    │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ MCP Client   │  │ Session Store│  │ Usage Tracker        │  │
│  │ (stdio/SSE)  │  │ (JSON file)  │  │ (tokens/cost)        │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## HTTP Client Architecture

### AnthropicClient Structure

```rust
// api/src/client.rs
pub struct AnthropicClient {
    base_url: String,
    api_key: Option<String>,
    oauth_token: Option<OAuthTokenSet>,
    http_client: reqwest::blocking::Client,
}

impl AnthropicClient {
    pub fn new() -> Result<Self, ApiError> {
        let base_url = read_base_url()?;

        let http_client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(600))  // 10 minute timeout
            .build()?;

        Ok(Self {
            base_url,
            api_key: env::var("ANTHROPIC_API_KEY").ok(),
            oauth_token: load_oauth_credentials().ok(),
            http_client,
        })
    }

    pub fn stream(&mut self, request: MessageRequest) -> Result<Vec<AssistantEvent>, ApiError> {
        let url = format!("{}/v1/messages", self.base_url);

        let mut req = self.http_client.post(&url)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request);

        // Add auth header
        req = match (&self.api_key, &self.oauth_token) {
            (Some(key), _) => req.header("x-api-key", key),
            (_, Some(tokens)) => req.header("Authorization", format!("Bearer {}", tokens.access_token)),
            _ => return Err(ApiError::NoCredentials),
        };

        // Send and parse SSE
        let response = req.send()?;
        parse_sse_stream(response)
    }
}
```

### Connection Pooling

```rust
// Use a shared client for connection pooling
lazy_static! {
    static ref HTTP_CLIENT: reqwest::blocking::Client =
        reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(600))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .unwrap();
}
```

---

## Authentication System

### Auth Source Resolution

```rust
// api/src/client.rs
pub enum AuthSource {
    ApiKey(String),
    OAuth(OAuthTokenSet),
    Proxy,
}

pub fn resolve_startup_auth_source() -> AuthSource {
    // Priority 1: API key from environment
    if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
        return AuthSource::ApiKey(key);
    }

    // Priority 2: OAuth from credentials file
    if let Ok(tokens) = load_oauth_credentials() {
        return AuthSource::OAuth(tokens);
    }

    // Priority 3: Proxy (for managed environments)
    AuthSource::Proxy
}
```

### OAuth Flow

```rust
// runtime/src/oauth.rs
pub struct OAuthConfig {
    pub client_id: String,
    pub authorize_url: String,
    pub token_url: String,
    pub callback_port: Option<u16>,
    pub scopes: Vec<String>,
}

pub struct OAuthTokenSet {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<SystemTime>,
}

// PKCE implementation
pub struct PkceCodePair {
    pub code_verifier: String,
    pub code_challenge: String,
}

pub fn generate_pkce_pair() -> Result<PkceCodePair, OAuthError> {
    let code_verifier = generate_random_string(64);
    let code_challenge = code_challenge_s256(&code_verifier)?;

    Ok(PkceCodePair {
        code_verifier,
        code_challenge,
    })
}

// OAuth authorization URL
pub struct OAuthAuthorizationRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
}

impl OAuthAuthorizationRequest {
    pub fn build_url(&self) -> String {
        let mut url = Url::parse(&self.authorize_url).unwrap();
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &self.scope)
            .append_pair("state", &self.state)
            .append_pair("code_challenge", &self.code_challenge)
            .append_pair("code_challenge_method", "S256");
        url.to_string()
    }
}

// Token exchange
pub async fn exchange_code_for_tokens(
    config: &OAuthConfig,
    code: &str,
    code_verifier: &str,
) -> Result<OAuthTokenSet, OAuthError> {
    let client = reqwest::Client::new();
    let response = client.post(&config.token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &config.redirect_uri),
            ("code_verifier", code_verifier),
            ("client_id", &config.client_id),
        ])
        .send()
        .await?;

    let tokens: OAuthTokenResponse = response.json().await?;
    Ok(tokens.into_token_set())
}

// Token refresh
pub fn oauth_token_is_expired(tokens: &OAuthTokenSet) -> bool {
    tokens.expires_at
        .map(|exp| exp <= SystemTime::now())
        .unwrap_or(false)
}

pub async fn refresh_oauth_token(
    config: &OAuthConfig,
    refresh_token: &str,
) -> Result<OAuthTokenSet, OAuthError> {
    let client = reqwest::Client::new();
    let response = client.post(&config.token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &config.client_id),
        ])
        .send()
        .await?;

    let tokens: OAuthTokenResponse = response.json().await?;
    Ok(tokens.into_token_set())
}
```

---

## SSE Streaming Protocol

### Server-Sent Events Format

```
event: message_start
data: {"type":"message_start","message":{"id":"msg_123",...}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":10}}

event: message_stop
data: {"type":"message_stop"}
```

### SSE Parser

```rust
// api/src/sse.rs
pub struct SseParser {
    buffer: String,
}

#[derive(Debug)]
pub struct SseFrame {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry: Option<u64>,
}

impl SseParser {
    pub fn new() -> Self {
        Self { buffer: String::new() }
    }

    pub fn push(&mut self, chunk: &str) -> Vec<SseFrame> {
        self.buffer.push_str(chunk);
        let mut frames = Vec::new();

        // Split by double newline
        let lines: Vec<&str> = self.buffer.split("\n\n").collect();

        // Keep the last incomplete chunk in buffer
        self.buffer = lines.last().unwrap_or(&"").to_string();

        for chunk in lines.iter().take(lines.len() - 1) {
            if let Some(frame) = self.parse_frame(chunk) {
                frames.push(frame);
            }
        }

        frames
    }

    fn parse_frame(&self, chunk: &str) -> Option<SseFrame> {
        let mut event = None;
        let mut data = String::new();
        let mut id = None;
        let mut retry = None;

        for line in chunk.lines() {
            if let Some(rest) = line.strip_prefix("event: ") {
                event = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("data: ") {
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(rest);
            } else if let Some(rest) = line.strip_prefix("id: ") {
                id = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("retry: ") {
                retry = rest.parse().ok();
            }
        }

        if data.is_empty() && event.is_none() {
            return None;
        }

        Some(SseFrame { event, data, id, retry })
    }
}
```

### Event Type Mapping

```rust
// api/src/types.rs
#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart(MessageStartEvent),
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
    MessageDelta(MessageDeltaEvent),
    MessageStop(MessageStopEvent),
}

pub struct MessageStartEvent {
    pub message: MessageResponse,
}

pub struct ContentBlockStartEvent {
    pub index: u32,
    pub content_block: OutputContentBlock,
}

pub struct ContentBlockDeltaEvent {
    pub index: u32,
    pub delta: ContentBlockDelta,
}

pub struct ContentBlockStopEvent {
    pub index: u32,
}

pub struct MessageDeltaEvent {
    pub delta: MessageDelta,
    pub usage: Usage,
}

pub struct MessageStopEvent {}

// Parse SSE to StreamEvent
pub fn parse_stream_event(frame: &SseFrame) -> Result<StreamEvent, ApiError> {
    let event_type = frame.event.as_deref().unwrap_or("message");
    let data: serde_json::Value = serde_json::from_str(&frame.data)?;

    match event_type {
        "message_start" => {
            let message = serde_json::from_value(data["message"].clone())?;
            Ok(StreamEvent::MessageStart(MessageStartEvent { message }))
        }
        "content_block_start" => {
            let index = data["index"].as_u64().unwrap_or(0) as u32;
            let content_block = serde_json::from_value(data["content_block"].clone())?;
            Ok(StreamEvent::ContentBlockStart(ContentBlockStartEvent { index, content_block }))
        }
        "content_block_delta" => {
            let index = data["index"].as_u64().unwrap_or(0) as u32;
            let delta = serde_json::from_value(data["delta"].clone())?;
            Ok(StreamEvent::ContentBlockDelta(ContentBlockDeltaEvent { index, delta }))
        }
        "content_block_stop" => {
            let index = data["index"].as_u64().unwrap_or(0) as u32;
            Ok(StreamEvent::ContentBlockStop(ContentBlockStopEvent { index }))
        }
        "message_delta" => {
            let delta = serde_json::from_value(data["delta"].clone())?;
            let usage = serde_json::from_value(data["usage"].clone())?;
            Ok(StreamEvent::MessageDelta(MessageDeltaEvent { delta, usage }))
        }
        "message_stop" => {
            Ok(StreamEvent::MessageStop(MessageStopEvent {}))
        }
        _ => Err(ApiError::UnknownEvent(event_type.to_string())),
    }
}
```

---

## Request/Response Types

### MessageRequest

```rust
// api/src/types.rs
#[derive(Debug, Clone, Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub messages: Vec<InputMessage>,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub stream: bool,
    pub tools: Option<Vec<ToolDefinition>>,
    pub tool_choice: Option<ToolChoice>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
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
        content: Vec<ToolResultContentBlock>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Any,
    Tool { name: String },
}
```

### MessageResponse

```rust
// api/src/types.rs
#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub role: String,
    pub content: Vec<OutputContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputContentBlock {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
}

impl Usage {
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
            + self.cache_creation_input_tokens.unwrap_or(0)
            + self.cache_read_input_tokens.unwrap_or(0)
    }
}
```

---

## MCP Protocol Implementation

### MCP Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     MCP Server Manager                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │  Server 1   │  │  Server 2   │  │  Server 3   │             │
│  │  (stdio)    │  │  (stdio)    │  │  (stdio)    │             │
│  │  npm run fs │  │  git server │  │  db server  │             │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘             │
│         │                │                │                     │
│         └────────────────┴────────────────┘                     │
│                          │                                      │
│                          ▼                                      │
│              ┌───────────────────────┐                         │
│              │  JSON-RPC 2.0 Client  │                         │
│              └───────────────────────┘                         │
└─────────────────────────────────────────────────────────────────┘
```

### MCP Configuration

```rust
// runtime/src/config.rs
#[derive(Debug, Clone)]
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpRemoteServerConfig),
    Http(McpRemoteServerConfig),
    Ws(McpWebSocketServerConfig),
    Sdk(McpSdkServerConfig),
    ClaudeAiProxy(McpClaudeAiProxyServerConfig),
}

#[derive(Debug, Clone)]
pub struct McpStdioServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}
```

### JSON-RPC 2.0 Protocol

```rust
// runtime/src/mcp_stdio.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T = serde_json::Value> {
    pub jsonrpc: String,  // Always "2.0"
    pub id: JsonRpcId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(u64),
    String(String),
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse<T = serde_json::Value> {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
```

### MCP Initialize Flow

```rust
pub async fn initialize_server(
    process: &mut McpStdioProcess,
    server_name: &str,
) -> Result<McpInitializeResult, McpServerManagerError> {
    let params = McpInitializeParams {
        protocol_version: "2024-11-05".to_string(),
        capabilities: json!({
            "roots": {"listChanged": true},
            "sampling": {}
        }),
        client_info: McpInitializeClientInfo {
            name: "claw".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    let request = JsonRpcRequest::new(
        JsonRpcId::Number(1),
        "initialize",
        Some(params),
    );

    let response = process.send_request(request).await?;

    if let Some(error) = response.error {
        return Err(McpServerManagerError::JsonRpc {
            server_name: server_name.to_string(),
            method: "initialize",
            error,
        });
    }

    let result: McpInitializeResult = response.result
        .ok_or_else(|| McpServerManagerError::InvalidResponse {
            server_name: server_name.to_string(),
            method: "initialize",
            details: "missing result".to_string(),
        })?
        .into();

    // Send initialized notification
    let initialized = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: JsonRpcId::Null,
        method: "notifications/initialized".to_string(),
        params: None,
    };
    process.send_notification(initialized).await?;

    Ok(result)
}
```

### Tool Listing and Execution

```rust
pub async fn list_tools(
    process: &mut McpStdioProcess,
    server_name: &str,
) -> Result<Vec<McpTool>, McpServerManagerError> {
    let params = McpListToolsParams { cursor: None };
    let request = JsonRpcRequest::new(
        JsonRpcId::Number(2),
        "tools/list",
        Some(params),
    );

    let response = process.send_request(request).await?;

    if let Some(error) = response.error {
        return Err(McpServerManagerError::JsonRpc {
            server_name: server_name.to_string(),
            method: "tools/list",
            error,
        });
    }

    let result: McpListToolsResult = response.result
        .ok_or_else(|| McpServerManagerError::InvalidResponse {
            server_name: server_name.to_string(),
            method: "tools/list",
            details: "missing result".to_string(),
        })?
        .into();

    Ok(result.tools)
}

pub async fn call_tool(
    process: &mut McpStdioProcess,
    server_name: &str,
    tool_name: &str,
    arguments: Option<serde_json::Value>,
) -> Result<McpToolCallResult, McpServerManagerError> {
    let params = McpToolCallParams {
        name: tool_name.to_string(),
        arguments,
        meta: None,
    };

    let request = JsonRpcRequest::new(
        JsonRpcId::Number(3),
        "tools/call",
        Some(params),
    );

    let response = process.send_request(request).await?;

    if let Some(error) = response.error {
        return Err(McpServerManagerError::JsonRpc {
            server_name: server_name.to_string(),
            method: "tools/call",
            error,
        });
    }

    let result: McpToolCallResult = response.result
        .ok_or_else(|| McpServerManagerError::InvalidResponse {
            server_name: server_name.to_string(),
            method: "tools/call",
            details: "missing result".to_string(),
        })?
        .into();

    Ok(result)
}
```

### Spawning MCP stdio Processes

```rust
pub async fn spawn_mcp_stdio_process(
    command: &str,
    args: &[String],
    env: &BTreeMap<String, String>,
) -> Result<McpStdioProcess, io::Error> {
    let mut cmd = Command::new(command);
    cmd.args(args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);

    // Add environment variables
    for (key, value) in env {
        cmd.env(key, value);
    }

    let mut child = cmd.spawn()?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    Ok(McpStdioProcess {
        child: Some(child),
        stdin: Some(McpStdioWriter::new(stdin)),
        stdout: BufReader::new(stdout),
        request_id: 1,
    })
}
```

---

## Session Persistence

### Session Structure

```rust
// runtime/src/session.rs
#[derive(Debug, Clone)]
pub struct Session {
    pub version: u32,
    pub messages: Vec<ConversationMessage>,
}

#[derive(Debug, Clone)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub blocks: Vec<ContentBlock>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
    ToolResult {
        tool_use_id: String,
        tool_name: String,
        output: String,
        is_error: bool,
    },
}
```

### JSON Serialization

```rust
impl Session {
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();
        object.insert(
            "version".to_string(),
            JsonValue::Number(i64::from(self.version)),
        );
        object.insert(
            "messages".to_string(),
            JsonValue::Array(
                self.messages
                    .iter()
                    .map(ConversationMessage::to_json)
                    .collect(),
            ),
        );
        JsonValue::Object(object)
    }

    pub fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value.as_object()
            .ok_or_else(|| SessionError::Format("session must be an object".to_string()))?;

        let version = object.get("version")
            .and_then(JsonValue::as_i64)
            .ok_or_else(|| SessionError::Format("missing version".to_string()))?;

        let messages = object.get("messages")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| SessionError::Format("missing messages".to_string()))?
            .iter()
            .map(ConversationMessage::from_json)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            version: u32::try_from(version)?,
            messages,
        })
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), SessionError> {
        fs::write(path, self.to_json().render())?;
        Ok(())
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, SessionError> {
        let contents = fs::read_to_string(path)?;
        Self::from_json(&JsonValue::parse(&contents)?)
    }
}
```

### Session Compaction

```rust
// runtime/src/compact.rs
pub struct CompactionConfig {
    pub max_estimated_tokens: u32,
    pub preserve_recent_messages: usize,
}

pub struct CompactionResult {
    pub compacted_session: Session,
    pub removed_message_count: usize,
    pub summary: String,
}

pub fn compact_session(
    session: &Session,
    config: CompactionConfig,
) -> CompactionResult {
    // Keep system message and recent messages
    let preserve_count = 1 + config.preserve_recent_messages;
    let messages_to_compact = if session.messages.len() > preserve_count {
        &session.messages[1..session.messages.len() - config.preserve_recent_messages]
    } else {
        return CompactionResult {
            compacted_session: session.clone(),
            removed_message_count: 0,
            summary: String::new(),
        };
    };

    // Generate summary
    let summary = generate_compaction_summary(messages_to_compact);

    // Build compacted session
    let mut compacted = Session::new();
    compacted.messages.push(ConversationMessage::system(summary.clone()));

    // Append preserved messages
    for msg in session.messages.iter()
        .skip(session.messages.len() - config.preserve_recent_messages)
    {
        compacted.messages.push(msg.clone());
    }

    CompactionResult {
        compacted_session: compacted,
        removed_message_count: messages_to_compact.len(),
        summary,
    }
}

pub fn should_compact(session: &Session, threshold: u32) -> bool {
    estimate_session_tokens(session) > threshold
}

pub fn estimate_session_tokens(session: &Session) -> usize {
    session.messages.iter().map(|msg| {
        msg.blocks.iter().map(|block| {
            match block {
                ContentBlock::Text { text } => text.len() / 4,
                ContentBlock::ToolUse { input, .. } => input.len() / 4,
                ContentBlock::ToolResult { output, .. } => output.len() / 4,
            }
        }).sum::<usize>()
    }).sum()
}
```

---

## Token Tracking and Cost Estimation

### Usage Tracker

```rust
// runtime/src/usage.rs
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

impl TokenUsage {
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
    }
}

#[derive(Debug, Default)]
pub struct UsageTracker {
    turns: u32,
    cumulative_usage: TokenUsage,
    turn_costs: Vec<TurnCost>,
}

#[derive(Debug)]
pub struct TurnCost {
    pub turn: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost_usd: f64,
}

impl UsageTracker {
    pub fn record(&mut self, usage: TokenUsage) {
        self.turns += 1;
        self.cumulative_usage.input_tokens += usage.input_tokens;
        self.cumulative_usage.output_tokens += usage.output_tokens;
        self.cumulative_usage.cache_creation_input_tokens += usage.cache_creation_input_tokens;
        self.cumulative_usage.cache_read_input_tokens += usage.cache_read_input_tokens;

        let cost = estimate_cost(&usage, "claude-sonnet-4-6");
        self.turn_costs.push(TurnCost {
            turn: self.turns,
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            estimated_cost_usd: cost,
        });
    }

    pub fn cumulative_usage(&self) -> TokenUsage {
        self.cumulative_usage.clone()
    }

    pub fn turns(&self) -> u32 {
        self.turns
    }

    pub fn total_cost(&self) -> f64 {
        self.turn_costs.iter().map(|tc| tc.estimated_cost_usd).sum()
    }
}
```

### Pricing Table

```rust
pub struct ModelPricing {
    pub input_price_per_1m: f64,
    pub output_price_per_1m: f64,
    pub cache_write_price_per_1m: f64,
    pub cache_read_price_per_1m: f64,
}

pub fn pricing_for_model(model: &str) -> ModelPricing {
    match model {
        m if m.contains("opus") => ModelPricing {
            input_price_per_1m: 15.0,
            output_price_per_1m: 75.0,
            cache_write_price_per_1m: 18.75,
            cache_read_price_per_1m: 1.50,
        },
        m if m.contains("sonnet") => ModelPricing {
            input_price_per_1m: 3.0,
            output_price_per_1m: 15.0,
            cache_write_price_per_1m: 3.75,
            cache_read_price_per_1m: 0.30,
        },
        m if m.contains("haiku") => ModelPricing {
            input_price_per_1m: 0.80,
            output_price_per_1m: 4.0,
            cache_write_price_per_1m: 1.0,
            cache_read_price_per_1m: 0.08,
        },
        _ => ModelPricing {
            input_price_per_1m: 3.0,
            output_price_per_1m: 15.0,
            cache_write_price_per_1m: 3.75,
            cache_read_price_per_1m: 0.30,
        },
    }
}

pub fn estimate_cost(usage: &TokenUsage, model: &str) -> f64 {
    let pricing = pricing_for_model(model);

    let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * pricing.input_price_per_1m;
    let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * pricing.output_price_per_1m;
    let cache_write_cost = (usage.cache_creation_input_tokens as f64 / 1_000_000.0)
        * pricing.cache_write_price_per_1m;
    let cache_read_cost = (usage.cache_read_input_tokens as f64 / 1_000_000.0)
        * pricing.cache_read_price_per_1m;

    input_cost + output_cost + cache_write_cost + cache_read_cost
}

pub fn format_usd(amount: f64) -> String {
    format!("${:.4}", amount)
}
```

---

## Rust Implementation Details

### Error Handling

```rust
// api/src/error.rs
#[derive(Debug)]
pub enum ApiError {
    Http(reqwest::Error),
    Json(serde_json::Error),
    Io(io::Error),
    Authentication { message: String },
    RateLimit { retry_after: Option<Duration> },
    ServerError { status: u16, message: String },
    UnknownEvent(String),
    NoCredentials,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Http(e) => write!(f, "HTTP error: {}", e),
            ApiError::Json(e) => write!(f, "JSON error: {}", e),
            ApiError::Authentication { message } => write!(f, "Authentication failed: {}", message),
            ApiError::RateLimit { retry_after } => {
                write!(f, "Rate limit exceeded")?;
                if let Some(duration) = retry_after {
                    write!(f, ", retry after {}s", duration.as_secs())?;
                }
                Ok(())
            }
            ApiError::ServerError { status, message } => {
                write!(f, "Server error ({}): {}", status, message)
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        ApiError::Http(error)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        ApiError::Json(error)
    }
}

impl From<io::Error> for ApiError {
    fn from(error: io::Error) -> Self {
        ApiError::Io(error)
    }
}
```

### Async vs Blocking

```rust
// Current implementation uses blocking client for simplicity
use reqwest::blocking::{Client, Response};

// For production, consider async with tokio
use reqwest::{Client, Response};
use tokio::io::AsyncBufReadExt;

pub async fn stream_async(
    client: &Client,
    url: &str,
    request: &MessageRequest,
) -> Result<Vec<AssistantEvent>, ApiError> {
    let response = client.post(url).json(request).send().await?;

    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();
    let mut events = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let text = String::from_utf8_lossy(&chunk);
        let frames = parser.push(&text);

        for frame in frames {
            if let Some(event) = parse_stream_event(&frame)? {
                events.push(event);
            }
        }
    }

    Ok(events)
}
```

---

## Production Considerations

### 1. Rate Limiting

```rust
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;

pub struct RateLimitedClient {
    client: AnthropicClient,
    limiter: RateLimiter,
}

impl RateLimitedClient {
    pub fn new() -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(60).unwrap());
        Self {
            client: AnthropicClient::new().unwrap(),
            limiter: RateLimiter::direct(quota),
        }
    }

    pub async fn stream(&self, request: MessageRequest) -> Result<Vec<AssistantEvent>, ApiError> {
        self.limiter.until_ready().await;
        self.client.stream(request).await
    }
}
```

### 2. Retry Logic

```rust
use retry::{delay::Exponential, retry};

pub fn stream_with_retry(
    client: &mut AnthropicClient,
    request: MessageRequest,
) -> Result<Vec<AssistantEvent>, ApiError> {
    retry(
        Exponential::from_millis(100).take(5),
        || match client.stream(request.clone()) {
            Ok(events) => Ok(events),
            Err(ApiError::RateLimit { retry_after }) => {
                if let Some(duration) = retry_after {
                    std::thread::sleep(duration);
                }
                Err(retry::Error::Transient(()))
            }
            Err(ApiError::ServerError { status, .. }) if status >= 500 => {
                Err(retry::Error::Transient(()))
            }
            Err(e) => Err(retry::Error::Permanent(e)),
        },
    )
}
```

### 3. Connection Pooling

```rust
use reqwest::blocking::Client;
use std::time::Duration;

pub fn create_pooled_client() -> Client {
    Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(600))
        .connect_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .unwrap()
}
```

### 4. Telemetry

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(self, request), fields(model = request.model))]
pub fn stream(&mut self, request: MessageRequest) -> Result<Vec<AssistantEvent>, ApiError> {
    let start = Instant::now();

    match self.stream_inner(request.clone()) {
        Ok(events) => {
            let duration = start.elapsed();
            info!(
                duration_ms = duration.as_millis(),
                events_count = events.len(),
                "API request completed"
            );
            Ok(events)
        }
        Err(e) => {
            error!(error = %e, "API request failed");
            Err(e)
        }
    }
}
```

---

## References

- [Anthropic API Documentation](https://docs.anthropic.com/claude/reference)
- [MCP Specification](https://modelcontextprotocol.io/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Server-Sent Events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events)

---

*Generated: 2026-04-02*
