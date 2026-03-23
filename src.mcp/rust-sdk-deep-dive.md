---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/rust-sdk
repository: https://github.com/modelcontextprotocol/rust-sdk
explored_at: 2026-03-23T00:00:00Z
language: Rust
---

# Deep Dive: Rust SDK (RMCP)

## Overview

RMCP is the official Rust SDK for Model Context Protocol. It provides a production-ready, type-safe implementation of the MCP specification using Tokio for async runtime.

**Version:** 1.2.0 (as of exploration date)
**License:** Apache-2.0
**Documentation:** [docs.rs/rmcp](https://docs.rs/rmcp)

## Crate Structure

```
rust-sdk/
├── crates/
│   ├── rmcp/                      # Core protocol implementation
│   │   ├── src/
│   │   │   ├── model/             # Protocol data types
│   │   │   │   ├── capabilities.rs
│   │   │   │   ├── content.rs
│   │   │   │   ├── prompt.rs
│   │   │   │   ├── resource.rs
│   │   │   │   ├── tool.rs
│   │   │   │   └── mod.rs         # 130KB total
│   │   │   ├── handler/
│   │   │   │   ├── client.rs      # Client handler trait
│   │   │   │   ├── server.rs      # Server handler trait (39KB)
│   │   │   │   └── server/
│   │   │   │       ├── router.rs  # Tool/prompt routing
│   │   │   │       ├── tool.rs    # Tool call handling
│   │   │   │       └── prompt.rs  # Prompt handling
│   │   │   ├── service.rs         # Core service trait (39KB)
│   │   │   ├── transport/         # Transport implementations
│   │   │   │   ├── io.rs          # Stdio transport
│   │   │   │   ├── child_process.rs
│   │   │   │   ├── sink_stream.rs
│   │   │   │   ├── async_rw.rs
│   │   │   │   ├── worker.rs
│   │   │   │   ├── streamable_http_client.rs
│   │   │   │   └── streamable_http_server/
│   │   │   ├── task_manager.rs    # Task lifecycle management
│   │   │   ├── error.rs           # Error types
│   │   │   └── lib.rs
│   │   ├── tests/
│   │   └── Cargo.toml
│   │
│   └── rmcp-macros/               # Procedural macros
│       ├── src/
│       │   ├── tool.rs            # #[tool] macro
│       │   ├── tool_handler.rs    # #[tool_handler] macro
│       │   ├── tool_router.rs     # #[tool_router] macro
│       │   ├── prompt.rs          # #[prompt] macro
│       │   └── lib.rs
│       └── Cargo.toml
│
├── examples/
│   ├── clients/                   # Client examples
│   ├── servers/                   # Server examples (15+ examples)
│   └── transport/                 # Transport examples
│
└── conformance/                   # Protocol conformance tests
```

## Core Architecture

### Service Trait Pattern

The SDK uses a service trait pattern with role-based generics:

```rust
pub trait ServiceRole: Debug + Send + Sync + 'static + Copy + Clone {
    type Req: TransferObject + GetMeta + GetExtensions;
    type Resp: TransferObject;
    type Not: TryInto<CancelledNotification, Error = Self::Not> + From<CancelledNotification> + TransferObject;
    type PeerReq: TransferObject + GetMeta + GetExtensions;
    type PeerResp: TransferObject;
    type PeerNot: TryInto<CancelledNotification, Error = Self::PeerNot> + From<CancelledNotification> + TransferObject + GetMeta + GetExtensions;
    type InitializeError;
    const IS_CLIENT: bool;
    type Info: TransferObject;
    type PeerInfo: TransferObject;
}

// Two concrete role types
pub struct RoleClient;   // IS_CLIENT = true
pub struct RoleServer;   // IS_CLIENT = false
```

### Service Implementation

```rust
pub trait Service<R: ServiceRole>: Send + Sync + 'static {
    fn handle_request(
        &self,
        request: R::PeerReq,
        context: RequestContext<R>,
    ) -> impl Future<Output = Result<R::Resp, McpError>> + MaybeSendFuture + '_;

    fn handle_notification(
        &self,
        notification: R::PeerNot,
        context: NotificationContext<R>,
    ) -> impl Future<Output = Result<(), McpError>> + MaybeSendFuture + '_;

    fn get_info(&self) -> R::Info;
}
```

## Server Implementation

### ServerHandler Trait

The `ServerHandler` trait provides default implementations for all MCP server methods:

```rust
pub trait ServerHandler: Sized + Send + Sync + 'static {
    // Capability declaration
    fn get_info(&self) -> ServerInfo;

    // Request handlers with default "not implemented" responses
    fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<InitializeResult, McpError>> { ... }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> { ... }

    fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> { ... }

    // Notification handlers
    fn on_initialized(
        &self,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> { ... }

    fn on_cancelled(
        &self,
        params: CancelledNotificationParam,
        context: NotificationContext<RoleServer>,
    ) -> impl Future<Output = ()> { ... }
}
```

### Tool Handler Macro

The `#[tool]` and `#[tool_handler]` macros simplify tool implementation:

```rust
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, schemars::JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AddParams {
    pub a: i32,
    pub b: i32,
}

#[derive(Clone)]
pub struct Calculator;

#[tool_handler]
impl ServerHandler for Calculator {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            ..Default::default()
        }
    }
}

#[tool(name = "add", description = "Add two numbers")]
async fn add(&self, Parameters(params): Parameters<AddParams>) -> i32 {
    params.a + params.b
}
```

### Router Pattern

The SDK uses a router pattern for composing tools and prompts:

```rust
pub struct Router<S> {
    pub tool_router: ToolRouter<S>,
    pub prompt_router: PromptRouter<S>,
    pub service: Arc<S>,
}

impl<S> Router<S>
where
    S: ServerHandler,
{
    pub fn with_tool<R, A>(mut self, route: R) -> Self
    where
        R: IntoToolRoute<S, A>,
    {
        self.tool_router.add_route(route.into_tool_route());
        self
    }

    pub fn with_prompt<R, A>(mut self, route: R) -> Self
    where
        R: IntoPromptRoute<S, A>,
    {
        self.prompt_router.add_route(route.into_prompt_route());
        self
    }
}
```

### Example: Complete Server

```rust
use rmcp::{
    ServerHandler, ServiceExt, model::*,
    transport::stdio, tool, tool_handler,
    handler::server::wrapper::Parameters,
    schemars::JsonSchema,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EchoArgs {
    pub message: String,
}

#[derive(Clone)]
pub struct EchoServer;

#[tool_handler]
impl ServerHandler for EchoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build(),
            ..Default::default()
        }
    }
}

#[tool(description = "Echo back the input message")]
async fn echo(&self, Parameters(args): Parameters<EchoArgs>) -> String {
    format!("Echo: {}", args.message)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = stdio();
    let server = EchoServer.serve(transport).await?;
    server.waiting().await?;
    Ok(())
}
```

## Client Implementation

### ClientHandler Trait

```rust
pub trait ClientHandler: Sized + Send + Sync + 'static {
    // Server request handlers
    fn create_message(
        &self,
        params: CreateMessageRequestParams,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<CreateMessageResult, McpError>> { ... }

    fn list_roots(
        &self,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<ListRootsResult, McpError>> { ... }

    // Notification handlers
    fn on_tool_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> { ... }

    fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> { ... }
}
```

### Example: Client with Sampling

```rust
use rmcp::{ClientHandler, ServiceExt, model::*, transport::TokioChildProcess};
use tokio::process::Command;

#[derive(Clone, Default)]
struct MyClient;

impl ClientHandler for MyClient {
    async fn create_message(
        &self,
        params: CreateMessageRequestParams,
        _context: RequestContext<RoleClient>,
    ) -> Result<CreateMessageResult, ErrorData> {
        // Integrate with your LLM here
        Ok(CreateMessageResult {
            message: SamplingMessage::assistant_text("Response text"),
            model: "my-model".into(),
            stop_reason: Some(CreateMessageResult::STOP_REASON_END_TURN.into()),
        })
    }

    async fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> Result<ListRootsResult, ErrorData> {
        Ok(ListRootsResult {
            roots: vec![Root {
                uri: "file:///home/user/project".into(),
                name: Some("My Project".into()),
            }],
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = TokioChildProcess::new(Command::new("npx").arg("-y").arg("@modelcontextprotocol/server-everything"))?;
    let client = MyClient.serve(transport).await?;

    // List available tools
    let tools = client.list_all_tools().await?;
    println!("Available tools: {:?}", tools);

    // Call a tool
    let result = client.call_tool(CallToolRequestParams {
        name: "example-tool".into(),
        arguments: Some(object!({ "key": "value" })),
        task: None,
    }).await?;
    println!("Tool result: {:?}", result);

    Ok(())
}
```

## Transport Layer

### Transport Trait

```rust
pub trait Transport<R>: Send
where
    R: ServiceRole,
{
    type Error: std::error::Error + Send + Sync + 'static;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<R>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static;

    fn receive(&mut self) -> impl Future<Output = Option<RxJsonRpcMessage<R>>> + Send;

    fn close(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
```

### Built-in Transports

| Transport | Feature Flag | Description |
|-----------|-------------|-------------|
| **Stdio** | `transport-io` | Standard input/output |
| **Child Process** | `transport-child-process` | Spawn and communicate with subprocess |
| **Streamable HTTP Client** | `transport-streamable-http-client` | HTTP streaming client |
| **Streamable HTTP Server** | `transport-streamable-http-server` | HTTP streaming server |
| **Sink/Stream** | (base) | From futures Sink/Stream |
| **Async Read/Write** | `transport-async-rw` | From tokio AsyncRead/AsyncWrite |
| **Worker** | `transport-worker` | Cross-task communication |

### Transport Examples

```rust
// Stdio transport
use rmcp::transport::stdio;
let transport = stdio();

// Child process transport
use rmcp::transport::{TokioChildProcess, ConfigureCommandExt};
use tokio::process::Command;
let transport = TokioChildProcess::new(
    Command::new("npx").configure(|cmd| {
        cmd.arg("-y").arg("@modelcontextprotocol/server-everything");
    })
)?;

// TCP stream transport
use tokio::net::TcpStream;
let stream = TcpStream::connect("127.0.0.1:8001").await?;
let transport = stream;  // Automatically implements IntoTransport

// HTTP Server transport (streamable)
use rmcp::transport::streamable_http_server::StreamableHttpService;
use axum::{Router, routing::post};

let app = Router::new()
    .route("/mcp", post(StreamableHttpService::new(MyServer)));
axum::serve(listener, app).await?;
```

## Model Types

### Core Type Categories

1. **JSON-RPC Types**
   - `JsonRpcRequest`, `JsonRpcNotification`, `JsonRpcResponse`
   - `RequestId`, `ProgressToken`, `Cursor`

2. **Protocol Types**
   - `ProtocolVersion` - Version negotiation
   - `ServerCapabilities`, `ClientCapabilities` - Feature negotiation
   - `Implementation` - Client/server metadata

3. **Feature Types**
   - **Tools**: `Tool`, `CallToolRequestParams`, `CallToolResult`, `ToolAnnotation`
   - **Resources**: `Resource`, `ResourceTemplate`, `ResourceContents`, `TextResourceContents`, `BlobResourceContents`
   - **Prompts**: `Prompt`, `PromptMessage`, `GetPromptRequestParams`, `GetPromptResult`
   - **Sampling**: `CreateMessageRequestParams`, `CreateMessageResult`, `SamplingMessage`, `ModelPreferences`

4. **Notification Types**
   - `ProgressNotificationParam` - Progress updates
   - `CancelledNotificationParam` - Cancellation
   - `LoggingMessageNotificationParam` - Log messages

### Type Design Patterns

```rust
// Result pattern with meta field
pub trait Result {
    fn _meta(&self) -> Option<&JsonObject>;
}

// Paginated results
pub struct PaginatedResult {
    pub next_cursor: Option<Cursor>,
}

// Annotations for UI hints
pub struct ToolAnnotations {
    pub title: Option<String>,
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub idempotent_hint: bool,
    pub open_world_hint: bool,
}

// Content union type
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text(TextContent),
    #[serde(rename = "image")]
    Image(ImageContent),
    #[serde(rename = "audio")]
    Audio(AudioContent),
    #[serde(rename = "resource")]
    Resource(ResourceLink),
}
```

## Error Handling

### ErrorData Type

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    pub code: ErrorCode,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // JSON-RPC standard errors
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,

    // MCP-specific errors
    UrlElicitationRequired = -32042,
}

// Builder pattern for error creation
impl ErrorData {
    pub fn new(code: ErrorCode, message: impl Into<String>, data: Option<Value>) -> Self;
    pub fn method_not_found<M: ConstString>() -> Self;
    pub fn invalid_params(message: impl Into<String>, data: Option<Value>) -> Self;
    pub fn internal_error(message: impl Into<String>, data: Option<Value>) -> Self;
    pub fn resource_not_found(message: impl Into<String>, data: Option<Value>) -> Self;
}
```

## Feature Flags

```toml
[features]
default = ["base64", "macros", "server"]
local = []                              # Disable Send for WASM
client = ["dep:tokio-stream"]
server = ["transport-async-rw", "dep:schemars", "dep:pastey"]
macros = ["dep:rmcp-macros", "dep:pastey"]
elicitation = ["dep:url"]

# HTTP client variants
__reqwest = ["dep:reqwest"]
reqwest = ["__reqwest", "reqwest?/rustls"]
reqwest-native-tls = ["__reqwest", "reqwest?/native-tls"]

# Server-side HTTP
server-side-http = ["uuid", "dep:rand", "dep:tokio-stream", "dep:http", ...]

# Streamable HTTP
transport-streamable-http-client = ["client-side-sse", "transport-worker"]
transport-streamable-http-server = ["transport-streamable-http-server-session", "server-side-http"]

# OAuth authentication
auth = ["dep:oauth2", "__reqwest", "dep:url"]
auth-client-credentials-jwt = ["auth", "dep:jsonwebtoken", "uuid"]
```

## Task Management

The SDK supports task-augmented execution for long-running operations:

```rust
// Task metadata in request
pub struct CallToolRequestParams {
    pub name: String,
    pub arguments: Option<JsonObject>,
    pub task: Option<TaskMetadata>,  // Optional task augmentation
}

// Task-related requests
pub enum ClientRequest {
    ListTasksRequest(ListTasksRequest),
    GetTaskInfoRequest(GetTaskInfoRequest),
    GetTaskResultRequest(GetTaskResultRequest),
    CancelTaskRequest(CancelTaskRequest),
}

// Server task handling
impl ServerHandler for MyServer {
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        if request.task.is_some() {
            // Enqueue as task
            self.enqueue_task(request.params, context).await
        } else {
            // Direct execution
            self.call_tool_direct(request.params, context).await
        }
    }
}
```

## Key Insights for Production Implementation

1. **Type-Safe Protocol**: All protocol messages are strongly typed with serde for serialization

2. **Zero-Cost Abstractions**: The trait-based design allows for both dynamic and static dispatch

3. **Async-First**: Built on Tokio with proper cancellation and timeout support

4. **Composable Architecture**: Router pattern allows easy composition of tools and prompts

5. **Feature-Gated**: Minimal default features with opt-in for specific transports

6. **Macro Ergonomics**: Procedural macros reduce boilerplate for common patterns

7. **OAuth Support**: Built-in OAuth 2.0 with client credentials and JWT support

## Example: Production Server with HTTP Transport

```rust
use axum::{Router, routing::post, extract::State};
use rmcp::{
    ServerHandler, ServiceExt, model::*,
    transport::streamable_http_server::StreamableHttpService,
    tool, tool_handler,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    data: Arc<RwLock<Vec<String>>>,
}

#[tool_handler]
impl ServerHandler for AppState {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            ..Default::default()
        }
    }
}

#[tool(description = "Add item to the list")]
async fn add_item(
    &self,
    Parameters(params): Parameters<AddItemParams>,
) -> Result<String, ErrorData> {
    self.data.write().await.push(params.item);
    Ok(format!("Added: {}", params.item))
}

#[tokio::main]
async fn main() {
    let state = AppState {
        data: Arc::new(RwLock::new(vec![])),
    };

    let http_service = StreamableHttpService::new(state.clone());
    let app = Router::new()
        .route("/mcp", post(http_service))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## Related Resources

- [API Documentation](https://docs.rs/rmcp)
- [GitHub Repository](https://github.com/modelcontextprotocol/rust-sdk)
- [Examples Directory](./examples/)
- [Migration Guide](https://github.com/modelcontextprotocol/rust-sdk/discussions/716)
