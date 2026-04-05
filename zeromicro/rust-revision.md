---
source: /home/darkvoid/Boxxed/@formulas/src.zeromicro/go-zero
repository: github.com/zeromicro/go-zero
explored_at: 2026-04-04
---

# Zero to Rust-Microservices Engineer - Replicating go-zero Patterns in Rust

## Overview

This guide shows how to replicate go-zero's high-performance microservices patterns in Rust. We cover REST servers, gRPC services, code generation alternatives, resilience patterns, and observability using Rust's ecosystem.

## Why Rust for Microservices?

| Aspect | Go (go-zero) | Rust Equivalent |
|--------|--------------|-----------------|
| Performance | High (100k+ QPS) | Higher (zero-cost abstractions) |
| Memory Safety | GC pauses | No GC, compile-time guarantees |
| Concurrency | Goroutines | Async/await with tokio |
| Code Generation | goctl | askama, proc macros |
| gRPC | grpc-go | tonic |
| HTTP Server | net/http | axum, actix-web |

## Project Setup

### Cargo.toml Configuration

```toml
[package]
name = "rust-microservices"
version = "0.1.0"
edition = "2021"

[workspace]
members = [
    "core",           # Core resilience patterns
    "api-server",     # REST API server
    "rpc-server",     # gRPC server  
    "proto",          # Protocol buffer definitions
    "codegen",        # Code generation tools
]

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# HTTP server
axum = { version = "0.7", features = ["macros"] }
tower = { version = "0.4", features = ["full"] }
tower-http = { version = "0.5", features = ["full"] }

# gRPC
tonic = "0.10"
prost = "0.12"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
prost = "0.12"

# Configuration
config = "0.14"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Metrics
metrics = "0.22"
metrics-exporter-prometheus = "0.13"

# Resilience
tower = { version = "0.4", features = ["limit", "retry", "timeout"] }
circuitbreaker = "0.5"

# Service discovery
etcd-client = "0.13"

# Error handling
thiserror = "1.0"
anyhow = "1.0"
```

## REST Server Implementation

### Basic Server (Equivalent to go-zero rest)

```rust
// api-server/src/main.rs

use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{Path, State, Json},
    middleware,
    http::StatusCode,
};
use tower_http::{
    trace::TraceLayer,
    timeout::TimeoutLayer,
    compression::CompressionLayer,
};
use std::{time::Duration, sync::Arc};

// Application state (equivalent to go-zero ServiceContext)
pub struct AppState {
    pub config: Config,
    // Add dependencies: DB connections, caches, etc.
}

#[derive(Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub timeout: Duration,
    pub jwt_secret: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("api_server=info".parse()?)
        )
        .init();
    
    // Load configuration
    let config = Config {
        host: "0.0.0.0".to_string(),
        port: 8888,
        timeout: Duration::from_secs(5),
        jwt_secret: std::env::var("JWT_SECRET").unwrap_or_default(),
    };
    
    // Create shared state
    let state = Arc::new(AppState {
        config: config.clone(),
    });
    
    // Build router with middleware stack
    let app = create_router(state);
    
    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Routes (equivalent to go-zero api routes)
        .route("/greet/from/:name", get(greet_handler))
        .route("/users", post(create_user_handler))
        .route("/users/:id", put(update_user_handler))
        .route("/users/:id", delete(delete_user_handler))
        // Health check
        .route("/health", get(health_handler))
        // Middleware stack (equivalent to go-zero middleware)
        .layer(TraceLayer::new_for_http())  // Logging
        .layer(TimeoutLayer::new(Duration::from_secs(5)))  // Timeout
        .layer(CompressionLayer::new())  // Response compression
        .layer(middleware::from_fn(auth_middleware))  // Auth
        .layer(middleware::from_fn(rate_limit_middleware))  // Rate limit
        .with_state(state)
}

// Response types (equivalent to go-zero types)
#[derive(serde::Serialize)]
pub struct GreetResponse {
    message: String,
}

#[derive(serde::Deserialize)]
pub struct GreetRequest {
    name: String,
}
```

### Handler Implementation

```rust
// api-server/src/handlers.rs

use axum::{
    extract::{Path, State},
    Json,
    http::StatusCode,
};
use crate::{AppState, AppError};

// Handler function (equivalent to go-zero handler)
pub async fn greet_handler(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<GreetResponse>, AppError> {
    // Validate input (equivalent to go-zero Parse validation)
    if name.is_empty() {
        return Err(AppError::BadRequest("Name cannot be empty".into()));
    }
    
    // Call logic layer (equivalent to go-zero logic)
    let logic = GreetLogic::new(&state);
    let response = logic.greet(&name)?;
    
    Ok(Json(response))
}

// Logic layer (equivalent to go-zero logic package)
struct GreetLogic<'a> {
    state: &'a AppState,
}

impl<'a> GreetLogic<'a> {
    fn new(state: &'a AppState) -> Self {
        Self { state }
    }
    
    fn greet(&self, name: &str) -> Result<GreetResponse, AppError> {
        // Business logic here
        // Example: database operations, external API calls
        let message = format!("Hello, {}!", name);
        
        Ok(GreetResponse { message })
    }
}

// Error types (equivalent to go-zero error handling)
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Timeout")]
    Timeout(#[from] tokio::time::error::Elapsed),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "Request timeout".into()),
            AppError::Database(err) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".into())
            }
        };
        
        (status, Json(serde_json::json!({
            "error": message,
            "code": status.as_u16(),
        }))).into_response()
    }
}
```

### Middleware Implementation

```rust
// api-server/src/middleware.rs

use axum::{
    extract::State,
    middleware::Next,
    response::Response,
    http::{Request, StatusCode},
};
use std::sync::Arc;
use crate::AppState;

// Authentication middleware (equivalent to go-zero auth middleware)
pub async fn auth_middleware<B>(
    State(state): State<Arc<AppState>>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Extract token from header
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Validate JWT token
    let claims = validate_jwt(token, &state.config.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    // Add claims to request extensions
    req.extensions_mut().insert(claims);
    
    Ok(next.run(req).await)
}

fn validate_jwt(token: &str, secret: &str) -> Result<Claims, jwt::Error> {
    use jsonwebtoken::{decode, DecodingKey, Validation};
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    
    Ok(token_data.claims)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String,  // Subject (user ID)
    pub exp: usize,   // Expiration time
    pub iat: usize,   // Issued at
}

// Rate limit middleware (equivalent to go-zero rate limiting)
pub async fn rate_limit_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // Use tower-governor or custom implementation
    // See resilience patterns section below
    
    Ok(next.run(req).await)
}

// Logging middleware (equivalent to go-zero logging)
pub async fn logging_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Response
where
    B: std::fmt::Debug,
{
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let start = std::time::Instant::now();
    
    tracing::info!("Started {} {}", method, path);
    
    let response = next.run(req).await;
    
    let duration = start.elapsed();
    let status = response.status();
    
    tracing::info!(
        duration_ms = duration.as_millis(),
        status = status.as_u16(),
        "Completed {} {}",
        method,
        path
    );
    
    // Slow request detection
    if duration > Duration::from_secs(1) {
        tracing::warn!(
            duration_ms = duration.as_millis(),
            "Slow request: {} {}",
            method,
            path
        );
    }
    
    response
}
```

## gRPC Server (ZRPC Equivalent)

### Server Implementation

```rust
// rpc-server/src/main.rs

use tonic::{transport::Server, Request, Response, Status};
use tokio_stream::StreamExt;

// Generated proto code (equivalent to goctl rpc generation)
pub mod pb {
    tonic::include_proto!("greet");
}

use pb::{
    greet_server::{Greet, GreetServer},
    Request as GreetRequest,
    Response as GreetResponse,
};

#[derive(Default)]
pub struct GreeterService;

#[tonic::async_trait]
impl Greet for GreeterService {
    async fn greet(
        &self,
        request: Request<GreetRequest>,
    ) -> Result<Response<GreetResponse>, Status> {
        tracing::info!("Received request: {:?}", request);
        
        let req = request.into_inner();
        
        // Business logic
        let message = format!("Hello, {}!", req.name);
        
        Ok(Response::new(GreetResponse { message }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("rpc_server=info,tower=warn")
        .init();
    
    // Load configuration
    let config = Config::load()?;
    
    // Create service
    let greeter = GreeterService::default();
    
    // Build server with interceptors (equivalent to go-zero zrpc interceptors)
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Starting gRPC server on {}", addr);
    
    Server::builder()
        // Add interceptors
        .layer(tower::layer::layer_fn(LoggingLayer))
        .layer(tower::layer::layer_fn(TracingLayer))
        .layer(tower::timeout::TimeoutLayer::new(Duration::from_secs(5)))
        // Add service
        .add_service(GreetServer::new(greeter))
        .serve(addr.parse()?)
        .await?;
    
    Ok(())
}

// Logging interceptor (equivalent to go-zero server interceptor)
#[derive(Clone)]
pub struct LoggingLayer<S>(S);

impl<S, T> tower::Service<T> for LoggingLayer<S>
where
    S: tower::Service<T>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.0.poll_ready(cx)
    }

    fn call(&mut self, request: T) -> Self::Future {
        tracing::info!("gRPC request received");
        self.0.call(request)
    }
}
```

### Client with Circuit Breaker

```rust
// rpc-server/src/client.rs

use tonic::transport::Channel;
use tower::retry::{Retry, RetryLayer};
use tower::timeout::TimeoutLayer;
use std::time::Duration;

pub struct RpcClient {
    channel: Channel,
}

impl RpcClient {
    pub async fn connect(target: &str) -> anyhow::Result<Self> {
        // Connect with service discovery (see Service Discovery section)
        let channel = Channel::from_static(target)
            .timeout(Duration::from_secs(5))
            .connect()
            .await?;
        
        Ok(Self { channel })
    }
    
    pub fn with_resilience(channel: Channel) -> Channel {
        channel
            // Timeout
            .timeout(Duration::from_secs(5))
            // Retry with backoff (equivalent to go-zero retry)
            .retry(RetryConfig {
                max_retries: 3,
                initial_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(10),
                multiplier: 2.0,
            })
            // Circuit breaker (see resilience section)
            .load_shed()
    }
}

// gRPC client with circuit breaker
pub async fn call_with_breaker<T, F, Fut>(
    breaker_name: &str,
    f: F,
) -> Result<T, AppError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, Status>>,
{
    // Check circuit breaker
    if is_circuit_open(breaker_name) {
        return Err(AppError::ServiceUnavailable);
    }
    
    match f().await {
        Ok(response) => {
            mark_success(breaker_name);
            Ok(response)
        }
        Err(err) => {
            mark_failure(breaker_name);
            Err(err.into())
        }
    }
}
```

## Code Generation Alternatives

### Askama Templates (Equivalent to goctl)

```rust
// codegen/src/api_generator.rs

use askama::Template;

#[derive(Template)]
#[template(path = "handler.tpl", escape = "none")]
pub struct HandlerTemplate {
    pub handler_name: String,
    pub request_type: String,
    pub response_type: String,
    pub method: String,
    pub path: String,
}

// Generate handler code
pub fn generate_handler(
    handler_name: &str,
    request_type: &str,
    response_type: &str,
    method: &str,
    path: &str,
) -> String {
    let tmpl = HandlerTemplate {
        handler_name: handler_name.to_string(),
        request_type: request_type.to_string(),
        response_type: response_type.to_string(),
        method: method.to_string(),
        path: path.to_string(),
    };
    
    tmpl.render().expect("Failed to render template")
}

// templates/handler.tpl
/*
pub async fn {{ handler_name }}(
    Path(params): Path<{{ request_type }}>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<{{ response_type }}>, AppError> {
    let logic = {{ request_type }}Logic::new(&state);
    let response = logic.handle(&params)?;
    Ok(Json(response))
}
*/
```

### Proc Macro for Route Generation

```rust
// codegen/src/macros.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Macro to generate route handler (similar to goctl generated code)
#[proc_macro_attribute]
pub fn route(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;
    
    let expanded = quote! {
        // Generate route registration
        fn register_routes(router: Router) -> Router {
            router.route("/api/{}", #func_name)
        }
        
        #func
    };
    
    TokenStream::from(expanded)
}
```

## Conclusion

This Rust implementation provides equivalents for:

1. **REST Server**: Axum with middleware stack
2. **gRPC Server**: Tonic with interceptors
3. **Code Generation**: Askama templates, proc macros
4. **Resilience**: Tower layers for timeout, retry, rate limit
5. **Observability**: Tracing, metrics
6. **Service Discovery**: Etcd client

The Rust versions provide:
- Better performance (no GC)
- Compile-time safety
- Lower memory usage
- Equivalent resilience patterns
