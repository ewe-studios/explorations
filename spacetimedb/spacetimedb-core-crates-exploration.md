---
name: SpacetimeDB Core Crates
description: Core database engine and infrastructure crates that power SpacetimeDB's in-memory relational database with real-time synchronization
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/SpacetimeDB/crates/
---

# SpacetimeDB Core Crates - Database Engine Internals

## Overview

The SpacetimeDB Core Crates comprise the **internal architecture of SpacetimeDB**, the in-memory relational database that executes application logic directly. This collection of Rust crates implements the database engine, query execution, storage, transaction management, and client synchronization. Understanding these crates provides insight into how SpacetimeDB achieves its unique architecture of "database as server."

Key components:
- **core** - Main database engine and runtime
- **execution** - Query execution and reducer runtime
- **storage** - In-memory storage with durability
- **query** - SQL parser and query planner
- **sats** - Algebraic data types and serialization
- **commitlog** - Write-ahead log for durability
- **client-api** - Client connection handling
- **bindings** - SDK bindings generation

## Directory Structure

```
SpacetimeDB/crates/
├── core/                       # Database core engine
│   ├── src/
│   │   ├── db/                 # Database implementation
│   │   ├── host/               # Module hosting (Wasmtime)
│   │   ├── subscription/       # Query subscriptions
│   │   ├── transaction/        # Transaction management
│   │   └── vm/                 # Virtual machine for modules
│   └── Cargo.toml
├── execution/                  # Query execution engine
│   ├── src/
│   │   ├── reducer/            # Reducer execution
│   │   ├── query/              # Query execution
│   │   └── plan/               # Execution plans
│   └── Cargo.toml
├── query/                      # Query parsing and planning
│   ├── src/
│   │   ├── parser/             # SQL parser
│   │   ├── planner/            # Query planner
│   │   └── optimizer/          # Query optimizer
│   └── Cargo.toml
├── sats/                       # Algebraic data types
│   ├── src/
│   │   ├── types/              # Type definitions
│   │   ├── serialize/          # Serialization
│   │   └── deserialize/        # Deserialization
│   └── Cargo.toml
├── commitlog/                  # Write-ahead log
│   ├── src/
│   │   ├── wal.rs              # WAL implementation
│   │   ├── segment.rs          # Log segments
│   │   └── replay.rs           # Log replay
│   └── Cargo.toml
├── datastore/                  # Storage layer
│   ├── src/
│   │   ├── table/              # Table storage
│   │   ├── index/              # Index implementations
│   │   └── traits.rs           # Datastore traits
│   └── Cargo.toml
├── client-api/                 # Client API server
│   ├── src/
│   │   ├── connection.rs       # Client connections
│   │   ├── messages.rs         # Protocol messages
│   │   └── subscription.rs     # Subscription handling
│   └── Cargo.toml
├── bindings*/                  # Language bindings
│   ├── bindings-macro/         # Procedural macros
│   ├── bindings-sys/           # FFI bindings
│   ├── bindings-typescript/    # TypeScript SDK
│   ├── bindings-csharp/        # C# SDK
│   └── bindings-cpp/           # C++ SDK
├── cli/                        # Command-line interface
├── standalone/                 # Standalone server
└── lib/                        # Public library interface
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Client Connections                           │
│              (WebSocket, HTTP, gRPC, Language SDKs)             │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Client API Layer                             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Connection Manager  │  Message Router  │  Auth         │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Subscription Engine                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Query Subscriptions  │  Row Filtering  │  Delta Stream │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Core Database Engine                         │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐ │
│  │  Table Storage  │  │  Index Engine   │  │  Transactions  │ │
│  │  (In-Memory)    │  │  (BTree, Hash)  │  │  (ACID)        │ │
│  └─────────────────┘  └─────────────────┘  └────────────────┘ │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐ │
│  │  Query Engine   │  │  Reducer Exec   │  │  Module Host  │ │
│  │  (SQL Parser)   │  │  (Wasmtime)     │  │  (Sandbox)     │ │
│  └─────────────────┘  └─────────────────┘  └────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Durability Layer                             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  CommitLog (WAL)  │  Snapshots  │  Checkpoint Manager  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Core Database Engine

### Table Storage

```rust
// crates/datastore/src/table/mod.rs
use crate::index::{BTreeIndex, HashIndex, Index};
use spacetimedb_sats::{AlgebraicType, ProductType, Value};

/// In-memory table storage
pub struct Table {
    /// Table name
    name: String,
    /// Table schema
    schema: ProductType,
    /// Row storage (columnar or row-based)
    storage: TableStorage,
    /// Indexes on columns
    indexes: HashMap<ColumnName, Box<dyn Index>>,
    /// Primary key column
    primary_key: Option<ColumnName>,
}

enum TableStorage {
    /// Row-oriented storage
    RowStore(Vec<Row>),
    /// Column-oriented storage
    ColumnStore {
        columns: Vec<Column>,
        row_count: usize,
    },
}

impl Table {
    /// Insert a row
    pub fn insert(&mut self, row: Row) -> Result<RowPointer, TableError> {
        // Validate schema
        row.validate(&self.schema)?;

        // Check primary key constraint
        if let Some(pk_col) = &self.primary_key {
            let pk_value = row.get(pk_col);
            if self.indexes.get(pk_col).unwrap().contains(pk_value) {
                return Err(TableError::PrimaryKeyViolation);
            }
        }

        // Insert row
        let ptr = self.storage.insert(row);

        // Update indexes
        self.update_indexes(ptr, self.storage.get(ptr));

        Ok(ptr)
    }

    /// Delete a row
    pub fn delete(&mut self, ptr: RowPointer) -> Result<Row, TableError> {
        let row = self.storage.get(ptr);

        // Remove from indexes
        self.remove_from_indexes(ptr, &row);

        // Delete row
        self.storage.delete(ptr);

        Ok(row)
    }

    /// Scan table with filter
    pub fn scan(&self, filter: Option<&Expr>) -> TableIterator {
        TableIterator::new(&self.storage, filter)
    }
}
```

### Index Implementations

```rust
// crates/datastore/src/index/btree.rs
use std::collections::BTreeMap;
use spacetimedb_sats::Value;
use crate::table::RowPointer;

/// B-Tree index for ordered column lookups
pub struct BTreeIndex {
    column: ColumnName,
    tree: BTreeMap<Value, Vec<RowPointer>>,
}

impl Index for BTreeIndex {
    fn insert(&mut self, key: Value, ptr: RowPointer) {
        self.tree.entry(key).or_default().push(ptr);
    }

    fn remove(&mut self, key: &Value, ptr: RowPointer) {
        if let Some(pointers) = self.tree.get_mut(key) {
            pointers.retain(|p| p != &ptr);
            if pointers.is_empty() {
                self.tree.remove(key);
            }
        }
    }

    fn lookup(&self, key: &Value) -> Vec<RowPointer> {
        self.tree.get(key).cloned().unwrap_or_default()
    }

    fn range_scan(
        &self,
        start: Bound<&Value>,
        end: Bound<&Value>,
    ) -> impl Iterator<Item = RowPointer> {
        self.tree
            .range((start, end))
            .flat_map(|(_, pointers)| pointers.iter())
            .copied()
    }
}

// crates/datastore/src/index/hash.rs
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use spacetimedb_sats::Value;
use crate::table::RowPointer;

/// Hash index for exact match lookups
pub struct HashIndex {
    column: ColumnName,
    map: HashMap<Value, Vec<RowPointer>, BuildHasherDefault<IdentityHash>>,
}

impl Index for HashIndex {
    fn insert(&mut self, key: Value, ptr: RowPointer) {
        self.map.entry(key).or_default().push(ptr);
    }

    fn remove(&mut self, key: &Value, ptr: RowPointer) {
        if let Some(pointers) = self.map.get_mut(key) {
            pointers.retain(|p| p != &ptr);
            if pointers.is_empty() {
                self.map.remove(key);
            }
        }
    }

    fn lookup(&self, key: &Value) -> Vec<RowPointer> {
        self.map.get(key).cloned().unwrap_or_default()
    }
}
```

## Transaction Management

```rust
// crates/core/src/transaction/mod.rs
use crate::db::Database;
use crate::datastore::Datastore;
use spacetimedb_commitlog::CommitLog;

/// Database transaction
pub struct Transaction<'db> {
    db: &'db mut Database,
    datastore: Datastore,
    tx_data: TransactionData,
    read_set: HashSet<RowPointer>,
    write_set: HashSet<RowPointer>,
}

struct TransactionData {
    energy_quanta: u64,
    reducer_call: Option<ReducerCallInfo>,
    timestamp: Timestamp,
}

impl<'db> Transaction<'db> {
    /// Begin a new transaction
    pub fn begin(db: &'db mut Database) -> Self {
        Self {
            db,
            datastore: db.datastore.snapshot(),
            tx_data: TransactionData {
                energy_quanta: 0,
                reducer_call: None,
                timestamp: Timestamp::now(),
            },
            read_set: HashSet::new(),
            write_set: HashSet::new(),
        }
    }

    /// Commit the transaction
    pub fn commit(mut self) -> Result<CommitInfo, TransactionError> {
        // Write to commit log
        let commit_log_entry = self.create_commit_log_entry();
        self.db.commitlog.append(commit_log_entry)?;

        // Apply changes to database
        self.datastore.apply(self.write_set)?;

        // Update metrics
        self.update_metrics();

        Ok(CommitInfo {
            tx_id: self.tx_data.reducer_call.map(|rc| rc.tx_id),
            timestamp: self.tx_data.timestamp,
            energy_used: self.tx_data.energy_quanta,
        })
    }

    /// Rollback the transaction
    pub fn rollback(self) {
        // Discard all changes
        // (datastore snapshot is dropped)
    }
}
```

## Query Execution

```rust
// crates/execution/src/reducer/mod.rs
use crate::plan::ExecutionPlan;
use crate::query::QueryExecutor;
use spacetimedb_lib::ReducerDef;

/// Reducer execution engine
pub struct ReducerExecutor {
    reducer: ReducerDef,
    wasm_instance: WasmInstance,
}

impl ReducerExecutor {
    /// Execute a reducer
    pub fn execute(
        &mut self,
        args: &[Value],
        tx: &mut Transaction,
    ) -> Result<ReducerResult, ReducerError> {
        // Set up WASM instance
        self.wasm_instance.set_args(args);

        // Execute reducer in WASM
        let result = self.wasm_instance.call(&self.reducer.name)?;

        // Process side effects (table mutations)
        self.process_side_effects(tx)?;

        Ok(ReducerResult {
            success: result.is_ok(),
            energy_used: self.calculate_energy(),
        })
    }

    fn process_side_effects(
        &mut self,
        tx: &mut Transaction,
    ) -> Result<(), ReducerError> {
        // Get table operations from WASM memory
        let ops = self.wasm_instance.get_table_ops();

        for op in ops {
            match op {
                TableOp::Insert { table, row } => {
                    tx.insert(table, row)?;
                }
                TableOp::Delete { table, ptr } => {
                    tx.delete(table, ptr)?;
                }
                TableOp::Update { table, ptr, row } => {
                    tx.update(table, ptr, row)?;
                }
            }
        }

        Ok(())
    }
}
```

## Subscription Engine

```rust
// crates/core/src/subscription/mod.rs
use crate::query::Query;
use crate::db::Database;
use spacetimedb_client_api::SubscriptionId;

/// Query subscription for real-time updates
pub struct Subscription {
    id: SubscriptionId,
    queries: Vec<Query>,
    client_sender: tokio::sync::mpsc::Sender<DatabaseUpdate>,
}

pub struct SubscriptionManager {
    subscriptions: HashMap<Identity, Vec<Subscription>>,
    database: Arc<Database>,
}

impl SubscriptionManager {
    /// Add a new subscription
    pub fn add(
        &mut self,
        client_id: Identity,
        queries: Vec<Query>,
        sender: tokio::sync::mpsc::Sender<DatabaseUpdate>,
    ) -> Result<SubscriptionId, SubscriptionError> {
        // Validate queries
        for query in &queries {
            self.validate_query(query)?;
        }

        // Create subscription
        let id = SubscriptionId::generate();
        let subscription = Subscription {
            id,
            queries,
            client_sender: sender,
        };

        // Store subscription
        self.subscriptions
            .entry(client_id)
            .or_default()
            .push(subscription);

        Ok(id)
    }

    /// Evaluate subscriptions after a transaction
    pub fn evaluate_after_tx(
        &self,
        tx: &Transaction,
    ) -> Result<(), SubscriptionError> {
        for (client_id, subscriptions) in &self.subscriptions {
            for subscription in subscriptions {
                // Evaluate each query
                for query in &subscription.queries {
                    let delta = self.evaluate_query_delta(query, tx)?;

                    // Send update to client
                    if !delta.is_empty() {
                        subscription.client_sender.try_send(DatabaseUpdate {
                            subscription_id: subscription.id,
                            delta,
                        })?;
                    }
                }
            }
        }

        Ok(())
    }
}
```

## Commit Log (WAL)

```rust
// crates/commitlog/src/wal.rs
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

/// Write-ahead log for durability
pub struct CommitLog {
    /// Current log segment
    current_segment: SegmentWriter,
    /// Segment directory
    segment_dir: PathBuf,
    /// Maximum segment size
    max_segment_size: usize,
}

struct SegmentWriter {
    file: BufWriter<File>,
    sequence_number: u64,
    byte_offset: usize,
}

impl CommitLog {
    /// Append entry to log
    pub fn append(&mut self, entry: CommitLogEntry) -> Result<u64, CommitLogError> {
        // Serialize entry
        let bytes = entry.serialize()?;

        // Check if we need to roll to new segment
        if self.current_segment.byte_offset + bytes.len() > self.max_segment_size {
            self.roll_segment()?;
        }

        // Write entry
        let offset = self.current_segment.byte_offset as u64;
        self.current_segment.write(&bytes)?;

        Ok(offset)
    }

    /// Roll to new segment
    fn roll_segment(&mut self) -> Result<(), CommitLogError> {
        // Flush current segment
        self.current_segment.file.flush()?;

        // Create new segment
        let sequence_number = self.current_segment.sequence_number + 1;
        let segment_path = self.segment_dir.join(format!("{:020}", sequence_number));

        let file = BufWriter::new(File::create(segment_path)?);

        self.current_segment = SegmentWriter {
            file,
            sequence_number,
            byte_offset: 0,
        };

        Ok(())
    }

    /// Replay log from beginning
    pub fn replay(&self) -> Result<impl Iterator<Item = CommitLogEntry>, CommitLogError> {
        let segments = self.list_segments()?;
        let mut readers = Vec::new();

        for segment_path in segments {
            let file = File::open(segment_path)?;
            readers.push(SegmentReader::new(file));
        }

        Ok(readers.into_iter().flatten())
    }
}
```

## Algebraic Data Types (SATs)

```rust
// crates/sats/src/types.rs
/// Algebraic types for SpacetimeDB schema
#[derive(Debug, Clone)]
pub enum AlgebraicType {
    /// Unit type
    Unit,
    /// Boolean
    Bool,
    /// 8-bit integer
    I8,
    U8,
    /// 16-bit integer
    I16,
    U16,
    /// 32-bit integer
    I32,
    U32,
    /// 64-bit integer
    I64,
    U64,
    /// 128-bit integer
    I128,
    U128,
    /// Floating point
    F32,
    F64,
    /// String
    String,
    /// Array type
    Array(Box<AlgebraicType>),
    /// Product type (struct/tuple)
    Product(ProductType),
    /// Sum type (enum)
    Sum(SumType),
}

/// Product type (struct with named fields)
#[derive(Debug, Clone)]
pub struct ProductType {
    pub elements: Vec<ProductElement>,
}

#[derive(Debug, Clone)]
pub struct ProductElement {
    pub name: String,
    pub algebraic_type: AlgebraicType,
}

/// Sum type (enum with variants)
#[derive(Debug, Clone)]
pub struct SumType {
    pub variants: Vec<SumTypeVariant>,
}

#[derive(Debug, Clone)]
pub struct SumTypeVariant {
    pub name: String,
    pub algebraic_type: AlgebraicType,
}
```

## Client API

```rust
// crates/client-api/src/connection.rs
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use spacetimedb_messages::client::{ClientMessage, Subscribe};
use spacetimedb_messages::server::{ServerMessage, DatabaseUpdate};

/// Client connection handler
pub struct ClientConnection {
    identity: Identity,
    websocket: WebSocketStream<TcpStream>,
    subscriptions: Vec<SubscriptionId>,
    tx: tokio::sync::mpsc::Sender<DatabaseUpdate>,
    rx: tokio::sync::mpsc::Receiver<ClientMessage>,
}

impl ClientConnection {
    /// Handle incoming messages
    pub async fn handle_messages(&mut self) -> Result<(), ConnectionError> {
        loop {
            tokio::select! {
                // Incoming from client
                Some(msg) = self.rx.recv() => {
                    self.handle_client_message(msg).await?;
                }
                // Incoming from WebSocket
                Some(result) = self.websocket.next() => {
                    let msg = self.parse_message(result?)?;
                    self.handle_client_message(msg).await?;
                }
                // Database updates
                Some(update) = self.tx.recv() => {
                    self.send_update(update).await?;
                }
            }
        }
    }

    async fn handle_client_message(
        &mut self,
        msg: ClientMessage,
    ) -> Result<(), ConnectionError> {
        match msg {
            ClientMessage::Subscribe(Subscribe { query_strings }) => {
                // Parse and validate queries
                let queries = self.parse_queries(query_strings)?;

                // Register subscription
                let sub_id = self.register_subscription(queries).await?;

                // Send confirmation
                self.send(ServerMessage::SubscriptionConfirmation {
                    subscription_id: sub_id,
                }).await?;
            }
            ClientMessage::CallReducer { name, args } => {
                // Execute reducer
                let result = self.call_reducer(name, args).await?;

                // Send result
                self.send(ServerMessage::ReducerResponse { result }).await?;
            }
            _ => {}
        }

        Ok(())
    }
}
```

## Related Documents

- [Wasmtime Runtime](../wasmtime/wasmtime-runtime-exploration.md) - WASM execution
- [WASI](../wasmtime/wasi-exploration.md) - System interface
- [OmniPaxos](./omnipaxos-exploration.md) - Consensus protocol

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/SpacetimeDB/crates/`
- SpacetimeDB Architecture: https://spacetimedb.com/docs/architecture
- GitHub: https://github.com/clockworklabs/SpacetimeDB
