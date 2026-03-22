---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/instantdb/
explored_at: 2026-03-22
revised_at: 2026-03-22
workspace: instantdb-rust-workspace
---

# Rust Revision: InstantDB Clone

## Overview

This document translates InstantDB's architecture into Rust, providing idiomatic implementations for building a real-time local-first database with sync capabilities.

## Workspace Structure

```
instantdb-rust-workspace/
├── Cargo.toml                    # Workspace definition
├── crates/
│   ├── instant-client/           # Core client library
│   ├── instant-sync/             # Sync protocol and WebSocket handling
│   ├── instant-storage/          # SQLite/IndexedDB storage layer
│   ├── instant-query/            # Datalog/InstaQL query engine
│   ├── instant-perms/            # CEL-like permission evaluator
│   └── instant-crdt/             # CRDT types for conflict resolution
├── examples/
│   ├── chat-app/                 # Example chat application
│   └── collaborative-editor/     # Real-time editing example
└── tests/
    ├── integration/              # End-to-end sync tests
    └── performance/              # Benchmark tests
```

### Workspace Cargo.toml

```toml
[workspace]
members = [
    "crates/instant-client",
    "crates/instant-sync",
    "crates/instant-storage",
    "crates/instant-query",
    "crates/instant-perms",
    "crates/instant-crdt",
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.7", features = ["sqlite", "postgres", "runtime-tokio-rustls"] }
thiserror = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

## Crate 1: instant-client

### Purpose
Core client API with idiomatic Rust patterns for queries, transactions, and sync state management.

### Cargo.toml

```toml
[package]
name = "instant-client"
version = "0.1.0"
edition = "2021"

[dependencies]
instant-sync = { path = "../instant-sync" }
instant-storage = { path = "../instant-storage" }
instant-query = { path = "../instant-query" }
tokio = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
uuid = { workspace = true }
dashmap = "5"  # Concurrent HashMap
im = "15"      # Immutable data structures
```

### Core Types

```rust
// crates/instant-client/src/lib.rs
use instant_sync::SyncEngine;
use instant_storage::LocalStore;
use instant_query::{Query, QueryResult};
use uuid::Uuid;
use std::sync::Arc;
use dashmap::DashMap;

pub struct InstantDb {
    app_id: String,
    local_store: Arc<LocalStore>,
    sync_engine: Arc<SyncEngine>,
    query_cache: DashMap<QueryId, CachedQuery>,
    transaction_queue: tokio::sync::mpsc::Sender<Transaction>,
}

pub struct InstantDbBuilder {
    app_id: String,
    websocket_url: Option<String>,
    storage_path: Option<PathBuf>,
}

impl InstantDb {
    pub fn builder(app_id: impl Into<String>) -> InstantDbBuilder {
        InstantDbBuilder {
            app_id: app_id.into(),
            websocket_url: None,
            storage_path: None,
        }
    }

    /// Execute a query against local cache with sync fallback
    pub async fn query<Q: Into<Query>>(&self, query: Q) -> Result<QueryResult> {
        let query = query.into();

        // Try cache first
        if let Some(cached) = self.query_cache.get(&query.id()) {
            if cached.is_fresh() {
                return Ok(cached.result.clone());
            }
        }

        // Query local store
        let local_result = self.local_store.query(&query).await?;

        // Register for sync updates
        self.sync_engine.register_query(query.clone()).await?;

        Ok(local_result)
    }

    /// Execute a transaction with optimistic updates
    pub async fn transact(&self, tx: Transaction) -> Result<TransactionResult> {
        // Apply optimistically
        let optimistic_result = tx.apply_local(&self.local_store).await?;

        // Queue for sync
        self.transaction_queue.send(tx).await?;

        Ok(optimistic_result)
    }

    /// Subscribe to query updates (real-time)
    pub fn subscribe<F>(&self, query: Query, callback: F) -> Subscription
    where
        F: Fn(QueryResult) + Send + Sync + 'static,
    {
        self.sync_engine.subscribe(query, callback)
    }
}
```

### Transaction Builder Pattern

```rust
// crates/instant-client/src/transaction.rs
use uuid::Uuid;
use serde::Serialize;

#[derive(Clone)]
pub struct Transaction {
    id: Uuid,
    operations: Vec<Operation>,
    optimistic: bool,
}

#[derive(Clone, Serialize)]
pub enum Operation {
    Insert {
        entity_type: String,
        entity_id: Uuid,
        attributes: serde_json::Value,
    },
    Update {
        entity_type: String,
        entity_id: Uuid,
        attributes: serde_json::Value,
    },
    Delete {
        entity_type: String,
        entity_id: Uuid,
    },
    Link {
        from_type: String,
        from_id: Uuid,
        relation: String,
        to_type: String,
        to_id: Uuid,
    },
}

pub struct TxBuilder {
    operations: Vec<Operation>,
}

impl TxBuilder {
    pub fn new() -> Self {
        Self { operations: Vec::new() }
    }

    pub fn insert(mut self, entity_type: &str, id: Uuid, attrs: impl Serialize) -> Self {
        self.operations.push(Operation::Insert {
            entity_type: entity_type.to_string(),
            entity_id: id,
            attributes: serde_json::to_value(attrs).unwrap(),
        });
        self
    }

    pub fn update(mut self, entity_type: &str, id: Uuid, attrs: impl Serialize) -> Self {
        self.operations.push(Operation::Update {
            entity_type: entity_type.to_string(),
            entity_id: id,
            attributes: serde_json::to_value(attrs).unwrap(),
        });
        self
    }

    pub fn link(mut self, from: EntityRef, relation: &str, to: EntityRef) -> Self {
        self.operations.push(Operation::Link {
            from_type: from.entity_type,
            from_id: from.id,
            relation: relation.to_string(),
            to_type: to.entity_type,
            to_id: to.id,
        });
        self
    }

    pub fn build(self) -> Transaction {
        Transaction {
            id: Uuid::now_v7(),
            operations: self.operations,
            optimistic: true,
        }
    }
}

#[derive(Clone)]
pub struct EntityRef {
    pub entity_type: String,
    pub id: Uuid,
}

// Usage example:
// let tx = TxBuilder::new()
//     .insert("posts", post_id, Post { title: "Hello", content: "World" })
//     .link(EntityRef { entity_type: "posts".to_string(), id: post_id }, "author", user_ref)
//     .build();
```

### Query Cache Implementation

```rust
// crates/instant-client/src/cache.rs
use dashmap::DashMap;
use std::collections::HashSet;
use instant_query::{Query, QueryResult};
use uuid::Uuid;

pub type QueryId = Uuid;

pub struct QueryCache {
    cache: DashMap<QueryId, CachedQuery>,
    /// Map from triple key to query IDs that depend on it
    indexes: DashMap<String, HashSet<QueryId>>,
}

pub struct CachedQuery {
    pub query: Query,
    pub result: QueryResult,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_synced: Option<chrono::DateTime<chrono::Utc>>,
}

impl CachedQuery {
    pub fn is_fresh(&self) -> bool {
        self.last_synced
            .map(|t| chrono::Utc::now() - t < chrono::Duration::seconds(5))
            .unwrap_or(false)
    }
}

impl QueryCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            indexes: DashMap::new(),
        }
    }

    pub fn register(&self, query_id: QueryId, query: Query, result: QueryResult) {
        let cached = CachedQuery {
            query: query.clone(),
            result: result.clone(),
            created_at: chrono::Utc::now(),
            last_synced: Some(chrono::Utc::now()),
        };
        self.cache.insert(query_id, cached);

        // Index triple dependencies
        let triple_keys = self.extract_triple_keys(&query);
        for key in triple_keys {
            self.indexes
                .entry(key)
                .or_insert_with(HashSet::new)
                .insert(query_id);
        }
    }

    pub fn invalidate(&self, changed_triples: &[Triple]) -> HashSet<QueryId> {
        let mut affected_ids = HashSet::new();

        for triple in changed_triples {
            let key = self.triple_key(triple);
            if let Some(query_ids) = self.indexes.get(&key) {
                for id in query_ids.iter() {
                    affected_ids.insert(*id);
                }
            }
        }

        affected_ids
    }

    fn extract_triple_keys(&self, query: &Query) -> Vec<String> {
        // Extract which entity types and attributes this query touches
        // Used for invalidation tracking
        query.entity_types()
            .iter()
            .map(|t| format!("entity:{}", t))
            .collect()
    }

    fn triple_key(&self, triple: &Triple) -> String {
        format!("entity:{}", triple.entity_type)
    }
}
```

## Crate 2: instant-sync

### Purpose
WebSocket-based sync protocol with message handling, presence management, and real-time broadcasting.

### Cargo.toml

```toml
[package]
name = "instant-sync"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true }
tokio-tungstenite = "0.21"
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
uuid = { workspace = true }
futures-util = "0.3"
```

### Sync Engine

```rust
// crates/instant-sync/src/engine.rs
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct SyncEngine {
    ws_url: String,
    query_subscriptions: Arc<DashMap<QueryId, broadcast::Sender<QueryResult>>>,
    presence: Arc<PresenceManager>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "subscribe-query")]
    SubscribeQuery { payload: SubscribeQueryPayload },
    #[serde(rename = "transact")]
    Transact { payload: TransactPayload },
    #[serde(rename = "presence")]
    Presence { payload: PresenceUpdate },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "query-update")]
    QueryUpdate { payload: QueryUpdatePayload },
    #[serde(rename = "transact-result")]
    TransactResult { payload: TransactResultPayload },
    #[serde(rename = "presence-update")]
    PresenceUpdate { payload: PresenceUpdatePayload },
}

impl SyncEngine {
    pub async fn connect(ws_url: impl Into<String>) -> Result<Self> {
        let ws_url = ws_url.into();
        // WebSocket connection handled internally
        Ok(Self {
            ws_url,
            query_subscriptions: Arc::new(DashMap::new()),
            presence: Arc::new(PresenceManager::new()),
        })
    }

    pub async fn register_query(&self, query: Query) -> Result<()> {
        let (tx, _rx) = broadcast::channel(100);
        self.query_subscriptions.insert(query.id(), tx);

        let msg = ClientMessage::SubscribeQuery {
            payload: SubscribeQueryPayload {
                query: query.clone(),
                cid: query.id().to_string(),
            },
        };

        self.send(msg).await
    }

    pub fn subscribe<F>(&self, query: Query, callback: F) -> Subscription
    where
        F: Fn(QueryResult) + Send + Sync + 'static,
    {
        let query_id = query.id();
        let tx = self.query_subscriptions
            .entry(query_id)
            .or_insert_with(|| broadcast::channel(100).0);
        let mut rx = tx.subscribe();

        // Spawn task to receive updates
        tokio::spawn(async move {
            while let Ok(result) = rx.recv().await {
                callback(result);
            }
        });

        Subscription { query_id }
    }

    async fn send(&self, msg: ClientMessage) -> Result<()> {
        // Send over WebSocket
        Ok(())
    }
}
```

### Presence Manager

```rust
// crates/instant-sync/src/presence.rs
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct PresenceManager {
    rooms: DashMap<String, DashMap<Uuid, PresenceData>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresenceData {
    pub user_id: Uuid,
    pub room_id: Option<String>,
    pub typing: bool,
    pub cursor: Option<CursorPosition>,
    pub metadata: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: f64,
    pub y: f64,
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
        }
    }

    pub fn set_presence(&self, user_id: Uuid, data: PresenceData) {
        if let Some(room_id) = &data.room_id {
            let room = self.rooms.entry(room_id.clone()).or_insert_with(DashMap::new);
            room.insert(user_id, data);
        }
    }

    pub fn get_peers(&self, room_id: &str) -> Vec<PresenceData> {
        self.rooms
            .get(room_id)
            .map(|room| room.iter().map(|e| e.value().clone()).collect())
            .unwrap_or_default()
    }

    pub fn broadcast_typing(&self, room_id: &str, user_id: Uuid, typing: bool) {
        if let Some(room) = self.rooms.get(room_id) {
            room.entry(user_id).and_modify(|p| p.typing = typing);
        }
    }
}
```

## Crate 3: instant-storage

### Purpose
Local persistence layer using SQLite (via sqlx) with triple store schema.

### Cargo.toml

```toml
[package]
name = "instant-storage"
version = "0.1.0"
edition = "2021"

[dependencies]
sqlx = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
tokio = { workspace = true }
```

### Triple Store Schema

```rust
// crates/instant-storage/src/schema.rs
use sqlx::postgres::PgPool;
use sqlx::sqlite::SqlitePool;

pub async fn run_migrations(pool: &SqlitePool) -> sqlx::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await
}

// migrations/1_triples.sql
/*
CREATE TABLE IF NOT EXISTS triples (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    attribute TEXT NOT NULL,
    value JSON NOT NULL,
    value_type TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    is_deleted INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_entity ON triples(entity_id);
CREATE INDEX IF NOT EXISTS idx_entity_type ON triples(entity_type);
CREATE INDEX IF NOT EXISTS idx_attribute ON triples(attribute);
CREATE INDEX IF NOT EXISTS idx_entity_attr ON triples(entity_id, attribute);
*/
```

### Storage Implementation

```rust
// crates/instant-storage/src/store.rs
use sqlx::sqlite::SqlitePool;
use serde_json::Value;
use uuid::Uuid;

pub struct LocalStore {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct Triple {
    pub entity_id: String,
    pub entity_type: String,
    pub attribute: String,
    pub value: Value,
    pub value_type: String,
}

impl LocalStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn query(&self, query: &Query) -> Result<QueryResult> {
        // Execute query against local triples
        let triples = self.fetch_triples(&query.filters).await?;
        Ok(self.assemble_result(triples, query))
    }

    async fn fetch_triples(&self, filters: &QueryFilters) -> Result<Vec<Triple>> {
        let sql = match filters {
            QueryFilters::All => "SELECT * FROM triples WHERE is_deleted = 0".to_string(),
            QueryFilters::ByType(entity_type) => {
                "SELECT * FROM triples WHERE entity_type = ? AND is_deleted = 0".to_string()
            }
            QueryFilters::Where { entity_type, attribute, value } => {
                "SELECT * FROM triples WHERE entity_type = ? AND attribute = ? AND value = ? AND is_deleted = 0".to_string()
            }
        };

        let triples = sqlx::query_as::<_, Triple>(&sql)
            .fetch_all(&self.pool)
            .await?;

        Ok(triples)
    }

    pub async fn apply_transaction(&self, tx: &Transaction) -> Result<()> {
        let mut tx_handle = self.pool.begin().await?;

        for op in &tx.operations {
            match op {
                Operation::Insert { entity_type, entity_id, attributes } => {
                    self.insert_triple(&mut tx_handle, entity_type, entity_id, attributes).await?;
                }
                Operation::Update { entity_type, entity_id, attributes } => {
                    self.update_triples(&mut tx_handle, entity_type, entity_id, attributes).await?;
                }
                Operation::Delete { entity_type, entity_id } => {
                    self.delete_triples(&mut tx_handle, entity_type, entity_id).await?;
                }
                Operation::Link { from_type, from_id, relation, to_type, to_id } => {
                    self.create_link(&mut tx_handle, from_type, from_id, relation, to_type, to_id).await?;
                }
            }
        }

        tx_handle.commit().await?;
        Ok(())
    }
}
```

## Crate 4: instant-query

### Purpose
Query engine with Datalog-like capabilities and InstaQL-style query language.

### Cargo.toml

```toml
[package]
name = "instant-query"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
petgraph = "0.6"  # For query planning
```

### Query Language

```rust
// crates/instant-query/src/lib.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Query {
    pub id: QueryId,
    pub selections: Vec<EntitySelection>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitySelection {
    pub entity_type: String,
    pub attributes: Vec<String>,
    pub relations: Vec<RelationSelection>,
    pub filters: Option<Filter>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationSelection {
    pub name: String,
    pub selection: Box<EntitySelection>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Filter {
    Eq { field: String, value: serde_json::Value },
    Ne { field: String, value: serde_json::Value },
    Gt { field: String, value: serde_json::Value },
    Lt { field: String, value: serde_json::Value },
    In { field: String, values: Vec<serde_json::Value> },
    And { filters: Vec<Filter> },
    Or { filters: Vec<Filter> },
}

pub type QueryId = Uuid;
pub type QueryResult = serde_json::Value;
```

## Crate 5: instant-perms

### Purpose
CEL-like permission rule evaluation for access control.

### Cargo.toml

```toml
[package]
name = "instant-perms"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
rhai = "1"  # Embedded scripting language for rules
```

### Permission Evaluator

```rust
// crates/instant-perms/src/lib.rs
use rhai::{Engine, Scope};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub struct PermissionEngine {
    rhai_engine: Engine,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PermissionRules {
    pub entities: HashMap<String, EntityRules>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EntityRules {
    pub bind: String,  // Rhai expression
    pub allow: ActionRules,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActionRules {
    pub view: String,
    pub create: String,
    pub update: String,
    pub delete: String,
}

pub struct PermissionContext {
    pub auth: Option<AuthContext>,
    pub data: serde_json::Value,
    pub action: String,
}

pub struct AuthContext {
    pub user_id: Uuid,
    pub roles: Vec<String>,
}

impl PermissionEngine {
    pub fn new() -> Self {
        Self {
            rhai_engine: Engine::new(),
        }
    }

    pub fn check(&self, rules: &EntityRules, ctx: &PermissionContext) -> Result<bool> {
        let mut scope = Scope::new();

        // Bind data variable
        if let Some(bind) = &rules.bind {
            // Evaluate bind expression
        }

        // Add auth context
        if let Some(auth) = &ctx.auth {
            scope.push("auth", auth.clone());
        }

        // Get the rule for the action
        let rule = match ctx.action.as_str() {
            "view" => &rules.allow.view,
            "create" => &rules.allow.create,
            "update" => &rules.allow.update,
            "delete" => &rules.allow.delete,
            _ => return Ok(false),
        };

        // Evaluate rule
        let result: bool = self.rhai_engine.eval_with_scope::<bool>(&mut scope, rule)?;
        Ok(result)
    }
}

// Usage:
// let rules = EntityRules {
//     bind: "post == data".to_string(),
//     allow: ActionRules {
//         view: "true".to_string(),
//         create: "auth.is_some()".to_string(),
//         update: "auth.map_or(false, |a| a.user_id == post.author_id)".to_string(),
//         delete: "auth.map_or(false, |a| a.user_id == post.author_id)".to_string(),
//     }
// };
```

## Crate 6: instant-crdt

### Purpose
CRDT types for conflict-free replication of shared state.

### Cargo.toml

```toml
[package]
name = "instant-crdt"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
ordcrdt = "0.6"  # Or implement custom CRDTs
```

### LWW Register CRDT

```rust
// crates/instant-crdt/src/lww.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LwwRegister<T> {
    value: Option<T>,
    timestamp: DateTime<Utc>,
    replica_id: u64,
}

impl<T: Clone + Serialize> LwwRegister<T> {
    pub fn new(replica_id: u64) -> Self {
        Self {
            value: None,
            timestamp: Utc::now(),
            replica_id,
        }
    }

    pub fn set(&mut self, value: T, timestamp: DateTime<Utc>) {
        if timestamp > self.timestamp {
            self.value = Some(value);
            self.timestamp = timestamp;
        }
    }

    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        } else if other.timestamp == self.timestamp && other.replica_id > self.replica_id {
            self.value = other.value.clone();
        }
    }
}
```

## Performance Considerations

### Concurrency

```rust
// Use DashMap for concurrent access
use dashmap::DashMap;

pub struct ConcurrentCache<K, V> {
    map: DashMap<K, V>,
}

// Use tokio streams for async operations
use tokio_stream::StreamExt;
```

### Memory Management

```rust
// Use Arc for shared state
use std::sync::Arc;

pub struct SharedState {
    data: Arc<DashMap<String, Value>>,
}

// Clone is cheap (just increment refcount)
impl Clone for SharedState {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}
```

## Testing Strategy

```rust
// tests/integration/sync_test.rs
#[cfg(test)]
mod tests {
    use instant_client::InstantDb;
    use instant_storage::LocalStore;

    #[tokio::test]
    async fn test_optimistic_transaction() {
        let db = InstantDb::builder("test-app").build().await.unwrap();

        let tx = TxBuilder::new()
            .insert("users", user_id, User { name: "Alice" })
            .build();

        let result = db.transact(tx).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_query_cache_invalidation() {
        // Test that cache is invalidated when triples change
    }
}
```

## Summary

This Rust revision provides:
- **Idiomatic Rust APIs** with builder patterns and strong typing
- **Async-first design** using tokio
- **Concurrent data structures** (DashMap, Arc) for thread-safe access
- **SQLite storage** with triple store schema
- **CEL-like permissions** using Rhai scripting
- **CRDT foundation** for conflict resolution
- **WebSocket sync** with tokio-tungstenite
