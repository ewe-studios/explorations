---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/rust-sdk/examples/servers
explored_at: 2026-03-23T00:00:00Z
language: Rust
---

# Deep Dive: MCP Server Implementations

## Overview

This deep dive examines reference MCP server implementations in the Rust SDK. These examples demonstrate production-ready patterns for building MCP servers with various features and transport configurations.

## Example Servers Catalog

| Server | Transport | Features Demonstrated |
|--------|-----------|----------------------|
| **counter_stdio** | Stdio | Basic tools, prompts, resources |
| **counter_streamhttp** | Streamable HTTP | HTTP transport with sessions |
| **counter_hyper_streamable_http** | Hyper | Low-level HTTP server |
| **progress_demo** | Stdio, HTTP | Progress notifications, streaming |
| **completion_stdio** | Stdio | Completions API |
| **prompt_stdio** | Stdio | Prompts with arguments |
| **sampling_stdio** | Stdio | LLM sampling from server |
| **elicitation_stdio** | Stdio | Interactive elicitation |
| **memory_stdio** | Stdio | State management |
| **structured_output** | Stdio | JSON Schema output validation |
| **simple_auth_streamhttp** | HTTP | Basic OAuth authentication |
| **complex_auth_streamhttp** | HTTP | Advanced OAuth flows |

## Server Architecture Patterns

### 1. Basic Server Structure

```rust
use rmcp::{
    ServerHandler, ServiceExt, model::*,
    tool, tool_handler, tool_router,
    handler::server::wrapper::Parameters,
};
use tokio::sync::Mutex;
use std::sync::Arc;

// Server state
#[derive(Clone)]
pub struct Counter {
    counter: Arc<Mutex<i32>>,
    tool_router: ToolRouter<Counter>,
    prompt_router: PromptRouter<Counter>,
}

// Tool router with macro generation
#[tool_router]
impl Counter {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    // Simple synchronous tool
    #[tool(description = "Increment the counter by 1")]
    async fn increment(&self) -> Result<CallToolResult, McpError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(CallToolResult::success(vec![Content::text(counter.to_string())]))
    }

    // Tool with typed parameters
    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(
        &self,
        Parameters(StructRequest { a, b }): Parameters<StructRequest>,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text((a + b).to_string())]))
    }
}

// Server handler implementation
#[tool_handler]
impl ServerHandler for Counter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_prompts()
                .build()
        )
        .with_protocol_version(ProtocolVersion::LATEST)
        .with_server_info(Implementation::from_build_env())
        .with_instructions("Counter server instructions".to_string())
    }
}

// Main entry point
#[tokio::main]
async fn main() -> Result<()> {
    let server = Counter::new();
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

### 2. Progress Notification Pattern

The progress demo shows how to send progress notifications during long-running operations:

```rust
#[tool(description = "Process data stream with progress updates")]
async fn stream_processor(
    &self,
    ctx: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    let mut counter = 0;
    let mut data_source = self.data_source.clone();

    loop {
        let chunk = data_source.next().await;
        if chunk.is_none() {
            break;
        }

        let chunk = chunk.unwrap().unwrap();
        counter += 1;

        // Create progress notification
        let progress_param = ProgressNotificationParam {
            progress_token: ProgressToken(NumberOrString::Number(counter)),
            progress: counter as f64,
            total: Some(100.0),  // Optional total
            message: Some(format!("Processed chunk {}", counter)),
        };

        // Send progress notification
        match ctx.peer.notify_progress(progress_param).await {
            Ok(_) => debug!("Progress: {}", counter),
            Err(e) => {
                return Err(McpError::internal_error(
                    format!("Failed to notify progress: {}", e),
                    Some(json!({ "progress": counter }))
                ));
            }
        }
    }

    Ok(CallToolResult::success(vec![Content::text(
        format!("Processed {} records successfully", counter)
    )]))
}
```

**Key Points:**
- Progress token must match the one from the request's `_meta.progressToken`
- Progress values should monotonically increase
- Total is optional but helps clients display accurate progress bars
- Message provides human-readable status updates

### 3. Streamable HTTP Server Pattern

For production deployments, HTTP transport with session management is recommended:

```rust
use rmcp::transport::streamable_http_server::{
    StreamableHttpService,
    session::local::LocalSessionManager,
};
use axum::Router;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create service with session management
    let service = StreamableHttpService::new(
        || Ok(Counter::new()),  // Factory for creating server instances per session
        LocalSessionManager::default().into(),  // Session state management
        Default::default(),  // Configuration
    );

    // Create Axum router
    let router = Router::new().nest_service("/mcp", service);

    // Bind and serve
    let tcp_listener = tokio::net::TcpListener::bind("127.0.0.1:8001").await?;

    tracing::info!("MCP server started at http://127.0.0.1:8001/mcp");

    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap();
        })
        .await?;

    Ok(())
}
```

**Session Management:**
- `LocalSessionManager`: In-memory sessions (single server)
- For distributed deployments, implement custom `SessionManager`

### 4. Task-Based Execution Pattern

For long-running operations that may exceed timeout limits:

```rust
use rmcp::task_manager::{OperationProcessor, OperationResultTransport};

// Operation result wrapper
struct ToolCallOperationResult {
    id: String,
    result: Result<CallToolResult, McpError>,
}

impl OperationResultTransport for ToolCallOperationResult {
    fn operation_id(&self) -> &str { &self.id }
    fn as_any(&self) -> &dyn Any { self }
}

#[derive(Clone)]
pub struct Counter {
    processor: Arc<Mutex<OperationProcessor>>,
    // ... other fields
}

#[tool_router]
impl Counter {
    #[tool(description = "Long running task example")]
    async fn long_task(&self) -> Result<CallToolResult, McpError> {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        Ok(CallToolResult::success(vec![Content::text("Long task completed")]))
    }
}

// Task-augmented execution
async fn call_tool_as_task(
    &self,
    request: CallToolRequestParams,
    ctx: RequestContext<RoleServer>,
) -> Result<CreateTaskResult, McpError> {
    let task_id = self.processor.lock().await.create_task();

    // Spawn background task
    tokio::spawn({
        let processor = self.processor.clone();
        async move {
            let result = execute_tool(request).await;
            processor.lock().await.complete_task(&task_id, result);
        }
    });

    Ok(CreateTaskResult {
        task_id: TaskId(task_id),
        status: TaskStatus::Pending,
    })
}
```

### 5. OAuth Authentication Pattern

For servers requiring authentication:

```rust
use rmcp::transport::{
    streamable_http_server,
    auth::{AuthorizationManager, InMemoryStateStore, InMemoryCredentialStore},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure OAuth
    let auth_manager = AuthorizationManager::new(
        oauth2::ClientId::new("mcp-server".to_string()),
        oauth2::ClientSecret::new("secret".to_string()),
        "http://localhost:8000/callback".parse()?,
        vec!["read".to_string(), "write".to_string()],
        InMemoryCredentialStore::new(),
        InMemoryStateStore::new(),
    );

    let service = StreamableHttpService::new(
        || Ok(Counter::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::builder()
            .authorization_manager(auth_manager)
            .build(),
    );

    let router = Router::new().nest_service("/mcp", service);

    // ... serve
    Ok(())
}
```

### 6. Completions Pattern

For argument auto-completion:

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CompleteArgs {
    #[schemars(description = "Type of completion")]
    pub kind: String,
    pub partial: String,
}

#[tool(description = "Get completion suggestions")]
async fn complete(
    &self,
    Parameters(args): Parameters<CompleteArgs>,
    _ctx: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    let suggestions = match args.kind.as_str() {
        "sql_operation" => {
            vec!["SELECT", "INSERT", "UPDATE", "DELETE"]
                .into_iter()
                .filter(|s| s.starts_with(&args.partial.to_uppercase()))
                .map(String::from)
                .collect::<Vec<_>>()
        }
        "table_name" => {
            vec!["users", "orders", "products"]
                .into_iter()
                .filter(|s| s.starts_with(&args.partial.to_lowercase()))
                .map(String::from)
                .collect::<Vec<_>>()
        }
        _ => vec![],
    };

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&suggestions).unwrap()
    )]))
}
```

### 7. Sampling Pattern (Server requesting LLM)

For servers that need LLM assistance:

```rust
#[tool(description = "Analyze error message with LLM help")]
async fn analyze_error(
    &self,
    Parameters(params): Parameters<ErrorParams>,
    ctx: RequestContext<RoleServer>,
) -> Result<CallToolResult, McpError> {
    // Request LLM sampling from client
    let response = ctx.peer.create_message(CreateMessageRequestParams {
        messages: vec![SamplingMessage::user_text(
            format!("Explain this error: {}", params.error_message)
        )],
        model_preferences: Some(ModelPreferences {
            hints: Some(vec![ModelHint { name: Some("claude".into()) }]),
            cost_priority: Some(0.3),
            speed_priority: Some(0.8),
            intelligence_priority: Some(0.7),
        }),
        system_prompt: Some("You are a helpful error analysis assistant.".into()),
        include_context: Some(ContextInclusion::None),
        temperature: Some(0.7),
        max_tokens: 500,
        stop_sequences: None,
        metadata: None,
        tools: None,
        tool_choice: None,
    }).await?;

    // Extract response
    let analysis = response.message.content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.clone())
        .unwrap_or_else(|| "No analysis available".to_string());

    Ok(CallToolResult::success(vec![Content::text(analysis)]))
}
```

### 8. Logging Pattern

For structured logging to clients:

```rust
impl ServerHandler for MyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_logging()
                .build(),
            ..Default::default()
        }
    }

    async fn set_level(
        &self,
        request: SetLevelRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        // Store log level for filtering
        self.log_level.store(request.level);
        Ok(())
    }
}

// Send log messages
async fn process_request(&self, ctx: RequestContext<RoleServer>) {
    // Debug log
    ctx.peer.notify_logging_message(LoggingMessageNotificationParam {
        level: LoggingLevel::Debug,
        logger: Some("my-server".into()),
        data: json!({ "event": "Request received" }),
    }).await?;

    // Info log
    ctx.peer.notify_logging_message(LoggingMessageNotificationParam {
        level: LoggingLevel::Info,
        logger: Some("my-server".into()),
        data: json!({ "event": "Processing complete", "items": 42 }),
    }).await?;
}
```

## Tool and Resource Exposure Patterns

### Tool Registration Patterns

**Pattern 1: Direct Method Annotation**
```rust
#[tool_router]
impl MyServer {
    #[tool(name = "my-tool", description = "Does something")]
    async fn my_tool(&self) -> Result<CallToolResult, McpError> {
        // Implementation
    }
}
```

**Pattern 2: Dynamic Tool Registration**
```rust
impl MyServer {
    pub fn add_tool(&mut self, tool: Tool, handler: ToolHandler) {
        self.tool_router.add_route(ToolRoute::new(tool, handler));
        // Notify clients
        self.notify_tool_list_changed();
    }
}
```

### Resource Exposure Patterns

**Pattern 1: Static Resources**
```rust
impl ServerHandler for MyServer {
    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                RawResource::new("file:///config.json", "Configuration")
                    .with_description("Server configuration file")
                    .with_mime_type("application/json")
                    .no_annotation(),
                RawResource::new("memo://insights", "Analysis Insights")
                    .with_description("AI-generated insights")
                    .no_annotation(),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match request.uri.as_str() {
            "file:///config.json" => Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    r#"{"key": "value"}"#,
                    &request.uri
                )],
            }),
            _ => Err(McpError::resource_not_found(
                "Resource not found",
                Some(json!({ "uri": request.uri }))
            )),
        }
    }
}
```

**Pattern 2: Resource Templates**
```rust
async fn list_resource_templates(
    &self,
    _request: Option<PaginatedRequestParams>,
    _context: RequestContext<RoleServer>,
) -> Result<ListResourceTemplatesResult, McpError> {
    Ok(ListResourceTemplatesResult {
        resource_templates: vec![
            ResourceTemplate {
                uri_template: "file:///{path}".to_string(),
                name: "File System".to_string(),
                description: Some("Access files by path".to_string()),
                mime_type: None,
                annotations: None,
                meta: None,
            }
        ],
        next_cursor: None,
        meta: None,
    })
}
```

**Pattern 3: Subscription-Based Resources**
```rust
#[derive(Clone)]
pub struct SubscriptionServer {
    subscribers: Arc<Mutex<HashSet<String>>>,
}

impl ServerHandler for SubscriptionServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_resources()
                .enable_resources_subscribe()
                .build(),
            ..Default::default()
        }
    }

    async fn subscribe(
        &self,
        request: SubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        self.subscribers.lock().await.insert(request.uri);
        Ok(())
    }

    async fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<(), McpError> {
        self.subscribers.lock().await.remove(&request.uri);
        Ok(())
    }
}

// When resource changes
async fn notify_subscribers(&self, uri: &str) {
    for peer in self.peers.iter() {
        peer.notify_resource_updated(ResourceUpdatedNotificationParam {
            uri: uri.to_string(),
        }).await?;
    }
}
```

## Configuration and Environment

### Logging Configuration

```rust
use tracing_subscriber::{self, EnvFilter};

// Initialize with environment-based log level
tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::from_default_env()
            .add_directive(tracing::Level::DEBUG.into())
    )
    .with_writer(std::io::stderr)
    .with_ansi(false)
    .init();
```

### Environment Variables

```rust
// Transport mode selection
let transport_mode = env::args()
    .nth(1)
    .unwrap_or_else(|| env::var("TRANSPORT_MODE").unwrap_or("stdio".into()));

// HTTP binding
const HTTP_BIND_ADDRESS: &str = "127.0.0.1:8001";
// Or from environment
let bind_addr = env::var("MCP_BIND_ADDR").unwrap_or_else(|_| HTTP_BIND_ADDRESS.to_string());

// OAuth configuration
let client_id = env::var("MCP_OAUTH_CLIENT_ID")?;
let client_secret = env::var("MCP_OAUTH_CLIENT_SECRET")?;
```

## Key Insights for Production Servers

1. **Use Router Macros**: The `#[tool_router]` and `#[tool_handler]` macros significantly reduce boilerplate

2. **Session Management**: For HTTP deployments, proper session state management is critical

3. **Progress for Long Operations**: Always use progress notifications for operations taking >1 second

4. **Task Augmentation**: Consider task-based execution for operations that may exceed client timeouts

5. **Graceful Shutdown**: Implement proper shutdown handling with `with_graceful_shutdown()`

6. **Error Context**: Include structured error context with `json!()` for debugging

7. **Capability Declaration**: Only declare capabilities you actually implement

8. **Instructions**: Provide clear server instructions to help clients understand capabilities

## References

- [RMCP Examples](./rust-sdk/examples/)
- [MCP Specification](https://modelcontextprotocol.io/specification)
- [Axum Documentation](https://docs.rs/axum)
- [Tokio Documentation](https://docs.rs/tokio)
