---
title: "Zero to DB Engineer: GoatPlatform & Real-Time Databases"
subtitle: "Real-time database fundamentals, CRDTs, and SQL sync protocols"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.goatplatform
related: exploration.md
---

# 00 - Zero to DB Engineer: GoatPlatform & Real-Time Databases

## Overview

This document covers real-time database fundamentals - how to build applications where data syncs instantly across all clients, conflict resolution strategies, and the SQL sync protocol pattern.

## Part 1: The Real-Time Data Problem

### The Sync Challenge

```
Traditional Request/Response Model:

┌─────────────────────────────────────────────────────────┐
│ Client A          Server            Database    Client B │
│    │                │                  │         │      │
│    │ GET /posts     │                  │         │      │
│    │───────────────>│                  │         │      │
│    │                │ SELECT * FROM   │         │      │
│    │                │ posts           │         │      │
│    │                │─────────────────>│         │      │
│    │                │                 │         │      │
│    │                │ [posts data]    │         │      │
│    │                │<─────────────────│         │      │
│    │ [posts data]   │                  │         │      │
│    │<───────────────│                  │         │      │
│    │                │                  │         │      │
│    │                │                  │  GET /posts   │
│    │                │                  │──────────────>│
│    │                │                  │  [same data]  │
│    │                │                  │<──────────────│
│    │                │                  │         │      │
│ Problem: Client B doesn't see Client A's updates        │
│ until Client B explicitly requests new data             │
└───────────────────────────────────────────────────────────┘
```

```
Real-Time Sync Model:

┌─────────────────────────────────────────────────────────┐
│ Client A          Server            Database    Client B │
│    │                │                  │         │      │
│    │ INSERT INTO    │                  │         │      │
│    │ posts VALUES   │                  │         │      │
│    │───────────────>│                  │         │      │
│    │                │ INSERT INTO      │         │      │
│    │                │ posts VALUES     │         │      │
│    │                │─────────────────>│         │      │
│    │                │                  │         │      │
│    │                │ [ack]           │         │      │
│    │                │<─────────────────│         │      │
│    │ [ack]          │                  │         │      │
│    │<───────────────│                  │         │      │
│    │                │                  │         │      │
│    │                │ [CHANGE_EVENT]   │ PUSH    │      │
│    │                │─────────────────────────>│      │
│    │                │                  │         │      │
│    │                │                  │  [new post]   │
│    │                │                  │<──────────────│
│    │                │                  │         │      │
│ Solution: Client B receives updates instantly           │
└───────────────────────────────────────────────────────────┘
```

### Real-Time Use Cases

```
┌─────────────────────────────────────────────────────────┐
│ Use Case              │ Sync Requirements               │
├─────────────────────────────────────────────────────────┤
│ Collaborative Editing│ Sub-100ms latency, conflict     │
│ (Google Docs style)  │ resolution, operational transform│
│                      │ or CRDTs                        │
├─────────────────────────────────────────────────────────┤
│ Chat Applications    │ Ordered delivery, presence,      │
│ (Slack, Discord)     │ typing indicators, read receipts│
├─────────────────────────────────────────────────────────┤
│ Live Dashboards      │ High throughput, aggregations,   │
│ (Analytics, IoT)     │ time-series data, subscriptions │
├─────────────────────────────────────────────────────────┤
│ Multiplayer Games    │ State sync, conflict-free,       │
│ (Turn-based, Real-time)│ rollback, reconciliation       │
├─────────────────────────────────────────────────────────┤
│ Shared Whiteboards   │ CRDTs for drawings, concurrent   │
│ (Miro, Figma)        │ edits, undo/redo sync           │
├─────────────────────────────────────────────────────────┤
│ Financial Trading    │ Lowest latency, strict ordering, │
│ (Stock tickers)      │ audit trail, replay             │
└───────────────────────────────────────────────────────────┘
```

## Part 2: CRDT Fundamentals

### What are CRDTs?

```
CRDT (Conflict-Free Replicated Data Types):

Definition:
- Data structures that can be replicated across multiple nodes
- Updates can be applied independently without coordination
- All replicas converge to the same state eventually
- No conflicts possible by design

Two Types of CRDTs:

1. State-Based CRDTs (Convergent Replicated Data Types)
   - Each node maintains local state
   - Periodically send full state to other nodes
   - Merge function combines states: merge(state1, state2)
   - Merge must be: associative, commutative, idempotent

2. Operation-Based CRDTs (Commutative Replicated Data Types)
   - Nodes broadcast operations (not state)
   - Operations must commute: op1 then op2 = op2 then op1
   - Lower bandwidth (only changes, not full state)
```

```
CRDT Math Properties:

For state-based CRDTs, merge operation must satisfy:

1. Associative: merge(a, merge(b, c)) = merge(merge(a, b), c)
   ┌─────────────────────────────────────────────────────┐
   │ merge(a, merge(b, c)):                              │
   │   a = {1, 2}                                        │
   │   b = {2, 3}                                        │
   │   c = {3, 4}                                        │
   │   merge(b, c) = {2, 3, 4}                          │
   │   merge(a, {2, 3, 4}) = {1, 2, 3, 4}               │
   │                                                     │
   │ merge(merge(a, b), c):                              │
   │   merge(a, b) = {1, 2, 3}                          │
   │   merge({1, 2, 3}, c) = {1, 2, 3, 4}               │
   │                                                     │
   │ Result: Same! ✓                                     │
   └─────────────────────────────────────────────────────┘

2. Commutative: merge(a, b) = merge(b, a)
   merge({1, 2}, {2, 3}) = {1, 2, 3}
   merge({2, 3}, {1, 2}) = {1, 2, 3}
   Result: Same! ✓

3. Idempotent: merge(a, a) = a
   merge({1, 2}, {1, 2}) = {1, 2}
   Result: Same! ✓

These properties guarantee convergence regardless of:
- Order messages arrive
- Network delays
- Temporary disconnections
```

### Common CRDT Types

```
G-Counter (Grow-only Counter):

┌─────────────────────────────────────────────────────────┐
│ Structure:                                              │
│ - Each node has its own counter                         │
│ - Global count = sum of all node counters               │
│                                                         │
│ Node A: {A: 3, B: 2, C: 5}                             │
│ Node B: {A: 3, B: 2, C: 5}                             │
│ Node C: {A: 3, B: 2, C: 5}                             │
│                                                         │
│ Total = 3 + 2 + 5 = 10                                  │
│                                                         │
│ Increment (at Node A):                                  │
│   Node A: {A: 4, B: 2, C: 5}  (only A increments A)    │
│                                                         │
│ Merge: Element-wise maximum                            │
│   merge({A: 4, B: 2, C: 5}, {A: 3, B: 5, C: 5})        │
│   = {max(4,3), max(2,5), max(5,5)}                     │
│   = {A: 4, B: 5, C: 5}                                 │
└───────────────────────────────────────────────────────────┘
```

```
PN-Counter (Positive-Negative Counter):

┌─────────────────────────────────────────────────────────┐
│ Structure:                                              │
│ - Two G-Counters: one for increments, one for decrements│
│ - Value = sum(P) - sum(N)                              │
│                                                         │
│ Counter: {P: {A: 5, B: 3}, N: {A: 2, B: 1}}            │
│ Value = (5+3) - (2+1) = 8 - 3 = 5                      │
│                                                         │
│ Increment (at A): P.A += 1                             │
│ Decrement (at B): N.B += 1                             │
│                                                         │
│ Merge: Element-wise max for both P and N               │
└───────────────────────────────────────────────────────────┘
```

```
LWW-Register (Last-Writer-Wins Register):

┌─────────────────────────────────────────────────────────┐
│ Structure:                                              │
│ - Value with timestamp                                  │
│ - On conflict, highest timestamp wins                   │
│                                                         │
│ Register A: {value: "hello", timestamp: 100}           │
│ Register B: {value: "world", timestamp: 99}            │
│                                                         │
│ merge(A, B):                                            │
│   A.timestamp (100) > B.timestamp (99)                 │
│   Result: {value: "hello", timestamp: 100}             │
│                                                         │
│ Problem: Clock skew can cause data loss!                │
│ Solution: Use logical clocks or hybrid logical clocks   │
└───────────────────────────────────────────────────────────┘
```

```
OR-Set (Observed-Remove Set):

┌─────────────────────────────────────────────────────────┐
│ Structure:                                              │
│ - Each element has unique tag (element, unique_id)      │
│ - Add: Create new tag                                   │
│ - Remove: Track removed tags                            │
│                                                         │
│ Set state: (elements, tombstones)                       │
│                                                         │
│ Add "apple" at Node A:                                  │
│   elements: {("apple", A1)}                            │
│   tombstones: {}                                        │
│                                                         │
│ Add "banana" at Node B:                                 │
│   elements: {("banana", B1)}                           │
│   tombstones: {}                                        │
│                                                         │
│ Merge:                                                  │
│   elements = union of both element sets                 │
│   tombstones = union of both tombstone sets             │
│   Result elements = elements - tombstones               │
│                                                         │
│ Concurrent add/remove:                                  │
│ - Node A adds "apple" with tag A2                       │
│ - Node B removes "apple" (tag A1)                       │
│ - After merge: "apple" still present (A2 not tombstoned)│
│ - This is correct! Add happened concurrently            │
└───────────────────────────────────────────────────────────┘
```

### CRDT Text Implementation

```
RGA (Replicated Growable Array) for Text:

┌─────────────────────────────────────────────────────────┐
│ Structure for collaborative text editing:               │
│                                                         │
│ Each character has:                                     │
│ - value: the character                                  │
│ - id: unique identifier (node_id, sequence_number)      │
│ - parent_id: id of previous character                   │
│ - visibility: true/false (tombstone for deleted)        │
│                                                         │
│ Initial state:                                          │
│ [ROOT]                                                 │
│                                                         │
│ Type "cat":                                             │
│ [ROOT] -> [(c, A1, ROOT)] -> [(a, A2, A1)] -> [(t, A3, A2)]
│                                                         │
│ Insert "h" after "c" (at Node B, concurrently):         │
│ [ROOT] -> [(c, A1, ROOT)] -> [(h, B1, A1)] -> [(a, A2, A1)] ...
│                                                         │
│ Delete "a" (at Node C):                                 │
│ - Mark (a, A2, A1) as visibility: false                 │
│ - Don't remove from structure (maintains ordering)      │
│                                                         │
│ Display text:                                           │
│ - Traverse from ROOT                                    │
│ - Collect visible characters in order                   │
│ - "cht" (a is hidden)                                   │
└───────────────────────────────────────────────────────────┘

Implementation:

```rust
#[derive(Debug, Clone)]
struct Character {
    value: char,
    id: (NodeId, u64),  // (node_id, sequence_number)
    parent_id: Option<(NodeId, u64)>,
    visible: bool,
}

#[derive(Debug)]
struct CRDTText {
    characters: Vec<Character>,
    local_counter: u64,
    node_id: NodeId,
}

impl CRDTText {
    fn insert(&mut self, after_id: (NodeId, u64), value: char) {
        self.local_counter += 1;
        let new_char = Character {
            value,
            id: (self.node_id, self.local_counter),
            parent_id: Some(after_id),
            visible: true,
        };

        // Find insertion point (topological sort)
        self.insert_in_order(new_char);
    }

    fn delete(&mut self, char_id: (NodeId, u64)) {
        if let Some(c) = self.characters.iter_mut()
            .find(|c| c.id == char_id) {
            c.visible = false;  // Tombstone
        }
    }

    fn to_string(&self) -> String {
        self.characters.iter()
            .filter(|c| c.visible)
            .map(|c| c.value)
            .collect()
    }

    fn merge(&mut self, other: &CRDTText) {
        // Merge characters from other replica
        for char in &other.characters {
            self.insert_in_order(char.clone());
        }
    }

    fn insert_in_order(&mut self, new_char: Character) {
        // Check if already exists
        if self.characters.iter().any(|c| c.id == new_char.id) {
            return;  // Already present
        }

        // Topological insertion based on parent_id
        // ... (implementation details)
        self.characters.push(new_char);
    }
}
```
```

## Part 3: SQL Sync Protocol

### The SQL Sync Pattern

```
SQL Sync Architecture:

┌─────────────────────────────────────────────────────────┐
│                                                         │
│  Client (SQLite)              Server (PostgreSQL)       │
│  ┌─────────────┐              ┌─────────────┐          │
│  │ Local DB    │◄────────────►│ Central DB  │          │
│  │             │   Sync       │             │          │
│  │ - Tables    │   Protocol   │ - Tables    │          │
│  │ - CRDT cols │              │ - CRDT cols │          │
│  └─────────────┘              └─────────────┘          │
│        ▲                              ▲                 │
│        │                              │                 │
│  ┌─────────────┐              ┌─────────────┐          │
│  │ React App   │              │ Sync Server │          │
│  │             │              │             │          │
│  │ SELECT/INSERT│             │ Merge &     │          │
│  │ (local)     │              │ Broadcast   │          │
│  └─────────────┘              └─────────────┘          │
│                                                         │
│  Benefits:                                              │
│  - Local-first: app works offline                       │
│  - Instant UI: no network latency for local ops         │
│  - Automatic sync: background reconciliation            │
│  - Standard SQL: use existing skills                    │
└───────────────────────────────────────────────────────────┘
```

```
CRDT-Enabled Table Schema:

┌─────────────────────────────────────────────────────────┐
│ Standard Table:                                         │
│ CREATE TABLE documents (                                │
│   id UUID PRIMARY KEY,                                  │
│   title TEXT,                                           │
│   content TEXT,                                         │
│   updated_at TIMESTAMP                                  │
│ );                                                      │
│                                                         │
│ Problem: How to merge concurrent updates?               │
│                                                         │
│ CRDT-Enabled Table:                                     │
│ CREATE TABLE documents (                                │
│   id UUID PRIMARY KEY,                                  │
│   title TEXT,                                           │
│   title_actor BLOB,        -- CRDT metadata             │
│   content TEXT,                                         │
│   content_actor BLOB,      -- CRDT metadata             │
│   created_at TIMESTAMP,                                 │
│   created_by UUID                                       │
│ );                                                      │
│                                                         │
│ Or with LWW (simpler but less powerful):               │
│ CREATE TABLE documents (                                │
│   id UUID PRIMARY KEY,                                  │
│   title TEXT,                                           │
│   title_timestamp BIGINT,  -- For conflict resolution   │
│   content TEXT,                                         │
│   content_timestamp BIGINT,                             │
│   row_version BLOB           -- Vector clock            │
│ );                                                      │
└───────────────────────────────────────────────────────────┘
```

### Sync Protocol Flow

```
Initial Sync (Client connects for first time):

┌─────────────────────────────────────────────────────────┐
│ Client                              Server              │
│    │                                 │                  │
│    │ 1. SYNC_REQUEST {               │                  │
│    │      client_clock: {},          │                  │
│    │      tables: ["documents"]      │                  │
│    │    }                            │                  │
│    │────────────────────────────────>│                  │
│    │                                 │                  │
│    │ 2. Query changes since client_clock                │
│    │    SELECT * FROM documents                        │
│    │    WHERE row_clock > client_clock                 │
│    │                                 │                  │
│    │ 3. SYNC_RESPONSE {              │                  │
│    │      changes: [                 │                  │
│    │        {table: "documents",     │                  │
│    │         op: "INSERT",           │                  │
│    │         row: {...},             │                  │
│    │         clock: {A: 5, B: 3}}    │                  │
│    │      ],                         │                  │
│    │      server_clock: {A: 10}      │                  │
│    │    }                            │                  │
│    │<────────────────────────────────│                  │
│    │                                 │                  │
│    │ 4. Apply changes to local SQLite                   │
│    │    INSERT INTO documents VALUES (...)             │
└───────────────────────────────────────────────────────────┘
```

```
Continuous Sync (Client makes local change):

┌─────────────────────────────────────────────────────────┐
│ Client                              Server              │
│    │                                 │                  │
│    │ User types in editor            │                  │
│    │                                 │                  │
│    │ 1. INSERT INTO documents       │                  │
│    │    (id, title, content,         │                  │
│    │     title_timestamp)            │                  │
│    │    VALUES (..., ..., ..., now())│                  │
│    │                                 │                  │
│    │ 2. Record change in local log   │                  │
│    │    INSERT INTO change_log       │                  │
│    │    (table, op, row_id, clock)   │                  │
│    │    VALUES (...)                 │                  │
│    │                                 │                  │
│    │ 3. SYNC_UPLOAD {                │                  │
│    │      changes: [...]             │                  │
│    │    }                            │                  │
│    │────────────────────────────────>│                  │
│    │                                 │                  │
│    │ 4. Merge changes (CRDT merge)   │                  │
│    │    - Apply insert               │                  │
│    │    - Handle conflicts            │                  │
│    │                                 │                  │
│    │ 5. BROADCAST to other clients   │                  │
│    │    (push via WebSocket)         │                  │
│    │                                 │                  │
│    │ 6. SYNC_ACK {                   │                  │
│    │      accepted: [...],           │                  │
│    │      server_clock: {A: 11}      │                  │
│    │    }                            │                  │
│    │<────────────────────────────────│                  │
└───────────────────────────────────────────────────────────┘
```

### Conflict Resolution Strategies

```
Strategy Comparison:

┌─────────────────────────────────────────────────────────┐
│ Strategy         │ Pros              │ Cons             │
├─────────────────────────────────────────────────────────┤
│ Last-Writer-Wins │ Simple to         │ Can lose data    │
│ (LWW)            │ implement         │ on clock skew    │
│                  │ Low overhead      │ Not merge-aware  │
├─────────────────────────────────────────────────────────┤
│ CRDTs            │ Guaranteed        │ Higher storage   │
│ (RGA, OR-Set)    │ convergence       │ overhead         │
│                  │ No data loss      │ Complex to       │
│                  │                   │ implement        │
├─────────────────────────────────────────────────────────┤
│ Operational      │ Fine-grained      │ Requires         │
│ Transforms (OT)  │ (character-level) │ central server   │
│                  │ Industry standard │ Complex math     │
├─────────────────────────────────────────────────────────┤
│ Manual Merge     │ Application-aware │ Requires user    │
│                  │ Can handle any    │ intervention     │
│                  │ conflict          │ UX complexity    │
└───────────────────────────────────────────────────────────┘
```

```
LWW Implementation with SQL Triggers:

-- Table with LWW columns
CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    title TEXT,
    title_ts INTEGER,  -- Unix timestamp in microseconds
    content TEXT,
    content_ts INTEGER,
    _version INTEGER DEFAULT 0
);

-- Trigger for conflict-free updates
CREATE TRIGGER update_note_title
BEFORE UPDATE ON notes
FOR EACH ROW
WHEN NEW.title_ts <= OLD.title_ts
BEGIN
    SELECT RAISE(IGNORE);  -- Skip update, newer value exists
END;

-- Application-side merge
UPDATE notes
SET title = COALESCE(
        CASE WHEN title_ts < NEW.title_ts
             THEN NEW.title
             ELSE title END,
        NEW.title
    ),
    title_ts = GREATEST(title_ts, NEW.title_ts)
WHERE id = NEW.id;
```

## Part 4: Building with GoatPlatform

### goatdb Architecture

```
goatdb Component Architecture:

┌─────────────────────────────────────────────────────────┐
│ Application Layer                                       │
│ ┌─────────────────────────────────────────────────────┐│
│ │ SQL Interface (SQLite-compatible)                   ││
│ │ - SELECT, INSERT, UPDATE, DELETE                    ││
│ │ - Prepared statements                               ││
│ └─────────────────────────────────────────────────────┘│
│                          │                              │
│ ┌─────────────────────────────────────────────────────┐│
│ │ Sync Engine                                         ││
│ │ - Change detection                                  ││
│ │ - CRDT merge                                        ││
│ │ - Conflict resolution                               ││
│ └─────────────────────────────────────────────────────┘│
│                          │                              │
│ ┌─────────────────────────────────────────────────────┐│
│ │ Storage Engine                                      ││
│ │ - B-Tree indexes                                    ││
│ │ - WAL (Write-Ahead Log)                             ││
│ │ - Page cache                                        ││
│ └─────────────────────────────────────────────────────┘│
│                          │                              │
│ ┌─────────────────────────────────────────────────────┐│
│ │ Network Layer                                       ││
│ │ - WebSocket connections                             ││
│ │ - Sync protocol encoding                            ││
│ │ - Broadcast to peers                                ││
│ └─────────────────────────────────────────────────────┘│
└───────────────────────────────────────────────────────────┘
```

### Basic Usage Pattern

```typescript
// Client-side with goatdb

import { goatdb } from '@goatplatform/goatdb';

// Initialize local database
const db = await goatdb.open('my-app-db');

// Create table with CRDT columns
await db.exec(`
  CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    title TEXT,
    title_actor TEXT,
    completed BOOLEAN DEFAULT FALSE,
    completed_actor TEXT,
    created_at INTEGER
  )
`);

// Insert (local, instant)
await db.run(`
  INSERT INTO tasks (id, title, created_at)
  VALUES (?, ?, ?)
`, ['task-1', 'Buy milk', Date.now()]);

// Query (local, instant)
const tasks = await db.all(`
  SELECT * FROM tasks ORDER BY created_at DESC
`);

// Sync happens automatically in background
// db.sync({ server: 'wss://sync.example.com' });
```

```typescript
// Sync server

import { createServer } from '@goatplatform/sqlsync-server';

const server = createServer({
  database: {
    type: 'postgres',
    connection: process.env.DATABASE_URL,
  },
  tables: ['tasks', 'users', 'projects'],

  // Optional: custom conflict resolution
  onConflict: async (table, local, remote) => {
    // Custom merge logic per table
    if (table === 'tasks' && local.title !== remote.title) {
      // Keep longer title (heuristic)
      return local.title.length > remote.title.length ? local : remote;
    }
    // Default: LWW by timestamp
    return local.updated_at > remote.updated_at ? local : remote;
  },
});

server.listen(3000);
```

---

*This document is part of the GoatPlatform exploration series. See [exploration.md](./exploration.md) for the complete index.*
