# SpacetimeDB: Rust Revision - Complete Translation Guide

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.spacetime/SpacetimeDB`
**Target:** Rust with valtron executor (no async/await, no tokio)
**Date:** 2026-03-27

---

## 1. Overview

### 1.1 What We're Translating

SpacetimeDB is a Rust-based in-memory database with:
- In-memory table storage
- Write-ahead log (commitlog) for persistence
- SQL query engine with incremental view maintenance
- WebAssembly module hosting
- Real-time client synchronization

### 1.2 Key Design Decisions

#### Ownership Strategy

```rust
// SpacetimeDB uses Arc for shared state across threads
use std::sync::Arc;

struct Database {
    tables: Arc<DashMap<TableId, Table>>,
    commitlog: Arc<Mutex<Commitlog>>,
    subscriptions: Arc<DashMap<SubscriptionId, Subscription>>,
}

// For valtron (single-threaded), use Rc
use std::rc::Rc;
use std::cell::RefCell;

struct ValtronDatabase {
    tables: RefCell<HashMap<TableId, Table>>,
    commitlog: RefCell<Commitlog>,
    subscriptions: RefCell<HashMap<SubscriptionId, Subscription>>,
}
```

#### Reference Handling

| Pattern | SpacetimeDB | Valtron (Single-threaded) |
|---------|-------------|--------------------------|
| Shared ownership | `Arc<T>` | `Rc<T>` |
| Interior mutability | `Mutex<T>`, `DashMap` | `RefCell<T>` |
| Borrowing | `&T`, `&mut T` | Same |
| Thread-safe | Yes | No (single-threaded) |

---

## 2. Type System Design

### 2.1 Core Database Types

```rust
/// Database value
#[derive(Debug, Clone, PartialEq)]
pub enum DbValue {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Null,
}

/// Row identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowId {
    pub page: u32,
    pub offset: u32,
}

/// Table identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TableId(pub u64);

/// Column identifier
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub unique: bool,
}

/// Data type
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Bool,
    I8, I16, I32, I64,
    U8, U16, U32, U64,
    F32, F64,
    String,
    Bytes,
    Array(Box<DataType>),
}
```

### 2.2 Table Structure

```rust
/// In-memory table
pub struct Table {
    pub id: TableId,
    pub name: String,
    pub schema: TableSchema,
    pub storage: RowHeap,
    pub indexes: Vec<Index>,
}

/// Table schema
pub struct TableSchema {
    pub columns: Vec<ColumnDef>,
    pub primary_key: Option<usize>,
    pub constraints: Vec<Constraint>,
}

/// Row heap storage
pub struct RowHeap {
    pages: Vec<HeapPage>,
    free_space: Vec<u16>,
}

struct HeapPage {
    data: Vec<u8>,
    row_offsets: Vec<u16>,
}

impl Table {
    pub fn new(id: TableId, name: String, schema: TableSchema) -> Self {
        Self {
            id,
            name,
            schema,
            storage: RowHeap::new(),
            indexes: Vec::new(),
        }
    }

    pub fn insert(&mut self, row: &[u8]) -> Result<RowId> {
        // Validate constraints
        self.validate_row(row)?;

        // Insert into heap
        let row_id = self.storage.insert(row);

        // Update indexes
        for index in &mut self.indexes {
            index.insert(row_id, row)?;
        }

        Ok(row_id)
    }

    pub fn get(&self, row_id: RowId) -> Option<&[u8]> {
        self.storage.get(row_id)
    }

    pub fn delete(&mut self, row_id: RowId) -> Result<Option<Vec<u8>>> {
        let row = self.storage.remove(row_id)?;

        // Update indexes
        for index in &mut self.indexes {
            index.remove(row_id)?;
        }

        Ok(row)
    }
}
```

---

## 3. Commitlog Translation

### 3.1 Original SpacetimeDB (with tokio)

```rust
// Original uses tokio for async I/O
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct Commitlog {
    file: File,
    offset: u64,
}

impl Commitlog {
    pub async fn append(&mut self, record: &[u8]) -> Result<u64> {
        let len = record.len() as u32;
        self.file.write_all(&len.to_be_bytes()).await?;
        self.file.write_all(record).await?;
        self.file.sync_all().await?;
        self.offset += 4 + record.len() as u64;
        Ok(self.offset)
    }
}
```

### 3.2 Valtron Version (no async)

```rust
// Valtron uses blocking I/O with TaskIterator
use std::fs::{File, OpenOptions};
use std::io::{Write, Seek, SeekFrom};

pub struct Commitlog {
    file: File,
    offset: u64,
    buffer: Vec<u8>,
}

impl Commitlog {
    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let offset = file.metadata()?.len();

        Ok(Self {
            file,
            offset,
            buffer: Vec::with_capacity(4096),
        })
    }

    pub fn append(&mut self, record: &[u8]) -> Result<u64> {
        // Write to buffer
        let len = record.len() as u32;
        self.buffer.extend_from_slice(&len.to_be_bytes());
        self.buffer.extend_from_slice(record);

        // Flush if buffer large enough
        if self.buffer.len() >= 8192 {
            self.flush()?;
        }

        Ok(self.offset)
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        self.file.seek(SeekFrom::Start(self.offset))?;
        self.file.write_all(&self.buffer)?;
        self.file.sync_all()?;

        self.offset += self.buffer.len() as u64;
        self.buffer.clear();

        Ok(())
    }

    pub fn replay(&mut self, from_offset: u64) -> Result<Vec<Vec<u8>>> {
        self.file.seek(SeekFrom::Start(from_offset))?;

        let mut records = Vec::new();
        let mut len_buf = [0u8; 4];

        loop {
            // Try to read length
            if self.file.read_exact(&mut len_buf).is_err() {
                break;  // EOF
            }

            let len = u32::from_be_bytes(len_buf) as usize;

            // Read payload
            let mut payload = vec![0u8; len];
            if self.file.read_exact(&mut payload).is_err() {
                break;  // Partial write (crash)
            }

            records.push(payload);
        }

        Ok(records)
    }
}
```

---

## 4. Query Engine Translation

### 4.1 Physical Plan without async

```rust
/// Physical query plan (synchronous)
pub trait PhysicalPlan: Send {
    type Output: Send;
    type Error: Send;

    /// Execute plan and return results
    fn execute(&self) -> Result<Vec<Row>, Self::Error>;
}

/// Table scan execution
pub struct TableScanExec {
    table: Rc<RefCell<Table>>,
    columns: Vec<usize>,
    filter: Option<CompiledExpr>,
}

impl PhysicalPlan for TableScanExec {
    type Output = Vec<Row>;
    type Error = QueryError;

    fn execute(&self) -> Result<Self::Output, Self::Error> {
        let table = self.table.borrow();
        let mut results = Vec::new();

        for row_id in table.storage.row_ids() {
            if let Some(row_data) = table.storage.get(row_id) {
                let row = Row::new(row_id, row_data);

                // Apply filter if present
                if let Some(filter) = &self.filter {
                    if !filter.evaluate_bool(&row) {
                        continue;
                    }
                }

                // Project columns
                let projected = row.project(&self.columns);
                results.push(projected);
            }
        }

        Ok(results)
    }
}
```

### 4.2 Hash Join without async

```rust
pub struct HashJoinExec {
    build_side: Box<dyn PhysicalPlan<Output = Vec<Row>>>,
    probe_side: Box<dyn PhysicalPlan<Output = Vec<Row>>>,
    build_key: usize,
    probe_key: usize,
    join_type: JoinType,
}

impl PhysicalPlan for HashJoinExec {
    type Output = Vec<Row>;
    type Error = QueryError;

    fn execute(&self) -> Result<Self::Output, Self::Error> {
        // Phase 1: Build hash table
        let build_rows = self.build_side.execute()?;
        let mut hash_table = HashMap::new();

        for row in build_rows {
            let key = row.get_value(self.build_key);
            hash_table
                .entry(key)
                .or_insert(Vec::new())
                .push(row);
        }

        // Phase 2: Probe
        let probe_rows = self.probe_side.execute()?;
        let mut results = Vec::new();

        for probe_row in probe_rows {
            let key = probe_row.get_value(self.probe_key);

            if let Some(build_rows) = hash_table.get(&key) {
                for build_row in build_rows {
                    results.push(self.combine_rows(build_row, &probe_row));
                }
            } else if self.join_type == JoinType::Left {
                results.push(self.combine_with_nulls(&probe_row));
            }
        }

        Ok(results)
    }
}
```

---

## 5. Subscription System Translation

### 5.1 Incremental View Maintenance

```rust
/// Subscription with incremental updates
pub struct Subscription {
    pub id: SubscriptionId,
    pub query: CompiledQuery,
    pub materialized: MaterializedView,
    pub clients: Vec<ClientId>,
}

pub struct MaterializedView {
    rows: HashMap<RowId, Row>,
    index: HashMap<DbValue, Vec<RowId>>,
}

impl Subscription {
    /// Compute delta from table change
    pub fn compute_delta(&mut self, change: &TableChange) -> ViewDelta {
        match change {
            TableChange::Insert { row } => {
                if self.query.matches(row) {
                    ViewDelta::Insert { row: row.clone() }
                } else {
                    ViewDelta::NoChange
                }
            }

            TableChange::Delete { row_id } => {
                if self.materialized.rows.contains_key(row_id) {
                    ViewDelta::Delete { row_id: *row_id }
                } else {
                    ViewDelta::NoChange
                }
            }

            TableChange::Update { row_id, new_row } => {
                let was_matching = self.materialized.rows.contains_key(row_id);
                let is_matching = self.query.matches(new_row);

                match (was_matching, is_matching) {
                    (true, true) => ViewDelta::Update { row_id: *row_id, new_row: new_row.clone() },
                    (true, false) => ViewDelta::Delete { row_id: *row_id },
                    (false, true) => ViewDelta::Insert { row: new_row.clone() },
                    (false, false) => ViewDelta::NoChange,
                }
            }
        }
    }
}
```

---

## 6. Valtron TaskIterator Pattern

### 6.1 TaskIterator for Query Execution

```rust
/// Task iterator for non-async execution
pub trait TaskIterator {
    type Ready;
    type Pending;
    type Spawner;
    type Error;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner, Self::Error>>;
}

pub enum TaskStatus<Ready, Pending, Spawner, Error> {
    Ready(Result<Ready, Error>),
    Pending(Pending),
    Spawned(Spawner),
    Done,
}

/// Query execution as TaskIterator
pub struct QueryTask {
    plan: PhysicalPlan,
    state: QueryState,
}

enum QueryState {
    Initial,
    BuildingHashTable { rows: Vec<Row>, current: usize },
    Probing { hash_table: HashMap<_, _>, probe_rows: Vec<Row>, current: usize },
    Complete,
}

impl TaskIterator for QueryTask {
    type Ready = Vec<Row>;
    type Pending = ();
    type Spawner = ();
    type Error = QueryError;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner, Self::Error>> {
        match &mut self.state {
            QueryState::Initial => {
                // Execute build side
                let build_rows = match self.plan.build_side.execute() {
                    Ok(rows) => rows,
                    Err(e) => return Some(TaskStatus::Ready(Err(e))),
                };

                self.state = QueryState::BuildingHashTable {
                    rows: build_rows,
                    current: 0,
                };

                // Continue to next iteration
                self.next()
            }

            QueryState::BuildingHashTable { rows, current } => {
                // Build hash table incrementally
                let mut hash_table = HashMap::new();

                // Process batch of rows
                let batch_size = 100;
                for row in rows.iter().skip(*current).take(batch_size) {
                    let key = row.get_value(self.plan.build_key);
                    hash_table.entry(key).or_insert(Vec::new()).push(row.clone());
                }

                *current += batch_size;

                if *current >= rows.len() {
                    // Hash table complete, move to probing
                    let probe_rows = match self.plan.probe_side.execute() {
                        Ok(rows) => rows,
                        Err(e) => return Some(TaskStatus::Ready(Err(e))),
                    };

                    self.state = QueryState::Probing {
                        hash_table,
                        probe_rows,
                        current: 0,
                    };

                    self.next()
                } else {
                    // Continue building
                    Some(TaskStatus::Pending(()))
                }
            }

            QueryState::Probing { hash_table, probe_rows, current } => {
                // Probe incrementally
                let mut results = Vec::new();
                let batch_size = 100;

                for row in probe_rows.iter().skip(*current).take(batch_size) {
                    let key = row.get_value(self.plan.probe_key);
                    if let Some(build_rows) = hash_table.get(&key) {
                        for build_row in build_rows {
                            results.push(self.combine_rows(build_row, row));
                        }
                    }
                }

                *current += batch_size;

                if *current >= probe_rows.len() {
                    self.state = QueryState::Complete;
                    Some(TaskStatus::Ready(Ok(results)))
                } else {
                    Some(TaskStatus::Pending(()))
                }
            }

            QueryState::Complete => Some(TaskStatus::Done),
        }
    }
}
```

---

## 7. Error Handling

### 7.1 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: DataType, actual: DataType },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Query error: {0}")]
    Query(#[from] QueryError),
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Planning error: {0}")]
    Planning(String),

    #[error("Execution error: {0}")]
    Execution(String),
}

type Result<T> = std::result::Result<T, DatabaseError>;
```

---

## 8. Memory Management

### 8.1 Bounded Memory Execution

```rust
/// Memory budget for query execution
pub struct MemoryBudget {
    limit: usize,
    used: usize,
}

impl MemoryBudget {
    pub fn new(limit: usize) -> Self {
        Self { limit, used: 0 }
    }

    pub fn allocate(&mut self, bytes: usize) -> Result<()> {
        if self.used + bytes > self.limit {
            return Err(QueryError::Execution("Out of memory".into()));
        }
        self.used += bytes;
        Ok(())
    }

    pub fn deallocate(&mut self, bytes: usize) {
        self.used = self.used.saturating_sub(bytes);
    }
}

/// Spill-to-disk for large sorts
pub struct ExternalSort {
    memory_budget: MemoryBudget,
    temp_dir: PathBuf,
    runs: Vec<PathBuf>,
}

impl ExternalSort {
    pub fn sort(&mut self, rows: Vec<Row>, order_by: &[OrderByExpr]) -> Result<Vec<Row>> {
        // If fits in memory, use in-memory sort
        let estimated_size = rows.len() * 100;  // Rough estimate
        if estimated_size <= self.memory_budget.limit {
            return Ok(self.in_memory_sort(rows, order_by));
        }

        // Otherwise, external sort
        self.external_sort(rows, order_by)
    }

    fn external_sort(&mut self, mut rows: Vec<Row>, order_by: &[OrderByExpr]) -> Result<Vec<Row>> {
        // Create sorted runs
        let chunk_size = self.memory_budget.limit / 100;  // bytes per row estimate

        while !rows.is_empty() {
            let chunk = rows.split_off(rows.len().min(chunk_size));
            let sorted = self.in_memory_sort(chunk, order_by);

            // Write run to temp file
            let run_path = self.temp_dir.join(format!("run_{}", self.runs.len()));
            self.write_run(&run_path, &sorted)?;
            self.runs.push(run_path);
        }

        // Merge runs
        self.merge_runs()
    }
}
```

---

## 9. Complete Example: Simple Database

```rust
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;
use std::rc::Rc;

/// Simple in-memory database with WAL
pub struct SimpleDb {
    tables: RefCell<HashMap<String, Table>>,
    wal: RefCell<Wal>,
}

struct Table {
    rows: HashMap<RowId, Vec<u8>>,
    next_id: u64,
}

struct Wal {
    file: File,
    offset: u64,
}

impl SimpleDb {
    pub fn open(path: &Path) -> Result<Self> {
        let wal_path = path.join("wal.bin");
        let wal_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&wal_path)?;

        let mut db = Self {
            tables: RefCell::new(HashMap::new()),
            wal: RefCell::new(Wal {
                file: wal_file,
                offset: 0,
            }),
        };

        // Replay WAL
        db.replay()?;

        Ok(db)
    }

    pub fn create_table(&self, name: String) -> Result<()> {
        let mut tables = self.tables.borrow_mut();
        if tables.contains_key(&name) {
            return Err("Table exists");
        }

        tables.insert(name, Table {
            rows: HashMap::new(),
            next_id: 0,
        });

        // Log to WAL
        self.log(WalEntry::CreateTable(name))?;

        Ok(())
    }

    pub fn insert(&self, table_name: &str, data: Vec<u8>) -> Result<RowId> {
        let mut tables = self.tables.borrow_mut();
        let table = tables.get_mut(table_name)
            .ok_or("Table not found")?;

        let row_id = RowId(table.next_id);
        table.next_id += 1;

        // Log to WAL first
        self.log(WalEntry::Insert {
            table: table_name.to_string(),
            row_id,
            data: data.clone(),
        })?;

        // Then apply to memory
        table.rows.insert(row_id, data);

        Ok(row_id)
    }

    pub fn get(&self, table_name: &str, row_id: RowId) -> Option<Vec<u8>> {
        let tables = self.tables.borrow();
        tables.get(table_name)
            .and_then(|table| table.rows.get(&row_id).cloned())
    }

    fn log(&self, entry: WalEntry) -> Result<()> {
        let mut wal = self.wal.borrow_mut();
        let bytes = bincode::serialize(&entry).unwrap();

        let len = bytes.len() as u32;
        wal.file.write_all(&len.to_be_bytes())?;
        wal.file.write_all(&bytes)?;
        wal.file.sync_all()?;

        wal.offset += 4 + bytes.len() as u64;

        Ok(())
    }

    fn replay(&mut self) -> Result<()> {
        let mut wal = self.wal.borrow_mut();
        wal.file.seek(SeekFrom::Start(0))?;

        let mut len_buf = [0u8; 4];

        loop {
            if wal.file.read_exact(&mut len_buf).is_err() {
                break;
            }

            let len = u32::from_be_bytes(len_buf) as usize;
            let mut bytes = vec![0u8; len];

            if wal.file.read_exact(&mut bytes).is_err() {
                break;
            }

            let entry: WalEntry = bincode::deserialize(&bytes).unwrap();

            match entry {
                WalEntry::CreateTable(name) => {
                    let mut tables = self.tables.borrow_mut();
                    tables.insert(name, Table {
                        rows: HashMap::new(),
                        next_id: 0,
                    });
                }
                WalEntry::Insert { table, row_id, data } => {
                    let mut tables = self.tables.borrow_mut();
                    if let Some(tbl) = tables.get_mut(&table) {
                        tbl.rows.insert(row_id, data);
                    }
                }
            }
        }

        Ok(())
    }
}

enum WalEntry {
    CreateTable(String),
    Insert { table: String, row_id: RowId, data: Vec<u8> },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RowId(u64);
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial Rust revision guide created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
