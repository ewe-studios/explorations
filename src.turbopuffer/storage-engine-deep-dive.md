# Storage Engine Deep Dive

## How Turbopuffer Stores Vector Data

This document explores the storage architecture of Turbopuffer, analyzing how vector data is persisted, indexed, and retrieved at billion-scale.

---

## Table of Contents

1. [Storage Architecture Overview](#storage-architecture-overview)
2. [On-Disk Data Structures](#on-disk-data-structures)
3. [Memory-Mapped File Usage](#memory-mapped-file-usage)
4. [Index Structures](#index-structures)
5. [Compression Techniques](#compression-techniques)
6. [Query Execution Engine](#query-execution-engine)
7. [Rust Implementation Guide](#rust-implementation-guide)

---

## Storage Architecture Overview

### The Three-Tier Storage Model

Turbopuffer employs a three-tier storage architecture that strategically places data based on access patterns and size:

```
┌─────────────────────────────────────────────────────────────────┐
│                         QUERY                                   │
└─────────────────────────────────────────────────────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│     TIER 1      │  │     TIER 2      │  │     TIER 3      │
│   L3 Cache /    │  │      DRAM       │  │    NVMe SSD     │
│     DRAM        │  │                 │  │                 │
├─────────────────┤  ├─────────────────┤  ├─────────────────┤
│ Tree Centroids  │  │ Quantized Data  │  │ Full-Precision  │
│ (Quantized)     │  │ Vectors         │  │ Data Vectors    │
│                 │  │                 │  │                 │
│ Size: ~100 MB   │  │ Size: ~1-10 GB  │  │ Size: ~100 GB   │
│ Bandwidth:      │  │ Bandwidth:      │  │ Bandwidth:      │
│ ~600 GB/s       │  │ ~300 GB/s       │  │ ~10 GB/s        │
│ Access:         │  │ Access:         │  │ Access:         │
│ Random, ~10 ns  │  │ Random, ~100 ns │  │ Random, ~100 μs │
└─────────────────┘  └─────────────────┘  └─────────────────┘
                              │
                              ▼
                 ┌─────────────────────────┐
                 │   Object Storage (S3)   │
                 │                         │
                 │   Size: Unlimited       │
                 │   Bandwidth: ~1 GB/s    │
                 │   Access: ~10-100 ms    │
                 └─────────────────────────┘
```

### Key Design Principles

1. **Data Duplication for Performance:**
   - Every vector stored twice (quantized + full precision)
   - 16-32x compression enables keeping quantized in faster memory
   - Only fetch full precision for <1% of vectors

2. **Spatial Locality:**
   - Related vectors stored contiguously on disk
   - Clustered layout minimizes disk seeks
   - Sequential reads where possible

3. **Temporal Locality:**
   - Frequently accessed data (centroids) in faster memory
   - LRU eviction from hot to cold storage
   - Tree upper levels naturally stay resident

---

## On-Disk Data Structures

### Namespace File Format

Based on analysis of the API and benchmark templates, here's the inferred on-disk structure:

```
Namespace File Layout:
┌─────────────────────────────────────────────────────────────────┐
│                         HEADER (64 bytes)                        │
├─────────────────────────────────────────────────────────────────┤
│  Magic Number (8 bytes)     │ Version (4 bytes) │ Flags (4 b)  │
│  Dimension (4 bytes)        │ Distance Metric (4 bytes)        │
│  Vector Count (8 bytes)     │ Centroid Levels (4 bytes)        │
│  Reserved (24 bytes)                                              │
├─────────────────────────────────────────────────────────────────┤
│                    CENTROID TREE SECTION                         │
├─────────────────────────────────────────────────────────────────┤
│  Level 0: Root Centroid (1 × dimension × 4 bytes)               │
├─────────────────────────────────────────────────────────────────┤
│  Level 1: Centroids + Child Pointers                            │
│  [Centroid][ChildPtrs] [Centroid][ChildPtrs] ...                │
├─────────────────────────────────────────────────────────────────┤
│  Level 2-N: Same structure, increasing size                     │
├─────────────────────────────────────────────────────────────────┤
│                  QUANTIZED VECTOR SECTION                        │
├─────────────────────────────────────────────────────────────────┤
│  Cluster 0: [Binary Vector 0][Binary Vector 1]...               │
│  Cluster 1: [Binary Vector 100][Binary Vector 101]...           │
│  ...                                                            │
├─────────────────────────────────────────────────────────────────┤
│                 FULL-PRECISION VECTOR SECTION                    │
├─────────────────────────────────────────────────────────────────┤
│  Cluster 0: [FP Vector 0][FP Vector 1]...                       │
│  Cluster 1: [FP Vector 100][FP Vector 101]...                   │
│  ...                                                            │
├─────────────────────────────────────────────────────────────────┤
│                   ATTRIBUTE INDEX SECTION                        │
├─────────────────────────────────────────────────────────────────┤
│  Attribute Schema                                               │
│  Inverted Index for Filtering                                   │
│  Document Metadata                                              │
└─────────────────────────────────────────────────────────────────┘
```

### Cluster Structure

Each cluster contains vectors grouped by spatial proximity:

```rust
/// On-disk cluster format
#[repr(C)]
struct ClusterHeader {
    cluster_id: u64,
    parent_centroid_id: u64,
    vector_count: u32,
    bounding_radius: f32,  // Max distance from centroid
    centroid_offset: u64,   // Offset to centroid data
    quantized_offset: u64,  // Offset to quantized vectors
    full_precision_offset: u64,  // Offset to full vectors
}

/// Centroid representation
struct Centroid {
    id: u64,
    vector: Vec<f32>,  // dimension × 4 bytes
    child_count: u32,
    child_ids: Vec<u64>,  // For non-leaf nodes
}
```

### Attribute Storage

For filtering and metadata retrieval:

```rust
/// Attribute schema definition
struct AttributeSchema {
    name: String,
    attr_type: AttributeType,  // String, Int, Float, Bool
    index_type: IndexType,     // None, Inverted, BTree
}

/// Inverted index for string attributes
struct InvertedIndex {
    term_to_ids: HashMap<String, Vec<u64>>,  // Term → Vector IDs
}

/// B-Tree for numeric attributes
struct NumericIndex {
    ranges: BTreeMap<(f64, f64), Vec<u64>>,  // Range → Vector IDs
}
```

---

## Memory-Mapped File Usage

### Why Memory Mapping?

Memory-mapped files provide several advantages for vector storage:

1. **Lazy Loading:** Pages loaded on-demand by OS
2. **Zero-Copy Access:** No intermediate buffers needed
3. **OS Caching:** Leverages OS page cache automatically
4. **Simplified Code:** No manual buffer management

### Rust Implementation with memmap2

```rust
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::path::Path;

/// Memory-mapped vector store
pub struct MmapVectorStore {
    mmap: Mmap,
    dimension: usize,
    vector_count: usize,
    quantized_offset: usize,
    full_precision_offset: usize,
}

impl MmapVectorStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        // Parse header
        let dimension = u32::from_le_bytes(mmap[16..20].try_into()?) as usize;
        let vector_count = u64::from_le_bytes(mmap[24..32].try_into()?) as usize;
        let quantized_offset = u64::from_le_bytes(mmap[32..40].try_into()?) as usize;
        let full_precision_offset = u64::from_le_bytes(mmap[40..48].try_into()?) as usize;

        Ok(Self {
            mmap,
            dimension,
            vector_count,
            quantized_offset,
            full_precision_offset,
        })
    }

    /// Access quantized vector at index (zero-copy)
    pub fn get_quantized(&self, index: usize) -> &[u8] {
        let bytes_per_vector = (self.dimension + 7) / 8;  // 1 bit per dim
        let offset = self.quantized_offset + index * bytes_per_vector;
        &self.mmap[offset..offset + bytes_per_vector]
    }

    /// Access full-precision vector at index (zero-copy)
    pub fn get_full_precision(&self, index: usize) -> &[f32] {
        let offset = self.full_precision_offset + index * self.dimension * 4;
        let slice = &self.mmap[offset..offset + self.dimension * 4];
        bytemuck::cast_slice(slice)
    }
}
```

### Page-Aligned Access

For optimal performance, align data structures with memory pages:

```rust
const PAGE_SIZE: usize = 4096;

/// Ensure cluster boundaries align with page sizes
fn align_to_page(offset: usize) -> usize {
    (offset + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Cluster layout with padding for alignment
struct AlignedCluster {
    header: ClusterHeader,      // 64 bytes
    _padding1: [u8; 24],        // Pad to 88 bytes
    centroid: [f32; 1024],      // 4096 bytes (exactly 1 page)
    _padding2: [u8; 128],       // Pad cluster metadata to page boundary
    quantized_start: usize,     // Offset within mmap
    full_precision_start: usize,
}
```

---

## Index Structures

### Centroid Tree (SPFresh-inspired)

The tree structure enables efficient candidate selection:

```rust
/// Centroid tree for hierarchical search
pub struct CentroidTree {
    levels: Vec<CentroidLevel>,
    branching_factor: usize,
}

pub struct CentroidLevel {
    /// Centroid vectors for this level
    centroids: Vec<f32>,  // Flattened: [c0_dim0, c0_dim1, ..., c1_dim0, ...]

    /// Child pointers (for non-leaf levels)
    /// Each centroid has `branching_factor` children
    children: Vec<u32>,   // Indices into next level

    /// Bounding radii for pruning
    bounding_radii: Vec<f32>,
}

impl CentroidTree {
    /// Find candidate clusters for a query
    pub fn find_candidates(&self, query: &[f32], probes: usize) -> Vec<u32> {
        let mut candidates = vec![0];  // Start at root

        for level in &self.levels {
            let mut next_candidates = Vec::with_capacity(probes * self.branching_factor);

            for &centroid_idx in &candidates {
                let children_start = centroid_idx as usize * self.branching_factor;
                let children_end = children_start + self.branching_factor;

                // Compare query to all children
                let mut child_distances: Vec<_> = (children_start..children_end)
                    .map(|i| {
                        let child_centroid = &level.centroids[i * self.dimension..];
                        (cosine_distance(query, child_centroid), i)
                    })
                    .collect();

                // Sort and take top probes
                child_distances.select_unstable(probes, |a, b| a.0.partial_cmp(&b.0).unwrap());

                for (_, child_idx) in child_distances.iter().take(probes) {
                    next_candidates.push(*child_idx as u32);
                }
            }

            candidates = next_candidates;
        }

        candidates
    }
}
```

### Quantized Vector Index

Binary vectors stored with bit-packing:

```rust
use bitvec::prelude::*;

/// Packed binary vector storage
pub struct QuantizedVectorIndex {
    /// All vectors packed contiguously
    data: BitVec<u8, Lsb0>,
    dimension: usize,
    count: usize,
}

impl QuantizedVectorIndex {
    pub fn new(dimension: usize, count: usize) -> Self {
        let total_bits = dimension * count;
        Self {
            data: bitvec![u8, Lsb0; 0; total_bits],
            dimension,
            count,
        }
    }

    /// Set a quantized vector
    pub fn set(&mut self, index: usize, bits: &[bool]) {
        let start = index * self.dimension;
        for (i, &bit) in bits.iter().enumerate() {
            self.data.set(start + i, bit);
        }
    }

    /// Get a quantized vector as a view
    pub fn get(&self, index: usize) -> BitSlice<u8, Lsb0> {
        let start = index * self.dimension;
        &self.data[start..start + self.dimension]
    }

    /// Hamming distance between two quantized vectors
    pub fn hamming_distance(&self, a: usize, b: usize) -> usize {
        let a_bits = self.get(a);
        let b_bits = self.get(b);
        a_bits.hamming_distance(&b_bits)
    }
}
```

---

## Compression Techniques

### Binary Quantization (RaBitQ)

```rust
/// RaBitQ binary quantization
pub struct RaBitQuantizer {
    dimension: usize,
    /// Precomputed scaling factors for error estimation
    scale_factors: Vec<f32>,
}

impl RaBitQuantizer {
    pub fn new(dimension: usize) -> Self {
        // Compute scale factors based on dimension
        // See RaBitQ paper for exact formula
        let scale_factors = (0..dimension)
            .map(|i| /* computation based on dim */)
            .collect();

        Self { dimension, scale_factors }
    }

    /// Quantize a full-precision vector to binary
    pub fn quantize(&self, vector: &[f32]) -> Vec<bool> {
        vector.iter().map(|&x| x > 0.0).collect()
    }

    /// Estimate distance from quantized vectors
    /// Returns (estimated_distance, error_bound)
    pub fn estimate_distance(
        &self,
        query_quantized: &[bool],
        data_quantized: &[bool],
    ) -> (f32, f32) {
        // Count matching bits
        let matching = query_quantized
            .iter()
            .zip(data_quantized.iter())
            .filter(|(&q, &d)| q == d)
            .count();

        // RaBitQ distance estimation formula
        // This is simplified; actual formula from paper is more complex
        let estimated = 1.0 - (2.0 * matching as f32 / self.dimension as f32);
        let error_bound = self.compute_error_bound();

        (estimated, error_bound)
    }

    fn compute_error_bound(&self) -> f32 {
        // Error bound depends on dimension and quantization quality
        // Higher dimension = tighter bounds (concentration of measure)
        1.0 / (self.dimension as f32).sqrt()
    }
}
```

### Scalar Quantization (for f16 storage)

```rust
use half::f16;

/// Convert f32 vector to f16 for storage
pub fn quantize_to_f16(vector: &[f32]) -> Vec<f16> {
    vector.iter().map(|&x| f16::from_f32(x)).collect()
}

/// Convert back to f32 for computation
pub fn dequantize_from_f16(vector: &[f16]) -> Vec<f32> {
    vector.iter().map(|&x| x.to_f32()).collect()
}

/// Direct computation on f16 (requires f16 arithmetic support)
pub fn dot_product_f16(a: &[f16], b: &[f16]) -> f32 {
    let sum: f32 = a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| x.to_f32() * y.to_f32())
        .sum();
    sum
}
```

---

## Query Execution Engine

### Multi-Stage Query Pipeline

```rust
pub struct QueryEngine {
    tree_searcher: TreeSearcher,
    quantized_searcher: QuantizedSearcher,
    reranker: FullPrecisionReranker,
    filter_engine: FilterEngine,
}

pub struct QueryRequest {
    query_vector: Vec<f32>,
    filter: Option<FilterExpression>,
    top_k: usize,
    include_attributes: bool,
}

pub struct QueryResult {
    vectors: Vec<RankedVector>,
    total_candidates: usize,
    reranked_count: usize,
}

impl QueryEngine {
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResult> {
        // Stage 1: Tree traversal to find candidate clusters
        let candidate_clusters = self.tree_searcher
            .find_candidates(&request.query_vector, probes=5);

        // Stage 2: Apply filters (if any)
        let filtered_clusters = if let Some(filter) = &request.filter {
            self.filter_engine.apply(&candidate_clusters, filter)?
        } else {
            candidate_clusters
        };

        // Stage 3: Quantized search within clusters
        let quantized_results = self.quantized_searcher
            .search(&request.query_vector, &filtered_clusters)
            .await;

        // Stage 4: Identify rerank candidates using error bounds
        let rerank_ids = self.identify_rerank_candidates(
            &quantized_results,
            request.top_k
        );

        // Stage 5: Scatter-gather full-precision vectors
        let reranked = self.reranker
            .fetch_and_rank(&request.query_vector, &rerank_ids)
            .await;

        Ok(QueryResult {
            vectors: reranked.into_iter().take(request.top_k).collect(),
            total_candidates: quantized_results.len(),
            reranked_count: rerank_ids.len(),
        })
    }

    fn identify_rerank_candidates(
        &self,
        results: &[QuantizedResult],
        top_k: usize,
    ) -> Vec<u64> {
        // Find the threshold: best possible score for (top_k + 1)-th candidate
        let mut sorted: Vec<_> = results.iter()
            .map(|r| (r.estimated_distance - r.error_bound, r.vector_id))
            .collect();
        sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let threshold = sorted.get(top_k)
            .map(|&(score, _)| score)
            .unwrap_or(f32::INFINITY);

        // All candidates whose best possible score beats threshold
        results.iter()
            .filter(|r| r.estimated_distance + r.error_bound < threshold)
            .map(|r| r.vector_id)
            .collect()
    }
}
```

### Scatter-Gather Optimization

```rust
impl FullPrecisionReranker {
    /// Fetch multiple vectors concurrently using scatter-gather
    pub async fn fetch_and_rank(
        &self,
        query: &[f32],
        vector_ids: &[u64],
    ) -> Vec<RankedVector> {
        // Batch vector IDs by cluster for efficient fetching
        let by_cluster = self.group_by_cluster(vector_ids);

        // Fetch clusters in parallel
        let fetch_futures: Vec<_> = by_cluster
            .into_iter()
            .map(|(cluster_id, ids)| self.fetch_cluster_batch(cluster_id, &ids))
            .collect();

        let cluster_results = futures::future::join_all(fetch_futures).await;

        // Compute distances and rank
        let mut results: Vec<_> = cluster_results
            .into_iter()
            .flatten()
            .filter_map(|(id, vector)| {
                vector.map(|v| (id, v))
            })
            .map(|(id, vector)| {
                let distance = self.distance_fn(query, &vector);
                RankedVector { id, distance, vector: None }  // Don't return full vector
            })
            .collect();

        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
        results
    }

    /// Fetch a batch of vectors from a single cluster
    async fn fetch_cluster_batch(
        &self,
        cluster_id: u64,
        vector_ids: &[u64],
    ) -> Vec<(u64, Option<Vec<f32>>)> {
        // Compute offsets for all vectors in cluster
        let offsets: Vec<_> = vector_ids
            .iter()
            .map(|&id| self.vector_offset(cluster_id, id))
            .collect();

        // Single async read for all vectors in cluster
        let cluster_data = self.mmap_store.read_cluster(cluster_id).await?;

        // Extract individual vectors
        vector_ids.iter()
            .zip(offsets.iter())
            .map(|(&id, &offset)| {
                let vector = cluster_data.get(offset);
                (id, vector)
            })
            .collect()
    }
}
```

---

## Rust Implementation Guide

### Crate Recommendations

```toml
[dependencies]
# Core data structures
ndarray = "0.16"           # N-dimensional arrays
ndarray-linalg = "0.17"    # Linear algebra operations
nalgebra = "0.33"          # Alternative linear algebra

# Bit manipulation
bitvec = "1.0"             # Bit-level operations
half = "2.4"               # f16 support

# Memory mapping
memmap2 = "0.9"            # Memory-mapped files

# Parallelism
rayon = "1.10"             # Data parallelism
tokio = { version = "1", features = ["full"] }  # Async runtime

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rkyv = "0.7"               # Zero-copy deserialization
bytemuck = "1.16"          # Safe casts for mmap

# SIMD (optional, for advanced optimization)
simba = "0.9"              # SIMD linear algebra
portable-simd = "0.1"      # Portable SIMD (nightly)

# Error handling
anyhow = "1.0"
thiserror = "2.0"
```

### Project Structure

```
src/
├── lib.rs
├── storage/
│   ├── mod.rs
│   ├── mmap_store.rs       # Memory-mapped file handling
│   ├── cluster.rs          # Cluster data structures
│   └── writer.rs           # Writing new data
├── index/
│   ├── mod.rs
│   ├── centroid_tree.rs    # Hierarchical clustering
│   ├── quantized_index.rs  # Binary vector storage
│   └── builder.rs          # Index construction
├── query/
│   ├── mod.rs
│   ├── engine.rs           # Query execution
│   ├── tree_searcher.rs    # Centroid tree traversal
│   ├── quantized_search.rs # Binary vector search
│   └── reranker.rs         # Full-precision reranking
├── compression/
│   ├── mod.rs
│   ├── binary_quantize.rs  # RaBitQ implementation
│   └── scalar_quantize.rs  # f16 quantization
└── filter/
    ├── mod.rs
    ├── expression.rs       # Filter AST
    └── engine.rs           # Filter evaluation
```

### Example: Building an Index

```rust
use turbopuffer_rs::{VectorStore, IndexBuilder, RaBitQuantizer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a new vector store
    let mut store = VectorStore::create("my_namespace")?;

    // Configure index
    let builder = IndexBuilder::new()
        .dimension(1024)
        .distance_metric(DistanceMetric::Cosine)
        .branching_factor(100)
        .tree_depth(3);

    // Add vectors
    let quantizer = RaBitQuantizer::new(1024);

    for (id, vector) in load_vectors().await? {
        let quantized = quantizer.quantize(&vector);
        store.insert(id, vector, quantized)?;
    }

    // Build the index (constructs centroid tree)
    store.build_index(&builder)?;

    // Flush to disk
    store.flush()?;

    Ok(())
}
```

### Example: Querying

```rust
use turbopuffer_rs::{VectorStore, QueryRequest, Filter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load existing store
    let store = VectorStore::open("my_namespace")?;
    let engine = store.query_engine();

    // Build query
    let query = QueryRequest {
        query_vector: embed_query("find similar products").await?,
        filter: Some(Filter::parse("price < 50 AND category = 'shoes'")?),
        top_k: 10,
        include_attributes: true,
    };

    // Execute query
    let results = engine.query(query).await?;

    // Display results
    for result in results.vectors.iter().take(5) {
        println!("ID: {}, Distance: {:.4}", result.id, result.distance);
        if let Some(attrs) = &result.attributes {
            println!("  Attributes: {:?}", attrs);
        }
    }

    Ok(())
}
```

---

## Summary

Turbopuffer's storage engine achieves billion-scale vector search through:

1. **Three-tier storage** strategically placing data by access frequency
2. **Dual representation** (quantized + full precision) for filtering and accuracy
3. **Memory-mapped files** for zero-copy access and OS caching
4. **Hierarchical clustering** to bound search space
5. **Scatter-gather I/O** for efficient random access

The key insight is working with the memory hierarchy rather than against it—compressing data to fit in faster memory tiers while maintaining accuracy through careful error bounding.
