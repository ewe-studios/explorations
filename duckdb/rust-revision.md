---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs/src.duckdb/duckdb/
explored_at: 2026-04-04
focus: Replicating DuckDB patterns in Rust using DataFusion, Arrow, Parquet crates
---

# Rust Revision: Building an Analytical Database in Rust

## Overview

This guide shows how to replicate DuckDB's analytical database patterns in Rust using the Apache Arrow ecosystem (DataFusion, Arrow, Parquet). We cover columnar storage, vectorized execution, compression, and query optimization.

## Why Rust for Analytical Databases?

| Feature | DuckDB (C++) | Rust Equivalent |
|---------|--------------|-----------------|
| Memory Safety | Manual (RAII) | Ownership system |
| Columnar Format | Custom | Apache Arrow |
| Vectorized Exec | Custom vectors | Arrow Arrays |
| Query Engine | Custom | DataFusion |
| Parquet | Custom | parquet crate |
| Compression | Custom | arrow/bitpacking, fsst-rs |
| SIMD | Manual intrinsics | std::simd, portable-simd |

## Project Setup

### Cargo.toml

```toml
[package]
name = "rust-analytics-db"
version = "0.1.0"
edition = "2021"

[dependencies]
# Apache Arrow ecosystem
arrow = { version = "50", features = ["prettyprint"] }
arrow-array = "50"
arrow-schema = "50"
arrow-select = "50"
arrow-compute = "50"
arrow-ord = "50"
arrow-row = "50"
parquet = { version = "50", features = ["async"] }
datafusion = "35"
datafusion-common = "35"
datafusion-expr = "35"
datafusion-execution = "35"
datafusion-physical-expr = "35"

# Async runtime
tokio = { version = "1.35", features = ["full"] }
tokio-util = { version = "0.7", features = ["compat"] }

# Object storage
aws-config = "1.1"
aws-sdk-s3 = "1.1"
aws-credential-types = "1.1"
reqwest = { version = "0.11", features = ["stream"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Compression
lz4_flex = "0.11"
zstd = "0.13"
snap = "1.1"

# Utilities
bytes = "1.5"
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
async-trait = "0.1"
futures = "0.3"

# SIMD (nightly for std::simd, or use portable-simd)
# portable-simd = "0.1"  # Stable alternative

[profile.release]
lto = true
codegen-units = 1
target-cpu = "native"  # Enable CPU-specific optimizations
```

## Columnar Storage with Arrow

### Array Structure

```rust
// src/storage/arrow_column.rs

use arrow_array::{
    Array, ArrayRef,
    Int32Array, Int64Array, Float32Array, Float64Array,
    StringArray, BooleanArray,
    GenericByteArray,
};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;

/// Column wrapper with compression metadata
pub struct ArrowColumn {
    name: String,
    data_type: DataType,
    batches: Vec<ArrayRef>,
    total_rows: usize,
}

impl ArrowColumn {
    pub fn new(name: String, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            batches: Vec::new(),
            total_rows: 0,
        }
    }
    
    /// Append data batch
    pub fn append_batch(&mut self, batch: ArrayRef) -> Result<(), DatabaseError> {
        if batch.data_type() != &self.data_type {
            return Err(DatabaseError::TypeMismatch {
                expected: self.data_type.clone(),
                actual: batch.data_type().clone(),
            });
        }
        
        self.total_rows += batch.len();
        self.batches.push(batch);
        Ok(())
    }
    
    /// Get concatenated array (for small datasets)
    pub fn concatenate(&self) -> Result<ArrayRef, arrow::error::ArrowError> {
        arrow::compute::concat(&self.batches.iter().map(|a| a.as_ref()).collect::<Vec<_>>())
    }
    
    /// Get total row count
    pub fn len(&self) -> usize {
        self.total_rows
    }
    
    pub fn is_empty(&self) -> bool {
        self.total_rows == 0
    }
}

/// Record batch with compression info
pub struct CompressedRecordBatch {
    batch: arrow::record_batch::RecordBatch,
    compression: CompressionType,
    compressed_size: usize,
    uncompressed_size: usize,
}

impl CompressedRecordBatch {
    pub fn new(batch: arrow::record_batch::RecordBatch) -> Self {
        let uncompressed_size = batch.get_array_memory_size();
        
        Self {
            batch,
            compression: CompressionType::Uncompressed,
            compressed_size: uncompressed_size,
            uncompressed_size,
        }
    }
    
    /// Compress the batch
    pub fn compress(&mut self, compression: CompressionType) -> Result<(), DatabaseError> {
        self.compression = compression;
        // Apply compression based on type
        self.compressed_size = match compression {
            CompressionType::Lz4 => self.compress_lz4(),
            CompressionType::Zstd => self.compress_zstd(),
            CompressionType::Snappy => self.compress_snappy(),
            CompressionType::Uncompressed => self.uncompressed_size,
        };
        Ok(())
    }
    
    fn compress_lz4(&mut self) -> usize {
        // LZ4 compression logic
        // In practice, use parquet's built-in compression
        self.uncompressed_size // Placeholder
    }
    
    fn compress_zstd(&mut self) -> usize {
        self.uncompressed_size // Placeholder
    }
    
    fn compress_snappy(&mut self) -> usize {
        self.uncompressed_size // Placeholder
    }
    
    pub fn compression_ratio(&self) -> f64 {
        self.compressed_size as f64 / self.uncompressed_size as f64
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CompressionType {
    Uncompressed,
    Lz4,
    Zstd,
    Snappy,
    // Custom compression types
    Rle,
    Dictionary,
    BitPacked,
}
```

### Custom Columnar Format (DuckDB-style)

```rust
// src/storage/column_format.rs

use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::sync::Arc;

/// Column segment with compression
pub struct ColumnSegment {
    segment_id: u32,
    row_count: u32,
    compression: CompressionType,
    data: Bytes,
    statistics: ColumnStatistics,
}

impl ColumnSegment {
    pub fn encode<T: Encodable>(values: &[T], compression: CompressionType) -> Self {
        // Encode values to bytes
        let mut buffer = BytesMut::new();
        for value in values {
            value.encode(&mut buffer);
        }
        
        // Apply compression
        let compressed_data = match compression {
            CompressionType::Rle => Self::compress_rle(buffer.freeze()),
            CompressionType::Dictionary => Self::compress_dictionary(buffer.freeze()),
            CompressionType::BitPacked => Self::compress_bitpacked(buffer.freeze()),
            _ => buffer.freeze(),
        };
        
        // Calculate statistics
        let stats = ColumnStatistics::from_values(values);
        
        Self {
            segment_id: 0,
            row_count: values.len() as u32,
            compression,
            data: compressed_data,
            statistics: stats,
        }
    }
    
    pub fn decode<T: Decodable>(&self) -> Result<Vec<T>, DatabaseError> {
        // Decompress
        let decompressed = match self.compression {
            CompressionType::Rle => Self::decompress_rle(&self.data),
            CompressionType::Dictionary => Self::decompress_dictionary(&self.data),
            CompressionType::BitPacked => Self::decompress_bitpacked(&self.data),
            _ => self.data.clone(),
        };
        
        // Decode values
        let mut values = Vec::with_capacity(self.row_count as usize);
        let mut buf = decompressed.reader();
        
        while buf.has_remaining() {
            values.push(T::decode(&mut buf)?);
        }
        
        Ok(values)
    }
    
    /// RLE compression for runs
    fn compress_rle(data: Bytes) -> Bytes {
        let mut output = BytesMut::new();
        let mut input = data.reader();
        
        while input.has_remaining() {
            let byte = input.get_u8();
            let mut run_length = 1u8;
            
            // Count run length
            let pos = input.reader().position();
            while input.has_remaining() && run_length < 127 {
                let next = input.get_u8();
                if next == byte {
                    run_length += 1;
                } else {
                    input.set_position(pos);
                    break;
                }
            }
            
            // Encode: high bit = is_run, lower 7 bits = length/value
            if run_length > 1 {
                output.put_u8(0x80 | run_length);
                output.put_u8(byte);
            } else {
                output.put_u8(byte);
            }
        }
        
        output.freeze()
    }
    
    fn decompress_rle(data: &Bytes) -> Bytes {
        let mut output = BytesMut::new();
        let mut input = data.reader();
        
        while input.has_remaining() {
            let header = input.get_u8();
            
            if header & 0x80 != 0 {
                // Run
                let run_length = (header & 0x7F) as usize;
                let value = input.get_u8();
                for _ in 0..run_length {
                    output.put_u8(value);
                }
            } else {
                // Literal
                output.put_u8(header);
            }
        }
        
        output.freeze()
    }
}

/// Column statistics for pruning
pub struct ColumnStatistics {
    pub null_count: u32,
    pub row_count: u32,
    pub min_value: Option<Bytes>,
    pub max_value: Option<Bytes>,
    pub distinct_count: Option<u32>,
}

impl ColumnStatistics {
    pub fn from_values<T: Ord + Clone>(values: &[T]) -> Self {
        let null_count = 0; // Simplified
        let row_count = values.len() as u32;
        
        let (min, max) = if values.is_empty() {
            (None, None)
        } else {
            let mut sorted = values.to_vec();
            sorted.sort();
            (
                Some(sorted.first().cloned()),
                Some(sorted.last().cloned()),
            )
        };
        
        Self {
            null_count,
            row_count,
            min_value: None, // Would serialize min
            max_value: None,
            distinct_count: None,
        }
    }
    
    /// Check if predicate can prune this segment
    pub fn can_prune_eq(&self, value: &Bytes) -> bool {
        // If value < min or value > max, can prune
        if let Some(min) = &self.min_value {
            if value < min {
                return true;
            }
        }
        if let Some(max) = &self.max_value {
            if value > max {
                return true;
            }
        }
        false
    }
    
    pub fn can_prune_lt(&self, value: &Bytes) -> bool {
        // If min >= value, can prune
        if let Some(min) = &self.min_value {
            return min >= value;
        }
        false
    }
    
    pub fn can_prune_gt(&self, value: &Bytes) -> bool {
        // If max <= value, can prune
        if let Some(max) = &self.max_value {
            return max <= value;
        }
        false
    }
}

pub trait Encodable {
    fn encode(&self, buf: &mut BytesMut);
}

pub trait Decodable: Sized {
    fn decode(buf: &mut impl Buf) -> Result<Self, DatabaseError>;
}

impl Encodable for i32 {
    fn encode(&self, buf: &mut BytesMut) {
        buf.put_i32(*self);
    }
}

impl Decodable for i32 {
    fn decode(buf: &mut impl Buf) -> Result<Self, DatabaseError> {
        Ok(buf.get_i32())
    }
}
```

## Vectorized Execution

### Vector (Array) Operations

```rust
// src/execution/vectorized.rs

use arrow_array::{Array, ArrowPrimitiveType, PrimitiveArray};
use arrow_schema::ArrowError;
use std::marker::PhantomData;

/// Vectorized operator trait
pub trait VectorizedOperator {
    type Input: Array;
    type Output: Array;
    
    fn execute(&self, input: &Self::Input) -> Result<Self::Output, ArrowError>;
}

/// Vectorized filter
pub struct VectorizedFilter<P>
where
    P: Fn(&dyn Array) -> Result<BooleanArray, ArrowError>,
{
    predicate: P,
}

impl<P> VectorizedFilter<P>
where
    P: Fn(&dyn Array) -> Result<BooleanArray, ArrowError>,
{
    pub fn new(predicate: P) -> Self {
        Self { predicate }
    }
    
    pub fn filter(&self, batch: &RecordBatch) -> Result<RecordBatch, ArrowError> {
        // Apply predicate to get selection
        let predicate_array = (self.predicate)(batch.column(0))?;
        
        // Filter all columns
        let filtered_columns: Result<Vec<_>, _> = batch
            .columns()
            .iter()
            .map(|col| arrow::compute::filter(col, &predicate_array))
            .collect();
        
        RecordBatch::try_new(batch.schema(), filtered_columns?)
    }
}

/// Vectorized projection
pub struct VectorizedProjection<F>
where
    F: Fn(&[&dyn Array]) -> Result<ArrayRef, ArrowError>,
{
    transform: F,
}

impl<F> VectorizedProjection<F>
where
    F: Fn(&[&dyn Array]) -> Result<ArrayRef, ArrowError>,
{
    pub fn new(transform: F) -> Self {
        Self { transform }
    }
    
    pub fn project(&self, batch: &RecordBatch) -> Result<ArrayRef, ArrowError> {
        let arrays: Vec<&dyn Array> = batch.columns().iter().map(|a| a.as_ref()).collect();
        (self.transform)(&arrays)
    }
}

/// SIMD-optimized sum for primitive arrays
pub fn simd_sum<T>(array: &PrimitiveArray<T>) -> f64
where
    T: ArrowPrimitiveType<Native = i32 + i64 + f32 + f64>,
{
    let values = array.values();
    let mut sum = 0.0;
    
    // Process in chunks of 8 (SIMD width)
    let chunk_size = 8;
    let chunks = values.chunks_exact(chunk_size);
    let remainder = chunks.remainder();
    
    // SIMD chunk processing
    for chunk in chunks {
        sum += chunk.iter().map(|&v| v as f64).sum::<f64>();
    }
    
    // Remainder
    for &v in remainder {
        sum += v as f64;
    }
    
    sum
}

/// Batch iterator for large datasets
pub struct BatchIterator<'a> {
    batches: &'a [RecordBatch],
    current: usize,
}

impl<'a> BatchIterator<'a> {
    pub fn new(batches: &'a [RecordBatch]) -> Self {
        Self { batches, current: 0 }
    }
}

impl<'a> Iterator for BatchIterator<'a> {
    type Item = &'a RecordBatch;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.batches.len() {
            let batch = &self.batches[self.current];
            self.current += 1;
            Some(batch)
        } else {
            None
        }
    }
}
```

### Query Execution Pipeline

```rust
// src/execution/pipeline.rs

use datafusion::physical_plan::{
    ExecutionPlan,
    SendableRecordBatchStream,
    RecordBatchStream,
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};

/// Execution pipeline builder
pub struct PipelineBuilder {
    sources: Vec<Arc<dyn ExecutionPlan>>,
    operators: Vec<Box<dyn PhysicalOperator>>,
    sink: Option<Box<dyn PhysicalSink>>,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            operators: Vec::new(),
            sink: None,
        }
    }
    
    pub fn add_source(mut self, source: Arc<dyn ExecutionPlan>) -> Self {
        self.sources.push(source);
        self
    }
    
    pub fn add_operator(mut self, op: Box<dyn PhysicalOperator>) -> Self {
        self.operators.push(op);
        self
    }
    
    pub fn with_sink(mut self, sink: Box<dyn PhysicalSink>) -> Self {
        self.sink = Some(sink);
        self
    }
    
    pub fn build(self) -> ExecutionPipeline {
        ExecutionPipeline {
            sources: self.sources,
            operators: self.operators,
            sink: self.sink,
        }
    }
}

/// Physical operator trait
pub trait PhysicalOperator: Send + Sync {
    fn name(&self) -> &str;
    
    /// Execute operator on input stream
    fn execute(
        &self,
        input: SendableRecordBatchStream,
    ) -> Result<SendableRecordBatchStream, DataFusionError>;
}

/// Physical sink trait
pub trait PhysicalSink: Send + Sync {
    fn write(&mut self, batch: &RecordBatch) -> Result<(), DataFusionError>;
    fn close(&mut self) -> Result<(), DataFusionError>;
}

/// Execution pipeline
pub struct ExecutionPipeline {
    sources: Vec<Arc<dyn ExecutionPlan>>,
    operators: Vec<Box<dyn PhysicalOperator>>,
    sink: Option<Box<dyn PhysicalSink>>,
}

impl ExecutionPipeline {
    pub async fn execute(&mut self) -> Result<(), DataFusionError> {
        // Execute each source through the operator chain
        for source in &self.sources {
            let mut stream = source.execute(0, &TaskContext::default())?;
            
            // Apply operators
            for operator in &self.operators {
                stream = operator.execute(stream)?;
            }
            
            // Write to sink
            if let Some(sink) = &mut self.sink {
                while let Some(result) = stream.next().await {
                    let batch = result?;
                    sink.write(&batch)?;
                }
                sink.close()?;
            }
        }
        
        Ok(())
    }
}

/// Filter operator implementation
pub struct FilterOperator {
    predicate: Arc<dyn PhysicalExpr>,
}

impl FilterOperator {
    pub fn new(predicate: Arc<dyn PhysicalExpr>) -> Self {
        Self { predicate }
    }
}

impl PhysicalOperator for FilterOperator {
    fn name(&self) -> &str {
        "Filter"
    }
    
    fn execute(
        &self,
        input: SendableRecordBatchStream,
    ) -> Result<SendableRecordBatchStream, DataFusionError> {
        Ok(Box::pin(FilterStream {
            input,
            predicate: self.predicate.clone(),
        }))
    }
}

struct FilterStream {
    input: SendableRecordBatchStream,
    predicate: Arc<dyn PhysicalExpr>,
}

impl Stream for FilterStream {
    type Item = Result<RecordBatch, DataFusionError>;
    
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.input.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(batch))) => {
                // Apply predicate
                let predicate_result = self.predicate.evaluate(&batch)?;
                let predicate_array = predicate_result.into_array(batch.num_rows())?;
                let bool_array = predicate_array.as_any().downcast_ref::<BooleanArray>().unwrap();
                
                // Filter batch
                let filtered = arrow::compute::filter_record_batch(&batch, bool_array)?;
                Poll::Ready(Some(Ok(filtered)))
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
```

## Object Storage Integration

```rust
// src/storage/object_store.rs

use aws_sdk_s3::{Client, Config};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Object store abstraction
pub trait ObjectStore: Send + Sync {
    /// Get object metadata
    fn head(&self, path: &str) -> BoxFuture<Result<ObjectMeta, ObjectStoreError>>;
    
    /// Get object content
    fn get(&self, path: &str) -> BoxFuture<Result<Bytes, ObjectStoreError>>;
    
    /// Get object content with range
    fn get_range(
        &self,
        path: &str,
        start: u64,
        end: u64,
    ) -> BoxFuture<Result<Bytes, ObjectStoreError>>;
    
    /// List objects with prefix
    fn list(&self, prefix: Option<&str>) -> BoxStream<Result<ObjectMeta, ObjectStoreError>>;
}

/// S3 implementation
pub struct S3ObjectStore {
    client: Client,
    bucket: String,
    config: S3Config,
}

impl S3ObjectStore {
    pub fn new(client: Client, bucket: String, config: S3Config) -> Self {
        Self {
            client,
            bucket,
            config,
        }
    }
}

impl ObjectStore for S3ObjectStore {
    fn head(&self, path: &str) -> BoxFuture<Result<ObjectMeta, ObjectStoreError>> {
        Box::pin(async move {
            let response = self
                .client
                .head_object()
                .bucket(&self.bucket)
                .key(path)
                .send()
                .await
                .map_err(|e| ObjectStoreError::S3Error(e.into()))?;
            
            Ok(ObjectMeta {
                path: path.to_string(),
                size: response.content_length() as u64,
                last_modified: response.last_modified(),
            })
        })
    }
    
    fn get(&self, path: &str) -> BoxFuture<Result<Bytes, ObjectStoreError>> {
        Box::pin(async move {
            let response = self
                .client
                .get_object()
                .bucket(&self.bucket)
                .key(path)
                .send()
                .await
                .map_err(|e| ObjectStoreError::S3Error(e.into()))?;
            
            let body = response.body.collect().await.map_err(|e| e.into())?;
            Ok(body.to_bytes())
        })
    }
    
    fn get_range(&self, path: &str, start: u64, end: u64) -> BoxFuture<Result<Bytes, ObjectStoreError>> {
        Box::pin(async move {
            let range = format!("bytes={}-{}", start, end);
            
            let response = self
                .client
                .get_object()
                .bucket(&self.bucket)
                .key(path)
                .range(range)
                .send()
                .await
                .map_err(|e| ObjectStoreError::S3Error(e.into()))?;
            
            let body = response.body.collect().await.map_err(|e| e.into())?;
            Ok(body.to_bytes())
        })
    }
    
    fn list(&self, prefix: Option<&str>) -> BoxStream<Result<ObjectMeta, ObjectStoreError>> {
        // S3 list implementation
        todo!()
    }
}

/// Cached object store wrapper
pub struct CachedObjectStore<S: ObjectStore> {
    inner: S,
    cache: Arc<Mutex<ObjectCache>>,
}

impl<S: ObjectStore> CachedObjectStore<S> {
    pub fn new(inner: S, cache_size: usize) -> Self {
        Self {
            inner,
            cache: Arc::new(Mutex::new(ObjectCache::new(cache_size))),
        }
    }
}

impl<S: ObjectStore> ObjectStore for CachedObjectStore<S> {
    fn head(&self, path: &str) -> BoxFuture<Result<ObjectMeta, ObjectStoreError>> {
        Box::pin(async move {
            // Check cache first
            {
                let cache = self.cache.lock().await;
                if let Some(meta) = cache.get_meta(path) {
                    return Ok(meta.clone());
                }
            }
            
            // Fetch from inner
            let meta = self.inner.head(path).await?;
            
            // Cache metadata
            {
                let mut cache = self.cache.lock().await;
                cache.put_meta(path.to_string(), meta.clone());
            }
            
            Ok(meta)
        })
    }
    
    fn get(&self, path: &str) -> BoxFuture<Result<Bytes, ObjectStoreError>> {
        Box::pin(async move {
            // Check cache first
            {
                let cache = self.cache.lock().await;
                if let Some(data) = cache.get_data(path) {
                    return Ok(data.clone());
                }
            }
            
            // Fetch from inner
            let data = self.inner.get(path).await?;
            
            // Cache data
            {
                let mut cache = self.cache.lock().await;
                cache.put_data(path.to_string(), data.clone());
            }
            
            Ok(data)
        })
    }
    
    fn get_range(&self, path: &str, start: u64, end: u64) -> BoxFuture<Result<Bytes, ObjectStoreError>> {
        Box::pin(async move {
            // For range requests, check if we have the full object cached
            // If not, fetch the range directly
            
            let range_size = (end - start + 1) as usize;
            
            // Heuristic: if range is large, might as well cache it
            if range_size > 1024 * 1024 {
                // Fetch and cache
                let data = self.inner.get_range(path, start, end).await?;
                
                {
                    let mut cache = self.cache.lock().await;
                    cache.put_data(format!("{}:{}:{}", path, start, end), data.clone());
                }
                
                return Ok(data);
            }
            
            // Small range: fetch directly
            self.inner.get_range(path, start, end).await
        })
    }
    
    fn list(&self, prefix: Option<&str>) -> BoxStream<Result<ObjectMeta, ObjectStoreError>> {
        self.inner.list(prefix)
    }
}

/// In-memory cache with LRU eviction
pub struct ObjectCache {
    max_size: usize,
    current_size: usize,
    data: lru::LruCache<String, Bytes>,
    meta: HashMap<String, ObjectMeta>,
}

impl ObjectCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            current_size: 0,
            data: lru::LruCache::unbounded(),
            meta: HashMap::new(),
        }
    }
    
    pub fn get_data(&mut self, key: &str) -> Option<Bytes> {
        self.data.get(key).cloned()
    }
    
    pub fn put_data(&mut self, key: String, data: Bytes) {
        let size = data.len();
        
        // Evict if necessary
        while self.current_size + size > self.max_size {
            if let Some((_, evicted)) = self.data.pop_lru() {
                self.current_size -= evicted.len();
            } else {
                break;
            }
        }
        
        self.data.put(key, data);
        self.current_size += size;
    }
    
    pub fn get_meta(&self, key: &str) -> Option<&ObjectMeta> {
        self.meta.get(key)
    }
    
    pub fn put_meta(&mut self, key: String, meta: ObjectMeta) {
        self.meta.insert(key, meta);
    }
}

pub struct ObjectMeta {
    pub path: String,
    pub size: u64,
    pub last_modified: Option<String>,
}

pub struct S3Config {
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}
```

## Query Optimization

### Statistics-Based Pruning

```rust
// src/optimizer/pruning.rs

use arrow_schema::DataType;
use std::collections::HashMap;

/// Partition pruning based on statistics
pub struct PartitionPruner {
    partitions: Vec<PartitionStats>,
}

impl PartitionPruner {
    pub fn new(partitions: Vec<PartitionStats>) -> Self {
        Self { partitions }
    }
    
    /// Get partitions that match predicate
    pub fn prune(&self, predicate: &Expr) -> Vec<usize> {
        let mut matching = Vec::new();
        
        for (i, partition) in self.partitions.iter().enumerate() {
            if !partition.can_prune(predicate) {
                matching.push(i);
            }
        }
        
        matching
    }
}

/// Statistics for a partition
pub struct PartitionStats {
    partition_id: usize,
    column_stats: HashMap<String, ColumnRange>,
    row_count: u64,
}

impl PartitionStats {
    pub fn can_prune(&self, predicate: &Expr) -> bool {
        match predicate {
            Expr::Column(name) => false,
            Expr::Literal(_) => false,
            Expr::BinaryExpr { left, op, right } => {
                self.can_prune_binary(left, op, right)
            }
            Expr::InList { expr, list, .. } => {
                self.can_prune_in_list(expr, list)
            }
            _ => false,
        }
    }
    
    fn can_prune_binary(&self, left: &Expr, op: &Operator, right: &Expr) -> bool {
        // Get column name and value from predicate
        let (col_name, value) = match (left.as_ref(), right.as_ref()) {
            (Expr::Column(name), Expr::Literal(val)) => (name, val),
            (Expr::Literal(val), Expr::Column(name)) => (name, val),
            _ => return false,
        };
        
        // Get column statistics
        let Some(col_stats) = self.column_stats.get(col_name) else {
            return false;
        };
        
        match op {
            Operator::Eq => {
                // Can prune if value outside [min, max]
                value < &col_stats.min || value > &col_stats.max
            }
            Operator::NotEq => false,  // Can't prune for !=
            Operator::Lt => {
                // Can prune if min >= value
                col_stats.min >= *value
            }
            Operator::LtEq => {
                col_stats.min > *value
            }
            Operator::Gt => {
                // Can prune if max <= value
                col_stats.max <= *value
            }
            Operator::GtEq => {
                col_stats.max < *value
            }
            _ => false,
        }
    }
    
    fn can_prune_in_list(&self, expr: &Expr, list: &[Expr]) -> bool {
        let Expr::Column(col_name) = expr else {
            return false;
        };
        
        let Some(col_stats) = self.column_stats.get(col_name) else {
            return false;
        };
        
        // Check if any value in list is within range
        for val in list {
            if let Expr::Literal(val) = val {
                if val >= &col_stats.min && val <= &col_stats.max {
                    return false;  // Can't prune, at least one value matches
                }
            }
        }
        
        true  // All values outside range
    }
}

pub struct ColumnRange {
    pub min: ScalarValue,
    pub max: ScalarValue,
    pub null_count: u64,
}
```

## Conclusion

Rust provides excellent foundations for building analytical databases:

1. **Type Safety**: Compile-time guarantees for data types
2. **Zero-Cost Abstractions**: Arrow arrays with minimal overhead
3. **Async Runtime**: Tokio for I/O-bound operations
4. **Memory Management**: Ownership prevents use-after-free
5. **SIMD Support**: Portable SIMD for vectorized operations
6. **Ecosystem**: DataFusion, Arrow, Parquet crates

Key differences from DuckDB:
- Use Arrow format instead of custom columnar
- Leverage DataFusion instead of custom query engine
- Use tokio instead of custom threading
- Compile-time safety vs runtime checks
