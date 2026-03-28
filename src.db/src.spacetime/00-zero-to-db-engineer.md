---
title: "Zero to Database Engineer: A First-Principles Journey Through SpacetimeDB"
subtitle: "Complete textbook-style guide from database fundamentals to in-memory storage and Rust replication"
based_on: "SpacetimeDB - In-Memory Database with Serverless Modules"
level: "Beginner to Intermediate - No prior database knowledge assumed"
---

# Zero to Database Engineer: First-Principles Guide

## Table of Contents

1. [What Are Databases?](#1-what-are-databases)
2. [Storage Engines Explained](#2-storage-engines-explained)
3. [Query Execution Fundamentals](#3-query-execution-fundamentals)
4. [Distributed Consensus Basics](#4-distributed-consensus-basics)
5. [In-Memory Database Architecture](#5-in-memory-database-architecture)
6. [Your Learning Path](#6-your-learning-path)

---

## 1. What Are Databases?

### 1.1 The Fundamental Question

**What is a database?**

A database is a system for:
1. **Storing** data persistently (survives restarts)
2. **Organizing** data efficiently (tables, indexes, relationships)
3. **Retrieving** data quickly (queries, filters, joins)
4. **Modifying** data safely (transactions, ACID guarantees)

```
┌─────────────────────────────────────────────────────────┐
│                    Database System                       │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐          │
│  │  Store   │ -> │  Query   │ -> │  Return  │          │
│  │ (Write)  │    │  (Read)  │    │ (Result) │          │
│  └──────────┘    └──────────┘    └──────────┘          │
│       ^                                   |             │
│       └────────── Disk/Memory ────────────┘             │
└─────────────────────────────────────────────────────────┘
```

**Real-world analogy:** A library

| Aspect | Library | Database |
|--------|---------|----------|
| Storage | Bookshelves | Tables on disk |
| Organization | Dewey Decimal System | Indexes, schemas |
| Retrieval | Librarian search | SQL queries |
| Modification | Adding/removing books | INSERT/UPDATE/DELETE |

### 1.2 Why Databases Instead of Files?

You could store data in plain files:

```python
# Naive file-based approach
with open("users.txt", "a") as f:
    f.write(f"{user_id},{name},{email}\n")

# To find a user:
with open("users.txt", "r") as f:
    for line in f:
        if line.startswith(f"{user_id},"):
            return line  # Slow O(n) scan!
```

**Problems with files:**
- **Slow lookups** - must scan entire file (O(n))
- **No concurrent access** - two writers corrupt data
- **No crash recovery** - partial writes corrupt data
- **No query language** - custom parsing for every query
- **No indexing** - can't optimize common queries

**Databases solve these problems:**
- **Fast lookups** - B-trees, hash indexes (O(log n) or O(1))
- **Concurrent access** - transactions, locks, MVCC
- **Crash recovery** - write-ahead logging (WAL)
- **Query language** - SQL, relational algebra
- **Indexing** - automatic index selection

### 1.3 Types of Databases

| Type | Description | Examples | Use Cases |
|------|-------------|----------|-----------|
| **Relational (OLTP)** | Tables, rows, SQL, ACID | PostgreSQL, MySQL, SQLite | Web apps, transactions |
| **Relational (OLAP)** | Columnar, analytics | Snowflake, Redshift | Data warehouses |
| **Document** | JSON documents | MongoDB, CouchDB | Content management |
| **Key-Value** | Simple key-value pairs | Redis, DynamoDB | Caching, sessions |
| **Column-Family** | Wide columns | Cassandra, HBase | Time series, IoT |
| **Graph** | Nodes, edges | Neo4j, Dgraph | Social networks |
| **In-Memory** | RAM-first storage | SpacetimeDB, Redis | Real-time apps |

### 1.4 SpacetimeDB's Niche

SpacetimeDB is unique because it combines:
1. **In-memory storage** - All state in RAM for speed
2. **WAL persistence** - Crash recovery via commitlog
3. **Server-side modules** - Logic runs IN the database
4. **Real-time sync** - Clients subscribe to queries

```
Traditional Stack:
┌─────────┐    ┌─────────┐    ┌─────────┐
│ Client  │ -> │  Server │ -> │  DB     │
│ (React) │ <- │  (Node) │ <- │ (Postgres)│
└─────────┘    └─────────┘    └─────────┘

SpacetimeDB Stack:
┌─────────┐    ┌─────────────────┐
│ Client  │ -> │  SpacetimeDB    │
│ (React) │ <- │  (DB + Server)  │
└─────────┘    └─────────────────┘
```

---

## 2. Storage Engines Explained

### 2.1 What Is a Storage Engine?

A **storage engine** is the component responsible for:
- How data is **laid out on disk** (pages, files)
- How data is **organized in memory** (B-trees, LSM trees)
- How data is **written durably** (WAL, fsync)
- How data is **recovered after crashes** (replay, checkpoints)

### 2.2 B-Trees: The Classic Approach

**B-Trees** (Balanced Trees) are used by PostgreSQL, MySQL, SQLite:

```
           [50]
          /    \
      [20]      [80]
     /   \      /   \
   [10][30]  [60][90]
```

**Properties:**
- **Balanced** - All leaf nodes at same depth
- **Sorted** - Keys in order for range queries
- **Self-balancing** - Automatic rebalancing on insert/delete

**Operations:**
| Operation | Complexity | Notes |
|-----------|------------|-------|
| Point lookup | O(log n) | Find exact key |
| Range scan | O(log n + k) | k = results |
| Insert | O(log n) | May split nodes |
| Delete | O(log n) | May merge nodes |

**B-Tree Page Structure:**
```
┌────────────────────────────────────┐
│ Page Header (16 bytes)             │
│ - Page type (leaf/internal)        │
│ - Number of cells                  │
│ - Free space offset                │
├────────────────────────────────────┤
│ Cell Pointers (4 bytes each)       │
│ - Offset to cell data              │
├────────────────────────────────────┤
│ Free Space                           │
├────────────────────────────────────┤
│ Cell Data (variable size)          │
│ - Key + Value pairs                │
└────────────────────────────────────┘
```

### 2.3 LSM Trees: The Modern Approach

**LSM Trees** (Log-Structured Merge Trees) are used by RocksDB, Cassandra, LevelDB:

```
Memory (MemTable)
┌──────────────┐
│ Write buffer │ -> Full -> Flush to disk as SSTable
│ (in-memory)  │
└──────────────┘

Disk (SSTables)
Level 0: [SST0] [SST1]  (newest, unsorted between files)
Level 1: [SST2]         (sorted, compacted)
Level 2: [SST3]         (older, more compacted)
```

**How LSM Works:**
1. **Write** -> MemTable (fast, in-memory)
2. **MemTable full** -> Flush to SSTable (sequential disk write)
3. **Background compaction** -> Merge SSTables, remove duplicates/deletes

**Operations:**
| Operation | Complexity | Notes |
|-----------|------------|-------|
| Point lookup | O(log n) | Check memtable + SSTables |
| Range scan | O(n/m) | m = SSTable size |
| Write | O(1) | Just memtable + WAL |
| Delete | O(1) | Tombstone marker |

**LSM vs B-Tree:**

| Aspect | B-Tree | LSM Tree |
|--------|--------|----------|
| **Write throughput** | Good (random I/O) | Excellent (sequential I/O) |
| **Read latency** | Consistent | Variable (depends on compaction) |
| **Space amplification** | Low | Higher (multiple copies) |
| **Write amplification** | Low | Higher (compaction rewrites) |
| **Best for** | Read-heavy | Write-heavy |

### 2.4 Write-Ahead Logging (WAL)

**Problem:** How to recover from crashes without corrupting data?

**Solution:** Write-Ahead Logging (WAL)

```
Before modifying data:
1. Write operation to WAL (durable)
2. Modify in-memory data
3. Acknowledge to client

On crash recovery:
1. Load last checkpoint
2. Replay WAL from checkpoint
3. Restore consistent state
```

**WAL Format:**
```
┌────────────────────────────────────┐
│ WAL Header                         │
│ - Magic number                     │
│ - Page size                        │
│ - Checkpoint info                  │
├────────────────────────────────────┤
│ Frame 1                            │
│ - Page number                      │
│ - Frame checksum                   │
│ - Page data (4KB)                  │
├────────────────────────────────────┤
│ Frame 2                            │
│ ...                                │
└────────────────────────────────────┘
```

**SpacetimeDB's Commitlog:**
```rust
// Simplified SpacetimeDB commitlog structure
struct Commitlog {
    path: PathBuf,
    writer: BufWriter<File>,
    current_offset: u64,
}

impl Commitlog {
    fn append(&mut self, record: CommitRecord) -> Result<u64> {
        // Write record with length prefix
        let bytes = record.serialize();
        self.writer.write_all(&(bytes.len() as u32).to_be_bytes())?;
        self.writer.write_all(&bytes)?;
        self.writer.flush()?;  // Ensure durability
        Ok(self.current_offset)
    }

    fn replay(&mut self, from_offset: u64) -> Result<Vec<CommitRecord>> {
        // Read records from offset
        // Reconstruct state
    }
}
```

### 2.5 SpacetimeDB's Hybrid Approach

SpacetimeDB uses **in-memory tables** with **commitlog persistence**:

```
┌─────────────────────────────────────────┐
│ In-Memory State (all tables)            │
│ ┌─────────┐ ┌─────────┐ ┌─────────┐    │
│ │ Table A │ │ Table B │ │ Table C │    │
│ │ (rows)  │ │ (rows)  │ │ (rows)  │    │
│ └─────────┘ └─────────┘ └─────────┘    │
└─────────────────────────────────────────┘
              │
              │ Every mutation logged
              ▼
┌─────────────────────────────────────────┐
│ Commitlog (WAL on disk)                 │
│ [Txn1][Txn2][Txn3]...                   │
└─────────────────────────────────────────┘
```

**Key insight:** By keeping ALL state in memory, SpacetimeDB avoids disk I/O for reads. The commitlog is only for recovery, not for serving queries.

---

## 3. Query Execution Fundamentals

### 3.1 The Query Pipeline

```
SQL Query -> Parser -> AST -> Planner -> Optimizer -> Physical Plan -> Executor -> Results
```

**Step by step:**

1. **Parser** - Converts SQL text to Abstract Syntax Tree (AST)
2. **Analyzer** - Validates table/column names, types
3. **Planner** - Creates logical query plan
4. **Optimizer** - Chooses best physical plan
5. **Executor** - Runs the plan, returns results

### 3.2 Parsing Example

```sql
SELECT users.name, orders.total
FROM users
JOIN orders ON users.id = orders.user_id
WHERE users.country = 'US'
ORDER BY orders.total DESC
LIMIT 10;
```

**AST Representation:**
```
SelectStatement
├── projections: [name, total]
├── from: users
├── joins: [
│   └── JOIN orders ON users.id = orders.user_id
│]
├── where: users.country = 'US'
├── order_by: [orders.total DESC]
└── limit: 10
```

### 3.3 Logical vs Physical Plans

**Logical Plan** (what to do):
```
Limit(10)
  └── Sort(orders.total DESC)
      └── Filter(users.country = 'US')
          └── Join(users.id = orders.user_id)
              ├── Scan(users)
              └── Scan(orders)
```

**Physical Plan** (how to do it):
```
LimitExec(10)
  └── SortExec(key=total, desc=true)
      └── FilterExec(predicate=country = 'US')
          └── HashJoinExec(left_key=user_id, right_key=id)
              ├── TableScanExec(users)
              └── TableScanExec(orders)
```

### 3.4 Query Optimization

**Question:** Why not just execute the logical plan directly?

**Answer:** Many ways to execute the same query, vastly different performance!

**Example: Join Ordering**
```sql
SELECT * FROM A JOIN B ON A.id = B.a_id JOIN C ON B.id = C.b_id
WHERE A.x = 1;
```

**Bad plan:**
```
Join(B, C) -> 1M rows
  Join(A)  -> 10K rows (after filtering A.x=1)
Cost: O(1M * log(10K))
```

**Good plan:**
```
Filter(A.x=1) -> 10K rows
  Join(B)     -> 100K rows
    Join(C)   -> 10K rows
Cost: O(10K * log(100K))
```

**Optimizer techniques:**
- **Cost estimation** - Cardinality, I/O, CPU
- **Dynamic programming** - Find optimal join order
- **Heuristics** - Push filters down, eliminate redundant ops

### 3.5 SpacetimeDB's Incremental View Maintenance

SpacetimeDB uses **incremental view maintenance** for subscriptions:

```
Client subscribes to: SELECT * FROM users WHERE active = true

Initial sync:
- Run full query
- Send all matching rows

On INSERT INTO users:
- Check if new row matches WHERE
- If yes, send INSERT to client

On UPDATE users SET active = false WHERE id = 5:
- Check if row still matches
- If no, send DELETE to client
```

**Why this matters:** Instead of re-running the full query on every change, SpacetimeDB incrementally computes the difference.

---

## 4. Distributed Consensus Basics

### 4.1 The Consensus Problem

**Scenario:** Multiple database nodes must agree on the order of operations.

```
Client: "INSERT INTO users VALUES (1, 'Alice')"
       /         |         \
      /          |          \
   Node A     Node B      Node C

What if Node A receives it at 10:00:00, Node B at 10:00:01?
What if Node A crashes before telling others?
```

**Requirements:**
1. **Agreement** - All non-faulty nodes decide on same value
2. **Validity** - If all propose same value, that's decided
3. **Termination** - All non-faulty nodes eventually decide

### 4.2 CAP Theorem

You can only have 2 of 3:

| Property | Description | Trade-off |
|----------|-------------|-----------|
| **Consistency** | All nodes see same data | vs Availability |
| **Availability** | Every request gets response | vs Consistency |
| **Partition Tolerance** | System works despite network failures | Required for distributed |

**SpacetimeDB's choice:** CP (Consistency + Partition Tolerance)
- Sacrifices availability during network partitions
- Ensures all clients see consistent state

### 4.3 Raft Consensus

**Raft** is a consensus algorithm used by etcd, CockroachDB:

```
State Machine:
┌─────────────┐
│   Leader    │ <- Handles all writes
├─────────────┤
│ Follower 1  │ <- Replicate from leader
│ Follower 2  │
└─────────────┘

Term-based election:
1. Followers elect leader
2. Leader sends heartbeats
3. If no heartbeat, new election
```

**Raft Log Replication:**
```
Client: "SET x = 1"
Leader: Append to log[5] = "SET x = 1"
       Replicate to followers
       Wait for majority ACK
       Apply to state machine
       Respond to client
```

### 4.4 SpacetimeDB's Replication

SpacetimeDB uses a **leader-based replication** model:

```
┌─────────────┐
│  Leader     │ <- Receives writes, replicates
├─────────────┤
│ Replica 1   │ <- Receive from leader
│ Replica 2   │
└─────────────┘

Write path:
1. Client sends to leader
2. Leader appends to commitlog
3. Leader replicates to followers
4. Majority ACK -> commit
5. Respond to client
```

---

## 5. In-Memory Database Architecture

### 5.1 Why In-Memory?

**Traditional databases** are optimized for disk:
- Data on disk, cached in RAM
- Optimized for minimizing disk I/O
- Block-based access (4KB pages)

**In-memory databases** assume data fits in RAM:
- All data in RAM
- Optimized for CPU cache efficiency
- Row/column-based access

**Performance comparison:**
| Operation | Disk-based (PostgreSQL) | In-memory (SpacetimeDB) |
|-----------|------------------------|------------------------|
| Point lookup | ~1ms (disk seek) | ~100ns (RAM) |
| Range scan | ~10ms (multiple seeks) | ~1ms (sequential RAM) |
| Write | ~5ms (fsync) | ~100ns (RAM) |

### 5.2 SpacetimeDB's Memory Layout

```
In-Memory Structure:
┌────────────────────────────────────────┐
│ Table Space                            │
│ ┌──────────┬──────────┬──────────┐    │
│ │ Table A  │ Table B  │ Table C  │    │
│ │          │          │          │    │
│ │ - Rows   │ - Rows   │ - Rows   │    │
│ │ - Index  │ - Index  │ - Index  │    │
│ └──────────┴──────────┴──────────┘    │
└────────────────────────────────────────┘

Each Table:
┌────────────────────────────────────────┐
│ Table                                  │
│ ├── Schema (column names, types)       │
│ ├── Row Data (Vec<Row>)                │
│ ├── Indexes (B-tree or hash)           │
│ └── Constraints (unique, FK)           │
└────────────────────────────────────────┘
```

### 5.3 Crash Recovery

```
Startup sequence:
1. Load last snapshot (if exists)
2. Find commitlog start position from snapshot
3. Replay commitlog from that position
4. Rebuild indexes
5. Ready for connections
```

**Snapshot optimization:**
- Periodic snapshots to avoid replaying entire log
- Snapshot contains: all table data + commitlog offset
- Only replay logs after snapshot

---

## 6. Your Learning Path

### 6.1 Prerequisites

| Topic | What to Know | Resources |
|-------|--------------|-----------|
| **Basic programming** | Variables, loops, functions | Any intro programming course |
| **Data structures** | Arrays, trees, hash maps | "Grokking Algorithms" |
| **Systems basics** | Files, memory, processes | "Computer Systems: A Programmer's Perspective" |

### 6.2 Recommended Reading Order

1. **Start here:** [00-zero-to-db-engineer.md](00-zero-to-db-engineer.md) (this document)
2. **Storage internals:** [01-storage-engine-deep-dive.md](01-storage-engine-deep-dive.md)
3. **Query execution:** [02-query-execution-deep-dive.md](02-query-execution-deep-dive.md)
4. **Distributed systems:** [03-consensus-replication-deep-dive.md](03-consensus-replication-deep-dive.md)
5. **Rust translation:** [rust-revision.md](rust-revision.md)
6. **Production deployment:** [production-grade.md](production-grade.md)
7. **Serverless Lambda:** [04-valtron-integration.md](04-valtron-integration.md)

### 6.3 Hands-On Exercises

**Exercise 1: Build a simple key-value store**
```rust
struct KeyValueStore {
    data: HashMap<String, String>,
}

impl KeyValueStore {
    fn get(&self, key: &str) -> Option<&String> { ... }
    fn set(&mut self, key: String, value: String) { ... }
    fn delete(&mut self, key: &str) -> bool { ... }
}
```

**Exercise 2: Add WAL persistence**
```rust
struct PersistentKV {
    store: KeyValueStore,
    wal: File,
}

impl PersistentKV {
    fn set(&mut self, key: String, value: String) {
        // 1. Write to WAL
        self.wal.write_all(format!("SET {} {}\n", key, value).as_bytes());
        // 2. Update in-memory store
        self.store.set(key, value);
    }
}
```

**Exercise 3: Add B-tree index**
```rust
struct BTreeIndex {
    tree: BTreeMap<i64, RowId>,
}

impl Index for BTreeIndex {
    fn insert(&mut self, key: i64, row_id: RowId) { ... }
    fn search(&self, key: i64) -> Vec<RowId> { ... }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial zero-to-engineer guide created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
