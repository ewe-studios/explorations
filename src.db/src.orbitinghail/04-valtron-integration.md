---
title: "OrbitingHail Valtron Integration"
subtitle: "Using Valtron patterns for SQL sync with algebraic effects"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.orbitinghail
related: production-grade.md
---

# 04 - Valtron Integration: OrbitingHail

## Overview

This document covers integrating SQLSync with Valtron's algebraic effects pattern - handling sync I/O without async/await, managing offline queues, and CRDT conflict resolution.

## Part 1: SQLSync Op Enum

```rust
use valtron::{Effect, TaskResult, TaskIterator};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SQLSyncOp {
    // Database operations
    Insert {
        table: String,
        row_id: String,
        data: serde_json::Value,
    },
    Update {
        table: String,
        row_id: String,
        changes: serde_json::Value,
    },
    Delete {
        table: String,
        row_id: String,
    },
    Get {
        table: String,
        row_id: String,
    },
    Query {
        sql: String,
        params: Vec<serde_json::Value>,
    },

    // Clock operations
    GetClock,
    IncrementClock,

    // Change log operations
    RecordChange {
        change: ChangeRecord,
    },
    GetUnsyncedChanges {
        limit: usize,
    },
    MarkChangesSynced {
        change_ids: Vec<i64>,
    },

    // Sync operations
    SyncUpload {
        changes: Vec<ChangeRecord>,
    },
    SyncDownload {
        server_clock: VectorClock,
    },

    // Network effects
    WebSocketSend {
        message: String,
    },
    WebSocketReceive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SQLSyncResponse {
    // Database responses
    InsertSuccess { row_id: String, clock: VectorClock },
    UpdateSuccess { row_id: String, clock: VectorClock },
    DeleteSuccess { row_id: String },
    GetResult { row: Option<serde_json::Value> },
    QueryResult { rows: Vec<serde_json::Value> },

    // Clock responses
    Clock { clock: VectorClock },

    // Change log responses
    ChangeRecorded { id: i64 },
    UnsyncedChanges { changes: Vec<ChangeRecord> },
    ChangesSynced { count: usize },

    // Sync responses
    SyncAck { accepted: Vec<String> },
    SyncChanges { changes: Vec<ChangeRecord> },

    // Network responses
    MessageSent,
    MessageReceived { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub table: String,
    pub row_id: String,
    pub operation: Operation,
    pub changes: serde_json::Value,
    pub clock: VectorClock,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Operation {
    Insert,
    Update,
    Delete,
}
```

## Part 2: Insert Operation with Valtron

```rust
pub fn insert(
    table: String,
    row_id: String,
    data: serde_json::Value,
) -> TaskIterator<TaskResult<SQLSyncResponse, SQLSyncOp>, SQLSyncOp> {
    TaskIterator::new(|| {
        // Step 1: Get current clock
        let clock = valtron::effect!(SQLSyncOp::GetClock)?;

        // Step 2: Increment clock
        valtron::effect!(SQLSyncOp::IncrementClock)?;
        let new_clock = valtron::effect!(SQLSyncOp::GetClock)?;

        // Step 3: Insert into database
        valtron::effect!(SQLSyncOp::Insert {
            table: table.clone(),
            row_id: row_id.clone(),
            data: data.clone(),
        })?;

        // Step 4: Record change for sync
        let change = ChangeRecord {
            table: table.clone(),
            row_id: row_id.clone(),
            operation: Operation::Insert,
            changes: data.clone(),
            clock: new_clock.clone(),
            timestamp: current_timestamp_us(),
        };

        valtron::effect!(SQLSyncOp::RecordChange { change })?;

        Ok(SQLSyncResponse::InsertSuccess {
            row_id,
            clock: new_clock,
        })
    })
}

pub fn update(
    table: String,
    row_id: String,
    changes: serde_json::Value,
) -> TaskIterator<TaskResult<SQLSyncResponse, SQLSyncOp>, SQLSyncOp> {
    TaskIterator::new(|| {
        // Step 1: Get and increment clock
        valtron::effect!(SQLSyncOp::IncrementClock)?;
        let clock = valtron::effect!(SQLSyncOp::GetClock)?;

        // Step 2: Update database (with LWW conflict resolution)
        valtron::effect!(SQLSyncOp::Update {
            table: table.clone(),
            row_id: row_id.clone(),
            changes: changes.clone(),
        })?;

        // Step 3: Record change
        let change = ChangeRecord {
            table: table.clone(),
            row_id: row_id.clone(),
            operation: Operation::Update,
            changes: changes.clone(),
            clock: clock.clone(),
            timestamp: current_timestamp_us(),
        };

        valtron::effect!(SQLSyncOp::RecordChange { change })?;

        Ok(SQLSyncResponse::UpdateSuccess { row_id, clock })
    })
}

pub fn get(
    table: String,
    row_id: String,
) -> TaskIterator<TaskResult<SQLSyncResponse, SQLSyncOp>, SQLSyncOp> {
    TaskIterator::new(|| {
        let result = valtron::effect!(SQLSyncOp::Get {
            table,
            row_id,
        })?;

        Ok(SQLSyncResponse::GetResult { row: result })
    })
}

fn current_timestamp_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}
```

## Part 3: Sync Operation

```rust
pub fn sync_with_server(
    server_url: String,
) -> TaskIterator<TaskResult<SQLSyncResponse, SQLSyncOp>, SQLSyncOp> {
    TaskIterator::new(|| {
        // Step 1: Get unsynced changes
        let unsynced_result = valtron::effect!(SQLSyncOp::GetUnsyncedChanges { limit: 100 })?;
        let changes = match unsynced_result {
            SQLSyncResponse::UnsyncedChanges { changes } => changes,
            _ => return Err(EffectError::UnexpectedResponse),
        };

        // Step 2: Get current clock
        let client_clock = match valtron::effect!(SQLSyncOp::GetClock)? {
            SQLSyncResponse::Clock { clock } => clock,
            _ => return Err(EffectError::UnexpectedResponse),
        };

        // Step 3: Build sync request
        let mut tables: std::collections::HashMap<String, Vec<ChangeRecord>> =
            std::collections::HashMap::new();

        for change in &changes {
            tables
                .entry(change.table.clone())
                .or_insert_with(Vec::new)
                .push(change.clone());
        }

        let request = SyncRequest {
            client_clock: client_clock.clone(),
            tables,
        };

        let request_json = serde_json::to_string(&request)?;

        // Step 4: Send to server
        valtron::effect!(SQLSyncOp::WebSocketSend {
            message: request_json,
        })?;

        // Step 5: Receive response
        let response_json = match valtron::effect!(SQLSyncOp::WebSocketReceive)? {
            SQLSyncResponse::MessageReceived { message } => message,
            _ => return Err(EffectError::UnexpectedResponse),
        };

        let response: SyncResponse = serde_json::from_str(&response_json)?;

        // Step 6: Apply downloaded changes
        for (table_name, table_changes) in response.download {
            for change in table_changes {
                apply_remote_change(&table_name, &change)?;
            }
        }

        // Step 7: Mark uploaded changes as synced
        let accepted_ids: Vec<i64> = response.accepted.values()
            .flatten()
            .filter_map(|id| id.parse().ok())
            .collect();

        valtron::effect!(SQLSyncOp::MarkChangesSynced {
            change_ids: accepted_ids.clone(),
        })?;

        // Step 8: Merge server clock
        // (Handled by effect handler)

        Ok(SQLSyncResponse::SyncAck {
            accepted: accepted_ids.iter().map(|i| i.to_string()).collect(),
        })
    })
}

fn apply_remote_change(
    table: &str,
    change: &ChangeRecord,
) -> TaskIterator<TaskResult<(), SQLSyncOp>, SQLSyncOp> {
    TaskIterator::new(|| {
        match change.operation {
            Operation::Insert => {
                valtron::effect!(SQLSyncOp::Insert {
                    table: table.to_string(),
                    row_id: change.row_id.clone(),
                    data: change.changes.clone(),
                })?;
            }
            Operation::Update => {
                valtron::effect!(SQLSyncOp::Update {
                    table: table.to_string(),
                    row_id: change.row_id.clone(),
                    changes: change.changes.clone(),
                })?;
            }
            Operation::Delete => {
                valtron::effect!(SQLSyncOp::Delete {
                    table: table.to_string(),
                    row_id: change.row_id.clone(),
                })?;
            }
        }
        Ok(())
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub client_clock: VectorClock,
    pub tables: std::collections::HashMap<String, Vec<ChangeRecord>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub server_clock: VectorClock,
    pub accepted: std::collections::HashMap<String, Vec<String>>,
    pub download: std::collections::HashMap<String, Vec<ChangeRecord>>,
}
```

## Part 4: Effect Handler

```rust
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InMemorySQLSyncHandler {
    conn: Arc<RwLock<Connection>>,
    clock: Arc<RwLock<VectorClock>>,
    actor_id: String,
}

impl InMemorySQLSyncHandler {
    pub fn new(conn: Connection, actor_id: String) -> Self {
        Self {
            conn: Arc::new(RwLock::new(conn)),
            clock: Arc::new(RwLock::new(VectorClock::new())),
            actor_id,
        }
    }
}

impl valtron::Handler<SQLSyncOp> for InMemorySQLSyncHandler {
    type Response = SQLSyncResponse;
    type Error = SQLSyncEffectError;

    async fn handle(
        &self,
        op: SQLSyncOp,
    ) -> Result<TaskResult<SQLSyncOp, Self::Response>, Self::Error> {
        match op {
            SQLSyncOp::Insert { table, row_id, data } => {
                // In real implementation, execute SQL insert
                println!("Insert into {}: {} = {:?}", table, row_id, data);
                Ok(TaskResult::Complete(SQLSyncResponse::InsertSuccess {
                    row_id,
                    clock: self.clock.read().await.clone(),
                }))
            }

            SQLSyncOp::Update { table, row_id, changes } => {
                println!("Update {}: {} = {:?}", table, row_id, changes);
                Ok(TaskResult::Complete(SQLSyncResponse::UpdateSuccess {
                    row_id,
                    clock: self.clock.read().await.clone(),
                }))
            }

            SQLSyncOp::Delete { table, row_id } => {
                println!("Delete from {}: {}", table, row_id);
                Ok(TaskResult::Complete(SQLSyncResponse::DeleteSuccess { row_id }))
            }

            SQLSyncOp::Get { table, row_id } => {
                // Return mock data
                Ok(TaskResult::Complete(SQLSyncResponse::GetResult {
                    row: Some(json!({"id": row_id})),
                }))
            }

            SQLSyncOp::GetClock => {
                let clock = self.clock.read().await.clone();
                Ok(TaskResult::Complete(SQLSyncResponse::Clock { clock }))
            }

            SQLSyncOp::IncrementClock => {
                self.clock.write().await.tick(&self.actor_id);
                Ok(TaskResult::Complete(SQLSyncResponse::Clock {
                    clock: self.clock.read().await.clone(),
                }))
            }

            SQLSyncOp::RecordChange { change } => {
                println!("Recording change: {:?}", change);
                Ok(TaskResult::Complete(SQLSyncResponse::ChangeRecorded { id: 1 }))
            }

            SQLSyncOp::GetUnsyncedChanges { limit } => {
                // Return empty for now
                Ok(TaskResult::Complete(SQLSyncResponse::UnsyncedChanges {
                    changes: Vec::new(),
                }))
            }

            SQLSyncOp::MarkChangesSynced { change_ids } => {
                println!("Marking {} changes as synced", change_ids.len());
                Ok(TaskResult::Complete(SQLSyncResponse::ChangesSynced {
                    count: change_ids.len(),
                }))
            }

            SQLSyncOp::WebSocketSend { message } => {
                println!("WS Send: {}", message);
                Ok(TaskResult::Complete(SQLSyncResponse::MessageSent))
            }

            SQLSyncOp::WebSocketReceive => {
                // In real implementation, wait for message
                Ok(TaskResult::Continue {
                    effect: Some(SQLSyncOp::WebSocketReceive),
                    value: None,
                })
            }

            _ => Ok(TaskResult::Continue {
                effect: None,
                value: None,
            }),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SQLSyncEffectError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unexpected response")]
    UnexpectedResponse,

    #[error("Network error: {0}")]
    Network(String),
}
```

## Part 5: Edge Deployment

### Lambda Sync Handler

```rust
use aws_lambda_events::event::ApiGatewayWebsocketProxyRequest;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

pub struct LambdaSyncHandler {
    // Handler for sync requests via API Gateway WebSocket
}

impl LambdaSyncHandler {
    pub async fn handle_request(
        &self,
        event: LambdaEvent<ApiGatewayWebsocketProxyRequest>,
    ) -> Result<serde_json::Value, Error> {
        let (event, _context) = event.into_parts();

        // Parse sync request
        let request: SyncRequest = serde_json::from_str(
            &event.body.unwrap_or_default()
        )?;

        // Process sync (conflict resolution, etc.)
        let response = self.process_sync(request).await?;

        // Return response
        Ok(serde_json::json!({
            "action": "sendMessage",
            "data": serde_json::to_string(&response)?,
        }))
    }

    async fn process_sync(&self, request: SyncRequest) -> Result<SyncResponse, SyncError> {
        // Apply upload changes with CRDT merge
        let mut accepted = std::collections::HashMap::new();
        let mut download = std::collections::HashMap::new();

        for (table, changes) in request.tables {
            let mut table_accepted = Vec::new();

            for change in changes {
                // Check for conflicts and apply CRDT merge
                if self.apply_change_with_crdt(&table, &change).await? {
                    table_accepted.push(change.row_id.clone());
                }
            }

            accepted.insert(table.clone(), table_accepted);
        }

        Ok(SyncResponse {
            server_clock: request.client_clock, // Simplified
            accepted,
            download,
        })
    }

    async fn apply_change_with_crdt(
        &self,
        table: &str,
        change: &ChangeRecord,
    ) -> Result<bool, SyncError> {
        // LWW conflict resolution
        // In production, would check against existing data
        Ok(true)
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let handler = LambdaSyncHandler {};
    run(service_fn(|event| handler.handle_request(event))).await
}
```

---

*This document is part of the OrbitingHail exploration series. See [exploration.md](./exploration.md) for the complete index.*
