# Superglue Rust Replication Plan

**Target:** Build a production-ready Rust implementation of Superglue

---

## Table of Contents

1. [Why Rust?](#why-rust)
2. [Architecture Overview](#architecture-overview)
3. [Crate Ecosystem](#crate-ecosystem)
4. [Project Structure](#project-structure)
5. [Core Type Definitions](#core-type-definitions)
6. [Implementation Guide](#implementation-guide)
7. [Performance Optimizations](#performance-optimizations)
8. [Deployment Strategy](#deployment-strategy)

---

## Why Rust?

### Performance Benefits

| Metric | Node.js (TypeScript) | Rust |
|--------|---------------------|------|
| Binary Size | ~500MB (with Node) | ~20MB |
| Memory Usage | 200-500MB typical | 50-100MB typical |
| Request Latency | 50-200ms overhead | <10ms overhead |
| Concurrent Requests | Limited by event loop | True parallel execution |
| Cold Start | 1-5 seconds | <100ms |

### Development Benefits

1. **Type Safety** - Compile-time guarantees, no runtime type errors
2. **Memory Safety** - No garbage collector, no memory leaks
3. **Concurrency** - Fearless concurrency with ownership model
4. **Error Handling** - Result<T, E> forces error handling
5. **Deployment** - Single binary, no npm/node dependencies

### Trade-offs

| Aspect | Benefit | Cost |
|--------|---------|------|
| Performance | 10-50x faster | Longer compile times |
| Memory | 5x less usage | Steeper learning curve |
| Safety | Compile-time checks | More verbose code |
| Deployment | Single binary | Larger team onboarding |

---

## Architecture Overview

### System Design

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CLIENT LAYER                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │  JavaScript SDK │  │   GraphQL CLI   │  │  External Apps  │     │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘     │
└───────────┼────────────────────┼────────────────────┼───────────────┘
            │                    │                    │
            └────────────────────┼────────────────────┘
                                 │
            ┌────────────────────▼────────────────────┐
            │         GRAPHQL API LAYER                │
            │         (async-graphql)                  │
            │  Port: 3000 (GraphQL)                    │
            │  Port: 3001 (Dashboard - separate)       │
            └────────────────────┬────────────────────┘
                                 │
            ┌────────────────────▼────────────────────┐
            │           CORE ENGINE                    │
            │  ┌─────────────────────────────────┐    │
            │  │  Resolver Layer (Schema)         │    │
            │  │  - QueryRoot                     │    │
            │  │  - MutationRoot                  │    │
            │  └─────────────────────────────────┘    │
            │  ┌─────────────────────────────────┐    │
            │  │  Service Layer                   │    │
            │  │  - ApiService                    │    │
            │  │  - ExtractService                │    │
            │  │  - TransformService              │    │
            │  │  - FileService                   │    │
            │  └─────────────────────────────────┘    │
            └────────────────────┬────────────────────┘
                                 │
            ┌────────────────────▼────────────────────┐
            │          DATASTORE LAYER                 │
            │  ┌─────────────┐  ┌─────────────────┐   │
            │  │   Redis     │  │   In-Memory     │   │
            │  │   (Prod)    │  │   (Dev/Test)    │   │
            │  └─────────────┘  └─────────────────┘   │
            └─────────────────────────────────────────┘
                                 │
            ┌────────────────────▼────────────────────┐
            │        EXTERNAL DATA SOURCES             │
            │  REST APIs │ GraphQL │ Files │ Legacy   │
            └─────────────────────────────────────────┘
```

### Crate Organization

```
superglue-rs/
├── Cargo.toml              # Workspace definition
├── Cargo.lock
├── README.md
├── docker-compose.yml
├── .env.example
│
├── crates/
│   ├── superglue-types/    # Shared type definitions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs
│   │
│   ├── superglue-core/     # Core transformation engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── transform.rs
│   │       ├── extract.rs
│   │       └── expression.rs
│   │
│   ├── superglue-api/      # GraphQL API layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── schema.rs
│   │       ├── query.rs
│   │       └── mutation.rs
│   │
│   ├── superglue-cache/    # Redis caching layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── redis.rs
│   │
│   ├── superglue-extract/  # Data extraction
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── http.rs
│   │       └── file.rs
│   │
│   ├── superglue-transform/# LLM-powered transformations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── llm.rs
│   │       └── jsonata.rs
│   │
│   └── superglue-schema/   # Schema validation
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           └── validator.rs
│
└── src/
    └── main.rs             # Application entry point
```

---

## Crate Ecosystem

### Core Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.35 | Async runtime |
| `serde` | 1.0 | Serialization |
| `serde_json` | 1.0 | JSON processing |
| `async-graphql` | 7.0 | GraphQL server |
| `reqwest` | 0.11 | HTTP client |
| `redis` | 0.24 | Redis client |
| `bb8` | 0.8 | Connection pooling |

### Data Processing

| Crate | Version | Purpose |
|-------|---------|---------|
| `csv` | 1.3 | CSV parsing |
| `quick-xml` | 0.31 | XML parsing |
| `calamine` | 0.23 | Excel files |
| `flate2` | 1.0 | GZIP decompression |
| `zip` | 0.6 | ZIP archives |

### LLM Integration

| Crate | Version | Purpose |
|-------|---------|---------|
| `async-openai` | 0.19 | OpenAI API client |
| `jsonschema` | 0.17 | JSON Schema validation |

### Observability

| Crate | Version | Purpose |
|-------|---------|---------|
| `tracing` | 0.1 | Structured logging |
| `tracing-subscriber` | 0.3 | Logging subscriber |
| `opentelemetry` | 0.21 | Telemetry |
| `opentelemetry-otlp` | 0.14 | OTLP exporter |

### Utilities

| Crate | Version | Purpose |
|-------|---------|---------|
| `chrono` | 0.4 | Date/time |
| `uuid` | 1.6 | UUID generation |
| `md-5` | 0.10 | Hash generation |
| `thiserror` | 1.0 | Error types |
| `anyhow` | 1.0 | Error handling |
| `secrecy` | 0.8 | Secret handling |
| `governor` | 0.6 | Rate limiting |

---

## Project Structure

### Workspace Configuration

**File:** `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/*",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
authors = ["Superglue Team"]

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
tokio-util = "0.7"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# GraphQL
async-graphql = { version = "7.0", features = ["chrono", "uuid"] }
async-graphql-axum = "7.0"

# HTTP
reqwest = { version = "0.11", features = ["json", "gzip"] }
axum = "0.7"

# Redis
redis = { version = "0.24", features = ["tokio-comp", "connection-manager"] }
bb8 = "0.8"
bb8-redis = "0.13"

# Data processing
csv = "1.3"
quick-xml = { version = "0.31", features = ["serialize"] }
calamine = "0.23"
flate2 = "1.0"
zip = "0.6"

# LLM
async-openai = "0.19"
jsonschema = "0.17"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
opentelemetry = "0.21"
opentelemetry-otlp = "0.14"
tracing-opentelemetry = "0.22"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
md-5 = "0.10"
thiserror = "1.0"
anyhow = "1.0"
secrecy = { version = "0.8", features = ["serde"] }
governor = "0.6"

# Testing
mockall = "0.12"
criterion = "0.5"
```

### Main Entry Point

**File:** `src/main.rs`

```rust
use anyhow::Result;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use superglue_api::{create_schema, AppState};
use superglue_cache::CacheService;
use superglue_transform::TransformEngine;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize services
    let config = load_config()?;

    // Redis cache
    let cache = CacheService::new(
        &config.redis_host,
        config.redis_port,
        config.redis_password.as_deref(),
    ).await?;

    // Transform engine (LLM)
    let transform_engine = TransformEngine::new(
        &config.openai_api_key,
        &config.openai_model,
        config.openai_base_url.as_deref(),
    );

    // Application state
    let state = Arc::new(AppState {
        datastore: Arc::new(cache),
        transform_engine: Arc::new(transform_engine),
    });

    // Create GraphQL schema
    let schema = create_schema(state.clone());

    // Create Axum router
    let app = axum::Router::new()
        .route("/graphql", axum::routing::post(graphql_handler))
        .route("/health", axum::routing::get(health_handler))
        .with_state(state);

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 Superglue server running at {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

async fn graphql_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    axum::Json(req): axum::Json<async_graphql::Request>,
) -> axum::Json<async_graphql::Response> {
    let schema = create_schema(state);
    axum::Json(schema.execute(req).await)
}

async fn health_handler() -> &'static str {
    "OK"
}

fn load_config() -> Result<Config> {
    Ok(Config {
        host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: std::env::var("PORT").unwrap_or_else(|_| "3000".to_string()),
        redis_host: std::env::var("REDIS_HOST")?,
        redis_port: std::env::var("REDIS_PORT")?.parse()?,
        redis_password: std::env::var("REDIS_PASSWORD").ok(),
        openai_api_key: std::env::var("OPENAI_API_KEY")?,
        openai_model: std::env::var("OPENAI_MODEL")?,
        openai_base_url: std::env::var("OPENAI_BASE_URL").ok(),
    })
}

struct Config {
    host: String,
    port: String,
    redis_host: String,
    redis_port: u16,
    redis_password: Option<String>,
    openai_api_key: String,
    openai_model: String,
    openai_base_url: Option<String>,
}
```

---

## Core Type Definitions

**File:** `crates/superglue-types/src/lib.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

/// Authentication types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuthType {
    None,
    Header,
    QueryParam,
    OAuth2,
}

/// File types for extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileType {
    Csv,
    Json,
    Xml,
    Excel,
    Auto,
}

/// Decompression methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecompressionMethod {
    Gzip,
    Deflate,
    Zip,
    None,
    Auto,
}

/// Pagination types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaginationType {
    OffsetBased,
    PageBased,
    Disabled,
}

/// Cache modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CacheMode {
    Enabled,
    Disabled,
    ReadOnly,
    WriteOnly,
}

/// Pagination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub pagination_type: PaginationType,
    pub page_size: Option<u32>,
}

/// Base configuration interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    pub id: String,
    pub version: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// API Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(flatten)]
    pub base: BaseConfig,
    pub url_host: String,
    pub url_path: Option<String>,
    pub instruction: String,
    pub method: Option<HttpMethod>,
    pub query_params: Option<HashMap<String, serde_json::Value>>,
    pub headers: Option<HashMap<String, serde_json::Value>>,
    pub body: Option<String>,
    pub documentation_url: Option<String>,
    pub response_schema: Option<serde_json::Value>,
    pub response_mapping: Option<String>,
    pub authentication: Option<AuthType>,
    pub pagination: Option<Pagination>,
    pub data_path: Option<String>,
}

/// Extract Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractConfig {
    #[serde(flatten)]
    pub base: BaseConfig,
    pub url_host: String,
    pub url_path: Option<String>,
    pub instruction: String,
    pub method: Option<HttpMethod>,
    pub query_params: Option<HashMap<String, serde_json::Value>>,
    pub headers: Option<HashMap<String, serde_json::Value>>,
    pub body: Option<String>,
    pub documentation_url: Option<String>,
    pub decompression_method: Option<DecompressionMethod>,
    pub authentication: Option<AuthType>,
    pub file_type: Option<FileType>,
    pub data_path: Option<String>,
}

/// Transform Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    #[serde(flatten)]
    pub base: BaseConfig,
    pub instruction: String,
    pub response_schema: serde_json::Value,
    pub response_mapping: Option<String>,
    pub confidence: Option<f64>,
    pub confidence_reasoning: Option<String>,
}

/// Configuration type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigType {
    Api(ApiConfig),
    Extract(ExtractConfig),
    Transform(TransformConfig),
}

/// Run result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub id: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub config: Option<ConfigType>,
}

/// Request options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestOptions {
    pub cache_mode: Option<CacheMode>,
    pub timeout: Option<u64>,
    pub retries: Option<u32>,
    pub retry_delay: Option<u64>,
    pub webhook_url: Option<String>,
}

/// API Input for calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInput {
    pub url_host: String,
    pub url_path: Option<String>,
    pub instruction: String,
    pub method: Option<HttpMethod>,
    pub query_params: Option<HashMap<String, serde_json::Value>>,
    pub headers: Option<HashMap<String, serde_json::Value>>,
    pub body: Option<String>,
    pub documentation_url: Option<String>,
    pub response_schema: Option<serde_json::Value>,
    pub response_mapping: Option<String>,
    pub authentication: Option<AuthType>,
    pub pagination: Option<Pagination>,
    pub data_path: Option<String>,
}

/// Extract Input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractInput {
    pub url_host: String,
    pub url_path: Option<String>,
    pub instruction: String,
    pub method: Option<HttpMethod>,
    pub query_params: Option<HashMap<String, serde_json::Value>>,
    pub headers: Option<HashMap<String, serde_json::Value>>,
    pub body: Option<String>,
    pub documentation_url: Option<String>,
    pub decompression_method: Option<DecompressionMethod>,
    pub authentication: Option<AuthType>,
    pub file_type: Option<FileType>,
}

/// Transform Input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformInput {
    pub instruction: String,
    pub response_schema: serde_json::Value,
    pub response_mapping: Option<String>,
}
```

---

## Implementation Guide

### 1. Transform Engine

**File:** `crates/superglue-transform/src/llm.rs`

```rust
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage,
        CreateChatCompletionRequest,
        ResponseFormat,
        ResponseFormatJsonSchema,
    },
};
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransformError {
    #[error("OpenAI API error: {0}")]
    OpenAI(#[from] async_openai::error::OpenAIError),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Max retries exceeded")]
    MaxRetries,
}

pub struct TransformMapping {
    pub jsonata: String,
    pub confidence: f64,
    pub confidence_reasoning: String,
}

pub struct TransformEngine {
    client: Client<OpenAIConfig>,
    model: String,
}

impl TransformEngine {
    pub fn new(api_key: &str, model: &str, base_url: Option<&str>) -> Self {
        let config = OpenAIConfig::new(api_key)
            .with_api_base(base_url.unwrap_or("https://api.openai.com/v1"));

        Self {
            client: Client::with_config(config),
            model: model.to_string(),
        }
    }

    pub async fn generate_mapping(
        &self,
        schema: &Value,
        source_data: &Value,
        instruction: Option<&str>,
    ) -> Result<TransformMapping, TransformError> {
        self.generate_mapping_with_retry(schema, source_data, instruction, 0, Vec::new())
            .await
    }

    async fn generate_mapping_with_retry(
        &self,
        schema: &Value,
        source_data: &Value,
        instruction: Option<&str>,
        retry_count: u32,
        mut messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<TransformMapping, TransformError> {
        const MAX_RETRIES: u32 = 5;

        // Initialize messages if first call
        if messages.is_empty() {
            messages = vec![
                ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessage {
                        content: PROMPT_MAPPING.to_string(),
                        ..Default::default()
                    }
                ),
                ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: self.build_user_prompt(schema, source_data, instruction),
                        ..Default::default()
                    }
                ),
            ];
        }

        // Temperature increases with retries
        let temperature = if self.model.starts_with('o') {
            None
        } else {
            Some((retry_count as f32 * 0.1).min(1.0))
        };

        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: messages.clone(),
            response_format: Some(ResponseFormat::JsonSchema {
                json_schema: ResponseFormatJsonSchema {
                    name: "jsonata_expression".to_string(),
                    schema: Some(JSONATA_SCHEMA.clone()),
                    ..Default::default()
                },
            }),
            temperature,
            ..Default::default()
        };

        let response = self.client.chat().create(request).await?;
        let content = response.choices[0]
            .message
            .content
            .as_ref()
            .ok_or_else(|| TransformError::Validation("No content in response".to_string()))?;

        // Parse response
        let mapping: GeneratedMapping = serde_json::from_str(content)?;
        messages.push(ChatCompletionRequestMessage::Assistant(
            async_openai::types::ChatCompletionRequestAssistantMessage {
                content: Some(content.clone()),
                ..Default::default()
            }
        ));

        // Validate the generated expression
        if let Err(e) = self.validate_mapping(source_data, &mapping.jsonata, schema).await {
            if retry_count < MAX_RETRIES {
                // Add error to messages and retry
                messages.push(ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: format!("Validation failed: {}. Please try again.", e),
                        ..Default::default()
                    }
                ));

                return self.generate_mapping_with_retry(
                    schema,
                    source_data,
                    instruction,
                    retry_count + 1,
                    messages,
                ).await;
            }

            return Err(TransformError::MaxRetries);
        }

        Ok(TransformMapping {
            jsonata: mapping.jsonata,
            confidence: mapping.confidence,
            confidence_reasoning: mapping.confidence_reasoning,
        })
    }

    fn build_user_prompt(
        &self,
        schema: &Value,
        source_data: &Value,
        instruction: Option<&str>,
    ) -> String {
        let mut prompt = format!(
            "Given a source data and structure, create a jsonata expression in JSON FORMAT.\n\n\
             Target Schema:\n{}\n\n",
            serde_json::to_string_pretty(schema).unwrap()
        );

        if let Some(instr) = instruction {
            prompt.push_str(&format!("Instruction: {}\n\n", instr));
        }

        // Sample and limit to 30KB
        let sampled = self.sample_data(source_data, 10);
        let data_str = serde_json::to_string_pretty(&sampled).unwrap();
        prompt.push_str(&format!(
            "Source Data Structure:\n{}\n\nSource Data Sample:\n{}",
            data_str,
            &data_str[..data_str.len().min(30000)]
        ));

        prompt
    }

    fn sample_data(&self, value: &Value, sample_size: usize) -> Value {
        match value {
            Value::Array(arr) => {
                if arr.len() <= sample_size {
                    Value::Array(arr.iter().map(|v| self.sample_data(v, sample_size)).collect())
                } else {
                    let step = arr.len() / sample_size;
                    Value::Array(
                        (0..sample_size)
                            .map(|i| self.sample_data(&arr[i * step], sample_size))
                            .collect()
                    )
                }
            }
            Value::Object(obj) => {
                Value::Object(
                    obj.iter()
                        .map(|(k, v)| (k.clone(), self.sample_data(v, sample_size)))
                        .collect()
                )
            }
            _ => value.clone(),
        }
    }

    async fn validate_mapping(
        &self,
        data: &Value,
        expr: &str,
        schema: &Value,
    ) -> Result<(), String> {
        // This would use a JSONata executor or custom expression engine
        // For now, placeholder
        Ok(())
    }
}

struct GeneratedMapping {
    jsonata: String,
    confidence: f64,
    confidence_reasoning: String,
}

// JSONata response schema
lazy_static::lazy_static! {
    static ref JSONATA_SCHEMA: Value = serde_json::json!({
        "type": "object",
        "properties": {
            "jsonata": {
                "type": "string",
                "description": "JSONata expression"
            },
            "confidence": {
                "type": "number",
                "description": "Confidence score 0-100"
            },
            "confidence_reasoning": {
                "type": "string",
                "description": "Reasoning for confidence score"
            }
        },
        "required": ["jsonata", "confidence", "confidence_reasoning"],
        "additionalProperties": false
    });
}

const PROMPT_MAPPING: &str = r#"You are an AI that generates JSONata mapping expressions..."#;
```

### 2. Redis Cache Service

**File:** `crates/superglue-cache/src/redis.rs`

```rust
use bb8_redis::{RedisConnectionManager, bb8::Pool};
use redis::AsyncCommands;
use serde::{Serialize, de::DeserializeOwned};
use md5::{Md5, Digest};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct CacheService {
    pool: Pool<RedisConnectionManager>,
    ttl: u64,
}

impl CacheService {
    pub async fn new(
        host: &str,
        port: u16,
        password: Option<&str>,
    ) -> Result<Self, CacheError> {
        let redis_url = if let Some(pwd) = password {
            format!("redis://:{}@{}:{}", pwd, host, port)
        } else {
            format!("redis://{}:{}", host, port)
        };

        let manager = RedisConnectionManager::new(redis_url)?;
        let pool = Pool::builder().build(manager).await?;

        Ok(Self {
            pool,
            ttl: 60 * 60 * 24 * 90, // 90 days
        })
    }

    fn make_key(&self, prefix: &str, id: &str, org_id: Option<&str>) -> String {
        match org_id {
            Some(org) => format!("{}:{}:{}", org, prefix, id),
            None => format!("{}:{}", prefix, id),
        }
    }

    fn generate_hash<T: Serialize>(data: &T) -> String {
        let json = serde_json::to_string(data).unwrap();
        let mut hasher = Md5::new();
        hasher.update(json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub async fn get<T: DeserializeOwned>(
        &self,
        prefix: &str,
        id: &str,
        org_id: Option<&str>,
    ) -> Result<Option<T>, CacheError> {
        let mut conn = self.pool.get().await?;
        let key = self.make_key(prefix, id, org_id);

        let data: Option<String> = conn.get(&key).await?;

        match data {
            Some(json) => Ok(Some(serde_json::from_str(&json)?)),
            None => Ok(None),
        }
    }

    pub async fn set<T: Serialize>(
        &self,
        prefix: &str,
        id: &str,
        value: &T,
        org_id: Option<&str>,
    ) -> Result<(), CacheError> {
        let mut conn = self.pool.get().await?;
        let key = self.make_key(prefix, id, org_id);
        let json = serde_json::to_string(value)?;

        redis::cmd("SET")
            .arg(&key)
            .arg(&json)
            .arg("EX")
            .arg(self.ttl)
            .query_async(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn delete(&self, prefix: &str, id: &str, org_id: Option<&str>) -> Result<bool, CacheError> {
        let mut conn = self.pool.get().await?;
        let key = self.make_key(prefix, id, org_id);

        let deleted: u32 = redis::cmd("DEL").arg(&key).query_async(&mut conn).await?;
        Ok(deleted > 0)
    }

    pub async fn list<T: DeserializeOwned>(
        &self,
        prefix: &str,
        org_id: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<T>, usize), CacheError> {
        let mut conn = self.pool.get().await?;
        let pattern = match org_id {
            Some(org) => format!("{}:{}*", org, prefix),
            None => format!("{}*", prefix),
        };

        let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query_async(&mut conn).await?;
        let total = keys.len();

        // Slice for pagination
        let sliced_keys: Vec<&String> = keys.iter().skip(offset).take(limit).collect();

        if sliced_keys.is_empty() {
            return Ok((Vec::new(), total));
        }

        // Pipeline for batch get
        let values: Vec<Option<String>> = redis::pipe()
            .atomic()
            .get(&sliced_keys)
            .query_async(&mut conn)
            .await?;

        let items: Result<Vec<T>, _> = values
            .into_iter()
            .filter_map(|v| v.map(|json| serde_json::from_str(&json)).transpose())
            .collect();

        Ok((items?, total))
    }
}
```

### 3. GraphQL Schema

**File:** `crates/superglue-api/src/schema.rs`

```rust
use async_graphql::*;
use std::sync::Arc;

use superglue_core::DataStore;
use superglue_types::*;

pub struct AppState {
    pub datastore: Arc<dyn DataStore>,
    pub transform_engine: Arc<dyn TransformEngine>,
}

pub fn create_schema(state: Arc<AppState>) -> Schema {
    Schema::build(
        QueryRoot { state: state.clone() },
        MutationRoot { state },
        EmptySubscription,
    )
    .finish()
}

pub struct QueryRoot {
    state: Arc<AppState>,
}

#[Object]
impl QueryRoot {
    async fn list_runs(
        &self,
        limit: i32,
        offset: i32,
        config_id: Option<ID>,
    ) -> Result<RunList> {
        let (items, total) = self.state.datastore
            .list_runs(limit as usize, offset as usize, config_id.map(|id| id.to_string()))
            .await?;

        Ok(RunList { items, total })
    }

    async fn get_run(&self, id: ID) -> Result<Option<RunResult>> {
        self.state.datastore.get_run(&id.to_string()).await
    }

    async fn list_apis(&self, limit: i32, offset: i32) -> Result<ApiList> {
        let (items, total) = self.state.datastore
            .list_api_configs(limit as usize, offset as usize, None)
            .await?;

        Ok(ApiList { items, total })
    }

    async fn get_api(&self, id: ID) -> Result<Option<ApiConfig>> {
        self.state.datastore.get_api_config(&id.to_string(), None).await
    }

    async fn generate_schema(
        &self,
        instruction: String,
        response_data: String,
    ) -> Result<serde_json::Value> {
        // Use LLM to generate schema
        todo!()
    }
}

pub struct MutationRoot {
    state: Arc<AppState>,
}

#[Object]
impl MutationRoot {
    async fn call(
        &self,
        input: ApiInputRequest,
        payload: Option<JsonObject>,
        credentials: Option<JsonObject>,
        options: Option<RequestOptions>,
    ) -> Result<RunResult> {
        // Implement call logic
        todo!()
    }

    async fn transform(
        &self,
        input: TransformInputRequest,
        data: JsonObject,
        options: Option<RequestOptions>,
    ) -> Result<RunResult> {
        // Implement transform logic
        todo!()
    }

    async fn upsert_api(&self, id: ID, input: JsonObject) -> Result<ApiConfig> {
        let config: ApiConfig = serde_json::from_value(serde_json::to_value(input)?)?;
        self.state.datastore.upsert_api_config(&id.to_string(), config, None).await
    }

    async fn delete_api(&self, id: ID) -> Result<bool> {
        self.state.datastore.delete_api_config(&id.to_string(), None).await
    }
}

// GraphQL wrapper types
#[derive(SimpleObject)]
pub struct RunList {
    items: Vec<RunResult>,
    total: usize,
}

#[derive(SimpleObject)]
pub struct ApiList {
    items: Vec<ApiConfig>,
    total: usize,
}

// Input types
#[derive(InputObject)]
pub struct ApiInputRequest {
    id: Option<ID>,
    endpoint: ApiInput,
}

#[derive(InputObject)]
pub struct ApiInput {
    url_host: String,
    url_path: Option<String>,
    instruction: String,
    // ... other fields
}
```

---

## Performance Optimizations

### 1. Connection Pooling

```rust
use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;

// Configure pool for high concurrency
let pool = Pool::builder()
    .max_size(50)           // Max connections
    .min_idle(Some(10))     // Minimum idle connections
    .max_lifetime(Some(std::time::Duration::from_secs(300)))
    .idle_timeout(Some(std::time::Duration::from_secs(60)))
    .build(manager)
    .await?;
```

### 2. Async HTTP with reqwest

```rust
use reqwest::{Client, ClientBuilder};
use std::time::Duration;

// Create optimized HTTP client
let client = ClientBuilder::new()
    .timeout(Duration::from_secs(300))
    .pool_max_idle_per_host(10)
    .tcp_keepalive(Duration::from_secs(30))
    .gzip(true)
    .build()?;

// Use connection pooling automatically
let response = client.get(url).send().await?;
```

### 3. Parallel Processing with tokio

```rust
use tokio::task::JoinSet;

// Process multiple transformations in parallel
async fn process_batch(items: Vec<TransformItem>) -> Vec<Result<Output>> {
    let mut join_set = JoinSet::new();

    for item in items {
        join_set.spawn(async move {
            transform_item(item).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result?);
    }

    results
}
```

### 4. Caching Strategy

```rust
use moka::future::Cache;

// In-memory LRU cache for hot data
let cache = Cache::builder()
    .max_capacity(10_000)
    .time_to_live(std::time::Duration::from_secs(3600))
    .build();

// Get or compute
let result = cache.get_or_insert_with(config_id, || async {
    fetch_and_transform(config_id).await
}).await;
```

---

## Deployment Strategy

### Docker Configuration

**File:** `Dockerfile`

```dockerfile
# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY src/ ./src/

# Build
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/superglue /usr/local/bin/

EXPOSE 3000

ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=3000

CMD ["superglue"]
```

### Docker Compose

**File:** `docker-compose.yml`

```yaml
version: '3.8'

services:
  superglue:
    build: .
    ports:
      - "3000:3000"
    environment:
      - REDIS_HOST=redis
      - REDIS_PORT=6379
      - REDIS_PASSWORD=${REDIS_PASSWORD}
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - OPENAI_MODEL=gpt-4
    depends_on:
      - redis

  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis_data:/data

volumes:
  redis_data:
```

### Kubernetes Manifest

**File:** `k8s/deployment.yaml`

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: superglue
spec:
  replicas: 3
  selector:
    matchLabels:
      app: superglue
  template:
    metadata:
      labels:
        app: superglue
    spec:
      containers:
      - name: superglue
        image: superglue/superglue:latest
        ports:
        - containerPort: 3000
        env:
        - name: REDIS_HOST
          value: "redis-service"
        - name: REDIS_PORT
          value: "6379"
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: superglue-secrets
              key: openai-api-key
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 10
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
```

---

## Summary

### Key Takeaways

1. **Rust provides significant performance benefits** - 10-50x faster, 5x less memory, 25x smaller deployment

2. **Recommended crate ecosystem**:
   - `async-graphql` for GraphQL server
   - `reqwest` for HTTP client
   - `redis` + `bb8` for caching
   - `async-openai` for LLM integration
   - `tokio` for async runtime

3. **Architecture mirrors TypeScript implementation** with Rust-specific optimizations:
   - Connection pooling with bb8
   - True parallel processing with tokio
   - Type-safe serialization with serde

4. **Implementation priorities**:
   - Start with core types and datastore layer
   - Implement transform engine with LLM integration
   - Add GraphQL API layer
   - Optimize with caching and connection pooling

5. **Deployment advantages**:
   - Single binary deployment
   - No npm/node dependencies
   - Better resource utilization
   - Production-ready observability

---

**Document completed:** 2026-03-25
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.superglue/`
