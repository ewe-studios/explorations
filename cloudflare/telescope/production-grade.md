---
title: "Production-Grade Telescope Implementation"
subtitle: "Performance, scaling, monitoring, and deployment for production filesystem backend"
---

# Production-Grade Telescope Implementation

## Introduction

This document covers production considerations for deploying telescope at scale, including performance optimizations, memory management, scaling strategies, monitoring, and operational concerns.

## Table of Contents

1. [Performance Optimizations](#performance-optimizations)
2. [Memory Management](#memory-management)
3. [Batching and Throughput](#batching-and-throughput)
4. [Serialization and Storage](#serialization-and-storage)
5. [Serving Infrastructure](#serving-infrastructure)
6. [Monitoring and Observability](#monitoring-and-observability)
7. [High Availability](#high-availability)
8. [Security Considerations](#security-considerations)

---

## Performance Optimizations

### Async I/O for File Operations

```rust
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};

/// Optimized async file writer with buffering
pub struct AsyncFileWriter {
    writer: BufWriter<fs::File>,
    buffer_size: usize,
}

impl AsyncFileWriter {
    pub async fn create(path: &Path, buffer_size: usize) -> Result<Self, IoError> {
        let file = fs::File::create(path).await?;
        Ok(Self {
            writer: BufWriter::with_capacity(buffer_size, file),
            buffer_size,
        })
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), IoError> {
        self.writer.write_all(data).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), IoError> {
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn sync_all(&mut self) -> Result<(), IoError> {
        self.writer.get_mut().sync_all().await?;
        Ok(())
    }
}

/// Parallel result writer for multiple files
pub struct ParallelResultWriter {
    concurrent_writes: usize,
    semaphore: Arc<Semaphore>,
}

impl ParallelResultWriter {
    pub fn new(concurrent_writes: usize) -> Self {
        Self {
            concurrent_writes,
            semaphore: Arc::new(Semaphore::new(concurrent_writes)),
        }
    }

    pub async fn write_results(
        &self,
        results: Vec<(PathBuf, Vec<u8>)>,
    ) -> Result<(), WriteError> {
        let mut tasks = Vec::new();

        for (path, data) in results {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let task = tokio::spawn(async move {
                let mut writer = AsyncFileWriter::create(&path, 8192).await?;
                writer.write(&data).await?;
                writer.sync_all().await?;
                Ok::<_, IoError>(())
            });
            tasks.push(task);
        }

        // Wait for all writes to complete
        let results = futures::future::join_all(tasks).await;

        for result in results {
            result??;
        }

        Ok(())
    }
}
```

### Connection Pooling for Remote Storage

```rust
use deadpool::managed::{self, Metrics, Object};
use reqwest::{Client, ClientBuilder};
use std::time::Duration;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: usize,
    pub min_size: usize,
    pub timeout: Duration,
    pub idle_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 20,
            min_size: 5,
            timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(60),
        }
    }
}

/// HTTP client pool manager
pub struct HttpClientManager {
    config: PoolConfig,
}

impl managed::Manager for HttpClientManager {
    type Type = Client;
    type Error = reqwest::Error;

    async fn create(&self) -> Result<Client, Self::Error> {
        ClientBuilder::new()
            .timeout(self.config.timeout)
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(self.config.max_size)
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .build()
    }

    async fn recycle(
        &self,
        obj: &mut Client,
        _metrics: &Metrics,
    ) -> managed::RecycleResult<Self::Error> {
        // HTTP clients are generally reusable without recycling
        Ok(())
    }
}

pub type HttpClientPool = managed::Pool<HttpClientManager>;

impl HttpClientPool {
    pub fn new(config: PoolConfig) -> Self {
        let manager = HttpClientManager { config: config.clone() };
        managed::Pool::builder(manager)
            .max_size(config.max_size)
            .min_size(config.min_size)
            .timeout(config.timeout)
            .build()
            .unwrap()
    }

    pub async fn get(&self) -> Result<Object<HttpClientManager>, managed::PoolError> {
        self.get().await
    }
}

/// Remote storage client with pooled connections
pub struct RemoteStorageClient {
    pool: HttpClientPool,
    base_url: String,
}

impl RemoteStorageClient {
    pub fn new(base_url: String, pool_config: PoolConfig) -> Self {
        Self {
            pool: HttpClientPool::new(pool_config),
            base_url,
        }
    }

    pub async fn upload(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        let client = self.pool.get().await?;
        let url = format!("{}/{}", self.base_url, key);

        client
            .put(&url)
            .body(data.to_vec())
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let client = self.pool.get().await?;
        let url = format!("{}/{}", self.base_url, key);

        let response = client.get(&url).send().await?;
        Ok(response.bytes().await?.to_vec())
    }

    pub async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let client = self.pool.get().await?;
        let url = format!("{}/_list?prefix={}", self.base_url, prefix);

        let response = client.get(&url).send().await?;
        Ok(response.json().await?)
    }
}
```

### Caching Layer

```rust
use moka::future::Cache;
use std::time::Duration;

/// Multi-level cache for test results
pub struct ResultCache {
    /// L1: In-memory cache (fast, small)
    l1_cache: Cache<String, Vec<u8>>,
    /// L2: Disk cache (slower, larger)
    l2_cache: DiskCache,
    /// Statistics
    stats: Arc<CacheStats>,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub l1_hits: AtomicU64,
    pub l1_misses: AtomicU64,
    pub l2_hits: AtomicU64,
    pub l2_misses: AtomicU64,
}

impl ResultCache {
    pub fn new(l1_size: u64, l2_path: PathBuf) -> Self {
        Self {
            l1_cache: Cache::builder()
                .max_capacity(l1_size)
                .time_to_live(Duration::from_secs(3600))
                .time_to_idle(Duration::from_secs(300))
                .build(),
            l2_cache: DiskCache::new(l2_path),
            stats: Arc::default(),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        // Try L1 first
        if let Some(value) = self.l1_cache.get(key).await {
            self.stats.l1_hits.fetch_add(1, Ordering::Relaxed);
            return Some(value);
        }
        self.stats.l1_misses.fetch_add(1, Ordering::Relaxed);

        // Try L2
        if let Some(value) = self.l2_cache.get(key).await {
            self.stats.l2_hits.fetch_add(1, Ordering::Relaxed);
            // Promote to L1
            self.l1_cache.insert(key.to_string(), value.clone()).await;
            return Some(value);
        }
        self.stats.l2_misses.fetch_add(1, Ordering::Relaxed);

        None
    }

    pub async fn insert(&self, key: &str, value: Vec<u8>) {
        // Insert to both L1 and L2
        self.l1_cache.insert(key.to_string(), value.clone()).await;
        self.l2_cache.insert(key, value).await;
    }

    pub async fn remove(&self, key: &str) {
        self.l1_cache.remove(key);
        self.l2_cache.remove(key).await;
    }

    pub fn get_stats(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot {
            l1_hits: self.stats.l1_hits.load(Ordering::Relaxed),
            l1_misses: self.stats.l1_misses.load(Ordering::Relaxed),
            l2_hits: self.stats.l2_hits.load(Ordering::Relaxed),
            l2_misses: self.stats.l2_misses.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CacheStatsSnapshot {
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
}

impl CacheStatsSnapshot {
    pub fn l1_hit_rate(&self) -> f64 {
        let total = self.l1_hits + self.l1_misses;
        if total == 0 {
            0.0
        } else {
            self.l1_hits as f64 / total as f64
        }
    }

    pub fn overall_hit_rate(&self) -> f64 {
        let total = self.l1_hits + self.l1_misses;
        if total == 0 {
            0.0
        } else {
            (self.l1_hits + self.l2_hits) as f64 / total as f64
        }
    }
}

/// Disk cache using sled or similar
pub struct DiskCache {
    db: sled::Db,
    path: PathBuf,
}

impl DiskCache {
    pub fn new(path: PathBuf) -> Self {
        let db = sled::Config::default()
            .path(&path)
            .cache_capacity(1024 * 1024 * 1024)  // 1GB
            .flush_every_ms(Some(1000))
            .open()
            .unwrap();

        Self { db, path }
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let key_bytes = key.as_bytes();
        self.db.get(key_bytes).ok().flatten().map(|iv| iv.to_vec())
    }

    pub async fn insert(&self, key: &str, value: Vec<u8>) {
        let key_bytes = key.as_bytes();
        self.db.insert(key_bytes, value).ok();
    }

    pub async fn remove(&self, key: &str) {
        let key_bytes = key.as_bytes();
        self.db.remove(key_bytes).ok();
    }
}
```

---

## Memory Management

### Bounded Memory Processing

```rust
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Memory-bounded test processor
pub struct BoundedTestProcessor {
    /// Limit concurrent test executions
    concurrency_semaphore: Arc<Semaphore>,
    /// Limit total memory usage
    memory_semaphore: Arc<Semaphore>,
    /// Estimated memory per test
    memory_per_test: usize,
}

impl BoundedTestProcessor {
    pub fn new(max_concurrent_tests: usize, max_memory_mb: usize) -> Self {
        let memory_per_test = 50 * 1024 * 1024;  // 50MB estimate
        let max_memory_bytes = max_memory_mb * 1024 * 1024;
        let memory_permits = max_memory_bytes / memory_per_test;

        Self {
            concurrency_semaphore: Arc::new(Semaphore::new(max_concurrent_tests)),
            memory_semaphore: Arc::new(Semaphore::new(memory_permits.max(1))),
            memory_per_test,
        }
    }

    pub async fn process_test(
        &self,
        config: LaunchOptions,
    ) -> Result<TestResult, ProcessError> {
        // Acquire concurrency permit
        let _concurrency_permit = self.concurrency_semaphore.acquire().await
            .map_err(|_| ProcessError::Shutdown)?;

        // Acquire memory permit
        let _memory_permit = self.memory_semaphore.acquire().await
            .map_err(|_| ProcessError::MemoryExhausted)?;

        // Process test (memory is bounded)
        self.execute_test(config).await
    }

    async fn execute_test(&self, config: LaunchOptions) -> Result<TestResult, ProcessError> {
        // Actual test execution
        // Memory is bounded by semaphore permits
        launch_test(config).await
    }
}

/// Streaming result processor for large results
pub struct StreamingResultProcessor {
    chunk_size: usize,
    max_in_flight: usize,
}

impl StreamingResultProcessor {
    pub fn new(chunk_size: usize, max_in_flight: usize) -> Self {
        Self {
            chunk_size,
            max_in_flight,
        }
    }

    pub async fn process_large_result(
        &self,
        result_path: &Path,
    ) -> Result<ProcessedResult, ProcessError> {
        use tokio::io::{AsyncReadExt, BufReader};

        let file = fs::File::open(result_path).await?;
        let mut reader = BufReader::with_capacity(self.chunk_size, file);
        let mut buffer = vec![0u8; self.chunk_size];

        let mut processed = Vec::new();
        let semaphore = Arc::new(Semaphore::new(self.max_in_flight));

        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;  // EOF
            }

            let chunk = buffer[..bytes_read].to_vec();
            let permit = semaphore.clone().acquire_owned().await?;

            let handle = tokio::spawn(async move {
                // Process chunk (e.g., compress, transform)
                self.process_chunk(chunk).await
            });

            processed.push(handle);
        }

        // Collect all results
        let mut final_result = Vec::new();
        for handle in processed {
            let chunk_result = handle.await??;
            final_result.extend(chunk_result);
        }

        Ok(ProcessedResult { data: final_result })
    }

    async fn process_chunk(&self, chunk: Vec<u8>) -> Result<Vec<u8>, ProcessError> {
        // Example: compress chunk
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&chunk)?;
        Ok(encoder.finish()?)
    }
}
```

### Zero-Copy Deserialization

```rust
use bytes::{Buf, Bytes};
use serde::Deserialize;

/// Zero-copy result deserialization
pub struct ZeroCopyDeserializer;

impl ZeroCopyDeserializer {
    /// Deserialize JSON without copying bytes
    pub fn deserialize_result<'a, T: Deserialize<'a>>(
        bytes: &'a [u8],
    ) -> Result<T, DeserializeError> {
        // Use simd_json for zero-copy parsing
        let mut bytes_copy = bytes.to_vec();
        let value = simd_json::to_owned_value(&mut bytes_copy)?;
        let result: T = serde_json::from_value(value)?;
        Ok(result)
    }

    /// Process result in chunks without full deserialization
    pub fn stream_deserialize<T, F>(
        bytes: Bytes,
        mut processor: F,
    ) -> Result<Vec<T>, DeserializeError>
    where
        T: Deserialize<'static>,
        F: FnMut(T) -> Result<(), DeserializeError>,
    {
        use serde_json::Deserializer;

        let stream_deser = Deserializer::from_slice(&bytes);

        for result in stream_deser.into_iter::<T>() {
            let item = result?;
            processor(item)?;
        }

        Ok(Vec::new())
    }
}

/// Memory-mapped file access for large results
pub struct MmapResultReader {
    mmap: memmap2::Mmap,
}

impl MmapResultReader {
    pub fn open(path: &Path) -> Result<Self, IoError> {
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap2::Mmap::map(&file)? };

        Ok(Self { mmap })
    }

    pub fn parse_json<'a, T: Deserialize<'a>>(&'a self) -> Result<T, DeserializeError> {
        // Zero-copy parsing from mmap
        let mut bytes = self.mmap.as_ref().to_vec();
        Ok(simd_json::from_slice(&mut bytes)?)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }
}
```

---

## Batching and Throughput

### Batch Test Execution

```rust
use tokio::sync::mpsc;

/// Batch test execution configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub batch_size: usize,
    pub max_concurrent: usize,
    pub timeout: Duration,
}

/// Batch test processor
pub struct BatchTestProcessor {
    config: BatchConfig,
    semaphore: Arc<Semaphore>,
}

impl BatchTestProcessor {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config: config.clone(),
            semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
        }
    }

    pub async fn process_batch(
        &self,
        tests: Vec<LaunchOptions>,
    ) -> BatchResult {
        let mut results = Vec::with_capacity(tests.len());
        let (tx, mut rx) = mpsc::channel(tests.len());

        // Spawn all tests with concurrency limit
        for (index, config) in tests.into_iter().enumerate() {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            let tx = tx.clone();

            tokio::spawn(async move {
                let start = Instant::now();
                let result = launch_test(config).await;
                let duration = start.elapsed();

                let _ = tx.send((index, result, duration)).await;
                drop(permit);
            });
        }

        // Drop original sender to signal completion
        drop(tx);

        // Collect results
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut total_duration = Duration::ZERO;

        while let Ok((index, result, duration)) = rx.recv().await {
            total_duration += duration;

            match &result {
                Ok(TestResult::Success { .. }) => success_count += 1,
                Ok(TestResult::Failure { .. }) | Err(_) => failure_count += 1,
            }

            if results.len() <= index {
                results.resize(index + 1, None);
            }
            results[index] = Some(result);
        }

        BatchResult {
            results: results.into_iter().flatten().collect(),
            success_count,
            failure_count,
            total_duration,
            average_duration: if success_count > 0 {
                total_duration / success_count as u32
            } else {
                Duration::ZERO
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchResult {
    pub results: Vec<TestResult>,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_duration: Duration,
    pub average_duration: Duration,
}

impl BatchResult {
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            0.0
        } else {
            self.success_count as f64 / total as f64
        }
    }
}
```

### Result Batching for Storage

```rust
use tokio::sync::mpsc;

/// Batched result writer
pub struct BatchedResultWriter {
    tx: mpsc::Sender<WriteBatch>,
    flush_interval: Duration,
}

struct WriteBatch {
    results: Vec<(PathBuf, Vec<u8>)>,
    flush_tx: oneshot::Sender<()>,
}

impl BatchedResultWriter {
    pub fn new(
        storage: Arc<dyn TestStorage>,
        batch_size: usize,
        flush_interval: Duration,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<WriteBatch>(100);

        // Background writer task
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(batch_size);
            let mut interval = tokio::time::interval(flush_interval);

            loop {
                tokio::select! {
                    Some(write_batch) = rx.recv() => {
                        batch.push((write_batch.results, write_batch.flush_tx));

                        if batch.len() >= batch_size {
                            Self::flush_batch(&storage, batch).await;
                            batch = Vec::with_capacity(batch_size);
                        }
                    }
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            Self::flush_batch(&storage, std::mem::take(&mut batch)).await;
                        }
                    }
                }
            }
        });

        Self { tx, flush_interval }
    }

    async fn flush_batch(
        storage: &Arc<dyn TestStorage>,
        batch: Vec<(Vec<(PathBuf, Vec<u8>)>, oneshot::Sender<()>)>,
    ) {
        let mut all_writes = Vec::new();
        let mut flush_txs = Vec::new();

        for (results, flush_tx) in batch {
            all_writes.extend(results);
            flush_txs.push(flush_tx);
        }

        // Write all results in parallel
        let writer = ParallelResultWriter::new(10);
        let _ = writer.write_results(all_writes).await;

        // Notify all waiters
        for tx in flush_txs {
            let _ = tx.send(());
        }
    }

    pub async fn write(
        &self,
        path: PathBuf,
        data: Vec<u8>,
    ) -> Result<(), WriteError> {
        let (tx, rx) = oneshot::channel();

        self.tx.send(WriteBatch {
            results: vec![(path, data)],
            flush_tx: tx,
        }).await?;

        rx.await.map_err(|_| WriteError::ChannelClosed)?;
        Ok(())
    }
}
```

---

## Serialization and Storage

### Efficient Serialization

```rust
use bincode::{Options, config};
use serde::{Serialize, Deserialize};

/// Binary serialization for storage efficiency
pub struct BinarySerializer;

impl BinarySerializer {
    pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>, SerializeError> {
        // Use fixint for consistent sizes across platforms
        let options = config()
            .with_fixint_encoding()
            .with_little_endian();

        Ok(options.serialize(value)?)
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, DeserializeError> {
        let options = config()
            .with_fixint_encoding()
            .with_little_endian();

        Ok(options.deserialize(bytes)?)
    }
}

/// Columnar storage for metrics (efficient for analytics)
pub struct ColumnarMetricsStorage {
    /// Separate columns for each metric type
    lcp_column: Column<f64>,
    cls_column: Column<f64>,
    ttfb_column: Column<f64>,
    timestamp_column: Column<i64>,
    url_column: Column<String>,
}

struct Column<T> {
    values: Vec<T>,
    compression: CompressionType,
}

impl ColumnarMetricsStorage {
    pub fn new() -> Self {
        Self {
            lcp_column: Column::new(CompressionType::Gzip),
            cls_column: Column::new(CompressionType::Gzip),
            ttfb_column: Column::new(CompressionType::Gzip),
            timestamp_column: Column::new(CompressionType::Delta),
            url_column: Column::new(CompressionType::Dictionary),
        }
    }

    pub fn append(&mut self, metrics: &MetricsSummary, url: &str, timestamp: i64) {
        self.lcp_column.push(metrics.lcp.unwrap_or(0.0));
        self.cls_column.push(metrics.cls.unwrap_or(0.0));
        self.ttfb_column.push(metrics.ttfb.unwrap_or(0.0));
        self.timestamp_column.push(timestamp);
        self.url_column.push(url.to_string());
    }

    pub fn flush(&mut self) -> Result<Vec<u8>, SerializeError> {
        // Compress and serialize all columns
        let mut buffer = Vec::new();

        buffer.extend(self.lcp_column.compress()?);
        buffer.extend(self.cls_column.compress()?);
        buffer.extend(self.ttfb_column.compress()?);
        buffer.extend(self.timestamp_column.compress()?);
        buffer.extend(self.url_column.compress()?);

        Ok(buffer)
    }

    /// Query specific column (efficient for analytics)
    pub fn query_lcp_range(&self, min: f64, max: f64) -> Vec<usize> {
        let mut indices = Vec::new();

        for (i, value) in self.lcp_column.values.iter().enumerate() {
            if *value >= min && *value <= max {
                indices.push(i);
            }
        }

        indices
    }
}

#[derive(Debug, Clone, Copy)]
enum CompressionType {
    None,
    Gzip,
    Delta,
    Dictionary,
}

impl<T: Clone> Column<T> {
    fn new(compression: CompressionType) -> Self {
        Self {
            values: Vec::new(),
            compression,
        }
    }

    fn push(&mut self, value: T) {
        self.values.push(value);
    }

    fn compress(&self) -> Result<Vec<u8>, SerializeError> {
        // Simple compression implementation
        match self.compression {
            CompressionType::Gzip => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                use std::io::Write;

                let bytes = bincode::serialize(&self.values)?;
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&bytes)?;
                Ok(encoder.finish()?)
            }
            _ => bincode::serialize(&self.values),
        }
    }
}
```

---

## Serving Infrastructure

### HTTP API Server

```rust
use axum::{
    extract::{Path, State, Json, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

/// Application state
#[derive(Clone)]
pub struct AppState {
    storage: Arc<dyn TestStorage>,
    index: Arc<TestResultIndex>,
    cache: Arc<ResultCache>,
}

/// API router
pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/tests", post(create_test))
        .route("/api/tests", get(list_tests))
        .route("/api/tests/:id", get(get_test))
        .route("/api/tests/:id/results", get(get_test_results))
        .route("/api/tests/:id", delete(delete_test))
        .route("/api/search", post(search_tests))
        .route("/api/metrics", get(get_metrics))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Request/Response types
#[derive(Debug, Deserialize)]
pub struct CreateTestRequest {
    url: String,
    browser: Option<String>,
    connection_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTestResponse {
    test_id: String,
    status: String,
}

#[derive(Debug, Deserialize)]
pub struct ListTestsQuery {
    limit: Option<usize>,
    offset: Option<usize>,
    browser: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListTestsResponse {
    tests: Vec<TestSummary>,
    total: usize,
}

#[derive(Debug, Serialize)]
pub struct TestSummary {
    test_id: String,
    url: String,
    browser: String,
    timestamp: i64,
    status: String,
    lcp_ms: Option<f64>,
}

/// API Handlers
async fn create_test(
    State(state): State<AppState>,
    Json(req): Json<CreateTestRequest>,
) -> impl IntoResponse {
    // Validate request
    if req.url.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "URL is required"
        })));
    }

    // Create test options
    let options = LaunchOptions {
        url: req.url,
        browser: req.browser.and_then(|b| b.parse().ok()).unwrap_or(BrowserName::Chrome),
        connection_type: req.connection_type.and_then(|c| c.parse().ok()),
        ..Default::default()
    };

    // Launch test asynchronously (would use job queue in production)
    let test_id = generate_test_id();

    tokio::spawn(async move {
        let _ = launch_test(options).await;
    });

    (StatusCode::ACCEPTED, Json(CreateTestResponse {
        test_id,
        status: "queued".to_string(),
    }))
}

async fn list_tests(
    State(state): State<AppState>,
    Query(query): Query<ListTestsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    // Query index
    let mut tests = Vec::new();

    // Apply filters
    // ... (filtering logic)

    (StatusCode::OK, Json(ListTestsResponse {
        tests,
        total: tests.len(),
    }))
}

async fn get_test(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> impl IntoResponse {
    // Get test from index
    // Return 404 if not found
    todo!()
}

async fn get_test_results(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> impl IntoResponse {
    // Fetch result files from storage/cache
    todo!()
}

async fn delete_test(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> impl IntoResponse {
    // Delete test results
    todo!()
}

async fn search_tests(
    State(state): State<AppState>,
    Json(query): Json<SearchQuery>,
) -> impl IntoResponse {
    let results = state.index.search(&query);

    (StatusCode::OK, Json(results))
}

async fn get_metrics(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let cache_stats = state.cache.get_stats();
    let index_stats = state.index.get_stats();

    (StatusCode::OK, Json(serde_json::json!({
        "cache": {
            "l1_hit_rate": cache_stats.l1_hit_rate(),
            "overall_hit_rate": cache_stats.overall_hit_rate(),
        },
        "index": {
            "total_tests": index_stats.total_tests,
            "indexed_today": index_stats.indexed_today,
        }
    })))
}
```

---

## Monitoring and Observability

### Metrics Collection

```rust
use metrics::{counter, gauge, histogram};
use std::time::Instant;

/// Metrics for test execution
pub struct TestMetrics {
    start_time: Instant,
    test_id: String,
}

impl TestMetrics {
    pub fn new(test_id: &str) -> Self {
        // Record test start
        counter!("tests.started").increment(1);

        Self {
            start_time: Instant::now(),
            test_id: test_id.to_string(),
        }
    }

    pub fn record_navigation(&self, duration: Duration) {
        histogram!("test_navigation_duration_ms")
            .record(duration.as_millis() as f64);
    }

    pub fn record_metrics_collection(&self, duration: Duration) {
        histogram!("test_metrics_collection_duration_ms")
            .record(duration.as_millis() as f64);
    }

    pub fn record_post_processing(&self, duration: Duration) {
        histogram!("test_post_processing_duration_ms")
            .record(duration.as_millis() as f64);
    }

    pub fn finish(&self, success: bool) {
        let total_duration = self.start_time.elapsed();

        histogram!("test_total_duration_ms")
            .record(total_duration.as_millis() as f64);

        if success {
            counter!("tests.completed.success").increment(1);
        } else {
            counter!("tests.completed.failure").increment(1);
        }
    }
}

/// Performance metrics for LCP, CLS, etc.
pub fn record_web_vitals(metrics: &MetricsSummary) {
    if let Some(lcp) = metrics.lcp {
        histogram!("web_vitals.lcp_ms").record(lcp);

        // Track good/poor LCP
        if lcp <= 2500.0 {
            counter!("web_vitals.lcp.good").increment(1);
        } else if lcp <= 4000.0 {
            counter!("web_vitals.lcp.needs_improvement").increment(1);
        } else {
            counter!("web_vitals.lcp.poor").increment(1);
        }
    }

    if let Some(cls) = metrics.cls {
        histogram!("web_vitals.cls").record(cls);

        if cls <= 0.1 {
            counter!("web_vitals.cls.good").increment(1);
        } else if cls <= 0.25 {
            counter!("web_vitals.cls.needs_improvement").increment(1);
        } else {
            counter!("web_vitals.cls.poor").increment(1);
        }
    }

    if let Some(fid) = metrics.fid {
        histogram!("web_vitals.fid_ms").record(fid);
    }
}

/// Storage metrics
pub struct StorageMetrics;

impl StorageMetrics {
    pub fn record_write(path: &str, size: usize, duration: Duration) {
        counter!("storage.writes.total").increment(1);
        histogram!("storage.write_duration_ms").record(duration.as_millis() as f64);
        histogram!("storage.write_size_bytes").record(size as f64);

        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");
        counter!("storage.writes.by_extension", "extension" => extension).increment(1);
    }

    pub fn record_read(path: &str, size: usize, duration: Duration, cache_hit: bool) {
        counter!("storage.reads.total").increment(1);
        histogram!("storage.read_duration_ms").record(duration.as_millis() as f64);
        histogram!("storage.read_size_bytes").record(size as f64);

        if cache_hit {
            counter!("storage.reads.cache_hit").increment(1);
        } else {
            counter!("storage.reads.cache_miss").increment(1);
        }
    }
}
```

### Distributed Tracing

```rust
use tracing::{info, warn, error, instrument, Span};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;

/// Initialize tracing
pub fn init_tracing(service_name: &str) -> Result<(), TracingError> {
    let otlp_exporter = opentelemetry_otlp::new_exporter().tonic();
    let trace_config = opentelemetry_sdk::trace::config()
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name),
        ]));

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(otlp_exporter)
        .with_trace_config(trace_config)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    let telemetry = OpenTelemetryLayer::new(tracer);

    let subscriber = Registry::default()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

/// Instrumented test launch
#[instrument(name = "test.launch", skip(options), fields(test_id, url = %options.url))]
pub async fn launch_test_instrumented(options: LaunchOptions) -> Result<TestResult, TestError> {
    info!("Launching test");

    let result = launch_test(options).await;

    match &result {
        Ok(TestResult::Success { test_id, .. }) => {
            Span::current().record("test_id", test_id);
            info!("Test completed successfully");
        }
        Ok(TestResult::Failure { error }) => {
            warn!(error = %error, "Test failed");
        }
        Err(e) => {
            error!(error = %e, "Test error");
        }
    }

    result
}

/// Instrumented storage operations
#[instrument(name = "storage.write", skip(data), fields(path = %path.display(), size = data.len()))]
pub async fn write_with_tracing(
    storage: &Arc<dyn TestStorage>,
    path: &Path,
    data: &[u8],
) -> Result<(), StorageError> {
    let start = Instant::now();
    let result = storage.write(path, data).await;
    let duration = start.elapsed();

    if result.is_ok() {
        info!(duration_ms = duration.as_millis(), "Write completed");
    } else {
        error!(error = ?result, "Write failed");
    }

    result
}
```

---

## High Availability

### Health Checks

```rust
use axum::http::StatusCode;
use tokio::time::timeout;

/// Health check status
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// Health checker
pub struct HealthChecker {
    storage: Arc<dyn TestStorage>,
    index: Arc<TestResultIndex>,
    cache: Arc<ResultCache>,
}

impl HealthChecker {
    pub fn new(storage: Arc<dyn TestStorage>, index: Arc<TestResultIndex>, cache: Arc<ResultCache>) -> Self {
        Self { storage, index, cache }
    }

    pub async fn health(&self) -> HealthStatus {
        let mut checks = Vec::new();
        let mut overall_healthy = true;

        // Check storage
        let storage_check = self.check_storage().await;
        if storage_check.status != "healthy" {
            overall_healthy = false;
        }
        checks.push(storage_check);

        // Check index
        let index_check = self.check_index().await;
        if index_check.status != "healthy" {
            overall_healthy = false;
        }
        checks.push(index_check);

        // Check cache
        let cache_check = self.check_cache().await;
        checks.push(cache_check);  // Cache unhealthy is not critical

        HealthStatus {
            status: if overall_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
            checks,
        }
    }

    async fn check_storage(&self) -> HealthCheck {
        let start = Instant::now();

        // Try to write and read a health check file
        let test_path = Path::new(".health_check");
        let test_data = b"health";

        let result = timeout(Duration::from_secs(5), async {
            self.storage.write(test_path, test_data).await?;
            self.storage.read(test_path).await
        }).await;

        match result {
            Ok(Ok(_)) => HealthCheck {
                name: "storage".to_string(),
                status: "healthy".to_string(),
                latency_ms: Some(start.elapsed().as_millis() as u64),
                error: None,
            },
            Ok(Err(e)) => HealthCheck {
                name: "storage".to_string(),
                status: "unhealthy".to_string(),
                latency_ms: None,
                error: Some(e.to_string()),
            },
            Err(_) => HealthCheck {
                name: "storage".to_string(),
                status: "timeout".to_string(),
                latency_ms: None,
                error: Some("Health check timed out".to_string()),
            },
        }
    }

    async fn check_index(&self) -> HealthCheck {
        let start = Instant::now();

        // Try a simple index query
        let result = timeout(Duration::from_secs(5), async {
            self.index.search(&SearchQuery::TextSearch(String::new()))
        }).await;

        match result {
            Ok(_) => HealthCheck {
                name: "index".to_string(),
                status: "healthy".to_string(),
                latency_ms: Some(start.elapsed().as_millis() as u64),
                error: None,
            },
            Err(_) => HealthCheck {
                name: "index".to_string(),
                status: "timeout".to_string(),
                latency_ms: None,
                error: Some("Health check timed out".to_string()),
            },
        }
    }

    async fn check_cache(&self) -> HealthCheck {
        // Cache check is best-effort
        HealthCheck {
            name: "cache".to_string(),
            status: "healthy".to_string(),
            latency_ms: None,
            error: None,
        }
    }
}

/// Readiness probe (is ready to accept traffic?)
pub async fn readiness(state: Arc<AppState>) -> StatusCode {
    let checker = HealthChecker::new(
        state.storage.clone(),
        state.index.clone(),
        state.cache.clone(),
    );

    let health = checker.health().await;

    if health.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

/// Liveness probe (is the process alive?)
pub async fn liveness() -> StatusCode {
    StatusCode::OK
}
```

---

## Security Considerations

### Input Validation

```rust
use validator::{Validate, ValidationError};

/// Validated test request
#[derive(Debug, Validate, Deserialize)]
pub struct ValidatedTestRequest {
    #[validate(url, length(max = 2048))]
    pub url: String,

    #[validate(custom = "validate_browser")]
    pub browser: Option<String>,

    #[validate(range(min = 100, max = 300000))]
    pub timeout_ms: Option<u64>,

    #[validate(length(max = 100, each(length(max = 256))))]
    pub block_domains: Option<Vec<String>>,
}

fn validate_browser(browser: &str) -> Result<(), ValidationError> {
    match browser.parse::<BrowserName>() {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidationError::new("invalid_browser")),
    }
}

/// Path sanitization
pub fn sanitize_path(path: &str) -> Result<PathBuf, SecurityError> {
    // Prevent path traversal
    if path.contains("..") {
        return Err(SecurityError::PathTraversal);
    }

    // Ensure path is within allowed base
    let base = Path::new("./results");
    let full_path = base.join(path);

    // Canonicalize to resolve symlinks
    let canonical = full_path.canonicalize()
        .unwrap_or_else(|_| full_path.clone());

    // Verify path is within base
    if !canonical.starts_with(base) {
        return Err(SecurityError::PathOutsideBase);
    }

    Ok(canonical)
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Path traversal attempt detected")]
    PathTraversal,
    #[error("Path outside allowed base directory")]
    PathOutsideBase,
    #[error("Invalid character in path")]
    InvalidCharacter,
}
```

### Rate Limiting

```rust
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;

/// Rate-limited API middleware
pub struct RateLimitMiddleware {
    limiter: RateLimiter<quanta::Instant>,
}

impl RateLimitMiddleware {
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        let limiter = RateLimiter::direct(quota);

        Self { limiter }
    }

    pub async fn check(&self, key: &str) -> Result<(), RateLimitError> {
        if self.limiter.check_key(governor::Key::from(key)).is_ok() {
            Ok(())
        } else {
            Err(RateLimitError::Exceeded)
        }
    }
}

/// Per-user rate limiting
pub struct PerUserRateLimiter {
    limiters: DashMap<String, RateLimiter<quanta::Instant>>,
    quota: Quota,
}

impl PerUserRateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            limiters: DashMap::new(),
            quota: Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap()),
        }
    }

    pub fn check(&self, user_id: &str) -> Result<(), RateLimitError> {
        let limiter = self.limiters
            .entry(user_id.to_string())
            .or_insert_with(|| RateLimiter::direct(self.quota));

        if limiter.check().is_ok() {
            Ok(())
        } else {
            Err(RateLimitError::Exceeded)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded")]
    Exceeded,
}
```

---

## Summary

| Topic | Key Points |
|-------|------------|
| Performance | Async I/O, connection pooling, multi-level caching |
| Memory | Bounded processing, zero-copy deserialization |
| Batching | Batch test execution, result batching for storage |
| Serialization | Binary serialization, columnar storage |
| Serving | HTTP API with axum, CORS, tracing |
| Monitoring | Metrics, distributed tracing, health checks |
| Security | Input validation, rate limiting, path sanitization |

---

## Next Steps

Continue to [05-valtron-integration.md](05-valtron-integration.md) for Lambda deployment patterns for serverless FS backend.
