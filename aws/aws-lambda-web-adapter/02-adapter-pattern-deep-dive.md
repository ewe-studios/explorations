---
title: "Adapter Pattern Deep Dive"
subtitle: "How Lambda Web Adapter translates between Lambda events and HTTP"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-web-adapter/02-adapter-pattern-deep-dive.md
related: /home/darkvoid/Boxxed/@dev/repo-expolorations/aws/aws-lambda-web-adapter/exploration.md
---

# Adapter Pattern Deep Dive

## Introduction

This document provides a comprehensive analysis of how Lambda Web Adapter implements the adapter pattern to translate between Lambda events and HTTP requests. We'll examine the source code, design patterns, and implementation details.

### Source Code Location

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.aws/aws-lambda-web-adapter/
├── src/
│   ├── lib.rs           # Main adapter logic
│   ├── main.rs          # Entry point
│   └── readiness.rs     # Health check implementation
```

---

## Part 1: Architecture Overview

### Component Diagram

```mermaid
flowchart TB
    subgraph Adapter["Lambda Web Adapter"]
        A1[Extension Registration]
        A2[Readiness Checker]
        A3[Event Loop]
        A4[HTTP Translator]
        A5[Response Handler]
    end

    subgraph Runtime["Lambda Runtime API"]
        R1[/invocation/next]
        R2[/invocation/{id}/response]
        R3[/invocation/{id}/error]
        R4[/extension/register]
    end

    subgraph WebApp["Web Application"]
        W1[HTTP Server :8080]
        W2[Framework Handler]
        W3[Health Endpoint]
    end

    A1 --> R4
    A2 --> W3
    A3 --> R1
    A3 --> A4
    A4 --> W1
    W2 --> A5
    A5 --> R2
```

### Data Flow

```
Lambda Event Flow:
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌─────────────┐
│   Lambda    │────►│  Runtime API │────►│   Adapter   │────►│  Web App    │
│   Service   │     │  (port 9001) │     │  (proxy)    │     │  (port 8080)│
└─────────────┘     └──────────────┘     └─────────────┘     └─────────────┘

Response Flow:
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌─────────────┐
│   Lambda    │◄────│  Runtime API │◄────│   Adapter   │◄────│  Web App    │
│   Service   │     │  (port 9001) │     │  (proxy)    │     │  (port 8080)│
└─────────────┘     └──────────────┘     └─────────────┘     └─────────────┘
```

---

## Part 2: Core Adapter Structure

### AdapterOptions

Configuration for the adapter:

```rust
pub struct AdapterOptions {
    /// Port the web application listens on
    pub port: u16,
    /// Host the web application binds to
    pub host: String,
    /// Readiness check configuration
    pub readiness_check: ReadinessConfig,
    /// Lambda invoke mode
    pub invoke_mode: LambdaInvokeMode,
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for AdapterOptions {
    fn default() -> Self {
        Self {
            port: env::var(ENV_PORT)
                .or_else(|_| env::var(ENV_PORT_DEPRECATED))
                .unwrap_or("8080".to_string())
                .parse()
                .unwrap_or(8080),
            host: env::var(ENV_HOST)
                .or_else(|_| env::var(ENV_HOST_DEPRECATED))
                .unwrap_or("127.0.0.1".to_string()),
            readiness_check: ReadinessConfig::default(),
            invoke_mode: LambdaInvokeMode::Buffered,
            enable_compression: env::var(ENV_ENABLE_COMPRESSION)
                .unwrap_or("false".to_string())
                .parse()
                .unwrap_or(false),
        }
    }
}
```

### Adapter State

```rust
pub struct Adapter {
    /// Configuration
    options: AdapterOptions,
    /// Lambda Runtime API client
    client: hyper_util::client::legacy::Client<HttpConnector>,
    /// Extension ID (after registration)
    extension_id: Option<String>,
    /// Readiness checkpoint
    checkpoint: Arc<AtomicBool>,
}
```

---

## Part 3: Readiness Check Implementation

### ReadinessConfig

```rust
pub struct ReadinessConfig {
    /// Port for readiness checks
    pub port: u16,
    /// Path for readiness checks
    pub path: String,
    /// Protocol: HTTP or TCP
    pub protocol: Protocol,
    /// HTTP status codes considered healthy
    pub healthy_status: StatusRange,
}

impl Default for ReadinessConfig {
    fn default() -> Self {
        Self {
            port: env::var(ENV_READINESS_CHECK_PORT)
                .or_else(|_| env::var(ENV_READINESS_CHECK_PORT_DEPRECATED))
                .unwrap_or_default()
                .parse()
                .unwrap_or(8080),
            path: env::var(ENV_READINESS_CHECK_PATH)
                .or_else(|_| env::var(ENV_READINESS_CHECK_PATH_DEPRECATED))
                .unwrap_or("/".to_string()),
            protocol: env::var(ENV_READINESS_CHECK_PROTOCOL)
                .or_else(|_| env::var(ENV_READINESS_CHECK_PROTOCOL_DEPRECATED))
                .unwrap_or("http".to_string())
                .as_str()
                .into(),
            healthy_status: env::var(ENV_READINESS_CHECK_HEALTHY_STATUS)
                .unwrap_or("100-499".to_string())
                .parse()
                .unwrap_or(StatusRange::default()),
        }
    }
}
```

### HTTP Readiness Check

```rust
async fn http_readiness_check(
    host: &str,
    port: u16,
    path: &str,
    healthy_status: &StatusRange,
) -> Result<Checkpoint, Error> {
    let url = format!("http://{}:{}{}", host, port, path);
    let req = Request::builder()
        .method(Method::GET)
        .uri(&url)
        .body(Body::empty())?;

    let client = hyper_util::client::legacy::Client::builder(
        hyper_util::rt::TokioExecutor::new()
    ).build(HttpConnector::new());

    match client.request(req).await {
        Ok(response) => {
            let status = response.status().as_u16();
            if healthy_status.contains(status) {
                Ok(Checkpoint::Healthy)
            } else {
                Ok(Checkpoint::Unhealthy(status))
            }
        }
        Err(_) => Ok(Checkpoint::Unreachable),
    }
}
```

### TCP Readiness Check

```rust
async fn tcp_readiness_check(host: &str, port: u16) -> Result<Checkpoint, Error> {
    let addr = format!("{}:{}", host, port);
    match TcpStream::connect(&addr).await {
        Ok(_) => Ok(Checkpoint::Healthy),
        Err(_) => Ok(Checkpoint::Unreachable),
    }
}
```

### Readiness Polling Loop

```rust
pub async fn check_init_health(&mut self) {
    let retry_strategy = FixedInterval::from_millis(100).take(300); // 30 second timeout

    Retry::spawn(retry_strategy, || async {
        let checkpoint = match self.options.readiness_check.protocol {
            Protocol::Http => {
                http_readiness_check(
                    &self.options.host,
                    self.options.readiness_check.port,
                    &self.options.readiness_check.path,
                    &self.options.readiness_check.healthy_status,
                ).await?
            }
            Protocol::Tcp => {
                tcp_readiness_check(
                    &self.options.host,
                    self.options.readiness_check.port,
                ).await?
            }
        };

        match checkpoint {
            Checkpoint::Healthy => Ok(()),
            Checkpoint::Unhealthy(status) => {
                tracing::warn!("Health check returned unhealthy status: {}", status);
                Err(ReadinessError::Unhealthy(status))
            }
            Checkpoint::Unreachable => {
                tracing::warn!("Health check unreachable");
                Err(ReadinessError::Unreachable)
            }
        }
    })
    .await
    .expect("Web application failed to become ready");

    self.checkpoint.store(true, Ordering::Relaxed);
    tracing::info!("Web application is ready");
}
```

---

## Part 4: Extension Registration

### Register as Extension

```rust
pub fn register_default_extension(&mut self) {
    let registration = ExtensionRegistration {
        events: vec![],  // Don't subscribe to any events
        extension_name: "lambda-web-adapter".to_string(),
    };

    // Register with Runtime API
    let response = self.register_extension(registration)
        .expect("Failed to register extension");

    self.extension_id = Some(response.extension_id);
    tracing::info!("Registered as extension: {}", response.extension_id);
}
```

### Extension Registration Request

```rust
async fn register_extension(
    &self,
    registration: ExtensionRegistration,
) -> Result<ExtensionRegistrationResponse, Error> {
    let url = format!(
        "http://{}/2020-01-01/extension/register",
        env::var(ENV_LAMBDA_RUNTIME_API).unwrap()
    );

    let body = serde_json::to_string(&registration)?;

    let req = build_request()
        .method(Method::POST)
        .uri(&url)
        .body(Body::from(body))?;

    let response = self.client.request(req).await?;
    let body = response.into_body().collect().await?.to_bytes();

    Ok(serde_json::from_slice(&body)?)
}
```

---

## Part 5: Event Loop Implementation

### Main Run Loop

```rust
pub async fn run(&mut self) -> Result<(), Error> {
    tracing::info!("Starting Lambda Web Adapter");

    // Wait for readiness
    self.check_init_health().await;

    // Enter invocation loop
    loop {
        // 1. Get next invocation from Runtime API
        let (request_id, event) = self.get_next_invocation().await?;

        // 2. Translate Lambda event to HTTP request
        let http_request = self.translate_to_http(&event)?;

        // 3. Forward to web application
        let http_response = self.forward_to_web_app(http_request).await?;

        // 4. Translate HTTP response to Lambda response
        let lambda_response = self.translate_to_lambda(http_response)?;

        // 5. Send response to Runtime API
        self.send_response(&request_id, lambda_response).await?;

        tracing::debug!("Completed invocation: {}", request_id);
    }
}
```

### Get Next Invocation

```rust
async fn get_next_invocation(&self) -> Result<(String, LambdaEvent), Error> {
    let url = format!(
        "http://{}/2018-06-01/runtime/invocation/next",
        env::var(ENV_LAMBDA_RUNTIME_API).unwrap()
    );

    let req = build_request()
        .method(Method::GET)
        .uri(&url)
        .body(Body::empty())?;

    let response = self.client.request(req).await?;

    // Extract headers
    let headers = response.headers().clone();
    let request_id = headers
        .get("Lambda-Runtime-Aws-Request-Id")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::MissingRequestID)?
        .to_string();

    // Read body
    let body = response.into_body().collect().await?.to_bytes();

    // Parse based on invoke mode
    let event = if self.invoke_mode == LambdaInvokeMode::ResponseStream {
        LambdaEvent::Streaming(body.to_vec())
    } else {
        LambdaEvent::Buffered(body.to_vec())
    };

    Ok((request_id, event))
}
```

---

## Part 6: HTTP Translation

### Event to HTTP Request

```rust
fn translate_to_http(&self, event: &LambdaEvent) -> Result<Request<Body>, Error> {
    match &event {
        LambdaEvent::ApiGatewayV2(api_event) => {
            // Extract method
            let method = Method::from_str(&api_event.request_context.http.method)?;

            // Build URI
            let uri = format!(
                "http://{}:{}{}",
                self.options.host,
                self.options.port,
                api_event.raw_path
            );

            // Build request
            let mut builder = Request::builder()
                .method(method)
                .uri(&uri);

            // Add headers
            for (key, value) in &api_event.headers {
                builder = builder.header(key, value);
            }

            // Add body
            let body = if api_event.is_base64_encoded {
                let decoded = base64::decode(&api_event.body)?;
                Body::from(decoded)
            } else {
                Body::from(api_event.body.clone())
            };

            Ok(builder.body(body)?)
        }

        LambdaEvent::Alb(alb_event) => {
            // Similar translation for ALB events
            // ...
        }

        LambdaEvent::PassThrough(raw) => {
            // For non-HTTP events, forward to pass-through path
            let uri = format!(
                "http://{}:{}{}",
                self.options.host,
                self.options.port,
                self.options.pass_through_path
            );

            Ok(Request::builder()
                .method(Method::POST)
                .uri(&uri)
                .header("Content-Type", "application/json")
                .body(Body::from(raw.clone()))?)
        }
    }
}
```

### HTTP Response to Lambda Response

```rust
fn translate_to_lambda(&self, response: Response<Body>) -> Result<LambdaResponse, Error> {
    let status_code = response.status().as_u16();

    // Convert headers
    let mut headers = HashMap::new();
    for (key, value) in response.headers() {
        headers.insert(
            key.to_string(),
            value.to_str().unwrap_or("").to_string(),
        );
    }

    // Read body
    let body_bytes = response.into_body().collect().await?.to_bytes();
    let body_string = String::from_utf8_lossy(&body_bytes).to_string();

    // Detect if binary (for base64 encoding)
    let is_base64_encoded = self.detect_if_binary(&body_bytes);

    let body = if is_base64_encoded {
        base64::encode(&body_bytes)
    } else {
        body_string
    };

    Ok(LambdaResponse {
        status_code,
        headers,
        body,
        is_base64_encoded,
    })
}
```

### Binary Detection

```rust
fn detect_if_binary(&self, bytes: &[u8]) -> bool {
    // Check for null bytes (common in binary files)
    if bytes.contains(&0) {
        return true;
    }

    // Check for high proportion of non-printable characters
    let non_printable = bytes
        .iter()
        .filter(|&&b| b < 32 || b > 126)
        .count();

    let ratio = non_printable as f32 / bytes.len() as f32;
    ratio > 0.3  // More than 30% non-printable = binary
}
```

---

## Part 7: Response Streaming

### Streaming Mode Configuration

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LambdaInvokeMode {
    #[default]
    Buffered,
    ResponseStream,
}

impl From<&str> for LambdaInvokeMode {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "response_stream" => LambdaInvokeMode::ResponseStream,
            _ => LambdaInvokeMode::Buffered,
        }
    }
}
```

### Streaming Forward

```rust
async fn forward_streaming(
    &self,
    http_request: Request<Body>,
    request_id: &str,
) -> Result<(), Error> {
    let url = format!(
        "http://{}/2018-06-01/runtime/invocation/{}/response",
        env::var(ENV_LAMBDA_RUNTIME_API).unwrap(),
        request_id
    );

    let response = self.client.request(http_request).await?;
    let mut stream = response.into_body();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        // Forward chunk to Runtime API
        let req = build_request()
            .method(Method::POST)
            .uri(&url)
            .body(Body::from(chunk))?;

        self.client.request(req).await?;
    }

    Ok(())
}
```

---

## Part 8: Compression

### Compression Layer

```rust
fn build_compressed_client() -> Client<HttpConnector, Body> {
    let service = ServiceBuilder::new()
        .layer(CompressionLayer::new()
            .gzip(true)
            .br(true)  // Brotli
        )
        .service(hyper_util::client::legacy::Client::builder(
            hyper_util::rt::TokioExecutor::new()
        ).build(HttpConnector::new()));

    Client::new(service)
}
```

### Compression Configuration

```rust
// Enable via environment variable
if env::var(ENV_ENABLE_COMPRESSION)
    .unwrap_or("false".to_string())
    .parse()
    .unwrap_or(false)
{
    // Use compressed client
    self.client = build_compressed_client();
}
```

---

## Part 9: Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Missing AWS_LAMBDA_RUNTIME_API")]
    MissingRuntimeAPI,

    #[error("Missing request ID")]
    MissingRequestID,

    #[error("HTTP error: {0}")]
    Http(#[from] http::Error),

    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Readiness check failed")]
    Readiness(#[from] ReadinessError),
}

#[derive(Debug, thiserror::Error)]
pub enum ReadinessError {
    #[error("Unhealthy status: {0}")]
    Unhealthy(u16),

    #[error("Endpoint unreachable")]
    Unreachable,
}
```

### Graceful Error Handling

```rust
async fn handle_invocation(&mut self) -> Result<(), Error> {
    match self.get_next_invocation().await {
        Ok((request_id, event)) => {
            match self.process_invocation(request_id, event).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::error!("Invocation error: {}", e);
                    self.send_error(&request_id, &e).await?;
                    Ok(())  // Continue loop
                }
            }
        }
        Err(e) => {
            tracing::error!("Get invocation error: {}", e);
            Err(e)  // May need to restart
        }
    }
}
```

---

## Summary

| Component | Purpose | Key Methods |
|-----------|---------|-------------|
| **AdapterOptions** | Configuration | `from_env()`, `default()` |
| **ReadinessConfig** | Health check settings | `http_check()`, `tcp_check()` |
| **Adapter** | Main adapter logic | `run()`, `register_extension()` |
| **Translation** | Event ↔ HTTP | `translate_to_http()`, `translate_to_lambda()` |
| **Streaming** | Response streaming | `forward_streaming()` |
| **Compression** | Response compression | `build_compressed_client()` |

---

*Continue to [03-valtron-integration.md](03-valtron-integration.md) to learn how to implement this pattern using Valtron instead of Tokio.*
