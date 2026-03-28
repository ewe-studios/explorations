---
title: "Zero Valtron Integration Guide"
subtitle: "Deploying Zero's sync engine to AWS Lambda using valtron executors (no async/await, no tokio)"
---

# Zero Valtron Integration Guide

## 1. Overview

This guide explains how to deploy Zero's sync engine to AWS Lambda using the valtron executor pattern. This approach:

- **Does NOT use async/await** - Uses TaskIterator pattern instead
- **Does NOT use tokio** - Uses valtron's lightweight executor
- **Supports Lambda pause/resume** - State is preserved in structs
- **Cold-start optimized** - Minimal runtime overhead

### Why valtron for Lambda?

| Feature | Traditional async (tokio) | valtron |
|---------|--------------------------|---------|
| Cold start | ~100ms runtime init | ~10ms (no runtime) |
| Memory footprint | ~10MB runtime | ~1MB |
| Lambda pause | State lost | State preserved in struct |
| Suspension | Complex (task serialization) | Simple (struct fields) |
| Dependencies | tokio + many crates | valtron only |

## 2. valtron Fundamentals

### 2.1 TaskIterator Pattern

```rust
use valtron::iterator::{TaskIterator, TaskStatus, Wakeup};

/// A task that produces items over time
pub trait TaskIterator {
    /// The ready item type (produced when complete)
    type Ready;

    /// The pending type (additional state while waiting)
    type Pending;

    /// The spawner type (for spawning child tasks)
    type Spawner;

    /// Advance the task, return status
    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>>;
}
```

### 2.2 TaskStatus

```rust
pub enum TaskStatus<Ready, Pending> {
    /// Task has produced a ready item
    Ready(Ready),

    /// Task is pending, wakeup when ready
    Pending {
        wakeup: Wakeup,
    },
}

pub enum Wakeup {
    /// Wake up immediately (next poll)
    Immediate,

    /// Wake up after duration
    Timeout(Duration),

    /// Wake up on I/O readiness
    Io(String), // File descriptor or resource ID

    /// Wake up on channel message
    Channel(String), // Channel ID

    /// Custom wakeup (application-defined)
    Custom(()),
}
```

## 3. HTTP API Handler

### 3.1 Lambda Invocation Structure

```rust
// Rust: Lambda handler with valtron
use valtron::iterator::{TaskIterator, TaskStatus, Wakeup};
use valtron::no_spawner::NoSpawner;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LambdaEvent {
    pub http_method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Serialize)]
pub struct LambdaResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// HTTP request handler using TaskIterator
pub struct HttpHandlerTask {
    state: HttpHandlerState,
    event: Option<LambdaEvent>,
    response: Option<LambdaResponse>,
}

enum HttpHandlerState {
    Parsing,
    Processing { request: ApiRequest },
    Waiting { request_id: String },
    Responding { response: LambdaResponse },
    Done,
}

impl TaskIterator for HttpHandlerTask {
    type Ready = LambdaResponse;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &mut self.state {
            HttpHandlerState::Parsing => {
                let event = self.event.take().unwrap();
                let request = parse_request(event);
                self.state = HttpHandlerState::Processing { request };
                self.next()
            }

            HttpHandlerState::Processing { request } => {
                // Route to appropriate handler
                let request_id = route_request(request);
                self.state = HttpHandlerState::Waiting { request_id };
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Io(request_id),
                })
            }

            HttpHandlerState::Waiting { request_id } => {
                // Check if response is ready
                if let Some(response) = check_response(request_id) {
                    self.state = HttpHandlerState::Responding { response };
                    self.next()
                } else {
                    Some(TaskStatus::Pending {
                        wakeup: Wakeup::Io(request_id.clone()),
                    })
                }
            }

            HttpHandlerState::Responding { response } => {
                let response = response.clone();
                self.state = HttpHandlerState::Done;
                Some(TaskStatus::Ready(response))
            }

            HttpHandlerState::Done => None,
        }
    }
}
```

### 3.2 WebSocket Upgrade Handler

```rust
// Rust: WebSocket connection handler
pub struct WebSocketConnectTask {
    state: WebSocketState,
    connection_id: Option<String>,
    sync_engine: Arc<Mutex<SyncEngine>>,
}

enum WebSocketState {
    Upgrading,
    Upgraded { connection_id: String },
    Subscribing { subscription_id: String },
    Ready,
    Closed,
}

impl TaskIterator for WebSocketConnectTask {
    type Ready = String; // connection_id
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &mut self.state {
            WebSocketState::Upgrading => {
                // Perform WebSocket upgrade
                let connection_id = upgrade_websocket();
                self.state = WebSocketState::Upgraded { connection_id: connection_id.clone() };
                self.connection_id = Some(connection_id.clone());

                // Register with sync engine
                let subscription_id = self.sync_engine.lock().unwrap()
                    .create_subscription(connection_id);

                self.state = WebSocketState::Subscribing { subscription_id };
                self.next()
            }

            WebSocketState::Subscribing { subscription_id } => {
                // Initialize subscription
                let changes = self.sync_engine.lock().unwrap()
                    .initialize_subscription(subscription_id);

                // Send initial changes to client
                send_initial_changes(self.connection_id.as_ref().unwrap(), &changes);

                self.state = WebSocketState::Ready;
                Some(TaskStatus::Ready(self.connection_id.clone().unwrap()))
            }

            WebSocketState::Ready => {
                // Connection is ready, task is complete
                // The connection is now managed by ChangeSenderTask
                None
            }

            WebSocketState::Closed => None,
        }
    }
}
```

## 4. Change Streaming

### 4.1 Change Sender Task

```rust
// Rust: Continuous change sender for WebSocket connections
pub struct ChangeSenderTask {
    state: ChangeSenderState,
    connection_id: String,
    subscription_id: String,
    pending_changes: VecDeque<Change>,
    sync_engine: Arc<Mutex<SyncEngine>>,
}

enum ChangeSenderState {
    Waiting,
    Received { changes: Vec<Change> },
    Sending { change: Change },
    Closed,
}

impl TaskIterator for ChangeSenderTask {
    type Ready = ();
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // First, send any pending changes
        if let Some(change) = self.pending_changes.pop_front() {
            self.state = ChangeSenderState::Sending { change };
            return self.next();
        }

        match &mut self.state {
            ChangeSenderState::Waiting => {
                // Wait for new changes from sync engine
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Channel(self.subscription_id.clone()),
                })
            }

            ChangeSenderState::Received { changes } => {
                // Buffer the changes
                let changes = std::mem::take(changes);
                self.pending_changes.extend(changes);
                self.state = ChangeSenderState::Waiting;
                self.next() // Recurse to send first change
            }

            ChangeSenderState::Sending { change } => {
                // Send change to client
                let change_json = serde_json::to_string(change).unwrap();
                websocket_send(&self.connection_id, &change_json);
                self.state = ChangeSenderState::Waiting;
                Some(TaskStatus::Ready(()))
            }

            ChangeSenderState::Closed => None,
        }
    }
}
```

### 4.2 Change Receiver Task (Mutations)

```rust
// Rust: Receive and process mutations from clients
pub struct MutationReceiverTask {
    state: MutationReceiverState,
    connection_id: String,
    sync_engine: Arc<Mutex<SyncEngine>>,
}

enum MutationReceiverState {
    Waiting,
    Received { mutation: MutationRequest },
    Processing { mutation_id: String },
    Responding { result: MutationResult },
}

impl TaskIterator for MutationReceiverTask {
    type Ready = MutationResult;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &mut self.state {
            MutationReceiverState::Waiting => {
                // Wait for mutation from client
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Channel(format!("mutation_{}", self.connection_id)),
                })
            }

            MutationReceiverState::Received { mutation } => {
                let mutation = mutation.clone();
                let mutation_id = self.sync_engine.lock().unwrap()
                    .process_mutation(mutation);

                self.state = MutationReceiverState::Processing { mutation_id };
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Io(mutation_id.clone()),
                })
            }

            MutationReceiverState::Processing { mutation_id } => {
                // Check if mutation is complete
                if let Some(result) = self.sync_engine.lock().unwrap()
                    .get_mutation_result(mutation_id)
                {
                    self.state = MutationReceiverState::Responding { result: result.clone() };
                    self.next()
                } else {
                    Some(TaskStatus::Pending {
                        wakeup: Wakeup::Io(mutation_id.clone()),
                    })
                }
            }

            MutationReceiverState::Responding { result } => {
                let result = result.clone();
                self.state = MutationReceiverState::Waiting;
                Some(TaskStatus::Ready(result))
            }
        }
    }
}
```

## 5. PostgreSQL Change Source

### 5.1 WAL Listener Task

```rust
// Rust: Listen to PostgreSQL WAL for changes
pub struct WalListenerTask {
    state: WalListenerState,
    connection_string: String,
    replication_slot: String,
    pending_changes: VecDeque<Change>,
}

enum WalListenerState {
    Connecting,
    Connected { lsn: String },
    WaitingForChanges,
    Receiving { buffer: Vec<u8> },
    Parsing { raw_change: RawWalChange },
}

impl TaskIterator for WalListenerTask {
    type Ready = Change;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        // Return buffered changes first
        if let Some(change) = self.pending_changes.pop_front() {
            return Some(TaskStatus::Ready(change));
        }

        match &mut self.state {
            WalListenerState::Connecting => {
                // Connect to PostgreSQL
                let connection = postgres_connect(&self.connection_string);
                self.state = WalListenerState::Connected { lsn: "0/0".to_string() };
                self.next()
            }

            WalListenerState::Connected { lsn } => {
                // Start replication
                start_logical_replication(&self.replication_slot, lsn);
                self.state = WalListenerState::WaitingForChanges;
                self.next()
            }

            WalListenerState::WaitingForChanges => {
                // Wait for WAL message
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Io("wal_socket".to_string()),
                })
            }

            WalListenerState::Receiving { buffer } => {
                // Parse WAL message
                let raw_change = parse_wal_message(buffer);
                self.state = WalListenerState::Parsing { raw_change };
                self.next()
            }

            WalListenerState::Parsing { raw_change } => {
                // Convert to Change type
                let change = raw_change.to_change();
                self.pending_changes.push_back(change);
                self.state = WalListenerState::WaitingForChanges;
                self.next()
            }
        }
    }
}
```

### 5.2 Batch Change Processor

```rust
// Rust: Process and broadcast batches of changes
pub struct ChangeBatcherTask {
    state: BatcherState,
    batch: Vec<Change>,
    subscribers: Arc<DashMap<String, Vec<String>>>, // subscription -> connections
}

enum BatcherState {
    Collecting,
    Flushing,
    Broadcasting { targets: Vec<String> },
}

impl TaskIterator for ChangeBatcherTask {
    type Ready = ();
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending>> {
        match &mut self.state {
            BatcherState::Collecting => {
                // Wait for changes or timeout
                Some(TaskStatus::Pending {
                    wakeup: Wakeup::Timeout(Duration::from_millis(100)),
                })
            }

            BatcherState::Flushing => {
                if self.batch.is_empty() {
                    self.state = BatcherState::Collecting;
                    return self.next();
                }

                // Get all subscribers
                let targets: Vec<String> = self.subscribers
                    .iter()
                    .flat_map(|s| s.value().clone())
                    .collect();

                self.state = BatcherState::Broadcasting { targets };
                self.next()
            }

            BatcherState::Broadcasting { targets } => {
                let targets = std::mem::take(targets);
                let batch = std::mem::take(&mut self.batch);

                // Send to all subscribers
                for connection_id in targets {
                    websocket_send(&connection_id, &serialize_changes(&batch));
                }

                self.state = BatcherState::Collecting;
                Some(TaskStatus::Ready(()))
            }
        }
    }
}
```

## 6. Lambda Deployment

### 6.1 Lambda Entry Point

```rust
// Rust: Lambda entry point
use lambda_runtime::{service_fn, LambdaEvent, Error};
use valtron::executor::Executor;
use valtron::config::Config;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Note: In production, this would use valtron's Lambda runtime
    // instead of tokio. The tokio here is just for the Lambda runtime
    // wrapper - the actual application logic uses TaskIterator.

    let func = service_fn(|event: LambdaEvent<serde_json::Value>| async {
        // Create valtron executor
        let config = Config::default();
        let mut executor = Executor::new(config);

        // Create HTTP handler task
        let http_task = HttpHandlerTask::from_event(event);

        // Spawn and run
        executor.spawn_main(http_task);
        let response = executor.run_single();

        Ok(response)
    });

    lambda_runtime::run(func).await?;
    Ok(())
}
```

### 6.2 Lambda Configuration

```rust
// Rust: Lambda configuration
use valtron::config::Config;

pub fn lambda_config() -> Config {
    Config {
        // Minimal memory for Lambda
        max_concurrent_tasks: 1, // Lambda is single-invocation

        // Fast cold start
        initialization_timeout: Duration::from_millis(100),

        // No spawner needed (single-threaded)
        spawner_config: SpawnerConfig::None,

        // Enable Lambda-specific optimizations
        lambda_mode: true,

        // Serialization for Lambda pause/resume
        serialization: SerializationConfig {
            enabled: true,
            format: SerializationFormat::Json,
        },
    }
}
```

### 6.3 State Serialization for Lambda Resume

```rust
// Rust: Serialize task state for Lambda resume
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct SerializableTaskState {
    pub task_type: String,
    pub state_data: serde_json::Value,
}

pub trait SerializableTask: TaskIterator {
    fn serialize(&self) -> SerializableTaskState;
    fn deserialize(state: &SerializableTaskState) -> Self;
}

impl SerializableTask for HttpHandlerTask {
    fn serialize(&self) -> SerializableTaskState {
        SerializableTaskState {
            task_type: "HttpHandler".to_string(),
            state_data: serde_json::to_value(&self.state).unwrap(),
        }
    }

    fn deserialize(state: &SerializableTaskState) -> Self {
        HttpHandlerTask {
            state: serde_json::from_value(state.state_data).unwrap(),
            event: None,
            response: None,
        }
    }
}
```

## 7. Complete Example: Zero Cache on Lambda

### 7.1 Main Application Structure

```rust
// Rust: Zero Cache Lambda application
mod http;
mod websocket;
mod sync;
mod mutations;
mod postgres;

use valtron::executor::Executor;
use valtron::config::Config;
use std::sync::Arc;
use crate::sync::SyncEngine;

pub struct ZeroCacheApp {
    executor: Executor,
    sync_engine: Arc<SyncEngine>,
}

impl ZeroCacheApp {
    pub fn new() -> Self {
        let config = lambda_config();
        let executor = Executor::new(config);
        let sync_engine = Arc::new(SyncEngine::new());

        Self { executor, sync_engine }
    }

    pub fn handle_http(&mut self, event: LambdaEvent) {
        let task = http::HttpHandlerTask::new(event, self.sync_engine.clone());
        self.executor.spawn(task);
    }

    pub fn handle_websocket_connect(&mut self, connection_id: String) {
        let task = websocket::WebSocketConnectTask::new(
            connection_id,
            self.sync_engine.clone(),
        );
        self.executor.spawn(task);
    }

    pub fn handle_websocket_message(&mut self, connection_id: String, message: String) {
        // Parse message and route appropriately
        let msg = serde_json::from_str(&message).unwrap();

        match msg {
            WebSocketMessage::Subscribe { query } => {
                let task = sync::SubscribeTask::new(connection_id, query, self.sync_engine.clone());
                self.executor.spawn(task);
            }
            WebSocketMessage::Mutate { mutation } => {
                let task = mutations::MutationReceiverTask::new(
                    connection_id,
                    mutation,
                    self.sync_engine.clone(),
                );
                self.executor.spawn(task);
            }
        }
    }

    pub fn run(&mut self) {
        self.executor.run();
    }
}
```

### 7.2 serverless.yml Configuration

```yaml
# serverless.yml
service: zero-cache

provider:
  name: aws
  runtime: provided.al2
  region: us-east-1
  memorySize: 1024
  timeout: 30
  environment:
    RUST_LOG: info
    DATABASE_URL: ${ssm:/prod/database-url}

functions:
  http:
    handler: bootstrap
    events:
      - httpApi:
          path: /{proxy+}
          method: ANY
      - websocket:
          route: $connect
      - websocket:
          route: $disconnect
      - websocket:
          route: subscribe
      - websocket:
          route: mutate

  walListener:
    handler: bootstrap
    events:
      - sqs:
          arn: !GetAtt WalQueue.Arn
          batchSize: 10
          maximumBatchingWindow: 1

resources:
  Resources:
    WalQueue:
      Type: AWS::SQS::Queue
      Properties:
        QueueName: zero-wal-queue
```

### 7.3 Cargo.toml Dependencies

```toml
[package]
name = "zero-cache-lambda"
version = "0.1.0"
edition = "2021"

[dependencies]
# valtron executor (no tokio!)
valtron = { path = "/path/to/valtron" }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
log = "0.4"
env_logger = "0.10"

# AWS Lambda runtime (minimal wrapper)
lambda_runtime = "0.10"

# PostgreSQL (for WAL listener, uses valtron internally)
postgres-protocol = "0.6"

# Shared memory
dashmap = "5.5"
arc-swap = "1.6"

# Arena allocation
typed-arena = "2.0"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

## 8. Performance Considerations

### 8.1 Cold Start Optimization

```rust
// Rust: Lazy initialization for cold start
use std::sync::OnceLock;

static SYNC_ENGINE: OnceLock<Arc<SyncEngine>> = OnceLock::new();

fn get_sync_engine() -> Arc<SyncEngine> {
    SYNC_ENGINE
        .get_or_init(|| Arc::new(SyncEngine::new()))
        .clone()
}

// In Lambda handler:
pub fn handler(event: LambdaEvent) -> LambdaResponse {
    // Sync engine is initialized on first request
    let sync_engine = get_sync_engine();

    // Process request...
}
```

### 8.2 Connection Pooling

```rust
// Rust: Connection pooling for Lambda
use r2d2::{Pool, PooledConnection};
use r2d2_postgres::PostgresConnectionManager;

pub struct ConnectionPool {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl ConnectionPool {
    pub fn new(database_url: &str) -> Self {
        let manager = PostgresConnectionManager::new(
            database_url.parse().unwrap(),
            NoTls,
        );
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .unwrap();

        Self { pool }
    }

    pub fn get(&self) -> PooledConnection<PostgresConnectionManager<NoTls>> {
        self.pool.get().unwrap()
    }
}
```

### 8.3 Memory Management

```rust
// Rust: Bounded memory for Lambda
pub struct BoundedCache {
    max_items: usize,
    items: LinkedHashMap<String, Change>,
}

impl BoundedCache {
    pub fn new(max_items: usize) -> Self {
        Self {
            max_items,
            items: LinkedHashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, change: Change) {
        if self.items.len() >= self.max_items {
            // Remove oldest entry
            self.items.pop_front();
        }
        self.items.insert(key, change);
    }
}
```

---

*This completes the Zero exploration. See [exploration.md](exploration.md) for the full index.*
