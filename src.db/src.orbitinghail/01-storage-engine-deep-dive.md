---
title: "OrbitingHail Storage Engine Deep Dive"
subtitle: "SQLSync storage internals, CRDT persistence, and change tracking"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.orbitinghail
related: 00-zero-to-db-engineer.md
---

# 01 - Storage Engine Deep Dive: OrbitingHail

## Overview

This document covers SQLSync storage internals - how CRDT data is persisted, change log management, and efficient sync tracking.

## Part 1: Storage Layout

### SQLite Schema with CRDT Metadata

```sql
-- Main data table with CRDT columns
CREATE TABLE documents (
    -- Business data
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,

    -- CRDT metadata for each mutable column
    title_value TEXT,
    title_timestamp INTEGER,
    title_actor TEXT,

    content_value TEXT,
    content_timestamp INTEGER,
    content_actor TEXT,

    -- Row-level metadata
    created_at INTEGER DEFAULT (strftime('%s', 'now')),
    created_by TEXT,
    row_version INTEGER DEFAULT 0
);

-- Vector clock table (tracks causality)
CREATE TABLE vector_clocks (
    table_name TEXT NOT NULL,
    row_id TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    counter INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (table_name, row_id, actor_id)
);

-- Change log (for sync)
CREATE TABLE change_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    row_id TEXT NOT NULL,
    operation TEXT NOT NULL,  -- INSERT, UPDATE, DELETE

    -- Changed columns (JSON)
    changes TEXT NOT NULL,

    -- Vector clock at change time
    clock_data TEXT NOT NULL,

    -- Metadata
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    synced INTEGER DEFAULT 0,
    sync_group_id TEXT
);

-- Indexes for efficient sync
CREATE INDEX idx_change_log_synced ON change_log(synced, timestamp);
CREATE INDEX idx_change_log_table_row ON change_log(table_name, row_id);
CREATE INDEX idx_vector_clocks_table_row ON vector_clocks(table_name, row_id);

-- Trigger to auto-update change log on modifications
CREATE TRIGGER documents_after_update AFTER UPDATE ON documents
FOR EACH ROW
BEGIN
    INSERT INTO change_log (table_name, row_id, operation, changes, clock_data)
    VALUES (
        'documents',
        NEW.id,
        'UPDATE',
        json_object(
            'title', CASE WHEN OLD.title != NEW.title
                         THEN json_object('value', NEW.title,
                                          'timestamp', NEW.title_timestamp,
                                          'actor', NEW.title_actor)
                         END,
            'content', CASE WHEN OLD.content != NEW.content
                           THEN json_object('value', NEW.content,
                                            'timestamp', NEW.content_timestamp,
                                            'actor', NEW.content_actor)
                           END
        ),
        (SELECT json_group_object(actor_id, counter)
         FROM vector_clocks
         WHERE table_name = 'documents' AND row_id = NEW.id)
    );
END;
```

### Page Structure for CRDT Data

```
CRDT Row Layout:

┌─────────────────────────────────────────────────────────┐
│ SQLite Row with CRDT Metadata                          │
├─────────────────────────────────────────────────────────┤
│ Column              │ Size    │ Description             │
├─────────────────────────────────────────────────────────┤
│ id                  │ 36 bytes│ UUID string            │
│ title               │ variable│ Current title value    │
│ title_timestamp     │ 8 bytes │ Unix timestamp (μs)    │
│ title_actor         │ 36 bytes│ Actor UUID             │
│ content             │ variable│ Current content        │
│ content_timestamp   │ 8 bytes │ Unix timestamp (μs)    │
│ content_actor       │ 36 bytes│ Actor UUID             │
│ created_at          │ 8 bytes │ Creation timestamp     │
│ created_by          │ 36 bytes│ Creator UUID           │
│ row_version         │ 4 bytes │ Optimistic lock        │
├─────────────────────────────────────────────────────────┤
│ Total (fixed): ~172 bytes + variable text              │
└───────────────────────────────────────────────────────────┘

Vector Clock Entry:
┌─────────────────────────────────────────────────────────┐
│ table_name │ row_id │ actor_id │ counter              │
├─────────────────────────────────────────────────────────┤
│ documents  │ doc-1  │ client-a │ 5                    │
│ documents  │ doc-1  │ client-b │ 3                    │
│ documents  │ doc-1  │ server   │ 10                   │
└───────────────────────────────────────────────────────────┘

Reconstruct Vector Clock:
{client-a: 5, client-b: 3, server: 10}
```

## Part 2: Change Log Management

### Change Recording

```rust
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, params};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub table_name: String,
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

pub struct ChangeLog {
    conn: Connection,
    local_actor_id: String,
}

impl ChangeLog {
    pub fn new(conn: Connection, local_actor_id: String) -> Self {
        Self { conn, local_actor_id }
    }

    /// Record a change for later sync
    pub fn record_change(
        &self,
        table_name: &str,
        row_id: &str,
        operation: Operation,
        changes: &serde_json::Value,
        clock: &VectorClock,
    ) -> Result<i64, rusqlite::Error> {
        let clock_json = serde_json::to_string(clock)?;
        let changes_json = serde_json::to_string(changes)?;

        let mut stmt = self.conn.prepare(
            "INSERT INTO change_log
             (table_name, row_id, operation, changes, clock_data, timestamp)
             VALUES (?, ?, ?, ?, ?, strftime('%s', 'now'))"
        )?;

        let change_id = stmt.insert(params![
            table_name,
            row_id,
            operation as u8,
            changes_json,
            clock_json
        ])?;

        Ok(change_id as i64)
    }

    /// Get unsynced changes
    pub fn get_unsynced_changes(
        &self,
        limit: usize,
    ) -> Result<Vec<ChangeRecord>, rusqlite::Error> {
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
                    changes: serde_json::from_str(&changes_json).unwrap(),
                    clock: serde_json::from_str(&clock_json).unwrap(),
                    timestamp: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(changes)
    }

    /// Mark changes as synced
    pub fn mark_synced(&self, change_ids: &[i64]) -> Result<(), rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "UPDATE change_log SET synced = 1, sync_group_id = ?1 WHERE id = ?2"
        )?;

        let sync_group = uuid::Uuid::new_v4().to_string();

        for change_id in change_ids {
            stmt.execute(params![sync_group, change_id])?;
        }

        Ok(())
    }

    /// Get changes since a vector clock
    pub fn changes_since(
        &self,
        table_name: &str,
        clock: &VectorClock,
    ) -> Result<Vec<ChangeRecord>, rusqlite::Error> {
        // Build clock filter JSON
        let clock_json = serde_json::to_string(clock)?;

        let mut stmt = self.conn.prepare(
            "SELECT table_name, row_id, operation, changes, clock_data, timestamp
             FROM change_log
             WHERE table_name = ?1
               AND timestamp > (
                   SELECT MAX(timestamp) FROM change_log
                   WHERE clock_data LIKE ?2
               )
             ORDER BY timestamp ASC"
        )?;

        // Simplified: just get all changes after oldest clock entry
        let changes = stmt
            .query_map(params![table_name, format!("%{}%", clock_json)])?
            .filter_map(|r| r.ok())
            .collect();

        Ok(changes)
    }

    /// Prune old synced changes
    pub fn prune_old_changes(&self, retention_days: u32) -> Result<usize, rusqlite::Error> {
        let cutoff = chrono::Utc::now()
            - chrono::Duration::days(retention_days as i64);
        let cutoff_ts = cutoff.timestamp();

        let mut stmt = self.conn.prepare(
            "DELETE FROM change_log
             WHERE synced = 1 AND timestamp < ?"
        )?;

        let deleted = stmt.execute(params![cutoff_ts])?;

        Ok(deleted)
    }
}
```

### Incremental Sync State

```rust
/// Tracks sync state per peer
#[derive(Debug, Clone)]
pub struct SyncState {
    peer_id: String,
    last_sync_clock: VectorClock,
    last_sync_timestamp: u64,
    pending_send: Vec<ChangeRecord>,
    pending_receive: Vec<ChangeRecord>,
}

impl SyncState {
    pub fn new(peer_id: &str) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            last_sync_clock: VectorClock::new(),
            last_sync_timestamp: 0,
            pending_send: Vec::new(),
            pending_receive: Vec::new(),
        }
    }

    /// Update sync state after successful sync
    pub fn update_sync_state(&mut self, clock: VectorClock, timestamp: u64) {
        self.last_sync_clock = clock;
        self.last_sync_timestamp = timestamp;
        self.pending_send.clear();
        self.pending_receive.clear();
    }

    /// Add change to pending send queue
    pub fn queue_for_send(&mut self, change: ChangeRecord) {
        self.pending_send.push(change);
    }

    /// Get pending changes to send
    pub fn get_pending(&self) -> &[ChangeRecord] {
        &self.pending_send
    }
}

/// Persisted sync state table
impl ChangeLog {
    pub fn load_sync_state(&self, peer_id: &str) -> Result<SyncState, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT last_sync_clock, last_sync_timestamp
             FROM sync_state
             WHERE peer_id = ?"
        )?;

        match stmt.query_row(params![peer_id], |row| {
            let clock_json: String = row.get(0)?;
            let timestamp: u64 = row.get(1)?;

            Ok((
                serde_json::from_str::<VectorClock>(&clock_json).unwrap_or_default(),
                timestamp,
            ))
        }) {
            Ok((clock, timestamp)) => Ok(SyncState {
                peer_id: peer_id.to_string(),
                last_sync_clock: clock,
                last_sync_timestamp: timestamp,
                pending_send: Vec::new(),
                pending_receive: Vec::new(),
            }),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Ok(SyncState::new(peer_id))
            }
            Err(e) => Err(e),
        }
    }

    pub fn save_sync_state(&self, state: &SyncState) -> Result<(), rusqlite::Error> {
        let clock_json = serde_json::to_string(&state.last_sync_clock)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO sync_state
             (peer_id, last_sync_clock, last_sync_timestamp)
             VALUES (?, ?, ?)",
            params![state.peer_id, clock_json, state.last_sync_timestamp],
        )?;

        Ok(())
    }
}
```

## Part 3: Efficient Change Detection

### Trigger-Based Change Detection

```sql
-- Automatic change detection using triggers

-- For INSERT
CREATE TRIGGER documents_after_insert AFTER INSERT ON documents
FOR EACH ROW
BEGIN
    INSERT INTO change_log (
        table_name, row_id, operation, changes, clock_data, timestamp
    )
    VALUES (
        'documents',
        NEW.id,
        'INSERT',
        json_object(
            'title', json_object('value', NEW.title, 'timestamp', NEW.title_timestamp, 'actor', NEW.title_actor),
            'content', json_object('value', NEW.content, 'timestamp', NEW.content_timestamp, 'actor', NEW.content_actor)
        ),
        json_object('local', 1),
        strftime('%s', 'now')
    );
END;

-- For DELETE
CREATE TRIGGER documents_after_delete AFTER DELETE ON documents
FOR EACH ROW
BEGIN
    INSERT INTO change_log (
        table_name, row_id, operation, changes, clock_data, timestamp
    )
    VALUES (
        'documents',
        OLD.id,
        'DELETE',
        json_object('deleted', true),
        (SELECT json_group_object(actor_id, counter) FROM vector_clocks
         WHERE table_name = 'documents' AND row_id = OLD.id),
        strftime('%s', 'now')
    );
END;
```

### Batch Change Aggregation

```rust
/// Aggregate multiple changes to same row
pub fn aggregate_changes(
    changes: Vec<ChangeRecord>,
) -> Vec<ChangeRecord> {
    use std::collections::HashMap;

    let mut aggregated: HashMap<String, ChangeRecord> = HashMap::new();

    for change in changes {
        let key = format!("{}:{}", change.table_name, change.row_id);

        if let Some(existing) = aggregated.get_mut(&key) {
            // Merge changes
            match (&mut existing.operation, change.operation) {
                // INSERT + UPDATE = INSERT with merged values
                (Operation::Insert, Operation::Update) => {
                    existing.changes = merge_json(&existing.changes, &change.changes);
                }
                // UPDATE + UPDATE = merged UPDATE
                (Operation::Update, Operation::Update) => {
                    existing.changes = merge_json(&existing.changes, &change.changes);
                }
                // Any + DELETE = DELETE
                (_, Operation::Delete) => {
                    existing.operation = Operation::Delete;
                    existing.changes = json!({"deleted": true});
                }
                _ => {}
            }

            // Merge clocks
            existing.clock.merge(&change.clock);
        } else {
            aggregated.insert(key.clone(), change);
        }
    }

    aggregated.into_values().collect()
}

fn merge_json(base: &serde_json::Value, patch: &serde_json::Value) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(mut b), serde_json::Value::Object(p)) => {
            for (key, value) in p {
                b.insert(key.clone(), value.clone());
            }
            serde_json::Value::Object(b)
        }
        _ => patch.clone(),
    }
}
```

---

*This document is part of the OrbitingHail exploration series. See [exploration.md](./exploration.md) for the complete index.*
