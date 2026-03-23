---
source: Model Context Protocol (MCP)
repository: https://github.com/modelcontextprotocol
revised_at: 2026-03-23T00:00:00Z
workspace: mcp-rust
---

# Rust Revision: Model Context Protocol (MCP)

## Overview

This guide provides a comprehensive roadmap for reproducing the Model Context Protocol (MCP) in Rust at production level. The implementation covers the full protocol specification including server/client architecture, tool and resource exposure patterns, and all protocol features.

**Key Design Decisions:**
- Use existing RMCP SDK as the foundation (official Rust implementation)
- Extend with custom features as needed
- Follow idiomatic Rust patterns with strong typing
- Leverage Tokio for async runtime
- Use serde for serialization with JSON Schema generation

## Workspace Structure

```
mcp-rust/
├── Cargo.toml                      # Workspace root
├── README.md
├── LICENSE
│
├── crates/
│   ├── mcp-core/                   # Core protocol types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── protocol.rs         # Protocol version, types
│   │       ├── capabilities.rs     # Client/Server capabilities
│   │       ├── jsonrpc.rs          # JSON-RPC message types
│   │       └── error.rs            # Error types
│   │
│   ├── mcp-server/                 # Server SDK
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── handler.rs          # ServerHandler trait
│   │       ├── router.rs           # Tool/Prompt routers
│   │       ├── tool.rs             # Tool definitions
│   │       ├── resource.rs         # Resource handling
│   │       ├── prompt.rs           # Prompt templates
│   │       └── transport/          # Server transports
│   │           ├── stdio.rs
│   │           ├── http.rs
│   │           └── websocket.rs
│   │
│   ├── mcp-client/                 # Client SDK
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── handler.rs          # ClientHandler trait
│   │       ├── sampling.rs         # LLM sampling
│   │       ├── roots.rs            # Root management
│   │       └── transport/          # Client transports
│   │
│   ├── mcp-macros/                 # Procedural macros
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── tool.rs             # #[tool] macro
│   │       ├── prompt.rs           # #[prompt] macro
│   │       └── router.rs           # #[router] macro
│   │
│   └── mcp-transports/             # Shared transport utilities
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── streamable_http.rs  # Streamable HTTP protocol
│           ├── sse.rs              # Server-Sent Events
│           └── auth.rs             # OAuth 2.0 authentication
│
├── examples/
│   ├── server-basic/               # Basic server example
│   ├── server-http/                # HTTP server example
│   ├── client-basic/               # Basic client example
│   └── server-oauth/               # OAuth-protected server
│
└── tests/
    ├── conformance/                # Protocol conformance tests
    └── integration/                # Integration tests
```

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| **Async Runtime** | tokio | 1.0 | Full-featured async runtime with TCP/UDP support |
| **Serialization** | serde + serde_json | 1.0 | Industry standard for JSON serialization |
| **JSON Schema** | schemars | 0.8 | Automatic JSON Schema generation from types |
| **Error Handling** | thiserror | 2.0 | Derive macros for custom error types |
| **Logging** | tracing + tracing-subscriber | 0.1 | Async-aware structured logging |
| **HTTP Server** | axum | 0.8 | Ergonomic HTTP framework |
| **HTTP Client** | reqwest | 0.12 | Async HTTP client |
| **WebSocket** | tokio-tungstenite | 0.26 | WebSocket implementation |
| **SSE** | sse-stream | 0.1 | Server-Sent Events parsing |
| **OAuth 2.0** | oauth2 | 5.0 | OAuth 2.0 client/server implementation |
| **JWT** | jsonwebtoken | 10 | JWT encoding/decoding |
| **UUID** | uuid | 1.0 | Session ID generation |
| **Futures** | futures | 0.3 | Async utilities and combinators |
| **Pin Project** | pin-project-lite | 0.2 | Zero-cost futures pinning |

## Type System Design

### Core Protocol Types

```rust
/// Protocol version identifier
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ProtocolVersion(Cow<'static, str>);

impl ProtocolVersion {
    pub const V_2024_11_05: Self = Self(Cow::Borrowed("2024-11-05"));
    pub const V_2025_03_26: Self = Self(Cow::Borrowed("2025-03-26"));
    pub const V_2025_06_18: Self = Self(Cow::Borrowed("2025-06-18"));
    pub const LATEST: Self = Self::V_2025_06_18;
}

/// JSON-RPC request ID
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum RequestId {
    Number(i64),
    String(Arc<str>),
}

/// Progress tracking token
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ProgressToken(pub RequestId);

/// Pagination cursor
pub type Cursor = String;
```

### JSON-RPC Message Types

```rust
/// JSON-RPC message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
    Response(JsonRpcResponse),
    ErrorResponse(JsonRpcErrorResponse),
}

/// JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: ConstString<"2.0">,
    pub id: RequestId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<JsonObject>,
}

/// JSON-RPC notification (no response expected)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: ConstString<"2.0">,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<JsonObject>,
}

/// JSON-RPC success response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: ConstString<"2.0">,
    pub id: RequestId,
    pub result: JsonObject,
}

/// JSON-RPC error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: ConstString<"2.0">,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    pub error: ErrorData,
}
```

### Error Types

```rust
/// MCP error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // JSON-RPC standard
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,

    // MCP-specific
    UrlElicitationRequired = -32042,
}

/// Error data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ErrorData {
    pub fn new(code: ErrorCode, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            code,
            message: message.into(),
            data,
        }
    }

    pub fn method_not_found<M: ConstString>() -> Self {
        Self::new(
            ErrorCode::MethodNotFound,
            format!("Method '{}' not found", M::VALUE),
            None,
        )
    }

    pub fn invalid_params(message: impl Into<String>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::InvalidParams, message, data)
    }

    pub fn internal_error(message: impl Into<String>, data: Option<Value>) -> Self {
        Self::new(ErrorCode::InternalError, message, data)
    }
}

pub type Result<T, E = ErrorData> = std::result::Result<T, E>;
```

### Capability Types

```rust
/// Server capabilities declaration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<JsonObject>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<CompletionCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TaskCapabilities>,
}

impl ServerCapabilities {
    pub fn builder() -> ServerCapabilitiesBuilder {
        ServerCapabilitiesBuilder::default()
    }
}

/// Builder pattern for capabilities
#[derive(Default)]
pub struct ServerCapabilitiesBuilder {
    logging: bool,
    completions: bool,
    prompts_list_changed: bool,
    resources_subscribe: bool,
    resources_list_changed: bool,
    tools_list_changed: bool,
}

impl ServerCapabilitiesBuilder {
    pub fn enable_logging(mut self) -> Self {
        self.logging = true;
        self
    }

    pub fn enable_completions(mut self) -> Self {
        self.completions = true;
        self
    }

    pub fn enable_prompts(mut self) -> Self {
        self.prompts_list_changed = true;
        self
    }

    pub fn enable_resources(mut self) -> Self {
        self.resources_list_changed = true;
        self
    }

    pub fn enable_resources_subscribe(mut self) -> Self {
        self.resources_subscribe = true;
        self.resources_list_changed = true;
        self
    }

    pub fn enable_tools(mut self) -> Self {
        self.tools_list_changed = true;
        self
    }

    pub fn build(self) -> ServerCapabilities {
        ServerCapabilities {
            logging: self.logging.then_some(LoggingCapabilities {}),
            completions: self.completions.then_some(CompletionCapabilities {}),
            prompts: self.prompts_list_changed.then_some(PromptCapabilities {
                list_changed: Some(true),
            }),
            resources: Some(ResourceCapabilities {
                subscribe: self.resources_subscribe,
                list_changed: Some(self.resources_list_changed),
            }),
            tools: self.tools_list_changed.then_some(ToolCapabilities {
                list_changed: Some(true),
            }),
            experimental: None,
            tasks: None,
        }
    }
}
```

### Tool Types

```rust
/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "inputSchema")]
    pub input_schema: JsonSchemaObject,

    #[serde(skip_serializing_if = "Option::is_none", rename = "outputSchema")]
    pub output_schema: Option<JsonSchemaObject>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,

    #[serde(skip_serializing_if = "Option::is_none", rename = "taskSupport")]
    pub task_support: Option<TaskSupport>,
}

/// JSON Schema object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchemaObject {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<std::collections::BTreeMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Tool annotations for UI hints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub read_only_hint: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub destructive_hint: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub idempotent_hint: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub open_world_hint: bool,
}

/// Task support declaration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskSupport {
    Required,
    Optional,
    Forbidden,
}
```

### Content Types

```rust
/// Content union type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    Text(TextContent),
    Image(ImageContent),
    Audio(AudioContent),
    Resource(ResourceLink),
}

impl Content {
    pub fn text(text: impl Into<String>) -> Self {
        Content::Text(TextContent { text: text.into() })
    }

    pub fn image(data: impl Into<Vec<u8>>, mime_type: impl Into<String>) -> Self {
        Content::Image(ImageContent {
            data: base64_encode(data.into()),
            mime_type: mime_type.into(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub data: String,  // base64 encoded
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioContent {
    pub data: String,  // base64 encoded
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLink {
    pub resource: ResourceContents,
}
```

## Key Rust-Specific Changes

### 1. ConstString Type for Fixed Values

**Source Pattern (TypeScript):**
```typescript
interface JSONRPCRequest {
  jsonrpc: "2.0";
}
```

**Rust Translation:**
```rust
pub trait ConstString: Default {
    const VALUE: &str;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JsonRpcVersion2_0;

impl ConstString for JsonRpcVersion2_0 {
    const VALUE: &str = "2.0";
}

impl Serialize for JsonRpcVersion2_0 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Self::VALUE.serialize(serializer)
    }
}
```

**Rationale:** Compile-time guarantee that the value is always "2.0" with zero runtime cost.

### 2. Type-State Pattern for Protocol Roles

**Source Pattern:**
```typescript
class Client extends Protocol { ... }
class Server extends Protocol { ... }
```

**Rust Translation:**
```rust
pub trait ServiceRole {
    type Req: TransferObject;
    type Resp: TransferObject;
    type PeerReq: TransferObject;
    type PeerResp: TransferObject;
    const IS_CLIENT: bool;
}

#[derive(Debug, Clone, Copy)]
pub struct RoleClient;
#[derive(Debug, Clone, Copy)]
pub struct RoleServer;

impl ServiceRole for RoleClient {
    type Req = ClientRequest;
    type Resp = ClientResult;
    type PeerReq = ServerRequest;
    type PeerResp = ServerResult;
    const IS_CLIENT: bool = true;
}

impl ServiceRole for RoleServer {
    type Req = ServerRequest;
    type Resp = ServerResult;
    type PeerReq = ClientRequest;
    type PeerResp = ClientResult;
    const IS_CLIENT: bool = false;
}
```

**Rationale:** Type-level guarantee of client/server role safety with shared protocol implementation.

### 3. Future-Based Async Trait Pattern

**Source Pattern:**
```typescript
interface ServerHandler {
    callTool(params: CallToolParams): Promise<CallToolResult>;
}
```

**Rust Translation:**
```rust
pub trait ServerHandler: Sized + Send + Sync + 'static {
    fn call_tool(
        &self,
        params: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, ErrorData>> + Send + '_;
}

// Using async-trait for boxed futures when needed
#[async_trait]
pub trait DynServerHandler: Send + Sync + 'static {
    async fn call_tool(
        &self,
        params: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData>;
}
```

**Rationale:** Zero-cost abstraction with `impl Future` for most cases, boxed futures for dynamic dispatch.

### 4. Router Pattern with Procedural Macros

**Source Pattern:**
```typescript
server.tool('greet', handler);
server.prompt('review', promptHandler);
```

**Rust Translation:**
```rust
#[tool_router]
impl MyServer {
    #[tool(name = "greet", description = "Greet someone")]
    async fn greet(&self, Parameters(params): Parameters<GreetParams>) -> Result<String, McpError> {
        Ok(format!("Hello, {}!", params.name))
    }
}

#[tool_handler]
impl ServerHandler for MyServer {
    fn get_info(&self) -> ServerInfo { ... }
}
```

**Rationale:** Compile-time code generation for type-safe routing with minimal boilerplate.

## Ownership & Borrowing Strategy

### Service as Arc

```rust
use std::sync::Arc;

#[derive(Clone)]
pub struct MyServer {
    state: Arc<Mutex<State>>,
    tool_router: ToolRouter<Self>,
}

// Clone is cheap - just increments Arc
let server1 = MyServer::new();
let server2 = server1.clone();  // Shares state
```

### Request Context with Lifetime

```rust
pub struct RequestContext<'a, R: ServiceRole> {
    pub peer: &'a Peer<R>,
    pub request_id: RequestId,
    pub extensions: Extensions,
}

// Borrowed peer reference avoids unnecessary cloning
async fn handle(
    &self,
    ctx: RequestContext<'_, RoleServer>,
) -> Result<CallToolResult, ErrorData> {
    // Can send notifications via ctx.peer
    ctx.peer.notify_progress(...).await?;
    Ok(result)
}
```

### Parameter Extraction

```rust
// Wrapper type for parameter extraction
pub struct Parameters<T>(pub T);

// Extractor pattern for handlers
#[tool(description = "Add two numbers")]
async fn add(
    &self,
    Parameters(AddParams { a, b }): Parameters<AddParams>,
) -> Result<i32, ErrorData> {
    Ok(a + b)
}
```

## Concurrency Model

**Approach:** Async with Tokio runtime

**Rationale:**
- MCP is inherently I/O-bound (network, subprocess communication)
- Tokio provides best-in-class async primitives
- Multiple concurrent client connections
- Non-blocking tool execution

```rust
use tokio::sync::{Mutex, RwLock, broadcast, mpsc};

// Shared state with async mutex
pub struct ServerState {
    counter: Mutex<i32>,
    subscribers: RwLock<HashSet<String>>,
    notifications: broadcast::Sender<Notification>,
}

// Spawn background tasks
tokio::spawn(async move {
    // Long-running operation
});

// Concurrent request handling
let (tx, mut rx) = mpsc::channel(100);
while let Some(request) = rx.recv().await {
    // Process concurrently
}
```

## Memory Considerations

### Stack vs. Heap

- Small fixed-size types on stack: `RequestId`, `ProgressToken`, `ErrorCode`
- Large types on heap via `Box`, `Arc`, `Vec`: `JsonObject`, `Content`, `Tool`
- String interning for repeated strings: `Arc<str>` for method names

### Smart Pointer Usage

```rust
// For shared immutable data
type MethodName = Arc<str>;

// For mutable shared state
state: Arc<Mutex<ServerState>>

// For owned heap data
boxed_error: Box<dyn std::error::Error + Send + Sync>
```

### No Unsafe Code Required

The implementation can be entirely safe Rust - no FFI or low-level memory manipulation needed.

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| **Invalid JSON** | `serde_json` returns descriptive error |
| **Missing required field** | Type system enforces presence at compile time |
| **Unknown method** | `METHOD_NOT_FOUND` error with method name |
| **Concurrent modification** | `Mutex`/`RwLock` ensures exclusive access |
| **Channel disconnect** | `Result` from send/recv operations |
| **Timeout** | `tokio::time::timeout()` wrapper |
| **Cancellation** | `CancellationToken` propagation |
| **Invalid URI** | `url::Url` parsing with error handling |

## Code Examples

### Complete Server Implementation

```rust
use mcp_server::{
    ServerHandler, ServiceExt, model::*,
    tool, tool_handler, tool_router,
    handler::server::wrapper::Parameters,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

// Tool arguments
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AddParams {
    /// First number
    pub a: i32,
    /// Second number
    pub b: i32,
}

// Server with shared state
#[derive(Clone)]
pub struct CalculatorServer {
    history: Arc<Mutex<Vec<(i32, i32, i32)>>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CalculatorServer {
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(vec![])),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "add",
        description = "Add two numbers together"
    )]
    async fn add(
        &self,
        Parameters(params): Parameters<AddParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let result = params.a + params.b;

        // Record in history
        self.history.lock().await.push((params.a, params.b, result));

        Ok(CallToolResult::success(vec![
            Content::text(format!("{} + {} = {}", params.a, params.b, result))
        ]))
    }

    #[tool(description = "Get calculation history")]
    async fn history(&self) -> Result<CallToolResult, ErrorData> {
        let history = self.history.lock().await;
        let history_text = history.iter()
            .map(|(a, b, r)| format!("{} + {} = {}", a, b, r))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CallToolResult::success(vec![
            Content::text(if history_text.is_empty() {
                "No calculations yet".to_string()
            } else {
                history_text
            })
        ]))
    }
}

#[tool_handler]
impl ServerHandler for CalculatorServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build()
        )
        .with_protocol_version(ProtocolVersion::LATEST)
        .with_server_info(Implementation {
            name: "calculator-server".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        })
        .with_instructions(
            "A calculator server that performs basic arithmetic.".to_string()
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::init();

    let server = CalculatorServer::new();
    let transport = stdio();
    let service = server.serve(transport).await?;

    tracing::info!("Calculator server started");
    service.waiting().await?;

    Ok(())
}
```

### Complete Client Implementation

```rust
use mcp_client::{
    ClientHandler, ServiceExt, model::*,
    transport::TokioChildProcess,
};
use tokio::process::Command;

#[derive(Clone, Default)]
struct MyClient;

impl ClientHandler for MyClient {
    // Handle sampling requests from server
    async fn create_message(
        &self,
        params: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, ErrorData> {
        // Integrate with your LLM here
        println!("Sampling request: {:?}", params.messages);

        Ok(CreateMessageResult {
            message: SamplingMessage::assistant_text("This is a sample response"),
            model: "my-llm-model".to_string(),
            stop_reason: Some(StopReason::EndTurn),
        })
    }

    // Provide workspace roots
    async fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> Result<ListRootsResult, ErrorData> {
        Ok(ListRootsResult {
            roots: vec![
                Root {
                    uri: "file:///home/user/project".to_string(),
                    name: Some("My Project".to_string()),
                }
            ],
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to server via subprocess
    let transport = TokioChildProcess::new(
        Command::new("cargo")
            .arg("run")
            .arg("--example")
            .arg("calculator-server")
    )?;

    let client = MyClient.serve(transport).await?;

    // List available tools
    let tools = client.list_all_tools().await?;
    println!("Available tools:");
    for tool in &tools {
        println!("  - {}: {}", tool.name, tool.description.as_deref().unwrap_or(""));
    }

    // Call a tool
    let result = client.call_tool(CallToolRequestParams {
        name: "add".to_string(),
        arguments: Some(mcp_server::object!({
            "a": 10,
            "b": 20
        })),
        task: None,
    }).await?;

    println!("Result: {:?}", result);

    // Get history
    let history = client.call_tool(CallToolRequestParams {
        name: "history".to_string(),
        arguments: None,
        task: None,
    }).await?;

    println!("History: {:?}", history);

    Ok(())
}
```

### HTTP Server with OAuth

```rust
use mcp_server::{
    ServerHandler, model::*,
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    },
};
use mcp_transports::auth::AuthorizationManager;
use axum::Router;

#[derive(Clone)]
pub struct ProtectedServer;

#[tool_router]
impl ProtectedServer {
    #[tool(description = "Protected resource access")]
    async fn get_data(&self) -> Result<CallToolResult, ErrorData> {
        Ok(CallToolResult::success(vec![
            Content::text("Sensitive data here")
        ]))
    }
}

#[tool_handler]
impl ServerHandler for ProtectedServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build()
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure OAuth 2.0
    let auth_manager = AuthorizationManager::builder()
        .client_id("mcp-server")
        .client_secret("super-secret")
        .redirect_uri("http://localhost:8000/callback")
        .scopes(vec!["read", "write"])
        .build();

    // Create HTTP service with authentication
    let http_service = StreamableHttpService::new(
        || Ok(ProtectedServer),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::builder()
            .authorization_manager(auth_manager)
            .build(),
    );

    let router = Router::new()
        .nest_service("/mcp", http_service);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;

    tracing::info!("Protected MCP server listening on http://0.0.0.0:8000/mcp");

    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap();
        })
        .await?;

    Ok(())
}
```

## Migration Path

For migrating from TypeScript/other implementations:

1. **Phase 1: Core Types**
   - Define all protocol types with serde
   - Implement JSON-RPC message handling
   - Create error types

2. **Phase 2: Transport Layer**
   - Implement stdio transport
   - Implement streamable HTTP transport
   - Add SSE parsing

3. **Phase 3: Service Layer**
   - Create Service trait with role generics
   - Implement request routing
   - Add notification handling

4. **Phase 4: Server SDK**
   - Define ServerHandler trait
   - Create tool/prompt routers
   - Implement procedural macros

5. **Phase 5: Client SDK**
   - Define ClientHandler trait
   - Implement sampling, roots
   - Add middleware support

6. **Phase 6: Authentication**
   - OAuth 2.0 flow implementation
   - JWT validation
   - Session management

7. **Phase 7: Testing**
   - Unit tests for all types
   - Integration tests with conformance suite
   - End-to-end tests

## Performance Considerations

1. **Zero-Copy Deserialization**: Use `Cow<str>` for strings that may be borrowed

2. **Connection Pooling**: Reuse HTTP connections with connection pools

3. **Batching**: Support batch JSON-RPC requests for reduced latency

4. **Async I/O**: All I/O operations should be non-blocking

5. **Memory Pooling**: Consider object pools for frequently allocated types

6. **Inlining**: Use `#[inline]` for small, frequently-called functions

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version_serialization() {
        let version = ProtocolVersion::LATEST;
        let json = serde_json::to_string(&version).unwrap();
        assert_eq!(json, r#""2025-06-18""#);
    }

    #[test]
    fn test_error_data_creation() {
        let error = ErrorData::method_not_found::<CallToolRequestMethod>();
        assert_eq!(error.code, ErrorCode::MethodNotFound);
    }

    #[tokio::test]
    async fn test_tool_call() {
        let server = CalculatorServer::new();
        let result = server.add(AddParams { a: 2, b: 3 }).await;
        assert!(result.is_ok());
    }
}
```

## Open Considerations

1. **Task Augmentation**: Full task-based execution requires careful state machine design

2. **Elicitation Flow**: URL-based elicitation needs browser integration strategy

3. **Distributed Sessions**: Multi-server deployments need shared session storage (Redis)

4. **Rate Limiting**: Consider tower middleware for rate limiting

5. **Metrics**: Add OpenTelemetry integration for observability
