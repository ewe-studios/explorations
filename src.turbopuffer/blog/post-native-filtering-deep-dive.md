# Native Filtering for High-Recall Vector Search - Deep Dive

## Executive Summary

Vector search with attribute filters (WHERE clauses) is a common production requirement. However, most vector indexes suffer severe recall degradation when filters are applied. This post explains why filtered vector search is fundamentally different from "vector search + filters" and how turbopuffer achieves native filtering with >90% recall.

---

## The Problem: Filters Break Vector Search

### The Use Case

```
Search over a codebase:
- Find similar code snippets
- BUT only within foo/src/* directory
- OR only for files modified in last 30 days
- OR only for specific programming language

Query:
{
  "vector": [/* embedding of query */],
  "filter": ["path", "StartsWith", "foo/src/"],
  "top_k": 10
}
```

### The Challenge Illustrated

```
2D Vector Space Visualization:

┌─────────────────────────────────────────────────────────────┐
│                                                             │
│    x  x       x                                             │
│  x    x          x                                          │
│     x   x       x    x                                      │  Legend:
│  x        Q   x        x                                    │  Q = Query vector
│    x   x    x     x   x                                     │  x = Unfiltered document
│         x    x   x                                          │  + = Matches filter
│  x   x     x      x   +   ++                                │  - = Doesn't match filter
│     x     x   x        +  + +                               │
│  x   x   x       x      ++++                                │
│    x   x    x   x         +                                 │
│       x    x   x                                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Problem: The 10 nearest neighbors to Q are ALL 'x' (unfiltered).
None of them match the filter (+).

If we search first, then filter: 0 results (0% recall)
If we filter first, then search: Need to scan entire database
```

---

## Why Pre-Filtering and Post-Filtering Fail

### Post-Filtering (Search Then Filter)

```
Algorithm:
1. Find top-k nearest vectors to query
2. Apply filter to results
3. Return filtered results

Problem:
┌─────────────────────────────────────────────────────────────┐
│  Query: "similar Python functions in /src/"                │
│                                                             │
│  Step 1: Vector search returns:                            │
│    - /docs/example.py (distance: 0.1) ✓                    │
│    - /tests/test1.py (distance: 0.15) ✓                    │
│    - /vendor/lib.py (distance: 0.2) ✓                      │
│    - /src/main.py (distance: 0.25) ✓ ← ONLY MATCH          │
│    - /src/utils.py (distance: 0.3) ✓ ← ONLY MATCH          │
│    - /docs/guide.md (distance: 0.35) ✓                     │
│    - ... (all 10 results)                                  │
│                                                             │
│  Step 2: Apply filter ["path", "StartsWith", "/src/"]      │
│                                                             │
│  Step 3: Results after filter:                             │
│    - /src/main.py                                          │
│    - /src/utils.py                                         │
│                                                             │
│  Only 2 results returned (wanted 10!)                      │
│  Recall: 2/10 = 20%                                        │
└─────────────────────────────────────────────────────────────┘

Fundamental Issue:
The vector index doesn't know about the filter, so it returns
the globally nearest vectors, not the nearest FILTERED vectors.
```

### Pre-Filtering (Filter Then Search)

```
Algorithm:
1. Find all documents matching the filter
2. Compute distance to each matching document
3. Return top-k nearest

Problem:
┌─────────────────────────────────────────────────────────────┐
│  Dataset: 1 billion vectors                                 │
│  Filter match: 10 million documents (1%)                    │
│                                                             │
│  Step 1: Find 10M matching documents ✓                     │
│                                                             │
│  Step 2: Compute distances to ALL 10M documents            │
│          - 10M × 1024 dimensions = 10 billion FLOPs        │
│          - Latency: O(matches × dimensions)                │
│          - Time: ~10 seconds per query                     │
│                                                             │
│  Step 3: Sort and return top-k ✓                           │
│                                                             │
│  Result: 100% recall, but WAY too slow!                    │
└─────────────────────────────────────────────────────────────┘

Fundamental Issue:
Pre-filtering bypasses the vector index entirely,
requiring exhaustive distance computation.
```

---

## Native Filtering: The Solution

### Core Insight

```
The vector index and filter index must COOPERATE.

Traditional approach:
┌────────────────┐    ┌────────────────┐
│  Vector Index  │    │  Filter Index  │
│  (independent) │    │  (independent) │
└────────┬───────┘    └───────┬────────┘
         │                    │
         └──────┬─────────────┘
                │
         Query planner picks one
         (both options are bad)

Native filtering approach:
┌────────────────────────────────────────┐
│      Integrated Vector+Filter Index    │
│                                        │
│  - Clusters know about filter values  │
│  - Filter bitmaps guide vector search │
│  - Single index serves both needs     │
└────────────────────────────────────────┘
```

### Turbopuffer's Architecture

```
Clustering-Based Index with Filter Integration:

┌─────────────────────────────────────────────────────────────┐
│                    Centroid Tree                             │
│                   (stored in DRAM)                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│                         Root                                │
│                    Centroid: [0.1, ...]                     │
│             Filter stats: {path: {/src/: 50%, /docs/: 50%}} │
│                          │                                  │
│           ┌──────────────┼──────────────┐                  │
│           ▼              ▼              ▼                   │
│     ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│     │Cluster A │  │Cluster B │  │Cluster C │              │
│     │[0.05,..] │  │[0.15,..] │  │[0.12,..] │              │
│     │/src/: 90%│  │/src/: 10%│  │/src/: 5% │              │
│     └────┬─────┘  └────┬─────┘  └────┬─────┘              │
│          │             │             │                      │
│          ▼             ▼             ▼                      │
│     ┌────────┐   ┌────────┐   ┌────────┐                  │
│     │ Vectors│   │ Vectors│   │ Vectors│                  │
│     │ +filters│  │ +filters│  │ +filters│                 │
│     │ (SSD)  │   │ (SSD)  │   │ (SSD)  │                  │
│     └────────┘   └────────┘   └────────┘                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Key innovation: Each cluster tracks filter value distribution!
```

---

## Query Execution with Native Filtering

### Step 1: Filter-Aware Cluster Selection

```rust
fn select_clusters_for_query(
    query: &Query,
    filter: &Filter,
) -> Vec<ClusterRef> {
    let mut clusters = Vec::new();

    // For each cluster in the centroid tree:
    for cluster in &self.centroid_tree {
        // Check filter selectivity in this cluster
        let filter_selectivity = cluster.filter_stats
            .get_selectivity(filter);

        // Skip clusters with no matching documents
        if filter_selectivity == 0.0 {
            continue; // PRUNE this branch!
        }

        // Compute distance to centroid
        let distance = cosine_distance(&query.vector, &cluster.centroid);

        // Adjust score based on filter selectivity
        // (clusters with more matches get priority)
        let adjusted_score = distance * filter_selectivity;

        clusters.push(ClusterRef {
            id: cluster.id,
            score: adjusted_score,
            expected_matches: cluster.size * filter_selectivity,
        });
    }

    // Return top-k clusters by adjusted score
    clusters.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    clusters.truncate(self.search_params.num_clusters);
    clusters
}
```

### Step 2: Filtered Vector Search

```rust
fn search_cluster_filtered(
    cluster: &Cluster,
    query: &[f32],
    filter: &Filter,
    top_k: usize,
) -> Vec<SearchResult> {
    // 1. Load filter bitmap for this cluster
    //    (which documents in this cluster match the filter?)
    let filter_bitmap = cluster.get_filter_bitmap(filter);

    // 2. Search vectors, but only consider matching documents
    let mut results = Vec::new();

    for (idx, vector) in cluster.vectors.iter() {
        // Skip if doesn't match filter
        if !filter_bitmap.get(idx) {
            continue;
        }

        // Compute distance
        let distance = cosine_distance(query, vector);

        // Add to results
        results.push(SearchResult {
            doc_id: cluster.doc_ids[idx],
            distance,
            metadata: cluster.metadata[idx].clone(),
        });
    }

    // 3. Return top-k
    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap());
    results.truncate(top_k);
    results
}
```

### Step 3: Bitmap Optimization

```rust
/// Efficient filter bitmap representation
struct FilterBitmap {
    /// Compressed bitmap (Roaring-style)
    data: Vec<u64>,

    /// Number of set bits (for quick selectivity check)
    cardinality: usize,
}

impl FilterBitmap {
    /// Check if document matches filter - O(1)
    fn get(&self, idx: usize) -> bool {
        let word_idx = idx / 64;
        let bit_idx = idx % 64;

        if word_idx >= self.data.len() {
            return false;
        }

        (self.data[word_idx] >> bit_idx) & 1 == 1
    }

    /// Compute intersection of multiple bitmaps - SIMD optimized
    fn intersect(bitmaps: &[&FilterBitmap]) -> FilterBitmap {
        let mut result = FilterBitmap {
            data: vec![u64::MAX; bitmaps[0].data.len()],
            cardinality: 0,
        };

        // Process 64 bits at a time
        for i in 0..result.data.len() {
            let mut word = u64::MAX;
            for bitmap in bitmaps {
                word &= bitmap.data[i];
            }
            result.data[i] = word;
            result.cardinality += word.count_ones() as usize;
        }

        result
    }
}
```

---

## Why This Works

### High Recall Guarantee

```
Traditional Post-Filtering:
- Recall depends on filter distribution
- Can be 0% if filters are skewed
- No way to detect or compensate

Native Filtering:
- Only searches clusters with matching documents
- Expands search to more clusters if needed
- Recall bounded by cluster selection quality (>90% typical)
```

### Performance Characteristics

```
Query: "Find similar vectors WHERE path LIKE '/src/%'"

Native Filtering:
┌─────────────────────────────────────────────────────────────┐
│  Total vectors: 1 billion                                   │
│  Filter matches: 10 million (1%)                            │
│  Clusters: 10,000 (100K vectors each)                       │
│                                                             │
│  Step 1: Filter-aware cluster selection                    │
│          - Scan 10K centroids (in DRAM)                    │
│          - Prune clusters with 0% selectivity              │
│          - Select ~100 clusters to search                  │
│          - Time: ~5ms                                      │
│                                                             │
│  Step 2: Filtered search in selected clusters              │
│          - Search 100 clusters × 100K vectors              │
│          - But only 10% match filter (1M vectors)          │
│          - Bitmap filtering is O(1) per vector             │
│          - Time: ~20ms                                     │
│                                                             │
│  Total: ~25ms with >90% recall                             │
└─────────────────────────────────────────────────────────────┘

vs. Pre-Filtering: ~10 seconds
vs. Post-Filtering: 20% recall
```

---

## Filter Types and Support

### Supported Filter Operations

```rust
enum Filter {
    /// Exact match: path = "/src/main.py"
    Equals { attribute: String, value: Value },

    /// Not equal: path != "/src/main.py"
    NotEquals { attribute: String, value: Value },

    /// Contains (for arrays): tags Contains "python"
    Contains { attribute: String, value: Value },

    /// Contains any (for arrays): tags ContainsAny ["python", "rust"]
    ContainsAny { attribute: String, values: Vec<Value> },

    /// Range: created_at > 1000000
    GreaterThan { attribute: String, value: Value },

    /// Range: created_at < 2000000
    LessThan { attribute: String, value: Value },

    /// Prefix match: path StartsWith "/src/"
    StartsWith { attribute: String, prefix: String },

    /// Compound: (path LIKE "/src/%") AND (lang = "python")
    And { filters: Vec<Filter> },

    /// Compound: (path LIKE "/src/%") OR (path LIKE "/lib/%")
    Or { filters: Vec<Filter> },
}
```

### Filter Statistics per Cluster

```rust
/// Statistics maintained for each cluster
struct ClusterFilterStats {
    /// For exact match filters
    exact_counts: HashMap<String, HashMap<Value, usize>>,

    /// For range filters
    min_max: HashMap<String, (Value, Value)>,

    /// For prefix filters
    prefix_counts: HashMap<String, HashMap<String, usize>>,

    /// Sample values (for estimation)
    samples: HashMap<String, Vec<Value>>,
}

impl ClusterFilterStats {
    /// Estimate selectivity of a filter
    fn get_selectivity(&self, filter: &Filter) -> f32 {
        match filter {
            Filter::Equals { attribute, value } => {
                let total = self.total_docs();
                let matches = self.exact_counts
                    .get(attribute)
                    .and_then(|m| m.get(value))
                    .copied()
                    .unwrap_or(0);
                matches as f32 / total as f32
            }

            Filter::GreaterThan { attribute, value } => {
                // Estimate from min/max and samples
                self.estimate_range_selectivity(attribute, value, Bound::Greater)
            }

            Filter::StartsWith { attribute, prefix } => {
                // Use prefix counts if available
                self.prefix_counts
                    .get(attribute)
                    .and_then(|m| {
                        m.iter()
                            .filter(|(k, _)| k.starts_with(prefix))
                            .map(|(_, v)| v)
                            .sum::<usize>()
                    })
                    .unwrap_or(0) as f32 / self.total_docs() as f32
            }

            // ... other filter types
        }
    }
}
```

---

## Handling Filter Updates

### The Challenge

```
When documents are added/updated/deleted:
- Vector index must update
- Filter statistics must update
- Filter bitmaps must update

All while maintaining:
- Consistency (reads see valid state)
- Performance (updates don't block queries)
- Durability (updates survive crashes)
```

### Turbopuffer's Solution: LSM-Based Updates

```
┌─────────────────────────────────────────────────────────────┐
│                LSM Tree for Filter Index                    │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  MemTable (in-memory)                                       │
│  ┌───────────────────────────────────────────────┐         │
│  │ /src/main.py → {cluster: 5, idx: 100, ...}    │         │
│  │ /src/utils.py → {cluster: 3, idx: 50, ...}    │         │
│  │ /docs/readme.md → {cluster: 8, idx: 25, ...}  │         │
│  └───────────────────────────────────────────────┘         │
│                          │ flush                            │
│                          ▼                                  │
│  SSTable (object storage)                                   │
│  ┌───────────────────────────────────────────────┐         │
│  │ Sorted, immutable filter index data           │         │
│  │ - Cluster filter stats updates                │         │
│  │ - Bitmap deltas (additions/removals)          │         │
│  └───────────────────────────────────────────────┘         │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Compaction merges filter stats updates:
- Old: /src/ = 50%
- Delta: +100 docs to /src/
- New: /src/ = 55%
```

---

## Performance Evaluation

### Benchmark Setup

```
Dataset: 100 million code snippets (1024-dim embeddings)
Filters: path (string), language (categorical), created_at (timestamp)
Query: Find similar code WITH path filter

Baselines:
- Post-Filter: Standard ANN search, filter results
- Pre-Filter: Filter first, brute-force search matches
- Native: Turbopuffer's integrated approach
```

### Results

```
┌─────────────────────────────────────────────────────────────┐
│           Latency vs. Recall Comparison                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Filter Selectivity: 1% (1M matches out of 100M)           │
│                                                             │
│  Method        │ Latency (p99) │ Recall@10 │ Recall@100   │
│  ─────────────────────────────────────────────────────────  │
│  Post-Filter   │    15ms       │   12%     │    25%       │
│  Pre-Filter    │  8500ms       │  100%     │   100%       │
│  Native (Ours) │    25ms       │   94%     │    98%       │
│                                                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│           Varying Filter Selectivity                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Selectivity │ Post-Filter R@10 │ Native R@10 │ Improvement│
│  ───────────────────────────────────────────────────────────│
│  0.01%       │     1%           │    91%      │   91x      │
│  0.1%        │     8%           │    92%      │   11.5x    │
│  1%          │    12%           │    94%      │   7.8x     │
│  10%         │    75%           │    95%      │   1.27x    │
│  50%         │    98%           │    96%      │   0.98x    │
│                                                             │
│  Key insight: Native filtering shines for selective filters│
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Considerations

### Memory Overhead

```
Filter statistics per cluster:
- Centroid tree: ~100MB for 1 billion vectors
- Filter stats: ~50MB additional
- Total overhead: ~150MB (manageable)

Filter bitmaps:
- 1 bit per document per filter value
- Compressed with Roaring bitmaps
- ~100MB for high-cardinality filters
```

### Update Strategies

```rust
/// How to handle filter updates during search

enum UpdateStrategy {
    /// Snapshot isolation: queries see consistent state
    Snapshot,

    /// Read-committed: queries may see partial updates
    ReadCommitted,

    /// Real-time: updates immediately visible
    RealTime,
}

// Turbopuffer uses Snapshot isolation:
// - Queries see state at query start time
// - Updates are batched and applied atomically
// - No query sees inconsistent filter stats
```

### Edge Cases

```
1. Filter matches no documents
   - Detected during cluster selection (all selectivities = 0)
   - Return empty results immediately

2. Filter matches all documents
   - Filter bitmap is all 1s
   - Degrades to unfiltered search (no overhead)

3. Highly selective filter (< 0.001%)
   - May need to search more clusters
   - Adaptive cluster selection compensates

4. Rapidly changing filters
   - LSM compaction keeps up
   - Filter stats eventually consistent
```

---

## Summary

### Key Takeaways

1. **Filtered vector search ≠ vector search + filters**: The problems are fundamentally different

2. **Native filtering requires index integration**: Vector and filter indexes must cooperate

3. **Cluster-level filter stats enable pruning**: Know which clusters to skip before searching

4. **Bitmap filtering is O(1) per vector**: Negligible overhead during search

5. **High recall is achievable**: >90% recall with <30ms latency is possible

### Design Principles

- **Statistics-driven**: Use filter distribution to guide search
- **Early pruning**: Skip irrelevant data as early as possible
- **Efficient bitmaps**: Use compressed bitsets for O(1) filtering
- **Adaptive**: Adjust search strategy based on selectivity

### When to Use Native Filtering

```
Good use cases:
- Filter selectivity 0.01% - 10%
- High cardinality filters (many unique values)
- Compound filters (AND/OR combinations)
- High recall requirements (>90%)

Less beneficial:
- Filter selectivity > 50% (post-filtering works)
- Very low recall requirements (<50%)
- Static datasets (pre-compute per-filter indexes)
```

---

## Appendix: Related Work

### Other Approaches

**Vamana/Graph-based with Filtering:**
```
Graph indexes (HNSW, Vamana) can support filtering by:
- Modifying graph traversal to skip non-matching nodes
- Building separate graphs per filter value

Challenges:
- Graph traversal is already cache-unfriendly
- Filtering makes it worse
- Recall degrades for selective filters
```

**IVF with Filtering:**
```
Inverted File index can support filtering by:
- Partitioning by filter value
- Searching only relevant partitions

Challenges:
- Explodes for high-cardinality filters
- Doesn't handle compound filters well
- Update overhead for maintaining partitions
```

**Learned Indexes:**
```
ML-based approaches predict which regions to search:
- Train model to predict filter + vector matches
- Can be very fast once trained

Challenges:
- Training overhead
- Model staleness as data changes
- Black-box behavior (hard to debug)
```

### Comparison Matrix

```
┌──────────────────┬──────────┬─────────┬──────────┬─────────┐
│     Method       │  Recall  │ Latency │ Update   │ Memory  │
├──────────────────┼──────────┼─────────┼──────────┼─────────┤
│ Post-Filter      │   Low    │   Low   │   Fast   │   Low   │
│ Pre-Filter       │  High    │  High   │   Fast   │   Low   │
│ IVF + Filter     │  Medium  │  Medium │  Medium  │  Medium │
│ HNSW + Filter    │  Medium  │  Medium │  Slow    │  High   │
│ Native (Ours)    │  High    │   Low   │  Medium  │  Medium │
└──────────────────┴──────────┴─────────┴──────────┴─────────┘
```
