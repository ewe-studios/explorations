# SpacetimeDB Storage Engine Deep Dive

## Overview

This document explores the storage engine implementation in SpacetimeDB, covering:
- In-memory table structures
- Row storage layout
- Index implementations
- Commitlog (WAL) persistence
- Page management and memory mapping

---

## 1. In-Memory Table Architecture

### 1.1 Table Structure

```rust
// Conceptual structure based on SpacetimeDB's table crate
struct Table {
    /// Table schema (column names, types, constraints)
    schema: TableSchema,

    /// Row data stored in pages
    heap: RowHeap,

    /// Indexes for fast lookups
    indexes: Vec<Index>,

    /// Constraints (unique, foreign key)
    constraints: Vec<Constraint>,

    /// Statistics for query optimization
    stats: TableStats,
}
```

**Key insight:** SpacetimeDB keeps ALL table data in memory. This eliminates disk I/O for reads but requires efficient memory management.

### 1.2 Row Storage Layout

```
Row Layout in Memory:
┌───────────────────────────────────────────────────────┐
│ Fixed-length portion (if all columns fixed)          │
│ ┌──────┬──────┬──────┬──────┐                        │
│ │col 0 │col 1 │col 2 │col 3 │ ...                    │
│ │(i64) │(i64) │(bool)│(u32) │                        │
│ └──────┴──────┴──────┴──────┘                        │
└───────────────────────────────────────────────────────┘

Variable-length Row Layout:
┌───────────────────────────────────────────────────────┐
│ Header (8 bytes)                                      │
│ - Row ID (4 bytes)                                    │
│ - Offsets array pointer (4 bytes)                     │
├───────────────────────────────────────────────────────┤
│ Fixed columns                                         │
│ ┌──────┬──────┬──────┐                               │
│ │col 0 │col 1 │col 2 │                               │
│ │(i64) │(i64) │(bool)│                               │
│ └──────┴──────┴──────┘                               │
├───────────────────────────────────────────────────────┤
│ Offsets array                                         │
│ [offset0, offset1, offset2, ...]                     │
├───────────────────────────────────────────────────────┤
│ Variable-length data                                  │
│ ┌─────────┬─────────┬─────────┐                      │
│ │String 1 │String 2 │ Array   │ ...                  │
│ └─────────┴─────────┴─────────┘                      │
└───────────────────────────────────────────────────────┘
```

### 1.3 Page-Based Memory Management

```rust
// Page-based organization (inspired by SpacetimeDB's page management)
struct Page {
    /// Page data buffer
    data: [u8; PAGE_SIZE],  // Typically 4KB or 8KB

    /// Number of bytes used
    used: usize,

    /// Page type (data, index, overflow)
    page_type: PageType,
}

struct PageManager {
    /// Allocated pages
    pages: Vec<Page>,

    /// Free list for reuse
    free_list: Vec<PageId>,

    /// Dirty pages (modified, need persistence)
    dirty_pages: HashSet<PageId>,
}

impl PageManager {
    /// Allocate a new page or reuse from free list
    fn allocate(&mut self) -> PageId {
        if let Some(id) = self.free_list.pop() {
            self.pages[id.0].used = 0;
            id
        } else {
            let id = PageId(self.pages.len());
            self.pages.push(Page::new());
            id
        }
    }

    /// Mark page as dirty (modified)
    fn mark_dirty(&mut self, page_id: PageId) {
        self.dirty_pages.insert(page_id);
    }
}
```

**Why pages?**
- Efficient memory allocation (batch operations)
- Easy to flush to disk for persistence
- Cache-friendly access patterns
- Simplifies compaction and garbage collection

---

## 2. Index Implementations

### 2.1 B-Tree Index

```rust
// B-Tree index for ordered lookups and range scans
struct BTreeIndex {
    /// Root page ID
    root: PageId,

    /// Column being indexed
    column: ColumnId,

    /// Page manager for B-tree pages
    page_mgr: PageManager,

    /// Number of entries
    cardinality: u64,
}

// B-Tree node structure
struct BTreeNode {
    /// Internal node or leaf
    node_type: NodeType,

    /// Keys in this node (sorted)
    keys: Vec<DbValue>,

    /// Child pointers (for internal nodes)
    children: Vec<PageId>,

    /// Row IDs (for leaf nodes)
    row_ids: Vec<RowId>,

    /// Next leaf (for range scans)
    next_leaf: Option<PageId>,
}

impl BTreeIndex {
    /// Insert key -> row_id mapping
    fn insert(&mut self, key: DbValue, row_id: RowId) -> Result<()> {
        // Find leaf node
        let leaf = self.find_leaf(&key)?;

        // Insert into leaf
        let node = &mut self.page_mgr.pages[leaf.0];
        node.insert_sorted(key, row_id);

        // Check if split needed
        if node.is_full() {
            self.split_node(leaf)?;
        }

        Ok(())
    }

    /// Point lookup - find row_ids for exact key
    fn lookup(&self, key: DbValue) -> Vec<RowId> {
        let mut node_id = self.root;

        // Descend tree
        loop {
            let node = &self.page_mgr.pages[node_id.0];

            match node.node_type {
                NodeType::Internal => {
                    // Find child to descend into
                    let child_idx = node.find_child_index(&key);
                    node_id = node.children[child_idx];
                }
                NodeType::Leaf => {
                    // Found leaf, search for key
                    return node.lookup(key);
                }
            }
        }
    }

    /// Range scan - find all keys in range [start, end)
    fn range_scan(&self, start: DbValue, end: DbValue) -> Vec<RowId> {
        let mut results = Vec::new();

        // Find starting leaf
        let mut leaf = self.find_leaf(&start);

        // Scan leaves until we pass end
        while let Some(node) = self.page_mgr.pages.get(leaf.0) {
            for (key, row_id) in &node.entries {
                if key >= &end {
                    return results;
                }
                if key >= &start {
                    results.push(*row_id);
                }
            }

            // Move to next leaf
            leaf = node.next_leaf?;
        }

        results
    }
}
```

**B-Tree complexity:**
| Operation | Time Complexity |
|-----------|-----------------|
| Point lookup | O(log_b n) where b = branching factor |
| Range scan | O(log_b n + k) where k = results |
| Insert | O(log_b n) |
| Delete | O(log_b n) |

### 2.2 Hash Index

```rust
// Hash index for O(1) point lookups
struct HashIndex {
    /// Hash map: hash bucket -> entries
    buckets: Vec<Bucket>,

    /// Number of buckets (power of 2)
    num_buckets: usize,

    /// Load factor threshold for resizing
    load_factor: f64,

    /// Total entries
    entries: u64,
}

struct Bucket {
    /// Entries in this bucket (can be multiple due to collisions)
    entries: Vec<(u64, RowId)>,  // (hash, row_id)
}

impl HashIndex {
    /// Insert into hash index
    fn insert(&mut self, key: DbValue, row_id: RowId) {
        let hash = self.hash_value(&key);
        let bucket_idx = hash as usize % self.num_buckets;

        self.buckets[bucket_idx].entries.push((hash, row_id));
        self.entries += 1;

        // Check if resize needed
        if self.entries as f64 / self.num_buckets as f64 > self.load_factor {
            self.resize();
        }
    }

    /// O(1) average lookup
    fn lookup(&self, key: DbValue) -> Vec<RowId> {
        let hash = self.hash_value(&key);
        let bucket_idx = hash as usize % self.num_buckets;

        self.buckets[bucket_idx]
            .entries
            .iter()
            .filter(|(h, _)| *h == hash)  // Handle hash collisions
            .map(|(_, row_id)| *row_id)
            .collect()
    }

    fn hash_value(&self, key: DbValue) -> u64 {
        // Use ahash or xxhash for fast hashing
        use ahash::AHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = AHasher::default();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Double bucket count and rehash all entries
    fn resize(&mut self) {
        let old_buckets = std::mem::take(&mut self.buckets);
        self.num_buckets *= 2;
        self.buckets.resize(self.num_buckets, Bucket { entries: Vec::new() });

        for bucket in old_buckets {
            for (hash, row_id) in bucket.entries {
                let new_idx = hash as usize % self.num_buckets;
                self.buckets[new_idx].entries.push((hash, row_id));
            }
        }
    }
}
```

**Hash vs B-Tree:**

| Aspect | Hash Index | B-Tree Index |
|--------|------------|--------------|
| Point lookup | O(1) average | O(log n) |
| Range scan | NOT supported | O(log n + k) |
| Ordered iteration | NOT supported | Supported |
| Memory | Hash table overhead | Tree node overhead |
| Best for | Equality predicates | Range predicates |

### 2.3 Composite Indexes

```rust
// Composite index on multiple columns
struct CompositeIndex {
    /// Indexed columns (order matters!)
    columns: Vec<ColumnId>,

    /// Underlying index (B-tree or hash)
    inner: IndexImpl,
}

// Composite key for multi-column index
struct CompositeKey {
    values: Vec<DbValue>,
}

impl Ord for CompositeKey {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lexicographic comparison
        for (a, b) in self.values.iter().zip(&other.values) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                other => return other,
            }
        }
        Ordering::Equal
    }
}

impl CompositeIndex {
    /// Create composite key from row
    fn make_key(&self, row: &Row) -> CompositeKey {
        CompositeKey {
            values: self.columns.iter().map(|col| row.get(*col)).collect(),
        }
    }

    /// Query with prefix (leftmost columns)
    fn prefix_lookup(&self, prefix: Vec<DbValue>) -> Vec<RowId> {
        // Can use index if query matches leftmost columns
        // SELECT * WHERE col0 = ? AND col1 = ?  ✓ Uses index
        // SELECT * WHERE col1 = ?               ✗ Cannot use index
        self.inner.range_search(prefix, prefix_with_max_value())
    }
}
```

**Composite index rule:** Queries can use the index only if they filter on the **leftmost** columns in order.

---

## 3. Commitlog (WAL) Implementation

### 3.1 WAL Structure

```
Commitlog File Format:
┌─────────────────────────────────────────────────────────┐
│ Header (32 bytes)                                       │
│ - Magic: "STDLOG" (8 bytes)                            │
│ - Version (4 bytes)                                     │
│ - Page size (4 bytes)                                   │
│ - Checkpoint offset (8 bytes)                           │
│ - Checksum (8 bytes)                                    │
├─────────────────────────────────────────────────────────┤
│ Frame 1                                                 │
│ - Frame length (4 bytes)                                │
│ - Transaction ID (8 bytes)                              │
│ - Timestamp (8 bytes)                                   │
│ - Payload length (4 bytes)                              │
│ - Payload (variable)                                    │
│ - CRC32 checksum (4 bytes)                              │
├─────────────────────────────────────────────────────────┤
│ Frame 2                                                 │
│ ...                                                     │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Commitlog Implementation

```rust
use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek, SeekFrom};
use std::path::PathBuf;

struct Commitlog {
    /// Path to commitlog file
    path: PathBuf,

    /// File handle
    file: File,

    /// Current write offset
    write_offset: u64,

    /// Current transaction ID
    current_txid: u64,

    /// Buffer for batching writes
    write_buffer: Vec<u8>,
}

#[derive(Debug, Clone)]
struct CommitRecord {
    txid: u64,
    timestamp: u64,
    payload: Vec<u8>,
}

impl Commitlog {
    /// Open or create commitlog
    fn open(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        let metadata = file.metadata()?;
        let write_offset = metadata.len();

        Ok(Self {
            path,
            file,
            write_offset,
            current_txid: 0,
            write_buffer: Vec::with_capacity(4096),
        })
    }

    /// Append a record to the commitlog
    fn append(&mut self, payload: Vec<u8>) -> Result<u64> {
        self.current_txid += 1;
        let txid = self.current_txid;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;

        // Serialize record
        let record = CommitRecord {
            txid,
            timestamp,
            payload,
        };

        let bytes = self.serialize_record(&record);

        // Write to buffer
        self.write_buffer.extend_from_slice(&bytes);

        // Flush buffer if large enough
        if self.write_buffer.len() >= 8192 {
            self.flush()?;
        }

        Ok(txid)
    }

    /// Flush write buffer to disk
    fn flush(&mut self) -> Result<()> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }

        // Write buffered data
        self.file.seek(SeekFrom::Start(self.write_offset))?;
        self.file.write_all(&self.write_buffer)?;

        // Ensure durability (fsync)
        self.file.sync_all()?;

        self.write_offset += self.write_buffer.len() as u64;
        self.write_buffer.clear();

        Ok(())
    }

    /// Replay commitlog from beginning or checkpoint
    fn replay(&mut self, from_offset: u64) -> Result<Vec<CommitRecord>> {
        let mut records = Vec::new();

        self.file.seek(SeekFrom::Start(from_offset))?;

        loop {
            // Try to read frame header
            let mut length_buf = [0u8; 4];
            if self.file.read_exact(&mut length_buf).is_err() {
                break;  // End of file or partial write
            }

            let frame_length = u32::from_be_bytes(length_buf) as usize;

            // Read frame payload
            let mut payload = vec![0u8; frame_length];
            if self.file.read_exact(&mut payload).is_err() {
                break;  // Partial write (crash during write)
            }

            // Read checksum
            let mut checksum_buf = [0u8; 4];
            if self.file.read_exact(&mut checksum_buf).is_err() {
                break;
            }

            // Verify checksum
            let stored_checksum = u32::from_be_bytes(checksum_buf);
            let computed_checksum = crc32c::crc32c(&payload);

            if stored_checksum != computed_checksum {
                // Corrupted record, stop replay
                log::warn!("Checksum mismatch at offset {}", self.file.stream_position()?);
                break;
            }

            // Deserialize and add to replay list
            let record = self.deserialize_record(&payload)?;
            records.push(record);
        }

        Ok(records)
    }

    fn serialize_record(&self, record: &CommitRecord) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Frame length
        let payload = bincode::serialize(&record).unwrap();
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());

        // Payload
        bytes.extend_from_slice(&payload);

        // Checksum
        let checksum = crc32c::crc32c(&payload);
        bytes.extend_from_slice(&checksum.to_be_bytes());

        bytes
    }
}
```

### 3.3 Checkpoint Mechanism

```rust
struct Checkpoint {
    /// Commitlog offset at checkpoint time
    commitlog_offset: u64,

    /// Snapshot of all table data
    snapshot: TableSnapshot,

    /// Timestamp
    timestamp: u64,
}

impl Commitlog {
    /// Create a checkpoint
    fn create_checkpoint(&mut self, tables: &TableRegistry) -> Result<Checkpoint> {
        // Flush all pending writes
        self.flush()?;

        // Record current offset
        let offset = self.write_offset;

        // Create snapshot of all tables
        let snapshot = TableSnapshot {
            tables: tables
                .iter()
                .map(|table| (table.id, table.snapshot()))
                .collect(),
        };

        // Write checkpoint marker to commitlog
        let checkpoint_record = CheckpointRecord {
            offset,
            snapshot_data: bincode::serialize(&snapshot)?,
        };

        self.append(bincode::serialize(&checkpoint_record)?)?;

        Ok(Checkpoint {
            commitlog_offset: offset,
            snapshot,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64,
        })
    }

    /// Load from checkpoint and replay
    fn recover(&mut self) -> Result<TableRegistry> {
        // Find latest checkpoint
        let checkpoint = self.find_latest_checkpoint()?;

        // Load snapshot
        let mut tables = checkpoint.snapshot.restore();

        // Replay commitlog from checkpoint
        let records = self.replay(checkpoint.commitlog_offset)?;

        for record in records {
            if let Some(op) = self.deserialize_operation(&record.payload)? {
                tables.apply_operation(op)?;
            }
        }

        Ok(tables)
    }
}
```

---

## 4. SpacetimeDB-Specific Optimizations

### 4.1 Row Heap with Pointer Compression

```rust
// SpacetimeDB uses pointer compression for large tables
struct RowHeap {
    /// Data pages (compressed pointers)
    pages: Vec<HeapPage>,

    /// Free space map for each page
    free_space: Vec<u16>,
}

struct HeapPage {
    /// Page data
    data: Vec<u8>,

    /// Row offsets within page (compressed)
    row_offsets: Vec<u16>,  // Relative offsets, not absolute
}

impl RowHeap {
    /// Insert row, return compressed RowId
    fn insert(&mut self, row: &[u8]) -> RowId {
        // Find page with enough space
        let page_idx = self.find_page_with_space(row.len());

        let page = &mut self.pages[page_idx];
        let offset = page.data.len() as u16;

        page.data.extend_from_slice(row);
        page.row_offsets.push(offset);

        RowId {
            page: page_idx as u32,
            offset: offset,
        }
    }

    /// Get row by compressed RowId
    fn get(&self, row_id: RowId) -> &[u8] {
        let page = &self.pages[row_id.page as usize];
        let offset = page.row_offsets[row_id.offset as usize];

        // Variable-length row: read length prefix
        let len = u16::from_le_bytes([
            page.data[offset as usize],
            page.data[(offset + 1) as usize],
        ]) as usize;

        &page.data[(offset + 2) as usize..(offset + 2 + len) as usize]
    }
}
```

### 4.2 Incremental View Maintenance

```rust
// SpacetimeDB maintains query results incrementally
struct SubscriptionIndex {
    /// Query -> Subscribers
    subscriptions: HashMap<QueryId, Subscription>,

    /// Materialized results for each subscription
    results: HashMap<QueryId, MaterializedResult>,
}

struct Subscription {
    /// The SQL query
    query: SqlQuery,

    /// Compiled query plan
    plan: QueryPlan,

    /// Subscribers (clients)
    clients: Vec<ClientId>,
}

impl SubscriptionIndex {
    /// Update subscription results when table changes
    fn on_table_update(&mut self, table_id: TableId, row: &Row, op: Operation) {
        for (query_id, subscription) in &mut self.subscriptions {
            if subscription.affects_query(table_id, row) {
                let result = &mut self.results[query_id];

                match op {
                    Operation::Insert => {
                        if subscription.query.matches(row) {
                            result.insert(row.clone());
                        }
                    }
                    Operation::Delete => {
                        result.remove(row.id());
                    }
                    Operation::Update => {
                        if subscription.query.matches(row) {
                            result.insert(row.clone());
                        } else {
                            result.remove(row.id());
                        }
                    }
                }
            }
        }
    }
}
```

**Why this matters:** Instead of re-running queries on every change, SpacetimeDB incrementally updates materialized results. This is crucial for real-time sync.

---

## 5. Memory-Mapped I/O for Commitlog

```rust
use memmap2::MmapMut;

struct MmapCommitlog {
    /// Memory-mapped file
    mmap: MmapMut,

    /// Current write position
    write_pos: usize,

    /// File size
    file_size: usize,
}

impl MmapCommitlog {
    /// Open commitlog with memory mapping
    fn open(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        // Set initial file size (can grow)
        file.set_len(1024 * 1024 * 1024)?;  // 1GB

        // Memory map the file
        let mmap = unsafe { MmapMut::map_mut(&file)? };

        Ok(Self {
            mmap,
            write_pos: 0,
            file_size: 1024 * 1024 * 1024,
        })
    }

    /// Write to mmap (no kernel syscall!)
    fn append(&mut self, data: &[u8]) {
        self.mmap[self.write_pos..self.write_pos + data.len()]
            .copy_from_slice(data);
        self.write_pos += data.len();

        // Advise kernel about access pattern
        // madvise(MADV_SEQUENTIAL) could help here
    }

    /// Sync mmap to disk
    fn sync(&self) -> Result<()> {
        self.mmap.flush()?;
        Ok(())
    }
}
```

**Benefits of mmap:**
- No read() syscall overhead
- Kernel handles paging automatically
- Copy-on-write for snapshots
- Efficient for sequential writes

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial storage engine deep dive created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
