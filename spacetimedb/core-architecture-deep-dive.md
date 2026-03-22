# SpacetimeDB Core Architecture Deep Dive

## Overview

This document explains the internal architecture of SpacetimeDB, focusing on how the various crates interact to provide a unified database experience.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Client Layer                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐  │
│  │ TypeScript  │  │    Rust     │  │     C#      │  │    C++     │  │
│  │    SDK      │  │    SDK      │  │    SDK      │  │    SDK     │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              │ WebSocket / HTTP
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      spacetimedb-client-api                          │
│  - Connection handling                                              │
│  - Message routing                                                  │
│  - Authentication                                                   │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     spacetimedb-core                                 │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                    Module Host (WASM)                           │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │ │
│  │  │ Rust Module  │  │  C# Module   │  │  TS Module   │         │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘         │ │
│  └────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     spacetimedb-datastore                            │
│  - Transaction management                                           │
│  - ACID guarantees                                                  │
│  - Snapshot isolation                                               │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      spacetimedb-table                               │
│  - In-memory table storage                                          │
│  - Index management (B-Tree, Hash)                                  │
│  - Row deduplication                                                │
└─────────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   spacetimedb-commitlog                              │
│  - Write-ahead logging                                              │
│  - Crash recovery                                                   │
│  - zstd compression                                                 │
└─────────────────────────────────────────────────────────────────────┘
```

## Crate Dependencies

### Core Dependencies Graph

```
spacetimedb-standalone (binary)
├── spacetimedb-core
│   ├── spacetimedb-datastore
│   │   ├── spacetimedb-table
│   │   ├── spacetimedb-commitlog
│   │   ├── spacetimedb-durability
│   │   ├── spacetimedb-snapshot
│   │   └── spacetimedb-execution
│   ├── spacetimedb-subscription
│   ├── spacetimedb-query
│   └── spacetimedb-sats
├── spacetimedb-client-api
└── spacetimedb-cli
```

### Type System (sats)

The **Spacetime Algebraic Type System (SATS)** is the foundation:

```rust
// crates/sats/src/lib.rs

/// Algebraic types supported by SpacetimeDB
pub enum AlgebraicType {
    Bool,
    I8, I16, I32, I64, I128, I256,
    U8, U16, U32, U64, U128, U256,
    F32, F64,
    String,
    Bytes,
    Product(ProductType),      // Struct-like
    Sum(SumType),              // Enum-like
    Array(Box<AlgebraicType>),
    Option(Box<AlgebraicType>),
}

/// Product type (struct)
pub struct ProductType {
    pub elements: Vec<ProductTypeElement>,
}

pub struct ProductTypeElement {
    pub name: Option<String>,
    pub algebraic_type: AlgebraicType,
}
```

### Serialization (BSATN)

**BSATN** (Binary SATS Network) is the wire format:

```rust
// Binary serialization format
// ┌─────────┬─────────────────────────┐
// │  type   │       value             │
// │  tag    │      (variable)         │
// └─────────┴─────────────────────────┘

// Example: Product value serialization
// Type tag: 0x02 (product)
// Field count: u32
// For each field:
//   - field value (recursively serialized)
```

### BFLATN (Binary Flat)

**BFLATN** is the storage format optimized for direct memory access:

```rust
// Key difference from BSATN:
// - Fixed-width fields at predictable offsets
// - Variable data referenced by offset
// - Zero-copy deserialization possible

struct BflatnLayout {
    fixed_size: usize,
    var_members: Vec<VarMember>,
}
```

## Transaction System

### Transaction Lifecycle

```
┌─────────────────────────────────────────────────────────────────┐
│                    Transaction States                            │
│                                                                  │
│  ┌─────────┐    begin    ┌───────────┐    commit   ┌─────────┐ │
│  │  Idle   │ ──────────▶ │  Active   │ ──────────▶ │ Committed│ │
│  └─────────┘             └───────────┘             └─────────┘ │
│                              │                                   │
│                              │ rollback                         │
│                              ▼                                   │
│                        ┌───────────┐                            │
│                        │ Aborted   │                            │
│                        └───────────┘                            │
└─────────────────────────────────────────────────────────────────┘
```

### Locking Transaction Datastore

```rust
// crates/datastore/src/locking_tx_datastore/mod.rs

pub struct LockingTxDatastore {
    /// Committed state (readable by all)
    committed_state: CommittedState,
    /// Active transactions
    transactions: HashMap<TxId, MutTx>,
}

pub struct MutTx {
    /// Private workspace for transaction
    tx_state: TxState,
    /// Reads performed (for conflict detection)
    read_set: HashSet<TableId>,
    /// Writes performed
    write_set: HashMap<TableId, TxTableState>,
}
```

### Conflict Detection

```rust
impl LockingTxDatastore {
    pub fn commit(&mut self, tx_id: TxId) -> CommitResult {
        let tx = self.transactions.remove(&tx_id)?;

        // Check for read-write conflicts
        for table_id in &tx.read_set {
            if self.committed_state.was_modified_since_read(table_id, tx.read_timestamp) {
                return Err(CommitError::SerializationConflict);
            }
        }

        // Apply writes to committed state
        self.committed_state.apply(tx.write_set)?;

        // Write commit log entry
        self.commitlog.append(tx.as_record())?;

        Ok(())
    }
}
```

## Query Execution

### Query Pipeline

```
SQL Query
    │
    ▼
┌─────────────────────┐
│   SQL Parser        │  (sqlparser-rs)
│   SELECT * FROM ... │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│   Logical Plan      │  (Relational Algebra)
│   TableScan → Filter│
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│   Physical Plan     │  (Execution Strategy)
│   IndexSeek + Proj  │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│   Execution Engine  │
│   Row-by-row eval   │
└─────────────────────┘
    │
    ▼
Result Set
```

### Expression Evaluation

```rust
// crates/expr/src/lib.rs

pub enum Expr {
    Column(ColId),
    Literal(AlgebraicValue),
    BinaryOp {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Function {
        name: String,
        args: Vec<Expr>,
    },
}

pub trait Evaluable {
    fn eval(&self, row: RowRef) -> Result<AlgebraicValue>;
}
```

## Subscription System

### Subscription Query

```rust
// crates/subscription/src/lib.rs

pub struct Subscription {
    /// SQL query string
    query: String,
    /// Compiled query plan
    plan: QueryPlan,
    /// Last result sent to client
    last_result: HashSet<RowPointer>,
}

pub struct SubscriptionEngine {
    /// Active subscriptions per client
    client_subscriptions: HashMap<ClientId, Vec<Subscription>>,
    /// Table -> subscriptions that watch it
    table_subscriptions: HashMap<TableId, HashSet<SubscriptionId>>,
}
```

### Update Propagation

```rust
impl SubscriptionEngine {
    pub fn on_table_modified(&mut self, table_id: TableId, changes: &TableChanges) {
        // Find all subscriptions watching this table
        let subscriptions = self.table_subscriptions.get(&table_id);

        for sub_id in subscriptions {
            let sub = &mut self.subscriptions[sub_id];

            // Determine which rows match the subscription now
            let new_matches = sub.evaluate_changes(changes);

            // Compute delta
            let inserts = new_matches.difference(&sub.last_result);
            let deletes = sub.last_result.difference(&new_matches);

            // Send updates to client
            self.send_update(sub.client_id, inserts, deletes);

            // Update last_result
            sub.last_result = new_matches;
        }
    }
}
```

## Module System (WASM)

### WASM ABI

```rust
// crates/bindings-sys/src/lib.rs

/// Module export: initialization
#[no_mangle]
pub extern "C" fn init() {
    // Called when module is loaded
}

/// Module export: reducer call
#[no_mangle]
pub extern "C" fn __call_reducer__(
    reducer_id: u32,
    args_ptr: u32,
    args_len: u32,
) -> u32 {
    // Dispatch to appropriate reducer
}

/// Host import: table operations
extern "C" {
    fn __host_table_insert__(
        table_id: u32,
        row_ptr: u32,
        row_len: u32,
    ) -> bool;

    fn __host_table_delete__(
        table_id: u32,
        row_ptr: u32,
    ) -> bool;
}
```

### Module Execution Context

```rust
// crates/core/src/module_executor.rs

pub struct ModuleExecutor {
    /// WASM instance
    instance: wasmtime::Instance,
    /// Module metadata
    module_identity: Identity,
    /// Available reducers
    reducers: HashMap<String, ReducerInfo>,
}

pub struct ReducerContext {
    /// Sender's identity
    pub sender: Identity,
    /// Sender's connection ID
    pub connection_id: Option<ConnectionId>,
    /// Timestamp of reducer start
    pub timestamp: u64,
}
```

## Metrics System

```rust
// crates/metrics/src/lib.rs

pub struct Metrics {
    /// Prometheus registry
    registry: Registry,

    /// Table metrics
    pub table_row_count: IntGaugeVec,
    pub table_size_bytes: IntGaugeVec,

    /// Query metrics
    pub query_duration: HistogramVec,

    /// Transaction metrics
    pub tx_commit_duration: HistogramVec,
    pub tx_conflicts: CounterVec,
}
```

## Error Handling

```rust
// Common error types

#[derive(Debug, thiserror::Error)]
pub enum DatastoreError {
    #[error("Table not found: {0}")]
    TableNotFound(TableId),

    #[error("Index not found: {0}")]
    IndexNotFound(IndexId),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Out of memory")]
    OutOfMemory,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum CommitError {
    #[error("Serialization conflict")]
    SerializationConflict,

    #[error("Commit log write failed: {0}")]
    LogWrite(#[source] std::io::Error),
}
```

## Configuration

```rust
// crates/core/src/config.rs

pub struct Config {
    /// Maximum memory usage (bytes)
    pub max_memory: u64,

    /// Commit log segment size (bytes)
    pub commitlog_segment_size: u64,

    /// Snapshot interval (seconds)
    pub snapshot_interval: u64,

    /// Query timeout (milliseconds)
    pub query_timeout_ms: u64,

    /// Reducer timeout (milliseconds)
    pub reducer_timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_memory: 4 * 1024 * 1024 * 1024, // 4GB
            commitlog_segment_size: 64 * 1024 * 1024, // 64MB
            snapshot_interval: 300, // 5 minutes
            query_timeout_ms: 30_000, // 30 seconds
            reducer_timeout_ms: 30_000, // 30 seconds
        }
    }
}
```

## Threading Model

```
┌─────────────────────────────────────────────────────────────────┐
│                      Tokio Runtime                               │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Worker Threads (N)                             │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐                 │ │
│  │  │  Task 1  │  │  Task 2  │  │  Task 3  │                 │ │
│  │  │ (Query)  │  │(Reducer) │  │(Network) │                 │ │
│  │  └──────────┘  └──────────┘  └──────────┘                 │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Blocking Thread Pool                           │ │
│  │  - Disk I/O (commit log)                                    │ │
│  │  - Snapshot writes                                          │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Security Model

### Isolation

1. **WASM Sandbox** - Module code runs in WASM with no direct system access
2. **Capability-based** - Modules can only access their own tables
3. **Identity-based auth** - All operations authenticated via JWT

### Access Control

```rust
// Tables can be:
// - private (only module can access)
// - public (clients can read via subscription)

#[spacetimedb::table(public)]  // Public read access
pub struct PublicData { ... }

#[spacetimedb::table]  // Private
pub struct InternalState { ... }
```
