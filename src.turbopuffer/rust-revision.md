# Rust Replication Plan

## Complete Guide to Building a Turbopuffer Clone in Rust

This document provides a comprehensive plan for replicating Turbopuffer's functionality in Rust, including crate recommendations, architecture decisions, and implementation guidance.

---

## Table of Contents

1. [Project Overview](#project-overview)
2. [Crate Recommendations](#crate-recommendations)
3. [Architecture Design](#architecture-design)
4. [Storage Layer Implementation](#storage-layer-implementation)
5. [Query Engine Implementation](#query-engine-implementation)
6. [API Layer](#api-layer)
7. [Production Considerations](#production-considerations)
8. [Development Roadmap](#development-roadmap)

---

## Project Overview

### Goals

Build a Rust-based vector search system with:
- **Billion-scale** vector storage
- **Millisecond** query latency
- **High throughput** (10,000+ QPS)
- **Incremental updates** (no full rebuilds)
- **Attribute filtering** (pre-filtering support)

### Scope

This plan covers:
- Core storage engine
- Index structures (SPFresh-inspired)
- Quantization (RaBitQ)
- Query execution
- HTTP API (optional)

Out of scope:
- Managed service infrastructure
- Multi-tenancy
- Authentication/authorization

---

## Crate Recommendations

### Core Dependencies

```toml
[dependencies]
# Linear Algebra
ndarray = "0.16"
ndarray-linalg = "0.17"
nalgebra = "0.33"

# Distance computations
simba = "0.9"              # SIMD linear algebra
wide = "0.7"               # Portable SIMD

# Quantization
half = "2.4"               # f16 support
bitvec = "1.0"             # Bit-level operations

# Storage
memmap2 = "0.9"            # Memory-mapped files
rkyv = "0.7"               # Zero-copy deserialization
byteorder = "1.5"          # Byte conversion utilities

# Concurrency
rayon = "1.10"             # Data parallelism
tokio = { version = "1", features = ["full"] }  # Async runtime
parking_lot = "0.12"       # Fast locks

# Indexing
hnsw = "0.11"              # HNSW implementation (for comparison)

# Filtering
fst = "0.4"                # Finite state transducers for string filtering
roaring = "0.10"           # Bitmaps for filtering

# HTTP API (optional)
axum = "0.8"               # Web framework
tower = "0.5"              # Middleware
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Logging and tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Metrics (optional)
metrics = "0.24"
metrics-exporter-prometheus = "0.16"
```

### Development Dependencies

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
proptest = "1.5"           # Property-based testing
tempfile = "3.15"          # Temp files for tests
criterion = "0.5"          # Benchmarking
```

---

## Architecture Design

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         API Layer                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   HTTP (Axum)   │  │   gRPC (Tonic)  │  │   CLI           │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Query Engine                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Parser        │  │   Optimizer     │  │   Executor      │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       Index Layer                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  Centroid Tree  │  │  Quantized Idx  │  │  Filter Index   │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Storage Layer                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  Memory Map     │  │  Write Buffer   │  │  Compaction     │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Module Structure

```
src/
├── lib.rs
├── main.rs
├── api/
│   ├── mod.rs
│   ├── http.rs          # HTTP API handlers
│   ├── grpc.rs          # gRPC service (optional)
│   └── types.rs         # API types
├── storage/
│   ├── mod.rs
│   ├── mmap.rs          # Memory-mapped file handling
│   ├── writer.rs        # Write operations
│   ├── reader.rs        # Read operations
│   └── compaction.rs    # Background compaction
├── index/
│   ├── mod.rs
│   ├── tree.rs          # Centroid tree
│   ├── quantized.rs     # Quantized vector index
│   ├── builder.rs       # Index construction
│   └── updater.rs       # Incremental updates
├── query/
│   ├── mod.rs
│   ├── engine.rs        # Query execution engine
│   ├── searcher.rs      # Vector search
│   ├── filter.rs        # Filter evaluation
│   └── reranker.rs      # Full-precision reranking
├── compression/
│   ├── mod.rs
│   ├── binary.rs        # Binary quantization (RaBitQ)
│   └── scalar.rs        # Scalar quantization (f16)
├── distance/
│   ├── mod.rs
│   ├── cosine.rs        # Cosine distance
│   ├── l2.rs            # L2 distance
│   ├── dot.rs           # Dot product
│   └── simd.rs          # SIMD implementations
└── filter/
    ├── mod.rs
    ├── expression.rs    # Filter AST
    ├── bitmap.rs        # Bitmap filtering
    └── inverted.rs      # Inverted index
```

---

## Storage Layer Implementation

### Memory-Mapped Vector Store

```rust
// src/storage/mmap.rs
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::path::Path;

/// Memory-mapped vector storage
pub struct MmapVectorStore {
    mmap: Mmap,
    header: StoreHeader,
}

#[derive(Debug, Clone)]
pub struct StoreHeader {
    pub magic: u64,           // Magic number for format identification
    pub version: u32,         // Format version
    pub dimension: u32,       // Vector dimension
    pub vector_count: u64,    // Number of vectors
    pub flags: u64,           // Storage flags
}

impl MmapVectorStore {
    /// Open existing store
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        // Parse header (first 64 bytes)
        let header = Self::parse_header(&mmap[..64])?;

        Ok(Self { mmap, header })
    }

    /// Create new store
    pub fn create<P: AsRef<Path>>(
        path: P,
        dimension: u32,
        capacity: u64,
    ) -> Result<Self> {
        // Create file with initial size
        let file = File::create(path)?;
        let initial_size = Self::calculate_size(dimension, capacity);
        file.set_len(initial_size as u64)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        // Write header
        let header = StoreHeader {
            magic: 0x54505546465f5354,  // "TPUFF_ST"
            version: 1,
            dimension,
            vector_count: 0,
            flags: 0,
        };
        Self::write_header(&mut mmap[..64], &header);

        Ok(Self { mmap, header })
    }

    /// Get quantized vector by index (zero-copy)
    pub fn get_quantized(&self, index: u64) -> Option<&[u8]> {
        if index >= self.header.vector_count {
            return None;
        }

        let bytes_per_vector = (self.header.dimension as usize + 7) / 8;
        let offset = self.quantized_offset() + index as usize * bytes_per_vector;

        Some(&self.mmap[offset..offset + bytes_per_vector])
    }

    /// Get full-precision vector by index (zero-copy)
    pub fn get_full_precision(&self, index: u64) -> Option<&[f32]> {
        if index >= self.header.vector_count {
            return None;
        }

        let offset = self.full_precision_offset() + index as usize * self.header.dimension as usize * 4;
        let slice = &self.mmap[offset..offset + self.header.dimension as usize * 4];

        // Safe cast due to alignment
        Some(bytemuck::cast_slice(slice))
    }

    fn parse_header(bytes: &[u8]) -> Result<StoreHeader> {
        // Parse header fields from bytes
        // ...
    }

    fn write_header(bytes: &mut [u8], header: &StoreHeader) {
        // Write header fields to bytes
        // ...
    }

    fn calculate_size(dimension: u32, capacity: u64) -> usize {
        64 +  // Header
        capacity as usize * ((dimension as usize + 7) / 8) +  // Quantized
        capacity as usize * dimension as usize * 4  // Full precision
    }

    fn quantized_offset(&self) -> usize { 64 }

    fn full_precision_offset(&self) -> usize {
        64 + self.header.vector_count as usize * ((self.header.dimension as usize + 7) / 8)
    }
}
```

### Write-Ahead Log (WAL)

```rust
// src/storage/wal.rs
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};

/// Write-ahead log for durability
pub struct Wal {
    file: BufWriter<File>,
    sequence: u64,
}

#[derive(Debug, Clone)]
pub enum WalEntry {
    Insert { id: u64, vector: Vec<f32>, attributes: serde_json::Value },
    Delete { id: u64 },
}

impl Wal {
    pub async fn open(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        Ok(Self {
            file: BufWriter::new(file),
            sequence: 0,
        })
    }

    pub async fn write(&mut self, entry: WalEntry) -> Result<u64> {
        let seq = self.sequence;
        self.sequence += 1;

        // Serialize entry
        let json = serde_json::to_vec(&entry)?;

        // Write: [seq:8][len:4][data:len]
        self.file.write_u64(seq).await?;
        self.file.write_u32(json.len() as u32).await?;
        self.file.write_all(&json).await?;
        self.file.flush().await?;

        Ok(seq)
    }

    pub async fn replay<F>(&mut self, mut apply: F) -> Result<u64>
    where
        F: FnMut(WalEntry) -> Result<()>,
    {
        // Replay entries from WAL
        // ...
    }

    pub async fn truncate(&mut self) -> Result<()> {
        // Truncate WAL after checkpoint
        let path = "...";
        tokio::fs::remove_file(path).await?;
        self.file = BufWriter::new(
            OpenOptions::new().create(true).append(true).open(path).await?
        );
        Ok(())
    }
}
```

---

## Query Engine Implementation

### Centroid Tree Search

```rust
// src/index/tree.rs
use ndarray::Array1;
use rayon::prelude::*;

/// Centroid tree for hierarchical search
pub struct CentroidTree {
    levels: Vec<CentroidLevel>,
    dimension: usize,
    branching_factor: usize,
}

struct CentroidLevel {
    centroids: Vec<Array1<f32>>,
    children: Vec<Vec<u32>>,  // Child indices for each centroid
    bounding_radii: Vec<f32>,
}

impl CentroidTree {
    /// Build tree from vectors
    pub fn build(vectors: &[Array1<f32>], branching_factor: usize) -> Self {
        let mut levels = Vec::new();
        let mut current: Vec<Array1<f32>> = vectors.to_vec();
        let mut indices: Vec<Vec<u32>> = vec![(0..vectors.len() as u32).collect()];

        while current.len() > 100 {
            let n_clusters = branching_factor.min(current.len());

            // Cluster vectors at this level
            let clusters = kmeans_cluster(&current, n_clusters);

            // Compute centroids
            let centroids: Vec<_> = clusters.iter()
                .map(|ids| {
                    let sum: Array1<f32> = ids.iter()
                        .map(|&i| current[i as usize].clone())
                        .sum();
                    sum / ids.len() as f32
                })
                .collect();

            // Compute bounding radii
            let bounding_radii: Vec<_> = centroids.iter().zip(&clusters)
                .map(|(centroid, ids)| {
                    ids.iter()
                        .map(|&i| cosine_distance(centroid, &current[i as usize]))
                        .fold(0.0, f32::max)
                })
                .collect();

            levels.push(CentroidLevel {
                centroids,
                children: clusters,
                bounding_radii,
            });

            current = levels.last().unwrap().centroids.clone();
            indices = levels.last().unwrap().children.clone();
        }

        Self {
            levels,
            dimension: vectors[0].len(),
            branching_factor,
        }
    }

    /// Find candidate vector IDs for query
    pub fn find_candidates(&self, query: &Array1<f32>, probes: usize) -> Vec<u32> {
        let mut candidates = vec![0];  // Start at root

        for level in &self.levels {
            let mut next_candidates = Vec::new();

            for &node_idx in &candidates {
                let children = &level.children[node_idx as usize];

                // Compare query to all children
                let mut distances: Vec<_> = children.iter()
                    .map(|&child_idx| {
                        let dist = cosine_distance(query, &level.centroids[child_idx as usize]);
                        (dist, child_idx)
                    })
                    .collect();

                // Sort and take top probes
                distances.select_unstable(probes, |a, b| a.0.partial_cmp(&b.0).unwrap());

                for (_, child_idx) in distances.iter().take(probes) {
                    next_candidates.push(*child_idx);
                }
            }

            candidates = next_candidates;
        }

        candidates
    }
}
```

### Two-Phase Query Execution

```rust
// src/query/engine.rs
use crate::compression::RaBitQuantizer;
use crate::storage::MmapVectorStore;

pub struct QueryEngine {
    store: MmapVectorStore,
    tree: CentroidTree,
    quantizer: RaBitQuantizer,
}

pub struct QueryRequest {
    pub query_vector: Vec<f32>,
    pub filter: Option<FilterExpression>,
    pub top_k: usize,
}

pub struct QueryResult {
    pub vectors: Vec<RankedVector>,
    pub total_scanned: usize,
    pub reranked_count: usize,
}

impl QueryEngine {
    pub async fn query(&self, request: QueryRequest) -> Result<QueryResult> {
        // Phase 1: Tree traversal
        let candidate_ids = self.tree.find_candidates(
            &request.query_vector,
            probes: 5,
        );

        // Phase 2: Apply filters
        let filtered_ids = if let Some(filter) = &request.filter {
            self.apply_filter(&candidate_ids, filter)?
        } else {
            candidate_ids
        };

        // Phase 3: Quantized search
        let query_quant = self.quantizer.quantize(&request.query_vector);
        let mut quantized_results = Vec::new();

        for &id in &filtered_ids {
            if let Some(data_quant) = self.store.get_quantized(id as u64) {
                let (est_dist, error) = self.quantizer.estimate_distance(
                    &query_quant,
                    data_quant,
                );
                quantized_results.push((est_dist, error, id));
            }
        }

        // Phase 4: Identify rerank candidates
        let rerank_ids = self.identify_rerank_candidates(
            &quantized_results,
            request.top_k,
        );

        // Phase 5: Full-precision rerank
        let mut final_results = Vec::new();
        for &id in &rerank_ids {
            if let Some(vector) = self.store.get_full_precision(id as u64) {
                let dist = cosine_distance(&request.query_vector, vector);
                final_results.push(RankedVector {
                    id,
                    distance: dist,
                });
            }
        }

        final_results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());

        Ok(QueryResult {
            vectors: final_results.into_iter().take(request.top_k).collect(),
            total_scanned: filtered_ids.len(),
            reranked_count: rerank_ids.len(),
        })
    }

    fn identify_rerank_candidates(
        &self,
        results: &[(f32, f32, usize)],
        top_k: usize,
    ) -> Vec<usize> {
        // Find threshold based on error bounds
        // ...
    }
}
```

---

## API Layer

### HTTP API with Axum

```rust
// src/api/http.rs
use axum::{
    extract::State,
    json::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct QueryRequest {
    vector: Vec<f32>,
    top_k: usize,
    filter: Option<FilterExpression>,
}

#[derive(Serialize)]
struct QueryResponse {
    vectors: Vec<RankedVector>,
    total_scanned: usize,
}

pub fn create_router(engine: QueryEngine) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/query", post(query))
        .with_state(engine)
}

async fn health() -> &'static str {
    "OK"
}

async fn query(
    State(engine): State<QueryEngine>,
    Json(request): Json<QueryRequest>,
) -> Json<QueryResponse> {
    let result = engine.query(QueryRequest {
        query_vector: request.vector,
        filter: request.filter,
        top_k: request.top_k,
    }).await.unwrap();

    Json(QueryResponse {
        vectors: result.vectors,
        total_scanned: result.total_scanned,
    })
}
```

---

## Production Considerations

### 1. Metrics Collection

```rust
use metrics::{counter, histogram};

// Track query latency
let start = Instant::now();
let result = engine.query(request).await?;
histogram!("query_latency").record(start.elapsed());

// Track query results
counter!("queries_total").increment(1);
histogram!("results_per_query").record(result.vectors.len() as f64);
```

### 2. Tracing

```rust
use tracing::{info, instrument, warn};

#[instrument(skip(self, request), fields(query_id = %uuid::Uuid::new_v4()))]
async fn query(&self, request: QueryRequest) -> Result<QueryResult> {
    info!("Starting query");

    // ... query logic

    if result.total_scanned > 10000 {
        warn!(scanned = result.total_scanned, "Large scan");
    }

    Ok(result)
}
```

### 3. Configuration

```rust
use config::{Config, ConfigError, File};

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub workers: usize,
    pub max_concurrent_queries: usize,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub data_directory: String,
    pub mmap_size_gb: usize,
    pub wal_enabled: bool,
}

pub fn load_config() -> Result<Config, ConfigError> {
    Config::builder()
        .add_source(File::with_name("config").required(false))
        .add_source(config::Environment::with_prefix("TPUFF"))
        .build()
}
```

---

## Development Roadmap

### Phase 1: Core Storage (Weeks 1-2)

- [ ] Implement memory-mapped vector store
- [ ] Add WAL for durability
- [ ] Basic CRUD operations
- [ ] Unit tests for storage layer

### Phase 2: Index Structure (Weeks 3-4)

- [ ] Implement centroid tree
- [ ] Add k-means clustering
- [ ] Tree traversal algorithm
- [ ] Benchmark tree search

### Phase 3: Quantization (Week 5)

- [ ] Implement RaBitQ quantization
- [ ] Add error bound calculations
- [ ] Two-phase search integration

### Phase 4: Query Engine (Week 6)

- [ ] Implement full query pipeline
- [ ] Add filter support
- [ ] Optimize hot paths

### Phase 5: API Layer (Week 7)

- [ ] HTTP API with Axum
- [ ] Request validation
- [ ] Error handling

### Phase 6: Production Hardening (Weeks 8-10)

- [ ] Add metrics and tracing
- [ ] Load testing
- [ ] Performance optimization
- [ ] Documentation

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_distance() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert!((cosine_distance(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_quantization_roundtrip() {
        let quantizer = RaBitQuantizer::new(1024);
        let vector = vec![0.5; 1024];
        let quantized = quantizer.quantize(&vector);
        let (est, err) = quantizer.estimate_distance(&quantized, &quantized);
        assert!(est < 0.1);  // Should be very close to 0
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_query() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut store = MmapVectorStore::create(
        temp_dir.path().join("test.store"),
        128,
        1000,
    ).unwrap();

    // Insert vectors
    for i in 0..100 {
        let vector = vec![i as f32 / 100.0; 128];
        store.insert(i, vector).unwrap();
    }

    // Build index
    store.build_index().unwrap();

    // Query
    let query = vec![0.5; 128];
    let results = store.query(&query, 10).await.unwrap();

    assert_eq!(results.len(), 10);
}
```

---

## Summary

This Rust replication plan provides a comprehensive guide for building a Turbopuffer-like vector search system. Key components:

1. **Storage:** Memory-mapped files with WAL
2. **Index:** Centroid tree (SPFresh-inspired)
3. **Quantization:** RaBitQ binary quantization
4. **Query:** Two-phase search (quantized filter + full-precision rerank)
5. **API:** HTTP API with Axum

The modular design allows incremental development and testing. Start with the core storage layer, then build up through indexing, quantization, and finally the API layer.
