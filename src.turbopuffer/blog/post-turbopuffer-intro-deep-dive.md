# Turbopuffer Architecture - Deep Dive

## Executive Summary

Turbopuffer is a managed vector search service built from first principles for cloud-native deployment at billion-scale. Key architectural innovations include:

- **Object storage-first design** - S3/GCS as the primary storage layer
- **Stateless compute nodes** - Any node can serve any namespace
- **Multi-tenancy by design** - Built on learnings from Shopify's multi-tenant platform
- **Cost-optimized architecture** - 10x cheaper than traditional vector databases

This deep-dive explains the architectural decisions, trade-offs, and learnings that shaped Turbopuffer's design.

---

## Origin Story

### The Readwise Problem

**The Spark (2022):**

```
Readwise Reader (read-it-later app) needed:
├── Vector search on 100M+ documents
├── Article recommendations
├── Semantic search

Cost Reality:
├── Existing relational database: ~$5k/month
├── Vector search on same data: ~$20k/month+
└── Result: Feature shelved due to cost

The Insight:
"There has to be a better way to do vector search..."
```

**The Market Gap (2022):**

```
Vector Search Landscape:

Traditional Databases (PostgreSQL + pgvector):
├── Cost: $600/TB/month (3x SSD replicas)
├── Scale: Up to ~10M vectors practically
└── Problem: 100M+ vectors = $50k+/month

Specialized Vector DBs (Pinecone, Weaviate, Milvus):
├── Cost: $500-1000/TB/month (in-memory for performance)
├── Scale: Up to 100M vectors
└── Problem: All data must be in RAM/SSD at all times

The Opportunity:
├── Object storage is 10x cheaper than SSDs
├── Smart caching can bridge the latency gap
└── Build for the next billion-vector use case
```

### Founding Principles

**Learnings from Shopify:**

```
Justine (CTO) and co-founder spent 5+ years on last-resort pager at Shopify:

Key Learnings:
1. Fewer stateful dependencies = more nines of uptime
2. Multi-tenancy is paramount for reliability
3. Sharding protects customers from each other
4. Statelessness enables infinite scalability

Applied to Turbopuffer:
├── No dependencies in critical path except object storage
├── Multi-tenancy from day one
├── Any node can serve traffic for any namespace
└── Durability from S3, not replication
```

---

## Architecture Overview

### High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                         Client Requests                         │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
                 ┌─────────────────┐
                 │   API Gateway   │  (Auth, rate limiting)
                 └────────┬────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
  ┌──────────┐     ┌──────────┐     ┌──────────┐
  │  Node 1  │     │  Node 2  │     │  Node N  │  (Stateless compute)
  └────┬─────┘     └────┬─────┘     └────┬─────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
                        ▼
              ┌───────────────────┐
              │   Memory/SSD      │  (Hot cache for active data)
              │   Cache Layer     │
              └─────────┬─────────┘
                        │
                        ▼
              ┌───────────────────┐
              │   Object Storage  │  (S3/GCS - cold data)
              │   ┌─────────────┐ │
              │   │ Namespace 1 │ │
              │   │ Namespace 2 │ │
              │   │ Namespace N │ │
              │   └─────────────┘ │
              └───────────────────┘
```

### Key Architectural Decisions

**1. Object Storage as Truth**

```
Why Object Storage?

Durability:
├── S3: 11 nines (99.999999999%) durability
├── GCS: Similar durability guarantees
└── No need for manual replication

Scalability:
├── Unlimited storage capacity
├── Automatic scaling
└── Pay for what you use

Cost:
├── S3 Standard: ~$23/TB/month
├── S3 Infrequent Access: ~$12.50/TB/month
└── vs. SSD: ~$600/TB/month (3x replicas)

Trade-off:
└── Higher latency (~100ms for first byte)
    Mitigated by aggressive caching
```

**2. Stateless Compute**

```
Stateless Node Design:

Node Architecture:
┌─────────────────────────────────────────────┐
│  Node (stateless compute)                   │
│  ┌───────────────────────────────────────┐  │
│  │  Query Engine                         │  │
│  │  ├── Vector search                    │  │
│  │  ├── BM25 full-text search            │  │
│  │  └── Filter evaluation                │  │
│  └───────────────────────────────────────┘  │
│  ┌───────────────────────────────────────┐  │
│  │  Local Cache (Memory + SSD)           │  │
│  │  ├── Hot namespaces                   │  │
│  │  └── Recently accessed data           │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘

Failure Recovery:
├── Node dies → requests rerouted to other nodes
├── New node starts cold → loads data from S3 on demand
└── No manual recovery, no data loss

Benefit: Infinite horizontal scalability
```

**3. Multi-Tenancy by Design**

```
Namespace Isolation:

S3 Bucket Structure:
s3://turbopuffer-data/
├── namespace=cursor/
│   ├── shard=1/
│   │   ├── vectors/
│   │   ├── metadata/
│   │   └── index/
│   └── shard=2/
├── namespace=readwise/
│   └── shard=1/
└── namespace=customer-n/
    └── shard=1/

Isolation Guarantees:
├── Each namespace is independent
├── Noisy neighbor protection via resource quotas
├── Separate caches per namespace
└── Individual scaling per namespace

Benefit: 1000s of tenants on shared infrastructure
```

---

## Storage Architecture

### Centroid Tree + Vector Storage

**Two-Level Storage:**

```
Data Layout:

Level 1: Centroids (in DRAM)
┌─────────────────────────────────────────┐
│ Centroid Tree (loaded in memory)        │
│                                         │
│         ┌───────────────┐               │
│         │ Root Centroid │               │
│         └───────┬───────┘               │
│         ┌───────┴───────┐               │
│         ▼               ▼               │
│   ┌──────────┐    ┌──────────┐          │
│   │ Centroid │    │ Centroid │          │
│   └────┬─────┘    └────┬─────┘          │
│        │               │                 │
│   [Refs to SSD]   [Refs to SSD]         │
└─────────────────────────────────────────┘
Size: ~100MB for billion-vector index

Level 2: Vectors (on SSD/S3)
┌─────────────────────────────────────────┐
│ Vector Data (fetched on demand)         │
│                                         │
│ ┌─────────┐ ┌─────────┐ ┌─────────┐     │
│ │Cluster 1│ │Cluster 2│ │Cluster N│     │
│ │Vectors  │ │Vectors  │ │Vectors  │     │
│ │(binary) │ │(binary) │ │(binary) │     │
│ └─────────┘ └─────────┘ └─────────┘     │
│                                         │
│ Size: 6MB per 1M vectors (binary quant) │
└─────────────────────────────────────────┘
```

**Query Flow:**

```
1. Load centroids from DRAM (~1ns per comparison)
   └── Select top-k candidate clusters

2. Fetch candidate clusters from SSD (~100μs)
   └── Read compressed vector blocks

3. Rerank with full-precision vectors (~1ms)
   └── Final top-k results

Total: Sub-10ms latency for billion-scale search
```

### Binary Quantization (RaBitQ)

**Compression Pipeline:**

```
Original Vector (f32, 1024 dimensions):
├── [0.94, -0.01, 0.39, -0.72, 0.02, -0.85, -0.18, 0.99, ...]
├── 1024 × 4 bytes = 4096 bytes per vector
└── 1 billion vectors = 4TB

Binary Quantization:
├── Threshold at 0: positive → 1, negative → 0
├── [1, 0, 1, 0, 1, 0, 0, 1, ...]
├── 1024 bits = 128 bytes per vector
└── 1 billion vectors = 128GB (32x compression!)

RaBitQ Error Correction:
├── Estimate distance with bounded error
├── |d - d'| ≤ ε (provable error bound)
└── Enables accurate search on compressed data
```

**Two-Phase Search:**

```rust
struct QuantizedSearch {
    quantized_vectors: MmapFile,  // Binary quantized (fast scan)
    full_precision_vectors: Option<MmapFile>,  // For reranking
}

impl QuantizedSearch {
    fn search(&self, query: &[f32], top_k: usize) -> Vec<SearchResult> {
        // Phase 1: Scan quantized vectors (FAST)
        let quantized_query = self.quantize(query);
        let candidates = self.scan_quantized(
            &quantized_query,
            top_k * 10,  // Get more for reranking
        );

        // Phase 2: Rerank with full precision (PRECISE)
        let reranked = self.rerank_full_precision(
            &candidates,
            query,
            top_k,
        );

        reranked
    }

    fn scan_quantized(
        &self,
        query: &BitVector,
        n_candidates: usize,
    ) -> Vec<DocId> {
        // Hamming distance with bit operations
        // 64 documents processed per CPU cycle
        let mut scores = Vec::new();

        for (doc_id, doc_bits) in self.quantized_vectors.iter() {
            let distance = hamming_distance(query, &doc_bits);
            scores.push((doc_id, distance));
        }

        scores.select_top_k(n_candidates)
    }
}
```

---

## Query Processing

### Three-Roundtrip Design

**Query Latency Budget:**

```
Sub-second Cold Query Budget (~500ms total):

Roundtrip 1: Metadata fetch (~50ms)
├── Namespace configuration
├── Index metadata
└── Centroid tree (if not cached)

Roundtrip 2: Candidate selection (~200ms)
├── Scan quantized vectors
├── Identify top-k candidates
└── Prefetch for reranking

Roundtrip 3: Reranking (~250ms)
├── Fetch full-precision vectors
├── Compute exact distances
└── Final ranking

Buffer: 50ms for network, queueing
```

**Optimization Target:**

```rust
struct QueryOptimizer {
    target_roundtrips: usize,  // 3
    target_latency_ms: usize,  // 500
}

impl QueryOptimizer {
    fn optimize_query(&self, query: &Query) -> QueryPlan {
        let mut plan = QueryPlan::new();

        // Batch metadata fetches
        plan.add_parallel_fetch(vec![
            Fetch::NamespaceConfig(query.namespace),
            Fetch::CentroidTree(query.namespace),
        ]);

        // Prefetch candidate clusters
        let candidate_clusters = self.estimate_candidates(query);
        plan.add_prefetch(Fetch::Clusters(
            query.namespace,
            candidate_clusters,
        ));

        // Pipeline reranking
        plan.add_stage(Stage::Rerank {
            max_vectors: 1000,
            distance_metric: query.distance_metric,
        });

        plan
    }
}
```

### Hybrid Search Pipeline

**Vector + BM25 Combination:**

```rust
async fn hybrid_search(
    namespace: &str,
    query: HybridQuery,
) -> Vec<SearchResult> {
    // Execute both searches in parallel
    let (vector_results, bm25_results) = tokio::join!(
        self.vector_search(namespace, &query.vector, query.top_k * 2),
        self.bm25_search(namespace, &query.text, query.top_k * 2),
    );

    // Normalize scores to [0, 1] range
    let vector_scores = normalize_scores(vector_results?);
    let bm25_scores = normalize_scores(bm25_results?);

    // Combine scores
    let mut combined: HashMap<u32, f32> = HashMap::new();

    for (doc_id, score) in vector_scores {
        *combined.entry(doc_id).or_insert(0.0) +=
            query.alpha * score;
    }

    for (doc_id, score) in bm25_scores {
        *combined.entry(doc_id).or_insert(0.0) +=
            (1.0 - query.alpha) * score;
    }

    // Return top-k combined results
    let mut results: Vec<_> = combined.into_iter()
        .map(|(doc_id, score)| SearchResult { doc_id, score })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(query.top_k);
    results
}

fn normalize_scores(results: Vec<ScoredDoc>) -> HashMap<u32, f32> {
    if results.is_empty() {
        return HashMap::new();
    }

    let max_score = results.iter().map(|r| r.score).fold(
        f32::NEG_INFINITY,
        f32::max,
    );

    results.into_iter()
        .map(|r| (r.doc_id, r.score / max_score))
        .collect()
}
```

---

## Cost Analysis

### Infrastructure Cost Breakdown

**Storage Cost Comparison:**

```
Per-TB Monthly Costs:

Traditional Vector DBs (in-memory):
├── RAM: $1000/TB/month (for hot data)
├── Replication: 3x for HA
└── Total: ~$3000/TB/month

SSD-Based Systems:
├── SSD storage: $200/TB/month
├── Replication: 3x for durability
└── Total: ~$600/TB/month

Turbopuffer (S3 + Cache):
├── S3 storage: $23/TB/month
├── S3 requests: ~$5/TB/month
├── SSD cache: ~$40/TB/month (shared)
├── DRAM cache: ~$2/TB/month (shared)
└── Total: ~$70/TB/month

Savings: 8-40x vs. traditional approaches
```

**Cost Scaling with Access Patterns:**

```
Monthly Cost by Access Frequency:

Hot Data (accessed daily):
├── S3: $23/TB
├── Requests: $50/TB (frequent access)
├── Cache: $100/TB
└── Total: ~$173/TB

Warm Data (accessed weekly):
├── S3: $23/TB
├── Requests: $10/TB
├── Cache: $20/TB (partial caching)
└── Total: ~$53/TB

Cold Data (accessed monthly):
├── S3: $23/TB
├── Requests: $2/TB
├── Cache: $5/TB (minimal caching)
└── Total: ~$30/TB

Traditional DBs: Same cost regardless of access pattern
Turbopuffer: Cost scales with usage
```

### Real-World Case Study: Cursor

**The Migration:**

```
Cursor (AI Code Editor) - November 2023:

Before Turbopuffer:
├── In-memory vector indexes
├── 100% of data in RAM
├── Only subset of codebases active at once
└── Cost: $20k+/month

After Turbopuffer:
├── Object storage backend
├── Active data cached, rest on S3
├── Seamless access to all codebases
└── Cost: ~$1k/month (95% reduction!)

Migration Time: Few days
Performance: Great cold and warm latency
```

**Why It Worked:**

```
Cursor's Access Pattern:

Active Codebases (20%):
├── Currently being edited
├── Frequently queried
└── Cached in RAM/SSD

Inactive Codebases (80%):
├── Not being edited
├── Rarely queried
└── Stored on S3, loaded on demand

Perfect Match for Turbopuffer Architecture:
├── Pay for storage of inactive data ($23/TB)
├── Cache only active data
└── Seamless access to everything
```

---

## Reliability Design

### Stateless Node Benefits

**Failure Scenarios:**

```
Scenario 1: Node Crash

Before (stateful):
├── Data on crashed node unavailable
├── Manual recovery required
├── Potential data loss if not replicated
└── Downtime: minutes to hours

After (stateless):
├── Request rerouted to healthy node
├── New node loads data from S3
├── No data loss (S3 is durable)
└── Downtime: cold start latency only (~500ms)

Scenario 2: Deployment Rollout

Before (stateful):
├── Drain connections on node
├── Wait for in-flight queries
├── Graceful shutdown: 30-60s per node
└── Full deployment: hours for large cluster

After (stateless):
├── Start new nodes
├── Shift traffic gradually
├── Kill old nodes immediately
└── Full deployment: minutes
```

### Multi-Tenancy Isolation

**Noisy Neighbor Protection:**

```rust
struct TenantQuotas {
    namespace: String,
    max_queries_per_second: usize,
    max_vectors: usize,
    max_storage_bytes: u64,
}

struct RateLimiter {
    quotas: DashMap<String, TenantQuotas>,
    counters: DashMap<String, RateCounter>,
}

impl RateLimiter {
    fn check_rate_limit(&self, namespace: &str) -> Result<()> {
        let quota = self.quotas.get(namespace)
            .ok_or(Error::NamespaceNotFound)?;

        let counter = self.counters.entry(namespace.clone())
            .or_insert_with(RateCounter::new);

        if counter.current_qps() > quota.max_queries_per_second {
            return Err(Error::RateLimitExceeded);
        }

        Ok(())
    }
}

// Per-namespace resource isolation
struct NamespaceIsolation {
    cache_partition: CachePartition,  // Dedicated cache space
    compute_pool: ComputePool,  // Dedicated CPU/memory
}
```

---

## API Design

### Namespace Operations

```rust
// Create namespace
POST /api/v1/namespace
{
  "namespace": "my-vectors",
  "dimension": 1024,
  "distance_metric": "Cosine",
  "schema": {
    "doc_id": "string",
    "metadata": {
      "title": "string",
      "url": "string"
    }
  }
}

// Response
{
  "namespace_id": "ns_abc123",
  "status": "created",
  "limits": {
    "max_vectors": 1000000000,
    "max_queries_per_second": 1000
  }
}

// Delete namespace
DELETE /api/v1/namespace/{namespace}

// List namespaces
GET /api/v1/namespaces
{
  "namespaces": [
    {"name": "my-vectors", "vector_count": 1000000},
    {"name": "other-ns", "vector_count": 500000}
  ]
}
```

### Query Operations

```rust
// Vector search
POST /api/v1/namespace/{ns}/query
{
  "vector": [0.1, 0.2, ..., 0.9],
  "top_k": 10,
  "distance_metric": "Cosine",
  "include_attributes": ["title", "url"]
}

// Response
{
  "results": [
    {
      "id": "doc_123",
      "score": 0.95,
      "attributes": {
        "title": "Example Document",
        "url": "https://example.com/doc"
      }
    },
    ...
  ]
}

// Hybrid search (vector + BM25)
POST /api/v1/namespace/{ns}/query
{
  "vector": [0.1, 0.2, ..., 0.9],
  "rank_by": ["text", "BM25", "search terms"],
  "top_k": 100,
  "alpha": 0.7
}

// Filtered search
POST /api/v1/namespace/{ns}/query
{
  "vector": [0.1, 0.2, ..., 0.9],
  "top_k": 10,
  "filters": [
    ["category", "Equals", "electronics"],
    ["price", "LessThan", 500]
  ]
}
```

### Upsert Operations

```rust
// Batch upsert
POST /api/v1/namespace/{ns}/vectors
{
  "vectors": [
    {
      "id": "doc_1",
      "vector": [0.1, 0.2, ..., 0.9],
      "attributes": {"title": "Doc 1"}
    },
    {
      "id": "doc_2",
      "vector": [0.2, 0.3, ..., 0.8],
      "attributes": {"title": "Doc 2"}
    }
  ]
}

// Response
{
  "status": "success",
  "vectors_upserted": 2
}
```

---

## Lessons Learned

### What Worked Well

**1. Object Storage Choice**
```
Decision: S3/GCS as primary storage
Outcome: Validated - 10x cost savings realized
Learning: Bet on cloud primitives, don't reinvent storage
```

**2. Stateless Compute**
```
Decision: No persistent state on nodes
Outcome: Validated - Infinite horizontal scalability
Learning: Embrace statelessness for cloud-native design
```

**3. Multi-Tenancy from Day One**
```
Decision: Namespace isolation built-in
Outcome: Validated - 1000s of tenants on shared infra
Learning: Multi-tenancy is harder to retrofit than to build in
```

### What We'd Do Differently

**1. Earlier Focus on Filtering**
```
Initial: Vector search only
Reality: Production workloads need filtering
Outcome: Native filtering added later (harder than expected)
Learning: Filtering is table stakes, not a nice-to-have
```

**2. Query Optimization**
```
Initial: Simple query planning
Reality: Complex queries need sophisticated optimization
Outcome: FTS v2 with MAXSCORE required major refactor
Learning: Invest in query optimization earlier
```

---

## Summary

### Architectural Principles

1. **Object storage-first** - S3/GCS as the source of truth
2. **Stateless compute** - Any node can serve any namespace
3. **Multi-tenancy** - Built-in isolation from day one
4. **Cost-optimized** - Pay for what you use, scale with access patterns
5. **Simplicity** - Few dependencies = higher reliability

### Key Innovations

| Innovation | Traditional Approach | Turbopuffer |
|------------|---------------------|-------------|
| Storage | RAM/SSD replicas | Object storage + cache |
| Compute | Stateful nodes | Stateless nodes |
| Scaling | Manual sharding | Automatic per-namespace |
| Cost | $600-3000/TB/month | ~$70/TB/month |
| Reliability | Replication | Object storage durability |

### Future Directions

- **Edge caching** - Cloudflare R2 integration for lower latency
- **Real-time updates** - Sub-second index refresh
- **Advanced ranking** - ML-based relevance scoring
- **Cross-namespace search** - Federated queries across tenants

---

## Appendix: Getting Started

### Quick Start Example

```python
import turbopuffer as tp

# Connect
tp.api_key = "your-api-key"

# Create namespace
ns = tp.Namespace("my-vectors")
ns.upsert(
    vectors=[[0.1, 0.2, ...], [0.3, 0.4, ...]],
    ids=["doc1", "doc2"],
    attributes={"title": ["Doc 1", "Doc 2"]}
)

# Search
results = ns.query(
    vector=[0.15, 0.25, ...],
    top_k=10,
    include_attributes=["title"]
)

for result in results:
    print(f"{result.id}: {result.score} - {result.attributes['title']}")
```

### Architecture Diagram Reference

```
Full Stack:

┌────────────────────────────────────────────────────────────┐
│                        Client SDK                          │
└─────────────────────┬──────────────────────────────────────┘
                      │ HTTPS
                      ▼
┌────────────────────────────────────────────────────────────┐
│                      API Gateway                           │
│  - Authentication                                          │
│  - Rate limiting                                           │
│  - Request routing                                         │
└─────────────────────┬──────────────────────────────────────┘
                      │
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
  ┌──────────┐  ┌──────────┐  ┌──────────┐
  │  Node    │  │  Node    │  │  Node    │  (Stateless)
  │  Cache   │  │  Cache   │  │  Cache   │
  └────┬─────┘  └────┬─────┘  └────┬─────┘
       │             │             │
       └─────────────┼─────────────┘
                     │
                     ▼
        ┌────────────────────────┐
        │    Object Storage      │
        │  - S3 / GCS / R2       │
        │  - Durable             │
        │  - Unlimited scale     │
        └────────────────────────┘
```
