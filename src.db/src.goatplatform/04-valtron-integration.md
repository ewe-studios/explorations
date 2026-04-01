---
title: "GoatPlatform Valtron Integration"
subtitle: "Using Valtron patterns for real-time sync with algebraic effects"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.goatplatform
related: production-grade.md
---

# 05 - Valtron Integration: GoatPlatform

## Overview

This document covers integrating GoatPlatform with Valtron's algebraic effects pattern - handling sync I/O without async/await, managing offline queues, and conflict resolution.

## Part 1: Valtron Sync Operations

### GoatDB Op Enum

```rust
use valtron::{Effect, TaskResult, TaskIterator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoatDbOp {
    // Local database operations
    Insert {
        table: String,
        row_id: String,
        data: serde_json::Value,
    },
    Update {
        table: String,
        row_id: String,
        data: serde_json::Value,
    },
    Delete {
        table: String,
        row_id: String,
    },
    Get {
        table: String,
        row_id: String,
    },

    // Sync operations
    SyncUpload {
        changes: Vec<Change>,
    },
    SyncDownload {
        since_clock: VectorClock,
    },

    // Network effects
    WebSocketSend {
        message: String,
    },
    WebSocketReceive,

    // Persistence effects
    WalAppend {
        record: WalRecord,
    },
    Checkpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoatDbResponse {
    InsertSuccess { clock: VectorClock },
    UpdateSuccess { clock: VectorClock },
    DeleteSuccess { clock: VectorClock },
    GetResult { row: Option<serde_json::Value>, clock: Option<VectorClock> },
    SyncAck { acknowledged: Vec<String> },
    SyncChanges { changes: Vec<Change> },
    WebSocketMessage { message: String },
    WalAck,
}
```

### Insert Operation

```rust
pub fn insert(
    table: String,
    row_id: String,
    data: serde_json::Value,
) -> TaskIterator<TaskResult<GoatDbResponse, GoatDbOp>, GoatDbOp> {
    TaskIterator::new(move || {
        // Step 1: Read current clock
        let clock = valtron::effect!(GoatDbOp::GetClock)?;

        // Step 2: Increment clock
        let mut new_clock = clock.clone();
        new_clock.tick("local");

        // Step 3: Create change record
        let change = Change {
            table: table.clone(),
            row_id: row_id.clone(),
            operation: Operation::Insert,
            data: data.clone(),
            clock: new_clock.clone(),
        };

        // Step 4: Write to storage (effect)
        valtron::effect!(GoatDbOp::Insert {
            table: table.clone(),
            row_id: row_id.clone(),
            data: data.clone(),
        })?;

        // Step 5: Append to WAL
        let wal_record = WalRecord::from_change(&change);
        valtron::effect!(GoatDbOp::WalAppend { record: wal_record })?;

        // Step 6: Queue for sync (if offline, just queue; if online, send)
        valtron::effect!(GoatDbOp::QueueChange(change.clone()))?;

        // Return success
        Ok(GoatDbResponse::InsertSuccess { clock: new_clock })
    })
}

/// Get current clock effect
pub fn get_clock() -> TaskIterator<TaskResult<VectorClock, GoatDbOp>, GoatDbOp> {
    TaskIterator::effect(Effect::Io(|| {
        // Read clock from storage
        // This would be implemented by the handler
        Ok(VectorClock::new())
    }))
}
```

### Sync Operation

```rust
pub fn sync_with_server(
    server_url: String,
) -> TaskIterator<TaskResult<GoatDbResponse, GoatDbOp>, GoatDbOp> {
    TaskIterator::new(|| {
        // Step 1: Get pending changes from offline queue
        let pending = valtron::effect!(GoatDbOp::GetPendingChanges)?;

        // Step 2: Get current server clock
        let server_clock = valtron::effect!(GoatDbOp::GetServerClock)?;

        // Step 3: Build sync request
        let request = SyncRequest {
            client_clock: server_clock.clone(),
            tables: vec![],
            upload_changes: pending.clone(),
        };

        let request_json = serde_json::to_string(&request)?;

        // Step 4: Send to server via WebSocket
        valtron::effect!(GoatDbOp::WebSocketSend {
            message: request_json,
        })?;

        // Step 5: Receive response
        let response_json = valtron::effect!(GoatDbOp::WebSocketReceive)?;

        // Step 6: Parse response
        let response: SyncResponse = serde_json::from_str(&response_json)?;

        // Step 7: Apply downloaded changes
        for change in response.download_changes {
            apply_remote_change(change)?;
        }

        // Step 8: Acknowledge uploaded changes
        valtron::effect!(GoatDbOp::AcknowledgeChanges(response.accepted_changes.clone()))?;

        Ok(GoatDbResponse::SyncAck {
            acknowledged: response.accepted_changes,
        })
    })
}

fn apply_remote_change(
    change: Change,
) -> TaskIterator<TaskResult<(), GoatDbOp>, GoatDbOp> {
    TaskIterator::new(move || {
        match change.operation {
            Operation::Insert => {
                valtron::effect!(GoatDbOp::Insert {
                    table: change.table,
                    row_id: change.row_id,
                    data: change.data,
                })?;
            }
            Operation::Update => {
                valtron::effect!(GoatDbOp::Update {
                    table: change.table,
                    row_id: change.row_id,
                    data: change.data,
                })?;
            }
            Operation::Delete => {
                valtron::effect!(GoatDbOp::Delete {
                    table: change.table,
                    row_id: change.row_id,
                })?;
            }
            _ => {}
        }
        Ok(())
    })
}
```

## Part 2: Effect Handlers

### In-Memory Handler

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InMemoryGoatDbHandler {
    db: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    clock: Arc<RwLock<VectorClock>>,
    pending_changes: Arc<RwLock<Vec<Change>>>,
    server_clock: Arc<RwLock<VectorClock>>,
}

impl InMemoryGoatDbHandler {
    pub fn new() -> Self {
        Self {
            db: Arc::new(RwLock::new(HashMap::new())),
            clock: Arc::new(RwLock::new(VectorClock::new())),
            pending_changes: Arc::new(RwLock::new(Vec::new())),
            server_clock: Arc::new(RwLock::new(VectorClock::new())),
        }
    }
}

impl valtron::Handler<GoatDbOp> for InMemoryGoatDbHandler {
    type Response = GoatDbResponse;
    type Error = GoatDbError;

    async fn handle(
        &self,
        op: GoatDbOp,
    ) -> Result<TaskResult<GoatDbOp, Self::Response>, Self::Error> {
        match op {
            GoatDbOp::Insert { table, row_id, data } => {
                let key = format!("{}:{}", table, row_id);
                self.db.write().await.insert(key, data);
                Ok(TaskResult::Complete(GoatDbResponse::InsertSuccess {
                    clock: self.clock.read().await.clone(),
                }))
            }

            GoatDbOp::Update { table, row_id, data } => {
                let key = format!("{}:{}", table, row_id);
                self.db.write().await.insert(key, data);
                Ok(TaskResult::Complete(GoatDbResponse::UpdateSuccess {
                    clock: self.clock.read().await.clone(),
                }))
            }

            GoatDbOp::Delete { table, row_id } => {
                let key = format!("{}:{}", table, row_id);
                self.db.write().await.remove(&key);
                Ok(TaskResult::Complete(GoatDbResponse::DeleteSuccess {
                    clock: self.clock.read().await.clone(),
                }))
            }

            GoatDbOp::Get { table, row_id } => {
                let key = format!("{}:{}", table, row_id);
                let row = self.db.read().await.get(&key).cloned();
                Ok(TaskResult::Complete(GoatDbResponse::GetResult {
                    row,
                    clock: None,
                }))
            }

            GoatDbOp::GetClock => {
                let clock = self.clock.read().await.clone();
                Ok(TaskResult::Continue {
                    effect: None,
                    value: Some(GoatDbResponse::GetClock { clock }),
                })
            }

            GoatDbOp::QueueChange(change) => {
                self.pending_changes.write().await.push(change);
                Ok(TaskResult::Complete(GoatDbResponse::Queued))
            }

            GoatDbOp::GetPendingChanges => {
                let changes = self.pending_changes.read().await.clone();
                Ok(TaskResult::Complete(GoatDbResponse::PendingChanges { changes }))
            }

            GoatDbOp::WebSocketSend { message } => {
                // In real implementation, send via WebSocket
                println!("WS Send: {}", message);
                Ok(TaskResult::Complete(GoatDbResponse::Sent))
            }

            GoatDbOp::WebSocketReceive => {
                // In real implementation, wait for message
                Ok(TaskResult::Continue {
                    effect: Some(GoatDbOp::WebSocketReceive),
                    value: None,
                })
            }

            GoatDbOp::WalAppend { record } => {
                // Append to WAL
                println!("WAL Append: {:?}", record);
                Ok(TaskResult::Complete(GoatDbResponse::WalAck))
            }

            _ => Ok(TaskResult::Continue {
                effect: None,
                value: None,
            }),
        }
    }
}
```

### WebSocket Handler

```rust
use tokio::sync::broadcast;

pub struct WebSocketHandler {
    tx: broadcast::Sender<String>,
    rx: broadcast::Receiver<String>,
}

impl WebSocketHandler {
    pub fn new() -> Self {
        let (tx, rx) = broadcast::channel(1000);
        Self { tx, rx }
    }

    pub fn get_sender(&self) -> broadcast::Sender<String> {
        self.tx.clone()
    }
}

impl valtron::Handler<GoatDbOp> for WebSocketHandler {
    type Response = GoatDbResponse;
    type Error = GoatDbError;

    async fn handle(&self, op: GoatDbOp) -> Result<TaskResult<GoatDbOp, Self::Response>, Self::Error> {
        match op {
            GoatDbOp::WebSocketSend { message } => {
                self.tx.send(message)?;
                Ok(TaskResult::Complete(GoatDbResponse::Sent))
            }

            GoatDbOp::WebSocketReceive => {
                let mut rx = self.tx.subscribe();
                match rx.recv().await {
                    Ok(message) => Ok(TaskResult::Complete(GoatDbResponse::WebSocketMessage { message })),
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Missed messages, retry
                        Ok(TaskResult::Continue {
                            effect: Some(GoatDbOp::WebSocketReceive),
                            value: None,
                        })
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        Err(GoatDbError::ConnectionClosed)
                    }
                }
            }

            _ => Ok(TaskResult::Continue {
                effect: None,
                value: None,
            }),
        }
    }
}
```

## Part 3: Edge Deployment

### Lambda Sync Function

```rust
use aws_lambda_events::event::ApiGatewayWebsocketProxyRequest;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

pub struct LambdaSyncHandler {
    db: Arc<GoatDb>,
    connection_id_table: String,
}

impl LambdaSyncHandler {
    pub fn new(db: Arc<GoatDb>, connection_id_table: String) -> Self {
        Self { db, connection_id_table }
    }

    pub async fn handle_request(
        &self,
        event: LambdaEvent<ApiGatewayWebsocketProxyRequest>,
    ) -> Result<serde_json::Value, Error> {
        let (event, _context) = event.into_parts();

        // Parse request body
        let body: SyncRequest = serde_json::from_str(&event.body.unwrap_or_default())?;

        // Process sync request
        let response = self.process_sync(body).await?;

        // Send response back to client
        let response_json = serde_json::to_string(&response)?;

        Ok(serde_json::json!({
            "action": "sendMessage",
            "data": response_json,
        }))
    }

    async fn process_sync(&self, request: SyncRequest) -> Result<SyncResponse, SyncError> {
        // Apply upload changes
        let mut accepted = Vec::new();
        for change in request.upload_changes {
            // Resolve conflicts
            if self.resolve_conflict(&change).await? {
                // Apply change
                self.db.apply_change(change).await?;
                accepted.push(change.row_id.clone());
            }
        }

        // Get download changes
        let download_changes = self
            .db
            .changes_since(&request.client_clock)
            .await?;

        Ok(SyncResponse {
            server_clock: self.db.get_clock().await,
            accepted_changes: accepted,
            rejected_changes: vec![],
            download_changes,
        })
    }

    async fn resolve_conflict(&self, change: &Change) -> Result<bool, SyncError> {
        // Check for conflicts
        let existing = self.db.get_change(&change.table, &change.row_id).await?;

        if let Some(existing) = existing {
            match existing.clock.compare(&change.clock) {
                ClockOrdering::Concurrent => {
                    // Conflict! Use LWW or custom resolver
                    return Ok(true); // Accept incoming for now
                }
                ClockOrdering::Before => {
                    // Incoming is newer, accept
                    return Ok(true);
                }
                ClockOrdering::After => {
                    // Incoming is stale, reject
                    return Ok(false);
                }
                ClockOrdering::Equal => {
                    // Same change, accept (idempotent)
                    return Ok(true);
                }
            }
        }

        Ok(true)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let db = Arc::new(GoatDb::open(GoatDbConfig {
        path: "/tmp/goatdb".to_string(),
        ..Default::default()
    }).await?);

    let handler = LambdaSyncHandler::new(
        db,
        "connections".to_string(),
    );

    run(service_fn(|event| async {
        handler.handle_request(event).await
    })).await
}
```

---

*This document is part of the GoatPlatform exploration series. See [exploration.md](./exploration.md) for the complete index.*
