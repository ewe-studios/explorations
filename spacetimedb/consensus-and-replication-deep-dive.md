---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/SpacetimeDB/
explored_at: 2026-03-23
type: deep-dive
scope: consensus-and-replication
---

# SpacetimeDB Consensus and Replication Deep Dive

This document provides implementation-level detail on SpacetimeDB's replication model, consensus protocol, reducer execution, subscription system, durability guarantees, and efficiency comparisons. Every section references actual source code from the SpacetimeDB repository.

---

## 1. Replication Architecture Overview

### 1.1 Leader-Based Replication Model

SpacetimeDB uses a **leader-based replication model** rather than a full consensus protocol like Raft or Paxos. The architecture consists of:

- **Control Database**: A central coordinator (sled-based key-value store) that tracks replicas, nodes, and leader assignment
- **Leader Replica**: One replica per database is designated as the leader, handling all writes
- **Follower Replicas**: Read-only replicas that receive updates from the leader
- **Nodes**: Physical machines that host replicas

**Source:** `crates/core/src/messages/control_db.rs`, `crates/standalone/src/control_db.rs`

```rust
// Replica structure in control database
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Replica {
    pub id: u64,           // Internal replica ID
    pub database_id: u64,  // Which database this replica hosts
    pub node_id: u64,      // Which node hosts this replica
    pub leader: bool,      // Is this the leader replica?
}
```

### 1.2 Control Database Schema

The control database (implemented using sled) maintains several key collections:

```
Control DB Trees:
├── "dns"                    # Domain name -> Identity mappings
├── "reverse_dns"            # Identity -> Domain names
├── "database"               # Database definitions
├── "replica"                # Replica assignments
├── "node"                   # Node registrations
├── "energy_balance"         # Per-identity energy balances
└── "replication_state"      # Replication progress tracking
```

**Source:** `crates/standalone/src/control_db.rs`

```rust
pub struct ControlDb {
    db: sled::Db,  // Embedded KV store
}

// Key operations:
impl ControlDb {
    pub fn get_leader_replica_by_database(&self, database_id: u64) -> Option<Replica>
    pub fn get_replicas_by_database(&self, database_id: u64) -> Result<Vec<Replica>>
    pub fn insert_replica(&self, replica: Replica) -> Result<u64>
    pub fn delete_replica(&self, replica_id: u64) -> Result<()>
}
```

### 1.3 Replica Context

Each running database instance is represented by a `ReplicaContext`:

**Source:** `crates/core/src/replica_context.rs`

```rust
/// A "live" database.
#[derive(Clone)]
pub struct ReplicaContext {
    pub database: Database,         // Database metadata
    pub replica_id: u64,            // This replica's ID
    pub logger: Arc<DatabaseLogger>, // Commit log
    pub subscriptions: ModuleSubscriptions, // Active subscriptions
    pub relational_db: Arc<RelationalDB>,   // In-memory database
}
```

---

## 2. State Replication Model

### 2.1 How SpacetimeDB Replicates State

SpacetimeDB's replication is based on **commit log shipping** from leader to followers:

```
Leader Replica                          Follower Replica
┌─────────────────────┐                 ┌─────────────────────┐
│  Reducer executes   │                 │                     │
│  TxState created    │                 │                     │
│  Changes applied    │                 │                     │
│  Commitlog append   │───ship log──────▶│  Replay commitlog   │
│  Snapshot (periodic)│──ship snapshot──▶│  Apply snapshot     │
└─────────────────────┘                 └─────────────────────┘
```

**Key characteristics:**

1. **Single-writer model**: Only the leader accepts writes
2. **Commit log as source of truth**: All state changes are recorded in the commit log
3. **Snapshot-based bootstrapping**: New followers load from snapshots, then replay log
4. **Content-addressed storage**: Pages and blobs are identified by BLAKE3 hash

### 2.2 Consensus Protocol

SpacetimeDB does **not** implement a distributed consensus protocol like Raft or Paxos. Instead, it uses:

- **Centralized leader election**: The control database designates which replica is the leader
- **Quorum-based availability**: Multiple followers provide read availability and failover
- **Leader failover**: When a leader fails, the control database promotes a follower

**Source:** `smoketests/tests/replication.py`

```python
# Leader election test from replication tests
def test_leader_election_in_loop(self):
    """Fail a leader, wait for new leader to be elected and verify commits replicated"""
    iterations = 5
    for (first_id, second_id) in zip(row_ids[::2], row_ids[1::2]):
        cur_leader = self.cluster.wait_for_leader_change(None)
        self.cluster.ensure_leader_health(first_id)

        print(f"killing current leader: {cur_leader}")
        container_id = self.cluster.fail_leader()

        next_leader = self.cluster.wait_for_leader_change(cur_leader)
        self.assertNotEqual(cur_leader, next_leader)

        # Verify new leader accepts writes
        self.cluster.ensure_leader_health(second_id)
```

### 2.3 Leader Election Mechanism

Leader election is triggered by:

1. **Initial scheduling**: When a database is first published, a leader replica is created
2. **Leader failure**: Health checks detect leader unavailability
3. **Manual intervention**: `prefer_leader` control call

**Source:** `crates/standalone/src/lib.rs`

```rust
async fn leader(&self, database_id: u64) -> Result<Host, GetLeaderHostError> {
    let Some(leader) = self.control_db.get_leader_replica_by_database(database_id) else {
        return Err(GetLeaderHostError::NoSuchReplica);
    };

    let Some(database) = self.control_db.get_database_by_id(database_id)? else {
        return Err(GetLeaderHostError::NoSuchDatabase);
    };

    // Get or launch the module host for this leader
    self.host_controller
        .get_or_launch_module_host(database, leader.id)
        .await
        .map_err(|source| GetLeaderHostError::LaunchError { source })?;

    Ok(Host::new(leader.id, self.host_controller.clone()))
}

// Scheduling replicas (first one becomes leader)
async fn schedule_replicas(&self, database_id: u64, num_replicas: u8) -> Result<(), anyhow::Error> {
    for i in 0..num_replicas {
        let replica = Replica {
            id: 0,
            database_id,
            node_id: 0,  // Standalone: single node
            leader: i == 0,  // First replica is leader
        };
        self.insert_replica(replica).await?;
    }
    Ok(())
}
```

### 2.4 Consistency Guarantees

SpacetimeDB provides:

| Guarantee | Description |
|-----------|-------------|
| **Linearizable writes** | All writes go through the leader, providing a total order |
| **Eventual consistency for followers** | Followers may lag behind the leader |
| **Serializable transactions** | Single-writer model ensures serializability |
| **Durability** | Committed transactions persist via commit log |

---

## 3. Reducer Execution Model

### 3.1 Reducer Scheduling and Execution

Reducers are the primary mechanism for modifying database state. They execute as follows:

**Source:** `crates/core/src/host/module_host.rs`, `crates/datastore/src/locking_tx_datastore/`

```rust
// Reducer execution flow:
// 1. Client calls reducer via WebSocket
// 2. ModuleHost schedules reducer execution
// 3. Locking datastore acquires write lock
// 4. Reducer executes in WebAssembly runtime
// 5. TxState changes are committed or rolled back
// 6. Commit log is updated
// 7. Subscribers are notified

pub enum ReducerOutcome {
    Committed,
    BudgetExceeded,
    Failed(String),
}

pub struct ReducerCallResult {
    pub outcome: ReducerOutcome,
    pub timing: Duration,
    pub energy_used: u64,
}
```

### 3.2 Transaction Ordering Guarantees

**Source:** `crates/datastore/src/locking_tx_datastore/mod.rs`

```rust
/// Locking datastore with coarse-grained locking
pub struct Locking {
    committed_state: Arc<RwLock<CommittedState>>,
    sequence_state: Arc<Mutex<SequencesState>>,
    database_identity: Identity,
}

// Lock acquisition order (to prevent deadlocks):
// 1. committed_state (RwLock)
// 2. sequence_state (Mutex)
```

**Ordering guarantees:**

1. **Per-database total order**: Reducers for a single database execute sequentially
2. **No concurrent writes**: Write lock ensures only one reducer modifies state at a time
3. **FIFO within connection**: Reducer calls from a single connection are processed in order

### 3.3 Concurrency Control

SpacetimeDB uses a **two-state transaction model** with coarse-grained locking:

**Source:** `crates/datastore/src/locking_tx_datastore/tx_state.rs`

```rust
/// Per-transaction scratchpad
pub struct TxState {
    pub insert_tables: BTreeMap<TableId, Table>,   // New rows
    pub delete_tables: BTreeMap<TableId, DeleteTable>, // Deleted rows
    pub blob_store: HashMapBlobStore,               // New blobs
    pub pending_schema_changes: ThinVec<PendingSchemaChange>,
}

/// Two-state model:
/// - CommittedState: Canonical state, protected by RwLock
/// - TxState: Transaction scratchpad, discarded on rollback
```

**Concurrency characteristics:**

| Aspect | Implementation |
|--------|----------------|
| **Read transactions** | Multiple concurrent via `RwLock::read_lock()` |
| **Write transactions** | Exclusive via `RwLock::write_lock()` |
| **Conflict detection** | Not needed - single writer serializes all |
| **Deadlock prevention** | Fixed lock order |

### 3.4 The SquashedOffset Mechanism

**Source:** `crates/table/src/row_pointer.rs` (via storage-internals-deep-dive.md)

```rust
pub struct RowPointer(pub u64);
// Packs four fields into 64 bits:
// - Reserved bit: 1 bit (collision detection)
// - PageIndex: 39 bits (which page)
// - PageOffset: 16 bits (byte offset in page)
// - SquashedOffset: 8 bits (TX_STATE=0 or COMMITTED_STATE=1)

// The SquashedOffset distinguishes whether a pointer refers to
// the committed state table or the transaction scratchpad table.
```

This allows the query engine to seamlessly iterate over both committed and newly-inserted rows.

---

## 4. Subscription System

### 4.1 Subscription Architecture

**Source:** `crates/core/src/subscription/module_subscription_manager.rs`, `crates/subscription/`

```
Subscription Flow:
┌─────────────┐     ┌──────────────────────┐     ┌─────────────┐
│   Client    │────▶│ ModuleSubscription   │────▶│   Query     │
│  WebSocket  │     │     Manager          │     │   Planner   │
└─────────────┘     └──────────────────────┘     └─────────────┘
       ▲                       │                        │
       │                       │                        │
       │         ┌─────────────▼─────────────┐          │
       │         │    SubscriptionExecutor   │◀─────────┘
       │         │  - eval_delta()           │
       │         │  - UpdatesRelValue        │
       └─────────┴───────────────────────────┘
```

### 4.2 How Clients Subscribe

**Source:** `docs/docs/00200-core-concepts/00400-subscriptions.md`

```typescript
// TypeScript client example
const conn = DbConnection.builder()
  .withUri('wss://maincloud.spacetimedb.com')
  .withDatabaseName('my_module')
  .onConnect((ctx) => {
    ctx.subscriptionBuilder()
      .onApplied(() => {
        console.log('Subscription ready!');
      })
      .subscribe([tables.user, tables.message]);
  })
  .build();

// React to changes
conn.db.user.onInsert((ctx, user) => {
  console.log(`New user: ${user.name}`);
});
```

### 4.3 Delta Computation and Streaming

**Source:** `crates/subscription/src/delta.rs`

```rust
/// Evaluate a subscription plan against transaction changes
pub fn eval_delta<'a, Tx: Datastore + DeltaStore>(
    tx: &'a Tx,
    metrics: &mut ExecutionMetrics,
    plan: &SubscriptionPlan,
) -> Result<Option<UpdatesRelValue<'a>>> {
    let mut inserts = vec![];
    let mut deletes = vec![];

    if !plan.is_join() {
        // Single-table: iterate delta directly
        plan.for_each_insert(tx, metrics, &mut |row| {
            inserts.push(maybe_project(row)?);
            Ok(())
        })?;
        plan.for_each_delete(tx, metrics, &mut |row| {
            deletes.push(maybe_project(row)?);
            Ok(())
        })?;
    } else {
        // Join queries: track counts for bag semantics
        // Insert-delete cancellation for duplicate rows
        ...
    }

    Ok(Some(UpdatesRelValue { inserts, deletes }))
}
```

**Delta evaluation process:**

1. **Transaction commits**: `TxData` produced with inserts/deletes per table
2. **Subscription matching**: For each subscription, check if affected tables are subscribed
3. **Delta evaluation**: Run `eval_delta()` to compute actual changes
4. **Update packaging**: BSATN or JSON encode the changes
5. **WebSocket send**: Push updates to subscribed clients

### 4.4 Query Optimization for Subscriptions

**Source:** `crates/physical-plan/src/`, `crates/query-builder/`

Subscription queries are optimized through:

1. **Typed query builders**: Compile-time query construction (TypeScript, C#, Rust)
2. **Index utilization**: Queries use indexes when available
3. **Zero-copy subscriptions**: Duplicate subscriptions share state
4. **Bag semantics handling**: Join queries track match counts

**Zero-copy subscription optimization:**

```rust
// From subscriptions best practices:
// Subscribing to the same query more than once doesn't incur
// additional processing or serialization overhead.
```

---

## 5. Durability Guarantees

### 5.1 Write-Ahead Logging (Commit Log)

**Source:** `crates/commitlog/`, `crates/durability/`

The commit log is SpacetimeDB's durability layer:

```
Commit Log Structure:
┌──────────────────────────────────────────────┐
│ Segment Header (10 bytes)                     │
│  MAGIC: "(ds)^2"  (6 bytes)                   │
│  log_format_version: u8                       │
│  checksum_algorithm: u8                       │
│  reserved: [u8; 2]                            │
├──────────────────────────────────────────────┤
│ Commit 0                                      │
│ Commit 1                                      │
│ ...                                           │
└──────────────────────────────────────────────┘

Per-Commit Format:
┌──────────────────────────────────────────────┐
│ Commit Header (22 bytes)                      │
│  min_tx_offset: u64   (LE)                    │
│  epoch: u64           (LE)                    │
│  n: u16               (LE) -- num records     │
│  len: u32             (LE) -- payload bytes   │
├──────────────────────────────────────────────┤
│ Records (len bytes)                           │
│  Txdata<ProductValue> (BSATN encoded)         │
├──────────────────────────────────────────────┤
│ CRC32c checksum: u32 (LE)                     │
└──────────────────────────────────────────────┘

Total framing overhead: 26 bytes per commit
```

**Source:** `crates/commitlog/src/commit.rs`

```rust
pub fn write<W: Write>(&self, out: W) -> io::Result<u32> {
    let mut out = Crc32cWriter::new(out);
    out.write_all(&self.min_tx_offset.to_le_bytes())?;
    out.write_all(&epoch.to_le_bytes())?;
    out.write_all(&n.to_le_bytes())?;
    out.write_all(&len.to_le_bytes())?;
    out.write_all(&self.records)?;
    let crc = out.crc32c();
    let mut out = out.into_inner();
    out.write_all(&crc.to_le_bytes())?;
    Ok(crc)
}

// Decode with verification
pub fn decode<R: Read>(reader: R) -> io::Result<Option<Self>> {
    let mut reader = Crc32cReader::new(reader);
    let Some(hdr) = Header::decode_internal(&mut reader, v)? else {
        return Ok(None);  // EOF
    };
    let mut records = vec![0; hdr.len as usize];
    reader.read_exact(&mut records)?;
    let chk = reader.crc32c();
    let crc = decode_u32(reader.into_inner())?;
    if chk != crc {
        return Err(invalid_data(ChecksumMismatch));
    }
    Ok(Some(...))
}
```

### 5.2 Local Durability Implementation

**Source:** `crates/durability/src/imp/local.rs`

```rust
pub struct Local<T> {
    clog: Arc<Commitlog<Txdata<T>>>,
    durable_offset: watch::Receiver<Option<TxOffset>>,
    queue: mpsc::UnboundedSender<Transaction<Txdata<T>>>,
    queue_depth: Arc<AtomicU64>,
    shutdown: mpsc::Sender<ShutdownReply>,
    abort: AbortHandle,
}

impl<T: Encode + Send + Sync + 'static> Local<T> {
    pub fn open(
        replica_dir: ReplicaDir,
        rt: tokio::runtime::Handle,
        opts: Options,
        on_new_segment: Option<Arc<OnNewSegmentFn>>,
    ) -> Result<Self, OpenError> {
        // Lock the commitlog directory
        let lock = Lock::create(replica_dir.0.join("db.lock"))?;

        // Open commitlog
        let clog = Arc::new(Commitlog::open(
            replica_dir.commit_log(),
            opts.commitlog,
            on_new_segment,
        )?);

        // Spawn durability actor
        let abort = rt
            .spawn(Actor { ... }.run(txdata_rx, shutdown_rx))
            .abort_handle();

        Ok(Self { ... })
    }
}
```

### 5.3 Checkpointing and Snapshots

**Source:** `crates/snapshot/src/lib.rs`, `crates/core/src/db/snapshot.rs`

Snapshots are periodic point-in-time copies of the committed state:

```
Snapshot Directory Structure:
snapshot_dir/
  MAGIC: "txyz" (4 bytes)
  VERSION: u8
  MODULE_ABI_VERSION: [u16; 2]
  pages/
    <blake3_hash>.page  -- Content-addressed page files
  blobs/
    <blake3_hash>.blob  -- Content-addressed blob files
  snapshot.bsatn        -- Table metadata, schema, page assignments
```

**Snapshot characteristics:**

- **Frequency**: Every 1,000,000 transactions (configurable)
- **Content-addressed**: Pages and blobs identified by BLAKE3 hash
- **Incremental**: Only changed pages written (hardlink dedup)
- **Integrity verified**: Hash mismatches detect corruption

**Source:** `crates/snapshot/src/lib.rs`

```rust
pub const SNAPSHOT_FREQUENCY: u64 = 1_000_000;

// Recovery with snapshots:
// 1. Load latest snapshot at offset K
// 2. Replay commitlog from K+1 to current
// 3. Database state recovered
```

### 5.4 Recovery Procedures

**Crash recovery process:**

1. **Acquire lock**: `db.lock` file prevents concurrent access
2. **Find latest snapshot**: Scan for most recent snapshot directory
3. **Load snapshot**: Read content-addressed pages and blobs
4. **Replay commitlog**: Apply transactions after snapshot offset
5. **Verify integrity**: CRC32c checksums detect corruption
6. **Resume normal operation**: Accept new transactions

**Source:** `crates/commitlog/src/lib.rs` (via storage-internals-deep-dive.md)

```rust
// On startup (Generic::open):
// 1. List all segment files (sorted by offset)
// 2. Resume last segment for writing:
//    a. Read commits until checksum mismatch or EOF
//    b. Last valid commit determines resume point
//    c. Create new writer after last valid commit
// 3. If first commit in last segment corrupt, refuse to start
// 4. If no segments, start fresh from offset 0

// Commits with invalid checksums are silently truncated
// At most one commit (being written during crash) can be lost
```

---

## 6. Efficiency Comparisons

### 6.1 vs SQLite (Single-File, Embedded)

| Aspect | SpacetimeDB | SQLite |
|--------|-------------|--------|
| **Storage model** | In-memory pages + commit log | Disk-based B-tree |
| **Page size** | 64 KiB | 4 KiB (default) |
| **Concurrency** | Single-writer, multi-reader | Single-writer |
| **Row format** | BFLATN (aligned, direct access) | Cell-based |
| **Index types** | BTree, Hash, Direct | B-tree only |
| **Var-len storage** | Granule linked list | Overflow pages |
| **Blob threshold** | 992 bytes | ~page size |
| **Recovery** | Snapshot + commit log | WAL replay |
| **Real-time sync** | Built-in subscriptions | None |
| **Embedded logic** | WebAssembly modules | Triggers, extensions |

**Key differences:**

- SpacetimeDB is **memory-first** - all data lives in memory for fast access
- SQLite is **disk-first** - data persists on disk, cached in memory
- SpacetimeDB adds **real-time subscriptions** and **reducer execution**
- Both use **WAL-like durability** (commit log vs WAL)

### 6.2 vs PostgreSQL (Full RDBMS)

| Aspect | SpacetimeDB | PostgreSQL |
|--------|-------------|------------|
| **Storage model** | In-memory pages | Heap tables + TOAST |
| **Page size** | 64 KiB | 8 KiB (default) |
| **Concurrency** | Single-writer per DB | MVCC, multi-writer |
| **Transaction isolation** | Serializable | Read Committed, Repeatable Read, Serializable |
| **Index types** | BTree, Hash, Direct | BTree, Hash, GIN, GiST, BRIN, SP-GiST |
| **Var-len storage** | 64-byte granules | TOAST (out-of-line) |
| **Replication** | Leader-based, log shipping | Streaming, logical replication |
| **Query execution** | In-process (Wasm) | Separate server process |
| **Real-time** | Built-in subscriptions | LISTEN/NOTIFY, logical decoding |

**Key differences:**

- PostgreSQL supports **true concurrent writes** via MVCC
- SpacetimeDB runs **application logic inside the database**
- SpacetimeDB provides **automatic real-time sync** to clients
- PostgreSQL has more **mature query optimization**

### 6.3 vs FoundationDB (Distributed KV)

| Aspect | SpacetimeDB | FoundationDB |
|--------|-------------|--------------|
| **Data model** | Relational tables | Sorted key-value tuples |
| **Consistency** | Serializable (single-writer) | Strict serializable |
| **Replication** | Leader-based | Paxos-based |
| **Sharding** | Manual (per-database) | Automatic |
| **Transactions** | Single database | Cross-shard ACID |
| **Query language** | SQL (subscriptions) | None (API-based) |
| **Application logic** | Wasm modules | Application layer |

**Key differences:**

- FoundationDB is a **distributed KV store** with ordered transactions
- SpacetimeDB is a **single-database** system with relational semantics
- FoundationDB uses **Paxos** for consensus; SpacetimeDB uses **leader election**
- SpacetimeDB provides **built-in real-time sync**

### 6.4 Trade-offs SpacetimeDB Makes

**SpacetimeDB optimizes for:**

1. **Low latency**: In-memory data, direct pointer access
2. **Real-time sync**: Automatic subscriptions, delta streaming
3. **Simplicity**: Single binary, no DevOps, serverless
4. **Developer experience**: Type-safe SDKs, hot reload

**Trade-offs:**

1. **Write throughput**: Single-writer limits parallelism
2. **Database size**: Limited by available RAM
3. **Query complexity**: No complex joins, limited SQL
4. **Durability latency**: fsync on each commit (configurable batching)

---

## 7. Actual Source Code Analysis

### 7.1 Commit Log Implementation

**Source:** `crates/commitlog/src/repo.rs`, `crates/commitlog/src/commit.rs`

```rust
// Commitlog options
pub struct Options {
    pub max_segment_size: u64,      // Default: 64 MiB
    pub flush_interval: Duration,   // How often to fsync
}

// Segment writer
pub struct SegmentWriter {
    file: File,
    buf: BufWriter<File>,
    offset: u64,  // Current byte offset
}

// Transaction data encoding
pub struct Txdata<T> {
    pub schema: TableSchema,
    pub data: T,  // ProductValue for SpacetimeDB
}

impl<T: Encode> Encode for Txdata<T> {
    fn encode(&self, out: &mut impl BufWriter) -> io::Result<()> {
        // BSATN encode the ProductValue
        bsatn::encode(&self.data, out)
    }
}
```

### 7.2 Durability Actor

**Source:** `crates/durability/src/imp/local.rs`

```rust
struct Actor<T> {
    clog: Arc<Commitlog<Txdata<T>>>,
    durable_offset: watch::Sender<Option<TxOffset>>,
    queue_depth: Arc<AtomicU64>,
    batch_capacity: NonZeroUsize,
    lock: Lock,  // Advisory file lock
}

impl<T: Encode + Send + Sync + 'static> Actor<T> {
    async fn run(
        mut self,
        mut txdata_rx: mpsc::UnboundedReceiver<Transaction<Txdata<T>>>,
        mut shutdown_rx: mpsc::Receiver<ShutdownReply>,
    ) {
        let mut batch = Vec::with_capacity(self.batch_capacity.get());
        let mut shutdown_requested = None;

        loop {
            // Drain queue into batch
            while batch.len() < self.batch_capacity.get() {
                match txdata_rx.try_recv() {
                    Ok(tx) => batch.push(tx),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        if batch.is_empty() {
                            return;
                        }
                    }
                }
            }

            if batch.is_empty() {
                if let Some(reply) = shutdown_requested.take() {
                    // Shutdown complete
                    let _ = reply.send(self.clog.max_committed_offset());
                    return;
                }
                // Wait for new transactions
                tokio::select! {
                    Some(tx) = txdata_rx.recv() => batch.push(tx),
                    Some(reply) = shutdown_rx.recv() => shutdown_requested = Some(reply),
                }
            } else {
                // Write batch to commitlog
                let offset = self.clog.commit(batch.drain(..)).await?;
                self.durable_offset.send(Some(offset))?;
            }
        }
    }
}
```

### 7.3 Subscription Delta Evaluation

**Source:** `crates/subscription/src/subscription_executor.rs`

```rust
/// Execute subscriptions against transaction changes
pub struct SubscriptionExecutor<'a, Tx> {
    tx: &'a Tx,
    subscriptions: &'a ModuleSubscriptionManager,
}

impl<'a, Tx: Datastore + DeltaStore> SubscriptionExecutor<'a, Tx> {
    /// Evaluate all subscriptions affected by this transaction
    pub fn eval_all(&self, tx_data: &TxData) -> Vec<ClientUpdate> {
        let mut updates = Vec::new();

        for (subscription_id, plan) in &self.subscriptions.plans {
            // Check if any changed tables affect this subscription
            if plan.overlaps(tx_data) {
                let delta = eval_delta(self.tx, &mut metrics, plan)?;
                if let Some(delta) = delta {
                    updates.push(ClientUpdate {
                        subscription_id: *subscription_id,
                        inserts: delta.inserts,
                        deletes: delta.deletes,
                    });
                }
            }
        }

        updates
    }
}
```

### 7.4 Replication State Tracking

**Source:** SQL query from replication tests

```sql
-- Get current leader info
SELECT node_v2.id, node_v2.network_addr
FROM node_v2
JOIN replica ON replica.node_id = node_v2.id
JOIN replication_state ON replication_state.leader = replica.id
WHERE replication_state.database_id = {database_id}

-- Replication state tracks:
-- - leader: Current leader replica ID
-- - database_id: Which database
-- - replication progress (lag, last seen offset, etc.)
```

---

## 8. Summary and Architecture Diagram

### 8.1 Complete Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          SpacetimeDB Host                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                  │
│  │  Database 1 │    │  Database 2 │    │  Database N │                  │
│  │  (Leader)   │    │  (Follower) │    │  (Follower) │                  │
│  │             │    │             │    │             │                  │
│  │ ┌─────────┐ │    │ ┌─────────┐ │    │ ┌─────────┐ │                  │
│  │ │Reducer  │ │    │ │ Query   │ │    │ │ Query   │ │                  │
│  │ │Executor │ │    │ │ Engine  │ │    │ │ Engine  │ │                  │
│  │ └────┬────┘ │    │ └────┬────┘ │    │ └────┬────┘ │                  │
│  │ ┌────▼────┐ │    │ ┌────▼────┐ │    │ ┌────▼────┐ │                  │
│  │ │TxState  │ │    │ │ Pages   │ │    │ │ Pages   │ │                  │
│  │ └────┬────┘ │    │ │ Indexes │ │    │ │ Indexes │ │                  │
│  │ ┌────▼────┐ │    │ │ Blobs   │ │    │ │ Blobs   │ │                  │
│  │ │Commitlog│ │◀───┼────────────┼────┼────────────┤                  │
│  │ └─────────┘ │    │ └─────────┘ │    │ └─────────┘ │                  │
│  └──────┬──────┘    └─────────────┘    └─────────────┘                  │
│         │                                                                  │
│         │ Log shipping                                                     │
│         ▼                                                                  │
│  ┌─────────────┐                                                          │
│  │   Control   │◀───────── Cluster coordination                           │
│  │  Database   │     - Leader election                                     │
│  │   (sled)    │     - Replica tracking                                    │
│  └─────────────┘     - Health checks                                       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
         │
         │ WebSocket
         ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                              Clients                                     │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐                          │
│  │   React  │    │  Unity   │    │  Unreal  │                          │
│  │   App    │    │   Game   │    │   Game   │                          │
│  └──────────┘    └──────────┘    └──────────┘                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Key Takeaways

1. **Leader-based replication**: Simple, predictable, but limited write throughput
2. **Commit log durability**: Fast writes, crash recovery, snapshot optimization
3. **In-memory storage**: Sub-microsecond access, limited by RAM
4. **Real-time subscriptions**: Automatic delta computation and streaming
5. **Single-writer serialization**: No conflicts, simple concurrency model
6. **Content-addressed storage**: Efficient snapshots, deduplication

---

## References

- **Storage Internals**: `spacetimedb/storage-internals-deep-dive.md`
- **Subscription Docs**: `docs/docs/00200-core-concepts/00400-subscriptions.md`
- **Replication Tests**: `smoketests/tests/replication.py`
- **Control DB**: `crates/standalone/src/control_db.rs`
- **Commitlog**: `crates/commitlog/`
- **Durability**: `crates/durability/`
- **Snapshot**: `crates/snapshot/`
- **Subscription Engine**: `crates/subscription/`
