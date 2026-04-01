---
title: "GoatPlatform Rust Revision"
subtitle: "Building goatdb-like real-time sync database in Rust"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.goatplatform
related: 02-sync-protocol-deep-dive.md
---

# Rust Revision: GoatPlatform

## Overview

This document covers implementing a goatdb-like real-time sync database in Rust - storage engine, CRDT data types, and sync protocol implementation.

## Part 1: Core Database Structure

### Database Engine

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use sled::Db;

pub struct GoatDb {
    storage: Arc<Db>,
    indexes: RwLock<HashMap<String, Index>>,
    clock: RwLock<VectorClock>,
    node_id: String,
    config: GoatDbConfig,
}

#[derive(Debug, Clone)]
pub struct GoatDbConfig {
    pub path: String,
    pub sync_mode: SyncMode,
    pub cache_size: usize,
    pub node_id: String,
}

impl GoatDb {
    pub async fn open(config: GoatDbConfig) -> Result<Self, DbError> {
        let storage = sled::Config::new()
            .path(&config.path)
            .cache_capacity(config.cache_size as u64)
            .mode(sled::Mode::HighThroughput)
            .open()?;

        let clock = RwLock::new(VectorClock::new());

        Ok(Self {
            storage: Arc::new(storage),
            indexes: RwLock::new(HashMap::new()),
            clock,
            node_id: config.node_id.clone(),
            config,
        })
    }

    pub async fn insert(&self, table: &str, row: Row) -> Result<InsertResult, DbError> {
        // Increment clock
        let mut clock = self.clock.write().await;
        clock.tick(&self.node_id);

        // Create change record
        let change = Change {
            table: table.to_string(),
            row_id: row.id.clone(),
            operation: Operation::Insert,
            data: row.data.clone(),
            clock: clock.clone(),
        };

        // Write to storage
        let key = self.row_key(table, &row.id);
        let value = serde_json::to_vec(&RowWithMeta {
            row,
            clock: clock.clone(),
        })?;

        self.storage.insert(&key, value)?;
        self.storage.flush()?;

        // Write to WAL
        self.write_wal(&change).await?;

        Ok(InsertResult {
            row_id: change.row_id,
            clock: change.clock,
        })
    }

    pub async fn get(&self, table: &str, row_id: &str) -> Result<Option<Row>, DbError> {
        let key = self.row_key(table, row_id);

        match self.storage.get(&key)? {
            Some(data) => {
                let row_meta: RowWithMeta = serde_json::from_slice(&data)?;
                Ok(Some(row_meta.row))
            }
            None => Ok(None),
        }
    }

    pub async fn update(&self, table: &str, row_id: &str, data: serde_json::Value) -> Result<UpdateResult, DbError> {
        let key = self.row_key(table, row_id);

        // Get existing row
        let existing = match self.storage.get(&key)? {
            Some(data) => {
                let row_meta: RowWithMeta = serde_json::from_slice(&data)?;
                row_meta
            }
            None => return Err(DbError::NotFound),
        };

        // Increment clock
        let mut clock = self.clock.write().await;
        clock.tick(&self.node_id);

        // Merge data
        let merged_data = Self::merge_json(existing.row.data, data);

        let new_row = Row {
            id: row_id.to_string(),
            data: merged_data,
        };

        // Write updated row
        let value = serde_json::to_vec(&RowWithMeta {
            row: new_row.clone(),
            clock: clock.clone(),
        })?;

        self.storage.insert(&key, value)?;
        self.storage.flush()?;

        // Write to WAL
        let change = Change {
            table: table.to_string(),
            row_id: row_id.to_string(),
            operation: Operation::Update,
            data: new_row.data.clone(),
            clock: clock.clone(),
        };
        self.write_wal(&change).await?;

        Ok(UpdateResult {
            clock: clock.clone(),
        })
    }

    pub async fn delete(&self, table: &str, row_id: &str) -> Result<DeleteResult, DbError> {
        let key = self.row_key(table, row_id);

        // Increment clock
        let mut clock = self.clock.write().await;
        clock.tick(&self.node_id);

        // Remove from storage
        self.storage.remove(&key)?;
        self.storage.flush()?;

        // Write to WAL
        let change = Change {
            table: table.to_string(),
            row_id: row_id.to_string(),
            operation: Operation::Delete,
            data: serde_json::Value::Null,
            clock: clock.clone(),
        };
        self.write_wal(&change).await?;

        Ok(DeleteResult {
            clock: clock.clone(),
        })
    }

    fn row_key(&self, table: &str, row_id: &str) -> Vec<u8> {
        format!("row:{}:{}", table, row_id).into_bytes()
    }

    fn merge_json(base: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
        match (base, patch) {
            (serde_json::Value::Object(mut base_map), serde_json::Value::Object(patch_map)) => {
                for (key, value) in patch_map {
                    base_map.insert(key, value);
                }
                serde_json::Value::Object(base_map)
            }
            (_, patch) => patch,
        }
    }

    async fn write_wal(&self, change: &Change) -> Result<(), DbError> {
        // WAL implementation from storage engine deep dive
        todo!("Write to WAL")
    }
}

#[derive(Debug, Clone)]
pub struct Row {
    pub id: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
struct RowWithMeta {
    row: Row,
    clock: VectorClock,
}

#[derive(Debug, Clone)]
pub struct InsertResult {
    pub row_id: String,
    pub clock: VectorClock,
}

#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub clock: VectorClock,
}

#[derive(Debug, Clone)]
pub struct DeleteResult {
    pub clock: VectorClock,
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Sled error: {0}")]
    Sled(#[from] sled::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Row not found")]
    NotFound,
}
```

## Part 2: CRDT Data Types

### CRDT Counter

```rust
use std::collections::HashMap;

/// G-Counter (Grow-only Counter)
#[derive(Debug, Clone)]
pub struct GCounter {
    counts: HashMap<String, u64>,
}

impl GCounter {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn increment(&mut self, node_id: &str, amount: u64) {
        *self.counts.entry(node_id.to_string()).or_insert(0) += amount;
    }

    pub fn value(&self) -> u64 {
        self.counts.values().sum()
    }

    pub fn merge(&mut self, other: &GCounter) {
        for (node_id, count) in &other.counts {
            let entry = self.counts.entry(node_id.clone()).or_insert(0);
            *entry = (*entry).max(*count);
        }
    }
}

/// PN-Counter (Positive-Negative Counter)
#[derive(Debug, Clone)]
pub struct PNCounter {
    positive: GCounter,
    negative: GCounter,
}

impl PNCounter {
    pub fn new() -> Self {
        Self {
            positive: GCounter::new(),
            negative: GCounter::new(),
        }
    }

    pub fn increment(&mut self, node_id: &str, amount: u64) {
        self.positive.increment(node_id, amount);
    }

    pub fn decrement(&mut self, node_id: &str, amount: u64) {
        self.negative.increment(node_id, amount);
    }

    pub fn value(&self) -> i64 {
        self.positive.value() as i64 - self.negative.value() as i64
    }

    pub fn merge(&mut self, other: &PNCounter) {
        self.positive.merge(&other.positive);
        self.negative.merge(&other.negative);
    }
}
```

### CRDT Register

```rust
use std::hash::{Hash, Hasher};

/// MV-Register (Multi-Value Register)
#[derive(Debug, Clone)]
pub struct MVRegister<T> {
    values: Vec<(VectorClock, T)>,
}

impl<T: Clone + PartialEq> MVRegister<T> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
        }
    }

    pub fn set(&mut self, value: T, clock: VectorClock) {
        // Remove all values that are causally before the new clock
        self.values.retain(|(existing_clock, _)| {
            !matches!(existing_clock.compare(&clock), ClockOrdering::Before)
        });

        // Add new value
        self.values.push((clock, value));
    }

    pub fn read(&self) -> Vec<&T> {
        self.values.iter().map(|(_, v)| v).collect()
    }

    pub fn merge(&mut self, other: &MVRegister<T>) {
        // Collect values to remove
        let mut to_remove = Vec::new();

        for (i, (clock1, _)) in self.values.iter().enumerate() {
            for (clock2, _) in &other.values {
                if matches!(clock1.compare(clock2), ClockOrdering::Before) {
                    to_remove.push(i);
                    break;
                }
            }
        }

        // Remove dominated values
        for i in to_remove.into_iter().rev() {
            self.values.remove(i);
        }

        // Add concurrent values from other
        for (clock, value) in &other.values {
            let is_concurrent = self.values.iter().all(|(c, _)| {
                matches!(c.compare(clock), ClockOrdering::Concurrent)
            });

            if is_concurrent {
                self.values.push((clock.clone(), value.clone()));
            }
        }
    }
}

/// LWW-Register (Last-Writer-Wins Register)
#[derive(Debug, Clone)]
pub struct LWWRegister<T> {
    value: Option<T>,
    timestamp: u64,
}

impl<T: Clone> LWWRegister<T> {
    pub fn new() -> Self {
        Self {
            value: None,
            timestamp: 0,
        }
    }

    pub fn set(&mut self, value: T, timestamp: u64) {
        if timestamp > self.timestamp {
            self.value = Some(value);
            self.timestamp = timestamp;
        }
    }

    pub fn get(&self) -> Option<&T> {
        self.value.as_ref()
    }

    pub fn merge(&mut self, other: &LWWRegister<T>) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
        }
    }
}
```

### CRDT Set

```rust
/// OR-Set (Observed-Remove Set)
#[derive(Debug, Clone)]
pub struct ORSet<T: Eq + Hash + Clone> {
    elements: HashMap<T, Vec<u64>>,  // element -> list of unique tags
    tombstones: HashMap<T, Vec<u64>>, // removed tags
    tag_counter: u64,
}

impl<T: Eq + Hash + Clone> ORSet<T> {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            tombstones: HashMap::new(),
            tag_counter: 0,
        }
    }

    pub fn add(&mut self, element: T) -> u64 {
        // Generate unique tag
        self.tag_counter += 1;
        let tag = self.tag_counter;

        self.elements
            .entry(element)
            .or_insert_with(Vec::new)
            .push(tag);

        tag
    }

    pub fn remove(&mut self, element: &T) {
        if let Some(tags) = self.elements.get(element) {
            let tombstone = self.tombstones
                .entry(element.clone())
                .or_insert_with(Vec::new);
            tombstone.extend(tags);
        }
    }

    pub fn contains(&self, element: &T) -> bool {
        let element_tags = self.elements.get(element);
        let tombstone_tags = self.tombstones.get(element);

        match (element_tags, tombstone_tags) {
            (Some(tags), None) => !tags.is_empty(),
            (Some(tags), Some(tombs)) => {
                // Element exists if any tag is not tombstoned
                tags.iter().any(|t| !tombs.contains(t))
            }
            _ => false,
        }
    }

    pub fn merge(&mut self, other: &ORSet<T>) {
        // Merge elements
        for (element, tags) in &other.elements {
            self.elements
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .extend(tags);
        }

        // Merge tombstones
        for (element, tags) in &other.tombstones {
            self.tombstones
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .extend(tags);
        }

        // Update tag counter
        self.tag_counter = self.tag_counter.max(other.tag_counter);

        // Clean up
        self.cleanup();
    }

    fn cleanup(&mut self) {
        // Remove elements that are fully tombstoned
        let elements_to_remove: Vec<T> = self
            .elements
            .iter()
            .filter(|(elem, tags)| {
                if let Some(tombs) = self.tombstones.get(*elem) {
                    tags.iter().all(|t| toms.contains(t))
                } else {
                    false
                }
            })
            .map(|(elem, _)| elem.clone())
            .collect();

        for elem in elements_to_remove {
            self.elements.remove(&elem);
            self.tombstones.remove(&elem);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.elements.iter()
            .filter(|(elem, tags)| {
                if let Some(tombs) = self.tombstones.get(*elem) {
                    tags.iter().any(|t| !tombs.contains(t))
                } else {
                    !tags.is_empty()
                }
            })
            .map(|(elem, _)| elem)
    }
}
```

## Part 3: Sync Client

```rust
use tokio::sync::mpsc;
use tungstenite::client::async_connect;
use url::Url;

pub struct SyncClient {
    db: Arc<GoatDb>,
    config: SyncClientConfig,
    state: Arc<RwLock<SyncState>>,
}

#[derive(Debug, Clone)]
pub struct SyncClientConfig {
    pub server_url: String,
    pub node_id: String,
    pub batch_size: usize,
    pub flush_interval: Duration,
}

#[derive(Debug, Clone)]
struct SyncState {
    connected: bool,
    pending_changes: Vec<Change>,
    server_clock: VectorClock,
}

impl SyncClient {
    pub fn new(db: Arc<GoatDb>, config: SyncClientConfig) -> Self {
        Self {
            db,
            config,
            state: Arc::new(RwLock::new(SyncState {
                connected: false,
                pending_changes: Vec::new(),
                server_clock: VectorClock::new(),
            })),
        }
    }

    pub async fn start(&self) -> Result<(), SyncError> {
        // Spawn sync loop
        let state = self.state.clone();
        let db = self.db.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::sync_loop(db, config, state).await
        });

        Ok(())
    }

    async fn sync_loop(
        db: Arc<GoatDb>,
        config: SyncClientConfig,
        state: Arc<RwLock<SyncState>>,
    ) -> Result<(), SyncError> {
        loop {
            // Connect to server
            let ws_url = Url::parse(&config.server_url)?;
            let (ws_stream, _) = async_connect(ws_url).await?;

            let (mut write, mut read) = ws_stream.split();

            state.write().await.connected = true;

            // Process messages
            loop {
                tokio::select! {
                    // Receive from server
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                Self::handle_server_message(&text, &db, &state).await?;
                            }
                            Some(Ok(Message::Close(_))) => {
                                break; // Reconnect
                            }
                            _ => {}
                        }
                    }

                    // Send pending changes
                    _ = Self::flush_pending(&db, &state, &mut write) => {}
                }
            }

            // Mark disconnected
            state.write().await.connected = false;

            // Wait before reconnecting
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn handle_server_message(
        text: &str,
        db: &GoatDb,
        state: &Arc<RwLock<SyncState>>,
    ) -> Result<(), SyncError> {
        let msg: SyncMessage = serde_json::from_str(text)?;

        match msg {
            SyncMessage::Change(change) => {
                // Apply change to local database
                // Check if we already have this change
                let current_clock = db.clock.read().await;

                if change.clock.compare(&current_clock) == ClockOrdering::Concurrent
                    || change.clock.compare(&current_clock) == ClockOrdering::After
                {
                    // Apply change
                    Self::apply_remote_change(db, &change).await?;

                    // Update server clock
                    let mut state = state.write().await;
                    state.server_clock.merge(&change.clock);
                }
            }
            SyncMessage::Ack(ack) => {
                // Remove acknowledged changes from pending
                let mut state = state.write().await;
                state.pending_changes.retain(|c| {
                    !ack.acknowledged.contains(&c.row_id)
                });
            }
        }

        Ok(())
    }

    async fn apply_remote_change(
        db: &GoatDb,
        change: &Change,
    ) -> Result<(), SyncError> {
        match change.operation {
            Operation::Insert => {
                db.insert(&change.table, Row {
                    id: change.row_id.clone(),
                    data: change.data.clone(),
                }).await?;
            }
            Operation::Update => {
                db.update(&change.table, &change.row_id, change.data.clone()).await?;
            }
            Operation::Delete => {
                db.delete(&change.table, &change.row_id).await?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn flush_pending(
        db: &Arc<GoatDb>,
        state: &Arc<RwLock<SyncState>>,
        write: &mut futures_util::stream::SplitSink<_, Message>,
    ) -> Result<(), SyncError> {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let mut state = state.write().await;

        if !state.pending_changes.is_empty() {
            let batch: Vec<Change> = state.pending_changes.drain(..).collect();

            let request = SyncRequest {
                client_clock: state.server_clock.clone(),
                tables: vec![],
                upload_changes: batch,
            };

            let msg = serde_json::to_string(&request)?;
            write.send(Message::Text(msg)).await?;
        }

        Ok(())
    }

    pub async fn queue_change(&self, change: Change) -> Result<(), SyncError> {
        let mut state = self.state.write().await;
        state.pending_changes.push(change);
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum SyncMessage {
    Change(Change),
    Ack(SyncAck),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SyncAck {
    acknowledged: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] DbError),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}
```

---

*This document is part of the GoatPlatform exploration series. See [exploration.md](./exploration.md) for the complete index.*
