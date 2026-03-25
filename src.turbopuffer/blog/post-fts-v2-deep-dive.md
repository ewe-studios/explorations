# FTS v2 Architecture - Deep Dive

## Executive Summary

Turbopuffer's FTS v2 (Full-Text Search version 2) achieves **up to 20x better performance** compared to FTS v1 through two fundamental innovations:

1. **New index structure** - Fixed-size posting blocks with 10x size reduction on disk
2. **Better search algorithm** - MAXSCORE dynamic pruning optimized for long queries

FTS v2 now delivers performance **comparable to best-in-class search libraries** like Tantivy and Apache Lucene, while maintaining Turbopuffer's object storage architecture.

---

## Problem Context

### The FTS v1 Limitations

**Original Design Goals:**
```
FTS v1 Priorities:
1. Simplicity - Quick to implement and deploy
2. Integration - Work within existing object storage model
3. Basic functionality - Support BM25 ranking

Compromises Made:
- Posting lists stored inefficiently
- Linear scan query evaluation
- No advanced pruning optimizations
```

**FTS v1 Architecture:**

```
FTS v1 Index Structure:

Namespace
├── Term Dictionary (in memory)
│   └── "search" → pointer to posting list
│   └── "engine" → pointer to posting list
│
└── Posting Lists (on object storage)
    ├── "search": [(doc_1, tf=3), (doc_5, tf=1), (doc_8, tf=5), ...]
    │   └── Stored as one large blob per term
    │
    └── "engine": [(doc_1, tf=2), (doc_3, tf=4), (doc_8, tf=1), ...]
        └── Variable size: 1KB - 100MB+ depending on term frequency

Problems:
├── Hotspot terms (common words) create huge blobs
├── No block-level pruning metadata
├── Must fetch entire posting list to evaluate
└── Cache inefficiency for large posting lists
```

**Performance Characteristics:**

| Query Type | FTS v1 Latency | Bottleneck |
|------------|----------------|------------|
| Rare term | 10ms | Network fetch |
| Common term | 500ms+ | Posting list scan |
| Multi-term | 200ms | Merge overhead |
| Long query (10+ terms) | 1000ms+ | No pruning |

---

## FTS v2 Index Structure

### Fixed-Size Posting Blocks

**Core Innovation:**

```
FTS v2 Index Structure:

Term Dictionary (in memory)
├── "search" → [BlockRef, BlockRef, BlockRef]
├── "engine" → [BlockRef, BlockRef]
└── "the"    → [BlockRef, BlockRef, ..., BlockRef] (many blocks)

Posting Blocks (on object storage)
┌─────────────────────────────────────────────────────────────┐
│ Block 1 (256KB fixed size)                                  │
├─────────────────────────────────────────────────────────────┤
│ Header (16 bytes)                                           │
│ ├── magic: u32 = 0x54504654 ("TPFT")                        │
│ ├── version: u16 = 2                                        │
│ ├── count: u16 = 2048  (documents in block)                 │
│ └── block_max_score: f32 = 4.2 (max BM25 in block)         │
├─────────────────────────────────────────────────────────────┤
│ Doc IDs (delta + varint encoded)                            │
│ ├── doc_100 (base)                                          │
│ ├── doc_105 (delta=5, varint=1 byte)                        │
│ ├── doc_108 (delta=3, varint=1 byte)                        │
│ └── ... 2045 more documents                                 │
├─────────────────────────────────────────────────────────────┤
│ Term Frequencies (compressed)                               │
│ ├── tf_1 = 3 (varint)                                       │
│ ├── tf_2 = 1 (varint)                                       │
│ └── ... 2048 more frequencies                               │
├─────────────────────────────────────────────────────────────┤
│ Footer (8 bytes)                                            │
│ ├── doc_ids_offset: u32                                     │
│ └── checksum: u32 (CRC32)                                   │
└─────────────────────────────────────────────────────────────┘
```

**Block Size Analysis:**

```
256KB Block Capacity:

Header:        16 bytes
Footer:         8 bytes
Metadata:     2048 bytes (2048 docs × 1 byte avg varint)
Doc IDs:     51200 bytes (2048 docs × 25 bytes avg)
TF Values:    4096 bytes (2048 docs × 2 bytes avg)
────────────────────────────────
Total:      ~100KB used of 256KB

Remaining space for:
- Position data (optional)
- Additional compression
- Future metadata fields
```

### Delta Encoding + Varint Compression

**Doc ID Compression:**

```rust
fn compress_doc_ids(doc_ids: &[u32]) -> Vec<u8> {
    let mut compressed = Vec::new();
    let mut prev_doc_id = 0u32;

    for &doc_id in doc_ids {
        // Delta encoding: store difference from previous
        let delta = doc_id - prev_doc_id;
        prev_doc_id = doc_id;

        // Varint encoding: smaller numbers use fewer bytes
        let mut value = delta;
        while value >= 0x80 {
            // Set high bit to indicate more bytes
            compressed.push((value as u8) | 0x80);
            value >>= 7;
        }
        compressed.push(value as u8);
    }

    compressed
}

fn decompress_doc_ids(compressed: &[u8]) -> Vec<u32> {
    let mut doc_ids = Vec::new();
    let mut prev_doc_id = 0u32;
    let mut offset = 0;

    while offset < compressed.len() {
        // Varint decoding
        let mut delta = 0u32;
        let mut shift = 0;
        loop {
            let byte = compressed[offset];
            delta |= ((byte & 0x7F) as u32) << shift;
            offset += 1;
            if byte < 0x80 {
                break;  // Last byte
            }
            shift += 7;
        }

        doc_ids.push(prev_doc_id + delta);
        prev_doc_id += delta;
    }

    doc_ids
}
```

**Compression Ratio Analysis:**

```
Original vs Compressed:

Original (u32 per doc ID):
└── 1 million docs × 4 bytes = 4MB

Delta + Varint:
├── Average delta: 100 (for sequential doc IDs)
├── Varint(100) = 1 byte (0x64)
└── 1 million docs × 1 byte = 1MB

Compression ratio: 4:1 (75% reduction)

For highly sequential doc IDs (bulk imports):
├── Average delta: 1
├── Varint(1) = 1 byte
└── Compression ratio approaches 4:1

For random doc IDs:
├── Average delta: N/2 (large)
├── Varint(large) = 4-5 bytes
└── Compression ratio: ~1:1 (no benefit)
```

### Block-Max Optimization

**Pre-computed Block Maxima:**

```rust
struct PostingBlock {
    doc_ids: Vec<u32>,
    term_frequencies: Vec<u16>,
    block_max_score: f32,  // Pre-computed maximum BM25
}

fn compute_block_max_score(
    block: &PostingBlock,
    idf: f32,
    avgdl: f32,
) -> f32 {
    let mut max_score = 0.0f32;

    for &tf in &block.term_frequencies {
        // BM25 formula for this document
        let tf_normalized = (tf as f32 * (1.2 + 1.0)) /
            (tf as f32 + 1.2 * (1.0 - 0.75 + 0.75 * block.doc_len / avgdl));

        let score = idf * tf_normalized;

        if score > max_score {
            max_score = score;
        }
    }

    max_score
}
```

**Query-Time Pruning:**

```rust
fn evaluate_query_with_block_max(
    terms: &[QueryTerm],
    threshold: f32,
) -> Vec<ScoredDoc> {
    let mut results = Vec::new();

    for term in terms {
        for block_ref in &term.block_refs {
            // PRUNE: Skip entire block if max score can't beat threshold
            if block_ref.block_max_score < threshold {
                continue;  // Skip this 256KB block entirely!
            }

            // Fetch and process block
            let block = fetch_block(block_ref);
            let scores = evaluate_block(&block, term);

            // Collect documents above threshold
            for (doc_id, score) in scores {
                if score > threshold {
                    results.push(ScoredDoc { doc_id, score });
                }
            }
        }
    }

    results
}
```

**Pruning Efficiency:**

```
Query: "wireless headphones noise cancellation"
Threshold: 5.0 (after finding initial top-10)

Term: "wireless"
├── Block 1: max_score=3.2 → SKIP (below threshold)
├── Block 2: max_score=4.8 → SKIP
├── Block 3: max_score=6.5 → PROCESS (15 docs above threshold)
├── Block 4: max_score=2.1 → SKIP
└── Block 5: max_score=7.2 → PROCESS (22 docs above threshold)

Blocks processed: 2 of 5 (60% reduction)
I/O saved: 3 × 256KB = 768KB
```

---

## MAXSCORE Algorithm in FTS v2

### Term-Centric Evaluation

**Algorithm Overview:**

```rust
struct FtsV2Query {
    terms: Vec<WeightedTerm>,
    index: &'a InvertedIndex,
}

impl FtsV2Query {
    fn execute(&self, k: usize) -> Vec<SearchResult> {
        // Step 1: Sort terms by max_score descending
        let sorted_terms = self.sort_terms_by_max_score();

        // Step 2: Initialize top-k heap
        let mut topk = BinaryHeap::new();
        let mut threshold = 0.0f32;

        // Step 3: Process terms in order
        for term in &sorted_terms {
            // OPTIMIZATION: Skip non-essential terms
            if term.max_score < threshold {
                continue;  // This term alone can't beat threshold
            }

            // Step 4: Iterate through blocks
            for block_ref in &term.block_refs {
                // Block-max pruning
                if block_ref.max_score < threshold {
                    continue;  // Skip entire block
                }

                // Step 5: Fetch and evaluate block
                let block = self.fetch_block(block_ref);
                let candidates = self.evaluate_block(&block, term);

                // Step 6: Update top-k
                for (doc_id, score) in candidates {
                    if topk.len() < k || score > topk.min() {
                        topk.push((score, doc_id));
                        if topk.len() > k {
                            topk.pop_min();
                        }
                        threshold = topk.min();
                    }
                }
            }
        }

        // Step 7: Return sorted results
        topk.into_sorted_vec()
            .into_iter()
            .map(|(score, doc_id)| SearchResult { doc_id, score })
            .collect()
    }
}
```

### Essential vs Non-Essential Terms

**Dynamic Classification:**

```
Query: "the best wireless headphones for running"

Term Analysis (after finding initial top-10):
┌───────────────┬────────────┬───────────────────┬──────────────┐
│ Term          │ Max Score  │ Current Threshold │ Essential?   │
├───────────────┼────────────┼───────────────────┼──────────────┤
│ the           │ 0.5        │ 3.2               │ NO           │
│ best          │ 2.8        │ 3.2               │ NO           │
│ wireless      │ 4.5        │ 3.2               │ YES          │
│ headphones    │ 5.2        │ 3.2               │ YES          │
│ for           │ 0.8        │ 3.2               │ NO           │
│ running       │ 3.9        │ 3.2               │ YES          │
└───────────────┴────────────┴───────────────────┴──────────────┘

Essential terms (3): wireless, headphones, running
Non-essential terms (3): the, best, for

Processing:
- Essential terms: Full block iteration with scoring
- Non-essential terms: Only score documents from essential terms
```

**Score Computation for Non-Essential Terms:**

```rust
fn compute_full_score(
    doc_id: u32,
    all_terms: &[WeightedTerm],
    essential_terms: &HashSet<usize>,
) -> f32 {
    let mut total_score = 0.0f32;

    for (i, term) in all_terms.iter().enumerate() {
        if essential_terms.contains(&i) {
            // Already scored during candidate generation
            total_score += term.cached_score.get(doc_id).unwrap_or(&0.0);
        } else {
            // Non-essential: fetch score on-demand
            if let Some(score) = term.get_score_for_doc(doc_id) {
                total_score += score;
            }
        }
    }

    total_score
}
```

### Long Query Optimization

**Why MAXSCORE Excels on Long Queries:**

```
LLM-Generated Query (40+ terms):

"Find products that are wireless headphones with noise cancellation,
good battery life, comfortable for long listening sessions,
compatible with both iPhone and Android, and under $200"

Tokenized: ["Find", "products", "wireless", "headphones", "noise",
"cancellation", "good", "battery", "life", "comfortable",
"long", "listening", "sessions", "compatible", "iPhone",
"Android", "under", "$200", ...]  (40+ terms)

Term Classification (after initial top-10):
├── Essential terms: 5 (wireless, headphones, noise, cancellation, battery)
└── Non-essential terms: 35+ (stopwords, common adjectives)

Processing:
├── Candidate generation: 5 essential terms only
├── Scoring: All 40+ terms for candidates only
└── Efficiency: 35+ terms never generate candidates!

WAND Alternative:
└── Must calculate upper bounds across ALL 40+ terms per document
    O(terms × documents) vs O(essential_terms × documents)
```

---

## Performance Comparison

### Benchmark Results

**Dataset: 5M Wikipedia Documents**

| Query | FTS v1 | FTS v2 | Speedup |
|-------|--------|--------|---------|
| "san francisco" | 8ms | 3ms | 2.7x |
| "the who" | 57ms | 7ms | 8.1x |
| "united states constitution" | 20ms | 5ms | 4.0x |
| "lord of the rings" | 75ms | 6ms | 12.5x |
| Long LLM query (40+ terms) | 174ms | 20ms | 8.7x |

**Breakdown by Optimization:**

```
Query: "lord of the rings"

FTS v1 Baseline: 75ms

+ Block-max pruning:     45ms (1.7x from skipping blocks)
+ MAXSCORE algorithm:    15ms (3x from term pruning)
+ Better compression:     8ms (1.9x from faster I/O)
+ Vectorized scoring:     6ms (1.3x from SIMD)
────────────────────────────────────
FTS v2 Total:            6ms (12.5x total speedup)
```

### Index Size Comparison

```
FTS v1 vs FTS v2 Index Sizes:

Dataset: 5M Wikipedia articles (~10GB raw text)

FTS v1:
├── Term dictionary: 500MB
├── Posting lists: 2.5GB
└── Total: 3.0GB

FTS v2:
├── Term dictionary: 500MB
├── Posting blocks: 250MB (10x smaller!)
└── Total: 750MB (4x smaller overall)

Breakdown of FTS v2 savings:
├── Delta encoding: 4x reduction
├── Varint compression: 2x reduction
├── Block metadata: minimal overhead
└── Net: 8x compression on posting lists
```

### Scaling Characteristics

**Latency vs Document Count:**

```
Query: "wireless headphones"

Documents │ FTS v1 │ FTS v2 │ Speedup
──────────┼────────┼────────┼────────
1M        │ 5ms    │ 2ms    │ 2.5x
5M        │ 25ms   │ 6ms    │ 4.2x
10M       │ 50ms   │ 10ms   │ 5.0x
50M       │ 250ms  │ 35ms   │ 7.1x
100M      │ 500ms  │ 60ms   │ 8.3x

FTS v2 scales better because:
- Block-max pruning becomes more effective
- Cache efficiency improves with fixed-size blocks
- MAXSCORE eliminates more non-essential terms
```

---

## New Features in FTS v2

### Phrase Search

**Exact Phrase Matching:**

```rust
// POST /api/v1/namespace/{ns}/query
{
  "rank_by": [
    ["text", "phrase", "wireless headphones"]
  ],
  "top_k": 10
}

// Only matches documents with exact phrase "wireless headphones"
// Not documents with "wireless" and "headphones" far apart
```

**Implementation:**

```rust
struct PhraseMatcher {
    position_index: HashMap<String, Vec<(u32, u16)>>,  // term → [(doc_id, position)]
}

impl PhraseMatcher {
    fn find_phrase(&self, phrase: &[&str]) -> Vec<u32> {
        if phrase.is_empty() {
            return Vec::new();
        }

        // Get positions for first term
        let mut candidates = self.position_index.get(phrase[0])
            .cloned().unwrap_or_default();

        // Filter by subsequent terms
        for (offset, &term) in phrase.iter().enumerate().skip(1) {
            let positions = self.position_index.get(term)
                .cloned().unwrap_or_default();

            candidates.retain(|(doc_id, first_pos)| {
                // Check if subsequent terms appear at correct offsets
                for target_pos in first_pos + 1..first_pos + phrase.len() as u16 {
                    if !positions.iter().any(|(d, p)| {
                        *d == *doc_id && *p == target_pos
                    }) {
                        return false;
                    }
                }
                true
            });
        }

        candidates.into_iter().map(|(doc_id, _)| doc_id).collect()
    }
}
```

### Regex Filtering

**Regular Expression Search:**

```rust
// POST /api/v1/namespace/{ns}/query
{
  "rank_by": ["text", "BM25", "headphones"],
  "filter": ["text", "Regex", "wireless.*noise.*cancellation"],
  "top_k": 100
}
```

**Implementation:**

```rust
struct RegexFilter {
    pattern: regex::Regex,
}

impl RegexFilter {
    fn matches(&self, text: &str) -> bool {
        self.pattern.is_match(text)
    }

    fn optimize_to_terms(&self) -> Option<Vec<String>> {
        // Try to extract required terms from regex
        // "wireless.*headphones" → ["wireless", "headphones"]
        // Used for initial candidate filtering

        if self.pattern.as_str().contains(".*") {
            let parts: Vec<_> = self.pattern.as_str()
                .split(".*")
                .filter(|s| !s.is_empty())
                .collect();

            if parts.len() > 1 {
                return Some(parts.iter().map(|s| s.to_string()).collect());
            }
        }

        None
    }
}
```

### Attribute-Based Ranking

**Rank by Attributes:**

```rust
// Future feature (mentioned in roadmap)
{
  "rank_by": [
    ["text", "BM25", "product description"],
    ["attribute", "popularity", "desc"],
    ["attribute", "recency", "desc"]
  ],
  "top_k": 100
}

// Combined scoring:
// final_score = 0.7 * bm25_score + 0.2 * popularity + 0.1 * recency
```

---

## Hybrid Search Integration

### Vector + BM25 Combined Search

**Query Structure:**

```rust
// Hybrid search query
{
  "vector": [0.1, 0.2, ..., 0.9],
  "rank_by": ["text", "BM25", "wireless headphones"],
  "top_k": 100,
  "alpha": 0.7  // Weight toward vector search
}
```

**Combined Scoring:**

```rust
fn hybrid_score(
    vector_score: f32,
    bm25_score: f32,
    alpha: f32,
) -> f32 {
    // Normalize scores to [0, 1] range
    let normalized_vector = sigmoid(vector_score);
    let normalized_bm25 = sigmoid(bm25_score);

    // Weighted combination
    alpha * normalized_vector + (1.0 - alpha) * normalized_bm25
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}
```

**Query Execution:**

```rust
async fn hybrid_search(
    query: HybridQuery,
) -> Vec<SearchResult> {
    // Parallel execution
    let (vector_results, bm25_results) = tokio::join!(
        vector_index.search(&query.vector, query.top_k * 2),
        fts_index.search(&query.text, query.top_k * 2),
    );

    // Combine and rerank
    let mut combined_scores: HashMap<u32, f32> = HashMap::new();

    for result in vector_results {
        *combined_scores.entry(result.doc_id).or_insert(0.0) +=
            query.alpha * result.score;
    }

    for result in bm25_results {
        *combined_scores.entry(result.doc_id).or_insert(0.0) +=
            (1.0 - query.alpha) * result.score;
    }

    // Sort by combined score
    let mut results: Vec<_> = combined_scores.into_iter()
        .map(|(doc_id, score)| SearchResult { doc_id, score })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(query.top_k);
    results
}
```

---

## Implementation Details

### Index Building Pipeline

```rust
struct FtsV2IndexBuilder {
    term_dictionary: HashMap<String, Vec<u32>>,  // term → doc_ids
    term_frequencies: HashMap<String, Vec<u16>>, // term → tf per doc
    block_size: usize,  // 256KB target
}

impl FtsV2IndexBuilder {
    fn add_document(&mut self, doc_id: u32, text: &str) {
        // Tokenize
        let tokens = tokenize(text);

        // Count term frequencies
        let mut tf_counts: HashMap<String, u16> = HashMap::new();
        for token in tokens {
            *tf_counts.entry(token).or_insert(0) += 1;
        }

        // Update posting lists
        for (term, tf) in tf_counts {
            self.term_dictionary.entry(term.clone())
                .or_insert_with(Vec::new)
                .push(doc_id);

            self.term_frequencies.entry(term)
                .or_insert_with(Vec::new)
                .push(tf);
        }
    }

    fn flush_to_blocks(&mut self) -> Vec<PostingBlock> {
        let mut blocks = Vec::new();

        for (term, doc_ids) in &self.term_dictionary {
            let tf_values = &self.term_frequencies[term];

            // Split into fixed-size blocks
            let chunk_size = self.estimate_docs_per_block();

            for chunk in doc_ids.chunks(chunk_size) {
                let block_tf: Vec<_> = tf_values[
                    chunk.as_ptr() as usize - doc_ids.as_ptr() as usize..
                ].iter().take(chunk.len()).copied().collect();

                let block = PostingBlock::create(
                    chunk,
                    &block_tf,
                    self.compute_block_max_score(&block_tf, term),
                );

                blocks.push(block);
            }
        }

        blocks
    }
}
```

### Query Cache

```rust
struct FtsQueryCache {
    // Cache frequent query results
    cache: DashMap<QueryHash, CachedResult>,
    ttl: Duration,
}

struct CachedResult {
    result: Vec<SearchResult>,
    computed_at: Instant,
    access_count: AtomicUsize,
}

impl FtsQueryCache {
    fn get(&self, query: &Query) -> Option<Vec<SearchResult>> {
        let hash = query.hash();

        if let Some(entry) = self.cache.get(&hash) {
            if entry.computed_at.elapsed() < self.ttl {
                entry.access_count.fetch_add(1, Ordering::Relaxed);
                return Some(entry.result.clone());
            }
        }

        None
    }

    fn put(&self, query: Query, result: Vec<SearchResult>) {
        let hash = query.hash();

        self.cache.insert(hash, CachedResult {
            result,
            computed_at: Instant::now(),
            access_count: AtomicUsize::new(1),
        });

        // Evict least-recently-used if over capacity
        if self.cache.len() > 10000 {
            self.evict_lru();
        }
    }
}
```

---

## Summary

### Key Achievements

1. **10x smaller index** - Fixed-size blocks with delta+varint compression
2. **20x faster queries** - Block-max pruning + MAXSCORE algorithm
3. **Best-in-class performance** - Comparable to Lucene/Tantivy
4. **New features** - Phrase search, regex filtering, attribute ranking

### Design Principles

- **Learn from the best** - MAXSCORE from Lucene, block structure from Tantivy
- **Optimize for object storage** - Fixed-size blocks for predictable I/O
- **Prune early, prune often** - Block-max and term elimination
- **Vectorize everything** - SIMD score computation

### Future Roadmap

- **Ranking by attributes** - Combine BM25 with custom scoring functions
- **Highlighting** - Return matching snippets for UI display
- **Better search-as-you-type** - Optimized prefix matching
- **Fuzziness** - Approximate term matching for typos
- **Globbing** - Wildcard pattern matching (e.g., "wireless*")

---

## Appendix: Migration from FTS v1

### Index Migration Process

```rust
async fn migrate_namespace(namespace: &str) -> Result<()> {
    // Step 1: Create new FTS v2 index
    let v2_index = FtsV2Index::create(namespace);

    // Step 2: Stream all documents from v1
    let mut doc_stream = v1_index.stream_all_documents(namespace).await?;

    // Step 3: Rebuild index with v2 format
    while let Some(doc) = doc_stream.next().await {
        v2_index.add_document(doc.id, &doc.text).await?;
    }

    // Step 4: Swap indexes atomically
    v2_index.commit().await?;
    namespace_registry.update(namespace, v2_index).await?;

    Ok(())
}
```

### Backward Compatibility

```rust
// FTS v2 supports all FTS v1 query formats
{
  // v1 format still works
  "rank_by": ["text", "BM25", "search query"]

  // v2 format with new features
  "rank_by": [
    ["text", "phrase", "exact phrase"],
    ["text", "BM25", "additional terms"]
  ],
  "filter": ["text", "Regex", "pattern.*"]
}
```
