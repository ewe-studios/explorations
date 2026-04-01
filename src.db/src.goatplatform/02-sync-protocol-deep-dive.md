---
title: "GoatPlatform Sync Protocol Deep Dive"
subtitle: "sqlsync protocol, change propagation, and conflict resolution"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.goatplatform
related: 01-storage-engine-deep-dive.md
---

# 02 - Sync Protocol Deep Dive: GoatPlatform

## Overview

This document covers the sqlsync protocol - how changes are propagated between clients and servers, conflict resolution strategies, and achieving eventual consistency.

## Part 1: Sync Protocol Architecture

### Protocol Message Format

```
Sync Message Structure:

┌─────────────────────────────────────────────────────────┐
│ SyncRequest (Client → Server)                           │
├─────────────────────────────────────────────────────────┤
│ {                                                       │
│   "type": "sync_request",                               │
│   "client_clock": {                                     │
│     "client-a": 5,                                      │
│     "client-b": 3,                                      │
│     "server": 10                                        │
│   },                                                    │
│   "tables": ["users", "posts", "comments"],             │
│   "upload_changes": [                                   │
│     {                                                   │
│       "table": "posts",                                 │
│       "operation": "INSERT",                            │
│       "row_id": "post-123",                             │
│       "data": {                                         │
│         "title": "Hello World",                         │
│         "content": "...",                               │
│         "author_id": "user-456"                         │
│       },                                                │
│       "clock": {"client-a": 5}                          │
│     }                                                   │
│   ]                                                     │
│ }                                                       │
└───────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ SyncResponse (Server → Client)                          │
├─────────────────────────────────────────────────────────┤
│ {                                                       │
│   "type": "sync_response",                              │
│   "server_clock": {                                     │
│     "client-a": 5,                                      │
│     "client-b": 3,                                      │
│     "server": 12                                        │
│   },                                                    │
│   "accepted_changes": ["post-123"],                     │
│   "rejected_changes": [],                               │
│   "download_changes": [                                 │
│     {                                                   │
│       "table": "comments",                              │
│       "operation": "INSERT",                            │
│       "row_id": "comment-789",                          │
│       "data": {                                         │
│         "post_id": "post-123",                          │
│         "text": "Great post!",                          │
│         "author_id": "user-789"                         │
│       },                                                │
│       "clock": {"client-b": 3, "server": 11}            │
│     }                                                   │
│   ]                                                     │
│ }                                                       │
└───────────────────────────────────────────────────────────┘
```

### Sync State Machine

```
Client Sync State Machine:

┌─────────────────────────────────────────────────────────┐
│                                                         │
│    ┌──────────┐                                         │
│    │ IDLE     │                                         │
│    └────┬─────┘                                         │
│         │ Local write                                   │
│         ▼                                                 │
│    ┌──────────┐                                         │
│    │ PENDING  │───┐                                     │
│    └────┬─────┘   │                                     │
│         │         │ Batch timeout or threshold           │
│         ▼         ▼                                     │
│    ┌──────────┐                                         │
│    │ SYNCING  │                                         │
│    └────┬─────┘                                         │
│         │                                                 │
│    ┌────┴─────┐                                         │
│    │          │                                         │
│    ▼          ▼                                         │
│ ┌──────┐  ┌──────┐                                     │
│ │OK    │  │ERROR │─────┐                               │
│ └──────┘  └──┬───┘     │                                 │
│              │         │ Retry with backoff              │
│              └─────────┘                                 │
│                                                         │
└───────────────────────────────────────────────────────────┘

States:
- IDLE: No pending changes
- PENDING: Changes queued, waiting to sync
- SYNCING: Request in flight
- OK: Last sync successful
- ERROR: Last sync failed, will retry
```

## Part 2: Conflict Detection and Resolution

### Conflict Detection

```
Detecting Concurrent Changes:

┌─────────────────────────────────────────────────────────┐
│ Scenario: Two clients edit same row concurrently        │
│                                                         │
│ Timeline:                                               │
│                                                         │
│ Client-A: Edit post title to "Hello" (clock: {A: 5})   │
│      │                                                  │
│      │ (no sync yet)                                   │
│      │                                                  │
│ Client-B: Edit post content to "World" (clock: {B: 3}) │
│      │                                                  │
│      │                                                  │
│ Server: Receives both changes                           │
│                                                         │
│ Analysis:                                               │
│ - Change A clock: {A: 5, Server: 10}                   │
│ - Change B clock: {B: 3, Server: 10}                   │
│ - Clock comparison: A || B (concurrent)                │
│                                                         │
│ Result: CONFLICT DETECTED                              │
└───────────────────────────────────────────────────────────┘

Conflict Detection Logic:

```rust
#[derive(Debug, Clone)]
pub struct Change {
    pub table: String,
    pub row_id: String,
    pub operation: Operation,
    pub data: serde_json::Value,
    pub clock: VectorClock,
}

pub fn detect_conflict(
    existing: &Change,
    incoming: &Change,
) -> ConflictType {
    // Same row, same table
    if existing.table != incoming.table || existing.row_id != incoming.row_id {
        return ConflictType::None;
    }

    // Compare clocks
    match existing.clock.compare(&incoming.clock) {
        ClockOrdering::Before => {
            // Existing happened first, incoming is newer
            ConflictType::None
        }
        ClockOrdering::After => {
            // Existing is newer, incoming is stale
            ConflictType::Stale
        }
        ClockOrdering::Equal => {
            // Same change (idempotent)
            ConflictType::Duplicate
        }
        ClockOrdering::Concurrent => {
            // Both changed independently = conflict
            ConflictType::Concurrent
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConflictType {
    None,       // No conflict, can apply
    Stale,      // Incoming is stale, reject
    Duplicate,  // Same change, ignore
    Concurrent, // Real conflict, needs resolution
}
```
```

### Conflict Resolution Strategies

```
Strategy 1: Last-Writer-Wins (LWW)

```rust
pub struct LwwResolver {
    timestamp_field: String,
}

impl ConflictResolver for LwwResolver {
    fn resolve(
        &self,
        existing: &Change,
        incoming: &Change,
    ) -> ResolutionResult {
        // Extract timestamps
        let existing_ts = existing
            .data
            .get(&self.timestamp_field)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let incoming_ts = incoming
            .data
            .get(&self.timestamp_field)
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if incoming_ts >= existing_ts {
            ResolutionResult::AcceptIncoming
        } else {
            ResolutionResult::KeepExisting
        }
    }
}

// Usage:
let resolver = LwwResolver {
    timestamp_field: "updated_at".to_string(),
};
```

Strategy 2: Field-Level Merge

```rust
pub struct FieldLevelMerger;

impl ConflictResolver for FieldLevelMerger {
    fn resolve(
        &self,
        existing: &Change,
        incoming: &Change,
    ) -> ResolutionResult {
        match (&existing.data, &incoming.data) {
            (Value::Object(e), Value::Object(i)) => {
                let mut merged = e.clone();

                for (key, incoming_val) in i {
                    match merged.get(key) {
                        Some(existing_val) => {
                            // Both modified same field
                            // Use incoming (or could use LWW)
                            merged.insert(key.clone(), incoming_val.clone());
                        }
                        None => {
                            // Only incoming has this field
                            merged.insert(key.clone(), incoming_val.clone());
                        }
                    }
                }

                ResolutionResult::Merge(merged)
            }
            _ => ResolutionResult::AcceptIncoming,
        }
    }
}
```

Strategy 3: Custom Resolver per Table

```rust
pub type ResolverFn = Box<dyn Fn(&Change, &Change) -> ResolutionResult + Send + Sync>;

pub struct TableResolverRegistry {
    resolvers: HashMap<String, ResolverFn>,
    default: Option<ResolverFn>,
}

impl TableResolverRegistry {
    pub fn register<F>(&mut self, table: &str, resolver: F)
    where
        F: Fn(&Change, &Change) -> ResolutionResult + Send + Sync + 'static,
    {
        self.resolvers
            .insert(table.to_string(), Box::new(resolver));
    }

    pub fn resolve(&self, table: &str, existing: &Change, incoming: &Change) -> ResolutionResult {
        if let Some(resolver) = self.resolvers.get(table) {
            resolver(existing, incoming)
        } else if let Some(default) = &self.default {
            default(existing, incoming)
        } else {
            // Default: Last-Writer-Wins by clock
            match existing.clock.compare(&incoming.clock) {
                ClockOrdering::Concurrent => {
                    // For concurrent, prefer incoming (arbitrary choice)
                    ResolutionResult::AcceptIncoming
                }
                _ => ResolutionResult::KeepExisting,
            }
        }
    }
}

// Example: Custom resolver for posts table
registry.register("posts", |existing, incoming| {
    // For posts, merge title and content separately
    // Title: LWW by character count (longer wins)
    // Content: Always keep incoming

    let existing_title = existing.data.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let incoming_title = incoming.data.get("title").and_then(|v| v.as_str()).unwrap_or("");

    if incoming_title.len() >= existing_title.len() {
        ResolutionResult::AcceptIncoming
    } else {
        ResolutionResult::Merge(serde_json::json!({
            "title": existing_title,
            "content": incoming.data.get("content").cloned(),
        }))
    }
});
```

Strategy 4: CRDT-Based Merge

```rust
use crdts::{CmRDT, GCounter, MVRegister};

pub struct CrdtState {
    title: MVRegister<String>,
    content: MVRegister<String>,
    view_count: GCounter<String>,  // Distributed counter
}

impl CrdtState {
    pub fn merge(&mut self, other: &Self) {
        self.title.merge(&other.title);
        self.content.merge(&other.content);
        self.view_count.merge(&other.view_count);
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "title": self.title.read().first().cloned().unwrap_or_default(),
            "content": self.content.read().first().cloned().unwrap_or_default(),
            "view_count": self.view_count.read(),
        })
    }
}

// CRDT operations are commutative, associative, idempotent
// Multiple replicas always converge to same state
```
```

## Part 3: Sync Server Implementation

### WebSocket Sync Server

```rust
use tokio::sync::broadcast;
use warp::ws::{WebSocket, Message, Ws};

pub struct SyncServer {
    change_tx: broadcast::Sender<Change>,
    state: Arc<RwLock<ServerState>>,
}

struct ServerState {
    data: HashMap<String, serde_json::Value>,
    clocks: HashMap<String, VectorClock>,
    client_clocks: HashMap<ClientId, VectorClock>,
}

impl SyncServer {
    pub fn new() -> Self {
        let (change_tx, _) = broadcast::channel(1000);
        Self {
            change_tx,
            state: Arc::new(RwLock::new(ServerState {
                data: HashMap::new(),
                clocks: HashMap::new(),
                client_clocks: HashMap::new(),
            })),
        }
    }

    pub async fn handle_ws(&self, ws: Ws, client_id: ClientId) {
        let (tx, rx) = ws.split();
        let change_rx = self.change_tx.subscribe();

        // Spawn receive task (client → server)
        let state = self.state.clone();
        let tx_clone = self.change_tx.clone();
        let recv_task = tokio::spawn(async move {
            Self::handle_receive(rx, state, tx_clone, client_id).await
        });

        // Spawn send task (server → client)
        let send_task = tokio::spawn(async move {
            Self::handle_send(tx, change_rx).await
        });

        // Wait for either task to complete
        tokio::select! {
            _ = recv_task => {}
            _ = send_task => {}
        }
    }

    async fn handle_receive(
        mut rx: SplitSink<WebSocket, Message>,
        state: Arc<RwLock<ServerState>>,
        change_tx: broadcast::Sender<Change>,
        client_id: ClientId,
    ) {
        while let Some(Ok(msg)) = rx.next().await {
            if let Ok(text) = msg.to_str() {
                if let Ok(request) = serde_json::from_str::<SyncRequest>(text) {
                    // Process upload changes
                    for change in request.upload_changes {
                        let resolved = Self::resolve_change(
                            &state,
                            &change,
                            client_id,
                        ).await;

                        // Broadcast to all clients
                        let _ = change_tx.send(resolved);
                    }

                    // Send response
                    let response = Self::build_response(&state, client_id).await;
                    // Send response back...
                }
            }
        }
    }

    async fn handle_send(
        mut tx: SplitStream<WebSocket>,
        mut change_rx: broadcast::Receiver<Change>,
    ) {
        while let Ok(change) = change_rx.recv().await {
            let msg = serde_json::to_string(&SyncMessage::Change(change)).unwrap();
            let _ = tx.send(Message::text(msg)).await;
        }
    }

    async fn resolve_change(
        state: &Arc<RwLock<ServerState>>,
        incoming: &Change,
        client_id: ClientId,
    ) -> Change {
        let mut state = state.write().await;

        // Get existing change for same row
        let key = format!("{}:{}", incoming.table, incoming.row_id);

        if let Some(existing_clock) = state.clocks.get(&key) {
            // Check for conflict
            match existing_clock.compare(&incoming.clock) {
                ClockOrdering::Concurrent => {
                    // Conflict! Apply resolution strategy
                    // For now, just use incoming (LWW by default)
                }
                ClockOrdering::Stale => {
                    // Incoming is stale, reject
                    return Change {
                        operation: Operation::Reject,
                        ..incoming.clone()
                    };
                }
                _ => {}
            }
        }

        // Update clock
        let mut new_clock = incoming.clock.clone();
        new_clock.tick("server");

        // Merge with existing clock
        if let Some(existing_clock) = state.clocks.get(&key) {
            new_clock.merge(existing_clock);
        }

        // Store change
        state.data.insert(key.clone(), incoming.data.clone());
        state.clocks.insert(key, new_clock.clone());

        Change {
            clock: new_clock,
            ..incoming.clone()
        }
    }
}
```

### Batch Sync Optimization

```rust
pub struct BatchSyncConfig {
    pub max_batch_size: usize,
    pub flush_interval: Duration,
    pub compression: bool,
}

impl Default for BatchSyncConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            flush_interval: Duration::from_millis(500),
            compression: true,
        }
    }
}

pub struct BatchSyncBuffer {
    buffer: Vec<Change>,
    config: BatchSyncConfig,
    timer: tokio::time::Interval,
}

impl BatchSyncBuffer {
    pub fn new(config: BatchSyncConfig) -> Self {
        Self {
            buffer: Vec::with_capacity(config.max_batch_size),
            config,
            timer: tokio::time::interval(config.flush_interval),
        }
    }

    pub fn add(&mut self, change: Change) -> Option<Vec<Change>> {
        self.buffer.push(change);

        // Check if batch is full
        if self.buffer.len() >= self.config.max_batch_size {
            let batch = std::mem::replace(
                &mut self.buffer,
                Vec::with_capacity(self.config.max_batch_size),
            );
            Some(batch)
        } else {
            None
        }
    }

    pub async fn tick(&mut self) -> Option<Vec<Change>> {
        self.timer.tick().await;

        if !self.buffer.is_empty() {
            let batch = std::mem::replace(
                &mut self.buffer,
                Vec::with_capacity(self.config.max_batch_size),
            );
            Some(batch)
        } else {
            None
        }
    }

    pub fn compress(batch: Vec<Change>) -> Vec<u8> {
        let json = serde_json::to_vec(&batch).unwrap();

        if batch.len() > 10 {
            // Compress large batches
            use flate2::write::GzEncoder;
            use flate2::Compression;

            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&json).unwrap();
            encoder.finish().unwrap()
        } else {
            json
        }
    }
}
```

## Part 4: Offline Support

### Local Queue for Offline Changes

```rust
pub struct OfflineQueue {
    db: sled::Db,
    pending_key: &'static str,
}

impl OfflineQueue {
    pub fn new(db: sled::Db) -> Self {
        Self {
            db,
            pending_key: "pending_changes",
        }
    }

    pub fn enqueue(&self, change: Change) -> Result<(), sled::Error> {
        let mut pending = self.get_pending()?;
        pending.push(change);
        self.save_pending(pending)
    }

    pub fn dequeue(&self, count: usize) -> Result<Vec<Change>, sled::Error> {
        let mut pending = self.get_pending()?;
        let batch: Vec<Change> = pending.drain(..count.min(pending.len())).collect();
        self.save_pending(pending)?;
        Ok(batch)
    }

    pub fn acknowledge(&self, change_ids: Vec<String>) -> Result<(), sled::Error> {
        let mut pending = self.get_pending()?;
        pending.retain(|c| !change_ids.contains(&c.row_id));
        self.save_pending(pending)
    }

    fn get_pending(&self) -> Result<Vec<Change>, sled::Error> {
        match self.db.get(self.pending_key)? {
            Some(data) => Ok(serde_json::from_slice(&data).unwrap_or_default()),
            None => Ok(Vec::new()),
        }
    }

    fn save_pending(&self, pending: Vec<Change>) -> Result<(), sled::Error> {
        let data = serde_json::to_vec(&pending).unwrap();
        self.db.insert(self.pending_key, data)?;
        self.db.flush()?;
        Ok(())
    }
}

// Usage in client
pub struct SyncClient {
    offline_queue: OfflineQueue,
    online: AtomicBool,
}

impl SyncClient {
    pub fn on_local_change(&self, change: Change) {
        if self.online.load(Ordering::Relaxed) {
            // Send immediately
            self.send_change(change);
        } else {
            // Queue for later
            self.offline_queue.enqueue(change).unwrap();
        }
    }

    pub fn on_reconnect(&self) {
        self.online.store(true, Ordering::Relaxed);

        // Flush pending changes
        let batch = self.offline_queue.dequeue(100).unwrap();
        for change in batch {
            self.send_change(change);
        }
    }
}
```

---

*This document is part of the GoatPlatform exploration series. See [exploration.md](./exploration.md) for the complete index.*
