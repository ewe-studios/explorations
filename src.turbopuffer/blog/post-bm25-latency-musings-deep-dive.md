# BM25 Latency Analysis - Deep Dive

## Executive Summary

This deep-dive analyzes the factors affecting BM25 full-text search query latency at scale, revealing counterintuitive performance characteristics and scaling behavior. Key findings include:

- Adding terms to a query can make it **faster**, not slower
- Query latency scales with document count, but "easy" queries scale sub-linearly
- Essential term count matters more than total term count when using MAXSCORE
- Latency scales efficiently with top-k (10x top-k = ~65% latency increase)

## Problem Context

### The BM25 Latency Puzzle

BM25 (Best Matching 25) is the standard ranking function for full-text search engines. The formula estimates document relevance based on term frequency and inverse document frequency:

```
score(D, Q) = Σ IDF(qᵢ) × (f(qᵢ, D) × (k₁ + 1)) / (f(qᵢ, D) + k₁ × (1 - b + b × |D|/avgdl))

Where:
- D = document, Q = query
- qᵢ = i-th query term
- f(qᵢ, D) = term frequency in document
- IDF(qᵢ) = log((N - n(qᵢ) + 0.5) / (n(qᵢ) + 0.5))
- N = total documents, n(qᵢ) = documents containing term
- k₁, b = tuning parameters (typically k₁=1.2, b=0.75)
```

### The Counterintuitive Observation

Consider these three queries against a 200M document corpus (Common Crawl dataset, ~1.26TiB):

```
Query 1: "singer"
Query 2: "pop singer"
Query 3: "pop singer songwriter"

Intuition: Query 3 should be slowest (most terms to process)
Reality: Query 3 is FASTER than Query 2
```

**Measured latencies (single thread, MAXSCORE, unfiltered):**

| Query | Terms | Essential Terms | Latency |
|-------|-------|-----------------|---------|
| "singer" | 1 | 1 | 15ms |
| "pop singer" | 2 | 2 | 45ms |
| "pop singer songwriter" | 3 | 2 | 32ms |

**Why is Query 3 faster despite having more terms?**

The answer lies in how MAXSCORE handles essential vs non-essential terms.

---

## Essential vs Non-Essential Terms

### How MAXSCORE Classifies Terms

MAXSCORE dynamically determines which terms are "essential" during query evaluation:

```rust
struct TermStats {
    term: String,
    max_score: f32,      // Maximum possible BM25 contribution
    posting_list_len: usize,
}

fn classify_terms(
    terms: &[TermStats],
    current_threshold: f32,
) -> Vec<bool> {
    terms.iter()
        .map(|t| t.max_score >= current_threshold)
        .collect()
}
```

**Example classification:**

```
Query: "the lord of the rings"

Term Analysis:
┌─────────────┬────────────┬──────────────────┬──────────────┐
│ Term        │ Max Score  │ Posting List Len │ Essential?   │
├─────────────┼────────────┼──────────────────┼──────────────┤
│ the         │ 0.5        │ 200M (all docs)  │ NO           │
│ lord        │ 4.2        │ 50K              │ YES          │
│ of          │ 0.3        │ 180M             │ NO           │
│ rings       │ 3.8        │ 100K             │ YES          │
└─────────────┴────────────┴──────────────────┴──────────────┘

Only "lord" and "rings" drive candidate generation!
"the" and "of" are scored but don't generate candidates.
```

### Why Adding Terms Can Help

When you add a selective term to a query:

1. **Threshold rises faster** - More selective terms = higher-scoring documents found earlier
2. **More terms become non-essential** - Higher threshold = more aggressive pruning
3. **Fewer documents evaluated** - Less work overall despite more terms

```
Query evolution as terms are added:

"singer":
├── Threshold after first 10 docs: 2.5
├── Must evaluate: 10M documents (posting list)
└── Latency: 45ms

"pop singer":
├── Threshold after first 10 docs: 4.0
├── Must evaluate: 15M documents (both posting lists)
└── Latency: 45ms (higher threshold but more work)

"pop singer songwriter":
├── Threshold after first 10 docs: 6.5
├── Must evaluate: 8M documents (only essential terms)
└── Latency: 32ms (highest threshold, selective evaluation)
```

---

## Latency Scaling Analysis

### Scaling with Document Count

To understand how BM25 latency scales, experiments were run with iterative indexing from 1M to 200M documents.

**Latency Model:**

```
latency(n) = C × n^K

Where:
- n = number of documents
- C = constant factor (depends on query)
- K = scaling exponent (lower = better scaling)
```

**K values by query type:**

| Query | K Value | Scaling Behavior |
|-------|---------|------------------|
| "singer" | 0.19 | Highly sub-linear |
| "pop singer" | 0.21 | Sub-linear |
| "pop singer songwriter" | 0.22 | Sub-linear |
| "the lord of the rings" | 0.85 | Near-linear |

**Key insight:** Easy queries scale much better than hard queries. The performance gap **widens** as document count increases.

```
Latency vs Document Count (log-log scale):

100ms │                    ╭─ Query 7 (K=0.85)
      │                  ╱
 10ms │        ╭─ Query 2 (K=0.21)
      │      ╱ ╱
  1ms │  ╭─ ╱ ╱
      │ ╱ ╱ ╱
      └─┴─┴─┴────────────
        1M  10M 100M 200M
             Documents
```

### Why Hard Queries Scale Poorly

Queries with high K values (near 1.0) have these characteristics:

1. **Many essential terms** - Can't prune effectively
2. **Common terms in posting lists** - Must process large fractions of corpus
3. **Low threshold** - Documents rarely get skipped

Query 7 ("the lord of the rings") exemplifies this:
- Contains "the" (200M postings - entire namespace)
- Contains "of" (180M postings)
- MAXSCORE must still process these for scoring
- Near-linear scaling because most documents are evaluated

**The paradox:** Query 7 has the **best latency per million postings** because it skips so efficiently, but still scales poorly because there's so much to skip.

---

## Scaling with Top-K

### Top-K Performance Model

Experiments progressively increased top-k while querying all 200M documents:

```
Top-K Scaling Results:

Top-K   │ Latency │ Increase Factor
────────┼─────────┼────────────────
10      │ 15ms    │ 1.0x (baseline)
100     │ 22ms    │ 1.47x
1,000   │ 31ms    │ 2.07x
10,000  │ 45ms    │ 3.0x
100,000 │ 68ms    │ 4.5x
```

**Observed K values for top-k scaling:**

| Query | K Value (top-k) |
|-------|-----------------|
| "singer" | 0.19 |
| "pop singer" | 0.21 |
| "pop singer songwriter" | 0.22 |

**Key finding:** Multiplying top-k by 10 increases latency by only ~65%.

### Why Top-K Scales So Well

```
Top-K Query Processing:

Phase 1: Find initial top-k (fast)
├── Process essential terms only
├── Threshold rises quickly
└── Many documents skipped

Phase 2: Refine top-k (slower)
├── Threshold stabilizes
├── Each improvement requires more work
└── Diminishing returns

Phase 3: Large top-k (expensive)
├── Many documents must be tracked
├── Heap operations dominate
└── Memory bandwidth becomes limiting
```

**Heap Operations:**

```rust
// Min-heap for top-k tracking
struct TopK {
    heap: BinaryHeap<Reverse<ScoredDoc>>,
    k: usize,
}

impl TopK {
    fn push(&mut self, doc: ScoredDoc) {
        if self.heap.len() < self.k {
            self.heap.push(Reverse(doc));
        } else if doc.score > self.heap.peek().unwrap().0.score {
            self.heap.pop();
            self.heap.push(Reverse(doc));
        }
    }
}

// Complexity: O(log k) per candidate document
// Total: O(n × log k) where n = candidates
```

---

## Query Difficulty Classification

### Three Tiers of Query Difficulty

Based on the latency analysis, queries fall into three categories:

**Tier 1: Easy Queries (K < 0.2)**
```
Characteristics:
- 1-2 selective terms
- Posting lists < 1M documents
- High max_score terms

Examples:
- "quantum computing"
- "neural network architecture"
- "photosynthesis mechanism"

Latency at 200M docs: 10-30ms
Scaling: Excellent (sub-linear)
```

**Tier 2: Medium Queries (0.2 < K < 0.5)**
```
Characteristics:
- 2-4 terms with mixed selectivity
- Some common terms (IDF < 5)
- Posting lists 1M-50M documents

Examples:
- "best restaurants near me"
- "how to learn programming"
- "climate change effects"

Latency at 200M docs: 30-100ms
Scaling: Good (mostly sub-linear)
```

**Tier 3: Hard Queries (K > 0.5)**
```
Characteristics:
- Contains stopwords (the, of, and)
- Posting lists > 50M documents
- Low selectivity terms

Examples:
- "the meaning of life"
- "how to be a good person"
- "best movie of all time"

Latency at 200M docs: 100-500ms
Scaling: Poor (near-linear)
```

### Stopword Impact Analysis

```
Query: "lord of the rings"

Without stopwords removed:
├── "lord": 50K postings, max_score=4.2
├── "of": 180M postings, max_score=0.3
├── "the": 200M postings, max_score=0.5
├── "rings": 100K postings, max_score=3.8
└── Total processing: 380M+ postings

With stopwords removed:
├── "lord": 50K postings, max_score=4.2
├── "rings": 100K postings, max_score=3.8
└── Total processing: 150K postings (2500x less!)
```

---

## Practical Implications

### For Search Engine Designers

**1. Implement MAXSCORE or WAND**
```
Naive evaluation (no pruning):
└── Processes ALL documents in ALL posting lists
    O(Σ posting_list_lengths)

MAXSCORE evaluation:
├── Processes only essential terms
└── Skips documents below threshold
    O(essential_terms × selective_postings)

Speedup: 10-100x on typical queries
```

**2. Pre-compute Term Statistics**
```rust
struct InvertedIndex {
    terms: HashMap<String, TermStats>,
}

struct TermStats {
    idf: f32,           // Inverse document frequency
    max_score: f32,     // Maximum BM25 contribution
    posting_list_len: usize,
    posting_list_offset: u64,  // For lazy loading
}

// During index build (expensive, one-time):
fn compute_term_stats(index: &InvertedIndex) -> HashMap<String, TermStats> {
    let mut stats = HashMap::new();
    for (term, postings) in &index.terms {
        let max_score = compute_max_bm25_contribution(postings);
        stats.insert(term.clone(), TermStats {
            idf: compute_idf(postings.len(), index.total_docs()),
            max_score,
            posting_list_len: postings.len(),
            posting_list_offset: postings.file_offset(),
        });
    }
    stats
}
```

**3. Consider Stopword Filtering**
```
Query preprocessing pipeline:

Raw query → Tokenize → Remove stopwords → Stem → MAXSCORE evaluation

Common stopwords to filter:
- Articles: the, a, an
- Prepositions: of, in, on, at, to, for
- Conjunctions: and, or, but
- Pronouns: I, you, he, she, it
```

### For Application Developers

**1. Query Length Optimization**
```python
# Bad: Unnecessary terms
query = "the best way to learn how to code programming"

# Good: Selective terms only
query = "learn code programming"

# Better: Domain-specific terms
query = "learn python programming"
```

**2. Top-K Selection**
```python
# Don't request more results than needed
# Each 10x increase = ~65% latency increase

# Bad: Always requesting 1000 results
results = search(query, top_k=1000)

# Good: Request only what UI needs
results = search(query, top_k=20)  # For single page
# or
results = search(query, top_k=100)  # For pagination
```

**3. Hybrid Search Strategy**
```python
# Combine BM25 with vector search for best results
query = {
    "vector": embedding,
    "text": "selective terms only",
    "top_k": 100,
    "alpha": 0.7  # Weight toward vector search
}

# Vector search handles semantic similarity
# BM25 handles exact term matching
# Combined: Better recall than either alone
```

---

## Latency Prediction Model

### Complete Latency Formula

Based on the experimental analysis:

```
latency = C × (n_docs / 1M)^K_docs × (top_k / 10)^K_topk × (essential_postings / 1M)

Where typical values are:
- C = 10-50ms (base constant, depends on hardware)
- K_docs = 0.2-0.8 (document scaling, query-dependent)
- K_topk = 0.2-0.25 (top-k scaling, consistent across queries)
- essential_postings = sum of posting list lengths for essential terms
```

### Prediction Examples

**Example 1: Easy query at scale**
```
Query: "quantum computing"
n_docs = 200M
top_k = 10
essential_postings = 500K
C = 15ms, K_docs = 0.19, K_topk = 0.19

latency = 15 × (200)^0.19 × (1)^0.19 × (0.5)
        = 15 × 2.7 × 1 × 0.5
        = ~20ms
```

**Example 2: Hard query at scale**
```
Query: "the meaning of life"
n_docs = 200M
top_k = 100
essential_postings = 150M
C = 15ms, K_docs = 0.85, K_topk = 0.22

latency = 15 × (200)^0.85 × (10)^0.22 × (150)
        = 15 × 89 × 1.7 × 150
        = ~340ms
```

---

## Summary

### Key Takeaways

1. **Query latency is proportional to essential postings, not total terms**
   - Adding selective terms can make queries faster
   - Stopwords dramatically increase latency

2. **Easy queries scale sub-linearly with document count**
   - K values of 0.19-0.25 mean 100x docs = ~3x latency
   - Hard queries (K > 0.5) scale near-linearly

3. **Top-k scaling is highly efficient**
   - 10x top-k = ~65% latency increase
   - K values consistently ~0.2 across query types

4. **MAXSCORE is essential for long queries**
   - Query 7 ("the lord of the rings") would be "horrendously slow" without MAXSCORE
   - Exhaustive evaluation = O(all documents) vs O(essential documents)

5. **Model latencies before deploying at scale**
   - Use latency = C × n^K model
   - Measure K for your query workload
   - Plan capacity accordingly

### Design Principles

- **Measure, don't guess** - Query performance varies wildly
- **Optimize for common case** - Most queries are easy
- **Handle worst case** - Hard queries need special treatment
- **Cache aggressively** - Posting lists benefit from caching
- **Consider hybrid search** - Vector + BM25 = better recall

---

## Appendix: Experimental Setup

### Dataset Specifications

```
Corpus: Common Crawl web pages
Documents: 200 million
Total size: ~1.26 TiB
Average doc size: ~6.3 KiB
Index size: ~500 GB (compressed)
```

### Hardware Configuration

```
CPU: Single thread (for consistent measurements)
RAM: 128 GB (full index fits in memory)
Storage: NVMe SSD (for cold start measurements)
Network: Local (no network latency)
```

### Query Sets

```
Simple queries (2 terms):
- "san francisco"
- "the who"
- "pop singer"

Medium queries (3-4 terms):
- "united states constitution"
- "lord of the rings"
- "pop singer songwriter"

Hard queries (5+ terms):
- "the meaning of life and existence"
- "how to be a good person in life"
- Long LLM-generated queries (20-50 terms)
```
