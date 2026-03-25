# Vectorized MAXSCORE over WAND - Deep Dive

## Executive Summary

Turbopuffer's FTS v2 achieves up to **20x faster** full-text search performance by adopting a vectorized variant of block-max MAXSCORE, the same text search algorithm used by Apache Lucene. This post explains the algorithmic differences between MAXSCORE and WAND, and why MAXSCORE excels particularly on long LLM-generated queries.

## Problem Context

### The Search Problem

When searching for documents matching a query like "new york", the search engine must:
1. Find all documents containing the query terms
2. Score each document by relevance (BM25)
3. Return only the top-k results (typically k=10-100)

The naive approach—scoring ALL matching documents—is wasteful when we only need top-k.

### Query Score Calculation

For a query with multiple terms, each term contributes to the document score:

```
Query: "new york"
├── "new": max_score = 2.0 (common term)
└── "york": max_score = 5.0 (less common)

Document scores based on term presence:
┌──────────────────────┬─────────────────────┐
│ Document contains    │ Max possible score  │
├──────────────────────┼─────────────────────┤
│ only "new"           │ 2.0                 │
│ only "york"          │ 5.0                 │
│ both "new" + "york"  │ 7.0                 │
└──────────────────────┴─────────────────────┘
```

### The Key Insight

During top-k evaluation, once we've found k documents with score ≥ threshold, any document whose **maximum possible score** is below the threshold can be safely skipped without affecting recall.

Example: If the 10th-best document has score 4.0, documents containing only "new" (max score 2.0) can never enter the top-10.

---

## Algorithm 1: MAXSCORE (Term-Centric)

### How MAXSCORE Works

**Step 1: Sort Terms by Max Score**
```
Query: "new york population"
├── "new":        max_score = 2.0
├── "population": max_score = 4.0
└── "york":      max_score = 5.0

Sorted: [new(2.0), population(4.0), york(5.0)]
```

**Step 2: Iterate Through Posting Lists**
```
Initial state:
┌─────────────────────────────────────────────────────────┐
│ Term         │ Max Score │ Posting List (doc IDs)      │
├─────────────────────────────────────────────────────────┤
│ new          │ 2.0       │ 0, 1, 3, 5, 6, 7, 9, 10     │
│              │           │          ^ (current pos)     │
│ population   │ 4.0       │ 1, 2, 3, 8, 9               │
│              │           │          ^                   │
│ york         │ 5.0       │ 2, 3, 7, 10                 │
│              │           │       ^                      │
└─────────────────────────────────────────────────────────┘

TOPK_HEAP = [(6.0, doc1), (9.0, doc2), (11.0, doc3)]
min_threshold = 6.0
```

**Step 3: Identify Non-Essential Terms**
```
As min_threshold rises:
- When threshold > 2.0: "new" becomes non-essential
  - Documents with only "new" cannot qualify
  - Skip "new" when finding candidates

- When threshold > 4.0: "population" also becomes non-essential
  - Only "york" remains essential
  - Much faster query completion
```

### MAXSCORE Algorithm Pseudocode

```rust
fn maxscore_query(terms: Vec<Term>, k: usize) -> Vec<Document> {
    // Sort terms by max score descending
    let mut terms = terms.sort_by(|a, b| b.max_score - a.max_score);

    let mut topk = MinHeap::with_capacity(k);
    let mut threshold = 0.0;

    for term in &terms {
        // Skip non-essential terms for candidate generation
        if term.max_score < threshold {
            continue; // This term alone can't beat threshold
        }

        // Iterate through posting list
        for (doc_id, score) in term.posting_list {
            // Compute full score using ALL terms
            let full_score = compute_full_score(doc_id, &terms);

            // Update top-k heap
            if topk.len() < k || full_score > topk.min() {
                topk.push((full_score, doc_id));
                if topk.len() > k {
                    topk.pop_min();
                }
                threshold = topk.min();
            }
        }
    }

    topk.into_sorted()
}
```

### Key Characteristics

- **Term-centric**: Processes one term at a time
- **Essential vs Non-essential**: Dynamically determines which terms matter
- **Optimization**: Use only essential terms for candidate generation, all terms for scoring

---

## Algorithm 2: WAND (Document-Centric)

### How WAND Works

WAND (Weak AND) takes a document-centric approach:

**Step 1: Sort Doc IDs and Calculate Upper Bounds**
```
At the same breakpoint as MAXSCORE:
┌─────────────────────────────────────────────────────────┐
│ Term         │ Max Score │ Posting List (doc IDs)      │
├─────────────────────────────────────────────────────────┤
│ new          │ 2.0       │ 0, 1, 3, 5, 6, 7, 9, 10     │
│              │           │          ^                   │
│ population   │ 4.0       │ 1, 2, 3, 8, 9               │
│              │           │          ^                   │
│ york         │ 5.0       │ 2, 3, 7, 10                 │
│              │           │       ^                      │
└─────────────────────────────────────────────────────────┘

For each doc ID, calculate upper bound (assuming all future terms match):
┌───────────────┬─────────────────────────────┐
│ Doc ID        │ Upper Bound Calculation     │
├───────────────┼─────────────────────────────┤
│ 5             │ 2.0 (only "new" confirmed)  │
│ 7             │ 7.0 ("york" + maybe "new")  │
│ 8             │ 11.0 (all three possible)   │
└───────────────┴─────────────────────────────┘
```

**Step 2: Find Next Viable Document**
```
Find first doc where upper_bound > threshold:

If threshold = 6.0:
- Doc 5: upper_bound = 2.0 < 6.0 → SKIP
- Doc 7: upper_bound = 7.0 > 6.0 → EVALUATE NEXT

If threshold = 7.5:
- Doc 7: upper_bound = 7.0 < 7.5 → SKIP
- Doc 8: upper_bound = 11.0 > 7.5 → EVALUATE NEXT
```

### WAND Algorithm Pseudocode

```rust
fn wand_query(terms: Vec<Term>, k: usize) -> Vec<Document> {
    let mut iterators: Vec<PostingIterator> = terms.iter().collect();
    let mut topk = MinHeap::with_capacity(k);
    let mut threshold = 0.0;

    loop {
        // Sort current doc IDs
        let mut doc_ids: Vec<_> = iterators.iter()
            .map(|it| it.current_doc_id())
            .collect();
        doc_ids.sort();

        // Calculate upper bound for each doc ID
        let mut next_doc = None;
        for doc_id in doc_ids {
            let upper_bound = calculate_upper_bound(&iterators, doc_id);

            if upper_bound > threshold {
                next_doc = Some(doc_id);
                break;
            }
        }

        let doc_id = next_doc?; // No more candidates

        // Compute actual score
        let score = compute_full_score(doc_id, &terms);
        topk.push_or_replace_min((score, doc_id), k);
        threshold = topk.min();
    }

    topk.into_sorted()
}

fn calculate_upper_bound(iterators: &[PostingIterator], doc_id: DocId) -> f32 {
    let mut bound = 0.0;

    for it in iterators {
        if it.current_doc_id() <= doc_id {
            // Term might match this document
            bound += it.term_max_score;
        }
    }

    bound
}
```

### Key Characteristics

- **Document-centric**: Evaluates documents one at a time
- **Continuous skipping**: Prunes documents that cannot possibly qualify
- **Upper bound calculation**: Must assume terms "might" match if iterator hasn't passed

---

## Performance Comparison

### Benchmark Results (5M Wikipedia Documents)

| Query | FTS v1 | FTS v2 (MAXSCORE) | Speedup |
|-------|--------|-------------------|---------|
| "san francisco" | 8ms | 3ms | 2.7x |
| "the who" | 57ms | 7ms | 8.1x |
| "united states constitution" | 20ms | 5ms | 4.0x |
| "lord of the rings" | 75ms | 6ms | 12.5x |
| Long LLM query (40+ terms) | 174ms | 20ms | 8.7x |

### Why MAXSCORE Excels on Long Queries

**LLM-Generated Query Characteristics:**
- Often contain 20-50+ terms
- Many terms are low-importance (stopwords, common words)
- High redundancy in semantic meaning

**MAXSCORE Advantage:**
```
Long query with 40 terms:
- After finding first ~100 results, threshold rises quickly
- 35+ terms become non-essential (max_score < threshold)
- Only 5 essential terms needed for candidate generation
- Document evaluation becomes very selective

WAND Disadvantage:
- Must calculate upper bounds for every doc ID across all 40 terms
- Upper bound calculation is O(terms) per document
- More terms = more overhead before skipping
```

### When to Use Each Algorithm

| Scenario | Recommended Algorithm | Reason |
|----------|----------------------|--------|
| Short queries (1-5 terms) | Either | Similar performance |
| Long queries (10+ terms) | MAXSCORE | Faster term elimination |
| High k values (k > 1000) | WAND | Better for retrieving many results |
| Very selective queries | WAND | Document skipping more effective |

---

## Vectorization Optimizations

### SIMD-Accelerated MAXSCORE

Turbopuffer's implementation uses SIMD to accelerate score computation:

```rust
use std::arch::x86_64::*;

/// Vectorized score computation across multiple documents
unsafe fn compute_scores_simd(
    doc_ids: &[DocId],
    term_weights: &[f32],
    term_postings: &[&[u8]], // Compressed posting lists
) -> Vec<f32> {
    let mut scores = vec![0.0f32; doc_ids.len()];

    // Process 8 documents at a time with AVX2
    for term_idx in 0..term_weights.len() {
        let weight = _mm256_set1_ps(term_weights[term_idx]);
        let postings = term_postings[term_idx];

        for chunk in doc_ids.chunks_exact(8) {
            // Load 8 document term frequencies
            let tfs = load_term_frequencies(postings, chunk);

            // Compute BM25 contribution: weight * tf_normalized
            let contribution = _mm256_mul_ps(weight, tfs);

            // Accumulate into scores
            let score_ptr = scores.as_mut_ptr().add(chunk.as_ptr() as usize);
            let acc = _mm256_loadu_ps(score_ptr);
            _mm256_storeu_ps(score_ptr, _mm256_add_ps(acc, contribution));
        }
    }

    scores
}
```

### Block-Max Optimization

**Block-Max Index Structure:**
```
Posting List organized in blocks of 128 documents:
┌─────────┬─────────┬─────────┬─────────┐
│ Block 1 │ Block 2 │ Block 3 │ Block 4 │
├─────────┼─────────┼─────────┼─────────┤
│ max=3.2 │ max=4.1 │ max=2.8 │ max=5.0 │ ← pre-computed max scores
├─────────┼─────────┼─────────┼─────────┤
│ docs    │ docs    │ docs    │ docs    │
│ 1-128   │ 129-256 │ 257-384 │ 385-512 │
└─────────┴─────────┴─────────┴─────────┘

Query optimization:
If threshold > 4.5:
- Skip Block 1 (max 3.2)
- Skip Block 3 (max 2.8)
- Only process Block 2 and Block 4
```

---

## Implementation in Turbopuffer FTS v2

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     FTS v2 Query Pipeline                   │
├─────────────────────────────────────────────────────────────┤
│  1. Query Parser                                             │
│     └── Tokenize input, identify terms                      │
│                                                              │
│  2. Term Statistics Lookup                                  │
│     └── Fetch IDF, max_score for each term                  │
│                                                              │
│  3. MAXSCORE Executor ← (This post's focus)                 │
│     ├── Sort terms by max_score                             │
│     ├── Dynamic essential term tracking                     │
│     └── Vectorized score computation                        │
│                                                              │
│  4. Results Merger                                          │
│     └── Combine with vector search results (if hybrid)      │
└─────────────────────────────────────────────────────────────┘
```

### Code Structure (Simplified)

```rust
pub struct FtsV2Query {
    terms: Vec<WeightedTerm>,
    index: InvertedIndex,
    vectorized_executor: VectorizedMaxScoreExecutor,
}

impl FtsV2Query {
    pub fn execute(&self, k: usize) -> Vec<SearchResult> {
        // 1. Sort terms by max score
        let sorted_terms = self.sort_terms_by_max_score();

        // 2. Run MAXSCORE with vectorized execution
        let candidates = self.vectorized_executor.execute(
            &sorted_terms,
            k,
        );

        // 3. Final ranking and filtering
        self.rerank_and_filter(candidates, k)
    }

    fn sort_terms_by_max_score(&self) -> Vec<&WeightedTerm> {
        let mut terms: Vec<_> = self.terms.iter().collect();
        terms.sort_by(|a, b| {
            b.max_score.partial_cmp(&a.max_score).unwrap()
        });
        terms
    }
}
```

---

## Practical Applications

### Use Case 1: Semantic + Lexical Hybrid Search

```python
# Turbopuffer hybrid query
query = {
    "vector": [0.1, 0.2, ...],  # Semantic search
    "rank_by": ["text", "BM25", "product description"],  # FTS v2
    "top_k": 100
}

# Results combined from:
# 1. Vector search (semantic similarity)
# 2. BM25 search (lexical matching via MAXSCORE)
# Final ranking = weighted combination
```

### Use Case 2: LLM Agent Queries

```python
# LLM generates verbose natural language query
llm_query = """
    Find products that are wireless headphones
    with noise cancellation, good battery life,
    comfortable for long listening sessions,
    compatible with both iPhone and Android
"""

# FTS v2 handles 20+ terms efficiently
# Non-essential terms ("that", "are", "with") quickly eliminated
# Essential terms ("wireless", "headphones", "noise cancellation") drive search
```

### Use Case 3: Faceted Search with Full-Text

```python
# E-commerce search with filters
query = {
    "filters": [
        ["category", "Equals", "electronics"],
        ["price", "LessThan", 500]
    ],
    "rank_by": ["text", "BM25", "gaming laptop"]
}

# Filter bitmap restricts candidate documents
# MAXSCORE operates only on matching documents
```

---

## Summary

### Key Takeaways

1. **MAXSCORE is term-centric**: Processes terms individually, eliminating non-essential terms as threshold rises

2. **WAND is document-centric**: Evaluates documents one at a time, skipping based on upper bound calculations

3. **MAXSCORE excels on long queries**: With 20-50+ terms (common in LLM queries), MAXSCORE quickly eliminates most terms from candidate generation

4. **Vectorization amplifies benefits**: SIMD execution makes score computation even faster, shifting bottleneck to memory access

5. **Block-max indexing adds another optimization layer**: Pre-computed block maxima enable skipping entire posting list blocks

### Design Principles

- **Algorithm selection matters**: The "best" algorithm depends on query characteristics
- **Early elimination is key**: Both algorithms succeed by eliminating candidates early
- **Hardware awareness**: Vectorization and cache efficiency are critical for performance
- **Measure, don't guess**: Benchmark on real workloads to validate algorithm choice

### Future Directions

- **Learned rankings**: Integrating ML models with MAXSCORE pruning
- **Adaptive algorithms**: Dynamically switch between MAXSCORE/WAND based on query analysis
- **GPU acceleration**: Offloading score computation to GPUs for massive parallelism

---

## Appendix: Full Benchmark Results

### Dataset: English Wikipedia Export (~5M Documents)

| Query Type | Query | FTS v1 | FTS v2 | Speedup |
|------------|-------|--------|--------|---------|
| Simple (2 terms) | "san francisco" | 8ms | 3ms | 2.7x |
| Simple (2 terms) | "the who" | 57ms | 7ms | 8.1x |
| Medium (3 terms) | "united states constitution" | 20ms | 5ms | 4.0x |
| Medium (4 terms) | "lord of the rings" | 75ms | 6ms | 12.5x |
| Long (40+ terms) | "pop singer songwriter born 1989..." | 174ms | 20ms | 8.7x |

*Note: Current production p90 latencies are ~5-10ms on hot cache after FTS v2 optimizations*
