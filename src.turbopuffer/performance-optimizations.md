# Performance Optimizations

## SIMD, Caching, and Latency Optimization in Vector Search

This document analyzes performance optimization techniques used in Turbopuffer and similar vector search systems.

---

## Table of Contents

1. [Understanding the Bottleneck](#understanding-the-bottleneck)
2. [SIMD Optimization](#simd-optimization)
3. [Caching Strategies](#caching-strategies)
4. [Batch Operations](#batch-operations)
5. [Latency Optimizations](#latency-optimizations)
6. [Benchmarking](#benchmarking)
7. [Implementation Guide](#implementation-guide)

---

## Understanding the Bottleneck

### Bandwidth-Bound vs. Compute-Bound

**Key Insight:** Vector search transitions from bandwidth-bound to compute-bound after optimization.

```
Stage 1: Naive Implementation
├── Full-precision vectors (f32)
├── Linear scan through all vectors
└── Bottleneck: Memory bandwidth

    Operations: N × D FLOPs
    Memory: N × D × 4 bytes
    Intensity: 0.25 FLOPs/byte (VERY LOW)
    Result: Bandwidth-bound

Stage 2: With Indexing (IVF/HNSW)
├── Search only candidate subset
├── Bottleneck: Still bandwidth (but less data)

    Operations: (N/100) × D FLOPs
    Memory: (N/100) × D × 4 bytes
    Result: Bandwidth-bound (100x faster)

Stage 3: With Quantization
├── Binary vectors (1 bit per dimension)
├── 32x compression
└── Bottleneck: Compute!

    Operations: (N/100) × D FLOPs (same)
    Memory: (N/100) × D × 0.5 bytes (8x less)
    Intensity: 2 FLOPs/byte (8x higher)
    Result: Compute-bound
```

### Identifying Your Bottleneck

**Simple Test:**
```python
import time

def benchmark_search():
    # Warm cache first
    for _ in range(10):
        search(query)

    # Measure
    start = time.perf_counter()
    for _ in range(100):
        search(query)
    elapsed = time.perf_counter() - start

    # Calculate throughput
    qps = 100 / elapsed

    # Compare to theoretical limits
    bandwidth_used = qps * vectors_per_query * bytes_per_vector
    if bandwidth_used > measured_memory_bandwidth * 0.8:
        print("Bandwidth-bound")
    else:
        print("Compute-bound")
```

---

## SIMD Optimization

### What is SIMD?

**SIMD (Single Instruction, Multiple Data)** allows processing multiple data elements with a single instruction.

**Example: Dot Product**
```
Scalar (no SIMD):
for i in range(4):
    sum += a[i] * b[i]
# 4 multiply instructions, 4 add instructions

SIMD (SSE, 128-bit):
sum = dot_product_simd(a[0:4], b[0:4])
# 1 instruction does all 4 multiplications + adds

SIMD (AVX2, 256-bit):
sum = dot_product_simd(a[0:8], b[0:8])
# 1 instruction does 8 floats at once

SIMD (AVX-512, 512-bit):
sum = dot_product_simd(a[0:16], b[0:16])
# 1 instruction does 16 floats at once
```

### SIMD for Distance Computations

**Cosine Distance with AVX2:**
```rust
use std::arch::x86_64::*;

unsafe fn cosine_distance_simd(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len();
    let simd_len = len / 8 * 8;  // Process 8 floats at a time

    let mut dot = _mm256_setzero_ps();
    let mut norm_a = _mm256_setzero_ps();
    let mut norm_b = _mm256_setzero_ps();

    // SIMD loop
    for i in (0..simd_len).step_by(8) {
        let va = _mm256_loadu_ps(a.as_ptr().add(i));
        let vb = _mm256_loadu_ps(b.as_ptr().add(i));

        dot = _mm256_fmadd_ps(va, vb, dot);      // dot += a * b
        norm_a = _mm256_fmadd_ps(va, va, norm_a); // norm_a += a * a
        norm_b = _mm256_fmadd_ps(vb, vb, norm_b); // norm_b += b * b
    }

    // Horizontal sum
    let dot_sum = horizontal_sum(dot);
    let norm_a_sum = horizontal_sum(norm_a);
    let norm_b_sum = horizontal_sum(norm_b);

    // Cosine similarity
    dot_sum / (norm_a_sum * norm_b_sum).sqrt()
}
```

### Binary Quantization + SIMD

**Hamming Distance with AVX2:**
```rust
use std::arch::x86_64::*;

/// Compute Hamming distance between packed bit vectors
unsafe fn hamming_distance_simd(a: &[u8], b: &[u8]) -> usize {
    let len = a.len();
    let simd_len = len / 32 * 32;  // Process 32 bytes at a time

    let mut count = _mm256_setzero_si256();

    for i in (0..simd_len).step_by(32) {
        // Load 32 bytes each
        let va = _mm256_loadu_si256(a.as_ptr().add(i) as *const __m256i);
        let vb = _mm256_loadu_si256(b.as_ptr().add(i) as *const __m256i);

        // XOR to find differing bits
        let xor = _mm256_xor_si256(va, vb);

        // Count set bits (population count)
        let lo = _mm256_extracti128_si256(xor, 0);
        let hi = _mm256_extracti128_si256(xor, 1);

        count = _mm256_add_epi64(
            count,
            _mm256_set_epi64x(
                popcount_u64(_mm256_extract_epi64(hi, 1)) as i64,
                popcount_u64(_mm256_extract_epi64(hi, 0)) as i64,
                popcount_u64(_mm256_extract_epi64(lo, 1)) as i64,
                popcount_u64(_mm256_extract_epi64(lo, 0)) as i64,
            ),
        );
    }

    horizontal_sum(count) as usize
}
```

### SIMD Performance Gains

| Operation | Scalar | SSE (4x) | AVX2 (8x) | AVX-512 (16x) |
|-----------|--------|----------|-----------|---------------|
| Dot Product (f32) | 1.0x | 3.5x | 6.8x | 12x |
| L2 Distance (f32) | 1.0x | 3.2x | 6.2x | 11x |
| Hamming (binary) | 1.0x | 8x | 16x | 32x |

**Note:** Gains vary by CPU and memory alignment.

---

## Caching Strategies

### Multi-Level Caching Architecture

```
Query Flow with Caching:
┌─────────────────────────────────────────────────────────────────┐
│                         QUERY                                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Level 1: L3 Cache (~32 MB)                                      │
│ - Hot centroid vectors                                          │
│ - Frequently accessed quantized vectors                         │
│ Hit rate: ~80% for repeated queries                            │
└─────────────────────────────────────────────────────────────────┘
                              │ (miss)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Level 2: DRAM (~128 GB)                                         │
│ - All quantized vectors                                         │
│ - All centroids                                                 │
│ - Recently accessed full-precision vectors                      │
│ Hit rate: ~95% for warm queries                                 │
└─────────────────────────────────────────────────────────────────┘
                              │ (miss)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Level 3: SSD Cache (~10 TB)                                     │
│ - All full-precision vectors                                    │
│ - Attribute data                                                │
│ Hit rate: ~99% for indexed data                                 │
└─────────────────────────────────────────────────────────────────┘
                              │ (miss)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Level 4: Object Storage (unlimited)                             │
│ - Cold data backup                                              │
│ - Namespace snapshots                                           │
└─────────────────────────────────────────────────────────────────┘
```

### LRU Cache Implementation

```rust
use std::collections::{HashMap, VecDeque};

/// Simple LRU cache for vector data
struct VectorCache<K, V> {
    map: HashMap<K, V>,
    queue: VecDeque<K>,
    capacity: usize,
}

impl<K: Eq + std::hash::Hash + Clone, V> VectorCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            queue: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Move to front (most recently used)
            self.queue.retain(|k| k != key);
            self.queue.push_front(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    fn insert(&mut self, key: K, value: V) {
        if self.map.len() >= self.capacity {
            // Evict least recently used
            if let Some(oldest) = self.queue.pop_back() {
                self.map.remove(&oldest);
            }
        }
        self.queue.push_front(key.clone());
        self.map.insert(key, value);
    }
}
```

### Cache-Aware Data Layout

```rust
/// Struct of Arrays (SoA) layout for better SIMD + cache performance
struct VectorStoreSoA {
    /// All dimension 0 values contiguous
    dim_0: Vec<f32>,
    /// All dimension 1 values contiguous
    dim_1: Vec<f32>,
    // ...
    /// Or for packed SIMD: [v0_d0,v0_d1,v0_d2,v0_d3, v1_d0,v1_d1,...]
    packed: Vec<f32>,
}

impl VectorStoreSoA {
    /// Load 8 vectors' dimension 0-3 into SIMD register
    fn load_8_vectors(&self, base_idx: usize) -> __m256 {
        unsafe {
            // Contiguous access = prefetcher friendly
            _mm256_loadu_ps(self.packed.as_ptr().add(base_idx * 4))
        }
    }
}

/// Array of Structs (AoS) - worse for SIMD, better for single vector access
struct VectorStoreAoS {
    vectors: Vec<[f32; 1024]>,
}
```

### Query Result Caching

```rust
use moka::sync::Cache;

/// Cache query results for repeated queries
struct QueryResultCache {
    cache: Cache<Vec<f32>, Vec<SearchResult>>,
}

impl QueryResultCache {
    fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    fn get_or_compute<F>(&self, query: &[f32], compute: F) -> Vec<SearchResult>
    where
        F: FnOnce() -> Vec<SearchResult>,
    {
        // Note: This is simplified. Real implementation would need
        // approximate matching for similar (not identical) queries
        self.cache.get(query).unwrap_or_else(|| {
            let result = compute();
            self.cache.insert(query.to_vec(), result.clone());
            result
        })
    }
}
```

---

## Batch Operations

### Upsert Batching

```python
# Bad: One request per vector
for vector in vectors:
    client.namespace("products").write(
        upsert_rows=[{"id": ..., "vector": vector}]
    )
# N API calls, high latency

# Good: Batch upserts
client.namespace("products").write(
    upsert_rows=[
        {"id": id, "vector": vector}
        for id, vector in zip(ids, vectors)
    ]
)
# 1 API call, amortized latency
```

### Query Batching (Parallel)

```rust
use tokio::task::JoinSet;

/// Process multiple queries in parallel
async fn batch_query(
    engine: &QueryEngine,
    queries: Vec<Vec<f32>>,
    concurrency: usize,
) -> Vec<Vec<SearchResult>> {
    let mut set = JoinSet::new();
    let mut results = Vec::with_capacity(queries.len());

    for (i, query) in queries.into_iter().enumerate() {
        if set.len() >= concurrency {
            // Wait for one to complete
            if let Some(result) = set.join_next().await {
                results.push(result.unwrap());
            }
        }
        set.spawn(async move {
            engine.query(query, 10).await
        });
    }

    // Collect remaining
    while let Some(result) = set.join_next().await {
        results.push(result.unwrap());
    }

    results
}
```

### Prefetching

```rust
use std::arch::x86_64::_mm_prefetch;
use std::arch::x86_64::_MM_HINT_T0;

/// Prefetch next cluster while processing current
fn search_with_prefetch(
    clusters: &[Cluster],
    query: &[f32],
) -> Vec<SearchResult> {
    let mut results = Vec::new();

    for i in 0..clusters.len() {
        // Prefetch next cluster
        if i + 1 < clusters.len() {
            unsafe {
                _mm_prefetch(
                    clusters[i + 1].data.as_ptr() as *const i8,
                    _MM_HINT_T0,
                );
            }
        }

        // Process current cluster (data already prefetched)
        results.extend(process_cluster(&clusters[i], query));
    }

    results
}
```

---

## Latency Optimizations

### Reducing Tail Latency

**Problem:** p99 latency can be 10x p50 due to outliers.

**Solutions:**

**1. Query Timeout:**
```rust
use tokio::time::{timeout, Duration};

async fn query_with_timeout(
    engine: &QueryEngine,
    query: Vec<f32>,
) -> Result<Vec<SearchResult>, TimeoutError> {
    timeout(Duration::from_millis(100), engine.query(query))
        .await
        .map_err(|_| TimeoutError)?
}
```

**2. Hedged Requests:**
```rust
/// Send duplicate request if first takes too long
async fn hedged_query(
    engine: &QueryEngine,
    query: Vec<f32>,
    hedge_delay: Duration,
) -> Vec<SearchResult> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    // First request
    let engine1 = engine.clone();
    let query1 = query.clone();
    tokio::spawn(async move {
        let result = engine1.query(query1).await;
        let _ = tx.send(result);
    });

    // Hedge after delay
    tokio::time::sleep(hedge_delay).await;

    let engine2 = engine.clone();
    tokio::spawn(async move {
        engine2.query(query).await
    });

    // Return first to complete
    rx.await.unwrap()
}
```

**3. Load Shedding:**
```rust
use tokio::sync::Semaphore;

struct LoadShedder {
    semaphore: Semaphore,
    max_concurrent: usize,
}

impl LoadShedder {
    async fn query(&self, engine: &QueryEngine, query: Vec<f32>)
        -> Result<Vec<SearchResult>, Rejected>
    {
        let permit = self.semaphore.try_acquire()
            .map_err(|_| Rejected)?;

        let result = engine.query(query).await;
        drop(permit);  // Release permit
        Ok(result)
    }
}
```

### Warm Start Optimization

```rust
/// Keep index warm by preloading hot data
struct WarmStart {
    cache: VectorCache<u64, Vec<f32>>,
    hot_vectors: Vec<u64>,  // IDs of frequently accessed vectors
}

impl WarmStart {
    async fn warm_cache(&mut self, store: &VectorStore) {
        for &id in &self.hot_vectors {
            if let Some(vector) = store.get(id).await {
                self.cache.insert(id, vector);
            }
        }
    }

    fn query(&mut self, query: &[f32]) -> Vec<SearchResult> {
        // Check cache first for hot vectors
        // ...
    }
}
```

---

## Benchmarking

### tpuf-benchmark Tool

The `tpuf-benchmark` tool in the source directory provides comprehensive benchmarking:

```bash
# Build
cd tpuf-benchmark
go build -o tpuf-benchmark

# Run benchmark
./tpuf-benchmark \
    -api-key $TURBOPUFFER_API_KEY \
    -endpoint https://gcp-us-central1.turbopuffer.com \
    -namespace-count 1 \
    -namespace-combined-size 1000000 \
    -queries-per-sec 10 \
    -query-template templates/query_default.json.tmpl
```

### Custom Benchmarking

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_search(c: &mut Criterion) {
    let store = VectorStore::load("test_data").unwrap();
    let query = vec![0.1; 1024];

    c.bench_function("search_1m_vectors", |b| {
        b.iter(|| {
            black_box(search(&store, black_box(&query), 10))
        })
    });
}

fn bench_distance(c: &mut Criterion) {
    let a = vec![0.1; 1024];
    let b = vec![0.2; 1024];

    c.bench_function("cosine_distance_scalar", |b| {
        b.iter(|| cosine_distance(black_box(&a), black_box(&b)))
    });

    c.bench_function("cosine_distance_simd", |b| {
        b.iter(|| cosine_distance_simd(black_box(&a), black_box(&b)))
    });
}

criterion_group!(benches, bench_search, bench_distance);
criterion_main!(benches);
```

### Profiling

```bash
# CPU profiling with perf
perf record -g ./target/release/vector_search
perf report

# Memory profiling with heaptrack
heaptrack ./target/release/vector_search
heaptrack_gui heaptrack.output

# Flamegraph generation
cargo flamegraph --example search_benchmark
```

---

## Implementation Guide

### Rust Crates for Optimization

```toml
[dependencies]
# SIMD
stdsimd = "0.1"          # Portable SIMD (nightly)
portable-simd = "0.1"    # Portable SIMD
simba = "0.9"            # SIMD linear algebra

# Caching
moka = "0.12"            # High-performance cache
dashmap = "6.0"          # Concurrent HashMap

# Memory management
memmap2 = "0.9"          # Memory mapping
page_size = "0.6"        # Page size detection

# Benchmarking
criterion = "0.5"        # Micro-benchmarking
iai = "0.1"              # Instruction counting

# Profiling
profiling = "1.0"        # Profiling macros
```

### SIMD Best Practices

```rust
// 1. Use safe abstractions when possible
use ndarray::Array1;

fn dot_product(a: &Array1<f32>, b: &Array1<f32>) -> f32 {
    a.dot(b)  // ndarray uses SIMD internally
}

// 2. Align data for optimal loading
#[repr(align(32))]  // 256-bit alignment for AVX2
struct AlignedVector {
    data: [f32; 1024],
}

// 3. Process multiple items together
fn batch_dot_products(pairs: &[(Vec<f32>, Vec<f32>)]) -> Vec<f32> {
    // Process 4 pairs at a time with SIMD
    // ...
}
```

### Cache Optimization Checklist

- [ ] Use contiguous memory layout (Vec, not HashMap)
- [ ] Align data structures to cache lines (64 bytes)
- [ ] Use Struct of Arrays for SIMD operations
- [ ] Prefetch data before needed
- [ ] Keep hot data in L3 cache (<32 MB)
- [ ] Use appropriate data types (f16 vs f32)
- [ ] Profile cache miss rates

---

## Summary

**Performance Optimization Hierarchy:**

1. **Algorithm Level:**
   - Use ANN instead of exact search (100-1000x)
   - Hierarchical indexing (10-100x)
   - Quantization (16-32x)

2. **System Level:**
   - Cache hot data (2-10x)
   - Batch operations (5-10x)
   - Parallel queries (2-8x on multi-core)

3. **Hardware Level:**
   - SIMD vectorization (4-32x)
   - Memory alignment (1.5-2x)
   - Prefetching (1.2-2x)

**Key Principle:** Optimize the bottleneck. Measuring is essential—don't guess!
