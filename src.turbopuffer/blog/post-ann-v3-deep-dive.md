# ANN v3: Turbopuffer's Third-Generation Approximate Nearest Neighbor Search

## Introduction

This document provides a comprehensive explanation of Turbopuffer's ANN v3 algorithm, breaking down every concept from first principles. The original blog post announced the third generation of their approximate nearest neighbor search system, which achieved a 100x throughput improvement over previous versions.

**Key Achievement:** 10,000+ QPS on billion-scale vector search with high recall.

---

## Part 1: Understanding the Problem

### What is Vector Search?

Vector search is the problem of finding similar items in a database using vector representations.

**Real-World Examples:**

1. **Image Search:**
   ```
   User uploads a photo of red boots
   → Convert to 768-dimensional vector using CLIP
   → Find vectors closest to this query
   → Return images of similar red boots
   ```

2. **Document Search:**
   ```
   User searches "how to debug python async code"
   → Convert query to embedding vector
   → Find document vectors nearby
   → Return most relevant documentation
   ```

3. **Recommendation Systems:**
   ```
   User watched "Inception" and "Interstellar"
   → Create user preference vector
   → Find movie vectors nearby
   → Recommend "Tenet" and "The Prestige"
   ```

### The Scale Problem

**Naive Approach (Exact Search):**
```python
def find_nearest_neighbors(query, database, k):
    distances = []
    for vector in database:  # O(N) iterations
        dist = cosine_distance(query, vector)  # O(D) operations
        distances.append((dist, vector))
    distances.sort()  # O(N log N)
    return distances[:k]
```

**Complexity Analysis:**
- N = number of vectors in database
- D = dimensionality of vectors
- Time complexity: O(N × D + N log N)

**For 100 billion vectors at 1024 dimensions:**
```
Operations per query = 100,000,000,000 × 1024 = 102 trillion operations

At 1 billion operations/second (optimistic):
Time per query = 102 seconds

This is obviously not acceptable for real-time search!
```

### Approximate Nearest Neighbor (ANN)

**Definition:**
ANN algorithms find "good enough" nearest neighbors much faster than exact search, trading perfect accuracy for speed.

**Recall vs. Latency Tradeoff:**
```
Recall@10 = (Correct results in top-10) / 10

High Recall (99%+) → More computation → Higher latency
Low Latency → Fewer checks → Lower recall

Goal: 95%+ recall at millisecond latency
```

---

## Part 2: Memory Hierarchy Fundamentals

### Why Memory Hierarchy Matters

The key insight of ANN v3 is that **vector search is bandwidth-bound, not compute-bound**. This means the bottleneck is moving data to the CPU, not the actual distance calculations.

**Understanding Memory Hierarchy:**

```
┌────────────────────────────────────────────────────────────┐
│                     CPU Package                            │
│  ┌──────────────┐                                          │
│  │   Cores      │  Registers: <1 KB, >10 TB/s, <1 ns      │
│  │  ┌────┐      │                                          │
│  │  │ ALU│      │  L1 Cache: ~32 KB, ~1 TB/s, ~1 ns       │
│  │  └────┘      │                                          │
│  │   ┌───┐      │  L2 Cache: ~256 KB, ~500 GB/s, ~3 ns    │
│  │   │ALU│      │                                          │
│  │   └───┘      │  L3 Cache: ~32 MB, ~100 GB/s, ~10 ns    │
│  └──────────────┘                                          │
└────────────────────────────────────────────────────────────┘
                         │
                         │ ~50 GB/s, ~100 ns
                         ▼
              ┌──────────────────┐
              │      DRAM        │  64-512 GB, ~50 GB/s
              │  (Main Memory)   │
              └──────────────────┘
                         │
                         │ ~7 GB/s, ~100 μs
                         ▼
              ┌──────────────────┐
              │    NVMe SSD      │  1-10 TB, ~7 GB/s
              └──────────────────┘
                         │
                         │ ~1 GB/s, ~10-100 ms
                         ▼
              ┌──────────────────┐
              │ Object Storage   │  Unlimited, ~1 GB/s
              │  (S3, GCS, etc.) │
              └──────────────────┘
```

**Bandwidth Gaps:**
- L3 → DRAM: ~2x slower
- DRAM → SSD: ~7x slower
- SSD → Object Storage: ~7x slower
- L3 → Object Storage: ~100x slower

**The Key Question:**
How do we structure our data and algorithms to minimize data movement across these bandwidth boundaries?

### Arithmetic Intensity

**Definition:**
Arithmetic intensity = (Floating-point operations) / (Bytes of memory transferred)

**For Cosine Distance:**
```
cosine_distance(A, B) where A, B are D-dimensional:

Numerator (dot product): D multiply-adds = 2D FLOPs
Denominator (memory): 2D floats × 4 bytes = 8D bytes

Arithmetic intensity = 2D / 8D = 0.25 FLOPs/byte

This is VERY LOW! Modern CPUs need ~10+ FLOPs/byte to be compute-bound.
```

**Conclusion:**
With such low arithmetic intensity, vector search will be **memory bandwidth bound**. The system can only go as fast as it can fetch vectors from memory.

---

## Part 3: Hierarchical Clustering

### The Core Idea

Instead of comparing the query to all N vectors, we:
1. Group vectors into clusters
2. Represent each cluster by its centroid (mean)
3. First compare to centroids to find promising clusters
4. Only search inside those clusters

**Analogy:**
Finding a book in a library:
- Don't look at every book (exact search)
- Find the right section first (compare to centroids)
- Then search within that section (search cluster)

### Tree Structure

```
Level 0 (Root):                    ┌─────────────┐
                                   │ Centroid C0 │
                                   └──────┬──────┘
                                          │
                    ┌─────────────────────┼─────────────────────┐
                    │                     │                     │
                    ▼                     ▼                     ▼
Level 1:    ┌─────────────┐       ┌─────────────┐       ┌─────────────┐
            │ Centroid C1 │       │ Centroid C2 │       │ Centroid C3 │
            └──────┬──────┘       └──────┬──────┘       └──────┬──────┘
                   │                     │                     │
         ┌─────────┴──────┐    ┌─────────┴──────┐    ┌─────────┴──────┐
         │                │    │                │    │                │
         ▼                ▼    ▼                ▼    ▼                ▼
Level 2: ┌────┐ ┌────┐   ┌────┐ ┌────┐   ┌────┐ ┌────┐   ┌────┐ ┌────┐
         │ v1 │ │ v2 │   │ v3 │ │ v4 │   │ v5 │ │ v6 │   │ v7 │ │ v8 │
         └────┘ └────┘   └────┘ └────┘   └────┘ └────┘   └────┘ └────┘
         (Data vectors at leaves)
```

### Why 100x Branching Factor?

Turbopuffer uses approximately 100 children per centroid. This is not arbitrary—it matches the hardware.

**Math:**
```
Centroid size: 1024 dimensions × 4 bytes (f32) = 4 KB per centroid

Level 1: 100 centroids = 400 KB (fits in L2 cache)
Level 2: 100² = 10,000 centroids = 40 MB (fits in L3 cache / DRAM boundary)
Level 3: 100³ = 1,000,000 centroids = 4 GB (fits in DRAM)

Data clusters: 100⁴ = 100,000,000 vectors at leaves
```

**The Magic:**
- All centroids up to level 3 fit in DRAM
- This means we can search 100 million vectors with only DRAM access for centroids
- Data vectors themselves are on SSD, but we only fetch a small fraction

### Query Algorithm

```python
def search_tree(query, root, probes=5):
    """
    Find candidate clusters by traversing the centroid tree.

    probes: Number of child centroids to explore at each level
    """
    candidates = []

    # Level 0: Start at root
    current_level = [root]

    for level in range(tree_depth):
        next_level = []

        for node in current_level:
            # Compare query to all children
            distances = []
            for child in node.children:
                dist = cosine_distance(query, child.centroid)
                distances.append((dist, child))

            # Take top-k most similar
            distances.sort()
            next_level.extend([child for _, child in distances[:probes]])

        current_level = next_level

    # Return data vectors in final candidate clusters
    for node in current_level:
        candidates.extend(node.data_vectors)

    return candidates
```

### Bounding Cold Query Latency

**Cold Query:** A query where no data is cached (everything must be fetched from object storage).

**Without Hierarchy (Graph-based):**
```
Graph traversal may require:
- Fetch node A → check neighbors
- Fetch neighbor B → not promising
- Fetch neighbor C → not promising
- ... potentially many round-trips

Worst case: O(depth) round-trips to object storage
Each round-trip: 10-100 ms
Total latency: Can be unbounded
```

**With Hierarchy:**
```
Tree traversal requires exactly:
- Fetch root level (always in memory)
- Fetch level 1 centroids: ~400 KB
- Fetch level 2 centroids: ~40 MB
- Fetch level 3 centroids: ~4 GB (but can be selective)
- Fetch data vectors from selected clusters

Maximum round-trips = tree height (typically 3-4)
```

---

## Part 4: Binary Quantization (RaBitQ)

### The Problem with Full Precision

Even with hierarchical clustering, we need to scan ~50,000 data vectors at the leaf level.

**Bandwidth Calculation:**
```
500 clusters × 100 vectors/cluster × 1024 dimensions × 4 bytes (f32)
= 200 MB per query

At 10 GB/s SSD bandwidth:
Max QPS = 10 GB/s ÷ 200 MB = 50 QPS
```

This is still too slow for the target throughput.

### Binary Quantization Explained

**Idea:**
Store each vector in two forms:
1. Full precision (f32 or f16) - for accurate distance calculation
2. Binary (1 bit per dimension) - for fast filtering

**Quantization Process:**
```python
def quantize(vector):
    """Convert f32 vector to binary."""
    return [1 if x > 0 else 0 for x in vector]

def dequantize(binary, scale=1.0):
    """Approximate original from binary (conceptual)."""
    return [b * scale for b in binary]
```

**Compression Ratio:**
```
f32: 32 bits per dimension
f16: 16 bits per dimension
Binary: 1 bit per dimension

Compression: 16-32x reduction!
```

### RaBitQ: Quantization with Guarantees

**The Challenge:**
Naive binary quantization loses too much accuracy. How can we quantify the error?

**RaBitQ Insight:**
In high-dimensional spaces, vector components become uniformly distributed (concentration of measure). This means quantization errors spread evenly across dimensions, allowing tight error bounds.

**Error Bound Formula:**
```
For vectors Vd (data) and Vq (query):

Let d = true cosine_distance(Vd, Vq)
Let d' = estimated distance from quantized vectors

RaBitQ guarantees: |d - d'| ≤ ε

Where ε depends on:
- Vector dimension (higher = tighter bounds)
- Quantization method parameters
```

**Practical Example:**
```
Query Q, Data vectors D1, D2, D3

Quantized distances:
- d'(Q, D1) = 0.50 ± 0.06 → range [0.44, 0.56]
- d'(Q, D2) = 0.70 ± 0.06 → range [0.64, 0.76]
- d'(Q, D3) = 0.52 ± 0.06 → range [0.46, 0.58]

We can conclude:
- D2 is definitely farther than D1 and D3 (ranges don't overlap)
- D1 vs D3 is inconclusive (ranges overlap)

→ Only need full-precision comparison for D1 and D3!
```

### Two-Phase Query Processing

```
Phase 1: Quantized Search (FAST)
├── Scan quantized vectors in candidate clusters
├── Compute distance estimates with error bounds
├── Maintain candidate set with overlap analysis
└── Output: Set of vectors needing reranking

Phase 2: Full-Precision Rerank (PRECISE)
├── Scatter-gather fetch full-precision vectors
├── Compute exact distances
├── Sort and return top-k
└── Output: Final ranked results
```

**Key Metric:**
Only <1% of vectors need full-precision reranking!

---

## Part 5: Putting It All Together

### Complete Query Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                         QUERY ARRIVES                            │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 1: Compare to Root Centroid                                 │
│ - Root always in memory                                          │
│ - Time: <1 μs                                                    │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 2: Traverse Tree (Levels 1-3)                               │
│ - Compare to ~100 children at each level                         │
│ - Select top-5 most similar                                      │
│ - Centroids in DRAM → ~300 GB/s bandwidth                        │
│ - Time: ~100 μs                                                  │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 3: Scan Quantized Vectors                                   │
│ - Search 500 clusters × 100 vectors = 50,000 vectors             │
│ - Binary vectors in DRAM → fast bitwise operations               │
│ - Compute estimates with error bounds                            │
│ - Time: ~500 μs                                                  │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 4: Identify Rerank Candidates                               │
│ - Vectors with overlapping confidence intervals                  │
│ - Typically <1% = ~500 vectors                                   │
│ - Time: ~50 μs                                                   │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 5: Scatter-Gather Full Precision                            │
│ - Random read ~500 vectors from SSD                              │
│ - NVMe excels at parallel random reads                           │
│ - Fetch ~2 MB total                                              │
│ - Time: ~200 μs                                                  │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────┐
│ Step 6: Final Rerank                                             │
│ - Compute exact distances for ~500 vectors                       │
│ - Sort and return top-10                                         │
│ - Time: ~50 μs                                                   │
└───────────────────────────┬──────────────────────────────────────┘
                            │
                            ▼
                         RESULT
                    Total: ~1 ms
```

### Throughput Analysis

**With Hierarchical Clustering Only:**
```
Per-level data: 500 clusters × 100 vectors × 1024 dims × 4 bytes = 200 MB

DRAM bandwidth (300 GB/s): 300 / 200 = 1.5 levels per query
SSD bandwidth (10 GB/s): 10 / 200 = 0.05 queries per second per machine

Wait, this can't be right...

Actually, for hierarchical clustering without quantization:
- Centroids in DRAM: ~100 MB per query (3 levels × 100 MB)
- Data vectors from SSD: ~200 MB per query

Max QPS (SSD-bound) = 10 GB/s / 200 MB = 50 QPS
```

**With Binary Quantization:**
```
Quantized per-level: 500 × 100 × 1024 × (1/8) bytes = 6.25 MB

Level 1-3 (quantized centroids in L3):
L3 bandwidth (600 GB/s): 600 / (3 × 6.25) = 32,000 QPS

Data vectors (quantized in DRAM):
DRAM bandwidth (300 GB/s): 300 / 6.25 = 48,000 QPS

Reranking (1% of full-precision from SSD):
SSD bandwidth (10 GB/s): 10 / (0.01 × 200 MB) = 5,000 QPS

Bottleneck: SSD reranking at ~5,000-10,000 QPS
```

**Actual Observed: ~1,000 QPS**

**Why the Gap?**
The system becomes **compute-bound** after quantization!

Binary quantization increases arithmetic intensity:
- Each bit is reused 4 times in RaBitQ distance estimation
- 64x higher arithmetic intensity than f32
- CPU can't keep up with the compressed data rate

**Solution:** SIMD vectorization and instruction optimization

---

## Part 6: Key Takeaways

### Design Principles

1. **Respect the Memory Hierarchy:**
   - Don't fight the hardware—work with it
   - Size data structures to fit at each level
   - Minimize cross-boundary transfers

2. **Approximation + Refinement:**
   - Fast approximate filtering first
   - Precise computation only for candidates
   - Error bounds make this rigorous

3. **Compression Enables Caching:**
   - Smaller data = higher cache levels
   - Higher cache levels = more bandwidth
   - More bandwidth = higher throughput

### Performance Summary

| Configuration | QPS | Bottleneck |
|--------------|-----|------------|
| Exact search | <1 | Everything |
| Hierarchical only | ~100 | SSD bandwidth |
| Hierarchical + Quantization | ~10,000 | Compute |
| Optimized (SIMD, etc.) | ~10,000+ | Compute |

### Lessons for System Design

1. **Measure Before Optimizing:**
   - Identify the true bottleneck (bandwidth vs. compute)
   - Optimize the bottleneck, not everything

2. **Understand Your Data:**
   - Can it be compressed?
   - What are the access patterns?
   - What approximations are acceptable?

3. **Leverage Hardware:**
   - Cache sizes dictate data structure sizes
   - Bandwidth ratios dictate algorithm choices
   - SIMD capabilities dictate compute optimizations

---

## Exercises for the Reader

1. **Calculate Tree Depth:**
   For 1 billion vectors with 100-way branching, how many levels does the tree have?

2. **Estimate Memory:**
   How much DRAM is needed for centroids with 1 billion vectors, 1024 dimensions, f32?

3. **Quantization Practice:**
   Implement binary quantization and measure the recall impact on a small dataset.

4. **Bandwidth Analysis:**
   For your use case, calculate the expected QPS given your hardware specs.

---

## Further Reading

1. **SPFresh Paper:** https://dl.acm.org/doi/10.1145/3600006.3613166
2. **RaBitQ Paper:** https://dl.acm.org/doi/pdf/10.1145/3654970
3. **Concentration of Measure:** https://en.wikipedia.org/wiki/Concentration_of_measure
4. **HNSW:** https://arxiv.org/abs/1603.09320 (alternative ANN algorithm)
