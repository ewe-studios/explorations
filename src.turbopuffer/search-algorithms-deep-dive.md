# Search Algorithms Deep Dive

## ANN, HNSW, IVF, and Turbopuffer's Approach

This document provides a comprehensive analysis of approximate nearest neighbor search algorithms, with detailed explanations from first principles.

---

## Table of Contents

1. [Introduction to Nearest Neighbor Search](#introduction-to-nearest-neighbor-search)
2. [Exact vs. Approximate Search](#exact-vs-approximate-search)
3. [IVF (Inverted File Index)](#ivf-inverted-file-index)
4. [HNSW (Hierarchical Navigable Small World)](#hnsw-hierarchical-navigable-small-world)
5. [SPFresh (Turbopuffer's Foundation)](#spfresh-turbopuffers-foundation)
6. [Binary Quantization (RaBitQ)](#binary-quantization-rabitq)
7. [Algorithm Comparison](#algorithm-comparison)
8. [Implementation Guide](#implementation-guide)

---

## Introduction to Nearest Neighbor Search

### The Fundamental Problem

**Problem Statement:**
Given:
- A database of N vectors: D = {v₁, v₂, ..., vₙ} where each vᵢ ∈ ℝᵈ
- A query vector: q ∈ ℝᵈ
- A distance metric: dist(q, v)

Find: The k vectors in D closest to q

**Applications:**
```
1. Similarity Search
   - "Find products similar to this image"
   - "Find documents about this topic"

2. Recommendation
   - "Find users similar to this user"
   - "Find items similar to liked items"

3. Clustering
   - "Find cluster centers for this data"
   - "Assign points to nearest cluster"

4. Deduplication
   - "Find near-duplicate images"
   - "Find similar customer records"
```

### Distance Metrics Explained

**1. Euclidean Distance (L2)**
```
dist(a, b) = √(Σ(aᵢ - bᵢ)²)

Intuition: "As the crow flies" distance
Properties:
- Range: [0, ∞)
- dist(a, a) = 0
- Satisfies triangle inequality

Use case: When magnitude matters
```

**2. Cosine Distance**
```
dist(a, b) = 1 - (a · b) / (||a|| × ||b||)

Where:
- a · b = Σ(aᵢ × bᵢ) = dot product
- ||a|| = √(Σaᵢ²) = magnitude

Intuition: Angle between vectors (ignoring magnitude)
Properties:
- Range: [0, 2] for general vectors
- Range: [0, 1] for normalized vectors
- dist(a, a) = 0 (for normalized vectors)

Use case: Text embeddings, where direction = meaning
```

**3. Dot Product (Inner Product)**
```
dist(a, b) = -(a · b)  (negative because we want to minimize)

Intuition: Projection of one vector onto another
Properties:
- Range: (-∞, ∞)
- Not a true metric (doesn't satisfy triangle inequality)

Use case: When vectors are already normalized
Equivalent to cosine distance for normalized vectors
```

**Relationship Between Metrics:**
```
For normalized vectors (||a|| = ||b|| = 1):

cosine_distance(a, b) = 1 - a · b
l2_distance(a, b)² = ||a - b||² = ||a||² + ||b||² - 2(a · b)
                    = 2 - 2(a · b)
                    = 2(1 - a · b)
                    = 2 × cosine_distance(a, b)

So for normalized vectors:
l2_distance(a, b) = √(2 × cosine_distance(a, b))

They're monotonically equivalent! Same ranking, different scale.
```

---

## Exact vs. Approximate Search

### Linear Scan (Exact)

**Algorithm:**
```python
def linear_scan(query, database, k):
    """O(N × D) exact nearest neighbor search."""
    distances = []

    for i, vector in enumerate(database):
        dist = cosine_distance(query, vector)
        distances.append((dist, i))

    distances.sort()  # O(N log N)
    return distances[:k]
```

**Complexity:**
- Time: O(N × D + N log N)
- Space: O(1) extra space beyond database

**Why It Doesn't Scale:**
```
Example: 100 billion vectors, 1024 dimensions

Operations per query:
- Distance computations: 100B × 1024 = 102.4 trillion FLOPs
- Sorting: 100B × log₂(100B) ≈ 3.7 trillion comparisons

At 1 TFLOP/s (optimistic):
Time = 102.4 seconds per query

For real-time search (<100ms), we need ~1000x speedup!
```

### Approximate Nearest Neighbor (ANN)

**Definition:**
ANN algorithms find "good enough" nearest neighbors much faster than exact search.

**Quality Metric: Recall@K**
```
Recall@K = (Number of true top-K neighbors found) / K

Example:
True top-10 neighbors: {A, B, C, D, E, F, G, H, I, J}
ANN returns: {A, B, C, X, D, E, Y, F, G, Z}
Correct: {A, B, C, D, E, F, G} = 7 items
Recall@10 = 7/10 = 70%
```

**The Tradeoff:**
```
        Accuracy │
                 │     *
                 │    *
                 │   *
                 │  *
                 │ *
                 │*
                 └────────────────── Speed
                    Fast    Slow

Goal: Top-right corner (fast AND accurate)
```

---

## IVF (Inverted File Index)

### Core Concept

IVF is the simplest ANN algorithm, based on clustering.

**Intuition:**
```
Library Analogy:
- Instead of searching every book (vector)
- Find the right section (cluster) first
- Then search within that section

This is how IVF works!
```

### Index Construction

```python
def build_ivf(vectors, n_clusters=1000):
    """Build IVF index from vectors."""
    # Step 1: Cluster vectors using k-means
    centroids = kmeans(vectors, n_clusters)

    # Step 2: Assign each vector to nearest centroid
    inverted_index = {i: [] for i in range(n_clusters)}
    for idx, vector in enumerate(vectors):
        centroid_id = argmin([distance(vector, c) for c in centroids])
        inverted_index[centroid_id].append(idx)

    return {
        'centroids': centroids,
        'inverted_index': inverted_index,
        'vectors': vectors
    }
```

**Visualization:**
```
Vectors in 2D Space:

    · · · · · ·     · · · ·     · · · · ·
    · · · · ·   ·   · · · · ·   · · · · ·
      · · · · · · · · · · · · · · · · ·
    ───────────────────────────────────────
    │  Cluster 0  │  Cluster 1  │  Cluster 2  │
    │  Centroid:  │  Centroid:  │  Centroid:  │
    │    (1.2,3.4)│    (5.6,7.8)│    (9.0,1.2)│
    ───────────────────────────────────────

Inverted Index:
Cluster 0: [0, 1, 2, 3, 4, 5, ...]
Cluster 1: [156, 157, 158, ...]
Cluster 2: [312, 313, 314, ...]
```

### Query Processing

```python
def query_ivf(index, query, k, n_probe=5):
    """
    Query IVF index.

    n_probe: Number of clusters to search
    """
    # Step 1: Find nearest centroids
    distances_to_centroids = [
        distance(query, c) for c in index['centroids']
    ]

    # Get indices of n_probe closest centroids
    probe_clusters = argsort(distances_to_centroids)[:n_probe]

    # Step 2: Search within those clusters
    candidates = []
    for cluster_id in probe_clusters:
        for vector_idx in index['inverted_index'][cluster_id]:
            vector = index['vectors'][vector_idx]
            dist = distance(query, vector)
            candidates.append((dist, vector_idx))

    # Step 3: Return top-k
    candidates.sort()
    return candidates[:k]
```

### IVF Performance

**Complexity:**
- Index build: O(N × D × iterations) for k-means
- Query time: O(D × (n_clusters + n_probe × N/n_clusters))

**Optimal Cluster Count:**
```
For N vectors, optimal n_clusters ≈ √N

Example:
N = 1,000,000 vectors
n_clusters ≈ √1,000,000 = 1,000

Each cluster has ~1,000 vectors
With n_probe = 10, we search 10,000 vectors instead of 1,000,000
Speedup: 100x!
```

**Limitations:**
1. Fixed partition boundaries (hard assignments)
2. Performance depends on cluster quality
3. Not incremental (must rebuild for updates)

---

## HNSW (Hierarchical Navigable Small World)

### Graph-Based ANN

HNSW is currently the most popular ANN algorithm, offering excellent speed/accuracy tradeoff.

**Key Ideas:**
1. **Navigable Small World:** Graph where greedy search finds short paths
2. **Hierarchical:** Multiple layers for faster long-distance navigation

### Graph Construction

```python
def build_hnsw(vectors, M=16, max_layer=-1):
    """
    Build HNSW graph.

    M: Maximum number of connections per node
    max_layer: Maximum layer (logarithmic in N)
    """
    graph = {i: {layer: [] for layer in range(max_layer+1)}
             for i in range(len(vectors))}

    for i, vector in enumerate(vectors):
        # Determine random layer for this element
        layer = random_geometric(max_layer)

        # Find insertion point starting from top layer
        entry_point = random_element()

        for layer in range(max_layer, -1, -1):
            # Greedy search to find nearest neighbors in this layer
            nearest = greedy_search(vector, entry_point, layer, graph)

            # Connect to M nearest neighbors
            connections = select_connections(nearest, M, vector)
            graph[i][layer] = connections

            # Update reverse links (undirected graph)
            for neighbor in connections:
                graph[neighbor][layer].append(i)

        entry_point = min(entry_point, vector, layer=0)

    return graph
```

**Visualization:**
```
Layer 2 (sparsest):     0 ───────── 1

Layer 1 (medium):       0 ─── 2 ─── 1
                        │         │
                        3 ─────── 4

Layer 0 (densest):      0 ─ 5 ─ 2 ─ 6 ─ 1
                        │   │   │   │   │
                        7 ─ 8 ─ 3 ─ 9 ─ 4
                        │           │
                        10──────────11

Search path from 10 to 1:
Layer 0: 10 → ... (many hops)
Layer 1: 10 → 3 → 4 → 1 (fewer hops)
Layer 2: 10 → 0 → 1 (long-range shortcut!)
```

### Query Algorithm

```python
def search_hnsw(graph, query, entry_point, ef_search, k):
    """
    Search HNSW graph.

    ef_search: Size of candidate pool (higher = more accurate, slower)
    """
    # Start from entry point (top layer)
    current = entry_point

    # Search each layer from top to bottom
    for layer in range(max_layer, -1, -1):
        # Greedy search in this layer
        candidates = PriorityQueue()
        visited = {current}

        candidates.push(distance(query, current), current)

        while candidates:
            dist, node = candidates.pop()

            # Explore neighbors
            for neighbor in graph[node][layer]:
                if neighbor not in visited:
                    visited.add(neighbor)
                    new_dist = distance(query, neighbor)
                    candidates.push(new_dist, neighbor)

        # Move to closest found element
        current = candidates.closest()

    # Final layer (layer 0) gives us the result
    return get_top_k(candidates, k)
```

### HNSW Parameters

**M (Connection Degree):**
```
Higher M:
- More edges per node
- Better connectivity
- Higher memory usage
- Slower (more neighbors to check)

Typical: M = 16-64
```

**ef_search (Candidate Pool Size):**
```
Higher ef_search:
- Explore more candidates
- Higher recall
- Slower query time

Typical: ef_search = 50-200
```

**Tradeoff Curve:**
```
Recall@10 │
          │           *
          │        *
          │     *
          │  *
          │*
          └───────────────── QPS
           Fast    Slow

M=16, ef=50:  Fast, ~80% recall
M=32, ef=100: Medium, ~90% recall
M=64, ef=200: Slow, ~95% recall
```

### Memory Usage

```
Memory per vector:
- Vector data: D × 4 bytes (f32)
- Graph edges: M × log₂(N) × 4 bytes (edge indices)

Example: 1M vectors, 1024 dimensions, M=32
- Vector: 1024 × 4 = 4 KB
- Edges: 32 × 20 × 4 = 2.5 KB
- Total: ~6.5 KB per vector
- For 1M vectors: ~6.5 GB
```

---

## SPFresh (Turbopuffer's Foundation)

### Overview

SPFresh is a centroid-based ANN index that supports **incremental updates**, making it suitable for dynamic databases.

**Paper:** "SPFresh: Incremental In-Memory Search Engine for Billion-Scale Approximate Nearest Neighbor Search" (VLDB 2024)

### Key Innovations

**1. Centroid-Based Indexing:**
```
Unlike HNSW (graph-based), SPFresh uses centroids:
- Each cluster represented by its centroid (mean)
- Vectors assigned to nearest centroid
- Simpler structure, easier to update
```

**2. Incremental Updates:**
```
HNSW limitation: Adding vectors changes graph structure
SPFresh solution: Just update centroid and add to cluster

Insertion: O(D) to update centroid + O(1) to add vector
vs. HNSW: O(log N) graph traversal + edge updates
```

### SPFresh Architecture

```
SPFresh Tree Structure (multi-level):

                    ┌─────────────────┐
                    │  Root Centroid  │
                    │      C(0,0)     │
                    └────────┬────────┘
                             │
           ┌─────────────────┼─────────────────┐
           │                 │                 │
           ▼                 ▼                 ▼
    ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
    │  Centroid   │   │  Centroid   │   │  Centroid   │
    │   C(1,0)    │   │   C(1,1)    │   │   C(1,2)    │
    └──────┬──────┘   └──────┬──────┘   └──────┬──────┘
           │                 │                 │
    ┌──────┴──────┐   ┌──────┴──────┐   ┌──────┴──────┐
    │             │   │             │   │             │
    ▼             ▼   ▼             ▼   ▼             ▼
┌───────┐   ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│Leaf 0 │   │Leaf 1 │ │Leaf 2 │ │Leaf 3 │ │Leaf 4 │ │Leaf 5 │
│(100v) │   │(100v) │ │(100v) │ │(100v) │ │(100v) │ │(100v) │
└───────┘   └───────┘ └───────┘ └───────┘ └───────┘ └───────┘

Each leaf contains ~100 vectors
Internal nodes are centroids
```

### Tree Construction

```python
def build_spfresh_tree(vectors, branching_factor=100, leaf_size=100):
    """
    Build SPFresh tree recursively.

    branching_factor: Children per node (~100)
    leaf_size: Vectors per leaf (~100)
    """
    if len(vectors) <= leaf_size:
        # Base case: leaf node
        return LeafNode(vectors)

    # Recursive case: cluster and build subtrees
    n_clusters = min(branching_factor, len(vectors) // leaf_size)

    # Cluster vectors
    clusters = kmeans_cluster(vectors, n_clusters)

    # Build child nodes
    children = []
    centroids = []

    for cluster in clusters:
        child = build_spfresh_tree(cluster, branching_factor, leaf_size)
        children.append(child)
        centroids.append(centroid(cluster))

    return InternalNode(centroids, children)
```

### Query Algorithm

```python
def search_spfresh(tree, query, probes=5, top_k=10):
    """
    Search SPFresh tree.

    probes: Clusters to explore at each level
    """
    candidates = []

    # Navigate tree top-down
    nodes_to_explore = [(tree, 0)]  # (node, depth)

    while nodes_to_explore:
        node, depth = nodes_to_explore.pop(0)

        if isinstance(node, LeafNode):
            # Search all vectors in leaf
            for vector in node.vectors:
                dist = distance(query, vector)
                candidates.append((dist, vector))
        else:
            # Compare to centroids
            distances = [
                distance(query, c) for c in node.centroids
            ]

            # Select top-probes clusters
            top_indices = argsort(distances)[:probes]

            for idx in top_indices:
                nodes_to_explore.append((node.children[idx], depth + 1))

    # Return top-k results
    candidates.sort()
    return candidates[:top_k]
```

### Incremental Updates

```python
def insert_vector(tree, new_vector):
    """
    Insert a new vector into SPFresh tree.

    This is O(depth × branching_factor) for tree traversal
    + O(leaf_size) for centroid update.
    """
    # Navigate to appropriate leaf
    node = tree
    while not isinstance(node, LeafNode):
        # Find closest centroid
        distances = [distance(new_vector, c) for c in node.centroids]
        closest_idx = argmin(distances)
        node = node.children[closest_idx]

    # Add vector to leaf
    node.vectors.append(new_vector)

    # Update centroid incrementally
    node.centroid = (node.centroid * old_count + new_vector) / (old_count + 1)

    # Check if leaf needs splitting
    if len(node.vectors) > max_leaf_size:
        split_leaf(node)
```

### Why Turbopuffer Chose SPFresh

1. **Hierarchical Structure:** Bounded latency for cold queries
2. **Incremental Updates:** No need to rebuild entire index
3. **Memory Efficiency:** Centroids are smaller than HNSW edges
4. **Predictable Performance:** Tree depth bounds worst-case

---

## Binary Quantization (RaBitQ)

### Motivation

Even with efficient indexing, scanning thousands of vectors is slow. Binary quantization reduces vector size 16-32x.

### Basic Binary Quantization

```python
def quantize_binary(vector):
    """Convert f32 vector to binary."""
    return [1 if x > 0 else 0 for x in vector]

def hamming_distance(a, b):
    """Count differing bits."""
    return sum(x != y for x, y in zip(a, b))

def estimate_cosine(query_bin, data_bin):
    """Estimate cosine from binary vectors."""
    n = len(query_bin)
    matching = sum(x == y for x, y in zip(query_bin, data_bin))
    return (2 * matching / n) - 1
```

### RaBitQ: Quantization with Guarantees

**Key Insight from RaBitQ Paper:**

In high-dimensional spaces, quantization errors are uniformly distributed due to "concentration of measure."

**Error Bound Theorem:**
```
For vectors a, b ∈ ℝᵈ and their binary forms a', b':

|cosine(a, b) - estimate(a', b')| ≤ O(1/√d)

For d = 1024: error ≤ ~0.03 (3%)
```

**Practical Implication:**
```
Query Q, candidates D1, D2:

estimate(Q, D1) = 0.75 ± 0.03 → range [0.72, 0.78]
estimate(Q, D2) = 0.80 ± 0.03 → range [0.77, 0.83]

Overlap exists! Can't confidently rank.

estimate(Q, D3) = 0.90 ± 0.03 → range [0.87, 0.93]

No overlap with D1! D3 is definitely farther.

→ Only D1 and D2 need full-precision comparison
```

### RaBitQ Implementation

```python
import numpy as np

class RaBitQQuantizer:
    def __init__(self, dimension):
        self.d = dimension
        # Precomputed constants for error bounds
        self.error_constant = 1.0 / np.sqrt(dimension)

    def quantize(self, vector):
        """Quantize vector to binary + metadata."""
        # Basic binary quantization
        binary = (vector > 0).astype(np.uint8)

        # Compute norm for distance estimation
        norm = np.linalg.norm(vector)

        return {
            'binary': binary,
            'norm': norm,
        }

    def estimate_distance(self, q_quant, d_quant):
        """
        Estimate distance with error bounds.

        Returns: (estimated_distance, error_bound)
        """
        # Hamming distance
        hamming = np.sum(q_quant['binary'] != d_quant['binary'])

        # Convert to cosine estimate
        estimated_cosine = 1 - (2 * hamming / self.d)

        # Error bound (simplified; actual formula from paper is more complex)
        error_bound = self.error_constant

        return estimated_cosine, error_bound

    def needs_reranking(self, result1, result2, top_k_threshold):
        """
        Check if result1 needs reranking given result2.

        If confidence intervals overlap, we need full precision.
        """
        est1, err1 = result1
        est2, err2 = result2

        interval1 = (est1 - err1, est1 + err1)
        interval2 = (est2 - err2, est2 + err2)

        # Check overlap
        return not (interval1[1] < interval2[0] or interval2[1] < interval1[0])
```

### Two-Phase Search with RaBitQ

```python
def two_phase_search(index, query, quantizer, top_k=10):
    """
    Two-phase search: quantized filter + full-precision rerank.
    """
    # Phase 1: Quantized search
    query_quant = quantizer.quantize(query)

    candidates = []
    for vector_id in index.get_candidate_ids():
        data_quant = index.get_quantized(vector_id)
        est_dist, error = quantizer.estimate_distance(query_quant, data_quant)
        candidates.append((est_dist, error, vector_id))

    # Sort by estimated distance
    candidates.sort()

    # Find rerank threshold
    rerank_ids = set()
    for i, (est, err, vid) in enumerate(candidates):
        if i < top_k:
            # Definitely include top-k by estimate
            rerank_ids.add(vid)
        else:
            # Check if could be in true top-k
            best_possible = est - err  # Lower bound
            worst_topk = candidates[top_k-1][0] + candidates[top_k-1][1]  # Upper bound

            if best_possible < worst_topk:
                rerank_ids.add(vid)
            else:
                break  # No more candidates can beat top-k

    # Phase 2: Full-precision rerank
    final_results = []
    for vid in rerank_ids:
        full_vector = index.get_full_precision(vid)
        exact_dist = cosine_distance(query, full_vector)
        final_results.append((exact_dist, vid))

    final_results.sort()
    return final_results[:top_k]
```

---

## Algorithm Comparison

### Summary Table

| Algorithm | Build Time | Query Time | Memory | Incremental | Recall |
|-----------|------------|------------|--------|-------------|--------|
| Linear Scan | O(1) | O(ND) | O(N) | ✓ | 100% |
| IVF | O(N) | O(D√N) | O(N) | ✗ | 70-90% |
| HNSW | O(N log N) | O(log N) | O(NM) | △ | 85-95% |
| SPFresh | O(N log N) | O(log N) | O(N) | ✓ | 85-95% |
| SPFresh+RaBitQ | O(N log N) | O(log N) | O(N/16) | ✓ | 90-95% |

### When to Use Each

**Linear Scan:**
- N < 10,000 vectors
- Need perfect recall
- Simplicity is paramount

**IVF:**
- N = 10K - 1M vectors
- Memory constrained
- Can tolerate 70-90% recall

**HNSW:**
- N = 100K - 100M vectors
- Want best speed/accuracy tradeoff
- Can tolerate higher memory

**SPFresh (Turbopuffer):**
- N = 1M - 100B+ vectors
- Need incremental updates
- Want bounded latency
- Need hierarchical caching

---

## Implementation Guide

### Rust Crates for ANN

```toml
[dependencies]
# Distance computations
ndarray = "0.16"
ndarray-linalg = "0.17"

# HNSW implementation
hnsw = "0.11"
usearch = "2.16"  # USearch (fast HNSW)

# Clustering
linfa = "0.7"  # ML library with k-means
linfa-clustering = "0.7"

# Bit manipulation
bitvec = "1.0"

# Parallelism
rayon = "1.10"
```

### Example: HNSW in Rust

```rust
use hnsw::Hnsw;

fn main() {
    // Build index
    let mut index = Hnsw::new(1024, 16);  // dimension, M

    for (id, vector) in vectors.iter().enumerate() {
        index.insert(id as u32, vector);
    }

    // Search
    let results = index.search(&query, 10, 50);  // query, k, ef

    for result in results {
        println!("ID: {}, Distance: {}", result.id, result.distance);
    }
}
```

### Example: SPFresh-style Tree

```rust
struct CentroidTree {
    levels: Vec<Level>,
    dimension: usize,
    branching_factor: usize,
}

struct Level {
    centroids: Vec<Vec<f32>>,
    children: Vec<Vec<u32>>,  // Indices into next level
}

impl CentroidTree {
    fn build(vectors: &[Vec<f32>], bf: usize) -> Self {
        let mut levels = Vec::new();
        let mut current = vectors.to_vec();

        while current.len() > 100 {
            let clusters = kmeans(&current, bf.min(current.len()));
            let centroids: Vec<_> = clusters.iter().map(centroid).collect();
            let children: Vec<_> = clusters.iter()
                .map(|c| c.iter().map(|&i| i as u32).collect())
                .collect();

            levels.push(Level { centroids, children });
            current = centroids;
        }

        Self {
            levels,
            dimension: vectors[0].len(),
            branching_factor: bf,
        }
    }

    fn search(&self, query: &[f32], probes: usize) -> Vec<u32> {
        let mut candidates = vec![0];  // Start at root

        for level in &self.levels {
            let mut next = Vec::new();

            for &node_idx in &candidates {
                let children = &level.children[node_idx as usize];
                let child_centroids: Vec<_> = children.iter()
                    .map(|&c| &level.centroids[c as usize])
                    .collect();

                // Find closest children
                let mut dists: Vec<_> = child_centroids.iter()
                    .enumerate()
                    .map(|(i, c)| (cosine_dist(query, c), i))
                    .collect();

                dists.select_unstable(probes, |a, b| a.0.partial_cmp(&b.0).unwrap());

                for (_, i) in dists.iter().take(probes) {
                    next.push(children[*i]);
                }
            }

            candidates = next;
        }

        candidates
    }
}
```

---

## Summary

**Key Takeaways:**

1. **ANN is essential** for billion-scale vector search
2. **IVF** is simple but limited
3. **HNSW** offers best speed/accuracy but high memory
4. **SPFresh** (Turbopuffer's choice) supports incremental updates with hierarchical structure
5. **Binary quantization (RaBitQ)** enables 16-32x compression with theoretical guarantees
6. **Two-phase search** (quantized filter + full-precision rerank) achieves both speed and accuracy

**For Rust Implementation:**
- Start with `hnsw` crate for HNSW
- Use `ndarray` for vector operations
- Use `bitvec` for binary quantization
- Consider `memmap2` for memory-mapped storage
