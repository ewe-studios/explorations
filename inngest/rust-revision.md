# Reproducing Inngest in Rust - Production Guide

## Overview

This guide provides a comprehensive roadmap for reproducing Inngest's functionality in Rust at production level. Inngest provides serverless durable function execution with event-driven triggers, step-based workflows, automatic retries, and flow control.

**Target Architecture**: A Rust-native implementation of:
1. Event ingestion and streaming
2. Durable function orchestration
3. Step execution with state persistence
4. Multi-tier queue with flow control
5. Scheduling (cron + delayed execution)
6. SDK protocol for function registration

---

## System Architecture

### High-Level Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust Inngest Server                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Event     │  │   Runner    │  │      Executor       │  │
│  │    API      │──▶│  (Worker)   │──▶│   (Step Runner)   │  │
│  │ (Axum HTTP) │  │  (NATS)     │  │    (Hyper HTTP)     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│         │                │                    │              │
│         ▼                ▼                    ▼              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                  State Store (Redis)                   │  │
│  │  - Function runs  - Step outputs  - Pauses            │  │
│  └───────────────────────────────────────────────────────┘  │
│         │                │                    │              │
│         ▼                ▼                    ▼              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   Database (PostgreSQL)                │  │
│  │  - Function defs  - Event history  - Audit logs       │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Data Structures

### 1. Event Model

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Event as received from SDKs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub data: serde_json::Value,
    #[serde(rename = "user", skip_serializing_if = "Option::is_none")]
    pub user: Option<serde_json::Value>,
    #[serde(rename = "ts", default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
    #[serde(rename = "v", skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Tracked event with internal metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedEvent {
    #[serde(flatten)]
    pub event: Event,
    pub internal_id: Ulid,
    pub account_id: Uuid,
    pub workspace_id: Uuid,
    pub app_id: Uuid,
    pub received_at: DateTime<Utc>,
}

impl Event {
    pub fn validate(&self) -> Result<(), EventError> {
        if self.name.is_empty() {
            return Err(EventError::MissingName);
        }
        if self.data.is_null() {
            return Err(EventError::MissingData);
        }
        Ok(())
    }

    pub fn now_timestamp() -> i64 {
        Utc::now().timestamp_millis()
    }
}
```

### 2. Function Definition

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: Uuid,
    pub name: String,
    pub app_id: Uuid,
    pub account_id: Uuid,
    pub workspace_id: Uuid,
    pub triggers: Vec<Trigger>,
    pub steps: HashMap<String, Step>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<ConcurrencyConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debounce: Option<DebounceConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_events: Option<BatchConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<TimeoutConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Trigger {
    #[serde(rename = "event")]
    Event {
        event: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        expression: Option<String>,
    },
    #[serde(rename = "cron")]
    Cron { cron: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub name: String,
    pub runtime: StepRuntime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<StepRetries>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRuntime {
    #[serde(rename = "type")]
    pub runtime_type: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRetries {
    pub attempts: u32,
}
```

### 3. Function Run State

```rust
use ulid::Ulid;

/// Unique identifier for a function run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RunIdentifier {
    pub run_id: Ulid,
    pub workflow_id: Uuid,
    pub workflow_version: u32,
    pub event_id: Ulid,
    pub event_ids: Vec<Ulid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<Ulid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub account_id: Uuid,
    pub workspace_id: Uuid,
    pub app_id: Uuid,
}

/// State for a function run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRunState {
    pub identifier: RunIdentifier,
    pub status: RunStatus,
    pub triggering_event: TrackedEvent,
    pub step_outputs: HashMap<String, serde_json::Value>,
    pub step_errors: HashMap<String, StepError>,
    pub pending_pauses: Vec<Pause>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<FunctionError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pause {
    pub id: Ulid,
    pub pause_type: PauseType,
    pub event_key: Option<String>,
    pub timeout_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leased_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PauseType {
    #[serde(rename = "wait_for_event")]
    WaitForEvent { step_id: String },
    #[serde(rename = "invoke")]
    Invoke { step_id: String, invoked_run_id: Ulid },
}
```

### 4. Queue Item

```rust
use chrono::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub job_id: String,
    pub group_id: String,
    pub workspace_id: Uuid,
    pub kind: QueueKind,
    pub identifier: RunIdentifier,
    pub attempt: u32,
    pub max_attempts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<QueuePayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttle: Option<ThrottleConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_factor: Option<i64>,
    pub run_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueKind {
    Start,
    Edge { edge_id: String },
    Sleep { wake_at: DateTime<Utc> },
    Pause { pause_id: Ulid },
    Debounce { debounce_id: Ulid },
    ScheduleBatch { batch_id: Ulid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuePayload {
    pub edge: Option<Edge>,
    pub debounce: Option<DebouncePayload>,
    pub batch: Option<BatchPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub step_id: String,
    pub parent_step_id: Option<String>,
}
```

---

## Event Processing Pipeline

### 1. Event Ingestion API

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;

pub struct AppState {
    pub event_sender: tokio::sync::mpsc::Sender<TrackedEvent>,
    // ... other state
}

/// POST /e/{key}
pub async fn ingest_event(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<IngestResponse>, ApiError> {
    // 1. Validate event key
    let app = validate_event_key(&key).await?;

    // 2. Parse event(s)
    let events = parse_events(body)?;

    // 3. Create tracked events
    let tracked: Vec<TrackedEvent> = events
        .into_iter()
        .map(|e| TrackedEvent::new(e, &app))
        .collect();

    // 4. Send to event stream
    for event in tracked {
        state.event_sender.send(event).await
            .map_err(|_| ApiError::InternalServerError)?;
    }

    Ok(Json(IngestResponse {
        ids: tracked.iter().map(|e| e.internal_id.to_string()).collect(),
    }))
}

fn parse_events(body: serde_json::Value) -> Result<Vec<Event>, ApiError> {
    match body {
        serde_json::Value::Object(_) => {
            let event = serde_json::from_value::<Event>(body)?;
            Ok(vec![event])
        }
        serde_json::Value::Array(arr) => {
            arr.into_iter()
                .map(|v| serde_json::from_value::<Event>(v))
                .collect()
        }
        _ => Err(ApiError::InvalidEvent),
    }
}
```

### 2. Event Stream Processor

```rust
use tokio::sync::broadcast;

pub struct EventStream {
    sender: broadcast::Sender<TrackedEvent>,
}

impl EventStream {
    pub fn new(buffer_size: usize) -> Self {
        let (sender, _) = broadcast::channel(buffer_size);
        Self { sender }
    }

    pub fn publish(&self, event: TrackedEvent) -> Result<(), EventError> {
        self.sender.send(event).map_err(|_| EventError::StreamFull)?;
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TrackedEvent> {
        self.sender.subscribe()
    }
}

// Alternative: Use NATS for production
use async_nats::{Client, Subject};

pub struct NatsEventStream {
    client: Client,
    subject: Subject,
}

impl NatsEventStream {
    pub async fn new(client: Client, stream_name: &str) -> Result<Self, NatsError> {
        let subject = Subject::from(format!("{}.events", stream_name));
        Ok(Self { client, subject })
    }

    pub async fn publish(&self, event: &TrackedEvent) -> Result<(), NatsError> {
        let data = serde_json::to_vec(event)?;
        self.client.publish(self.subject.clone(), data.into()).await?;
        Ok(())
    }

    pub async fn subscribe(&self) -> Result<async_nats::Subscriber, NatsError> {
        Ok(self.client.subscribe(self.subject.clone()).await?)
    }
}
```

### 3. Runner (Event Consumer)

```rust
use tokio::task::JoinHandle;

pub struct Runner {
    state_store: Arc<dyn StateStore>,
    queue: Arc<dyn Queue>,
    event_stream: EventStream,
}

impl Runner {
    pub fn new(
        state_store: Arc<dyn StateStore>,
        queue: Arc<dyn Queue>,
        event_stream: EventStream,
    ) -> Self {
        Self {
            state_store,
            queue,
            event_stream,
        }
    }

    pub fn start(&self) -> JoinHandle<Result<(), RunnerError>> {
        let mut receiver = self.event_stream.subscribe();
        let state_store = Arc::clone(&self.state_store);
        let queue = Arc::clone(&self.queue);

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                match Self::process_event(&state_store, &queue, event).await {
                    Ok(_) => {},
                    Err(e) => tracing::error!("Error processing event: {}", e),
                }
            }
            Ok(())
        })
    }

    async fn process_event(
        state_store: &Arc<dyn StateStore>,
        queue: &Arc<dyn Queue>,
        event: TrackedEvent,
    ) -> Result<(), RunnerError> {
        // 1. Find matching functions
        let functions = state_store
            .find_functions_by_trigger(&event.event.name)
            .await?;

        for function in functions {
            // 2. Evaluate expression filter (CEL)
            if let Some(expr) = function.trigger_expression() {
                if !evaluate_expression(expr, &event)? {
                    continue;
                }
            }

            // 3. Check debounce
            if let Some(debounce_config) = &function.debounce {
                Self::handle_debounce(state_store, queue, &event, &function, debounce_config)
                    .await?;
                continue;
            }

            // 4. Create function run state
            let identifier = RunIdentifier::new(&function, &event);
            let state = FunctionRunState::new(identifier.clone(), &event);
            state_store.create_run(state).await?;

            // 5. Enqueue function run
            queue.enqueue(QueueItem::new_start(identifier)).await?;
        }

        Ok(())
    }
}
```

---

## Step Execution Engine

### 1. Executor

```rust
use reqwest::Client;

pub struct Executor {
    http_client: Client,
    state_store: Arc<dyn StateStore>,
    queue: Arc<dyn Queue>,
    backoff: BackoffStrategy,
}

impl Executor {
    pub async fn execute_step(
        &self,
        item: &QueueItem,
    ) -> Result<ExecutionResult, ExecutorError> {
        // 1. Check for cancellation
        if self.state_store.is_cancelled(&item.identifier).await? {
            return Ok(ExecutionResult::Cancelled);
        }

        // 2. Fetch step configuration
        let function = self.state_store
            .get_function(&item.identifier.workflow_id)
            .await?;
        let step = function.steps.get(&item.step_id())
            .ok_or(ExecutorError::StepNotFound)?;

        // 3. Build request payload
        let request = self.build_sdk_request(&item).await?;

        // 4. Call SDK endpoint
        let response = self.http_client
            .post(&step.runtime.url)
            .json(&request)
            .send()
            .await?;

        // 5. Handle response
        match response.status() {
            200 => self.handle_completion(response, item).await,
            206 => self.handle_partial(response, item).await,
            500 => self.handle_error(response, item).await,
            _ => Err(ExecutorError::UnexpectedStatus(response.status())),
        }
    }

    async fn build_sdk_request(
        &self,
        item: &QueueItem,
    ) -> Result<SdkRequest, ExecutorError> {
        let state = self.state_store
            .get_run_state(&item.identifier)
            .await?;

        Ok(SdkRequest {
            ctx: CallContext {
                env: std::env::var("INNGEST_ENV").unwrap_or_else(|_| "dev".to_string()),
                fn_id: item.identifier.workflow_id.to_string(),
                run_id: item.identifier.run_id.to_string(),
                step_id: item.step_id(),
                attempt: item.attempt,
            },
            event: serde_json::to_value(&state.triggering_event.event)?,
            events: state.triggering_events_to_json()?,
            steps: state.step_outputs.clone(),
            use_api: false,
            version: 1,
        })
    }

    async fn handle_partial(
        &self,
        response: reqwest::Response,
        item: &QueueItem,
    ) -> Result<ExecutionResult, ExecutorError> {
        let opcodes: Vec<GeneratorOpcode> = response.json().await?;

        for opcode in opcodes {
            match opcode.op {
                OpcodeType::StepRun => {
                    // Schedule step execution
                    self.queue.enqueue(QueueItem::new_edge(
                        &item.identifier,
                        opcode.id,
                    )).await?;
                }
                OpcodeType::Sleep => {
                    let duration = parse_duration(&opcode.opts)?;
                    let wake_at = Utc::now() + duration;
                    self.queue.enqueue(QueueItem::new_sleep(
                        &item.identifier,
                        opcode.id,
                        wake_at,
                    )).await?;
                }
                OpcodeType::WaitForEvent => {
                    let pause = Pause::new_wait_for_event(
                        &item.identifier,
                        opcode.id,
                        &opcode.opts,
                    );
                    self.state_store.create_pause(pause.clone()).await?;
                }
                OpcodeType::InvokeFunction => {
                    // Handle function invocation
                    self.handle_invoke(&item.identifier, opcode).await?;
                }
            }
        }

        Ok(ExecutionResult::Partial)
    }

    async fn handle_completion(
        &self,
        response: reqwest::Response,
        item: &QueueItem,
    ) -> Result<ExecutionResult, ExecutorError> {
        let result: serde_json::Value = response.json().await?;

        // Update state
        self.state_store.complete_run(
            &item.identifier,
            result,
        ).await?;

        Ok(ExecutionResult::Completed)
    }

    async fn handle_error(
        &self,
        response: reqwest::Response,
        item: &QueueItem,
    ) -> Result<ExecutionResult, ExecutorError> {
        // Check retry headers
        let no_retry = response.headers()
            .get("X-Inngest-No-Retry")
            .is_some();

        if no_retry {
            self.state_store.fail_run(
                &item.identifier,
                FunctionError::Permanent(response.text().await?),
            ).await?;
            return Ok(ExecutionResult::Failed);
        }

        // Schedule retry with backoff
        let retry_at = self.backoff.next_retry(item.attempt);
        if item.attempt >= item.max_attempts.unwrap_or(5) {
            self.state_store.fail_run(
                &item.identifier,
                FunctionError::MaxRetriesExceeded,
            ).await?;
            return Ok(ExecutionResult::Failed);
        }

        self.queue.enqueue(QueueItem {
            attempt: item.attempt + 1,
            run_at: retry_at,
            ..item.clone()
        }).await?;

        Ok(ExecutionResult::Retrying)
    }
}
```

### 2. SDK Request/Response Types

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SdkRequest {
    pub ctx: CallContext,
    pub event: serde_json::Value,
    pub events: Vec<serde_json::Value>,
    pub steps: HashMap<String, Option<serde_json::Value>>,
    #[serde(rename = "useApi")]
    pub use_api: bool,
    pub version: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CallContext {
    pub attempt: u32,
    pub env: String,
    #[serde(rename = "fnId")]
    pub fn_id: String,
    #[serde(rename = "runId")]
    pub run_id: String,
    #[serde(rename = "stepId")]
    pub step_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratorOpcode {
    pub op: OpcodeType,
    pub id: String,
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    pub opts: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum OpcodeType {
    StepRun,
    Sleep,
    WaitForEvent,
    InvokeFunction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum SdkResponse {
    #[serde(rename = "200")]
    Complete { body: serde_json::Value },
    #[serde(rename = "206")]
    Partial { body: Vec<GeneratorOpcode> },
    #[serde(rename = "500")]
    Error { error: String },
}
```

---

## State Store Implementation

### 1. Redis State Store

```rust
use redis::{Client, Connection, AsyncCommands};

#[async_trait]
pub trait StateStore: Send + Sync {
    async fn create_run(&self, state: FunctionRunState) -> Result<(), StateError>;
    async fn get_run_state(&self, id: &RunIdentifier) -> Result<FunctionRunState, StateError>;
    async fn update_run(&self, state: FunctionRunState) -> Result<(), StateError>;
    async fn complete_run(&self, id: &RunIdentifier, result: serde_json::Value) -> Result<(), StateError>;
    async fn fail_run(&self, id: &RunIdentifier, error: FunctionError) -> Result<(), StateError>;
    async fn is_cancelled(&self, id: &RunIdentifier) -> Result<bool, StateError>;
    async fn create_pause(&self, pause: Pause) -> Result<(), StateError>;
    async fn get_pause(&self, id: Ulid) -> Result<Option<Pause>, StateError>;
    async fn consume_pause(&self, id: Ulid) -> Result<Pause, StateError>;
    async fn find_functions_by_trigger(&self, event_name: &str) -> Result<Vec<Function>, StateError>;
    async fn get_function(&self, id: &Uuid) -> Result<Function, StateError>;
}

pub struct RedisStateStore {
    client: Client,
}

#[async_trait]
impl StateStore for RedisStateStore {
    async fn create_run(&self, state: FunctionRunState) -> Result<(), StateError> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("run:{}", state.identifier.run_id);

        // Use WATCH for optimistic locking
        conn.watch(&key).await?;

        // Check if exists
        let exists: Option<String> = conn.get(&key).await?;
        if exists.is_some() {
            return Err(StateError::RunAlreadyExists);
        }

        let mut pipe = redis::pipe();
        pipe.atomic()
            .set(&key, serde_json::to_string(&state)?)
            .set(&format!("run:idempotency:{}", state.identifier.idempotency_key()), state.identifier.run_id.to_string());

        pipe.query_async(&mut conn).await?;
        Ok(())
    }

    async fn get_run_state(&self, id: &RunIdentifier) -> Result<FunctionRunState, StateError> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("run:{}", id.run_id);
        let data: String = conn.get(&key).await?;
        Ok(serde_json::from_str(&data)?)
    }

    async fn create_pause(&self, pause: Pause) -> Result<(), StateError> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("pause:{}", pause.id);

        // Set with TTL based on timeout
        let ttl = (pause.timeout_at - Utc::now()).num_seconds().max(1) as usize;

        conn.set_ex(&key, serde_json::to_string(&pause)?, ttl).await?;

        // Also index by event key for matching
        if let Some(event_key) = &pause.event_key {
            conn.s_add(&format!("pause:event:{}", event_key), pause.id.to_string()).await?;
        }

        Ok(())
    }

    async fn consume_pause(&self, id: Ulid) -> Result<Pause, StateError> {
        let mut conn = self.client.get_async_connection().await?;
        let key = format!("pause:{}", id);

        // Use Lua script for atomic lease
        let script = redis::Script::new(r#"
            local key = KEYS[1]
            local data = redis.call('GET', key)
            if not data then
                return nil
            end
            redis.call('DEL', key)
            return data
        "#);

        let result: Option<String> = script.key(&key).invoke_async(&mut conn).await?;
        match result {
            Some(data) => Ok(serde_json::from_str(&data)?),
            None => Err(StateError::PauseNotFound),
        }
    }
}
```

### 2. PostgreSQL for Persistence

```rust
use sqlx::{PgPool, FromRow};

#[derive(Debug, FromRow)]
pub struct DbFunction {
    pub id: Uuid,
    pub name: String,
    pub app_id: Uuid,
    pub triggers: serde_json::Value,
    pub steps: serde_json::Value,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PostgresStore {
    pool: PgPool,
}

impl PostgresStore {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn save_function(&self, function: &Function) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO functions (id, name, app_id, triggers, steps, config, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (id) DO UPDATE SET
                name = $2,
                triggers = $4,
                steps = $5,
                config = $6,
                updated_at = NOW()
            "#,
        )
        .bind(function.id)
        .bind(&function.name)
        .bind(function.app_id)
        .bind(serde_json::to_value(&function.triggers)?)
        .bind(serde_json::to_value(&function.steps)?)
        .bind(serde_json::to_value(&function)?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_event(&self, event: &TrackedEvent) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO events (
                internal_id, name, data, account_id, workspace_id, app_id, received_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(event.internal_id)
        .bind(&event.event.name)
        .bind(&event.event.data)
        .bind(event.account_id)
        .bind(event.workspace_id)
        .bind(event.app_id)
        .bind(event.received_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
```

---

## Queue Implementation

### 1. Redis Queue with ZSet

```rust
use redis::AsyncCommands;

#[async_trait]
pub trait Queue: Send + Sync {
    async fn enqueue(&self, item: QueueItem) -> Result<(), QueueError>;
    async fn dequeue(&self, workspace_id: Uuid) -> Result<Option<QueueItem>, QueueError>;
    async fn requeue(&self, item: QueueItem, delay: Duration) -> Result<(), QueueError>;
    async fn complete(&self, job_id: &str) -> Result<(), QueueError>;
}

pub struct RedisQueue {
    client: Client,
    namespace: String,
}

impl RedisQueue {
    pub async fn enqueue(&self, item: QueueItem) -> Result<(), QueueError> {
        let mut conn = self.client.get_async_connection().await?;

        // Serialize item
        let data = serde_json::to_string(&item)?;

        // Store item data
        let item_key = format!("{}:item:{}", self.namespace, item.job_id);
        conn.set(&item_key, &data).await?;

        // Add to sorted set with run_at as score
        let queue_key = format!("{}:queue:{}", self.namespace, item.workspace_id);
        let score = item.run_at.timestamp() as f64;
        conn.zadd(&queue_key, &item.job_id, score).await?;

        // Handle throttling
        if let Some(throttle) = &item.throttle {
            self.apply_throttle(&mut conn, &throttle.key, throttle.limit, throttle.period)
                .await?;
        }

        Ok(())
    }

    pub async fn dequeue(&self, workspace_id: Uuid) -> Result<Option<QueueItem>, QueueError> {
        let mut conn = self.client.get_async_connection().await?;

        let queue_key = format!("{}:queue:{}", self.namespace, workspace_id);
        let now = Utc::now().timestamp() as f64;

        // Use Lua script for atomic dequeue
        let script = redis::Script::new(r#"
            local queue_key = KEYS[1]
            local now = tonumber(ARGV[1])
            local limit = tonumber(ARGV[2])

            -- Get items ready to run
            local items = redis.call('ZRANGEBYSCORE', queue_key, '-inf', now, 'LIMIT', 0, limit)
            if #items == 0 then
                return nil
            end

            -- Remove and return first item
            local job_id = items[1]
            redis.call('ZREM', queue_key, job_id)

            return job_id
        "#);

        let job_id: Option<String> = script
            .key(&queue_key)
            .arg(now)
            .arg(1)
            .invoke_async(&mut conn)
            .await?;

        match job_id {
            Some(id) => {
                let item_key = format!("{}:item:{}", self.namespace, id);
                let data: String = conn.get(&item_key).await?;
                Ok(Some(serde_json::from_str(&data)?))
            }
            None => Ok(None),
        }
    }
}
```

### 2. Concurrency Manager

```rust
use redis::{Client, AsyncCommands};

pub struct ConcurrencyManager {
    client: Client,
    namespace: String,
}

impl ConcurrencyManager {
    pub async fn acquire(
        &self,
        key: &str,
        limit: usize,
    ) -> Result<bool, ConcurrencyError> {
        let mut conn = self.client.get_async_connection().await?;

        let script = redis::Script::new(r#"
            local key = KEYS[1]
            local limit = tonumber(ARGV[1])
            local run_id = ARGV[2]

            local current = redis.call('ZCARD', key)
            if current >= limit then
                return 0
            end

            redis.call('ZADD', key, os.time(), run_id)
            return 1
        "#);

        let acquired: i32 = script
            .key(&format!("{}:concurrency:{}", self.namespace, key))
            .arg(limit)
            .arg(run_id)
            .invoke_async(&mut conn)
            .await?;

        Ok(acquired == 1)
    }

    pub async fn release(&self, key: &str, run_id: &str) -> Result<(), ConcurrencyError> {
        let mut conn = self.client.get_async_connection().await?;
        let con_key = format!("{}:concurrency:{}", self.namespace, key);
        conn.zrem(&con_key, run_id).await?;
        Ok(())
    }
}
```

---

## Scheduling System

### 1. Cron Scheduler

```rust
use cron::Schedule;
use tokio::time::interval;

pub struct CronScheduler {
    state_store: Arc<dyn StateStore>,
    queue: Arc<dyn Queue>,
}

impl CronScheduler {
    pub fn start(&self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::seconds(10));
            loop {
                interval.tick().await;
                if let Err(e) = self.tick().await {
                    tracing::error!("Cron scheduler error: {}", e);
                }
            }
        })
    }

    async fn tick(&self) -> Result<(), SchedulerError> {
        let now = Utc::now();
        let functions = self.state_store.get_cron_functions().await?;

        for function in functions {
            for trigger in &function.triggers {
                if let Trigger::Cron { cron } = trigger {
                    let schedule: Schedule = cron.parse()?;

                    // Check if should run in this window
                    if schedule.upcoming(Utc).next()
                        .map(|t| t <= now + Duration::seconds(15))
                        .unwrap_or(false)
                    {
                        // Create synthetic event
                        let event = TrackedEvent::new_cron(&function, now);
                        self.state_store.save_event(&event).await?;

                        // Schedule function run
                        let identifier = RunIdentifier::new(&function, &event);
                        let state = FunctionRunState::new(identifier.clone(), &event);
                        self.state_store.create_run(state).await?;

                        self.queue.enqueue(QueueItem::new_start(identifier)).await?;
                    }
                }
            }
        }

        Ok(())
    }
}
```

### 2. Sleep Manager

```rust
pub struct SleepManager {
    queue: Arc<dyn Queue>,
}

impl SleepManager {
    pub fn start(&self) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::seconds(5));
            loop {
                interval.tick().await;
                // Process sleep queue items where run_at <= now
                // These are automatically picked up by the queue dequeue
            }
        })
    }
}
```

---

## Expression Evaluation (CEL)

```rust
use cel_interpreter::{Context, Program, Value as CelValue};

pub fn evaluate_expression(
    expression: &str,
    event: &TrackedEvent,
) -> Result<bool, ExpressionError> {
    let mut context = Context::default();

    // Add event data to context
    context.add_variable("event", CelValue::from_map(
        event.event.data.as_object()
            .map(|m| m.iter()
                .map(|(k, v)| (k.clone(), json_to_cel(v)))
                .collect()
            )
            .unwrap_or_default()
    )?);

    let program = Program::compile(expression)?;
    let result = program.execute(&context)?;

    match result {
        CelValue::Bool(b) => Ok(b),
        _ => Ok(true),
    }
}

fn json_to_cel(value: &serde_json::Value) -> CelValue {
    match value {
        serde_json::Value::Null => CelValue::Null,
        serde_json::Value::Bool(b) => CelValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                CelValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                CelValue::Double(f)
            } else {
                CelValue::Null
            }
        }
        serde_json::Value::String(s) => CelValue::String(s.clone()),
        serde_json::Value::Array(arr) => CelValue::List(
            arr.iter().map(json_to_cel).collect()
        ),
        serde_json::Value::Object(obj) => CelValue::Map(
            obj.iter().map(|(k, v)| (k.clone(), json_to_cel(v))).collect()
        ),
    }
}
```

---

## Cargo Dependencies

```toml
[package]
name = "inngest-rs-server"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web framework
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
hyper = { version = "1", features = ["full"] }
tower = "0.4"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Async runtime
async-trait = "0.1"
futures = "0.3"

# HTTP client
reqwest = { version = "0.11", features = ["json"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-native-tls", "postgres", "uuid", "chrono"] }
redis = { version = "0.24", features = ["aio", "tokio-comp", "cluster"] }

# NATS
async-nats = "0.33"

# Time
chrono = { version = "0.4", features = ["serde"] }

# IDs
uuid = { version = "1", features = ["v4", "serde"] }
ulid = { version = "1", features = ["serde"] }

# Cron
cron = "0.12"

# CEL expression evaluation
cel-interpreter = "0.5"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Error handling
thiserror = "1"
anyhow = "1"

# Configuration
config = "0.14"

# Metrics (optional)
metrics = "0.22"
metrics-exporter-prometheus = "0.13"
```

---

## Production Considerations

### 1. Observability

```rust
use tracing::{info, warn, error, instrument};
use metrics::{counter, histogram};

#[instrument(skip(self, item), fields(job_id = %item.job_id))]
async fn execute_step(&self, item: &QueueItem) -> Result<ExecutionResult, ExecutorError> {
    let start = std::time::Instant::now();

    let result = self.execute_inner(item).await;

    let duration = start.elapsed();
    histogram!("step_execution_duration", duration);
    counter!("step_executions_total", 1);

    match &result {
        Ok(r) => info!(status = ?r, "Step executed"),
        Err(e) => error!(error = %e, "Step execution failed"),
    }

    result
}
```

### 2. Graceful Shutdown

```rust
use tokio::signal;
use tokio::sync::watch;

pub struct ShutdownSignal {
    tx: watch::Sender<bool>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        let (tx, _) = watch::channel(false);
        Self { tx }
    }

    pub fn receiver(&self) -> watch::Receiver<bool> {
        self.tx.subscribe()
    }

    pub async fn wait_for_shutdown(&self) {
        let mut rx = self.receiver();
        let _ = rx.changed().await;
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(true);
    }
}

async fn graceful_shutdown(
    shutdown: ShutdownSignal,
    executor: Arc<Executor>,
) {
    signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
    info!("Shutting down gracefully...");
    shutdown.shutdown();

    // Wait for in-flight executions to complete
    // Timeout after 30 seconds
    tokio::time::timeout(
        Duration::from_secs(30),
        executor.drain()
    ).await.ok();
}
```

### 3. Configuration

```rust
use config::{Config, ConfigError, File};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
    pub database_url: String,
    pub redis_url: String,
    pub nats_url: Option<String>,
    pub signing_key: Option<String>,
}

impl ServerConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(config::Environment::with_prefix("INNGEST"))
            .build()?;

        config.try_deserialize()
    }
}
```

---

## Testing Strategy

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_op_hash() {
        let op = Op { id: "step-1".to_string(), pos: 0 };
        assert_eq!(
            op.hash(),
            "B4C9A28932D11D1B33D4F0B5A4F50E13E7D8F9A2"
        );
    }

    #[tokio::test]
    async fn test_state_store_create_run() {
        let store = RedisStateStore::new("redis://localhost").await.unwrap();
        let state = FunctionRunState::mock();
        store.create_run(state).await.unwrap();
    }
}
```

### 2. Integration Tests

```rust
#[cfg(test)]
mod integration {
    use testcontainers::*;

    #[tokio::test]
    async fn test_full_workflow() {
        // Start Redis container
        let redis = images::generic::GenericImage::new("redis", "7")
            .start()
            .await;

        // Start PostgreSQL container
        let postgres = images::generic::GenericImage::new("postgres", "15")
            .with_env_var("POSTGRES_PASSWORD", "test")
            .start()
            .await;

        // Run test...
    }
}
```

---

## Implementation Checklist

### Phase 1: Core Infrastructure
- [ ] Event ingestion API (Axum)
- [ ] Event stream (broadcast channel or NATS)
- [ ] State store trait + Redis implementation
- [ ] Basic queue with ZSet
- [ ] Function registration endpoint

### Phase 2: Execution Engine
- [ ] Runner (event consumer)
- [ ] Executor (step runner)
- [ ] SDK request/response handling
- [ ] Step position hashing
- [ ] Basic retry with backoff

### Phase 3: Flow Control
- [ ] Concurrency management
- [ ] Debounce implementation
- [ ] Rate limiting (GCRA)
- [ ] Batching support

### Phase 4: Advanced Features
- [ ] waitForEvent pauses
- [ ] Function invocation (invoke step)
- [ ] Cancellation handling
- [ ] Cron scheduling
- [ ] Expression evaluation (CEL)

### Phase 5: Production Readiness
- [ ] Graceful shutdown
- [ ] Observability (tracing, metrics)
- [ ] Configuration management
- [ ] Health check endpoints
- [ ] Admin API

---

## Key Differences from Go Implementation

1. **No panic-based control flow**: Rust uses Result types with explicit error propagation
2. **No reflection**: Step registration is explicit, no runtime function inspection
3. **Type safety**: Compile-time type checking vs Go's runtime reflection
4. **Memory safety**: No garbage collector, explicit memory management
5. **Async model**: Tokio runtime vs Go's goroutines

---

## Performance Optimizations

1. **Connection pooling**: Redis and PostgreSQL connection pools
2. **Batch operations**: Pipeline Redis commands
3. **Caching**: Cache function definitions and expression results
4. **Lazy loading**: Load step data on demand
5. **Zero-copy parsing**: Use `bytes` crate for efficient buffer handling
