# Rust Replication Plan: Building a Timescale-like System in Rust

**Target:** Build a production-ready time-series database platform inspired by TimescaleDB

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Storage Engine Design](#storage-engine-design)
3. [Query Engine Design](#query-engine-design)
4. [Analytics Implementation](#analytics-implementation)
5. [Recommended Crates](#recommended-crates)
6. [Project Structure](#project-structure)
7. [Implementation Roadmap](#implementation-roadmap)
8. [Production Considerations](#production-considerations)

---

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     RUST TIME-SERIES PLATFORM                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                      API LAYER                             │  │
│  │  - PostgreSQL Wire Protocol (pgwire)                      │  │
│  │  - HTTP/REST API (axum)                                   │  │
│  │  - gRPC Service (tonic)                                   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                     QUERY LAYER                            │  │
│  │  - SQL Parser (sqlparser)                                 │  │
│  │  - Query Planner                                          │  │
│  │  - DataFusion Integration                                 │  │
│  │  - Custom Scan Executors                                  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   ANALYTICS LAYER                          │  │
│  │  - Time-weighted aggregates                               │  │
│  │  - Percentile approximations (UddSketch)                  │  │
│  │  - Count distinct (HyperLogLog)                           │  │
│  │  - Statistical functions                                  │  │
│  │  - Downsampling (LTTB)                                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    STORAGE LAYER                           │  │
│  │  - LSM Tree Engine                                        │  │
│  │  - Columnar Segments                                      │  │
│  │  - Compression (DeltaDelta, Gorilla, SBQ)                 │  │
│  │  - Vector Search (DiskANN)                                │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                   INFRASTRUCTURE                           │  │
│  │  - WAL (Write-Ahead Log)                                  │  │
│  │  - Checkpoint Manager                                     │  │
│  │  - Replication (Raft)                                     │  │
│  │  - Metrics (Prometheus)                                   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
Write Path:
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Client  │───>│  Parser  │───>│  Planner │───>│ Executor │
└──────────┘    └──────────┘    └──────────┘    └────┬─────┘
                                                      │
                                                      v
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Storage │<───│compressor│<───│   MemTable│<───│   WAL    │
└──────────┘    └──────────┘    └──────────┘    └──────────┘

Read Path:
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│  Client  │<───│  Results │<───│  Execute │<───│  Planner │
└──────────┘    └──────────┘    └────┬─────┘    └──────────┘
                                     │
                    ┌────────────────┼────────────────┐
                    v                v                v
              ┌──────────┐    ┌──────────┐    ┌──────────┐
              │ MemTable │    │   SST    │    │  Cache   │
              └──────────┘    └──────────┘    └──────────┘
```

---

## Storage Engine Design

### LSM Tree Architecture

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use bytes::Bytes;

/// Main LSM tree structure
pub struct LsmTree {
    /// Current mutable memtable
    memtable: Arc<RwLock<MemTable>>,
    /// Immutable memtables waiting for flush
    immutables: Arc<RwLock<Vec<Arc<MemTable>>>>,
    /// SSTable levels L0-Ln
    levels: Vec<Level>,
    /// Write-ahead log
    wal: Arc<Wal>,
    /// Compaction manager
    compactor: Compactor,
    /// Configuration
    config: LsmConfig,
}

pub struct LsmConfig {
    /// Memtable size threshold (default: 64MB)
    pub memtable_size: usize,
    /// Number of levels (default: 7)
    pub num_levels: usize,
    /// Size ratio between levels (default: 10)
    pub level_ratio: usize,
    /// Compression algorithm
    pub compression: CompressionType,
}

impl LsmTree {
    pub async fn put(&self, key: Key, value: Value) -> Result<()> {
        // 1. Write to WAL
        self.wal.write(&key, &value).await?;

        // 2. Write to memtable
        let mut memtable = self.memtable.write().await;
        memtable.insert(key, value);

        // 3. Check if memtable is full
        if memtable.size() >= self.config.memtable_size {
            self.rotate_memtable().await?;
        }

        Ok(())
    }

    pub async fn get(&self, key: &Key) -> Result<Option<Value>> {
        // 1. Check memtable
        let memtable = self.memtable.read().await;
        if let Some(value) = memtable.get(key) {
            return Ok(value.clone());
        }

        // 2. Check immutable memtables
        for imm in self.immutables.read().await.iter() {
            if let Some(value) = imm.get(key) {
                return Ok(value.clone());
            }
        }

        // 3. Check SSTable levels (L0 to Ln)
        for level in &self.levels {
            if let Some(value) = level.get(key).await? {
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    async fn rotate_memtable(&self) -> Result<()> {
        // Create new memtable
        let new_memtable = Arc::new(RwLock::new(MemTable::new()));

        // Swap with current
        let old_memtable = std::mem::replace(
            &mut *self.memtable.write().await,
            new_memtable,
        );

        // Add to immutables
        self.immutables.write().await.push(old_memtable);

        // Trigger background flush
        self.compactor.schedule_flush();

        Ok(())
    }
}
```

### MemTable Implementation

```rust
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// In-memory buffer for writes
pub struct MemTable {
    /// Data stored as (key, value) pairs
    data: BTreeMap<Key, Value>,
    /// Current size in bytes
    size: AtomicUsize,
    /// Sequence number for ordering
    sequence: AtomicUsize,
}

impl MemTable {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
            size: AtomicUsize::new(0),
            sequence: AtomicUsize::new(0),
        }
    }

    pub fn insert(&mut self, key: Key, value: Value) {
        let key_size = key.encoded_len();
        let value_size = value.encoded_len();

        if let Some(old_value) = self.data.insert(key, value) {
            let old_size = old_value.encoded_len();
            self.size.fetch_sub(old_size, Ordering::Relaxed);
        }

        self.size.fetch_add(key_size + value_size, Ordering::Relaxed);
        self.sequence.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.data.get(key)
    }

    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Convert to SSTable for flushing
    pub fn to_sstable(&self) -> SSTable {
        let entries: Vec<_> = self.data.iter().collect();
        SSTable::build(entries, CompressionType::Lz4)
    }

    /// Range scan
    pub fn range(&self, start: &Key, end: &Key) -> Vec<(&Key, &Value)> {
        self.data.range((Bound::Included(start), Bound::Excluded(end))).collect()
    }
}
```

### SSTable Format

```rust
/// SSTable structure
pub struct SSTable {
    /// Data blocks
    blocks: Vec<DataBlock>,
    /// Index block (sparse index)
    index_block: IndexBlock,
    /// Metadata block
    metadata: MetadataBlock,
    /// Bloom filter
    bloom_filter: Option<BloomFilter>,
}

/// Data block (compressed key-value pairs)
pub struct DataBlock {
    /// Compressed data
    data: Bytes,
    /// First key in block (for index)
    first_key: Key,
    /// Last key in block
    last_key: Key,
    /// Block offsets
    offsets: Vec<u32>,
}

/// Index block for binary search
pub struct IndexBlock {
    entries: Vec<IndexEntry>,
}

pub struct IndexEntry {
    key: Key,
    block_id: u32,
    offset: u32,
}

/// Metadata block
pub struct MetadataBlock {
    /// Number of entries
    num_entries: u64,
    /// Creation time
    created_at: u64,
    /// Compression type
    compression: CompressionType,
    /// Key statistics
    min_key: Key,
    max_key: Key,
}

impl SSTable {
    pub fn build(entries: Vec<(&Key, &Value)>, compression: CompressionType) -> Self {
        let mut blocks = Vec::new();
        let mut index_entries = Vec::new();
        let mut current_block = Vec::new();
        let mut current_size = 0;
        const BLOCK_SIZE: usize = 64 * 1024; // 64KB

        for (key, value) in entries {
            let entry_size = key.encoded_len() + value.encoded_len();

            if current_size + entry_size > BLOCK_SIZE && !current_block.is_empty() {
                // Flush current block
                let first_key = current_block.first().unwrap().0.clone();
                let last_key = current_block.last().unwrap().0.clone();
                let block = DataBlock::encode(current_block, compression);
                index_entries.push(IndexEntry {
                    key: first_key,
                    block_id: blocks.len() as u32,
                    offset: 0,
                });
                blocks.push(block);
                current_block = Vec::new();
                current_size = 0;
            }

            current_block.push((key, value));
            current_size += entry_size;
        }

        // Build index and metadata
        let index_block = IndexBlock { entries: index_entries };
        let metadata = MetadataBlock {
            num_entries: entries.len() as u64,
            created_at: timestamp(),
            compression,
            min_key: entries.first().unwrap().0.clone(),
            max_key: entries.last().unwrap().0.clone(),
        };

        // Build bloom filter
        let bloom_filter = BloomFilter::build(entries.iter().map(|(k, _)| k), 0.01);

        Self {
            blocks,
            index_block,
            metadata,
            bloom_filter: Some(bloom_filter),
        }
    }

    pub fn get(&self, key: &Key) -> Option<Value> {
        // Check bloom filter first
        if let Some(bloom) = &self.bloom_filter {
            if !bloom.might_contain(key) {
                return None;
            }
        }

        // Binary search in index
        let block_idx = self.index_block.find_block(key)?;
        let block = &self.blocks[block_idx];

        // Search within block
        block.get(key)
    }
}
```

### Time-Series Segment Storage

```rust
/// Segment is the basic unit of time-series storage
pub struct Segment {
    /// Unique segment ID
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
#[derive(Clone, Copy, PartialEq)]
pub enum EncodingType {
    DeltaDelta,      // For integers/timestamps
    Gorilla,         // For floats
    Dictionary,      // Low cardinality
    Simple8b,        // Booleans/runs
    Raw,             // Uncompressed
}

/// Segment metadata
pub struct SegmentMetadata {
    pub row_count: u64,
    pub created_at: Timestamp,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub column_stats: Vec<ColumnStats>,
}

impl Segment {
    /// Create a new segment from columnar data
    pub fn create(
        id: SegmentId,
        time_range: Range<Timestamp>,
        columns: Vec<ColumnData>,
    ) -> Self {
        let mut column_segments = Vec::new();
        let mut total_compressed = 0u64;
        let mut total_uncompressed = 0u64;

        for column in columns {
            let (encoding, compressed_data) = Self::compress_column(&column);
            let uncompressed_size = column.size();
            let compressed_size = compressed_data.len() as u64;

            total_compressed += compressed_size;
            total_uncompressed += uncompressed_size;

            column_segments.push(ColumnSegment {
                column_id: column.id,
                encoding,
                data: compressed_data,
                null_bitmap: column.null_bitmap,
            });
        }

        let metadata = SegmentMetadata {
            row_count: columns.first().map(|c| c.len() as u64).unwrap_or(0),
            created_at: Timestamp::now(),
            compressed_size: total_compressed,
            uncompressed_size: total_uncompressed,
            column_stats: columns.iter().map(ColumnStats::from).collect(),
        };

        Self {
            id,
            time_range,
            columns: column_segments,
            metadata,
        }
    }

    fn compress_column(column: &ColumnData) -> (EncodingType, Bytes) {
        match &column.data {
            ColumnDataType::Timestamps(ts) => {
                (EncodingType::DeltaDelta, compress_delta_delta(ts))
            }
            ColumnDataType::Floats(fs) => {
                (EncodingType::Gorilla, compress_gorilla(fs))
            }
            ColumnDataType::Ints(is) => {
                (EncodingType::DeltaDelta, compress_delta_delta(is))
            }
            ColumnDataType::Texts(texts) => {
                if texts.cardinality() < texts.len() / 10 {
                    (EncodingType::Dictionary, compress_dictionary(texts))
                } else {
                    (EncodingType::Raw, compress_raw(texts))
                }
            }
            _ => (EncodingType::Raw, compress_raw(column)),
        }
    }
}
```

---

## Query Engine Design

### DataFusion Integration

```rust
use datafusion::prelude::*;
use datafusion::execution::context::SessionContext;
use datafusion::datasource::TableProvider;

/// Main query engine
pub struct QueryEngine {
    ctx: SessionContext,
    storage: Arc<SegmentStore>,
}

impl QueryEngine {
    pub fn new(storage: Arc<SegmentStore>) -> Self {
        let ctx = SessionContext::new();

        // Register custom table providers
        ctx.register_table(
            "metrics",
            Arc::new(HypertableProvider::new(storage.clone())),
        ).unwrap();

        // Register custom functions
        register_time_weight_functions(&ctx);
        register_udsketch_functions(&ctx);
        register_hyperloglog_functions(&ctx);

        Self { ctx, storage }
    }

    pub async fn query(&self, sql: &str) -> Result<Vec<RecordBatch>> {
        self.ctx.sql(sql).await?.collect().await
    }

    pub async fn query_logical(&self, plan: LogicalPlan) -> Result<Vec<RecordBatch>> {
        self.ctx.execute_logical_plan(plan).await?.collect().await
    }
}

/// Hypertable table provider
pub struct HypertableProvider {
    storage: Arc<SegmentStore>,
}

#[async_trait]
impl TableProvider for HypertableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("time", DataType::Timestamp(TimeUnit::Nanosecond, None), false),
            Field::new("sensor_id", DataType::Utf8, false),
            Field::new("value", DataType::Float64, true),
        ]))
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        state: &SessionState,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Extract time range from filters
        let time_range = extract_time_range(filters);

        // Get relevant segments
        let segments = self.storage.get_segments_for_range(&time_range);

        // Create custom scan executor
        Ok(Arc::new(HypertableScanExec::new(
            segments,
            projection.cloned(),
            filters.to_vec(),
            limit,
        )))
    }
}
```

### Custom Scan Executor

```rust
use datafusion::physical_plan::ExecutionPlan;
use datafusion::physical_plan::RecordBatchStream;

/// Custom scan for hypertable segments
pub struct HypertableScanExec {
    segments: Vec<SegmentId>,
    projection: Option<Vec<usize>>,
    filters: Vec<Expr>,
    limit: Option<usize>,
    schema: SchemaRef,
}

impl ExecutionPlan for HypertableScanExec {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn output_partitioning(&self) -> Partitioning {
        Partitioning::UnknownPartitioning(self.segments.len())
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let segment_id = self.segments[partition];
        Ok(Box::new(SegmentStream::new(
            segment_id,
            self.projection.clone(),
            self.filters.clone(),
            self.limit,
        )))
    }
}

/// Stream that reads segment data
pub struct SegmentStream {
    segment: Segment,
    decompressor: Decompressor,
    current_batch: Option<RecordBatch>,
}

impl RecordBatchStream for SegmentStream {
    fn schema(&self) -> SchemaRef {
        self.segment.schema()
    }
}

impl Stream for SegmentStream {
    type Item = Result<RecordBatch>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(batch) = self.current_batch.take() {
            return Poll::Ready(Some(Ok(batch)));
        }

        // Read and decompress next batch
        match self.decompressor.decompress_next() {
            Ok(Some(batch)) => Poll::Ready(Some(Ok(batch))),
            Ok(None) => Poll::Ready(None),
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}
```

---

## Analytics Implementation

### Time-Weighted Average

```rust
use datafusion::arrow::array::{Float64Array, TimestampNanosecondArray};
use datafusion::physical_plan::Accumulator;

/// Time-weighted average accumulator
pub struct TimeWeightAccumulator {
    method: TimeWeightMethod,
    first_time: Option<i64>,
    last_time: Option<i64>,
    first_value: Option<f64>,
    last_value: Option<f64>,
    weighted_sum: f64,
    count: usize,
}

impl Accumulator for TimeWeightAccumulator {
    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        Ok(vec![
            ScalarValue::from(self.weighted_sum),
            ScalarValue::from(self.first_time),
            ScalarValue::from(self.last_time),
            ScalarValue::from(self.first_value),
            ScalarValue::from(self.last_value),
        ])
    }

    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let times = values[0].as_any().downcast_ref::<TimestampNanosecondArray>().unwrap();
        let vals = values[1].as_any().downcast_ref::<Float64Array>().unwrap();

        for i in 0..times.len() {
            if vals.is_null(i) {
                continue;
            }

            let time = times.value(i);
            let value = vals.value(i);

            if self.first_time.is_none() {
                self.first_time = Some(time);
                self.first_value = Some(value);
            }

            if let (Some(prev_time), Some(prev_value)) = (self.last_time, self.last_value) {
                let duration = (time - prev_time) as f64;
                let contribution = match self.method {
                    TimeWeightMethod::Linear => {
                        (prev_value + value) * duration / 2.0
                    }
                    TimeWeightMethod::Locf => {
                        prev_value * duration
                    }
                };
                self.weighted_sum += contribution;
            }

            self.last_time = Some(time);
            self.last_value = Some(value);
            self.count += 1;
        }

        Ok(())
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        // Combine with other accumulators
        // (Implementation for parallel aggregation)
        Ok(())
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        let duration = match (self.first_time, self.last_time) {
            (Some(first), Some(last)) => (last - first) as f64,
            _ => return Ok(ScalarValue::Float64(None)),
        };

        if duration <= 0.0 {
            return Ok(ScalarValue::Float64(None));
        }

        Ok(ScalarValue::from(self.weighted_sum / duration))
    }

    fn size(&self) -> usize {
        std::mem::size_of_val(self)
    }
}
```

### UddSketch

```rust
/// UddSketch percentile approximation
pub struct UddSketch {
    gamma: f64,
    max_buckets: usize,
    buckets: BTreeMap<i32, u64>,
    count: u64,
    sum: f64,
    current_error: f64,
}

impl UddSketch {
    pub fn new(max_buckets: usize, max_error: f64) -> Self {
        Self {
            gamma: 1.0 + max_error,
            max_buckets,
            buckets: BTreeMap::new(),
            count: 0,
            sum: 0.0,
            current_error: max_error,
        }
    }

    fn get_bucket_index(&self, value: f64) -> i32 {
        if value <= 0.0 {
            return i32::MIN;
        }
        (value.ln() / self.gamma.ln()).floor() as i32
    }

    pub fn add(&mut self, value: f64) {
        let idx = self.get_bucket_index(value);
        *self.buckets.entry(idx).or_insert(0) += 1;
        self.count += 1;
        self.sum += value;

        if self.buckets.len() > self.max_buckets {
            self.conservative_collapse();
        }
    }

    fn conservative_collapse(&mut self) {
        let mut new_buckets = BTreeMap::new();
        let mut prev_idx: Option<i32> = None;

        for (&idx, &count) in &self.buckets {
            if let Some(prev) = prev_idx {
                if idx == prev + 1 {
                    let combined = new_buckets.remove(&prev).unwrap_or(0) + count;
                    new_buckets.insert(prev, combined);
                    prev_idx = None;
                    continue;
                }
            }
            prev_idx = Some(idx);
            *new_buckets.entry(idx).or_insert(0) += count;
        }

        self.buckets = new_buckets;
        self.current_error *= 2.0;
    }

    pub fn approx_percentile(&self, percentile: f64) -> f64 {
        if self.count == 0 {
            return f64::NAN;
        }

        let target = percentile * self.count as f64;
        let mut cumulative = 0u64;

        for (&idx, &count) in &self.buckets {
            cumulative += count;
            if cumulative >= target as u64 {
                let start = self.gamma.powi(idx);
                let end = self.gamma.powi(idx + 1);
                return (start + end) / 2.0;
            }
        }

        let (&last_idx, _) = self.buckets.last().unwrap();
        self.gamma.powi(last_idx)
    }
}
```

---

## Recommended Crates

| Category | Crate | Purpose |
|----------|-------|---------|
| **Core** | `tokio` | Async runtime |
| **Data** | `arrow` | Columnar format |
| **Query** | `datafusion` | SQL engine |
| **Storage** | `bytes` | Byte buffers |
| **Compression** | `lz4_flex` | LZ4 compression |
| **Compression** | `zstd` | Zstandard compression |
| **Compression** | `delta` | Delta-of-delta encoding |
| **Index** | `roaring` | Roaring bitmaps |
| **Index** | `bloom` | Bloom filters |
| **Hash** | `xxhash-rust` | XXHash |
| **Hash** | `ahash` | AHash |
| **Time** | `chrono` | Date/time |
| **Time** | `iana-time-zone` | Timezone handling |
| **Serialization** | `serde` | Serialization |
| **Serialization** | `bincode` | Binary serialization |
| **Networking** | `tokio-postgres` | PostgreSQL client |
| **Networking** | `pgwire` | PostgreSQL wire protocol |
| **HTTP** | `axum` | HTTP server |
| **RPC** | `tonic` | gRPC server |
| **Metrics** | `metrics` | Metrics framework |
| **Metrics** | `metrics-exporter-prometheus` | Prometheus exporter |
| **Tracing** | `tracing` | Distributed tracing |
| **Tracing** | `tracing-subscriber` | Tracing subscriber |

---

## Project Structure

```
timescale-rs/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
│
├── crates/
│   ├── core/                      # Core database engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs          # Configuration
│   │       ├── error.rs           # Error types
│   │       └── types.rs           # Data types
│   │
│   ├── storage/                   # Storage engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── lsm/               # LSM tree
│   │       │   ├── mod.rs
│   │       │   ├── memtable.rs
│   │       │   ├── sstable.rs
│   │       │   └── compactor.rs
│   │       ├── segment/           # Time-series segments
│   │       │   ├── mod.rs
│   │       │   ├── compression.rs
│   │       │   └── encoding.rs
│   │       └── wal/               # Write-ahead log
│   │           ├── mod.rs
│   │           └── record.rs
│   │
│   ├── query/                     # Query engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── planner.rs         # Query planner
│   │       ├── executor.rs        # Execution engine
│   │       └── hypertable.rs      # Hypertable support
│   │
│   ├── analytics/                 # Analytics functions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── time_weight.rs     # Time-weighted avg
│   │       ├── uddsketch.rs       # Percentile approx
│   │       ├── hyperloglog.rs     # Count distinct
│   │       ├── stats_agg.rs       # Statistics
│   │       └── lttb.rs            # Downsampling
│   │
│   ├── vector/                    # Vector search
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── diskann.rs         # DiskANN index
│   │       ├── quantization.rs    # SBQ compression
│   │       └── distance.rs        # Distance metrics
│   │
│   ├── api/                       # API layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pgwire.rs          # PostgreSQL protocol
│   │       ├── http.rs            # HTTP API
│   │       └── grpc.rs            # gRPC API
│   │
│   └── utils/                     # Utilities
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── bloom.rs           # Bloom filters
│           └── bitmap.rs          # Bitmap utilities
│
├── bin/
│   └── server/                    # Main server binary
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
│
├── tests/
│   ├── integration/               # Integration tests
│   └── benchmark/                 # Performance benchmarks
│
└── docs/
    ├── architecture.md
    ├── api.md
    └── deployment.md
```

---

## Implementation Roadmap

### Phase 1: Core Storage (Months 1-3)

**Goals:**
- [ ] Basic LSM tree implementation
- [ ] WAL for durability
- [ ] Basic compression (LZ4, DeltaDelta)
- [ ] Segment storage for time-series

**Milestones:**
1. Week 1-4: LSM tree with memtable and SSTables
2. Week 5-8: WAL implementation
3. Week 9-12: Compression and segment storage

### Phase 2: Query Engine (Months 4-6)

**Goals:**
- [ ] DataFusion integration
- [ ] Hypertable abstraction
- [ ] Time-range pruning
- [ ] Basic SQL support

**Milestones:**
1. Week 13-16: DataFusion integration
2. Week 17-20: Hypertable implementation
3. Week 21-24: Query optimization

### Phase 3: Analytics (Months 7-9)

**Goals:**
- [ ] Time-weighted average
- [ ] UddSketch implementation
- [ ] HyperLogLog
- [ ] Statistical functions

**Milestones:**
1. Week 25-28: Time-weighted aggregates
2. Week 29-32: Percentile approximation
3. Week 33-36: Statistical functions

### Phase 4: Production Features (Months 10-12)

**Goals:**
- [ ] Replication (Raft)
- [ ] Backup/restore
- [ ] Monitoring
- [ ] Performance tuning

**Milestones:**
1. Week 37-40: Replication
2. Week 41-44: Backup/restore
3. Week 45-48: Production hardening

---

## Production Considerations

### Durability

```rust
/// WAL configuration for durability
pub struct WalConfig {
    /// Sync mode
    pub sync_mode: SyncMode,
    /// WAL segment size
    pub segment_size: usize,
    /// Checkpoint interval
    pub checkpoint_interval: Duration,
}

#[derive(Clone, Copy)]
pub enum SyncMode {
    /// Sync after every write (safest, slowest)
    Full,
    /// Sync after every transaction
    Normal,
    /// Async sync (fastest, risk of data loss)
    Async,
}
```

### Replication

```rust
/// Raft-based replication
pub struct ReplicationConfig {
    /// Node ID
    pub node_id: NodeId,
    /// Peer nodes
    pub peers: Vec<NodeId>,
    /// Election timeout
    pub election_timeout: Duration,
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
}
```

### Monitoring

```rust
/// Prometheus metrics
use metrics::{counter, gauge, histogram};

// Track writes
counter!("writes_total", 1, "table" => "metrics");

// Track latency
histogram!("write_latency_seconds", elapsed.as_secs_f64());

// Track storage size
gauge!("storage_size_bytes", total_size as f64);
```

### Resource Management

```rust
/// Memory budget configuration
pub struct MemoryConfig {
    /// Total memory budget
    pub total_budget: usize,
    /// Memtable budget (typically 25%)
    pub memtable_budget: usize,
    /// Cache budget (typically 50%)
    pub cache_budget: usize,
    /// Query budget (typically 25%)
    pub query_budget: usize,
}

impl MemoryConfig {
    pub fn new(total: usize) -> Self {
        Self {
            memtable_budget: total / 4,
            cache_budget: total / 2,
            query_budget: total / 4,
            total_budget: total,
        }
    }
}
```

---

## Related Documentation

- [Storage System Guide](./storage-system-guide.md)
- [Production Guide](./production-grade.md)
- [Analytics Functions](./analytics-functions.md)
