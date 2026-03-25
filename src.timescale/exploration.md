# Timescale Project: Comprehensive Exploration

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.timescale/`

**Date:** 2026-03-25

---

## Executive Summary

Timescale is a comprehensive time-series database platform built on PostgreSQL. It consists of four main components:

1. **TimescaleDB** - The core time-series database engine with hypertables, compression, and continuous aggregates
2. **pgvectorscale** - Vector similarity search extension for AI/ML workloads
3. **timescaledb-toolkit** - Analytics functions library for time-series analysis
4. **tiger-agents-for-work** - Agent framework for work automation using the platform

This exploration covers the architecture, internals, and provides a roadmap for replicating similar functionality in Rust.

---

## Table of Contents

1. [Time-Series Database Fundamentals](#time-series-database-fundamentals)
2. [TimescaleDB Architecture](#timescaledb-architecture)
3. [pgvectorscale Deep Dive](#pgvectorscale-deep-dive)
4. [Analytics Functions](#analytics-functions)
5. [Rust Replication Plan](#rust-replication-plan)
6. [Related Documents](#related-documents)

---

## Time-Series Database Fundamentals

### What Makes Time-Series Data Special

Time-series data has unique characteristics that require specialized handling:

1. **Temporal Ordering**: Data arrives in time order (mostly), with late/out-of-order arrivals
2. **High Write Volume**: Millions of metrics per second in IoT, finance, monitoring
3. **Time-Range Queries**: Most queries filter by time ranges
4. **Aggregation Patterns**: rollups, downsampling, gap-filling are common
5. **Retention Policies**: Old data is compressed or deleted
6. **Immutable History**: Updates are rare; data is append-only

### Key Challenges

```
                    Traditional RDBMS              Time-Series DB
                    -----------------              ---------------
Write Pattern       Random writes                  Sequential (time-ordered)
Index Strategy      B-Trees on multiple columns    Time-partitioned + space indexes
Query Pattern       Point lookups                  Range scans + aggregations
Compression         Row-level                      Columnar (time-correlation)
Retention           Manual                         Policy-based automated
```

### Hypertables and Chunks

Hypertables are virtual tables that automatically partition data by time:

```sql
-- User sees a simple table
CREATE TABLE conditions (
  time        TIMESTAMPTZ NOT NULL,
  location    TEXT        NOT NULL,
  temperature DOUBLE PRECISION
);

-- Convert to hypertable with automatic partitioning
SELECT create_hypertable('conditions', 'time', chunk_time_interval => INTERVAL '1 day');
```

**Internal Structure:**

```
┌─────────────────────────────────────────────────────────────┐
│                      HYPERTABLE                              │
│  (virtual table - user queries this)                         │
├─────────────┬─────────────┬─────────────┬────────────────────┤
│   CHUNK 1   │   CHUNK 2   │   CHUNK 3   │   CHUNK N          │
│  (table)    │  (table)    │  (table)    │   (table)          │
│  Day 1      │  Day 2      │  Day 3      │   Day N            │
└─────────────┴─────────────┴─────────────┴────────────────────┘
```

### Time-Based Partitioning

Partitioning strategy divides data into manageable chunks:

1. **Range Partitioning by Time**: Each chunk covers a time range
2. **Hash Partitioning**: Optional secondary partitioning for distributed setups
3. **Adaptive Chunk Sizing**: Chunks sized to fit in memory for operations

Benefits:
- **Faster Queries**: Partition pruning eliminates irrelevant chunks
- **Efficient Deletes**: Drop old chunks instantly for retention
- **Parallel Operations**: Each chunk can be processed independently
- **Compression Units**: Chunks are compressed independently

### Compression Strategies

TimescaleDB uses specialized compression for time-series:

| Algorithm | Use Case | Description |
|-----------|----------|-------------|
| DeltaDelta | Integers | Stores delta-of-deltas, zigzag encoded, Simple8b compressed |
| Gorilla | Floats | Facebook's algorithm - XORs of adjacent values |
| Dictionary | Low cardinality | Stores unique values + index array |
| Simple8b RLE | Booleans/Runs | Run-length encoding with bit packing |
| Array | Fallback | Uncompressed storage with TOAST |

**Compression Example:**

```sql
-- Enable compression on a hypertable
ALTER TABLE conditions SET (
  timescaledb.compress,
  timescaledb.compress_segmentby = 'location'
);

-- Create policy to compress old chunks
SELECT add_compression_policy('conditions', INTERVAL '1 day');
```

### Continuous Aggregates

Materialized views that incrementally refresh:

```sql
CREATE MATERIALIZED VIEW daily_stats
WITH (timescaledb.continuous) AS
SELECT
  location,
  time_bucket('1 day', time) AS bucket,
  avg(temperature),
  max(temperature),
  min(temperature)
FROM conditions
GROUP BY location, bucket;

-- Auto-refresh policy
SELECT add_continuous_aggregate_policy(
  'daily_stats',
  start_offset => INTERVAL '1 month',
  end_offset => INTERVAL '1 day',
  schedule_interval => INTERVAL '1 hour'
);
```

**Internal State Machine:**

```
┌──────────────────────────────────────────────────────────────┐
│                    CONTINUOUS AGGREGATE                       │
├──────────────────────────────────────────────────────────────┤
│  1. User View (finalized results)                            │
│  2. Partial View (materializes new data)                     │
│  3. Direct View (original query)                             │
│  4. Materialization Hypertable (partial aggregates)          │
│  5. Invalidation Log (tracks changed regions)                │
│  6. Invalidation Threshold (tracks materialization point)    │
└──────────────────────────────────────────────────────────────┘
```

---

## TimescaleDB Architecture

### PostgreSQL Extension Model

TimescaleDB is built as a PostgreSQL extension:

```
┌─────────────────────────────────────────────────────────┐
│                    PostgreSQL Core                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │              TimescaleDB Extension               │    │
│  │  ┌─────────────┬─────────────┬────────────────┐ │    │
│  │  │  Hypertable │  Chunk      │  Compression   │ │    │
│  │  │  Manager    │  Manager    │  Engine        │ │    │
│  │  ├─────────────┼─────────────┼────────────────┤ │    │
│  │  │  Continuous │  Gapfill    │  Scheduler     │ │    │
│  │  │  Aggregates │  Engine     │  (BGW)         │ │    │
│  │  └─────────────┴─────────────┴────────────────┘ │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

**Key Extension Points:**
- Custom Scan Nodes (for gapfill, hypertable expansion)
- Index Access Methods (for time-series optimized indexes)
- Trigger System (for invalidation tracking)
- Background Worker (for scheduled tasks)

### Query Planning for Time-Series

TimescaleDB intercepts query planning to optimize for hypertables:

1. **Constraint Exclusion**: Prunes chunks outside time range
2. **Parallel Chunk Scans**: Scans multiple chunks in parallel
3. **Custom Aggregation**: Pushes aggregation into chunk scans
4. **Skip Scan**: Efficient DISTINCT queries on segmentby columns

### Index Strategies

**Time Index (Primary):**
- Every chunk has index on time column
- Enables efficient time-range scans

**Space Index (Segmentby):**
- Optional indexes on segmentby columns
- Enables partition-wise aggregation

**Time+Space Indexes:**
- Composite indexes for common query patterns
- `(time, location)` for location-filtered time queries

```sql
-- Recommended index pattern
CREATE INDEX ON conditions (location, time DESC);
```

### Write Path Optimizations

1. **Chunk Routing**: Directs inserts to correct chunk
2. **Insert State Caching**: Caches chunk insert state
3. **Bulk Insert**: Optimizes for batch inserts
4. **Compression Friendly**: Orders data for compression

### Read Path Optimizations

1. **Partition Pruning**: Eliminates irrelevant chunks
2. **Parallel Query**: Scans chunks in parallel
3. **Columnar Compression**: Decompresses only needed columns
4. **Continuous Aggregate Routing**: Routes queries to materialized data

---

## pgvectorscale Deep Dive

### Vector Similarity Search

pgvectorscale extends pgvector with production-grade performance:

```sql
-- Create table with embeddings
CREATE TABLE documents (
    id BIGINT PRIMARY KEY,
    embedding VECTOR(1536),
    labels SMALLINT[]
);

-- Create StreamingDiskANN index
CREATE INDEX ON documents
USING diskann (embedding vector_cosine_ops, labels);

-- Query with filtering
SELECT * FROM documents
WHERE labels && ARRAY[1, 3]
ORDER BY embedding <=> '[...]'::vector
LIMIT 10;
```

### Index Structures for Vectors

**StreamingDiskANN Algorithm:**

Based on Microsoft's DiskANN research:

```
┌─────────────────────────────────────────────────────────┐
│              STREAMING DISKANN INDEX                      │
├─────────────────────────────────────────────────────────┤
│  1. Graph-based ANN (Approximate Nearest Neighbor)       │
│  2. Vamana algorithm for graph construction              │
│  3. Disk-resident with SSD-optimized access              │
│  4. SBQ (Statistical Binary Quantization) compression    │
│  5. Label-based filtering support                        │
└─────────────────────────────────────────────────────────┘
```

**Performance Comparison:**
- 28x lower p95 latency vs Pinecone (50M vectors, 768 dimensions)
- 16x higher query throughput
- 75% cost reduction on AWS EC2

### Integration with Time-Series

Vector + time-series patterns:

```sql
-- Time-filtered vector search
SELECT * FROM embeddings
WHERE time > NOW() - INTERVAL '1 day'
ORDER BY embedding <=> '[query_vector]'
LIMIT 10;

-- Aggregation with vector search
SELECT
  time_bucket('1 hour', time),
  avg(similarity)
FROM (
  SELECT time, 1 - (embedding <=> '[vector]') as similarity
  FROM embeddings
)
GROUP BY 1;
```

---

## Analytics Functions

### Time-Series Analytics (timescaledb-toolkit)

The toolkit provides specialized analytics functions:

#### Time Weighted Average

For irregularly sampled data:

```sql
-- LOCF (Last Observation Carried Forward)
SELECT
  measure_id,
  average(time_weight('LOCF', ts, val))
FROM sensor_data
GROUP BY measure_id;

-- Linear Interpolation
SELECT
  measure_id,
  average(time_weight('Linear', ts, val))
FROM sensor_data
GROUP BY measure_id;
```

#### UddSketch (Percentile Approximation)

Adaptive histogram for percentile estimation:

```sql
-- Create sketch with 100 buckets, 0.5% max error
SELECT
  uddsketch(100, 0.005, value) as sketch
FROM metrics;

-- Extract percentiles
SELECT
  approx_percentile(0.50, sketch) as p50,
  approx_percentile(0.95, sketch) as p95,
  approx_percentile(0.99, sketch) as p99
FROM (
  SELECT uddsketch(100, 0.005, value) as sketch
  FROM metrics
);
```

#### HyperLogLog (Count Distinct)

Approximate distinct count:

```sql
SELECT
  distinct_count(hyperloglog(64, user_id))
FROM events;
```

#### Stats Agg (Statistical Functions)

```sql
-- 1D statistics
SELECT
  average(stats_agg(value)),
  stddev(stats_agg(value)),
  skewness(stats_agg(value)),
  kurtosis(stats_agg(value))
FROM metrics;

-- 2D regression
SELECT
  slope(stats_agg(y, x)),
  intercept(stats_agg(y, x)),
  corr(stats_agg(y, x))
FROM paired_data;
```

#### LTTB (Downsampling)

Largest Triangle Three Buckets for visual downsampling:

```sql
SELECT time, value
FROM unnest((
  SELECT lttb(time, value, 100)  -- Downsample to 100 points
  FROM high_frequency_data
));
```

### Gap Filling and Interpolation

```sql
-- Fill gaps with LOCF
SELECT
  time_bucket_gapfill('1 hour', time, start, end) as bucket,
  locf(min(value)) as value
FROM sensor_data
GROUP BY 1
ORDER BY 1;

-- Interpolate missing values
SELECT
  time_bucket_gapfill('1 hour', time, start, end) as bucket,
  interpolate(avg(value)) as value
FROM sensor_data
GROUP BY 1
ORDER BY 1;
```

### Financial Functions

```sql
-- Candlestick/OHLC aggregation
SELECT
  time_bucket('1 day', time) as day,
  ohlc(first(time, value), last(time, value), min(value), max(value)) as candle
FROM prices
GROUP BY 1;
```

---

## Rust Replication Plan

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  RUST TIME-SERIES ENGINE                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Storage Layer                        │   │
│  │  - Columnar segment storage                          │   │
│  │  - Compression (DeltaDelta, Gorilla, etc.)           │   │
│  │  - LSM-tree for writes                               │   │
│  │  - SSTable format for segments                       │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Query Engine                         │   │
│  │  - DataFusion for SQL parsing/planning               │   │
│  │  - Custom scan executors                             │   │
│  │  - Vectorized execution                              │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                  Analytics Layer                      │   │
│  │  - UddSketch implementation                          │   │
│  │  - Time-weighted aggregates                          │   │
│  │  - HyperLogLog                                       │   │
│  │  - LTTB downsampling                                 │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Storage Engine Design

**Segment-Based Storage:**

```rust
/// A segment is the basic unit of storage
pub struct Segment {
    /// Unique segment identifier
    pub id: SegmentId,
    /// Time range covered
    pub time_range: Range<Timestamp>,
    /// Column data (compressed)
    pub columns: Vec<ColumnSegment>,
    /// Segment metadata
    pub metadata: SegmentMetadata,
}

/// Column segment with compression
pub struct ColumnSegment {
    pub column_id: ColumnId,
    pub encoding: EncodingType,
    pub data: Bytes,
    pub null_bitmap: Option<Bitmap>,
}

/// Encoding types for time-series
pub enum EncodingType {
    DeltaDelta,      // For integers
    Gorilla,         // For floats
    Dictionary,      // Low cardinality
    Simple8b,        // Booleans/runs
    Raw,             // Uncompressed
}
```

**LSM Tree for Writes:**

```rust
pub struct LSMTree {
    /// In-memory buffer (MemTable)
    memtable: Arc<RwLock<MemTable>>,
    /// Immutable memtables waiting for flush
    immutables: Vec<Arc<MemTable>>,
    /// SSTable levels L0-Ln
    levels: Vec<Level>,
    /// WAL for durability
    wal: WriteAheadLog,
}
```

### Query Engine Design

**Using DataFusion:**

```rust
use datafusion::prelude::*;
use datafusion::execution::context::SessionContext;

pub struct TimeSeriesEngine {
    ctx: SessionContext,
    storage: Arc<SegmentStore>,
}

impl TimeSeriesEngine {
    pub fn new() -> Self {
        let ctx = SessionContext::new();
        // Register custom table providers
        ctx.register_table("metrics", Arc::new(HypertableProvider::new()));
        Self { ctx, storage: Arc::new(SegmentStore::new()) }
    }

    pub async fn query(&self, sql: &str) -> Result<RecordBatch> {
        self.ctx.sql(sql).await?.collect().await
    }
}
```

**Custom Scan Node:**

```rust
/// Custom scan for hypertable chunk pruning
pub struct HypertableScanExec {
    /// Time range filter
    pub time_filter: Option<Range<Timestamp>>,
    /// Available chunks
    pub chunks: Vec<ChunkMetadata>,
    /// Pruned chunks
    pub pruned_chunks: Vec<ChunkId>,
    /// Inner scan execution
    pub inner: Arc<dyn ExecutionPlan>,
}

impl ExecutionPlan for HypertableScanExec {
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        // Execute only pruned chunks
        self.inner.execute(partition, context)
    }
}
```

### Crate Recommendations

| Purpose | Crate | Description |
|---------|-------|-------------|
| SQL Engine | `datafusion` | Query parsing, planning, execution |
| Storage | `bytes` | Efficient byte buffers |
| Compression | `delta` | Delta-of-delta encoding |
| Compression | `gorilla` | Gorilla float compression |
| Hashing | `twox-hash` | XXHash for hashing |
| Bitmaps | `roaring` | Roaring bitmaps |
| Time | `chrono` / `time` | Timestamp handling |
| Async Runtime | `tokio` | Async runtime |
| Columnar | `arrow` | Arrow format for columns |
| LSM | `slatedb` / `foyer` | LSM implementations |
| Vectors | `pgvector-rs` | Vector operations |

### Production Considerations

1. **Durability**: WAL + periodic checkpoints
2. **Replication**: Raft consensus for HA
3. **Backups**: Point-in-time recovery
4. **Monitoring**: Prometheus metrics export
5. **Security**: TLS, authentication, authorization
6. **Resource Management**: Memory limits, backpressure

---

## Related Documents

This exploration is complemented by the following detailed documents:

| Document | Description |
|----------|-------------|
| [`timescaledb-architecture.md`](./timescaledb-architecture.md) | Hypertables, chunks, compression internals |
| [`query-optimization.md`](./query-optimization.md) | Query planning, indexes, execution strategies |
| [`pgvectorscale-deep-dive.md`](./pgvectorscale-deep-dive.md) | Vector search architecture and algorithms |
| [`analytics-functions.md`](./analytics-functions.md) | Time-series analytics implementation details |
| [`rust-revision.md`](./rust-revision.md) | Complete Rust replication plan |
| [`production-grade.md`](./production-grade.md) | Production deployment considerations |
| [`storage-system-guide.md`](./storage-system-guide.md) | Step-by-step implementation for engineers |

---

## Conclusion

Timescale represents a mature, production-ready time-series database platform built on PostgreSQL's extensibility. Its key innovations include:

1. **Hypertable Abstraction**: Transparent time-partitioning
2. **Specialized Compression**: Columnar compression for time-series
3. **Continuous Aggregates**: Incremental materialized views
4. **Analytics Toolkit**: Production-ready analytics functions
5. **Vector Search**: pgvectorscale for AI/ML workloads

The architecture demonstrates how to build a specialized database while leveraging PostgreSQL's ecosystem, providing a blueprint for Rust-based implementations.
