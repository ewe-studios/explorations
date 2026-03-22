# SpacetimeDB Storage Deep Dive

## Overview

This document explains how SpacetimeDB stores data on disk and in memory, including the file format, page structure, indexing strategies, and durability guarantees.

## Storage Hierarchy

```
┌─────────────────────────────────────────────────────────────┐
│ Level 1: Database (Logical)                                  │
│  - Multiple Tables                                           │
│  - Each table has indexes                                    │
│  - Schema metadata                                           │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ Level 2: Table (Physical)                                    │
│  - Pages (4KB each)                                         │
│  - Indexes (B-Tree, Hash)                                   │
│  - Blob Store (variable data)                               │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ Level 3: Page (Storage Unit)                                 │
│  - Header (metadata)                                        │
│  - Fixed Section (rows)                                     │
│  - Variable Section (granules)                              │
│  - Freelist (space reuse)                                   │
└─────────────────────────────────────────────────────────────┘
```

## Page Structure

### Page Layout (4KB)

```
  0                              PAGE_HEADER_SIZE                    PAGE_SIZE
  │────────────────────────────────────┬─────────────────────────────────────│
  │          Page Header               │           Page Data                 │
  │  - fixed_len (u16)                 │  ┌─────────────────────────────┐    │
  │  - var_len (u16)                   │  │  Fixed-Length Rows          │    │
  │  - fixed_free_head (FreeCellRef)   │  │  (grows downward)           │    │
  │  - var_free_head (FreeCellRef)     │  │                             │    │
  │  - checksum (u32)                  │  │  ... row data ...           │    │
  │                                    │  │                             │    │
  │                                    │  ├─────────────────────────────┤    │
  │                                    │  │  Variable Granules          │    │
  │                                    │  │  (grows upward)             │    │
  │                                    │  └─────────────────────────────┘    │
  └────────────────────────────────────┴─────────────────────────────────────┘
```

### Page Header Fields

| Field | Size | Purpose |
|-------|------|---------|
| `fixed_len` | u16 | Bytes used in fixed section |
| `var_len` | u16 | Bytes used in variable section |
| `fixed_free_head` | FreeCellRef | Head of fixed-len freelist |
| `var_free_head` | FreeCellRef | Head of variable-len freelist |
| `checksum` | u32 | CRC32C checksum for integrity |

### Row Pointer Structure

```rust
// 64-bit row pointer
struct RowPointer {
    page_index: PageIndex,   // Which page
    page_offset: PageOffset, // Offset within page
    squashed_offset: u8,     // TX_STATE or COMMITTED_STATE
}
```

## Row Storage Format

### BFLATN (Binary Flat) Format

SpacetimeDB uses a custom binary format called **BFLATN** for row storage:

```
Row in Page:
┌──────────────────────────────────────────────────────────┐
│  Fixed Portion (inline)                                  │
│  ┌─────────┬─────────┬─────────┬─────────┬────────────┐  │
│  │ col_0   │ col_1   │ col_2   │  ...    │ var_refs   │  │
│  │ (u64)   │ (u32)   │ (bool)  │         │ (u16 each) │  │
│  └─────────┴─────────┴─────────┴─────────┴────────────┘  │
└──────────────────────────────────────────────────────────┘
                          │
                          │ var_refs point to:
                          ▼
┌──────────────────────────────────────────────────────────┐
│  Variable Portion (blob store)                           │
│  ┌─────────────────┐  ┌─────────────────┐                │
│  │ String "hello"  │  │ Bytes [0xDEADBEEF]│              │
│  └─────────────────┘  └─────────────────┘                │
└──────────────────────────────────────────────────────────┘
```

### Variable Length Granules

Variable-length data is stored in **granules** with this structure:

```
Granule Header (8 bytes):
┌──────────────┬──────────────┐
│ prev_granule │ next_granule │
│ (u16)        │ (u16)        │
├──────────────┼──────────────┤
│    size      │   reserved   │
│    (u16)     │   (u16)      │
└──────────────┴──────────────┘

Granule Data:
┌────────────────────────────────┐
│  Actual variable data...       │
└────────────────────────────────┘
```

Granules are linked together for data larger than a single granule.

## Index Structures

### B-Tree Index

```rust
struct BTreeIndex {
    /// Column(s) indexed
    columns: ColList,
    /// Root page of B-Tree
    root_page: PageIndex,
    /// Unique constraint
    is_unique: bool,
}
```

**B-Tree Node Structure:**
```
Internal Node:
┌────────┬────────┬────────┬────────┬────────┐
│  key   │ child  │  key   │ child  │  ...   │
│  ptr   │  ptr   │  ptr   │  ptr   │        │
└────────┴────────┴────────┴────────┴────────┘

Leaf Node:
┌────────┬────────┬────────┬────────┐
│  key   │ row    │  key   │ row    │
│  ptr   │  ptr   │  ptr   │  ptr   │
└────────┴────────┴────────┴────────┘
```

### Hash Index

```rust
struct HashIndex {
    /// Column(s) indexed
    columns: ColList,
    /// Hash buckets
    buckets: Vec<Bucket>,
}
```

**Hash Bucket:**
```
┌────────┬────────┬────────┐
│ hash   │ row    │ next   │
│ (u64)  │ ptr    │ bucket │
└────────┴────────┴────────┘
```

### Pointer Map (Unique Constraint)

The `PointerMap` is a specialized index that ensures no duplicate rows:

```rust
struct PointerMap {
    /// Maps RowHash -> [RowPointer]
    map: HashMap<RowHash, Vec<RowPointer>>,
}

struct RowHash([u8; 32]); // Blake3 hash of row contents
```

This allows O(1) duplicate detection on insert.

## Commit Log (Write-Ahead Log)

### Segment Structure

```
Commit Log:
┌─────────────┬─────────────┬─────────────┬─────────────┐
│  Segment 0  │  Segment 1  │  Segment 2  │  ...        │
│  (closed)   │  (closed)   │  (active)   │             │
└─────────────┴─────────────┴─────────────┴─────────────┘
```

### Segment File Format

```
Segment Header (16 bytes):
┌──────────────┬──────────────┬──────────────┐
│ magic        │ version      │ prev_offset  │
│ (u64)        │ (u32)        │ (u64)        │
└──────────────┴──────────────┴──────────────┘

Commit Records:
┌──────────────┬──────────────┬──────────────┐
│ length       │ checksum     │ data         │
│ (u32)        │ (u32)        │ (compressed) │
└──────────────┴──────────────┴──────────────┘
```

### Compression

- Uses **zstd-framed** compression
- Each segment is independently compressed
- Frame size: typically 64KB

### Durability Guarantees

1. **Atomic commits** - All-or-nothing per reducer
2. **CRC32C checksums** - Detect corruption
3. **Sequential writes** - Append-only segments
4. **fsync on close** - Durability guarantee

## Snapshot System

### Snapshot Format

```
Snapshot Directory:
├── manifest.bin      # Metadata
├── table_0.bin       # Table data
├── table_1.bin
└── indexes/
    ├── index_0.bin
    └── ...
```

### Snapshot Creation Process

1. Acquire read lock on committed state
2. Serialize all tables to binary format
3. Write manifest with checksums
4. Release lock

### Incremental Snapshots

SpacetimeDB supports incremental snapshots by:
- Tracking modified pages since last snapshot
- Only writing changed data
- Using copy-on-write semantics

## Memory Management

### Page Pool

```rust
struct PagePool {
    /// Pre-allocated pages
    pages: Vec<Page>,
    /// Free list for reuse
    free_list: Vec<PageIndex>,
}
```

### Allocation Strategy

1. Check free list for existing page
2. If empty, allocate new page
3. Return to pool on deallocation (not OS free)

### Garbage Collection

Variable-length data uses reference counting:

```rust
struct BlobStore {
    /// Map of hash -> (ref_count, data)
    blobs: HashMap<BlobHash, BlobEntry>,
}

struct BlobEntry {
    ref_count: u32,
    data: Box<[u8]>,
}
```

When ref_count reaches 0, the blob is freed.

## Performance Optimizations

### 1. Zero-Copy Reads

Rows are accessed directly from pages without copying:

```rust
struct RowRef<'a> {
    ptr: RowPointer,
    table: &'a TableInner,
}

impl RowRef<'_> {
    fn read_column<T: Deserialize>(&self, col: ColId) -> T {
        // Direct memory access from page
        unsafe { ptr::read(...) }
    }
}
```

### 2. Batched Writes

Multiple row operations are batched into single commit:

```
Transaction:
  INSERT row1
  INSERT row2
  UPDATE row3
  DELETE row4
  ─────────────
  Single commit record
```

### 3. Index Caching

Frequently accessed index pages are cached in memory.

### 4. Prefetching

Sequential scans prefetch upcoming pages.

## Trade-offs

| Aspect | Choice | Trade-off |
|--------|--------|-----------|
| Storage | In-memory | Fast but limited by RAM |
| Durability | Commit log | Sequential writes only |
| Indexes | B-Tree + Hash | Memory overhead for dual indexes |
| Deduplication | PointerMap | Hash computation overhead |
| Compression | zstd | CPU cost for compression |
| Page size | 4KB | Standard but may fragment |

## Reproducing the Storage Format

To implement a compatible storage system:

### Step 1: Page Manager

```rust
pub struct PageManager {
    pages: Vec<Page>,
    free_list: Vec<PageIndex>,
}

impl PageManager {
    pub fn alloc(&mut self) -> PageIndex { ... }
    pub fn free(&mut self, index: PageIndex) { ... }
}
```

### Step 2: Row Serializer

Implement BFLATN encoding:

```rust
fn serialize_row(value: &ProductValue, layout: &RowTypeLayout) -> Vec<u8> {
    let mut buf = Vec::new();
    // 1. Write fixed columns
    // 2. Collect variable columns
    // 3. Write variable section
    // 4. Write var_refs
    buf
}
```

### Step 3: Index Implementation

```rust
pub trait Index {
    fn insert(&mut self, key: &[u8], row: RowPointer);
    fn delete(&mut self, key: &[u8], row: RowPointer);
    fn query(&self, range: RangeBounds<[u8]>) -> impl Iterator<Item = RowPointer>;
}
```

### Step 4: Commit Log

```rust
pub struct CommitLog {
    current_segment: SegmentWriter,
}

impl CommitLog {
    pub fn append(&mut self, commit: &CommitRecord) -> io::Result<()> {
        // 1. Serialize commit
        // 2. Compute checksum
        // 3. Write to segment
        // 4. Rotate segment if needed
    }
}
```

## File Size Estimation

For a table with N rows:

```
Size = N * (avg_row_size + index_overhead) + page_overhead

Where:
- avg_row_size = fixed_size + avg_var_size
- index_overhead = ~20 bytes per row per index
- page_overhead = ~32 bytes per 4KB page
```

Example: 1 million rows with 100 byte average size and 2 indexes:
```
Size ≈ 1M * (100 + 40) + 8KB ≈ 140 MB
```
