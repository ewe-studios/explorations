---
title: "Zero to DB Engineer: OrbitingHail & Distributed SQL"
subtitle: "Distributed SQL fundamentals, CRDTs, and SQL sync protocols"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.orbitinghail
related: exploration.md
---

# 00 - Zero to DB Engineer: OrbitingHail & Distributed SQL

## Overview

This document covers distributed SQL fundamentals - how to build SQL databases that sync across multiple devices using CRDTs, the SQL sync protocol, and federated database architecture.

## Part 1: The Distributed SQL Problem

### The Multi-Device Challenge

```
Single Database (Traditional):

┌─────────────────────────────────────────────────────────┐
│  Client A ──┐                                           │
│             ├───> Central Database <─── Client C        │
│  Client B ──┘                                           │
│                                                         │
│ All clients share same database                         │
│ - Single source of truth                                │
│ - Network required for all operations                   │
│ - Conflicts handled by DB (locks, isolation)            │
└───────────────────────────────────────────────────────────┘

Multi-Device Sync (Distributed):

┌─────────────────────────────────────────────────────────┐
│  Device A          Device B          Device C           │
│  ┌─────────┐      ┌─────────┐      ┌─────────┐         │
│  │ LocalDB │      │ LocalDB │      │ LocalDB │         │
│  │         │      │         │      │         │         │
│  │ Works   │      │ Works   │      │ Works   │         │
│  │ Offline │      │ Offline │      │ Offline │         │
│  └────┬────┘      └────┬────┘      └────┬────┘         │
│       │                │                │               │
│       └────────────────┼────────────────┘               │
│                        │                                 │
│              ┌─────────▼─────────┐                      │
│              │   Sync Server     │                      │
│              │   (Coordinator)   │                      │
│              └───────────────────┘                      │
│                                                         │
│ Each device has full local database                     │
│ - Works offline (local-first)                           │
│ - Sync when online                                      │
│ - Conflicts resolved with CRDTs                         │
└───────────────────────────────────────────────────────────┘
```

### Why CRDTs for SQL?

```
Traditional Replication Problems:

┌─────────────────────────────────────────────────────────┐
│ Scenario: Two devices update same row                   │
│                                                         │
│ Device A: UPDATE users SET name = 'Alice' WHERE id = 1  │
│ Device B: UPDATE users SET email = 'alice@test.com'     │
│             WHERE id = 1                                │
│                                                         │
│ Traditional (Primary-Based):                            │
│ - One device must be primary                            │
│ - Other device's change overwrites or blocks            │
│ - Network partition = unavailable                       │
│                                                         │
│ Problem: Lost updates!                                  │
│ - Device A's name change lost                           │
│ - OR Device B's email change lost                       │
└───────────────────────────────────────────────────────────┘

CRDT-Based Replication:

┌─────────────────────────────────────────────────────────┐
│ CRDT Solution:                                          │
│                                                         │
│ Each column is a CRDT register:                         │
│ - name: LWW-Register ('Alice', ts: 100)                │
│ - email: LWW-Register ('bob@test.com', ts: 99)         │
│                                                         │
│ Device A change:                                        │
│   name: LWW-Register ('Alice', ts: 100)                │
│                                                         │
│ Device B change:                                        │
│   email: LWW-Register ('alice@test.com', ts: 101)      │
│                                                         │
│ After sync (merge):                                     │
│   name: 'Alice' (ts: 100)                               │
│   email: 'alice@test.com' (ts: 101)                     │
│                                                         │
│ Result: BOTH changes preserved!                         │
└───────────────────────────────────────────────────────────┘
```

## Part 2: SQLSync Architecture

### SQLSync Components

```
SQLSync Architecture:

┌─────────────────────────────────────────────────────────┐
│ Application Layer                                       │
│ ┌─────────────────────────────────────────────────────┐│
│ │ SQL Interface (SQLite-compatible)                   ││
│ │ - Standard SQL: SELECT, INSERT, UPDATE, DELETE      ││
│ │ - Prepared statements                               ││
│ │ - Transactions (local)                              ││
│ └─────────────────────────────────────────────────────┘│
│                        │                                │
│ ┌─────────────────────────────────────────────────────┐│
│ │ CRDT Layer                                          ││
│ │ - Row-level CRDTs                                   ││
│ │ - Column-level conflict resolution                  ││
│ │ - Vector clocks for causality                       ││
│ └─────────────────────────────────────────────────────┘│
│                        │                                │
│ ┌─────────────────────────────────────────────────────┐│
│ │ Sync Engine                                         ││
│ │ - Change detection                                  ││
│ │ - Delta compression                                 ││
│ │ - Batch upload/download                             ││
│ └─────────────────────────────────────────────────────┘│
│                        │                                │
│ ┌─────────────────────────────────────────────────────┐│
│ │ Storage Layer                                       ││
│ │ - Local SQLite database                             ││
│ │ - CRDT metadata tables                              ││
│ │ - Sync log                                          ││
│ └─────────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────────────┘
```

### CRDT Table Schema

```
Standard SQL Table:

CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

SQLSync-Enabled Table (with CRDT metadata):

-- Main data table
CREATE TABLE users (
    id TEXT PRIMARY KEY,  -- UUID for global uniqueness
    name TEXT NOT NULL,
    name_timestamp INTEGER NOT NULL,
    name_actor TEXT NOT NULL,
    email TEXT NOT NULL,
    email_timestamp INTEGER NOT NULL,
    email_actor TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT NOT NULL
);

-- Vector clock table (per row)
CREATE TABLE row_clocks (
    table_name TEXT NOT NULL,
    row_id TEXT NOT NULL,
    clock_data BLOB NOT NULL,  -- Serialized vector clock
    PRIMARY KEY (table_name, row_id)
);

-- Change log table
CREATE TABLE change_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_name TEXT NOT NULL,
    row_id TEXT NOT NULL,
    operation TEXT NOT NULL,  -- INSERT, UPDATE, DELETE
    changes BLOB,  -- Column-level changes
    clock_data BLOB,  -- Vector clock at change
    timestamp INTEGER NOT NULL,
    synced INTEGER DEFAULT 0
);

-- Indexes for efficient queries
CREATE INDEX idx_change_log_synced ON change_log(synced);
CREATE INDEX idx_change_log_timestamp ON change_log(timestamp);
```

## Part 3: CRDT Types for SQL

### Column-Level CRDTs

```
CRDT Type Mapping:

┌─────────────────────────────────────────────────────────┐
│ SQL Type      │ CRDT Type      │ Merge Strategy        │
├─────────────────────────────────────────────────────────┤
│ INTEGER       │ LWW-Register   │ Highest timestamp wins│
│ REAL          │ LWW-Register   │ Highest timestamp wins│
│ TEXT          │ LWW-Register   │ Highest timestamp wins│
│ BLOB          │ LWW-Register   │ Highest timestamp wins│
│ BOOLEAN       │ LWW-Register   │ Highest timestamp wins│
│               │                │                       │
│ JSON/Object   │ MV-Register    │ Multi-value, keep     │
│               │                │ concurrent values     │
│               │                │                       │
│ SET/Array     │ OR-Set         │ Add-wins semantics    │
│               │                │                       │
│ Counter       │ PN-Counter     │ Positive-negative     │
│               │                │ counters per actor    │
│               │                │                       │
│ Timestamp     │ LWW-Register   │ Latest timestamp      │
└───────────────────────────────────────────────────────────┘
```

### LWW-Register Implementation

```rust
/// Last-Writer-Wins Register for SQL columns
#[derive(Debug, Clone)]
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
        if timestamp > self.timestamp {
            self.value = value;
            self.timestamp = timestamp;
            self.actor = actor;
        } else if timestamp == self.timestamp && actor > self.actor {
            // Tie-breaker: higher actor ID wins
            self.value = value;
            self.timestamp = timestamp;
            self.actor = actor;
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn merge(&mut self, other: &Self) {
        if other.timestamp > self.timestamp {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.actor = other.actor.clone();
        } else if other.timestamp == self.timestamp && other.actor > self.actor {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.actor = other.actor.clone();
        }
    }

    pub fn to_bytes(&self) -> Vec<u8>
    where
        T: serde::Serialize,
    {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        let actor_bytes = self.actor.as_bytes();
        bytes.extend_from_slice(&(actor_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(actor_bytes);
        bytes.extend_from_slice(&serde_json::to_vec(&self.value).unwrap());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self>
    where
        T: serde::de::DeserializeOwned,
    {
        if bytes.len() < 16 {
            return None;
        }

        let timestamp = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let actor_len = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize;

        if bytes.len() < 12 + actor_len {
            return None;
        }

        let actor = String::from_utf8(bytes[12..12 + actor_len].to_vec()).ok()?;
        let value: T = serde_json::from_slice(&bytes[12 + actor_len..]).ok()?;

        Some(Self { value, timestamp, actor })
    }
}

// SQL Integration Example:
// UPDATE users
// SET name = 'Alice',
//     name_timestamp = 100,
//     name_actor = 'client-a'
// WHERE id = 'user-123';
```

### OR-Set for Array Columns

```rust
/// Observed-Remove Set for array/set columns
#[derive(Debug, Clone)]
pub struct ORSet<T> {
    elements: std::collections::HashMap<T, Vec<(String, u64)>>,  // element -> [(actor, unique_id)]
    tombstones: std::collections::HashMap<T, Vec<(String, u64)>>,
}

impl<T: Eq + std::hash::Hash + Clone> ORSet<T> {
    pub fn new() -> Self {
        Self {
            elements: std::collections::HashMap::new(),
            tombstones: std::collections::HashMap::new(),
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
            (Some(tags), Some(tombs)) => {
                tags.iter().any(|tag| !tombs.contains(tag))
            }
            _ => false,
        }
    }

    pub fn merge(&mut self, other: &Self) {
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
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.elements.iter()
            .filter(|(element, tags)| {
                if let Some(tombs) = self.tombstones.get(*element) {
                    tags.iter().any(|tag| !tombs.contains(tag))
                } else {
                    !tags.is_empty()
                }
            })
            .map(|(elem, _)| elem)
    }
}

// SQL Integration for JSON array column:
// Column stores: {"crdt_type": "or-set", "value": ["tag1", "tag2"]}
// Tags stored in separate table for efficient queries
```

## Part 4: Sync Protocol

### Sync Message Format

```
Client → Server Sync Request:

{
    "type": "sync_request",
    "client_id": "client-a",
    "client_clock": {
        "client-a": 5,
        "client-b": 3,
        "server": 10
    },
    "tables": {
        "users": {
            "changes": [
                {
                    "row_id": "user-123",
                    "operation": "UPDATE",
                    "columns": {
                        "name": {
                            "value": "Alice",
                            "timestamp": 100,
                            "actor": "client-a"
                        }
                    },
                    "clock": {"client-a": 5}
                }
            ]
        }
    }
}

Server → Client Sync Response:

{
    "type": "sync_response",
    "server_clock": {
        "client-a": 5,
        "client-b": 3,
        "server": 12
    },
    "accepted": {
        "users": ["user-123"]
    },
    "rejected": {},
    "download": {
        "users": [
            {
                "row_id": "user-456",
                "operation": "INSERT",
                "columns": {
                    "name": {
                        "value": "Bob",
                        "timestamp": 101,
                        "actor": "client-b"
                    },
                    "email": {
                        "value": "bob@test.com",
                        "timestamp": 101,
                        "actor": "client-b"
                    }
                },
                "clock": {"client-b": 3, "server": 11}
            }
        ]
    }
}
```

### Conflict Resolution Flow

```
Conflict Detection and Resolution:

┌─────────────────────────────────────────────────────────┐
│ Step 1: Receive change from Client-A                   │
│                                                         │
│ Change: UPDATE users SET name = 'Alice'                │
│         WHERE id = 'user-123'                           │
│ Clock: {client-a: 5}                                   │
│                                                         │
│ Step 2: Check existing change in server                │
│                                                         │
│ Existing: UPDATE users SET email = 'old@test.com'      │
│           WHERE id = 'user-123'                         │
│ Clock: {client-b: 3, server: 10}                       │
│                                                         │
│ Step 3: Compare clocks                                 │
│                                                         │
│ {client-a: 5} vs {client-b: 3, server: 10}             │
│ Result: CONCURRENT (neither dominates)                 │
│                                                         │
│ Step 4: Column-level merge                             │
│                                                         │
│ - name column: Client-A's change (only modifier)       │
│ - email column: Keep existing (Client-B's change)      │
│                                                         │
│ Step 5: Apply merged result                            │
│                                                         │
│ Final row:                                              │
│   name = 'Alice' (from Client-A)                       │
│   email = 'old@test.com' (from Client-B)               │
│   clock = {client-a: 5, client-b: 3, server: 11}       │
│                                                         │
│ Step 6: Broadcast merged change to all clients         │
└───────────────────────────────────────────────────────────┘
```

## Part 5: SQLSync Usage

### Basic Operations

```typescript
import { SQLSync } from '@orbitinghail/sqlsync';

// Initialize SQLSync database
const db = new SQLSync({
    path: 'my-app.db',
    schema: `
        CREATE TABLE users (
            id TEXT PRIMARY KEY,
            name TEXT,
            email TEXT,
            tags JSON
        );
        CREATE TABLE posts (
            id TEXT PRIMARY KEY,
            title TEXT,
            content TEXT,
            author_id TEXT,
            FOREIGN KEY (author_id) REFERENCES users(id)
        );
    `,
    syncUrl: 'wss://sync.example.com',
});

// Local operations (instant, no network)
await db.run(`
    INSERT INTO users (id, name, email)
    VALUES (?, ?, ?)
`, ['user-123', 'Alice', 'alice@test.com']);

// Sync happens automatically in background
// When online: changes uploaded, remote changes downloaded
// When offline: changes queued for later sync
```

```typescript
// Conflict-free collaborative editing

// Client A updates name
await db.run(`
    UPDATE users SET name = 'Alice' WHERE id = 'user-123'
`);

// Client B updates email (concurrently)
await db.run(`
    UPDATE users SET email = 'alice@example.com' WHERE id = 'user-123'
`);

// After sync, both changes preserved:
// name = 'Alice' (from Client A)
// email = 'alice@example.com' (from Client B)

// No conflicts! Column-level CRDTs merge automatically
```

### Advanced Patterns

```typescript
// Counter CRDT for collaborative counters
await db.run(`
    UPDATE metrics
    SET view_count = view_count + 1,
        view_count_crdt = ?
    WHERE id = 'page-1'
`, [JSON.stringify({
    type: 'pn-counter',
    actor: 'client-a',
    increment: 1
})]);

// OR-Set for collaborative tags
await db.run(`
    UPDATE posts
    SET tags = json_insert(
        tags,
        '$.#',
        json_object('tag', 'javascript', 'actor', 'client-a', 'id', 1)
    )
    WHERE id = 'post-1'
`);

// Query with CRDT-aware results
const result = await db.query(`
    SELECT id, name, email FROM users WHERE id = 'user-123'
`);
// Returns merged values from all concurrent updates
```

---

*This document is part of the OrbitingHail exploration series. See [exploration.md](./exploration.md) for the complete index.*
