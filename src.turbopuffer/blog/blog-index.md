# Turbopuffer Blog Index

This index provides detailed explanations of all 10 blog posts from turbopuffer.com/blog, with foundational knowledge added for each concept.

## Blog Posts Overview

| Post | Topic | Key Innovation |
|------|-------|----------------|
| [ANN v3](#ann-v3-deep-dive) | Approximate Nearest Neighbor Search v3 | Hierarchical clustering + binary quantization |
| [BM25 Latency Musings](#bm25-latency-musings) | Full-Text Search Performance | Latency analysis for BM25 at scale |
| [Continuous Recall](#continuous-recall) | Search Quality Measurement | Real-time recall tracking |
| [FTS v2](#fts-v2) | Full-Text Search v2 | Improved BM25 implementation |
| [FTS v2 MaxScore](#fts-v2-maxscore) | Query Optimization | MaxScore optimization for BM25 |
| [FTS v2 Postings](#fts-v2-postings) | Index Structure | Postings list optimization |
| [Native Filtering](#native-filtering) | Pre-filtering Support | Server-side attribute filtering |
| [Object Storage Queue](#object-storage-queue) | Distributed Systems | Queue implementation on S3/GCS |
| [Turbopuffer](#turbopuffer-intro) | Company/Product Intro | Overview of the platform |
| [Zero-Cost](#zero-cost-abstractions) | Rust Programming | Zero-cost abstractions in practice |

---

## ANN v3 Deep Dive

### Original Topic
The ANN v3 post discusses the third generation of Turbopuffer's approximate nearest neighbor search algorithm.

### Foundational Concepts

#### What is Approximate Nearest Neighbor (ANN) Search?

**Problem Statement:**
Given a query vector Q and a database of N vectors {V₁, V₂, ..., Vₙ}, find the k vectors closest to Q according to some distance metric.

**Why "Approximate"?**
- Exact search requires O(N) distance computations
- For billion-scale databases, this is too slow for real-time queries
- ANN trades perfect accuracy for 100-1000x speedup

**Example:**
```
Database: 100 billion vectors (each 1024-dimensional)
Query: Find 10 most similar vectors to Q

Exact search: 100 billion × 1024 operations = impractical
ANN search: ~50,000 operations with 95%+ recall = feasible
```

#### Distance Metrics Explained

**1. Cosine Distance:**
```
cosine_distance(A, B) = 1 - (A · B) / (||A|| × ||B||)

Where:
- A · B = dot product = Σ(Aᵢ × Bᵢ)
- ||A|| = magnitude of A = sqrt(ΣAᵢ²)

Range: [0, 2] where 0 = identical direction
Use case: Text embeddings (direction matters, not magnitude)
```

**2. L2 (Euclidean) Distance:**
```
l2_distance(A, B) = sqrt(Σ(Aᵢ - Bᵢ)²)

This is "straight line" distance in vector space
```

**3. Dot Product:**
```
dot_product(A, B) = Σ(Aᵢ × Bᵢ)

Use case: When vectors are already normalized
```

#### Memory Hierarchy and Why It Matters

**The Key Insight from ANN v3:**
Vector search is **bandwidth-bound**, not compute-bound. The bottleneck is moving data, not computing distances.

**Memory Hierarchy:**
```
Level              Size        Bandwidth     Latency
─────────────────────────────────────────────────────
CPU Registers      <1 KB       >10 TB/s      <1 ns
L1 Cache           ~32 KB      ~1 TB/s       ~1 ns
L2 Cache           ~256 KB     ~500 GB/s     ~3 ns
L3 Cache           ~32 MB      ~100 GB/s     ~10 ns
DRAM               ~128 GB     ~50 GB/s      ~100 ns
NVMe SSD           ~10 TB      ~7 GB/s       ~100 μs
Object Storage     Unlimited   ~1 GB/s       ~10-100 ms
```

**Bandwidth Gap:**
- Object storage is ~10,000x slower than L3 cache
- DRAM is ~10x slower than L3 cache
- Good cache utilization is critical

### Turbopuffer's ANN v3 Architecture

**Two Key Techniques:**

**1. Hierarchical Clustering:**
```
Tree Structure:
                    ┌───────────────┐
                    │ Root Centroid │
                    └───────┬───────┘
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ Centroid │ │ Centroid │ │ Centroid │
        └────┬─────┘ └────┬─────┘ └────┬─────┘
             │            │            │
        ┌────┴────┐  ┌────┴────┐  ┌────┴────┐
        ▼         ▼  ▼         ▼  ▼         ▼
     ┌─────┐  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐
     │Data │  │Data │ │Data │ │Data │ │Data │ │Data │
     │Vec 1│  │Vec 2│ │Vec 3│ │Vec 4│ │Vec 5│ │Vec 6│
     └─────┘  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘

Query Process:
1. Compare query to root centroid
2. Select top-k most similar child centroids
3. Recurse down the tree
4. At leaves, search data vectors in selected clusters
```

**Why 100x Branching Factor?**
- Matches the ~10-50x size ratio between DRAM and SSD
- All centroids fit in DRAM while data vectors stay on SSD
- Tree depth of 3-4 levels for billion-scale databases

**2. Binary Quantization (RaBitQ):**
```
Quantization Process:
Original: [0.94, -0.01, 0.39, -0.72, 0.02, -0.85, -0.18, 0.99]
                    │
                    ▼ (threshold at 0)
Quantized: [1,      0,    1,    0,    1,    0,    0,    1]

Compression Ratio:
- f32: 32 bits per dimension
- f16: 16 bits per dimension
- Binary: 1 bit per dimension
- Savings: 16-32x reduction!
```

**RaBitQ Error Bounds:**
```
For vectors Vd (data) and Vq (query):
- True distance: d = cosine_distance(Vd, Vq)
- Estimated distance: d' = distance_from_quantized(Vd', Vq')
- Error bound: |d - d'| ≤ ε

This means if d' = 0.75 and ε = 0.06:
- True distance is in range [0.69, 0.81]
- We can confidently say Vd is closer than another vector with d' = 0.90
```

**The Two-Phase Query Process:**
```
Phase 1: Quantized Search (FAST)
├── Scan all quantized vectors in candidate clusters
├── Compute distance estimates with error bounds
└── Identify vectors that could be in true top-k

Phase 2: Full-Precision Rerank (PRECISE)
├── Fetch full-precision vectors ONLY for rerank candidates (<1%)
├── Compute exact distances
└── Return final ranked results
```

### Performance Analysis

**Without Quantization:**
```
Per-level bandwidth: 100MB
SSD bandwidth: 10 GB/s
Max QPS = 10 GB/s ÷ 100 MB = 100 QPS (disk-bound)
```

**With Quantization:**
```
Per-level bandwidth: 6MB (16x smaller)
DRAM bandwidth: 300 GB/s
Max QPS = 300 GB/s ÷ 6 MB = 50,000 QPS (theoretical)

But reranking is still disk-bound:
1% of 100MB = 1MB per query
SSD bandwidth: 10 GB/s
Max QPS = 10 GB/s ÷ 1 MB = 10,000 QPS

Actual observed: ~1,000 QPS (compute-bound!)
```

**Why Compute-Bound?**
- Binary quantization increases arithmetic intensity 64x
- Each bit is reused 4 times in RaBitQ distance estimation
- CPU cores can't keep up with the compressed data rate
- Optimization focus shifts to instruction efficiency and SIMD

---

## BM25 Latency Musings

### Foundational Concepts

#### What is BM25?

BM25 (Best Matching 25) is a ranking function used in full-text search engines to estimate document relevance.

**The BM25 Formula:**
```
score(D, Q) = Σ IDF(qᵢ) × (f(qᵢ, D) × (k₁ + 1)) / (f(qᵢ, D) + k₁ × (1 - b + b × |D|/avgdl))

Where:
- D = document
- Q = query
- qᵢ = i-th term in query
- f(qᵢ, D) = frequency of term qᵢ in document D
- IDF(qᵢ) = inverse document frequency = log((N - n(qᵢ) + 0.5) / (n(qᵢ) + 0.5))
- N = total number of documents
- n(qᵢ) = number of documents containing qᵢ
- |D| = document length
- avgdl = average document length
- k₁, b = tuning parameters (typically k₁=1.2, b=0.75)
```

**Intuition:**
- Terms that appear frequently in a document but rarely in the corpus are more important
- Document length normalization prevents long documents from being favored

#### Posting Lists

**Inverted Index Structure:**
```
Dictionary (Terms) → Posting Lists → Documents

Term: "search"
└── Posting List: [(doc_id=1, positions=[2, 15]),
                   (doc_id=5, positions=[1]),
                   (doc_id=8, positions=[3, 7, 22])]

Term: "engine"
└── Posting List: [(doc_id=1, positions=[8]),
                   (doc_id=3, positions=[5, 11]),
                   (doc_id=8, positions=[4])]
```

### Latency Analysis

The blog post analyzes factors affecting BM25 query latency at scale.

**Key Latency Factors:**
1. **Posting List Size:** Longer lists = more data to process
2. **Term Frequency:** Common terms have longer posting lists
3. **Query Complexity:** More terms = more lists to merge
4. **Document Count:** Larger corpus = larger posting lists

**Optimization Strategies:**
- WAND (Weak AND) / MaxScore: Skip irrelevant documents
- Impact-ordered posting lists: Process most relevant first
- Compression: Varint encoding for posting lists

---

## Continuous Recall

### Foundational Concepts

#### What is Recall in Search?

**Recall@K:**
```
Recall@K = (Relevant results in top-K) / (Total relevant results)

Example:
- Query has 20 truly relevant documents in database
- Search returns 10 results (K=10)
- 8 of those 10 are relevant
- Recall@10 = 8/10 = 80%

Note: This measures "precision at K" which is often called recall@K
in ANN literature because we're checking against a "ground truth" set.
```

**Why Measure Recall Continuously?**
1. Index quality can degrade over time (especially with incremental updates)
2. Different query types may have different recall characteristics
3. Parameter changes affect recall vs. latency tradeoff

**Continuous Recall Monitoring:**
```
System Architecture:
┌─────────────────┐     ┌──────────────────┐
│  Query Traffic  │────▶│   Search Engine  │
└─────────────────┘     └────────┬─────────┘
                                 │
                                 ▼
                        ┌─────────────────┐
                        │  Recall Monitor │
                        │  - Sample queries
                        │  - Compare to ground truth
                        │  - Track recall over time
                        └─────────────────┘
```

---

## FTS v2 (Full-Text Search v2)

### Foundational Concepts

#### Evolution of Full-Text Search

**FTS v1:**
- Basic BM25 implementation
- Postings lists stored separately
- Linear scan for query evaluation

**FTS v2 Improvements:**
- Better index structure
- Optimized query evaluation
- Integration with vector search

**BM25 + Vector Search:**
```
Hybrid Search Combines:
1. Lexical matching (BM25) - finds exact term matches
2. Semantic matching (Vectors) - finds conceptually similar content

Combined scoring: final_score = α × bm25_score + (1-α) × vector_score
```

---

## FTS v2 MaxScore

### MaxScore Optimization Explained

**Problem:**
When evaluating a multi-term query, processing all posting lists fully is wasteful if we only need top-K results.

**MaxScore Solution:**
```
For each term, pre-compute:
max_score(term) = maximum possible contribution of this term

During query evaluation:
1. Track current minimum top-K score (threshold)
2. For each document, compute max possible score
3. Skip documents where max_possible_score < threshold

Example:
Query: "search engine optimization"

Term MaxScores:
- "search": 3.5
- "engine": 2.8
- "optimization": 4.2

If current top-10 threshold is 7.0:
- A document missing "optimization" can score at most 3.5 + 2.8 = 6.3
- We can safely skip this document!
```

**Algorithm:**
```
max_score_query(query, k):
    threshold = 0
    results = []

    for each term in query (ordered by max_score):
        if can_prune(term, threshold):
            continue

        for doc in posting_list(term):
            if max_possible_score(doc) < threshold:
                continue

            score = bm25_score(doc, query)
            update_top_k(results, doc, score, k)
            threshold = min_top_k_score(results)

    return results
```

---

## FTS v2 Postings

### Posting List Optimization

**Posting List Structure:**
```
Naive Format:
[doc_id_1, doc_id_2, doc_id_3, ...]

Compressed Format (Delta Encoding + Varint):
- Store deltas: [doc_id_1, doc_id_2-doc_id_1, doc_id_3-doc_id_2, ...]
- Varint encode: smaller numbers use fewer bytes

Example:
Original:    [100, 105, 108, 200, 201]
Deltas:      [100, 5, 3, 92, 1]
Varint bytes: [100=1byte, 5=1byte, 3=1byte, 92=1byte, 1=1byte] = 5 bytes
vs 20 bytes for 5×4-byte integers
```

**Impact-Ordered Posting Lists:**
```
Traditional: Documents ordered by ID
[1, 5, 8, 12, 15, 20, ...]

Impact-ordered: Documents ordered by term importance
[8 (high TF), 1 (high TF), 20 (med TF), 5 (low TF), ...]

Benefit: MaxScore can terminate early when top-K is found
```

---

## Native Filtering

### Pre-Filtering vs Post-Filtering

**Post-Filtering (Traditional):**
```
1. Run vector search → Get top 100 candidates
2. Apply filters (price < 50, category = "shoes")
3. May end up with only 2 results
4. Problem: Filters reduce effective recall
```

**Pre-Filtering (Native):**
```
1. During search, only consider vectors matching filters
2. Return top 10 from matching set
3. Better: All 10 results satisfy filters
```

**Implementation Approaches:**

**1. Bitset Filtering:**
```
Filter bitset: [1, 0, 1, 1, 0, 0, 1, ...]
               ↑  ↑  ↑  ↑     ↑  ↑
               │  │  │  │     │  └─ Vector 6 matches
               │  │  │  │     └──── Vector 4 doesn't match
               │  │  │  └────────── Vector 3 matches
               │  │  └───────────── Vector 2 matches
               │  └──────────────── Vector 1 doesn't match
               └─────────────────── Vector 0 matches

During search: AND with bitset before adding to candidates
```

**2. Partitioned Index:**
```
Partition by filter values:
- Electronics namespace
- Clothing namespace
- Home namespace

Search only relevant partitions
```

---

## Object Storage Queue

### Building Queues on S3/GCS

**Problem:**
Distributed systems need durable, scalable queues. Traditional queues (Redis, Kafka) require managing infrastructure.

**Object Storage Queue Design:**
```
S3 Bucket Structure:
s3://queue-bucket/
├── queue/
│   ├── messages/
│   │   ├── 2024-01-01-12-00-00-abc123  (message body)
│   │   ├── 2024-01-01-12-00-01-def456
│   │   └── ...
│   ├── inflight/
│   │   └── (messages being processed)
│   └── deleted/
│       └── (processed messages)
└── metadata/
    └── (queue state, visibility timeout, etc.)
```

**Key Operations:**

**Enqueue:**
```
PUT s3://queue-bucket/messages/{timestamp}-{uuid}
Body: {message payload}
```

**Dequeue:**
```
1. LIST s3://queue-bucket/messages/ (ordered by timestamp)
2. For first N messages:
   - Check if not in inflight
   - Check visibility timeout expired
3. MOVE message to inflight/
4. Update visibility timeout
5. Return message
```

**Delete:**
```
MOVE inflight/{message} → deleted/{message}
(Background process periodically cleans deleted/)
```

**Advantages:**
- Infinite scalability (S3 handles the storage scaling)
- Durability (S3 11 9's durability)
- No infrastructure to manage

**Challenges:**
- Higher latency than Redis (~100ms vs ~1ms)
- LIST operations have pagination limits
- Need to handle concurrent consumers

---

## Turbopuffer Introduction

### Company and Product Overview

**What is Turbopuffer?**
A managed vector search service that handles billion-scale similarity search with millisecond latency.

**Key Features:**
1. **Scale:** Up to 100+ billion vectors
2. **Latency:** p99 < 50ms for most queries
3. **Throughput:** 10,000+ QPS per deployment
4. **Features:**
   - Vector search (cosine, L2, dot product)
   - Full-text search (BM25)
   - Native filtering
   - Attribute storage

**Architecture Overview:**
```
┌─────────────────┐
│  API Gateway    │  (Authentication, rate limiting)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Query Engine   │  (ANN search, filtering, ranking)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Storage Layer  │  (Centroids in DRAM, vectors on SSD)
└─────────────────┘
```

---

## Zero-Cost Abstractions

### Rust Performance Patterns

**What are Zero-Cost Abstractions?**
High-level programming constructs that compile to the same machine code as low-level manual implementations.

**Example 1: Iterators**
```rust
// High-level iterator code
let sum: i32 = (1..1000)
    .filter(|x| x % 2 == 0)
    .map(|x| x * x)
    .sum();

// Compiles to same code as:
let mut sum = 0;
for i in 1..1000 {
    if i % 2 == 0 {
        sum += i * i;
    }
}
```

**Example 2: Option/Result**
```rust
// Rust's Option type
fn find_first_even(nums: &[i32]) -> Option<i32> {
    nums.iter().find(|&&x| x % 2 == 0)
}

// No runtime overhead vs. returning -1 for "not found"
// But safer (can't accidentally use -1 as valid result)
```

**Example 3: Generics with Monomorphization**
```rust
// Generic function
fn process<T: Processor>(items: Vec<T>) {
    for item in items {
        item.process();
    }
}

// Compiler generates specialized version for each T:
fn process_string_processor(items: Vec<StringProcessor>) { ... }
fn process_image_processor(items: Vec<ImageProcessor>) { ... }

// No virtual dispatch overhead (vs. Java interfaces)
```

**Applications in Turbopuffer:**

1. **Distance Functions:**
```rust
// Generic over distance metric
fn compute_distance<M: DistanceMetric>(a: &[f32], b: &[f32]) -> f32 {
    M::compute(a, b)
}

// Compiles to specialized SIMD code for each metric
```

2. **Quantization:**
```rust
// Zero-copy bit vector access
struct BitVector<'a> {
    data: &'a [u8],
    dimension: usize,
}

impl BitVector<'_> {
    fn get(&self, index: usize) -> bool {
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        (self.data[byte_idx] >> bit_idx) & 1 == 1
    }
}
```

---

## How to Use This Guide

1. **Start with ANN v3** if you want to understand vector search architecture
2. **Read FTS v2 posts** for full-text search implementation details
3. **Study Native Filtering** for pre-filtering strategies
4. **Review Zero-Cost** for Rust performance patterns

Each post builds on concepts from others, so feel free to jump between sections.
