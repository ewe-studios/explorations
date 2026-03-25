# pgvectorscale Deep Dive: Vector Search Architecture

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.timescale/pgvectorscale/`

---

## Table of Contents

1. [Overview](#overview)
2. [StreamingDiskANN Algorithm](#streamingdiskann-algorithm)
3. [Index Structure](#index-structure)
4. [Vector Quantization](#vector-quantization)
5. [Filtered Search](#filtered-search)
6. [Query Execution](#query-execution)
7. [Performance Tuning](#performance-tuning)
8. [Rust Implementation Guide](#rust-implementation-guide)

---

## Overview

### What is pgvectorscale?

pgvectorscale is a PostgreSQL extension that provides production-grade vector similarity search, complementing pgvector with:

1. **StreamingDiskANN Index**: Disk-resident ANN (Approximate Nearest Neighbor) search
2. **SBQ Compression**: Statistical Binary Quantization for memory efficiency
3. **Label-Based Filtering**: Efficient filtered vector search
4. **Production Performance**: 28x lower p95 latency vs managed services

### Architecture Comparison

```
┌────────────────────────────────────────────────────────────┐
│                    PGVECTOR (IVFFlat/HNSW)                  │
├────────────────────────────────────────────────────────────┤
│  - In-memory index (HNSW) or simple lists (IVFFlat)        │
│  - Good for small datasets (< 1M vectors)                   │
│  - Limited disk-resident options                            │
│  - No native filtering optimization                         │
└────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────┐
│                    PGVECTORSCALE (DiskANN)                  │
├────────────────────────────────────────────────────────────┤
│  - Disk-resident graph index                               │
│  - Scales to 100M+ vectors                                  │
│  - SSD-optimized access patterns                            │
│  - Native label-based filtering                             │
│  - SBQ compression (2-4 bits per dimension)                 │
└────────────────────────────────────────────────────────────┘
```

### Performance Benchmarks

| Metric | pgvectorscale | Pinecone (s1) | Improvement |
|--------|---------------|---------------|-------------|
| p95 Latency | 50ms | 1400ms | 28x lower |
| Query Throughput | 1600 QPS | 100 QPS | 16x higher |
| Cost (AWS EC2) | $0.076/hr | $0.32/hr | 75% less |

*Benchmark: 50M Cohere embeddings, 768 dimensions, 99% recall*

---

## StreamingDiskANN Algorithm

### Algorithm Foundation

Based on Microsoft's DiskANN research, StreamingDiskANN uses the Vamana algorithm:

```
┌────────────────────────────────────────────────────────────┐
│                    VAMANA ALGORITHM                         │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Input:                                                     │
│    - Dataset P (vectors)                                    │
│    - Max degree R                                           │
│    - Search list size L                                     │
│    - Alpha (diversity factor)                               │
│                                                             │
│  Output:                                                    │
│    - Graph G = (V, E) where V = P                          │
│                                                             │
│  Algorithm:                                                 │
│    1. Sort points by ID (deterministic order)              │
│    2. For each point p in sorted order:                     │
│       a. Search existing graph for L nearest neighbors      │
│       b. Connect p to neighbors with dynamic pruning        │
│       c. Enforce max degree R                               │
│    3. Return constructed graph                              │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Dynamic Pruning

```rust
// Simplified dynamic pruning logic
fn dynamic_pruning(
    candidates: Vec<(NodeId, f32)>,  // (node, distance)
    query: &Vector,
    alpha: f32,
    r: usize,
) -> Vec<NodeId> {
    let mut selected = Vec::new();

    for (node, dist) in candidates {
        // Check if this node is "close enough" to the query
        // relative to already-selected nodes
        let mut should_add = true;

        for &selected_node in &selected {
            let selected_dist = distance(query, &selected_node);
            if dist > alpha * selected_dist {
                should_add = false;
                break;
            }
        }

        if should_add && selected.len() < r {
            selected.push(node);
        }
    }

    selected
}
```

### Graph Construction

```
┌────────────────────────────────────────────────────────────┐
│              GRAPH CONSTRUCTION PROCESS                      │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Iteration 0:                                               │
│  (entry)                                                    │
│    •                                                        │
│                                                             │
│  Iteration 1:                                               │
│  (entry)────•                                               │
│                                                             │
│  Iteration 2:                                               │
│  (entry)────•                                               │
│     │      /                                                │
│     •─────•                                                 │
│                                                             │
│  Final Graph (simplified):                                  │
│                                                             │
│         (entry)                                             │
│        /   |   \                                            │
│       •─── • ───•                                           │
│      / \  / \  / \                                          │
│     •───•───•───•                                           │
│    / \ / \ / \ / \                                          │
│   •───•───•───•───•                                         │
│                                                             │
│  Each node has max R outgoing edges                        │
│  Edges point "closer" to the entry point                   │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

---

## Index Structure

### Physical Layout

```
┌────────────────────────────────────────────────────────────┐
│                    DISKANN INDEX FILE                       │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    META PAGE                         │   │
│  │  - Magic number, version                            │   │
│  │  - Vector count, dimensions                         │   │
│  │  - Index parameters (R, L, alpha)                   │   │
│  │  - Pointer to entry node                            │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    VECTOR DATA                       │   │
│  │  Block 0: Vectors 0-999 (compressed)                │   │
│  │  Block 1: Vectors 1000-1999 (compressed)            │   │
│  │  ...                                                 │   │
│  │  Block N: Vectors (N*1000)-((N+1)*1000-1)           │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    GRAPH EDGES                       │   │
│  │  Node 0: [neighbor1, neighbor2, ...]                │   │
│  │  Node 1: [neighbor3, neighbor4, ...]                │   │
│  │  ...                                                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    LABEL BITMAPS                     │   │
│  │  Label 1: [1,0,1,0,0,1,...] (which vectors have it) │   │
│  │  Label 2: [0,1,0,1,1,0,...]                         │   │
│  │  ...                                                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Memory-Mapped I/O

```rust
// Memory-mapped file access
pub struct MmappedIndex {
    /// Memory-mapped file
    mmap: Mmap,
    /// Pointer to vector data
    vector_data: *const u8,
    /// Pointer to adjacency list
    adjacency_list: *const u8,
    /// Number of vectors
    num_vectors: usize,
    /// Vector dimensions
    dimensions: usize,
}

impl MmappedIndex {
    pub fn get_vector(&self, id: NodeId) -> &Vector {
        let offset = id * self.vector_size();
        unsafe {
            &*(self.vector_data.add(offset) as *const Vector)
        }
    }

    pub fn get_neighbors(&self, id: NodeId) -> &[NodeId] {
        let offset = id * self.edge_size();
        unsafe {
            let len = *(self.adjacency_list.add(offset) as *const usize);
            std::slice::from_raw_parts(
                self.adjacency_list.add(offset + size_of::<usize>()) as *const NodeId,
                len,
            )
        }
    }
}
```

### Page Layout

```
┌────────────────────────────────────────────────────────────┐
│                    INDEX PAGE (8KB)                         │
├────────────────────────────────────────────────────────────┤
│  Offset 0-7:    Page header (magic, flags, next pointer)   │
│  Offset 8-15:   Vector count in page                       │
│  Offset 16-23:  Dimension count                            │
│  Offset 24-31:  Compression metadata                       │
│  Offset 32+:    Vector data (compressed)                    │
│                                                             │
│  For SBQ compressed vectors (768 dims, 2 bits/dim):        │
│  - Each vector: 768 * 2 = 192 bytes = 1536 bits           │
│  - Vectors per page: 8192 / 192 ≈ 42 vectors              │
└────────────────────────────────────────────────────────────┘
```

---

## Vector Quantization

### Statistical Binary Quantization (SBQ)

SBQ compresses vectors from 32-bit floats to 1-2 bits per dimension:

```
┌────────────────────────────────────────────────────────────┐
│                    SBQ COMPRESSION                           │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  Original vector (768 dimensions, f32):                     │
│  [0.12, -0.45, 0.78, -0.23, ..., 0.56]                     │
│  Size: 768 * 4 = 3072 bytes                                 │
│                                                             │
│  Step 1: Compute statistics                                 │
│  - Mean: μ = mean(vector)                                  │
│  - StdDev: σ = stddev(vector)                              │
│                                                             │
│  Step 2: Quantize each dimension                            │
│  For 2-bit quantization:                                    │
│    if x < μ - σ/2: code = 00                               │
│    if μ - σ/2 <= x < μ: code = 01                          │
│    if μ <= x < μ + σ/2: code = 10                          │
│    if x >= μ + σ/2: code = 11                              │
│                                                             │
│  Compressed (768 dims, 2 bits/dim):                         │
│  Size: 768 * 2 / 8 = 192 bytes                              │
│  Compression ratio: 16x                                     │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Quantization Code

```rust
// 2-bit SBQ quantization
pub fn sbq_quantize_2bit(vector: &[f32]) -> Vec<u8> {
    let mean = vector.iter().sum::<f32>() / vector.len() as f32;
    let variance = vector.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f32>() / vector.len() as f32;
    let stddev = variance.sqrt();

    let threshold_low = mean - stddev / 2.0;
    let threshold_high = mean + stddev / 2.0;

    // Pack 4 values per byte (2 bits each)
    let mut compressed = vec![0u8; (vector.len() * 2 + 7) / 8];

    for (i, &value) in vector.iter().enumerate() {
        let code = if value < threshold_low {
            0b00
        } else if value < threshold_high {
            if value < mean { 0b01 } else { 0b10 }
        } else {
            0b11
        };

        let byte_idx = i / 4;
        let bit_offset = (i % 4) * 2;
        compressed[byte_idx] |= code << bit_offset;
    }

    compressed
}

// SBQ distance estimation (dot product approximation)
pub fn sbq_distance_approx(a: &[u8], b: &[u8], dims: usize) -> f32 {
    let mut distance = 0.0f32;

    // Decode and compute distance on-the-fly
    for i in 0..dims {
        let val_a = decode_2bit(a, i);
        let val_b = decode_2bit(b, i);
        distance += val_a * val_b;
    }

    distance
}

fn decode_2bit(data: &[u8], index: usize) -> f32 {
    let byte_idx = index / 4;
    let bit_offset = (index % 4) * 2;
    let code = (data[byte_idx] >> bit_offset) & 0b11;

    // Map back to approximate float values
    match code {
        0b00 => -1.0,
        0b01 => -0.33,
        0b10 => 0.33,
        0b11 => 1.0,
        _ => unreachable!(),
    }
}
```

### Comparison: Plain vs SBQ

| Aspect | Plain Storage | SBQ Storage |
|--------|---------------|-------------|
| Storage per vector (768D) | 3072 bytes | 192 bytes |
| Memory bandwidth | High | 16x lower |
| Distance accuracy | Exact | Approximate |
| Recall at K=100 | 100% | 99%+ |
| Index build time | Baseline | Faster |

---

## Filtered Search

### Label-Based Filtering

pgvectorscale supports efficient filtering during vector search:

```sql
-- Create index with label support
CREATE INDEX ON documents
USING diskann (embedding vector_cosine_ops, labels);

-- Query with label filter
SELECT * FROM documents
WHERE labels && ARRAY[1, 3]  -- Documents with label 1 OR 3
ORDER BY embedding <=> '[query_vector]'
LIMIT 10;
```

### Filtered Graph Traversal

```rust
// Filtered ANN search
pub fn filtered_ann_search(
    index: &DiskannIndex,
    query: &Vector,
    filter: &LabelFilter,
    k: usize,
    search_list_size: usize,
) -> Vec<(NodeId, f32)> {
    let mut visited = BitSet::new();
    let mut search_list = PriorityQueue::new();
    let mut candidate_queue = PriorityQueue::new();

    // Start from entry point
    let entry = index.entry_node();
    let distance = index.distance(entry, query);
    candidate_queue.push(entry, distance);

    while !candidate_queue.is_empty() && search_list.len() < search_list_size {
        let (node, dist) = candidate_queue.pop().unwrap();

        if visited.contains(node) {
            continue;
        }
        visited.insert(node);

        // Check if node passes filter
        if filter.matches(index.get_labels(node)) {
            search_list.push(node, dist);
        }

        // Explore neighbors
        for neighbor in index.get_neighbors(node) {
            if !visited.contains(neighbor) {
                let n_dist = index.distance(neighbor, query);
                candidate_queue.push(neighbor, n_dist);
            }
        }
    }

    // Return top-k from search list
    search_list.into_iter().take(k).collect()
}
```

### Label Bitmap Implementation

```rust
// Efficient label storage and filtering
pub struct LabelIndex {
    /// Map from label ID to bitmap of vectors with that label
    label_bitmaps: HashMap<LabelId, DynamicBitSet>,
    /// Reverse map: vector ID to its labels
    vector_labels: Vec<Vec<LabelId>>,
}

impl LabelIndex {
    pub fn new() -> Self {
        Self {
            label_bitmaps: HashMap::new(),
            vector_labels: Vec::new(),
        }
    }

    pub fn add_vector(&mut self, vector_id: VectorId, labels: Vec<LabelId>) {
        self.vector_labels.push(labels.clone());

        for label in labels {
            self.label_bitmaps
                .entry(label)
                .or_insert_with(|| DynamicBitSet::new())
                .insert(vector_id);
        }
    }

    pub fn get_matching_vectors(&self, filter_labels: &[LabelId]) -> DynamicBitSet {
        if filter_labels.is_empty() {
            return DynamicBitSet::all();
        }

        // OR the bitmaps for all filter labels
        let mut result = DynamicBitSet::new();
        for &label in filter_labels {
            if let Some(bitmap) = self.label_bitmaps.get(&label) {
                result.or_with(bitmap);
            }
        }
        result
    }

    pub fn has_label(&self, vector_id: VectorId, label: LabelId) -> bool {
        self.vector_labels
            .get(vector_id)
            .map(|labels| labels.contains(&label))
            .unwrap_or(false)
    }
}
```

### Smallint Array Overlap

```rust
// Implementation of && operator for smallint[]
#[pg_extern(immutable, parallel_safe)]
pub fn smallint_array_overlap(left: Array<i16>, right: Array<i16>) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }

    // Small arrays: quadratic search
    if left.len() <= 10 && right.len() <= 10 {
        for a in left.iter() {
            for b in right.iter() {
                if a.is_some() && b.is_some() && a.unwrap() == b.unwrap() {
                    return true;
                }
            }
        }
    } else {
        // Large arrays: hash set
        let mut left_set = HashSet::new();
        for a in left.into_iter().flatten() {
            left_set.insert(a);
        }

        for b in right.into_iter().flatten() {
            if left_set.contains(&b) {
                return true;
            }
        }
    }

    false
}
```

---

## Query Execution

### Greedy Search Algorithm

```rust
// Greedy search on the graph
pub fn greedy_search(
    index: &DiskannIndex,
    query: &Vector,
    entry_points: Vec<NodeId>,
    search_list_size: usize,
) -> Vec<(NodeId, f32)> {
    let mut visited = HashSet::new();
    let mut search_list = BinaryHeap::new();  // Max-heap by distance
    let mut candidates = BinaryHeap::new();

    // Initialize with entry points
    for entry in entry_points {
        let dist = index.distance(entry, query);
        candidates.push(Candidate { node: entry, distance: dist });
        visited.insert(entry);
    }

    while let Some(candidate) = candidates.pop() {
        // Check if we can stop
        if candidates.is_empty() ||
           (search_list.len() >= search_list_size &&
            candidate.distance > search_list.peek().unwrap().distance)
        {
            break;
        }

        // Add to search list
        if search_list.len() < search_list_size {
            search_list.push(candidate);
        } else {
            search_list.pop();
            search_list.push(candidate);
        }

        // Explore neighbors
        for neighbor in index.get_neighbors(candidate.node) {
            if !visited.contains(neighbor) {
                visited.insert(neighbor);
                let dist = index.distance(neighbor, query);
                candidates.push(Candidate { node: neighbor, distance: dist });
            }
        }
    }

    // Return sorted results (closest first)
    let mut results: Vec<_> = search_list.into_iter().collect();
    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
    results
}
```

### Rescoring

For improved accuracy, rescore top candidates with exact distances:

```sql
-- Rescore parameter
SET diskann.query_rescore = 400;
```

```rust
// Rescoring for improved accuracy
pub fn rescore_results(
    index: &DiskannIndex,
    query: &Vector,
    candidates: Vec<(NodeId, f32)>,  // (SBQ distance)
    rescore_count: usize,
) -> Vec<(NodeId, f32)> {
    let mut results = candidates;

    // Take top candidates for rescoring
    let to_rescore = results.iter().take(rescore_count).cloned().collect::<Vec<_>>();

    // Compute exact distances
    let mut rescored = Vec::new();
    for (node, _approx_dist) in to_rescore {
        let exact_dist = index.exact_distance(node, query);
        rescored.push((node, exact_dist));
    }

    // Sort by exact distance
    rescored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Replace in results
    for (i, (node, exact_dist)) in rescored.into_iter().enumerate() {
        results[i] = (node, exact_dist);
    }

    results
}
```

### Cost Estimation

```rust
// Query cost estimation for PostgreSQL planner
#[pg_guard]
pub extern "C-unwind" fn amcostestimate(
    root: *mut pg_sys::PlannerInfo,
    index: *mut pg_sys::IndexOptInfo,
    index_clauses: *mut pg_sys::List,
    index_orderbys: *mut pg_sys::List,
    index_correlation: *mut f64,
    index_startup_cost: *mut pg_sys::Cost,
    index_total_cost: *mut pg_sys::Cost,
    index_selectivity: *mut f64,
) {
    // Estimate based on:
    // 1. Number of vectors to scan
    // 2. Search list size (affects graph traversal)
    // 3. Filter selectivity (if applicable)
    // 4. Rescore overhead

    let num_vectors = index->rel->tuples as f64;
    let search_list_size = get_search_list_size() as f64;
    let filter_selectivity = estimate_filter_selectivity(index_clauses);

    // Cost model: O(log N * search_list_size * filter_selectivity)
    *index_startup_cost = 0.0;  // No startup cost for index
    *index_total_cost = (num_vectors.log2() * search_list_size * filter_selectivity) * 0.01;
    *index_selectivity = filter_selectivity;
    *index_correlation = 1.0;  // Index returns ordered results
}
```

---

## Performance Tuning

### Index Build Parameters

```sql
-- Memory for index build
SET maintenance_work_mem = '4GB';

-- Index with custom parameters
CREATE INDEX ON documents USING diskann (embedding vector_cosine_ops)
WITH (
    num_neighbors = 50,         -- R parameter (default: 50)
    search_list_size = 100,      -- L parameter (default: 100)
    max_alpha = 1.2,             -- Alpha parameter (default: 1.2)
    storage_layout = 'memory_optimized',  -- SBQ compression
    num_dimensions = 0,          -- 0 = all dimensions
    num_bits_per_dimension = 2   -- 1 or 2 bits (default: 2)
);

-- Parallel build parameters
SET diskann.parallel_flush_interval = 0.1;
SET diskann.force_parallel_workers = 4;
```

### Query Parameters

```sql
-- Search accuracy vs speed
SET diskann.query_search_list_size = 100;  -- Higher = more accurate, slower
SET diskann.query_rescore = 50;            -- 0 to disable

-- Transaction-local settings
BEGIN;
SET LOCAL diskann.query_search_list_size = 200;
SELECT * FROM documents
ORDER BY embedding <=> '[vector]'
LIMIT 10;
COMMIT;
```

### Parameter Guidelines

| Parameter | Effect | When to Increase | When to Decrease |
|-----------|--------|------------------|------------------|
| `num_neighbors` | Graph connectivity | Higher recall needed | Memory constrained |
| `search_list_size` | Search accuracy | Higher recall needed | Low latency required |
| `max_alpha` | Graph quality | Diverse datasets | Uniform datasets |
| `num_bits_per_dimension` | Compression | Memory constrained | Max accuracy needed |

### Monitoring

```sql
-- Index size
SELECT
    indexrelname,
    pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_stat_user_indexes
WHERE indexrelname LIKE '%diskann%';

-- Index usage
SELECT
    indexrelname,
    idx_scan,
    idx_tup_read
FROM pg_stat_user_indexes
WHERE indexrelname LIKE '%diskann%';
```

---

## Rust Implementation Guide

### Crate Structure

```
diskann-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Public API
│   ├── index/
│   │   ├── mod.rs       # Index module
│   │   ├── builder.rs   # Index construction
│   │   ├── search.rs    # Search algorithms
│   │   └── storage.rs   # Disk storage
│   ├── quantization/
│   │   ├── mod.rs       # Quantization module
│   │   ├── sbq.rs       # SBQ implementation
│   │   └── distance.rs  # Distance functions
│   ├── graph/
│   │   ├── mod.rs       # Graph structures
│   │   ├── neighbor.rs  # Neighbor management
│   │   └── traversal.rs # Graph traversal
│   └── filter/
│       ├── mod.rs       # Filtering module
│       └── bitmap.rs    # Label bitmaps
```

### Core Traits

```rust
/// Distance metric for vector comparison
pub trait DistanceMetric {
    type Vector;

    fn distance(&self, a: &Self::Vector, b: &Self::Vector) -> f32;
}

/// Quantization trait
pub trait Quantizer {
    type Original;
    type Compressed;

    fn quantize(&self, vector: &Self::Original) -> Self::Compressed;
    fn dequantize(&self, compressed: &Self::Compressed) -> Self::Original;
    fn compressed_distance(&self, a: &Self::Compressed, b: &Self::Compressed) -> f32;
}

/// Graph index trait
pub trait VectorIndex {
    type Vector;
    type Id;

    fn search(
        &self,
        query: &Self::Vector,
        k: usize,
        filter: Option<&dyn Fn(Self::Id) -> bool>,
    ) -> Vec<(Self::Id, f32)>;

    fn insert(&mut self, id: Self::Id, vector: Self::Vector);
    fn delete(&mut self, id: Self::Id);
}
```

### Basic Implementation

```rust
use std::sync::Arc;
use memmap2::Mmap;
use roaring::RoaringBitmap;

pub struct DiskannIndex {
    mmap: Mmap,
    entry_point: u32,
    num_vectors: usize,
    dimensions: usize,
    r: usize,
    l: usize,
    alpha: f32,
}

impl DiskannIndex {
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Parse header
        let entry_point = u32::from_le_bytes(mmap[0..4].try_into()?);
        let num_vectors = u32::from_le_bytes(mmap[4..8].try_into()?) as usize;
        let dimensions = u32::from_le_bytes(mmap[8..12].try_into()?) as usize;

        Ok(Self {
            mmap,
            entry_point,
            num_vectors,
            dimensions,
            r: 50,
            l: 100,
            alpha: 1.2,
        })
    }

    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        filter: Option<&RoaringBitmap>,
    ) -> Vec<(u32, f32)> {
        greedy_search(self, query, self.entry_point, k, self.l, filter)
    }
}
```

---

## Related Documentation

- [Analytics Functions](./analytics-functions.md)
- [Rust Implementation](./rust-revision.md)
- [Production Guide](./production-grade.md)
