---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/ironclaw
revised_at: 2026-03-22
---

# IronClaw Rust Implementation Deep Dive

This document provides a comprehensive analysis of IronClaw's Rust implementation, covering idiomatic patterns, design decisions, type system usage, and performance considerations.

---

## 1. Crate Overview

### 1.1 Package Structure

```toml
[package]
name = "ironclaw"
version = "0.1.3"
edition = "2024"
rust-version = "1.92"
description = "Secure personal AI assistant that protects your data"
license = "MIT OR Apache-2.0"
```

### 1.2 Library Binary Split

```rust
// src/lib.rs - Library root
pub mod agent;
pub mod channels;
pub mod tools;
// ... module declarations

pub use config::Config;
pub use error::{Error, Result};

pub mod prelude {
    pub use crate::channels::{Channel, IncomingMessage, MessageStream};
    pub use crate::context::{JobContext, JobState};
    pub use crate::tools::{Tool, ToolOutput, ToolRegistry};
}

// src/main.rs - Entry point
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // CLI parsing and bootstrap
}
```

---

## 2. Key Rust Patterns

### 2.1 Trait-Based Abstraction

**Database Trait (~60 methods):**
```rust
#[async_trait]
pub trait Database: Send + Sync {
    // Conversations
    async fn create_conversation(
        &self,
        user_id: &str,
        channel: &str,
        sender_id: &str,
    ) -> Result<Uuid, DatabaseError>;

    async fn get_conversation(
        &self,
        id: Uuid,
    ) -> Result<Conversation, DatabaseError>;

    // Jobs
    async fn create_job(&self, job: &AgentJob) -> Result<Uuid, DatabaseError>;
    async fn update_job_state(
        &self,
        job_id: Uuid,
        state: JobState,
    ) -> Result<(), DatabaseError>;

    // Workspace
    async fn get_document_by_path(
        &self,
        user_id: &str,
        agent_id: Option<Uuid>,
        path: &str,
    ) -> Result<MemoryDocument, WorkspaceError>;

    async fn hybrid_search(
        &self,
        user_id: &str,
        agent_id: Option<Uuid>,
        query: &str,
        embedding: Option<&[f32]>,
        config: &SearchConfig,
    ) -> Result<Vec<SearchResult>, WorkspaceError>;

    // ... 50+ more methods
}
```

**Implementation for PostgreSQL:**
```rust
pub struct Repository {
    pool: Pool,  // deadpool-postgres
}

#[async_trait]
impl Database for Repository {
    async fn create_conversation(
        &self,
        user_id: &str,
        channel: &str,
        sender_id: &str,
    ) -> Result<Uuid, DatabaseError> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "INSERT INTO conversations (user_id, channel, sender_id)
                 VALUES ($1, $2, $3)
                 RETURNING id",
                &[&user_id, &channel, &sender_id],
            )
            .await?;
        Ok(row.get(0))
    }

    // ... other implementations
}
```

**Implementation for libSQL:**
```rust
pub struct LibSqlBackend {
    db: libsql::Database,
}

#[async_trait]
impl Database for LibSqlBackend {
    async fn create_conversation(
        &self,
        user_id: &str,
        channel: &str,
        sender_id: &str,
    ) -> Result<Uuid, DatabaseError> {
        let conn = self.db.connect()?;
        let id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO conversations (id, user_id, channel, sender_id)
             VALUES (?1, ?2, ?3, ?4)",
            &[
                id.to_string().into(),
                user_id.into(),
                channel.into(),
                sender_id.into(),
            ],
        )
        .await?;
        Ok(id)
    }

    // ... other implementations
}
```

### 2.2 Newtype Pattern for Type Safety

```rust
// Strong typing for IDs
pub struct JobId(pub Uuid);
pub struct UserId(pub String);
pub struct SessionId(pub String);

// Usage prevents mixing up parameters
pub async fn get_job(id: JobId) -> Result<Job> {
    // ...
}

// Can't accidentally pass UserId where JobId expected
get_job(user_id);  // Compile error!
```

### 2.3 Builder Pattern for Configuration

```rust
#[derive(Clone, Default)]
pub struct HeartbeatConfig {
    pub interval: Duration,
    pub notify_channel: Option<String>,
    pub notify_user: Option<String>,
    pub max_retries: u32,
}

impl HeartbeatConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn with_notify(
        mut self,
        channel: impl Into<String>,
        user: impl Into<String>,
    ) -> Self {
        self.notify_channel = Some(channel.into());
        self.notify_user = Some(user.into());
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
}

// Usage
let config = HeartbeatConfig::new()
    .with_interval(Duration::from_secs(1800))
    .with_notify("gateway", "default");
```

### 2.4 Strategy Pattern with Trait Objects

```rust
// LLM Provider abstraction
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, messages: &[Message]) -> Result<Completion>;
    async fn count_tokens(&self, messages: &[Message]) -> Result<usize>;
    fn model_name(&self) -> &str;
}

// Multiple implementations
pub struct NearAiProvider {
    client: reqwest::Client,
    config: NearAiConfig,
}

pub struct FailoverProvider {
    primary: Arc<dyn LlmProvider>,
    fallbacks: Vec<Arc<dyn LlmProvider>>,
}

#[async_trait]
impl LlmProvider for FailoverProvider {
    async fn chat(&self, messages: &[Message]) -> Result<Completion> {
        // Try primary first
        match self.primary.chat(messages).await {
            Ok(response) => Ok(response),
            Err(LlmError::RateLimited { .. })
            | Err(LlmError::ModelNotAvailable { .. }) => {
                // Try fallbacks
                for fallback in &self.fallbacks {
                    match fallback.chat(messages).await {
                        Ok(response) => return Ok(response),
                        Err(_) => continue,
                    }
                }
                Err(LlmError::AllProvidersFailed)
            }
            Err(e) => Err(e),
        }
    }
}
```

### 2.5 Repository Pattern for Data Access

```rust
// Abstraction over database operations
pub struct Repository {
    pool: Pool,
}

impl Repository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn get_document_by_path(
        &self,
        user_id: &str,
        agent_id: Option<Uuid>,
        path: &str,
    ) -> Result<MemoryDocument, WorkspaceError> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "SELECT * FROM memory_documents
                 WHERE user_id = $1 AND path = $2",
                &[&user_id, &path],
            )
            .await
            .map_err(|_| WorkspaceError::DocumentNotFound {
                doc_type: "document".into(),
                user_id: user_id.into(),
            })?;
        Ok(row.into())
    }

    pub async fn hybrid_search(
        &self,
        user_id: &str,
        query: &str,
        embedding: Option<&[f32]>,
        limit: usize,
    ) -> Result<Vec<SearchResult>, WorkspaceError> {
        // Uses pgvector for semantic search + tsvector for FTS
        // Reciprocal Rank Fusion to combine results
    }
}
```

---

## 3. Error Handling

### 3.1 Error Type Hierarchy with thiserror

```rust
// src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Channel error: {0}")]
    Channel(#[from] ChannelError),

    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Safety error: {0}")]
    Safety(#[from] SafetyError),

    #[error("Job error: {0}")]
    Job(#[from] JobError),

    // ... more variants
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection pool error: {0}")]
    Pool(String),

    #[error("Query failed: {0}")]
    Query(String),

    #[error("Entity not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[cfg(feature = "postgres")]
    #[error("PostgreSQL error: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    #[cfg(feature = "libsql")]
    #[error("LibSQL error: {0}")]
    LibSql(#[from] libsql::Error),
}
```

### 3.2 Error Context Pattern

```rust
// Map errors with context
.some_async_operation()
    .await
    .map_err(|e| DatabaseError::Query {
        reason: format!("Failed to fetch job: {}", e),
    })?;

// Using anyhow for application-level errors
use anyhow::Context;

let config = std::fs::read_to_string("config.toml")
    .context("Failed to read config file")?;
```

### 3.3 Result Type Alias

```rust
// Consistent return types across codebase
pub type Result<T> = std::result::Result<T, Error>;

// Usage
pub async fn create_job(&self, job: &AgentJob) -> Result<Uuid> {
    // ...
}
```

### 3.4 No Panic Policy

```rust
// Production code: use proper error handling
let value = result.map_err(|e| Error::Config(ConfigError::MissingEnvVar("KEY".into())))?;

// Tests: unwrap() acceptable
#[test]
fn test_something() {
    let result = some_function().unwrap();
    assert_eq!(result, expected);
}
```

---

## 4. Concurrency Model

### 4.1 Tokio Runtime

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Bootstrap
    let (config, db, secrets) = bootstrap::bootstrap().await?;

    // Spawn channels as concurrent tasks
    let channel_handles = vec![
        tokio::spawn(repl_channel.run()),
        tokio::spawn(http_channel.run()),
        tokio::spawn(gateway_channel.run()),
    ];

    // Agent loop in main task
    agent.run().await?;

    Ok(())
}
```

### 4.2 Shared State with Arc

```rust
pub struct AgentDeps {
    pub llm: Arc<dyn LlmProvider>,
    pub tool_registry: Arc<ToolRegistry>,
    pub workspace: Workspace,
    pub safety: Arc<SafetyLayer>,
    pub db: Arc<dyn Database>,
    pub secrets: Arc<SecretsStore>,
}

// Clone Arc is cheap (reference count increment)
impl Clone for AgentDeps {
    fn clone(&self) -> Self {
        Self {
            llm: Arc::clone(&self.llm),
            tool_registry: Arc::clone(&self.tool_registry),
            workspace: self.workspace.clone(),
            safety: Arc::clone(&self.safety),
            db: Arc::clone(&self.db),
            secrets: Arc::clone(&self.secrets),
        }
    }
}
```

### 4.3 Interior Mutability

```rust
// RwLock for read-heavy shared state
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl SessionManager {
    pub async fn get_or_create(&self, sender_id: &str) -> Session {
        // Try read lock first
        {
            let sessions = self.sessions.read().await;
            if let Some(session) = sessions.get(sender_id) {
                return session.clone();
            }
        }

        // Need write lock to create
        let mut sessions = self.sessions.write().await;
        let session = Session::new(sender_id);
        sessions.insert(sender_id.to_string(), session.clone());
        session
    }
}

// AtomicUsize for counters
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct JobCounter {
    count: AtomicUsize,
}

impl JobCounter {
    pub fn increment(&self) -> usize {
        self.count.fetch_add(1, Ordering::SeqCst)
    }
}
```

### 4.4 Channel Communication

```rust
use tokio::sync::mpsc;

// Response channel from agent to channels
let (tx, mut rx) = mpsc::channel::<OutgoingResponse>(100);

// Spawn receiver task
tokio::spawn(async move {
    while let Some(response) = rx.recv().await {
        channel_manager.send(response).await?;
    }
    Ok::<_, ChannelError>(())
});

// Send responses
tx.send(response).await?;
```

### 4.5 Select for Stream Merging

```rust
use futures::stream::{select_all, StreamExt};

// Merge multiple channel streams
let streams: Vec<_> = channels
    .iter()
    .map(|c| c.receive())
    .collect();

let mut merged = select_all(streams);

while let Some(message) = merged.next().await {
    agent.handle_message(message).await?;
}
```

---

## 5. Type System Usage

### 5.1 Enum for State Machines

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobState {
    Pending,
    InProgress,
    Completed,
    Failed { reason: String },
    Stuck { since: DateTime<Utc> },
    Submitted,
    Accepted,
}

impl JobState {
    pub fn can_transition_to(&self, target: &JobState) -> bool {
        matches!(
            (self, target),
            (JobState::Pending, JobState::InProgress) |
            (JobState::InProgress, JobState::Completed) |
            (JobState::InProgress, JobState::Failed { .. }) |
            (JobState::InProgress, JobState::Stuck { .. }) |
            (JobState::Stuck { .. }, JobState::InProgress) |
            (JobState::Stuck { .. }, JobState::Failed { .. }) |
            (JobState::Completed, JobState::Submitted) |
            (JobState::Submitted, JobState::Accepted)
        )
    }
}
```

### 5.2 Struct Variants for Complex Enums

```rust
#[derive(Debug, Clone)]
pub enum Trigger {
    Cron {
        expression: String,  // Cron expression
        timezone: String,
    },
    Event {
        event_type: String,
        matcher: EventMatcher,
    },
    Webhook {
        path: String,
        secret: Option<String>,
    },
    Manual,
}

impl Trigger {
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        match self {
            Trigger::Cron { expression, .. } => {
                let cron = CronSchedule::parse(expression).ok()?;
                cron.is_due(now)
            }
            _ => false,
        }
    }
}
```

### 5.3 PhantomData for Type Parameters

```rust
use std::marker::PhantomData;

// Type-safe state machine
pub struct Job<S: JobStateType> {
    id: Uuid,
    state: S,
    _marker: PhantomData<S>,
}

pub trait JobStateType: Send + Sync {}

pub struct Pending;
pub struct InProgress;
pub struct Completed;

impl JobStateType for Pending {}
impl JobStateType for InProgress {}
impl JobStateType for Completed {}

// Can only transition with valid state
impl Job<Pending> {
    pub fn start(self) -> Job<InProgress> {
        Job {
            id: self.id,
            state: InProgress,
            _marker: PhantomData,
        }
    }
}
```

### 5.4 Newtype for Domain Types

```rust
// Prevent mixing up string types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelId(String);

// Usage in function signatures prevents errors
pub async fn get_user_sessions(
    user_id: UserId,
) -> Result<Vec<Session>> {
    // ...
}

// Can't accidentally pass ChannelId where UserId expected
```

---

## 6. Async Patterns

### 6.1 Async Trait Pattern

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError>;
}

// Implementation
#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Echoes back the message" }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: &JobContext,
    ) -> Result<ToolOutput, ToolError> {
        let message = params["message"].as_str()
            .ok_or(ToolError::InvalidParameters {
                reason: "missing 'message'".into()
            })?;
        Ok(ToolOutput::text(message, Duration::from_millis(1)))
    }
}
```

### 6.2 Timeout Pattern

```rust
use tokio::time::{timeout, Duration};

async fn execute_with_timeout<F, T>(
    future: F,
    timeout_duration: Duration,
) -> Result<T, ToolError>
where
    F: Future<Output = Result<T, ToolError>>,
{
    match timeout(timeout_duration, future).await {
        Ok(result) => result,
        Err(_) => Err(ToolError::Timeout(timeout_duration)),
    }
}

// Usage
let result = execute_with_timeout(
    tool.execute(params, ctx),
    tool.execution_timeout(),
).await?;
```

### 6.3 Retry with Backoff

```rust
use tokio::time::{sleep, Duration};

pub async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    max_retries: u32,
    base_delay: Duration,
) -> Result<T, E>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display,
{
    let mut delay = base_delay;
    let mut attempts = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempts >= max_retries => {
                return Err(e);
            }
            Err(_) => {
                attempts += 1;
                sleep(delay).await;
                delay *= 2;  // Exponential backoff
            }
        }
    }
}
```

---

## 7. Memory Management

### 7.1 Owned vs Borrowed Data

```rust
// Prefer borrowing for read-only operations
pub fn parse_message(message: &str) -> Result<MessageIntent> {
    // No allocation
    if message.starts_with('/') {
        Ok(MessageIntent::Command)
    } else {
        Ok(MessageIntent::Query)
    }
}

// Use Cow for data that might be modified
use std::borrow::Cow;

pub fn sanitize_input(input: &str) -> Cow<str> {
    if needs_sanitization(input) {
        Cow::Owned(escape_html(input))
    } else {
        Cow::Borrowed(input)
    }
}
```

### 7.2 Arena Allocation for Parsing

```rust
// For parsing many small strings from input
use typed_arena::Arena;

pub fn parse_tool_calls(
    input: &str,
) -> Vec<ToolCall> {
    let arena = Arena::new();
    let mut calls = Vec::new();

    // Allocate parsed strings in arena
    for match in TOOL_CALL_PATTERN.find_iter(input) {
        let name = arena.alloc(match.as_str());
        calls.push(ToolCall { name, ... });
    }

    calls
}
```

### 7.3 Limiting Memory Usage

```rust
// WASM memory limits
pub struct ResourceLimits {
    max_memory_pages: u32,
    max_fuel: u64,
}

impl wasmtime::ResourceLimiter for ResourceLimits {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        Ok(desired <= self.max_memory_pages as usize * 65536)
    }
}
```

---

## 8. Serialization Patterns

### 8.1 Serde for Database Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDocument {
    pub id: Uuid,
    pub user_id: String,
    pub agent_id: Option<Uuid>,
    pub path: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Custom serialization for compact storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAction {
    pub id: Uuid,
    pub job_id: Uuid,

    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,

    pub action_type: ActionType,
    pub parameters: serde_json::Value,
    pub result: Option<serde_json::Value>,
}
```

### 8.2 Custom Serde for Newtypes

```rust
#[derive(Debug, Clone)]
pub struct UserId(String);

impl Serialize for UserId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for UserId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(UserId(s))
    }
}
```

---

## 9. Testing Patterns

### 9.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/foo/bar/"), "foo/bar");
        assert_eq!(normalize_path("foo//bar"), "foo/bar");
    }

    #[test]
    fn test_job_state_transitions() {
        let pending = JobState::Pending;
        assert!(pending.can_transition_to(&JobState::InProgress));
        assert!(!pending.can_transition_to(&JobState::Completed));
    }
}
```

### 9.2 Async Tests

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_database_create_job() {
        let db = create_test_database().await;
        let job = create_test_job();

        let id = db.create_job(&job).await.unwrap();

        let retrieved = db.get_job(id).await.unwrap();
        assert_eq!(retrieved.state, JobState::Pending);
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let tool = EchoTool;
        let ctx = JobContext::default();

        let result = tool
            .execute(serde_json::json!({"message": "hello"}), &ctx)
            .await
            .unwrap();

        assert_eq!(result.result, serde_json::json!("hello"));
    }
}
```

### 9.3 Test Fixtures

```rust
// Shared test utilities
mod test_utils {
    use crate::db::{Database, libsql_backend::LibSqlBackend};
    use tempfile::TempDir;

    pub async fn create_test_database() -> Arc<dyn Database> {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let backend = LibSqlBackend::new(db_path).await.unwrap();
        Arc::new(backend)
    }

    pub fn create_test_job() -> AgentJob {
        AgentJob {
            id: Uuid::new_v4(),
            user_id: "test_user".into(),
            state: JobState::Pending,
            // ...
        }
    }
}
```

---

## 10. Feature Flag Patterns

### 10.1 Conditional Compilation

```rust
// Database backends
#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;

#[cfg(feature = "libsql")]
pub use libsql_backend::LibSqlBackend;

// Database trait implementation
#[cfg(feature = "postgres")]
#[async_trait]
impl Database for PostgresBackend {
    // ...
}

#[cfg(feature = "libsql")]
#[async_trait]
impl Database for LibSqlBackend {
    // ...
}
```

### 10.2 Platform-Specific Code

```rust
// Secrets store
#[cfg(target_os = "macos")]
mod keychain;

#[cfg(target_os = "linux")]
mod secret_service;

#[cfg(target_os = "windows")]
mod credential_manager;

// Usage
#[cfg(target_os = "macos")]
use keychain::KeychainSecrets as SecretsStore;

#[cfg(target_os = "linux")]
use secret_service::SecretServiceSecrets as SecretsStore;
```

### 10.3 Feature-Gated Dependencies

```toml
[dependencies]
# PostgreSQL (default)
deadpool-postgres = { version = "0.14", optional = true }
tokio-postgres = { version = "0.7", optional = true }
refinery = { version = "0.8", features = ["tokio-postgres"], optional = true }

# libSQL
libsql = { version = "0.6", optional = true }

# macOS only
[target.'cfg(target_os = "macos")'.dependencies]
security-framework = "3"

# Linux only
[target.'cfg(target_os = "linux")'.dependencies]
secret-service = { version = "4", features = ["rt-tokio-crypto-rust"] }
```

---

## 11. Performance Optimizations

### 11.1 Connection Pooling

```rust
use deadpool_postgres::{Config, Pool, Runtime};

pub async fn create_pool(config: &DatabaseConfig) -> Pool {
    let mut cfg = Config::new();
    cfg.host = Some(config.host.clone());
    cfg.port = Some(config.port);
    cfg.user = Some(config.user.clone());
    cfg.password = Some(config.password.clone());
    cfg.dbname = Some(config.database.clone());

    cfg.pool = Some(deadpool_postgres::PoolConfig {
        max_size: config.pool_size,
        min_size: Some(config.min_idle),
    });

    cfg.create_pool(Runtime::Tokio1).unwrap()
}
```

### 11.2 Caching with Moka

```rust
use moka::future::Cache;

pub struct ToolRegistry {
    schemas: Cache<String, serde_json::Value>,
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            schemas: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(3600))
                .build(),
            tools: HashMap::new(),
        }
    }

    pub async fn get_schema(&self, name: &str) -> Option<serde_json::Value> {
        self.schema
            .get(name)
            .await
            .or_else(|| {
                let tool = self.tools.get(name)?;
                let schema = tool.parameters_schema();
                Some(self.schema.insert(name.to_string(), schema))
            })
    }
}
```

### 11.3 Zero-Copy Parsing

```rust
use bytes::Bytes;
use http_body_util::BodyExt;

// Avoid copying request bodies
async fn read_body<B>(body: B) -> Result<Bytes>
where
    B: http_body::Body,
{
    body.collect()
        .await
        .map(|collected| collected.to_bytes())
        .map_err(|e| Error::Http(e.to_string()))
}
```

---

## 12. Safety & Security

### 12.1 Secret Handling with secrecy crate

```rust
use secrecy::{Secret, ExposeSecret};

pub struct ApiCredentials {
    pub api_key: Secret<String>,
}

impl ApiCredentials {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Secret::new(api_key),
        }
    }

    pub fn api_key(&self) -> &str {
        self.api_key.expose_secret()
    }
}

// Debug output hides secret
impl Debug for ApiCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiCredentials")
            .field("api_key", &"**REDACTED**")
            .finish()
    }
}
```

### 12.2 Constant-Time Comparison

```rust
use subtle::ConstantTimeEq;

pub fn verify_token(provided: &[u8], expected: &[u8]) -> bool {
    provided.ct_eq(expected).into()
}

// Usage in auth
pub fn authenticate(token: &str, stored_hash: &[u8]) -> bool {
    let provided_hash = blake3::hash(token.as_bytes());
    verify_token(provided_hash.as_bytes(), stored_hash)
}
```

### 12.3 Safe Unwrap Alternatives

```rust
// Instead of .unwrap():
let value = result.map_err(|e| Error::Config(ConfigError::ParseError(
    format!("Failed to parse config: {}", e)
)))?;

// Or with context:
use anyhow::Context;
let value = result.context("Failed to parse configuration")?;
```

---

## 13. Code Organization

### 13.1 Module Structure

```
src/
├── lib.rs              # Library root, re-exports
├── main.rs             # Entry point
├── config.rs           # Configuration types
├── error.rs            # Error types
│
├── agent/              # Core agent logic
│   ├── mod.rs          # Module root, re-exports
│   ├── agent_loop.rs   # Main Agent struct
│   ├── worker.rs       # Job execution
│   └── ...
│
├── channels/           # Input channels
│   ├── mod.rs
│   ├── channel.rs      # Channel trait
│   ├── manager.rs      # ChannelManager
│   └── ...
│
└── tools/              # Tool system
    ├── mod.rs
    ├── tool.rs         # Tool trait
    ├── registry.rs     # ToolRegistry
    └── ...
```

### 13.2 Re-export Pattern

```rust
// src/agent/mod.rs
mod agent_loop;
mod worker;
mod scheduler;

pub use agent_loop::{Agent, AgentDeps};
pub use worker::{Worker, WorkerDeps};
pub use scheduler::Scheduler;

// Internal items not re-exported remain private to crate
```

### 13.3 Prelude Pattern

```rust
// src/lib.rs
pub mod prelude {
    pub use crate::channels::{Channel, IncomingMessage, MessageStream};
    pub use crate::config::Config;
    pub use crate::context::{JobContext, JobState};
    pub use crate::error::{Error, Result};
    pub use crate::llm::LlmProvider;
    pub use crate::tools::{Tool, ToolOutput, ToolRegistry};
    pub use crate::workspace::{MemoryDocument, Workspace};
}

// Usage
use ironclaw::prelude::*;
```

---

## 14. Dependencies Analysis

### 14.1 Core Dependencies

| Crate | Purpose | Why This Choice |
|-------|---------|-----------------|
| `tokio` | Async runtime | Industry standard, full-featured |
| `axum` | Web framework | Type-safe, ergonomic, tokio-native |
| `thiserror` | Error types | Compile-time checked, no runtime cost |
| `serde` | Serialization | Industry standard, derive macros |
| `async-trait` | Async traits | Required for trait objects with async |
| `wasmtime` | WASM runtime | Best Rust WASM runtime, component model |
| `deadpool-postgres` | Connection pool | Simple, async, production-ready |
| `rig-core` | LLM abstraction | NEAR AI compatible |

### 14.2 Version Pinning Strategy

```toml
[dependencies]
# Use caret requirements for flexibility
tokio = "1"           # Any 1.x
serde = "1"           # Any 1.x
axum = "0.8"          # Any 0.8.x

# Pin minor versions for unstable crates
wasmtime = "28"       # Major versions may break
```

---

## Related Documents

- [`exploration.md`](./exploration.md) - Main exploration overview
- [`architecture-deep-dive.md`](./architecture-deep-dive.md) - Architecture analysis
- [`production-grade.md`](./production-grade.md) - Production deployment
