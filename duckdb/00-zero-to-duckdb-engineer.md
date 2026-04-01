---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb
repository: git@github.com:duckdb/duckdb.git
explored_at: 2026-03-29
language: C++11
category: Analytical Database System
---

# Zero to DuckDB Engineer - Complete Fundamentals

## Table of Contents

1. [What is DuckDB?](#what-is-duckdb)
2. [Core Architecture Overview](#core-architecture-overview)
3. [Storage Fundamentals](#storage-fundamentals)
4. [Query Execution Model](#query-execution-model)
5. [Vectorized Processing](#vectorized-processing)
6. [Object Storage Integration](#object-storage-integration)
7. [Parquet Integration](#parquet-integration)
8. [Compression Algorithms](#compression-algorithms)
9. [Buffer Management](#buffer-management)
10. [Transaction Model](#transaction-model)

## What is DuckDB?

DuckDB is a high-performance **analytical database management system** (OLAP - Online Analytical Processing). Unlike traditional row-oriented databases (MySQL, PostgreSQL) optimized for transactions, DuckDB is column-oriented and optimized for analytical queries that:

- Scan large portions of tables
- Perform aggregations across millions of rows
- Execute complex joins on large datasets
- Process data from external sources (Parquet, CSV, S3)

### Key Design Principles

1. **Embedded**: No server required - runs in-process like SQLite
2. **Column-oriented**: Data stored by column, not row - optimal for analytics
3. **Vectorized execution**: Process batches of data (vectors) efficiently
4. **Parallel execution**: Multi-core query processing
5. **Zero dependencies**: Single binary with no external requirements

## Core Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     SQL Query                                │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Parser (PostgreSQL-style SQL grammar via libpg_query)      │
│  - Converts SQL to ParseTree                                │
│  - Handles dialect extensions                               │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Planner                                                    │
│  - Binds identifiers to catalog entries                     │
│  - Type checking and coercion                               │
│  - Creates bound expressions                                │
│  - Generates physical operators                             │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Optimizer                                                  │
│  - Filter pushdown                                          │
│  - Projection pushdown                                      │
│  - Join reordering                                          │
│  - Statistics-based optimization                            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Execution Engine (Vectorized)                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Vector (Batch of ~2048 rows)                       │   │
│  │  - Column data stored contiguously                  │   │
│  │  - Validity masks for NULL handling                 │   │
│  │  - SIMD-optimized operations                        │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Storage Engine                                             │
│  - Row groups (~120K rows each)                            │
│  - Column data compressed per row group                    │
│  - Buffer managed by buffer pool                           │
│  - Checkpoint manager for persistence                      │
└─────────────────────────────────────────────────────────────┘
```

## Storage Fundamentals

### DataTable Structure

DuckDB stores data in **DataTable** objects which contain:

1. **Row Groups**: Horizontal partitions of ~120,000 rows
2. **Column Definitions**: Schema metadata with type information
3. **Index Structures**: Optional indexes (ART, BST, etc.)

```cpp
// Simplified DataTable structure
class DataTable {
    shared_ptr<DataTableInfo> info;      // Schema, indexes, IO manager
    vector<ColumnDefinition> columns;    // Column definitions
    shared_ptr<RowGroupCollection> row_groups;  // Data storage
    mutex append_lock;                   // Protects concurrent appends
};
```

### Row Group Layout

Each row group stores data column-by-column:

```
Row Group (120,000 rows)
├── Column 0 (Row 0-119,999)
│   ├── Validity Mask (15KB for 120K bools)
│   └── Data (compressed)
├── Column 1 (Row 0-119,999)
│   ├── Validity Mask
│   └── Data (compressed)
└── ...
```

### Column Data Storage

Each column in a row group uses a **StandardColumnData** structure:

1. **Validity Mask**: Bit-packed NULL indicators
2. **Primary Data**: The actual values (compressed)
3. **Update Info**: For in-place updates (MVCC)

## Query Execution Model

### Pull-Based Execution

DuckDB uses a **pull-based** (iterator-style) execution model:

```cpp
// Pseudocode for query execution
while (operator.TryGetNextVector(output_vector)) {
    // Process the vector
    process(output_vector);
}
```

### Operator Tree

A query like `SELECT sum(price) FROM orders WHERE amount > 100` creates:

```
HashAggregate (sum)
    └── Filter (amount > 100)
        └── TableScan (orders)
```

Each operator:
1. Requests vectors from child operators
2. Processes the vectors
3. Returns vectors to parent operators

## Vectorized Processing

### What is a Vector?

A **Vector** in DuckDB is a batch of values (default: 2048 rows) stored contiguously in memory:

```cpp
struct Vector {
    VectorType type;           // FLAT, DICTIONARY, CONSTANT, SEQUENCE
    LogicalType dtype;         // Data type
    data_ptr_t data;           // Raw data pointer
    validity_mask_t validity;  // NULL bitmap
    vector_buffer_t buffers;   // Auxiliary buffers
};
```

### Vector Types

1. **FLAT**: Standard contiguous array
2. **DICTIONARY**: Compressed representation (value IDs + dictionary)
3. **CONSTANT**: Single value repeated (for literals)
4. **SEQUENCE**: Incrementing sequence (for row IDs)

### SIMD Optimization

Vectorized execution enables SIMD (Single Instruction, Multiple Data):

```
Traditional (row-by-row):
  for i in 0..2047: result[i] = a[i] + b[i]

Vectorized (SIMD):
  // Process 8 values at once with AVX2
  for i in 0..256: result[i*8..i*8+7] = _mm256_add_epi32(a[i*8..], b[i*8..])
```

## Object Storage Integration

### External File Cache

DuckDB caches remote file reads to minimize network I/O:

```cpp
class ExternalFileCache {
    map<string, CachedFile> cached_files;
    BufferManager buffer_manager;

    struct CachedFile {
        string path;
        map<idx_t, shared_ptr<CachedFileRange>> ranges;  // location -> cached range
        time_t last_modified;
        string version_tag;  // ETag for HTTP/S3
    };
};
```

### Cached File Range

```cpp
struct CachedFileRange {
    shared_ptr<BlockHandle> block_handle;  // Pin to buffer
    idx_t nr_bytes;                         // Size of range
    idx_t location;                         // Offset in file
    string version_tag;                     // For validation

    // Checksum for debug validation
    void AddCheckSum();
    void VerifyCheckSum();
};
```

### Read Flow for Remote Files

1. **Check Cache**: Look for overlapping cached ranges
2. **Copy from Cache**: If found, copy from pinned buffer
3. **Read Interleaved**: Combine cached and fresh reads
4. **Insert New Range**: Add newly read data to cache

### S3 Integration

DuckDB supports S3-compatible storage via HTTP FS:

```sql
-- Read from S3
SELECT * FROM 's3://bucket/path/*.parquet';

-- Configure S3 credentials
CREATE SECRET (
    TYPE S3,
    KEY_ID 'your_key',
    SECRET 'your_secret',
    REGION 'us-east-1'
);
```

## Parquet Integration

### Why Parquet?

Parquet is the ideal format for analytical workloads:

1. **Columnar storage**: Matches DuckDB's internal format
2. **Efficient compression**: Per-column encoding (RLE, dictionary, bit-packing)
3. **Predicate pushdown**: Row group statistics enable filtering
4. **Schema evolution**: Supports adding/dropping columns

### Parquet Reader Architecture

```
ParquetReader
├── ParquetFileMetadata
│   ├── RowGroups[]
│   │   └── ColumnChunks[]
│   │       └── Statistics (min, max, null_count)
│   └── Schema
├── ColumnReaders[]
│   └── Decoders (PLAIN, RLE, Dictionary, Delta)
└── RowGroupPruner (uses statistics)
```

### Column Decoding

Parquet supports multiple encodings:

| Encoding | Use Case |
|----------|----------|
| PLAIN | Raw values |
| RLE | Repeated values (e.g., country codes) |
| Dictionary | Low cardinality (e.g., status) |
| Delta | Monotonically increasing (e.g., timestamps) |
| Bit-Packed | Compact integer storage |

## Compression Algorithms

DuckDB supports multiple compression schemes per column:

### FSST (Fast Static Symbol Table)

Optimized for string compression:

1. Build symbol table from most common substrings
2. Replace substrings with single-byte symbols
3. Achieves 2-10x compression on text

### Bit-Packing

For integers with small effective range:

```
Values: [1, 3, 2, 1, 0, 2, 1, 3]
Max: 3 → needs 2 bits
Packed: 0b11_01_10_01_00_10_01_11 = 0xD9
```

### Dictionary Compression

For low-cardinality columns:

```
Original: ["red", "blue", "red", "green", "blue"]
Dictionary: [0:"red", 1:"blue", 2:"green"]
Encoded: [0, 1, 0, 2, 1]  // 3 bits per value vs 24+ bits
```

### ZSTD

General-purpose compression (Facebook):

- Fast decompression (~500 MB/s)
- Good compression ratio (~3x on average)
- Used for large columns without better options

## Buffer Management

### Buffer Pool

DuckDB uses a global buffer pool shared across all operations:

```cpp
class BufferManager {
    vector<unique_ptr<BlockHandle>> blocks;
    size_t max_memory;
    atomic<size_t> used_memory;

    BufferHandle Allocate(MemoryTag tag, size_t bytes);
    void Unpin(BlockHandle* block);
};
```

### Memory Tags

Memory is tracked by category:

| Tag | Purpose |
|-----|---------|
| MAIN_TABLE | Base table storage |
| EXTERNAL_FILE_CACHE | Remote file caching |
| VERTEX | Query execution |
| LIST | List data (arrays) |

### Eviction Policy

DuckDB uses a **clock-sweep** algorithm:

1. Blocks are in a circular list
2. Each block has a "used" flag
3. Sweep clears used flags; second sweep evicts unused

## Transaction Model

### MVCC (Multi-Version Concurrency Control)

DuckDB supports snapshot isolation:

```
Transaction T1 starts at time t0
- Sees all data committed before t0
- Does not see data committed after t0
- Can commit if no conflicts
```

### Row Versioning

Updates create new versions:

```
Original Row: (id=1, value=100, row_id=1)
T1: UPDATE SET value=200 WHERE id=1
  → Creates: (id=1, value=200, row_id=2)
  → Old version marked as deleted for T1's transaction
```

### Write-Ahead Log (WAL)

Durability via WAL:

1. All changes written to WAL first
2. WAL entry flushed before commit returns
3. Checkpoint manager creates periodic snapshots
4. Recovery replays WAL from last checkpoint

---

This document provides the foundation. See the deep-dive documents for detailed exploration of each subsystem.
