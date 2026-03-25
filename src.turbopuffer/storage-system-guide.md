# Resilient Storage System Guide

## Step-by-Step Guide for Inexperienced Engineers

This guide walks you through building a resilient storage system for vector search, assuming minimal prior knowledge. We'll build up from the basics.

---

## Table of Contents

1. [Understanding the Basics](#understanding-the-basics)
2. [Step 1: In-Memory Storage](#step-1-in-memory-storage)
3. [Step 2: File-Based Persistence](#step-2-file-based-persistence)
4. [Step 3: Memory-Mapped Files](#step-3-memory-mapped-files)
5. [Step 4: Write-Ahead Logging](#step-4-write-ahead-logging)
6. [Step 5: Indexing](#step-5-indexing)
7. [Common Pitfalls](#common-pitfalls)
8. [Next Steps](#next-steps)

---

## Understanding the Basics

### What Problem Are We Solving?

**The Challenge:**
You have millions (or billions) of vectors. You need to:
1. Store them durably (survive crashes)
2. Find similar vectors quickly (milliseconds, not seconds)
3. Handle concurrent reads and writes
4. Scale as data grows

**Key Concepts:**

```
Vector: A list of numbers representing data
        Example: [0.1, -0.5, 0.8, 0.2] (4-dimensional)

Similarity: How "close" two vectors are
           Measured by distance (cosine, L2, etc.)

Index: A data structure for fast lookup
       Like a book's index, but for vectors

Durability: Data survives crashes
           Write to disk, not just memory
```

### System Requirements

Before coding, define your requirements:

```
Q: How many vectors?
A: Start with 100,000, design for 100 million

Q: What latency is acceptable?
A: < 100ms for p99 queries

Q: How much can you spend?
A: Optimize for cost-efficiency

Q: What's your team's expertise?
A: Be realistic about complexity tolerance
```

---

## Step 1: In-Memory Storage

Start simple. Get something working in memory first.

### Basic Vector Store

```rust
// storage/in_memory.rs
use std::collections::HashMap;

/// Simple in-memory vector store
pub struct InMemoryVectorStore {
    /// Store vectors by ID
    vectors: HashMap<u64, Vec<f32>>,
    /// Next ID to assign
    next_id: u64,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            vectors: HashMap::new(),
            next_id: 0,
        }
    }

    /// Insert a vector, return its ID
    pub fn insert(&mut self, vector: Vec<f32>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.vectors.insert(id, vector);
        id
    }

    /// Get a vector by ID
    pub fn get(&self, id: u64) -> Option<&Vec<f32>> {
        self.vectors.get(&id)
    }

    /// Delete a vector
    pub fn delete(&mut self, id: u64) -> bool {
        self.vectors.remove(&id).is_some()
    }

    /// Get all vectors (for searching)
    pub fn all_vectors(&self) -> impl Iterator<Item = (u64, &Vec<f32>)> {
        self.vectors.iter().map(|(&id, vec)| (id, vec))
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}
```

### Linear Search

```rust
// search/linear.rs
use crate::storage::InMemoryVectorStore;

/// Find k nearest neighbors using linear scan
pub fn linear_search(
    store: &InMemoryVectorStore,
    query: &[f32],
    k: usize,
) -> Vec<(u64, f32)> {
    let mut results: Vec<_> = store
        .all_vectors()
        .map(|(id, vector)| {
            let distance = cosine_distance(query, vector);
            (id, distance)
        })
        .collect();

    // Sort by distance (ascending)
    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Return top-k
    results.into_iter().take(k).collect()
}

/// Cosine distance between two vectors
fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    1.0 - (dot / (norm_a * norm_b))
}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut store = InMemoryVectorStore::new();
        let id = store.insert(vec![1.0, 2.0, 3.0]);

        let vector = store.get(id).unwrap();
        assert_eq!(vector, &vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_linear_search() {
        let mut store = InMemoryVectorStore::new();
        store.insert(vec![1.0, 0.0, 0.0]);  // id=0
        store.insert(vec![0.0, 1.0, 0.0]);  // id=1
        store.insert(vec![0.0, 0.0, 1.0]);  // id=2

        let results = linear_search(&store, &[1.0, 0.0, 0.0], 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0);  // Closest is id=0
    }
}
```

**Limitations of In-Memory Storage:**
- ❌ Data lost on program restart
- ❌ Limited by available RAM
- ❌ O(N) search is slow for large N

---

## Step 2: File-Based Persistence

Now let's make data survive restarts.

### Simple File Format

```rust
// storage/file_store.rs
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// File-based vector store
pub struct FileVectorStore {
    file_path: String,
    vectors: HashMap<u64, Vec<f32>>,
    next_id: u64,
}

impl FileVectorStore {
    /// Create or open existing store
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let mut store = Self {
            file_path: path_str,
            vectors: HashMap::new(),
            next_id: 0,
        };

        // Load existing data if file exists
        if Path::new(&store.file_path).exists() {
            store.load()?;
        }

        Ok(store)
    }

    /// Save all data to file
    pub fn save(&self) -> Result<()> {
        let file = File::create(&self.file_path)?;
        let mut writer = BufWriter::new(file);

        // Write header: [next_id]
        writer.write_all(&self.next_id.to_le_bytes())?;

        // Write each vector: [id][len][data...]
        for (&id, vector) in &self.vectors {
            writer.write_all(&id.to_le_bytes())?;
            writer.write_all(&(vector.len() as u32).to_le_bytes())?;
            for &value in vector {
                writer.write_all(&value.to_le_bytes())?;
            }
        }

        writer.flush()?;
        Ok(())
    }

    /// Load data from file
    fn load(&mut self) -> Result<()> {
        let file = File::open(&self.file_path)?;
        let mut reader = BufReader::new(file);

        // Read header
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;
        self.next_id = u64::from_le_bytes(buf);

        // Read vectors until EOF
        loop {
            // Read ID
            if reader.read_exact(&mut buf).is_err() {
                break;  // EOF
            }
            let id = u64::from_le_bytes(buf);

            // Read length
            reader.read_exact(&mut buf)?;
            let len = u32::from_le_bytes(buf) as usize;

            // Read vector data
            let mut vector = vec![0.0; len];
            for value in &mut vector {
                reader.read_exact(&mut [0u8; 4])?;
                *value = f32::from_le_bytes(/* read bytes */);
            }

            self.vectors.insert(id, vector);
        }

        Ok(())
    }

    pub fn insert(&mut self, vector: Vec<f32>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.vectors.insert(id, vector);
        id
    }

    /// Flush data to disk after each write (slow but safe)
    pub fn insert_with_flush(&mut self, vector: Vec<f32>) -> u64 {
        let id = self.insert(vector);
        let _ = self.save();  // Ignore errors for now
        id
    }
}
```

**Problems with This Approach:**
- ❌ Saving entire file on every write is slow
- ❌ No crash safety (partial writes corrupt data)
- ❌ Must load entire file to read any data

---

## Step 3: Memory-Mapped Files

Memory-mapped files let you access disk data like memory, with OS-managed caching.

### Introduction to mmap

```rust
// storage/mmap_store.rs
use memmap2::{Mmap, MmapOptions};
use std::fs::{File, OpenOptions};
use std::path::Path;

/// Memory-mapped vector store
pub struct MmapVectorStore {
    mmap: Mmap,
    dimension: usize,
    count: usize,
}

impl MmapVectorStore {
    pub fn open<P: AsRef<Path>>(path: P, dimension: usize) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        // Set file size if new
        let file_size = dimension * 4 * 1000000;  // 1M vectors
        file.set_len(file_size as u64)?;

        // Memory map the file
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        Ok(Self {
            mmap,
            dimension,
            count: 0,  // Would track actual count
        })
    }

    /// Get vector at index (zero-copy!)
    pub fn get_vector(&self, index: usize) -> &[f32] {
        let offset = index * self.dimension * 4;  // 4 bytes per f32
        let slice = &self.mmap[offset..offset + self.dimension * 4];

        // Safe cast: f32 has same representation in memory
        bytemuck::cast_slice(slice)
    }

    /// Set vector at index
    pub fn set_vector(&mut self, index: usize, vector: &[f32]) {
        let offset = index * self.dimension * 4;
        let slice = &mut self.mmap[offset..offset + self.dimension * 4];
        slice.copy_from_slice(bytemuck::cast_slice(vector));

        // Flush to ensure durability
        let _ = self.mmap.flush();
    }
}
```

**Benefits of mmap:**
- ✅ OS handles caching automatically
- ✅ Zero-copy access to data
- ✅ Can work with files larger than RAM
- ✅ Changes persist to disk

**Caveats:**
- ⚠️ mmap can fail (address space exhaustion)
- ⚠️ Still need WAL for crash safety
- ⚠️ File format is fixed (hard to change schema)

---

## Step 4: Write-Ahead Logging

WAL ensures durability: write intent to log before making changes.

### Simple WAL Implementation

```rust
// storage/wal.rs
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

/// Write-ahead log entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WalEntry {
    Insert { id: u64, vector: Vec<f32> },
    Delete { id: u64 },
    Checkpoint { vector_count: u64 },
}

/// Write-ahead log
pub struct Wal {
    file: BufWriter<File>,
    sequence: u64,
}

impl Wal {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)  // Append mode
            .open(path)?;

        Ok(Self {
            file: BufWriter::new(file),
            sequence: 0,
        })
    }

    /// Write entry to log
    pub fn write(&mut self, entry: WalEntry) -> Result<u64> {
        let seq = self.sequence;
        self.sequence += 1;

        // Serialize: [sequence][entry_json]
        let json = serde_json::to_vec(&entry)?;

        // Write length-prefixed entry
        self.file.write_all(&json.len().to_le_bytes())?;
        self.file.write_all(&json)?;
        self.file.flush()?;  // Force to disk!

        Ok(seq)
    }

    /// Replay entries (for recovery)
    pub fn replay<F>(&self, mut apply: F) -> Result<()>
    where
        F: FnMut(WalEntry) -> Result<()>,
    {
        // Read and replay all entries
        // Implementation omitted for brevity
        Ok(())
    }

    /// Truncate log (after checkpoint)
    pub fn truncate(&mut self) -> Result<()> {
        let path = "...";
        drop(self.file.clone().into_inner());
        std::fs::remove_file(path)?;
        self.file = BufWriter::new(
            OpenOptions::new().create(true).append(true).open(path)?
        );
        Ok(())
    }
}
```

### Using WAL with Vector Store

```rust
// storage/with_wal.rs
use crate::storage::{MmapVectorStore, Wal, WalEntry};

pub struct DurableVectorStore {
    store: MmapVectorStore,
    wal: Wal,
}

impl DurableVectorStore {
    pub fn new(store: MmapVectorStore, wal: Wal) -> Result<Self> {
        Ok(Self { store, wal })
    }

    /// Insert with durability guarantee
    pub fn insert(&mut self, vector: Vec<f32>) -> Result<u64> {
        // 1. Write to WAL first
        let id = self.store.count as u64;
        self.wal.write(WalEntry::Insert { id, vector: vector.clone() })?;

        // 2. Then update main storage
        self.store.set_vector(self.store.count, &vector);
        self.store.count += 1;

        Ok(id)
    }

    /// Recovery after crash
    pub fn recover(&mut self) -> Result<()> {
        // Replay WAL entries
        self.wal.replay(|entry| {
            match entry {
                WalEntry::Insert { id, vector } => {
                    self.store.set_vector(id as usize, &vector);
                }
                WalEntry::Delete { id } => {
                    // Mark as deleted
                }
                WalEntry::Checkpoint { .. } => {
                    // Reset replay position
                }
            }
            Ok(())
        })
    }
}
```

**WAL Best Practices:**
1. Write to WAL **before** modifying data
2. Flush WAL to disk (fsync)
3. Periodically checkpoint (compact WAL)
4. Delete old WAL entries after checkpoint

---

## Step 5: Indexing

Linear search is O(N). Indexing makes it O(log N) or better.

### Simple Clustering Index

```rust
// index/cluster.rs
use ndarray::{Array1, Array2};

/// Simple k-means clustering index
pub struct ClusterIndex {
    centroids: Vec<Array1<f32>>,
    /// vectors_in_cluster[i] = list of vector IDs in cluster i
    vectors_in_cluster: Vec<Vec<u64>>,
}

impl ClusterIndex {
    /// Build index from vectors
    pub fn build(vectors: &[(u64, Vec<f32>)], k: usize) -> Self {
        // Run k-means clustering
        let (centroids, assignments) = kmeans(vectors, k);

        // Group vectors by cluster
        let mut vectors_in_cluster = vec![Vec::new(); k];
        for (i, &cluster) in assignments.iter().enumerate() {
            vectors_in_cluster[cluster].push(vectors[i].0);
        }

        Self {
            centroids,
            vectors_in_cluster,
        }
    }

    /// Find candidate clusters for query
    pub fn find_candidate_clusters(&self, query: &[f32], n_candidates: usize) -> Vec<usize> {
        let query_arr = Array1::from_vec(query.to_vec());

        // Compute distance to each centroid
        let mut distances: Vec<_> = self.centroids
            .iter()
            .enumerate()
            .map(|(i, centroid)| (cosine_distance_arr(&query_arr, centroid), i))
            .collect();

        // Sort and return top candidates
        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        distances.into_iter()
            .take(n_candidates)
            .map(|(_, i)| i)
            .collect()
    }

    /// Get vector IDs in a cluster
    pub fn get_cluster_vectors(&self, cluster_id: usize) -> &[u64] {
        &self.vectors_in_cluster[cluster_id]
    }
}

fn kmeans(vectors: &[(u64, Vec<f32>)], k: usize) -> (Vec<Array1<f32>>, Vec<usize>) {
    // Simplified k-means implementation
    // In practice, use a library like `linfa-clustering`

    let dimension = vectors[0].1.len();
    let n = vectors.len();

    // Initialize centroids randomly
    let mut centroids: Vec<Array1<f32>> = (0..k)
        .map(|i| {
            let idx = (i * n) / k;
            Array1::from_vec(vectors[idx].1.clone())
        })
        .collect();

    // Iterate until convergence (simplified)
    let mut assignments = vec![0; n];
    for _iteration in 0..10 {
        // Assign vectors to nearest centroid
        for (i, (_, vector)) in vectors.iter().enumerate() {
            let mut best_cluster = 0;
            let mut best_distance = f32::INFINITY;

            for (j, centroid) in centroids.iter().enumerate() {
                let dist = cosine_distance_arr(&Array1::from_vec(vector.clone()), centroid);
                if dist < best_distance {
                    best_distance = dist;
                    best_cluster = j;
                }
            }

            assignments[i] = best_cluster;
        }

        // Update centroids
        for (j, centroid) in centroids.iter_mut().enumerate() {
            let cluster_vectors: Vec<_> = vectors
                .iter()
                .zip(&assignments)
                .filter(|(_, &a)| a == j)
                .map(|((_, v), _)| Array1::from_vec(v.clone()))
                .collect();

            if !cluster_vectors.is_empty() {
                *centroid = cluster_vectors.iter().sum::<Array1<_>>() / cluster_vectors.len() as f32;
            }
        }
    }

    (centroids, assignments)
}

fn cosine_distance_arr(a: &Array1<f32>, b: &Array1<f32>) -> f32 {
    let dot = a.dot(b);
    let norm_a = a.norm(2);
    let norm_b = b.norm(2);
    1.0 - dot / (norm_a * norm_b)
}
```

### Using the Index

```rust
// query/with_index.rs
use crate::storage::MmapVectorStore;
use crate::index::ClusterIndex;

pub struct IndexedSearch {
    store: MmapVectorStore,
    index: ClusterIndex,
}

impl IndexedSearch {
    pub fn new(store: MmapVectorStore, index: ClusterIndex) -> Self {
        Self { store, index }
    }

    pub fn search(&self, query: &[f32], k: usize) -> Vec<(u64, f32)> {
        // 1. Find candidate clusters
        let candidates = self.index.find_candidate_clusters(query, 5);

        // 2. Search only within those clusters
        let mut results = Vec::new();
        for &cluster_id in &candidates {
            for &vector_id in self.index.get_cluster_vectors(cluster_id) {
                let vector = self.store.get_vector(vector_id as usize);
                let distance = cosine_distance(query, vector);
                results.push((vector_id, distance));
            }
        }

        // 3. Sort and return top-k
        results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        results.into_iter().take(k).collect()
    }
}
```

**Performance Improvement:**
```
Linear search: O(N) where N = total vectors
Indexed search: O(N/k) where k = number of clusters

Example with 1M vectors, 1000 clusters:
Linear: 1,000,000 distance computations
Indexed: ~5,000 distance computations (5 candidate clusters × 1000 vectors each)

Speedup: ~200x!
```

---

## Common Pitfalls

### 1. Not Handling Crashes

```rust
// ❌ BAD: Data loss on crash
fn insert(&mut self, vector: Vec<f32>) {
    self.vectors.push(vector);
    // If program crashes here, data is lost!
}

// ✅ GOOD: WAL before modifying data
fn insert(&mut self, vector: Vec<f32>) -> Result<()> {
    self.wal.write(WalEntry::Insert { vector: vector.clone() })?;
    self.vectors.push(vector);
    Ok(())
}
```

### 2. Ignoring Concurrency

```rust
// ❌ BAD: Race condition with multiple writers
static mut STORE: Option<VectorStore> = None;

// ✅ GOOD: Use proper synchronization
use std::sync::{Arc, RwLock};

let store = Arc::new(RwLock::new(VectorStore::new()));

// Writer
store.write().unwrap().insert(vector);

// Reader
let results = store.read().unwrap().search(query);
```

### 3. Not Testing Edge Cases

```rust
// ❌ BAD: Only testing happy path
#[test]
fn test_insert() {
    let mut store = VectorStore::new();
    store.insert(vec![1.0, 2.0]);
    // What about empty vectors? Very large vectors? Concurrent access?
}

// ✅ GOOD: Test edge cases
#[test]
fn test_edge_cases() {
    let mut store = VectorStore::new();

    // Empty vector
    store.insert(vec![]);

    // Very large vector
    store.insert(vec![0.0; 10000]);

    // Many vectors (stress test)
    for i in 0..1000000 {
        store.insert(vec![i as f32]);
    }
}
```

### 4. Premature Optimization

```rust
// ❌ BAD: Optimizing before measuring
struct SuperOptimizedStore {
    // Complex data structure
    // SIMD code everywhere
    // But doesn't even work correctly!
}

// ✅ GOOD: Make it work, then measure, then optimize
struct SimpleStore {
    // Start simple
    vectors: Vec<Vec<f32>>,
}

// Profile to find bottlenecks
// Optimize only what matters
```

---

## Next Steps

### Building on This Foundation

Once you have the basics working:

1. **Add Quantization:**
   - Binary quantization for compression
   - f16 storage for reduced memory

2. **Improve Index:**
   - Hierarchical clustering (tree structure)
   - HNSW for better recall/speed tradeoff

3. **Add Filtering:**
   - Attribute storage
   - Pre-filtering support

4. **Production Features:**
   - Metrics and monitoring
   - Backup/restore procedures
   - Horizontal scaling

### Learning Resources

**Books:**
- "Designing Data-Intensive Applications" by Martin Kleppmann
- "Database Internals" by Alex Petrov

**Online:**
- Rust documentation: https://doc.rust-lang.org/
- memmap2 crate: https://docs.rs/memmap2/

**Papers:**
- SPFresh: https://dl.acm.org/doi/10.1145/3600006.3613166
- RaBitQ: https://dl.acm.org/doi/pdf/10.1145/3654970

---

## Summary

Building resilient storage:

1. **Start in-memory** - Get something working
2. **Add file persistence** - Survive restarts
3. **Use memory mapping** - Efficient large-scale storage
4. **Implement WAL** - Crash safety
5. **Add indexing** - Fast search

**Key Principles:**
- Measure before optimizing
- Test edge cases thoroughly
- Document design decisions
- Start simple, add complexity gradually

Remember: Every production system started as a simple prototype. The key is to build incrementally and test thoroughly at each step.
