# TimescaleDB Architecture: Hypertables, Chunks, and Compression

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.timescale/timescaledb/`

---

## Table of Contents

1. [Hypertable Architecture](#hypertable-architecture)
2. [Chunk Management](#chunk-management)
3. [Compression Internals](#compression-internals)
4. [Continuous Aggregates](#continuous-aggregates)
5. [Subspace Store](#subspace-store)

---

## Hypertable Architecture

### What is a Hypertable?

A hypertable is a virtual table that automatically partitions data by time (and optionally space). Users query hypertables like regular tables, but TimescaleDB handles the complexity of distributed storage.

```sql
-- Create a regular table
CREATE TABLE conditions (
  time        TIMESTAMPTZ       NOT NULL,
  location    TEXT              NOT NULL,
  temperature DOUBLE PRECISION  NULL,
  humidity    DOUBLE PRECISION  NULL
);

-- Convert to hypertable
SELECT create_hypertable(
  'conditions',
  'time',                        -- Partitioning column
  chunk_time_interval => INTERVAL '1 day',
  create_default_indexes => TRUE
);
```

### Internal Structure

```
┌─────────────────────────────────────────────────────────────────┐
│                         HYPERTABLE                               │
│  relid: 16384                                                   │
│  schema_name: public                                            │
│  table_name: conditions                                         │
│  num_dimensions: 2                                              │
├─────────────────────────────────────────────────────────────────┤
│  DIMENSIONS:                                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Dimension 0 (Open/Time):                                │   │
│  │    - column_name: time                                   │   │
│  │    - column_type: TIMESTAMPTZ                            │   │
│  │    - interval: 86400000000 (1 day in microseconds)       │   │
│  │    - aligned: true                                       │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Dimension 1 (Closed/Space):                             │   │
│  │    - column_name: location                               │   │
│  │    - column_type: TEXT                                   │   │
│  │    - num_slices: 4 (hash partitions)                     │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         CHUNKS                                   │
├──────────────┬──────────────┬──────────────┬─────────────────────┤
│   Chunk 1    │   Chunk 2    │   Chunk 3    │   Chunk N           │
│  relid:16400 │ relid:16401  │ relid:16402  │  relid:164XX        │
│  range:      │ range:       │ range:       │  range:             │
│  [day1,day2) │ [day2,day3)  │ [day3,day4)  │  [...]              │
│  location=*  │ location=*=  │ location=*   │  location=*         │
└──────────────┴──────────────┴──────────────┴─────────────────────┘
```

### Hypertable Metadata Catalog

```c
// Simplified catalog structure from TimescaleDB
typedef struct Hypertable {
    int32 id;                    // Hypertable ID
    char *schema_name;           // Schema name
    char *table_name;            // Table name
    char *associated_schema_name;// Internal schema
    char *associated_table_prefix;// Chunk name prefix
    Dimension *dimensions;       // Partition dimensions
    int16 num_dimensions;        // Number of dimensions
    uint16 num_constraints;      // Number of constraints
} Hypertable;

typedef struct Dimension {
    int32 id;
    int32 hypertable_id;
    int16 column_attno;          // Column attribute number
    char *column_name;
    DimensionType type;          // OPEN (time) or CLOSED (space)
    int64 interval;              // For OPEN: time interval
    int16 num_slices;            // For CLOSED: hash partitions
} Dimension;
```

### Dimension Slices and Hypercubes

Each chunk is defined by a hypercube of dimension slices:

```
Time Dimension Slices:
┌──────────┬──────────┬──────────┬──────────┐
│ Slice 1  │ Slice 2  │ Slice 3  │ Slice 4  │
│ [0, 1d)  │ [1d, 2d) │ [2d, 3d) │ [3d, 4d) │
└──────────┴──────────┴──────────┴──────────┘

Space Dimension Slices (hash):
┌──────┬──────┬──────┬──────┐
│ S0   │ S1   │ S2   │ S3   │
│ [0)  │ [1)  │ [2)  │ [3)  │
└──────┴──────┴──────┴──────┘

Resulting Chunks (Time x Space):
┌─────────┬─────────┬─────────┬─────────┐
│ C(S0,T0)│ C(S1,T0)│ C(S2,T0)│ C(S3,T0)│ <- Day 1
├─────────┼─────────┼─────────┼─────────┤
│ C(S0,T1)│ C(S1,T1)│ C(S2,T1)│ C(S3,T1)│ <- Day 2
├─────────┼─────────┼─────────┼─────────┤
│ C(S0,T2)│ C(S1,T2)│ C(S2,T2)│ C(S3,T2)│ <- Day 3
└─────────┴─────────┴─────────┴─────────┘
```

---

## Chunk Management

### Chunk Creation

Chunks are created automatically when data is inserted:

```c
// Conceptual chunk creation flow
Chunk *chunk_create(
    Hypertable *ht,
    Point *point,          // The data point triggering creation
    List *constraints,     // Constraints from hypercube
    int num_constraints
) {
    // 1. Calculate hypercube for the point
    Hypercube *cube = hypercube_calculate(ht, point);

    // 2. Create chunk table
    Chunk *chunk = chunk_create_table(ht, cube);

    // 3. Create indexes on chunk
    chunk_create_indexes(chunk);

    // 4. Add constraints
    chunk_add_constraints(chunk, constraints);

    // 5. Insert into catalog
    chunk_insert_catalog(chunk);

    return chunk;
}
```

### Chunk Constraints

Each chunk has CHECK constraints defining its data range:

```sql
-- Example chunk constraints
ALTER TABLE _timescaledb_internal._hyper_1_2_chunk
ADD CONSTRAINT constraint_1
CHECK (
  time >= '2024-01-01 00:00:00+00'::timestamptz
  AND time < '2024-01-02 00:00:00+00'::timestamptz
);

-- Space constraint (if applicable)
ALTER TABLE _timescaledb_internal._hyper_1_2_chunk
ADD CONSTRAINT constraint_2
CHECK (_timescaledb_internal.get_partition_hash(location) >= 0
  AND _timescaledb_internal.get_partition_hash(location) < 16384);
```

### Chunk Exclusion (Partition Pruning)

During query planning, chunks are excluded based on constraints:

```c
// Conceptual partition pruning
List *chunk_exclude(
    Hypertable *ht,
    List *constraints  // FROM clause constraints
) {
    List *excluded = NIL;

    foreach(chunk, ht->chunks) {
        if (chunk_contradicts_constraints(chunk, constraints)) {
            excluded = lappend(excluded, chunk);
        }
    }

    return excluded;
}
```

**Query Example:**

```sql
-- This query will only scan relevant chunks
EXPLAIN SELECT * FROM conditions
WHERE time >= '2024-01-15' AND time < '2024-01-16'
  AND location = 'office';

-- Plan shows chunk exclusion:
-- Append
--   -> Seq Scan on _hyper_1_2_chunk  (day 15, all locations)
--      Filter: (location = 'office')
--   -> Seq Scan on _hyper_1_3_chunk  (day 15, office hash)
--      Filter: (location = 'office')
```

---

## Compression Internals

### Compression Architecture

```
┌────────────────────────────────────────────────────────────┐
│                  COMPRESSION PIPELINE                        │
├────────────────────────────────────────────────────────────┤
│                                                              │
│  UNCOMPRESSED CHUNK                                          │
│  ┌─────────┬─────────┬─────────┬─────────┐                  │
│  │ time    │ loc     │ temp    │ humidity│                  │
│  ├─────────┼─────────┼─────────┼─────────┤                  │
│  │ 1000    │ office  │ 72.5    │ 45      │                  │
│  │ 1001    │ office  │ 72.6    │ 45      │                  │
│  │ 1002    │ office  │ 72.5    │ 46      │                  │
│  │ ...     │ ...     │ ...     │ ...     │                  │
│  └─────────┴─────────┴─────────┴─────────┘                  │
│                          │                                   │
│                          ▼                                   │
│  SEGMENT BY (loc=office)                                     │
│  ┌─────────────────────────────────────────┐                │
│  │ All rows with loc='office' grouped      │                │
│  └─────────────────────────────────────────┘                │
│                          │                                   │
│                          ▼                                   │
│  ORDER BY (time DESC)                                        │
│  ┌─────────────────────────────────────────┐                │
│  │ Rows sorted by time descending          │                │
│  └─────────────────────────────────────────┘                │
│                          │                                   │
│                          ▼                                   │
│  COLUMNAR COMPRESSION                                          │
│  ┌─────────┬─────────┬─────────┬─────────┐                  │
│  │ time    │ loc     │ temp    │ humidity│                  │
│  │(Delta)  │(Dict)   │(Gorilla)│(Delta)  │                  │
│  ├─────────┼─────────┼─────────┼─────────┤                  │
│  │ [1,1,1] │ [0]     │ XORs    │ [1,0,1] │                  │
│  │ ...     │ (idx)   │ ...     │ ...     │                  │
│  └─────────┴─────────┴─────────┴─────────┘                  │
│                          │                                   │
│                          ▼                                   │
│  COMPRESSED CHUNK                                              │
│  ┌─────────────────────────────────────────┐                │
│  │ Compressed column segments              │                │
│  └─────────────────────────────────────────┘                │
└────────────────────────────────────────────────────────────┘
```

### Compression Algorithms

#### 1. DeltaDelta (Integers)

```
Original:  [1000, 1001, 1002, 1003, 1004]
First Delta: [1,    1,    1,    1,    1   ]  (1001-1000, 1002-1001, ...)
Delta-Delta: [0,    0,    0,    0,    0   ]  (1-1, 1-1, ...)

Encoding:
  - Store first value: 1000
  - Store first delta: 1
  - Encode delta-deltas with Simple8b RLE
```

#### 2. Gorilla (Floats)

```
Original:  [72.5, 72.6, 72.5, 72.7, 72.5]
As bits:   [bits1, bits2, bits3, bits4, bits5]

XOR sequence:
  - Store first value as-is: 72.5
  - XOR subsequent values:
    - bits2 XOR bits1 = xor1
    - bits3 XOR bits2 = xor2
    - ...
  - Count leading/trailing zeros in XORs
  - Store: (leading_zeros, trailing_zeros, significant_bits)

Example:
  val1 = 72.5 = 0x4052666666666666
  val2 = 72.6 = 0x4052699999999999
  xor  = 0x00000FFFFFFFFFFF
  -> 12 leading zeros, 0 trailing zeros
  -> Store: (12, 0, 0xFFFFFFFFFFFF)
```

#### 3. Dictionary (Low Cardinality)

```
Original values: [office, office, garage, office, basement, garage]

Dictionary:
  [0] = office
  [1] = garage
  [2] = basement

Encoded indices: [0, 0, 1, 0, 2, 1]
-> Compressed with Simple8b RLE
```

#### 4. Simple8b RLE

```
Simple8b packs multiple values into 64-bit words:

For small values (0-15): Pack 60 values per word (4 bits each)
For runs: Encode (value, run_length) pairs

Example run encoding:
  [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1]
  -> (0, 8), (1, 4)  // value 0 repeated 8 times, value 1 repeated 4 times
```

### Compression Configuration

```sql
-- Set compression options
ALTER TABLE conditions SET (
  timescaledb.compress,
  timescaledb.compress_segmentby = 'location',  -- Group by location
  timescaledb.compress_orderby = 'time DESC'    -- Order within group
);

-- Advanced options
ALTER TABLE conditions SET (
  timescaledb.compress_chunk_time_interval = INTERVAL '7 days',
  timescaledb.compress_segmentby = 'location, sensor_id'
);
```

### Segmentby and Orderby

**segmentby**: Columns used to group rows before compression
- Rows with same segmentby values are compressed together
- Improves compression for columns correlated with segmentby
- Enables efficient filtering on segmentby columns

**orderby**: Column order within each segment
- Affects compression ratio (correlated columns compress better)
- Affects query performance (order matches query patterns)

---

## Continuous Aggregates

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│              CONTINUOUS AGGREGATE STRUCTURE                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  USER VIEW (conditions_summary_daily)                        │
│  SELECT location, bucket, avg_temp, max_temp, min_temp       │
│  FROM partial_view + finalize                                │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ PARTIAL VIEW (_internal_partial_view)                 │   │
│  │ SELECT location, bucket,                              │   │
│  │        INTERNAL_avg(temperature) as avg_state,       │   │
│  │        INTERNAL_max(temperature) as max_state        │   │
│  │ FROM raw hypertable                                   │   │
│  │ GROUP BY location, bucket                             │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ MATERIALIZATION HYPERTABLE                            │   │
│  │ Stores partial aggregate states per bucket            │   │
│  │                                                       │   │
│  │ location  | bucket    | avg_state | max_state | ...   │   │
│  │ office    | 2024-01-01| [sum,cnt] | [max]     | ...   │   │
│  │ office    | 2024-01-02| [sum,cnt] | [max]     | ...   │   │
│  │ garage    | 2024-01-01| [sum,cnt] | [max]     | ...   │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ INVALIDATION LOG                                      │   │
│  │ Tracks which time ranges need refresh                 │   │
│  │                                                       │   │
│  │ hypertable_id | start_time | end_time | processed     │   │
│  │ 1             | 1704067200 | 1704153600 | t          │   │
│  │ 1             | 1704153600 | 1704240000 | f          │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ INVALIDATION THRESHOLD                                │   │
│  │ Timestamp after which invalidations are not logged    │   │
│  │ (assumes recent data is still being written)          │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Invalidation Processing

```
1. Data Modification on Raw Hypertable

   INSERT INTO conditions VALUES ('2024-01-15', 'office', 72.5);
                          │
                          ▼
2. Trigger Fires (or WAL decoding)

   ┌─────────────────────────────────┐
   │ Record invalidation range       │
   │ [bucket_start, bucket_end)      │
   └─────────────────────────────────┘
                          │
                          ▼
3. Invalidation Written to Log

   hypertable_invalidations:
   (ht_id=1, start=1704067200, end=1704153600)
                          │
                          ▼
4. Refresh Transaction Reads Log

   ┌─────────────────────────────────┐
   │ Cut invalidation against        │
   │ refresh window                  │
   └─────────────────────────────────┘
                          │
                          ▼
5. Partial View Re-materializes

   INSERT INTO materialization_ht
   SELECT ... FROM raw_ht
   WHERE time IN (invalidated range)
                          │
                          ▼
6. Invalidation Marked Processed

   UPDATE hypertable_invalidations
   SET processed = true
   WHERE ...
```

### Refresh Policies

```sql
-- Refresh policy with offsets
SELECT add_continuous_aggregate_policy(
  'conditions_summary_daily',
  start_offset => INTERVAL '1 month',  -- Start refresh 1 month ago
  end_offset => INTERVAL '1 day',      -- End refresh 1 day ago
  schedule_interval => INTERVAL '1 hour'
);

-- The refresh window moves forward each hour:
-- [NOW - 1 month, NOW - 1 day]
```

### Partial Aggregate States

```sql
-- Internal state for AVG
CREATE TYPE avg_state AS (
  sum DOUBLE PRECISION,
  count BIGINT
);

-- Combination function
CREATE FUNCTION avg_combine(state1 avg_state, state2 avg_state)
RETURNS avg_state AS $$
  SELECT (state1.sum + state2.sum, state1.count + state2.count)::avg_state;
$$ LANGUAGE SQL;

-- Finalize function
CREATE FUNCTION avg_finalize(state avg_state)
RETURNS DOUBLE PRECISION AS $$
  SELECT CASE WHEN state.count > 0
         THEN state.sum / state.count
         ELSE NULL END;
$$ LANGUAGE SQL;
```

---

## Subspace Store

### Purpose

The SubspaceStore is a cache for per-chunk data structures:

```rust
// Conceptual Rust representation
pub struct SubspaceStore<K, V> {
    /// Tree structure: dimensions -> values
    tree: SubspaceTree<K, V>,
    /// Maximum entries before eviction
    max_entries: usize,
}

pub struct SubspaceTree<K, V> {
    /// Each level corresponds to a dimension
    levels: Vec<DimensionLevel>,
}

pub struct DimensionLevel {
    /// Slices for this dimension
    slices: BTreeMap<SliceKey, DimensionNode>,
}

pub struct DimensionNode {
    /// Children (next dimension level or leaf value)
    children: Option<Box<SubspaceTree<K, V>>>,
    /// Leaf value (if last dimension)
    value: Option<V>,
}
```

### Tree Structure

```
Hypertable: (time TIMESTAMP, location TEXT)
Dimensions:
  - time (open, 1 hour intervals)
  - location (closed, 2 hash partitions)

SubspaceStore Tree:
                         Root
                          │
          ┌───────────────┴───────────────┐
          ▼                               ▼
    Time: 00:00-01:00              Time: 01:00-02:00
          │                               │
    ┌─────┴─────┐                   ┌─────┴─────┐
    ▼           ▼                   ▼           ▼
Loc:0       Loc:1               Loc:0       Loc:1
    │           │                   │           │
    ▼           ▼                   ▼           ▼
  Chunk1    Chunk2              Chunk3      Chunk4

Lookup: (time='00:30', location_hash=0)
  -> Navigate Time: 00:00-01:00
  -> Navigate Loc: 0
  -> Return Chunk1
```

### Eviction Strategy

```rust
impl<K, V> SubspaceStore<K, V> {
    /// When store is full, evict the oldest time entries
    fn evict_oldest(&mut self) {
        // Get first entry from top-level (time) vector
        if let Some((time_key, node)) = self.tree.levels[0].slices.first_key_value() {
            // Remove entire subtree for this time slice
            self.tree.levels[0].slices.remove(time_key);
        }
    }
}
```

**Rationale:**
- Time-ordered access pattern assumed
- Oldest chunks least likely to be reused
- Efficient bulk eviction

### Usage in TimescaleDB

```c
// C representation from TimescaleDB source
typedef struct SubspaceStore {
    MemoryContext mcxt;           // Memory context
    Dimension **dimensions;       // Dimension info
    SubspaceStoreNode *root;      // Root node
    uint32 max_entries;           // Max entries before eviction
    uint32 num_entries;           // Current entry count
} SubspaceStore;

// Used for caching:
// - ChunkInsertState for efficient inserts
// - Compression state
// - Per-chunk query state
```

---

## Implementation Details

### Catalog Tables

```sql
-- Hypertable catalog
CREATE TABLE _timescaledb_catalog.hypertable (
    id INTEGER PRIMARY KEY,
    schema_name NAME NOT NULL,
    table_name NAME NOT NULL,
    associated_schema_name NAME NOT NULL,
    associated_table_prefix NAME NOT NULL,
    num_dimensions SMALLINT NOT NULL
);

-- Dimension catalog
CREATE TABLE _timescaledb_catalog.dimension (
    id INTEGER PRIMARY KEY,
    hypertable_id INTEGER REFERENCES hypertable(id),
    column_name NAME NOT NULL,
    column_attno SMALLINT NOT NULL,
    interval_interval BIGINT,      -- For time dimensions
    num_slices SMALLINT            -- For space dimensions
);

-- Chunk catalog
CREATE TABLE _timescaledb_catalog.chunk (
    id INTEGER PRIMARY KEY,
    hypertable_id INTEGER REFERENCES hypertable(id),
    schema_name NAME NOT NULL,
    table_name NAME NOT NULL,
    compressed_chunk_id INTEGER    -- Reference to compressed chunk
);

-- Chunk constraint catalog
CREATE TABLE _timescaledb_catalog.chunk_constraint (
    chunk_id INTEGER REFERENCES chunk(id),
    dimension_id INTEGER REFERENCES dimension(id),
    constraint_name NAME NOT NULL,
    constraint_expression TEXT NOT NULL
);
```

### Background Worker for Policies

```c
// Simplified background worker structure
void policy_worker_main(Datum main_arg) {
    // Register background worker
    BackgroundWorkerBlockSignals();
    BackgroundWorkerInitializeConnection();

    while (!TerminateWorkerRequested()) {
        // Sleep until next wake-up
        WaitLatch(&MyLatch, WL_LATCH_SET | WL_TIMEOUT,
                  policy_interval, PG_WAIT_EXTENSION);
        ResetLatch(&MyLatch);

        // Execute policies
        policies_run_all();
    }
}
```

---

## Performance Considerations

### Chunk Sizing

| Chunk Interval | Pros | Cons |
|---------------|------|------|
| Too small (< 1 hour) | Fast pruning, efficient deletes | Too many tables, catalog bloat |
| Too large (> 1 week) | Few tables | Slow pruning, memory pressure |
| Optimal (1 day) | Balanced | Requires tuning per workload |

### Compression Ratio Expectations

| Data Type | Typical Ratio | Best Case |
|-----------|---------------|-----------|
| Timestamps | 20-50x | 100x+ (regular intervals) |
| Floats | 5-15x | 50x+ (stable values) |
| Text (high cardinality) | 1-2x | 10x+ (low cardinality) |
| Integers | 10-30x | 100x+ (sequences) |

### Memory Management

```sql
-- Increase memory for chunk operations
SET timescaledb.max_open_chunks_per_txn = 100;

-- Memory for compression
SET maintenance_work_mem = '2GB';

-- Memory for queries
SET work_mem = '256MB';
```

---

## Related Documentation

- [Query Optimization Guide](./query-optimization.md)
- [Analytics Functions](./analytics-functions.md)
- [Rust Implementation](./rust-revision.md)
