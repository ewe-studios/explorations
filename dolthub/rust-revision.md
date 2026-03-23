# Rust Revision: Reproducing Dolt in Rust

## Executive Summary

This guide outlines how to reproduce Dolt's functionality at production level in Rust. Dolt is ~220K LOC in Go; a Rust implementation could achieve similar functionality in ~150-170K LOC by leveraging existing crates and learning from Dolt's architecture.

**Estimated Effort:** 12-18 months for a team of 3-4 experienced Rust developers

**Key Challenges:**
1. Content-addressed storage with chunking
2. Prolly tree (B+ tree) implementation
3. Three-way merge algorithm
4. SQL engine integration

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│              Wire Protocol Layer                     │
│         (tokio-postgres / mysql_async)              │
├─────────────────────────────────────────────────────┤
│              SQL Parser (sqlparser-rs)              │
├─────────────────────────────────────────────────────┤
│              Query Planner & Executor               │
├─────────────────────────────────────────────────────┤
│           Version Control Layer                      │
│    (Commits, Branches, Merge, Diff)                │
├─────────────────────────────────────────────────────┤
│              Storage Engine                          │
│         (Prolly Trees + Chunk Store)                │
└─────────────────────────────────────────────────────┘
```

## Phase 1: Storage Foundation (Months 1-4)

### 1.1 Chunk Store

**Goal:** Content-addressed storage layer

```rust
// Core traits
pub trait ChunkStore: Send + Sync {
    fn get(&self, hash: &ChunkHash) -> Result<Chunk>;
    fn put(&mut self, chunk: Chunk) -> Result<ChunkHash>;
    fn has(&self, hash: &ChunkHash) -> Result<bool>;
    fn delete(&mut self, hash: &ChunkHash) -> Result<()>;
}

pub struct ChunkHash([u8; 20]); // SHA-1

pub struct Chunk {
    hash: ChunkHash,
    data: Bytes,
}
```

**Key Crates:**
```toml
[dependencies]
sha1 = "0.10"
bytes = "1.5"
tokio = { version = "1", features = ["full"] }
```

**Implementation Steps:**

1. **In-memory store** (for testing)
```rust
pub struct MemoryStore {
    chunks: DashMap<ChunkHash, Chunk>,
}
```

2. **File-based store**
```rust
pub struct FileStore {
    dir: PathBuf,
    cache: ChunkCache,
}
```

3. **Table file format** (like NBS)
```rust
// Table file structure:
// - Chunk block (compressed)
// - Index block (hash → offset)
// - Footer (metadata, checksum)
```

### 1.2 Prolly Tree Implementation

**Goal:** B+ tree with content-defined chunking

```rust
pub struct ProllyMap<K, V> {
    root: NodeId,
    store: Arc<dyn ChunkStore>,
    key_ord: KeyOrdering<K>,
}

pub enum Node {
    Internal(InternalNode),
    Leaf(LeafNode),
}

pub struct InternalNode {
    entries: Vec<Entry>,
    level: u8,
}

pub struct LeafNode {
    entries: Vec<LeafEntry>,
}
```

**Key Operations:**

```rust
impl<K, V> ProllyMap<K, V> {
    pub async fn get(&self, key: &K) -> Result<Option<V>>;
    pub async fn insert(&mut self, key: K, value: V) -> Result<()>;
    pub async fn remove(&mut self, key: &K) -> Result<Option<V>>;
    pub async fn iter_range(&self, range: Range<K>) -> Result<Iter<'_>>;
}
```

**Chunking Algorithm:**

```rust
// Content-defined chunking with rolling hash
pub struct Chunker {
    rolling_hash: RollingHash,
    threshold: u64,
    buffer: Vec<u8>,
}

impl Chunker {
    pub fn add(&mut self, data: &[u8]) -> Option<Chunk> {
        for byte in data {
            self.rolling_hash.update(*byte);
            self.buffer.push(*byte);

            // Create chunk boundary when hash meets threshold
            if self.rolling_hash.value() % self.threshold == 0 {
                return Some(self.flush());
            }
        }
        None
    }
}
```

**Key Crates:**
```toml
[dependencies]
twox-hash = "1.6"      # Rolling hash
serde = { version = "1", features = ["derive"] }
```

### 1.3 Tuple Encoding

**Goal:** Type-safe tuple storage

```rust
pub struct Tuple {
    data: Bytes,
    desc: TupleDescriptor,
}

pub struct TupleDescriptor {
    types: Vec<ColumnType>,
}

pub enum ColumnType {
    Int8,
    Int32,
    Text,
    Timestamp,
    // ... more types
}
```

**Encoding:**

```rust
pub trait TupleEncoder {
    fn encode(&self, tuple: &Tuple) -> Result<Bytes>;
    fn decode(&self, data: Bytes) -> Result<Tuple>;
}

// FlatBuffers-based encoding
pub struct FlatBufferEncoder;
```

**Key Crates:**
```toml
[dependencies]
flatbuffers = "23"
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = "1.33"
```

## Phase 2: Version Control (Months 5-8)

### 2.1 RootValue

**Goal:** Database state representation

```rust
pub struct RootValue {
    tables: HashMap<TableName, TableHash>,
    foreign_keys: ForeignKeyCollection,
    feature_version: u64,
    collation: Collation,
    hash: RootHash,
}

impl RootValue {
    pub fn get_table(&self, name: &str) -> Option<&TableHash>;
    pub fn put_table(&mut self, name: TableName, hash: TableHash);
    pub fn remove_table(&mut self, name: &str);
}
```

### 2.2 Commit Graph

**Goal:** Commit history management

```rust
pub struct Commit {
    root_hash: RootHash,
    parents: Vec<CommitHash>,
    metadata: CommitMetadata,
    height: u64,
}

pub struct CommitMetadata {
    name: String,
    email: String,
    message: String,
    timestamp: i64,
}

pub struct CommitGraph {
    store: Arc<dyn ChunkStore>,
}

impl CommitGraph {
    pub fn get_commit(&self, hash: &CommitHash) -> Result<Commit>;
    pub fn add_commit(&mut self, commit: Commit) -> Result<CommitHash>;
    pub fn get_ancestors(&self, hash: &CommitHash) -> Result<Vec<CommitHash>>;
}
```

### 2.3 Branch Management

**Goal:** Lightweight branch references

```rust
pub struct BranchRef {
    name: String,
    commit_hash: CommitHash,
}

pub struct RefStore {
    refs_dir: PathBuf,
}

impl RefStore {
    pub fn get_branch(&self, name: &str) -> Result<Option<BranchRef>>;
    pub fn set_branch(&mut self, branch: BranchRef) -> Result<()>;
    pub fn delete_branch(&mut self, name: &str) -> Result<()>;
    pub fn list_branches(&self) -> Result<Vec<BranchRef>>;
}
```

### 2.4 Three-Way Diff

**Goal:** Efficient diff algorithm

```rust
pub enum DiffType {
    Added,
    Removed,
    Modified,
}

pub struct Diff {
    diff_type: DiffType,
    key: Tuple,
    from_value: Option<Tuple>,
    to_value: Option<Tuple>,
}

pub fn diff_maps<K, V>(
    ctx: &Context,
    from: &ProllyMap<K, V>,
    to: &ProllyMap<K, V>,
    mut callback: impl FnMut(Diff),
) -> Result<()> {
    // Skip subtrees with equal hashes
    if from.root_hash() == to.root_hash() {
        return Ok(());
    }

    // Recursively diff
    diff_nodes(from.root(), to.root(), callback)
}
```

### 2.5 Three-Way Merge

**Goal:** Merge algorithm with conflict detection

```rust
pub struct MergeResult {
    merged_root: RootValue,
    conflicts: Vec<Conflict>,
    stats: MergeStats,
}

pub struct Conflict {
    table: TableName,
    kind: ConflictKind,
    ancestor: Option<Row>,
    ours: Option<Row>,
    theirs: Option<Row>,
}

pub fn merge_roots(
    ctx: &Context,
    our_root: &RootValue,
    their_root: &RootValue,
    ancestor_root: &RootValue,
) -> Result<MergeResult> {
    let mut conflicts = Vec::new();
    let mut merged_tables = HashMap::new();

    // Find all tables across all roots
    let all_tables = collect_all_tables(our_root, their_root, ancestor_root);

    for table_name in all_tables {
        let merge_result = merge_table(
            ctx,
            our_root.get_table(table_name),
            their_root.get_table(table_name),
            ancestor_root.get_table(table_name),
        )?;

        merged_tables.insert(table_name, merge_result.table);
        conflicts.extend(merge_result.conflicts);
    }

    Ok(MergeResult {
        merged_root: RootValue::new(merged_tables),
        conflicts,
        stats: compute_stats(&merged_tables),
    })
}
```

**Merge Logic:**

```rust
fn merge_table(
    ctx: &Context,
    ours: Option<&Table>,
    theirs: Option<&Table>,
    ancestor: Option<&Table>,
) -> Result<TableMergeResult> {
    match (ours, theirs, ancestor) {
        // No changes
        (Some(o), Some(t), Some(a))
            if o.hash() == a.hash() && t.hash() == a.hash() =>
        {
            Ok(TableMergeResult::unchanged(o.clone()))
        }

        // Only ours changed
        (Some(o), Some(t), Some(a))
            if o.hash() != a.hash() && t.hash() == a.hash() =>
        {
            Ok(TableMergeResult::merged(o.clone()))
        }

        // Only theirs changed
        (Some(o), Some(t), Some(a))
            if o.hash() == a.hash() && t.hash() != a.hash() =>
        {
            Ok(TableMergeResult::merged(t.clone()))
        }

        // Both changed - need row-level merge
        (Some(o), Some(t), Some(a)) => {
            merge_row_data(ctx, o, t, a)
        }

        // Table added in both branches
        (Some(o), Some(t), None) => {
            merge_added_tables(o, t)
        }

        // Table deleted in one branch, modified in other
        (None, Some(t), Some(a)) if t.hash() != a.hash() => {
            Ok(TableMergeResult::conflict(Conflict::table_deleted_modified()))
        }

        // ... more cases
    }
}
```

## Phase 3: SQL Integration (Months 9-12)

### 3.1 SQL Parser Integration

**Goal:** Use sqlparser-rs for parsing

```rust
use sqlparser::ast::{Statement, Query, Expr};
use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect};
use sqlparser::parser::Parser;

pub struct SqlParser {
    dialect: Box<dyn Dialect>,
}

impl SqlParser {
    pub fn parse(&self, sql: &str) -> Result<Statement> {
        Parser::parse_sql(&*self.dialect, sql)
            .map_err(|e| Error::ParseError(e.to_string()))
    }
}
```

**Key Crates:**
```toml
[dependencies]
sqlparser = { version = "0.43", features = ["visitor"] }
```

### 3.2 Query Planner

**Goal:** Transform AST to execution plan

```rust
pub enum PlanNode {
    Project(ProjectNode),
    Filter(FilterNode),
    Join(JoinNode),
    TableScan(TableScanNode),
    Aggregate(AggregateNode),
    Sort(SortNode),
    Limit(LimitNode),
}

pub trait PlanNodeTrait {
    fn schema(&self) -> &Schema;
    fn children(&self) -> Vec<&PlanNode>;
    fn execute(&self, ctx: &ExecutionContext) -> Result<RowIter>;
}

pub struct QueryPlanner {
    analyzer: Analyzer,
}

impl QueryPlanner {
    pub fn plan(&self, statement: Statement) -> Result<PlanNode> {
        // 1. Analyze and resolve tables/columns
        let analyzed = self.analyzer.analyze(statement)?;

        // 2. Create execution plan
        let plan = self.create_plan(analyzed)?;

        // 3. Optimize plan
        let optimized = self.optimize(plan)?;

        Ok(optimized)
    }
}
```

### 3.3 Type System

**Goal:** SQL type system

```rust
pub enum SqlType {
    Boolean,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    Decimal { precision: u8, scale: u8 },
    Text,
    Blob,
    Date,
    Time,
    Timestamp,
    Json,
}

pub struct Column {
    name: String,
    data_type: SqlType,
    nullable: bool,
    default: Option<Expr>,
}

pub struct Schema {
    columns: Vec<Column>,
    primary_key: Vec<usize>,
}
```

### 3.4 Execution Engine

**Goal:** Execute query plans

```rust
pub struct ExecutionContext {
    root_value: RootValue,
    transaction: Option<Transaction>,
    variables: HashMap<String, Value>,
}

pub trait RowIterator: Send {
    fn next(&mut self) -> Result<Option<Row>>;
}

impl PlanNode {
    pub fn execute(&self, ctx: &ExecutionContext) -> Result<Box<dyn RowIterator>> {
        match self {
            PlanNode::TableScan(scan) => {
                let table = ctx.root_value.get_table(&scan.table_name)?;
                Ok(Box::new(table.scan(ctx)))
            }
            PlanNode::Filter(filter) => {
                let child_iter = filter.child.execute(ctx)?;
                Ok(Box::new(FilterIterator::new(child_iter, &filter.predicate)))
            }
            // ... more nodes
        }
    }
}
```

## Phase 4: Wire Protocol (Months 13-15)

### 4.1 MySQL Protocol

```rust
use mysql_async::{prelude::*, Conn, Opts};

pub struct MySqlServer {
    port: u16,
    engine: Arc<SqlEngine>,
}

impl MySqlServer {
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.port)).await?;

        while let Ok((stream, addr)) = listener.accept().await {
            let engine = self.engine.clone();
            tokio::spawn(async move {
                handle_connection(stream, addr, engine).await;
            });
        }
        Ok(())
    }
}
```

**Key Crates:**
```toml
[dependencies]
mysql_async = "0.33"  # Or implement protocol directly
tokio = { version = "1", features = ["net", "io-util"] }
```

### 4.2 PostgreSQL Protocol

```rust
use tokio_postgres::{Client, Config, NoTls};

pub struct PostgresServer {
    port: u16,
    engine: Arc<SqlEngine>,
}
```

**Key Crates:**
```toml
[dependencies]
tokio-postgres = "0.7"
```

## Phase 5: System Tables & Procedures (Months 16-18)

### 5.1 System Tables

```rust
pub struct SystemTableProvider {
    engine: Arc<SqlEngine>,
}

impl SystemTableProvider {
    pub fn dolt_status(&self, ctx: &Context) -> Result<RowIter> {
        let working_set = ctx.get_working_set()?;
        let head = ctx.get_head()?;

        // Compute status
        let mut rows = Vec::new();
        for table in working_set.tables() {
            let status = if head.has_table(table) {
                if working_set.table_hash(table) != head.table_hash(table) {
                    "modified"
                } else {
                    continue; // No change
                }
            } else {
                "new table"
            };
            rows.push(Row::new(vec![table.into(), status.into()]));
        }
        Ok(RowIter::new(rows))
    }

    pub fn dolt_log(&self, ctx: &Context) -> Result<RowIter> {
        let commit_graph = ctx.get_commit_graph()?;
        let mut rows = Vec::new();

        for commit in commit_graph.get_ancestors(ctx.head_hash())? {
            rows.push(Row::new(vec![
                commit.hash().into(),
                commit.metadata().name.into(),
                commit.metadata().message.into(),
                commit.timestamp().into(),
            ]));
        }
        Ok(RowIter::new(rows))
    }
}
```

### 5.2 Stored Procedures

```rust
pub struct DoltProcedures;

impl DoltProcedures {
    pub fn dolt_add(ctx: &Context, tables: Vec<String>) -> Result<()> {
        let working_set = ctx.get_working_set()?;
        for table in tables {
            working_set.stage_table(&table)?;
        }
        Ok(())
    }

    pub fn dolt_commit(ctx: &Context, message: String) -> Result<CommitHash> {
        let working_set = ctx.get_working_set()?;
        let staged = working_set.get_staged_tables()?;

        let commit = Commit {
            root_hash: working_set.root_hash(),
            parents: vec![ctx.head_hash()],
            metadata: CommitMetadata {
                message,
                name: ctx.user_name(),
                email: ctx.user_email(),
                timestamp: now(),
            },
            height: compute_height(ctx.head_hash())?,
        };

        let commit_hash = ctx.commit_graph().add_commit(commit)?;
        ctx.set_head(commit_hash)?;
        working_set.clear_staged()?;

        Ok(commit_hash)
    }

    pub fn dolt_merge(ctx: &Context, branch: String) -> Result<MergeResult> {
        let our_head = ctx.get_head()?;
        let their_head = ctx.get_branch(&branch)?;
        let ancestor = find_merge_base(our_head, their_head)?;

        let our_root = ctx.get_root(our_head)?;
        let their_root = ctx.get_root(their_head)?;
        let ancestor_root = ctx.get_root(ancestor)?;

        let result = merge_roots(ctx, &our_root, &their_root, &ancestor_root)?;

        if result.has_conflicts() {
            ctx.set_conflicts(result.conflicts)?;
            return Ok(MergeResult::with_conflicts(result));
        }

        // Auto-commit merge
        let commit = Commit {
            root_hash: result.merged_root.hash(),
            parents: vec![our_head, their_head],
            metadata: CommitMetadata {
                message: format!("Merge branch '{}'", branch),
                name: ctx.user_name(),
                email: ctx.user_email(),
                timestamp: now(),
            },
            height: 0,
        };

        let commit_hash = ctx.commit_graph().add_commit(commit)?;
        ctx.set_head(commit_hash)?;

        Ok(MergeResult::success(commit_hash))
    }
}
```

## Project Structure

```
dolt-rs/
├── Cargo.toml
├── README.md
├── crates/
│   ├── dolt-store/          # Chunk store, prolly trees
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── chunk.rs     # Chunk types
│   │       ├── store.rs     # Store trait
│   │       ├── prolly/      # Prolly trees
│   │       └── tuple.rs     # Tuple encoding
│   ├── dolt-version/        # Version control
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── commit.rs    # Commit graph
│   │       ├── branch.rs    # Branch management
│   │       ├── diff.rs      # Diff algorithm
│   │       └── merge.rs     # Merge algorithm
│   ├── dolt-sql/            # SQL engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── parser.rs    # SQL parsing
│   │       ├── planner.rs   # Query planning
│   │       ├── executor.rs  # Query execution
│   │       └── types.rs     # Type system
│   ├── dolt-protocol/       # Wire protocols
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── mysql/       # MySQL protocol
│   │       └── postgres/    # Postgres protocol
│   └── dolt/                # Main binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs      # CLI entry point
│           ├── commands/    # CLI commands
│           └── server.rs    # SQL server
└── tests/
    ├── integration/         # Integration tests
    └── sqllogic/            # SQL logic tests
```

## Dependencies (Cargo.toml)

```toml
[workspace]
members = ["crates/*"]

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Serialization
flatbuffers = "23"
serde = { version = "1", features = ["derive"] }
bytes = "1.5"

# Cryptography
sha1 = "0.10"
digest = "0.10"

# Collections
dashmap = "5.5"
indexmap = "2.1"

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# SQL
sqlparser = { version = "0.43", features = ["visitor"] }
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = "1.33"

# Testing
proptest = "1.4"
criterion = "0.5"
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_store_put_get() {
        let mut store = MemoryStore::new();
        let chunk = Chunk::new(b"hello".to_vec());
        let hash = store.put(chunk.clone()).unwrap();

        let retrieved = store.get(&hash).unwrap();
        assert_eq!(chunk.data(), retrieved.data());
    }

    #[test]
    fn test_prolly_map_insert_get() {
        let mut map = ProllyMap::new();
        map.insert(key(1), value("a")).unwrap();
        map.insert(key(2), value("b")).unwrap();

        assert_eq!(map.get(&key(1)).unwrap(), Some(value("a")));
    }
}
```

### Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prolly_map_roundtrip(keys in vec(any::<u64>(), 1..100)) {
        let mut map = ProllyMap::new();
        for &key in &keys {
            map.insert(key, key * 2).unwrap();
        }

        for &key in &keys {
            prop_assert_eq!(map.get(&key).unwrap(), Some(key * 2));
        }
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_merge_workflow() {
    let repo = TempRepo::new();

    // Create initial commit
    repo.sql("CREATE TABLE users (id INT PRIMARY KEY, name TEXT)").await;
    repo.commit("Initial schema").await;

    // Create branch and make changes
    repo.branch("feature").await;
    repo.checkout("feature").await;
    repo.sql("INSERT INTO users VALUES (1, 'Alice')").await;
    repo.commit("Add Alice").await;

    // Switch back and make different changes
    repo.checkout("main").await;
    repo.sql("INSERT INTO users VALUES (2, 'Bob')").await;
    repo.commit("Add Bob").await;

    // Merge feature branch
    let result = repo.merge("feature").await;
    assert!(result.success);

    // Verify merged data
    let rows = repo.query("SELECT * FROM users ORDER BY id").await;
    assert_eq!(rows.len(), 2);
}
```

## Performance Considerations

### 1. Memory Management

```rust
// Use arena allocation for query execution
pub struct ExecutionArena {
    arena: Arena,
}

// Zero-copy deserialization with FlatBuffers
pub fn decode_row(data: &[u8]) -> Row {
    // No allocation needed
}
```

### 2. Parallel Execution

```rust
// Parallel table scans
pub async fn scan_tables_parallel(
    tables: Vec<TableRef>,
) -> Result<Vec<RowIter>> {
    try_join_all(tables.into_iter().map(|t| t.scan())).await
}
```

### 3. Batched Writes

```rust
pub struct BatchWriter {
    buffer: Vec<Edit>,
    threshold: usize,
}

impl BatchWriter {
    pub async fn write(&mut self, edit: Edit) -> Result<()> {
        self.buffer.push(edit);
        if self.buffer.len() >= self.threshold {
            self.flush().await?;
        }
        Ok(())
    }
}
```

## Migration Path from Dolt

If migrating existing Dolt databases:

1. **Compatibility Layer** - Read Dolt's chunk format
2. **Feature Version** - Support Dolt's feature versioning
3. **Gradual Migration** - Export/import via SQL dump

## Success Criteria

### Phase 1 (Storage)
- [ ] Chunk store with 95%+ test coverage
- [ ] Prolly map with basic operations
- [ ] Tuple encoding/decoding

### Phase 2 (Version Control)
- [ ] Commit graph operations
- [ ] Branch management
- [ ] Three-way diff
- [ ] Three-way merge with conflict detection

### Phase 3 (SQL)
- [ ] SQL parsing (90%+ MySQL/Postgres syntax)
- [ ] Query planning
- [ ] Basic execution (SELECT, INSERT, UPDATE, DELETE)
- [ ] JOIN support

### Phase 4 (Protocol)
- [ ] MySQL or PostgreSQL wire protocol
- [ ] Authentication
- [ ] Statement execution

### Phase 5 (System Tables)
- [ ] dolt_status
- [ ] dolt_log
- [ ] dolt_branches
- [ ] dolt_add, dolt_commit, dolt_merge procedures

## References

- [Dolt Architecture](../../src.dolthub/dolt/)
- [go-mysql-server](../../src.dolthub/go-mysql-server/)
- [Prolly Trees](../../src.dolthub/dolt/go/store/prolly/)
- [Versioned Storage Deep Dive](./versioned-storage-deep-dive.md)
