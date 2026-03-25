# Turbopuffer: Comprehensive Exploration

## Executive Summary

Turbopuffer is a high-performance vector search engine designed for billion-scale approximate nearest neighbor (ANN) search. It achieves remarkable throughput (10,000+ QPS) through sophisticated memory hierarchy optimization, combining hierarchical clustering with binary quantization.

**Key Innovation:** The system is architected around the memory hierarchy rather than fighting against it. By understanding bandwidth constraints at each level (CPU registers → L3 cache → DRAM → NVMe SSD → Object Storage), Turbopuffer strategically places data to maximize throughput.

## Table of Contents

1. [Storage Engine Deep Dive](#storage-engine-deep-dive)
2. [Search Algorithms](#search-algorithms)
3. [SDK Comparison](#sdk-comparison)
4. [Performance Optimizations](#performance-optimizations)
5. [Blog Analysis](./blog/blog-index.md)
6. [Rust Replication Plan](#rust-replication-plan)
7. [Production-Grade Considerations](#production-grade-considerations)
8. [Resilient Storage Guide](#resilient-storage-guide)

---

## Storage Engine Deep Dive

### Memory Hierarchy Architecture

Turbopuffer's storage engine is built around a fundamental insight: **vector search is bandwidth-bound, not compute-bound**. The arithmetic intensity is low because each element in a data vector is used only once by the distance function.

```
Memory Hierarchy (fastest to slowest):
┌─────────────────┬───────────────────┬────────────────────┐
│ Level           │ Size              │ Bandwidth          │
├─────────────────┼───────────────────┼────────────────────┤
│ CPU Registers   │ < 1 KB            │ > 10 TB/s          │
│ L1/L2/L3 Cache  │ KBs - MBs         │ 1-10+ TB/s         │
│ DRAM            │ GBs - TBs         │ 100-500 GB/s       │
│ NVMe SSD        │ TBs - 10s TBs     │ 1-30 GB/s          │
│ Object Storage  │ PBs - EBs         │ 1-10 GB/s          │
└─────────────────┴───────────────────┴────────────────────┘
```

### Data Layout Strategy

**Full-Precision + Quantized Storage:**
- Every vector is stored in TWO forms:
  1. Full precision (f32 or f16 per dimension) - stored on NVMe SSD
  2. Binary quantized (1-bit per dimension) - stored in DRAM/L3 cache

This dual-storage approach enables:
- **16-32x compression** for data vectors
- Fast initial filtering using quantized vectors
- Precise reranking using full-precision vectors (only for <1% of candidates)

### File Format Structure

Based on analysis of the benchmark templates and API patterns:

```
Namespace Structure:
├── Tree Metadata (centroids at each level)
│   ├── Root centroid (always in memory)
│   ├── Level 1 centroids (~100 vectors, in DRAM)
│   └── Level 2 centroids (~10,000 vectors, in DRAM)
├── Quantized Data Vectors (1-bit per dimension)
│   └── Stored in DRAM for fast access
├── Full-Precision Data Vectors (f16/f32 per dimension)
│   └── Stored on NVMe SSD, fetched via scatter-gather
└── Attribute Index
    └── For filtering and metadata retrieval
```

### Compression Techniques

**Binary Quantization (RaBitQ):**
- Exploits "concentration of measure" in high-dimensional space
- Provides tight error bounds on distance estimates
- Enables filtering in quantized space before fetching full precision

**How it works:**
```
Original vector:  [0.94, -0.01, 0.39, -0.72, 0.02, -0.85, -0.18, 0.99]
Quantized:        [1,     0,    1,    0,    1,    0,    0,    1   ]

Distance estimation with error bounds:
- Quantized distance: d' = HammingDistance(vq, vd)
- Error bound: |d - d'| <= ε (theoretical guarantee from RaBitQ)
- Full-precision rerank only for vectors with overlapping confidence intervals
```

---

## Search Algorithms

### ANN v3 Architecture (SPFresh-based)

Turbopuffer uses a centroid-based index inspired by **SPFresh** (a research paper on incremental ANN search), extended with hierarchical clustering.

**Algorithm Overview:**

```
Query Processing Pipeline:
1. Compare query to root centroid
2. Select top-k promising child clusters
3. Recursively descend tree (bounded by tree height)
4. At leaf level, scan quantized vectors in selected clusters
5. Compute distance estimates with error bounds
6. Fetch full-precision vectors only for reranking candidates
7. Return final ranked results
```

### Hierarchical Clustering

**Tree Structure:**
- Branching factor: ~100 (matches DRAM:SSD size ratio)
- Each cluster represented by centroid (mean of vectors in cluster)
- Multi-level tree enables bounded object-storage round trips

**Why Hierarchical Clustering Works:**
1. **Spatial Locality:** Related vectors stored contiguously
2. **Temporal Locality:** Upper levels stay resident in faster memory
3. **Bounded Latency:** Tree height limits cold query round-trips

**Bandwidth Calculation per Tree Level:**
```
500 clusters × (100 vectors/cluster) × 1024 dimensions × 2 bytes/dim = 100MB/level

Without quantization: ~100 QPS (disk-bound at 10 GB/s)
With quantization:    ~10,000 QPS (compute-bound)
```

### Distance Metrics Supported

Based on SDK type definitions:
- `cosine_distance` - Angular similarity (default for normalized vectors)
- `l2_distance` - Euclidean distance
- `dot_product` - Inner product (for unnormalized vectors)

### Query Optimization

**The "Approximation + Refinement" Strategy:**

1. **Approximation Phase (Quantized Space):**
   - Fast distance estimation using binary vectors
   - Error bounds identify all potential top-k candidates
   - Processes ~50,000 QPS worth of data

2. **Refinement Phase (Full Precision):**
   - Scatter-gather fetches <1% of vectors
   - Precise distance computation
   - Final ranking

---

## SDK Comparison

### Architecture Overview

All SDKs are generated with [Stainless](https://www.stainless.com/), ensuring API consistency across languages.

| Feature | Python | TypeScript | Go | Java |
|---------|--------|------------|-----|------|
| Async Support | ✅ httpx | ✅ native | ❌ | ✅ CompletableFuture |
| Type System | TypedDict + Pydantic | TypeScript interfaces | omitzero semantics | Builder pattern |
| HTTP Client | httpx (default) | fetch API | net/http | OkHttp |
| Pagination | Auto-paging iterator | for-await iteration | AutoPaging methods | Iterable/Stream |
| Null Handling | `model_fields_set` | `null` vs omitted | `param.Opt[T]` | `JsonField<T>` |

### API Design Patterns

**Common Patterns Across All SDKs:**

1. **Namespace-based Resource Model:**
```python
# Python
client.namespace("products").write(...)
client.namespace("products").query(...)
```

```typescript
// TypeScript
client.namespace('products').write(...)
```

```go
// Go
namespace := client.Namespace("products")
namespace.Write(ctx, ...)
```

```java
// Java
client.namespaces().write(params)
```

2. **Retry Logic (default 2 retries):**
- Retried: Connection errors, 408, 409, 429, >=500
- Configurable via `max_retries` / `maxRetries`

3. **Timeout Handling:**
- Default: 60 seconds (Python/TS), varies by SDK
- Context-based timeout propagation

### Serialization Formats

**Request Serialization:**
- All SDKs use JSON for request/response bodies
- Go uses `omitzero` semantics (Go 1.24+)
- Java uses Jackson with `JsonField<T>` wrappers
- Python uses Pydantic models

**Response Handling:**
```go
// Go - Check field validity
if res.JSON.Name.Valid() {
    // Field was present in response
}
```

```python
# Python - Check if field was set
if 'my_field' not in response.model_fields_set:
    # Field was missing entirely
```

---

## Performance Optimizations

### SIMD Usage

While the source code for the core engine isn't available in this exploration, the blog posts reveal:

**Compute-Bound Optimization (after quantization):**
- Binary quantization increases arithmetic intensity 64x
- Each bit reused 4 times in RaBitQ distance estimation
- System becomes compute-bound at ~1,000 QPS

**Key Optimizations Needed:**
1. Fewer instructions per operation
2. Avoid pipeline stalls and branch mispredictions
3. Maximize SIMD vectorization for bitwise operations

### Caching Strategies

**Multi-Level Caching:**

1. **L3 Cache (504 MiB typical):**
   - Stores all quantized centroid vectors (Levels 1-3)
   - ~128 MiB for Level 3 (100³ × 128 bytes)
   - Enables ~33,000 QPS theoretical

2. **DRAM:**
   - Stores all quantized data vectors
   - ~50,000 QPS theoretical throughput

3. **SSD Cache:**
   - Full-precision vectors
   - Only 1% fetched per query for reranking

### Batch Operations

**Upsert Batching:**
- Templates support batch upserts via `upsert_rows` array
- Benchmark tool can generate sustained write load
- Recommended: Batch size tuned to stay under rate limits

**Query Batching:**
- Not explicitly supported in API
- Parallel queries recommended for throughput

### Latency Optimizations

**Cold Query Latency:**
- Hierarchical tree bounds object storage round-trips
- Tree height determines worst-case latency

**Warm Query Latency:**
- Quantized search in DRAM
- Scatter-gather for reranking (<1% of vectors)

---

## Rust Replication Plan

### Crate Recommendations

**Core Dependencies:**

```toml
[dependencies]
# ANN Index
hnsw = "0.11"                    # HNSW implementation
spatial = "0.2"                  # Spatial data structures
ndarray = "0.16"                 # N-dimensional arrays
ndarray-linalg = "0.17"          # Linear algebra operations

# Quantization
half = "2.4"                     # f16 support
bitvec = "1.0"                   # Bit-level operations

# Storage
memmap2 = "0.9"                  # Memory-mapped files
rayon = "1.10"                   # Parallel iteration
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rkyv = "0.7"                     # Zero-copy deserialization

# Distance computations
simba = "0.9"                    # SIMD linear algebra
```

### Storage Layer Design

```rust
/// Memory-mapped vector storage
pub struct VectorStore {
    /// Quantized vectors (1-bit per dimension) - kept in memory
    quantized_vectors: BitVecStore,

    /// Full-precision vectors - memory mapped from SSD
    mmap: MemMap,

    /// Centroid tree for hierarchical search
    centroid_tree: CentroidTree,

    /// Metadata and attributes
    attribute_index: AttributeIndex,
}

pub struct BitVecStore {
    /// Packed bit vectors (1024 dimensions = 128 bytes per vector)
    data: Vec<u8>,
    dimension: usize,
    count: usize,
}

pub struct CentroidTree {
    /// Each level contains centroids for the level below
    levels: Vec<Level>,
}

pub struct Level {
    centroids: Vec<Centroid>,
    /// Child indices for each centroid
    children: Vec<ChildRange>,
}
```

### Query Engine Design

```rust
pub struct QueryEngine {
    tree_searcher: TreeSearcher,
    quantized_searcher: QuantizedSearcher,
    reranker: FullPrecisionReranker,
}

impl QueryEngine {
    pub async fn query(&self, query: &[f32], top_k: usize) -> Vec<RankedResult> {
        // Phase 1: Tree traversal to find candidate clusters
        let candidate_clusters = self.tree_searcher.find_candidates(query);

        // Phase 2: Quantized search for initial ranking
        let quantized_candidates = self.quantized_searcher
            .search(query, &candidate_clusters)
            .await;

        // Phase 3: Compute error bounds and identify rerank candidates
        let rerank_ids = self.identify_rerank_candidates(
            &quantized_candidates, top_k
        );

        // Phase 4: Scatter-gather full-precision vectors
        let full_precision = self.reranker
            .fetch_and_rank(&rerank_ids)
            .await;

        full_precision.into_iter().take(top_k).collect()
    }
}
```

### Production-Grade Considerations

**1. Incremental Updates:**
- Use SPFresh's incremental update algorithm
- Maintain graph connections during inserts
- Background compaction for deleted vectors

**2. Fault Tolerance:**
- WAL (Write-Ahead Log) for durability
- Checkpointing for crash recovery
- Replication across zones

**3. Observability:**
- Prometheus metrics (QPS, latency, recall)
- Tracing for query execution paths
- Cache hit/miss ratios

---

## Production-Grade Considerations

### What Production-Ready Looks Like

**1. API Layer:**
- Rate limiting (per API key)
- Request validation
- Graceful degradation under load

**2. Storage Layer:**
- Multi-zone replication
- Automatic failover
- Backup/restore procedures

**3. Query Layer:**
- Query admission control
- Priority queuing
- Circuit breakers

**4. Monitoring:**
- p99 latency tracking
- Recall quality metrics
- Resource utilization dashboards

---

## Resilient Storage Guide

### Step-by-Step for Inexperienced Engineers

**Phase 1: Start Simple**
1. Begin with in-memory storage (ndarray + hnsw)
2. Implement basic CRUD operations
3. Add serialization to disk

**Phase 2: Add Persistence**
1. Implement memory-mapped file storage
2. Design on-disk format (header + data sections)
3. Add write-ahead logging

**Phase 3: Optimize**
1. Profile memory access patterns
2. Add caching layer
3. Implement quantization

**Common Pitfalls:**
- ❌ Don't optimize before measuring
- ❌ Don't skip error handling
- ✅ Do write tests for edge cases
- ✅ Do use existing crates when possible

---

## Blog Posts Analysis

See [./blog/blog-index.md](./blog/blog-index.md) for detailed analysis of all 10 blog posts.

### Key Technical Insights from Blogs

1. **ANN v3:** Hierarchical clustering + binary quantization = 100x throughput improvement
2. **FTS v2:** BM25 full-text search with max-score optimization
3. **Native Filtering:** Server-side filtering without post-processing
4. **Zero-Cost Abstractions:** Rust patterns for performance

---

## Appendix: Source Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.turbopuffer/
├── turbopuffer-python/          # Python SDK (httpx-based)
├── turbopuffer-typescript/      # TypeScript SDK (fetch-based)
├── turbopuffer-go/              # Go SDK (net/http-based)
├── turbopuffer-java/            # Java SDK (OkHttp-based)
├── turbopuffer.com/             # Website with blog posts
├── tpuf-benchmark/              # Go benchmark tool
├── search-benchmark-game/       # Lucene/tantivy comparison
├── turbogrep/                   # Rust semantic code search tool
├── custom-labels/               # Rust profiling labels library
├── turbopuffer-apigen/          # API codegen tool
└── turbopuffer-sdk-bench/       # SDK benchmarks
```
