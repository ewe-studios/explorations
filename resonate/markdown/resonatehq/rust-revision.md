# Reproducing ResonateHQ in Rust

## Overview

This guide provides a comprehensive approach to implementing ResonateHQ's distributed task orchestration functionality in Rust. The focus is on production-level implementations with attention to performance, safety, and ergonomics.

## Architecture Overview

ResonateHQ consists of these core components:

1. **Durable Promise Store** - Persistent promise storage with idempotency
2. **Task Scheduler** - Task claiming, execution, and completion
3. **Callback System** - HTTP/poll-based callbacks
4. **Scheduler** - CRON-based promise creation
5. **Lock Manager** - Distributed locking for exclusive execution
6. **SDK** - Client library for defining durable functions

## Recommended Crate Structure

```
resonate-rs/
├── Cargo.toml
├── resonate/           # Core server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── app/
│       │   ├── mod.rs
│       │   ├── coroutines/
│       │   │   ├── create_promise.rs
│       │   │   ├── complete_promise.rs
│       │   │   ├── claim_task.rs
│       │   │   └── ...
│       │   └── services/
│       ├── kernel/
│       │   ├── mod.rs
│       │   ├── aio.rs
│       │   ├── api.rs
│       │   └── system.rs
│       ├── pkg/
│       │   ├── mod.rs
│       │   ├── promise/
│       │   ├── task/
│       │   ├── callback/
│       │   └── schedule/
│       └── internal/
│           ├── mod.rs
│           ├── receiver/
│           └── util/
├── resonate-sdk/       # Rust SDK
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── context.rs
│       ├── promise.rs
│       ├── retry.rs
│       └── store/
├── resonate-store/     # Storage traits and implementations
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── trait.rs
│       ├── memory.rs
│       ├── sqlite.rs
│       └── postgres.rs
└── resonate-cli/       # CLI tools
    ├── Cargo.toml
    └── src/
        └── main.rs
```

## Core Data Models

### Promise

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Promise {
    pub id: String,
    pub state: PromiseState,
    pub param: Value,
    pub value: Value,
    pub timeout: i64,
    pub idempotency_key_for_create: Option<String>,
    pub idempotency_key_for_complete: Option<String>,
    pub tags: HashMap<String, String>,
    pub created_on: Option<i64>,
    pub completed_on: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PromiseState {
    Pending = 1,
    Resolved = 2,
    Rejected = 4,
    Canceled = 8,
    Timedout = 16,
}

impl PromiseState {
    pub fn in_mask(&self, mask: Self) -> bool {
        (*self as u8 & mask as u8) != 0
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Value {
    pub headers: HashMap<String, String>,
    pub data: Option<Vec<u8>>,
}
```

### Task

```rust
use crate::{promise::PromiseId, receiver::Receiver};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub process_id: Option<String>,
    pub root_promise_id: PromiseId,
    pub state: TaskState,
    pub recv: Option<Receiver>,
    pub mesg: Message,
    pub timeout: i64,
    pub counter: u32,
    pub attempt: u32,
    pub ttl: i64,
    pub expires_at: i64,
    pub created_on: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Init,
    Claimed,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Invoke,
    Claim,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub r#type: MessageType,
    pub root: String,
    pub leaf: String,
}
```

### Schedule

```rust
use cron::Schedule as CronSchedule;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub description: Option<String>,
    pub cron: String,
    pub tags: HashMap<String, String>,
    pub promise_id: String,
    pub promise_timeout: i64,
    pub promise_param: Value,
    pub promise_tags: HashMap<String, String>,
    pub last_run_time: Option<i64>,
    pub next_run_time: i64,
    pub idempotency_key: Option<String>,
    pub created_on: Option<i64>,
}

impl Schedule {
    pub fn next_run(&self, after: i64) -> Option<i64> {
        let cron: CronSchedule = self.cron.parse().ok()?;
        let after_datetime = DateTime::from_timestamp_millis(after)?;
        cron.after(&after_datetime)
            .next()
            .map(|dt| dt.timestamp_millis())
    }
}
```

## Storage Layer

### Storage Traits

```rust
use async_trait::async_trait;
use std::error::Error;

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug)]
pub enum StoreError {
    NotFound,
    AlreadyExists,
    Conflict,
    Internal(Box<dyn Error + Send + Sync>),
}

#[async_trait]
pub trait PromiseStore: Send + Sync {
    async fn create(&self, promise: Promise, idempotency_key: Option<String>) -> Result<Promise>;
    async fn resolve(&self, id: &str, value: Value, idempotency_key: Option<String>) -> Result<Promise>;
    async fn reject(&self, id: &str, error: Value, idempotency_key: Option<String>) -> Result<Promise>;
    async fn cancel(&self, id: &str, error: Value, idempotency_key: Option<String>) -> Result<Promise>;
    async fn get(&self, id: &str) -> Result<Promise>;
    async fn search(
        &self,
        id_pattern: &str,
        state: Option<PromiseState>,
        tags: Option<HashMap<String, String>>,
        limit: usize,
    ) -> Result<Vec<Promise>>;
}

#[async_trait]
pub trait TaskStore: Send + Sync {
    async fn create(&self, task: Task) -> Result<Task>;
    async fn claim(&self, id: &str, process_id: &str) -> Result<Task>;
    async fn complete(&self, id: &str) -> Result<Task>;
    async fn get(&self, id: &str) -> Result<Task>;
    async fn search(&self, state: Option<TaskState>, limit: usize) -> Result<Vec<Task>>;
    async fn heartbeat(&self, id: &str, process_id: &str) -> Result<()>;
    async fn timeout(&self, expires_at: i64) -> Result<Vec<Task>>;
}

#[async_trait]
pub trait ScheduleStore: Send + Sync {
    async fn create(&self, schedule: Schedule) -> Result<Schedule>;
    async fn get(&self, id: &str) -> Result<Schedule>;
    async fn update(&self, schedule: Schedule) -> Result<Schedule>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn search(&self, tags: Option<HashMap<String, String>>) -> Result<Vec<Schedule>>;
    async fn due(&self, now: i64) -> Result<Vec<Schedule>>;
}

#[async_trait]
pub trait LockStore: Send + Sync {
    async fn try_acquire(&self, id: &str, eid: &str, expiry_ms: i64) -> Result<bool>;
    async fn release(&self, id: &str, eid: &str) -> Result<bool>;
    async fn heartbeat(&self, id: &str, eid: &str, expiry_ms: i64) -> Result<bool>;
}
```

### SQLite Implementation

```rust
use sqlx::{Sqlite, SqlitePool, FromRow};

pub struct SqlitePromiseStore {
    pool: SqlitePool,
}

impl SqlitePromiseStore {
    pub async fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn migrate(pool: &SqlitePool) -> sqlx::Result<()> {
        sqlx::migrate!("./migrations").run(pool).await
    }
}

#[async_trait]
impl PromiseStore for SqlitePromiseStore {
    async fn create(&self, promise: Promise, idempotency_key: Option<String>) -> Result<Promise> {
        let mut tx = self.pool.begin().await.map_err(|e| StoreError::Internal(e.into()))?;

        // Check for existing promise
        let existing: Option<PromiseRow> = sqlx::query_as(
            "SELECT * FROM promises WHERE id = ?"
        )
        .bind(&promise.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| StoreError::Internal(e.into()))?;

        if let Some(row) = existing {
            // Handle idempotency
            if row.idempotency_key_for_create == idempotency_key {
                return Ok(row.into_promise());
            }
            return Err(StoreError::AlreadyExists);
        }

        // Create new promise
        let row = PromiseRow::from_promise(promise, idempotency_key);
        sqlx::query(
            "INSERT INTO promises (id, state, param, timeout, idempotency_key_for_create, tags, created_on)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&row.id)
        .bind(row.state as i32)
        .bind(&row.param)
        .bind(row.timeout)
        .bind(&row.idempotency_key_for_create)
        .bind(&row.tags)
        .bind(row.created_on)
        .execute(&mut *tx)
        .await
        .map_err(|e| StoreError::Internal(e.into()))?;

        tx.commit().await.map_err(|e| StoreError::Internal(e.into()))?;

        Ok(row.into_promise())
    }

    // ... other methods
}
```

### PostgreSQL Implementation

```rust
use sqlx::{PgPool, FromRow};

pub struct PostgresPromiseStore {
    pool: PgPool,
}

impl PostgresPromiseStore {
    pub async fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PromiseStore for PostgresPromiseStore {
    async fn create(&self, promise: Promise, idempotency_key: Option<String>) -> Result<Promise> {
        // Use PostgreSQL's ON CONFLICT for atomic upsert
        let row: PromiseRow = sqlx::query_as(
            r#"
            INSERT INTO promises
                (id, state, param, timeout, idempotency_key_for_create, tags, created_on)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE
                SET idempotency_key_for_create =
                    CASE WHEN promises.idempotency_key_for_create IS NULL
                         THEN $5
                         ELSE promises.idempotency_key_for_create
                    END
            RETURNING *
            "#
        )
        .bind(&promise.id)
        .bind(promise.state as i32)
        .bind(serde_json::to_value(&promise.param)?)
        .bind(promise.timeout)
        .bind(&idempotency_key)
        .bind(serde_json::to_value(&promise.tags)?)
        .bind(chrono::Utc::now().timestamp_millis())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(db_err) => {
                if db_err.constraint() == Some("promises_idempotency_key_for_create_key") {
                    StoreError::Conflict
                } else {
                    StoreError::Internal(e.into())
                }
            }
            _ => StoreError::Internal(e.into()),
        })?;

        Ok(row.into_promise())
    }
}
```

## Async Runtime & Coroutines

### Using Tokio

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

pub struct CoroutineHandle<T> {
    join_handle: tokio::task::JoinHandle<Result<T, CoroutineError>>,
}

pub struct Coroutine<T, U> {
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<U>,
}

impl<T, U> Coroutine<T, U>
where
    T: Send + 'static,
    U: Send + 'static,
{
    pub fn spawn<F, Fut>(f: F) -> Self
    where
        F: FnOnce(mpsc::Sender<T>, mpsc::Receiver<U>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let (tx, rx) = mpsc::channel(32);
        let (sx, sr) = mpsc::channel(32);

        let handle = tokio::spawn(f(tx, sr));

        Coroutine {
            sender: sx,
            receiver: rx,
        }
    }
}
```

### State Machine Approach

For the coroutine-like behavior in Go, use a state machine pattern:

```rust
use std::future::Future;
use std::pin::Pin;

pub enum CreatePromiseState {
    ReadPromise,
    CreatePromise,
    CreateTask,
    Complete,
}

pub struct CreatePromiseCoroutine {
    state: CreatePromiseState,
    request: CreatePromiseRequest,
    response: Option<CreatePromiseResponse>,
}

impl CreatePromiseCoroutine {
    pub async fn run(&mut self, ctx: &CoroutineContext) -> Result<CreatePromiseResponse> {
        loop {
            match &mut self.state {
                CreatePromiseState::ReadPromise => {
                    let promise = ctx.store.get(&self.request.id).await;
                    self.state = CreatePromiseState::CreatePromise;
                }
                CreatePromiseState::CreatePromise => {
                    // Create the promise
                    let promise = ctx.store.create(self.request.clone()).await?;
                    self.state = CreatePromiseState::Complete;
                }
                CreatePromiseState::Complete => {
                    return Ok(self.response.take().unwrap());
                }
            }
        }
    }
}
```

## SDK Design

### Function Registration

```rust
use std::future::Future;
use std::pin::Pin;

pub type DurableFunction = dyn Fn(Context, Vec<Value>) -> Pin<Box<dyn Future<Output = Result<Value>> + Send>> + Send + Sync;

pub struct Resonate {
    store: Arc<dyn PromiseStore>,
    registry: DashMap<String, Arc<DurableFunction>>,
    options: ResonateOptions,
}

impl Resonate {
    pub fn new(store: Arc<dyn PromiseStore>, options: ResonateOptions) -> Self {
        Self {
            store,
            registry: DashMap::new(),
            options,
        }
    }

    pub fn register<F, Fut>(&mut self, name: &str, f: F)
    where
        F: Fn(Context, Vec<Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value>> + Send + 'static,
    {
        let wrapped = Box::new(move |ctx, args| {
            Box::pin(f(ctx, args))
        });
        self.registry.insert(name.to_string(), Arc::new(wrapped));
    }

    pub async fn run(&self, name: &str, id: &str, args: Vec<Value>) -> Result<Value> {
        let func = self.registry.get(name)
            .ok_or_else(|| Error::FunctionNotFound(name.to_string()))?;

        let promise = self.store.create(Promise {
            id: id.to_string(),
            state: PromiseState::Pending,
            // ...
        }, None).await?;

        // Execute or recover
        self.execute_function(&func, id, args).await
    }
}
```

### Context Implementation

```rust
use std::sync::Arc;

pub struct Context {
    id: String,
    parent: Option<Arc<Context>>,
    resonate: Arc<Resonate>,
    children: DashMap<String, InvocationHandle>,
    resources: DashMap<String, Resource>,
    finalizers: Mutex<Vec<Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>>>,
}

impl Context {
    pub fn root(resonate: Arc<Resonate>, id: String) -> Self {
        Self {
            id,
            parent: None,
            resonate,
            children: DashMap::new(),
            resources: DashMap::new(),
            finalizers: Mutex::new(Vec::new()),
        }
    }

    pub fn child(&self, id: String) -> Self {
        Self {
            id,
            parent: Some(Arc::new(self.clone())),
            resonate: self.resonate.clone(),
            children: DashMap::new(),
            resources: DashMap::new(),
            finalizers: Mutex::new(Vec::new()),
        }
    }

    pub async fn run<F>(&self, name: &str, args: Vec<Value>) -> Result<Value>
    where
        F: Future<Output = Result<Value>> + Send + 'static,
    {
        // Similar to TypeScript invokeLocal
        let child_id = format!("{}.{}", self.id, self.children.len());
        let handle = self.invoke_local(name, child_id, args).await?;
        handle.result().await
    }

    pub async fn sleep(&self, ms: u64) -> Result<()> {
        let id = format!("{}.sleep.{}", self.id, self.children.len());
        let handle = self.invoke_remote(&id, ms).await?;
        handle.result().await
    }
}
```

### Retry System

```rust
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub enum RetryPolicy {
    Exponential {
        initial_delay: Duration,
        backoff_factor: f64,
        max_attempts: u32,
        max_delay: Duration,
    },
    Linear {
        delay: Duration,
        max_attempts: u32,
    },
    Never,
}

impl RetryPolicy {
    pub fn exponential(initial_ms: u64, factor: f64, max_attempts: u32, max_ms: u64) -> Self {
        Self::Exponential {
            initial_delay: Duration::from_millis(initial_ms),
            backoff_factor: factor,
            max_attempts,
            max_delay: Duration::from_millis(max_ms),
        }
    }

    pub fn iterator(&self) -> RetryIterator {
        RetryIterator::new(self, 0)
    }
}

pub struct RetryIterator {
    policy: RetryPolicy,
    attempt: u32,
}

impl Iterator for RetryIterator {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        match &self.policy {
            RetryPolicy::Exponential { initial_delay, backoff_factor, max_delay, .. } => {
                if self.attempt >= self.policy.max_attempts() {
                    return None;
                }

                let delay = if self.attempt == 0 {
                    Duration::ZERO
                } else {
                    let delay_ms = initial_delay.as_millis() as f64
                        * backoff_factor.powi((self.attempt - 1) as i32);
                    Duration::from_millis((delay_ms as u64).min(max_delay.as_millis() as u64))
                };

                self.attempt += 1;
                Some(delay)
            }
            // ... other policies
        }
    }
}

pub async fn run_with_retry<F, Fut, T>(
    mut f: F,
    on_retry: impl Fn() -> Fut2,
    policy: RetryPolicy,
    timeout: Duration,
) -> Result<T>
where
    F: FnMut() -> Fut + Send,
    Fut: Future<Output = Result<T>> + Send,
    Fut2: Future<Output = ()> + Send,
{
    let start = std::time::Instant::now();
    let mut last_error = None;

    for delay in policy.iterator() {
        if start.elapsed() >= timeout {
            return Err(Error::Timeout);
        }

        sleep(delay).await;

        if delay > Duration::ZERO {
            on_retry().await;
        }

        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap_or(Error::Unknown))
}
```

## HTTP API

### Using Axum

```rust
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

pub fn create_router(store: Arc<dyn PromiseStore>) -> Router {
    Router::new()
        .route("/promises", post(create_promise))
        .route("/promises/:id", get(get_promise))
        .route("/promises/:id/resolve", post(resolve_promise))
        .route("/promises/:id/reject", post(reject_promise))
        .route("/promises/:id/cancel", post(cancel_promise))
        .route("/tasks/claim", post(claim_task))
        .route("/tasks/complete", post(complete_task))
        .route("/tasks/heartbeat", post(heartbeat_tasks))
        .route("/schedules", post(create_schedule))
        .route("/schedules/:id", get(get_schedule))
        .with_state(store)
}

async fn create_promise(
    State(store): State<Arc<dyn PromiseStore>>,
    Json(req): Json<CreatePromiseRequest>,
) -> Result<Json<CreatePromiseResponse>, StatusCode> {
    let promise = Promise {
        id: req.id,
        state: PromiseState::Pending,
        param: req.param,
        value: Value::default(),
        timeout: req.timeout,
        idempotency_key_for_create: req.idempotency_key,
        idempotency_key_for_complete: None,
        tags: req.tags.unwrap_or_default(),
        created_on: Some(chrono::Utc::now().timestamp_millis()),
        completed_on: None,
    };

    match store.create(promise, req.idempotency_key).await {
        Ok(p) => Ok(Json(CreatePromiseResponse { promise: p })),
        Err(StoreError::AlreadyExists) => Err(StatusCode::CONFLICT),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

## WASM Considerations

ResonateHQ doesn't heavily utilize WASM, but for a Rust implementation targeting both web and non-web:

### Web Environment (WASM)

```rust
// For browser-based workers
#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;
    use web_sys::Window;

    #[wasm_bindgen]
    pub struct WasmWorker {
        // Worker state
    }

    #[wasm_bindgen]
    impl WasmWorker {
        pub async fn new() -> Result<Self, JsValue> {
            // Initialize in browser context
            Ok(Self { /* ... */ })
        }

        pub async fn claim_task(&self) -> Result<Task, JsValue> {
            // Use fetch API for HTTP requests
            let resp = web_sys::window()
                .unwrap()
                .fetch_with_str("/tasks/claim")
                .await?;
            // Parse response
        }
    }
}
```

### Non-Web Environment

```rust
// For native server workers
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use tokio::sync::mpsc;

    pub struct NativeWorker {
        sender: mpsc::Sender<Task>,
    }

    impl NativeWorker {
        pub async fn run(&self) -> Result<()> {
            loop {
                let task = self.claim_task().await?;
                self.execute(task).await?;
            }
        }
    }
}
```

## Performance Optimizations

### Connection Pooling

```rust
use deadpool::managed::Manager;
use sqlx::SqlitePool;

pub struct ConnectionPool {
    pool: deadpool::managed::Pool<SqliteManager>,
}

pub struct SqliteManager {
    pool: SqlitePool,
}

#[async_trait]
impl Manager for SqliteManager {
    type Type = SqlitePool;
    type Error = sqlx::Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        Ok(self.pool.clone())
    }
}
```

### Caching

```rust
use moka::future::Cache;

pub struct CachedPromiseStore {
    inner: Arc<dyn PromiseStore>,
    cache: Cache<String, Promise>,
}

#[async_trait]
impl PromiseStore for CachedPromiseStore {
    async fn get(&self, id: &str) -> Result<Promise> {
        // Check cache first
        if let Some(promise) = self.cache.get(id).await {
            return Ok(promise);
        }

        // Fall back to underlying store
        let promise = self.inner.get(id).await?;
        self.cache.insert(id.to_string(), promise.clone()).await;
        Ok(promise)
    }

    async fn resolve(&self, id: &str, value: Value, ikey: Option<String>) -> Result<Promise> {
        let promise = self.inner.resolve(id, value, ikey).await?;
        self.cache.insert(id.to_string(), promise.clone()).await;
        Ok(promise)
    }
}
```

### Batching

```rust
use tokio::sync::mpsc;

pub struct BatchedPromiseStore {
    sender: mpsc::Sender<BatchCommand>,
}

enum BatchCommand {
    Create(Promise, oneshot::Sender<Result<Promise>>),
    Resolve(String, Value, oneshot::Sender<Result<Promise>>),
    // ...
}

impl BatchedPromiseStore {
    pub fn spawn(inner: Arc<dyn PromiseStore>, batch_size: usize, batch_timeout: Duration) -> Self {
        let (tx, mut rx) = mpsc::channel(1024);

        tokio::spawn(async move {
            let mut batch = Vec::new();
            let mut interval = tokio::time::interval(batch_timeout);

            loop {
                tokio::select! {
                    cmd = rx.recv() => {
                        match cmd {
                            Some(BatchCommand::Create(promise, tx)) => {
                                batch.push(Command::Create(promise, tx));
                            }
                            None => break,
                        }
                    }
                    _ = interval.tick() => {
                        if batch.len() >= batch_size {
                            // Execute batch
                            Self::execute_batch(&inner, batch).await;
                            batch = Vec::new();
                        }
                    }
                }
            }
        });

        Self { sender: tx }
    }
}
```

## Idempotency Implementation

```rust
use sha2::{Sha256, Digest};

pub fn generate_idempotency_key(function_name: &str, args: &[Value]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(function_name.as_bytes());
    for arg in args {
        hasher.update(serde_json::to_vec(arg).unwrap());
    }
    let hash = hasher.finalize();
    hex::encode(hash)
}

// For deterministic child invocation
pub fn child_id(parent_id: &str, child_index: usize, function_name: &str) -> String {
    format!("{}.{}.{}", parent_id, child_index, function_name)
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryPromiseStore;

    #[tokio::test]
    async fn test_create_promise_idempotency() {
        let store = Arc::new(MemoryPromiseStore::new());
        let idempotency_key = Some("key-123".to_string());

        let promise1 = Promise {
            id: "test".to_string(),
            state: PromiseState::Pending,
            // ...
        };

        let result1 = store.create(promise1.clone(), idempotency_key.clone()).await;
        let result2 = store.create(promise1.clone(), idempotency_key.clone()).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap().id, result2.unwrap().id);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_durable_function_execution() {
    let store = Arc::new(SqlitePromiseStore::new(pool).await);
    let resonate = Resonate::new(store, ResonateOptions::default());

    resonate.register("add", |ctx, args| async move {
        let a = args[0].as_i64().unwrap();
        let b = args[1].as_i64().unwrap();
        Ok(Value::from(a + b))
    });

    let result = resonate.run("add", "test-id", vec![Value::from(1), Value::from(2)]).await;
    assert_eq!(result.unwrap().as_i64(), Some(3));
}
```

## Dependencies (Cargo.toml)

```toml
[package]
name = "resonate"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "postgres"] }
deadpool = "0.10"

# Time
chrono = { version = "0.4", features = ["serde"] }
cron = "0.12"

# Concurrency
dashmap = "5"
parking_lot = "0.12"

# Utilities
uuid = { version = "1", features = ["v4"] }
sha2 = "0.10"
hex = "0.4"
thiserror = "1"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# WASM (optional)
wasm-bindgen = { version = "0.2", optional = true }
wasm-bindgen-futures = { version = "0.4", optional = true }
web-sys = { version = "0.3", optional = true }

[features]
default = []
wasm = ["wasm-bindgen", "wasm-bindgen-futures", "web-sys"]
```

## Key Implementation Notes

1. **Use Newtypes for IDs**: Type-safe identifiers prevent mixing up promise IDs, task IDs, etc.

2. **Transactional Operations**: Ensure promise creation and task creation happen atomically.

3. **Proper Error Propagation**: Use `thiserror` for structured error types that map to HTTP status codes.

4. **Timeout Handling**: Implement a separate background task for checking and timing out expired promises/tasks.

5. **Heartbeat Mechanism**: Workers should heartbeat to maintain task ownership.

6. **Graceful Shutdown**: Ensure in-flight operations complete before shutdown.

7. **Metrics**: Use `metrics` crate for observability (prometheus, etc.).
