---
title: "OrbitingHail Rust Revision"
subtitle: "Building SQLSync in Rust with rusqlite and CRDTs"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.orbitinghail
related: 01-storage-engine-deep-dive.md
---

# Rust Revision: OrbitingHail

## Overview

This document covers implementing SQLSync in Rust - CRDT data types, SQLite integration with rusqlite, and sync protocol implementation.

## Part 1: Core SQLSync Database

### Database Structure

```rust
use rusqlite::{Connection, params};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SQLSyncDb {
    conn: Arc<Connection>,
    actor_id: String,
    clock: Arc<RwLock<VectorClock>>,
}

impl SQLSyncDb {
    pub fn open(path: &str, actor_id: String) -> Result<Self, SQLSyncError> {
        let conn = Connection::open(path)?;

        // Initialize schema
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                title TEXT,
                title_timestamp INTEGER,
                title_actor TEXT,
                content TEXT,
                content_timestamp INTEGER,
                content_actor TEXT,
                created_at INTEGER DEFAULT (strftime('%s', 'now')),
                created_by TEXT
            );

            CREATE TABLE IF NOT EXISTS vector_clocks (
                table_name TEXT NOT NULL,
                row_id TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                counter INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (table_name, row_id, actor_id)
            );

            CREATE TABLE IF NOT EXISTS change_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                table_name TEXT NOT NULL,
                row_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                changes TEXT NOT NULL,
                clock_data TEXT NOT NULL,
                timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                synced INTEGER DEFAULT 0
            );"
        )?;

        let clock = Arc::new(RwLock::new(VectorClock::new()));

        Ok(Self {
            conn: Arc::new(conn),
            actor_id,
            clock,
        })
    }

    pub async fn insert_document(
        &self,
        id: &str,
        title: &str,
        content: &str,
    ) -> Result<InsertResult, SQLSyncError> {
        let timestamp = current_timestamp_us();

        // Increment clock
        let mut clock = self.clock.write().await;
        clock.tick(&self.actor_id);

        // Begin transaction
        let tx = self.conn.transaction()?;

        // Insert document
        tx.execute(
            "INSERT INTO documents
             (id, title, title_timestamp, title_actor, content, content_timestamp, content_actor, created_by)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id, title, timestamp, self.actor_id,
                content, timestamp, self.actor_id, self.actor_id
            ],
        )?;

        // Initialize vector clock
        tx.execute(
            "INSERT INTO vector_clocks (table_name, row_id, actor_id, counter)
             VALUES ('documents', ?, ?, ?)",
            params![id, self.actor_id, clock.get(&self.actor_id).unwrap_or(0)],
        )?;

        // Record change
        let changes = json!({
            "title": {"value": title, "timestamp": timestamp, "actor": self.actor_id},
            "content": {"value": content, "timestamp": timestamp, "actor": self.actor_id}
        });

        tx.execute(
            "INSERT INTO change_log (table_name, row_id, operation, changes, clock_data)
             VALUES ('documents', ?, 'INSERT', ?, ?)",
            params![id, serde_json::to_string(&changes)?, serde_json::to_string(&*clock)?],
        )?;

        tx.commit()?;

        Ok(InsertResult {
            id: id.to_string(),
            clock: clock.clone(),
        })
    }

    pub async fn update_document(
        &self,
        id: &str,
        title: Option<&str>,
        content: Option<&str>,
    ) -> Result<UpdateResult, SQLSyncError> {
        let timestamp = current_timestamp_us();

        // Increment clock
        let mut clock = self.clock.write().await;
        clock.tick(&self.actor_id);

        // Begin transaction
        let tx = self.conn.transaction()?;

        // Get existing document for LWW comparison
        let existing: Option<(String, i64, String, String, i64, String)> = tx.query_row(
            "SELECT title, title_timestamp, title_actor, content, content_timestamp, content_actor
             FROM documents WHERE id = ?",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
        ).optional()?;

        let existing = existing.ok_or(SQLSyncError::NotFound)?;

        // Determine new values based on LWW
        let (new_title, new_title_ts, new_title_actor) = if let Some(title) = title {
            if timestamp >= existing.1 {
                (title.to_string(), timestamp, self.actor_id.clone())
            } else {
                (existing.0.clone(), existing.1, existing.2.clone())
            }
        } else {
            (existing.0.clone(), existing.1, existing.2.clone())
        };

        let (new_content, new_content_ts, new_content_actor) = if let Some(content) = content {
            if timestamp >= existing.4 {
                (content.to_string(), timestamp, self.actor_id.clone())
            } else {
                (existing.3.clone(), existing.4, existing.5.clone())
            }
        } else {
            (existing.3.clone(), existing.4, existing.5.clone())
        };

        // Update document
        tx.execute(
            "UPDATE documents
             SET title = ?, title_timestamp = ?, title_actor = ?,
                 content = ?, content_timestamp = ?, content_actor = ?
             WHERE id = ?",
            params![
                new_title, new_title_ts, new_title_actor,
                new_content, new_content_ts, new_content_actor,
                id
            ],
        )?;

        // Update vector clock
        tx.execute(
            "INSERT OR REPLACE INTO vector_clocks (table_name, row_id, actor_id, counter)
             VALUES ('documents', ?, ?, ?)",
            params![id, self.actor_id, clock.get(&self.actor_id).unwrap_or(0)],
        )?;

        // Record change
        let mut changes = serde_json::Map::new();
        if title.is_some() {
            changes.insert("title".to_string(), json!({
                "value": new_title, "timestamp": new_title_ts, "actor": new_title_actor
            }));
        }
        if content.is_some() {
            changes.insert("content".to_string(), json!({
                "value": new_content, "timestamp": new_content_ts, "actor": new_content_actor
            }));
        }

        tx.execute(
            "INSERT INTO change_log (table_name, row_id, operation, changes, clock_data)
             VALUES ('documents', ?, 'UPDATE', ?, ?)",
            params![id, serde_json::to_string(&changes)?, serde_json::to_string(&*clock)?],
        )?;

        tx.commit()?;

        Ok(UpdateResult {
            id: id.to_string(),
            clock: clock.clone(),
        })
    }

    pub fn get_document(&self, id: &str) -> Result<Option<Document>, SQLSyncError> {
        let doc = self.conn.query_row(
            "SELECT id, title, content, created_at, created_by
             FROM documents WHERE id = ?",
            params![id],
            |row| {
                Ok(Document {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    content: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                })
            }
        ).optional()?;

        Ok(doc)
    }

    pub async fn get_unsynced_changes(&self, limit: usize) -> Result<Vec<ChangeRecord>, SQLSyncError> {
        let mut stmt = self.conn.prepare(
            "SELECT table_name, row_id, operation, changes, clock_data, timestamp
             FROM change_log
             WHERE synced = 0
             ORDER BY timestamp ASC
             LIMIT ?"
        )?;

        let changes = stmt
            .query_map(params![limit], |row| {
                let changes_json: String = row.get(3)?;
                let clock_json: String = row.get(4)?;

                Ok(ChangeRecord {
                    table_name: row.get(0)?,
                    row_id: row.get(1)?,
                    operation: row.get(2)?,
                    changes: serde_json::from_str(&changes_json)?,
                    clock: serde_json::from_str(&clock_json)?,
                    timestamp: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(changes)
    }

    pub fn mark_changes_synced(&self, change_ids: &[i64]) -> Result<(), SQLSyncError> {
        let mut stmt = self.conn.prepare(
            "UPDATE change_log SET synced = 1 WHERE id = ?"
        )?;

        for id in change_ids {
            stmt.execute(params![id])?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: u64,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct InsertResult {
    pub id: String,
    pub clock: VectorClock,
}

#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub id: String,
    pub clock: VectorClock,
}

#[derive(Debug, thiserror::Error)]
pub enum SQLSyncError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Document not found")]
    NotFound,

    #[error("Clock error: {0}")]
    Clock(String),
}

fn current_timestamp_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}
```

## Part 2: CRDT Data Types

```rust
use std::collections::HashMap;

/// Vector Clock for causality tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorClock {
    counters: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self { counters: HashMap::new() }
    }

    pub fn tick(&mut self, actor: &str) {
        *self.counters.entry(actor.to_string()).or_insert(0) += 1;
    }

    pub fn get(&self, actor: &str) -> Option<u64> {
        self.counters.get(actor).copied()
    }

    pub fn merge(&mut self, other: &Self) {
        for (actor, count) in &other.counters {
            let entry = self.counters.entry(actor.clone()).or_insert(0);
            *entry = (*entry).max(*count);
        }
    }

    pub fn compare(&self, other: &Self) -> ClockOrdering {
        let mut less = false;
        let mut greater = false;

        let all_actors: std::collections::HashSet<_> = self
            .counters.keys()
            .chain(other.counters.keys())
            .collect();

        for actor in all_actors {
            let self_count = self.counters.get(actor).copied().unwrap_or(0);
            let other_count = other.counters.get(actor).copied().unwrap_or(0);

            if self_count < other_count {
                less = true;
            } else if self_count > other_count {
                greater = true;
            }
        }

        match (less, greater) {
            (true, true) => ClockOrdering::Concurrent,
            (true, false) => ClockOrdering::Before,
            (false, true) => ClockOrdering::After,
            (false, false) => ClockOrdering::Equal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockOrdering {
    Before,
    After,
    Concurrent,
    Equal,
}

/// LWW Register for scalar columns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LWWRegister<T> {
    value: T,
    timestamp: u64,
    actor: String,
}

impl<T: Clone> LWWRegister<T> {
    pub fn new(value: T, timestamp: u64, actor: String) -> Self {
        Self { value, timestamp, actor }
    }

    pub fn set(&mut self, value: T, timestamp: u64, actor: String) {
        if timestamp > self.timestamp
            || (timestamp == self.timestamp && actor > self.actor)
        {
            self.value = value;
            self.timestamp = timestamp;
            self.actor = actor;
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.actor > self.actor)
        {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.actor = other.actor.clone();
        }
    }
}

/// OR-Set for array/set columns
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ORSet<T> {
    elements: HashMap<T, Vec<(String, u64)>>,
    tombstones: HashMap<T, Vec<(String, u64)>>,
}

impl<T: Eq + std::hash::Hash + Clone> ORSet<T> {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            tombstones: HashMap::new(),
        }
    }

    pub fn add(&mut self, element: T, actor: String, unique_id: u64) {
        self.elements
            .entry(element)
            .or_insert_with(Vec::new)
            .push((actor, unique_id));
    }

    pub fn remove(&mut self, element: &T) {
        if let Some(tags) = self.elements.get(element) {
            self.tombstones
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .extend(tags);
        }
    }

    pub fn contains(&self, element: &T) -> bool {
        let element_tags = self.elements.get(element);
        let tombstone_tags = self.tombstones.get(element);

        match (element_tags, tombstone_tags) {
            (Some(tags), None) => !tags.is_empty(),
            (Some(tags), Some(tombs)) => tags.iter().any(|tag| !tombs.contains(tag)),
            _ => false,
        }
    }

    pub fn merge(&mut self, other: &Self) {
        for (element, tags) in &other.elements {
            self.elements
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .extend(tags);
        }
        for (element, tags) in &other.tombstones {
            self.tombstones
                .entry(element.clone())
                .or_insert_with(Vec::new)
                .extend(tags);
        }
    }
}
```

## Part 3: Sync Protocol Client

```rust
use tungstenite::{connect, Message, WebSocket};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub client_clock: VectorClock,
    pub tables: HashMap<String, Vec<ChangeRecord>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub server_clock: VectorClock,
    pub accepted: HashMap<String, Vec<String>>,
    pub download: HashMap<String, Vec<ChangeRecord>>,
}

pub struct SyncClient {
    db: Arc<SQLSyncDb>,
    server_url: String,
}

impl SyncClient {
    pub fn new(db: Arc<SQLSyncDb>, server_url: String) -> Self {
        Self { db, server_url }
    }

    pub fn sync(&self) -> Result<SyncResult, SQLSyncError> {
        // Get unsynced changes
        let unsynced = self.db.get_unsynced_changes(100)?;

        // Group by table
        let mut tables: HashMap<String, Vec<ChangeRecord>> = HashMap::new();
        for change in unsynced {
            tables
                .entry(change.table_name.clone())
                .or_insert_with(Vec::new)
                .push(change);
        }

        // Get current clock
        let clock = tokio::runtime::Handle::current()
            .block_on(async { self.db.clock.read().await.clone() });

        // Build request
        let request = SyncRequest {
            client_clock: clock.clone(),
            tables,
        };

        // Connect to server
        let (mut socket, _) = connect(&self.server_url)?;

        // Send request
        let request_json = serde_json::to_string(&request)?;
        socket.send(Message::Text(request_json))?;

        // Receive response
        if let Message::Text(response_json) = socket.read()? {
            let response: SyncResponse = serde_json::from_str(&response_json)?;

            // Apply downloaded changes
            for (table_name, changes) in &response.download {
                for change in changes {
                    self.apply_remote_change(table_name, change)?;
                }
            }

            // Mark uploaded changes as synced
            let synced_ids: Vec<i64> = response.accepted.values()
                .flatten()
                .filter_map(|id| id.parse().ok())
                .collect();
            self.db.mark_changes_synced(&synced_ids)?;

            // Update local clock
            tokio::runtime::Handle::current()
                .block_on(async {
                    let mut clock = self.db.clock.write().await;
                    clock.merge(&response.server_clock);
                });

            Ok(SyncResult {
                accepted: response.accepted,
                downloaded: response.download.len(),
            })
        } else {
            Err(SQLSyncError::Clock("Invalid response".into()))
        }
    }

    fn apply_remote_change(
        &self,
        table_name: &str,
        change: &ChangeRecord,
    ) -> Result<(), SQLSyncError> {
        match change.operation {
            Operation::Insert | Operation::Update => {
                // Extract values from changes JSON
                // Apply with LWW logic
                todo!("Apply remote change based on table type")
            }
            Operation::Delete => {
                // Delete the row
                self.db.conn.execute(
                    &format!("DELETE FROM {} WHERE id = ?"),
                    params![change.row_id],
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub table_name: String,
    pub row_id: String,
    pub operation: Operation,
    pub changes: serde_json::Value,
    pub clock: VectorClock,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub accepted: HashMap<String, Vec<String>>,
    pub downloaded: usize,
}
```

---

*This document is part of the OrbitingHail exploration series. See [exploration.md](./exploration.md) for the complete index.*
