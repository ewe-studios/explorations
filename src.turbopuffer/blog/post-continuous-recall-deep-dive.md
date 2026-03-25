# Continuous Recall Monitoring - Deep Dive

## Executive Summary

This deep-dive explores Turbopuffer's innovative approach to search quality monitoring through continuous recall measurement. Key innovations include:

- **1% traffic sampling** - Automatic recall measurement on production queries
- **Real-time monitoring** - Alerts when recall@10 drops below 90-95%
- **Production-grounded evaluation** - No reliance on academic benchmarks alone
- **Per-customer tracking** - Individual recall dashboards for each namespace

This is the first vector search system to implement continuous recall monitoring in production, recognizing that **what's not measured is not guaranteed**.

---

## Problem Context

### The Vector Search Accuracy Challenge

**The Approximate Nearest Neighbor Trade-off:**

```
Exact Search (Ground Truth):
├── Compare query to ALL vectors
├── Return true top-k nearest neighbors
├── Accuracy: 100%
└── Latency: O(N) - impractical for billion-scale databases

Approximate Search (ANN):
├── Compare query to subset of vectors
├── Return estimated top-k neighbors
├── Accuracy: 90-95% (target)
└── Latency: O(log N) or O(√N) - feasible for billions of vectors
```

**The Core Problem:**

How do you know your ANN index is returning accurate results without comparing to ground truth?

```
Traditional Approach:
├── Run offline benchmarks (SIFT, GloVe, Deep1B)
├── Measure recall on static test sets
├── Deploy to production
└── Hope accuracy holds up

Problem: Production queries differ from benchmarks!
```

### Why Recall Degrades in Production

**1. Index Update Patterns**
```
Incremental Updates:
├── New vectors added continuously
├── Centroids drift over time
├── Some clusters become overcrowded
└── Recall degrades gradually

Example:
Day 1:  95% recall@10 (fresh index)
Day 30: 92% recall@10 (moderate drift)
Day 90: 87% recall@10 (significant degradation - ALERT!)
```

**2. Query Distribution Shift**
```
Training queries (benchmarks):
├── Well-distributed across vector space
├── Standard query patterns
└── Expected difficulty levels

Production queries:
├── Clustered in popular regions
├── Edge case queries from AI agents
└── Unexpected difficulty spikes

Example:
- Benchmark queries: 95% recall
- Production queries: 82% recall (distribution shift!)
```

**3. Filter Complexity**
```
Unfiltered queries:
└── Recall@10: 95%

Filtered queries (namespace = "user-123"):
├── Smaller candidate pool
├── Clusters may not align with filter boundaries
└── Recall@10: 78%

Complex filters (price < 50 AND category = "electronics"):
├── Highly selective filtering
├── Very few candidates per cluster
└── Recall@10: 65%
```

---

## Recall Fundamentals

### What is Recall?

**Formal Definition:**

```
Recall@K = |{relevant results} ∩ {returned top-K}| / |{relevant results}|

Where:
- {relevant results} = true top-K from exhaustive search (ground truth)
- {returned top-K} = approximate top-K from ANN index
- Intersection = results that appear in both sets
```

**Visual Example:**

```
Ground Truth (exhaustive search):
┌────────────────────────────────────────┐
│ [doc_A: 0.95, doc_B: 0.92, doc_C: 0.89, │
│  doc_D: 0.87, doc_E: 0.85, ...]        │
└────────────────────────────────────────┘

ANN Results (approximate):
┌────────────────────────────────────────┐
│ [doc_A: 0.95, doc_B: 0.91, doc_X: 0.88, │
│  doc_C: 0.86, doc_Y: 0.84, ...]        │
└────────────────────────────────────────┘
         ↑        ↑              ↑
       Match   Match    NOT in ground truth

Recall@5 = 4/5 = 80%
(doc_A, doc_B, doc_C match; doc_X, doc_Y are false positives)
```

### Recall vs Precision in Vector Search

**Important Distinction:**

In traditional IR:
- **Recall** = fraction of all relevant docs that were retrieved
- **Precision** = fraction of retrieved docs that are relevant

In vector search literature:
- **"Recall@K"** actually measures **precision at K** against ground truth
- The term "recall" is used because we're checking overlap with exhaustive results

```
True terminology for vector search:

Precision@K = |{relevant} ∩ {top-K}| / K

Example:
Ground truth top-10: [A, B, C, D, E, F, G, H, I, J]
ANN top-10:          [A, B, X, C, Y, D, E, Z, F, G]

Precision@10 = 7/10 = 70% (A,B,C,D,E,F,G match)
This is what papers call "recall@10"
```

---

## Continuous Recall Architecture

### System Design

**High-Level Architecture:**

```
┌─────────────────────────────────────────────────────────────┐
│                    Production Traffic                       │
└─────────────────────────┬───────────────────────────────────┘
                          │
              ┌───────────┴───────────┐
              │                       │
              ▼                       ▼
    ┌─────────────────┐     ┌─────────────────┐
    │   99% Traffic   │     │   1% Sampling   │
    │   (Normal ops)  │     │   (Monitoring)  │
    └────────┬────────┘     └────────┬────────┘
             │                      │
             ▼                      ▼
    ┌─────────────────┐     ┌─────────────────┐
    │   Search Engine │     │ Recall Evaluator│
    │   (Fast path)   │     │ (Ground truth)  │
    └────────┬────────┘     └────────┬────────┘
             │                      │
             │                      ▼
             │            ┌─────────────────┐
             │            │ Compare Results │
             │            │ Compute Recall  │
             │            └────────┬────────┘
             │                     │
             │                     ▼
             │            ┌─────────────────┐
             │            │ Metrics Storage │
             │            │ + Alerting      │
             │            └─────────────────┘
             │
             ▼
    ┌─────────────────┐
    │   User Results  │
    └─────────────────┘
```

### Sampling Strategy

**1% Traffic Sampling:**

```rust
struct RecallSampler {
    sample_rate: f64,  // 0.01 = 1%
    rng: StdRng,
}

impl RecallSampler {
    fn should_sample(&mut self, query_id: u64) -> bool {
        // Consistent sampling based on query_id
        // Same query always sampled or not sampled
        let hash = self.hash_query(query_id);
        (hash as f64 / u64::MAX as f64) < self.sample_rate
    }

    fn hash_query(&self, query_id: u64) -> u64 {
        // Deterministic hash for consistent sampling
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        query_id.hash(&mut hasher);
        hasher.finish()
    }
}
```

**Why 1%?**

```
Traffic Analysis:

1M queries/day:
├── 1% sample = 10,000 recall evaluations/day
├── Enough for statistically significant metrics
└── Overhead: ~10% additional compute (ground truth is expensive)

100M queries/day:
├── 1% sample = 1M recall evaluations/day
├── More than enough for per-customer breakdowns
└── Overhead: ~1% additional compute (amortized)

Trade-off:
- Higher sample rate = better visibility, more overhead
- Lower sample rate = less overhead, noisier metrics
- 1% is the sweet spot for most workloads
```

### Ground Truth Computation

**Exhaustive Search Implementation:**

```rust
struct GroundTruthEvaluator {
    // All vectors stored efficiently for fast scan
    vectors: MmapVectors,  // Memory-mapped file
    distance_fn: DistanceMetric,
}

impl GroundTruthEvaluator {
    /// Compute exact top-K by scanning ALL vectors
    fn exhaustive_search(
        &self,
        query: &[f32],
        k: usize,
        filter: Option<&Filter>,
    ) -> Vec<ScoredDoc> {
        let mut topk = BinaryHeap::new();
        let mut min_score = f32::NEG_INFINITY;

        // Linear scan through all vectors
        for (doc_id, vector) in self.vectors.iter() {
            // Apply filter if present
            if let Some(f) = filter {
                if !f.matches(doc_id) {
                    continue;
                }
            }

            // Compute exact distance
            let score = self.distance_fn.compute(query, vector);

            // Update top-k heap
            if topk.len() < k || score > min_score {
                if topk.len() >= k {
                    topk.pop();
                }
                topk.push(ScoredDoc { doc_id, score });
                if topk.len() >= k {
                    min_score = topk.peek().map(|d| d.score).unwrap_or(f32::NEG_INFINITY);
                }
            }
        }

        // Return sorted by score descending
        topk.into_sorted_vec()
    }
}
```

**Optimization: Cached Ground Truth**

```rust
struct CachedGroundTruth {
    // Cache recent queries and their ground truth
    cache: DashMap<QueryHash, GroundTruthResult>,
    ttl: Duration,
}

struct GroundTruthResult {
    query_hash: QueryHash,
    ground_truth_topk: Vec<ScoredDoc>,
    computed_at: Instant,
}

impl CachedGroundTruth {
    fn get_or_compute(
        &self,
        query: &Query,
        evaluator: &GroundTruthEvaluator,
    ) -> Vec<ScoredDoc> {
        let hash = query.hash();

        // Check cache first
        if let Some(cached) = self.cache.get(&hash) {
            if cached.computed_at.elapsed() < self.ttl {
                return cached.ground_truth_topk.clone();
            }
        }

        // Compute fresh ground truth
        let result = evaluator.exhaustive_search(
            &query.vector,
            query.top_k * 2,  // Get more for robust comparison
            query.filter.as_ref(),
        );

        // Cache the result
        self.cache.insert(hash, GroundTruthResult {
            query_hash: hash,
            ground_truth_topk: result.clone(),
            computed_at: Instant::now(),
        });

        result
    }
}
```

### Recall Computation

**Comparing ANN vs Ground Truth:**

```rust
struct RecallMetrics {
    recall_at_10: f64,
    recall_at_100: f64,
    ndcg_at_10: f64,  // Normalized Discounted Cumulative Gain
}

fn compute_recall(
    ann_results: &[ScoredDoc],
    ground_truth: &[ScoredDoc],
    k: usize,
) -> f64 {
    let ann_topk: HashSet<_> = ann_results.iter()
        .take(k)
        .map(|d| d.doc_id)
        .collect();

    let gt_topk: HashSet<_> = ground_truth.iter()
        .take(k)
        .map(|d| d.doc_id)
        .collect();

    let intersection = ann_topk.intersection(&gt_topk).count();
    intersection as f64 / k as f64
}

fn compute_ndcg(
    ann_results: &[ScoredDoc],
    ground_truth: &[ScoredDoc],
    k: usize,
) -> f64 {
    // Create doc_id -> ideal_rank mapping from ground truth
    let ideal_ranks: HashMap<_> = ground_truth.iter()
        .take(k)
        .enumerate()
        .map(|(rank, doc)| (doc.doc_id, rank + 1))
        .collect();

    // Compute DCG for ANN results
    let mut dcg = 0.0;
    for (position, doc) in ann_results.iter().take(k).enumerate() {
        if let Some(&ideal_rank) = ideal_ranks.get(&doc.doc_id) {
            // Relevance = how high this doc ranked in ground truth
            let relevance = (k + 1 - ideal_rank) as f64;
            let discount = 1.0 / ((position + 1) as f64).log2();
            dcg += relevance * discount;
        }
    }

    // Compute IDCG (ideal DCG - perfect ranking)
    let mut idcg = 0.0;
    for (position, _) in ground_truth.iter().take(k).enumerate() {
        let relevance = (k - position) as f64;
        let discount = 1.0 / ((position + 1) as f64).log2();
        idcg += relevance * discount;
    }

    // Normalize
    if idcg > 0.0 { dcg / idcg } else { 0.0 }
}
```

---

## Monitoring Dashboard

### Per-Customer Recall Tracking

**Datadog Integration:**

```
Recall Dashboard (per customer/namespace):

┌─────────────────────────────────────────────────────────────┐
│ Customer: cursor.com                                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│ Recall@100 ────────────────────────────────────────────     │
│ 100% ┤╭────────────────────────────────────────────╮        │
│  95% ┤│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│        │
│  90% ┤│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│        │
│  85% ┤│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│        │
│  80% ┤╰────────────────────────────────────────────╯        │
│       └─────────────────────────────────────────────        │
│       00:00    06:00    12:00    18:00    24:00            │
│                                                             │
│ Recall@10 ─────────────────────────────────────────────     │
│ 100% ┤╭──────────────────────────────────────────────╮      │
│  95% ┤│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│      │
│  90% ┤│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│◄─── Target threshold
│  85% ┤│░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│      │
│  80% ┤╰──────────────────────────────────────────────╯      │
│       └─────────────────────────────────────────────        │
│       00:00    06:00    12:00    18:00    24:00            │
└─────────────────────────────────────────────────────────────┘

Average recall@100: 96.2%
Average recall@10:  94.8%
Alerts triggered: 0 (threshold: 90%)
```

### Alert Configuration

**Alert Rules:**

```yaml
# Datadog Monitor Configuration
recall_alert:
  name: "Vector Search Recall Degradation"
  type: "metric"
  query: |
    avg(last_5m):avg(turbopuffer.recall_at_10{namespace:*}) < 0.90

  thresholds:
    critical: 0.85  # Page on-call
    warning: 0.90   # Slack notification

  tags:
    - "service:vector-search"
    - "team:search-platform"

  # Per-namespace tracking
  group_by:
    - "namespace"

  # Notify specific teams per customer
  notification_overrides:
    "cursor-*": ["team-cursor"]
    "readwise-*": ["team-readwise"]
```

### Recall Trends Analysis

**Time-Series Analysis:**

```rust
struct RecallTrendAnalyzer {
    window_size: Duration,  // 24 hours
    degradation_threshold: f64,  // 0.05 = 5% drop
}

impl RecallTrendAnalyzer {
    fn detect_degradation(
        &self,
        recall_series: &[RecallDataPoint],
    ) -> Option<RecallDegradation> {
        if recall_series.len() < 10 {
            return None;  // Not enough data
        }

        // Linear regression to find trend
        let (slope, intercept) = self.linear_regression(recall_series);

        // Check if recall is declining
        if slope < -0.001 {  // Negative trend
            let current_recall = recall_series.last().unwrap().recall;
            let predicted_recall = slope * recall_series.len() as f64 + intercept;

            if predicted_recall < current_recall - self.degradation_threshold {
                return Some(RecallDegradation {
                    current_recall,
                    predicted_recall,
                    rate_of_decline: -slope,
                    estimated_time_to_threshold:
                        (current_recall - 0.90) / -slope,  // Hours until 90%
                });
            }
        }

        None
    }
}
```

---

## Filtered Query Recall

### The Filtered Query Challenge

**Why Filters Reduce Recall:**

```
Unfiltered Query:
├── Search all clusters
├── Each cluster contributes candidates
└── Good coverage of vector space
    Recall@10: 95%

Filtered Query (category = "electronics"):
├── Search all clusters
├── Filter OUT non-matching candidates
├── Some clusters may have 0 matching docs
└── Poor coverage in filtered subspace
    Recall@10: 78%
```

**Visual Example:**

```
Vector Space with Clusters:

    ┌─────────────────────────────────────────────┐
    │  Cluster A (90% electronics)                │
    │  ┌─────┐                                    │
    │  │▓▓▓▓▓│  ▓ = electronics                   │
    │  └─────┘  ░ = other                         │
    │                                             │
    │         ┌─────┐                             │
    │         │░░░▓░│  Cluster B (20% electronics)│
    │         └─────┘                             │
    │                                             │
    │  ┌─────┐                                    │
    │  │░░░░░│  Cluster C (0% electronics!)       │
    │  └─────┘                                    │
    └─────────────────────────────────────────────┘

Query: "wireless headphones" + filter "electronics"

Without filter awareness:
├── Cluster C still searched (waste!)
├── Cluster B returns few candidates
└── Only Cluster A contributes good results

With filter-aware search:
├── Cluster C skipped entirely
├── Cluster B searched with adjusted expectations
└── Cluster A prioritized
```

### Filter-Aware Recall Measurement

**Per-Filter Recall Tracking:**

```rust
struct FilterRecallTracker {
    // Track recall per filter pattern
    filter_patterns: DashMap<FilterHash, FilterStats>,
}

struct FilterStats {
    filter_pattern: Filter,
    query_count: u64,
    total_recall_sum: f64,
    recall_samples: Vec<f64>,
    p50_recall: f64,
    p90_recall: f64,
    min_recall: f64,
}

impl FilterRecallTracker {
    fn record_recall(
        &self,
        filter: &Filter,
        recall: f64,
    ) {
        let hash = filter.hash();
        let mut stats = self.filter_patterns
            .entry(hash)
            .or_insert_with(|| FilterStats::new(filter.clone()));

        stats.query_count += 1;
        stats.total_recall_sum += recall;
        stats.recall_samples.push(recall);

        // Keep rolling window of last 1000 samples
        if stats.recall_samples.len() > 1000 {
            stats.recall_samples.remove(0);
        }

        // Update percentiles
        stats.update_percentiles();
    }

    fn get_problematic_filters(&self) -> Vec<(&Filter, f64)> {
        let mut problematic = Vec::new();

        for entry in self.filter_patterns.iter() {
            let stats = entry.value();
            if stats.p90_recall < 0.85 {  # Below threshold
                problematic.push((
                    &stats.filter_pattern,
                    stats.p90_recall,
                ));
            }
        }

        problematic
    }
}
```

### Filter Selectivity Impact

**Measuring Filter Selectivity:**

```rust
fn compute_filter_selectivity(
    filter: &Filter,
    total_docs: u64,
) -> f64 {
    let matching_docs = filter.count_matches(total_docs);
    matching_docs as f64 / total_docs as f64
}

// Correlation analysis:
// Filter Selectivity vs Recall

Selectivity │ Recall@10
────────────┼──────────
100%        │ 95%       (no filter)
50%         │ 92%
25%         │ 88%
10%         │ 82%
5%          │ 75%
1%          │ 65%
0.1%        │ 45%

// Highly selective filters (< 1%) need special handling!
```

---

## Production Learnings

### Recall Patterns Observed

**1. Diurnal Patterns**

```
Recall@10 Over 24 Hours:

100% ┤╭──────────────────────────────────────╮
 95% ┤│░░░░░░░░░░░░░░╭───────────╮░░░░░░░░░░│
 90% ┤│░░░░░░░░░░░░░░│░░░░░░░░░░░│░░░░░░░░░░│
 85% ┤│░░░╰─────╮░░░░│░░░░░░░░░░░│░░░╭─────╮│
 80% ┤╰─────────╯░░░░╰───────────╯░░░╰─────╯│
     └──────────────────────────────────────
     00:00   06:00   12:00   18:00   24:00

Observation:
- Recall dips during peak traffic (US business hours)
- Likely due to cache pressure and resource contention
- Action: Scale up cache during peak hours
```

**2. Query Type Correlation**

```
Recall by Query Category:

Category              │ Recall@10 │ % of Traffic
──────────────────────┼───────────┼─────────────
Simple (1-2 terms)    │ 97%       │ 45%
Medium (3-5 terms)    │ 94%       │ 35%
Complex (5+ terms)    │ 89%       │ 15%
Filtered              │ 85%       │ 5%

Action: Complex and filtered queries need optimization
```

**3. Customer-Specific Patterns**

```
Customer Recall Comparison:

Customer    │ Recall@10 │ Recall@100 │ Traffic
────────────┼───────────┼────────────┼────────
Cursor      │ 96.2%     │ 98.5%      │ High
Readwise    │ 94.8%     │ 97.1%      │ Medium
Customer C  │ 91.5%     │ 95.2%      │ Low
Customer D  │ 88.3%     │ 92.1%      │ Medium  ⚠️

Customer D needs attention:
- Lower than average recall
- Investigation: Highly filtered queries (80% of traffic)
- Action: Optimize for filtered query patterns
```

### Actionable Insights

**1. Index Rebuilding Triggers**

```rust
struct IndexHealthMonitor {
    recall_threshold: f64,
    degradation_rate_threshold: f64,
}

impl IndexHealthMonitor {
    fn should_rebuild_index(
        &self,
        recall_history: &[RecallDataPoint],
    ) -> RebuildRecommendation {
        let current_recall = recall_history.last().unwrap().recall;

        // Immediate rebuild needed
        if current_recall < 0.85 {
            return RebuildRecommendation::Immediate;
        }

        // Scheduled rebuild recommended
        if current_recall < self.recall_threshold {
            return RebuildRecommendation::Scheduled(
                Duration::from_hours(24)
            );
        }

        // Check degradation trend
        if let Some(degradation) = self.analyze_trend(recall_history) {
            if degradation.rate_of_decline > self.degradation_rate_threshold {
                return RebuildRecommendation::Scheduled(
                    degradation.estimated_time_to_threshold
                );
            }
        }

        RebuildRecommendation::NotNeeded
    }
}
```

**2. Parameter Tuning Based on Recall**

```rust
struct QueryOptimizer {
    // Dynamically adjust search parameters based on recall
    target_recall: f64,
}

impl QueryOptimizer {
    fn get_search_params(
        &self,
        query_type: QueryType,
        recent_recall: f64,
    ) -> SearchParams {
        let mut params = SearchParams::default();

        if recent_recall < self.target_recall - 0.05 {
            // Recall is low, increase search thoroughness
            match query_type {
                QueryType::Unfiltered => {
                    params.num_clusters *= 1.5;
                    params.rerank_candidates *= 2;
                }
                QueryType::Filtered => {
                    params.filter_aware_search = true;
                    params.bitmap_intersection = true;
                }
            }
        }

        params
    }
}
```

---

## Recall API Design

### Exposing Recall to Users

**Recall Endpoint:**

```rust
// POST /api/v1/recall/evaluate
// Evaluate recall for a specific query against ground truth

#[derive(Serialize, Deserialize)]
struct RecallRequest {
    namespace: String,
    vector: Vec<f32>,
    top_k: usize,
    filter: Option<Filter>,
    distance_metric: DistanceMetric,
}

#[derive(Serialize)]
struct RecallResponse {
    ann_results: Vec<ScoredDoc>,
    ground_truth: Vec<ScoredDoc>,
    recall_at_k: f64,
    ndcg_at_k: f64,
    latency_ann_ms: f64,
    latency_exhaustive_ms: f64,
}

async fn evaluate_recall(
    req: RecallRequest,
) -> Result<RecallResponse> {
    let start_ann = Instant::now();

    // Run ANN search
    let ann_results = index.search(
        &req.vector,
        req.top_k,
        req.filter.as_ref(),
    ).await;

    let ann_latency = start_ann.elapsed().as_secs_f64() * 1000.0;

    // Run exhaustive search
    let start_exhaustive = Instant::now();
    let ground_truth = index.exhaustive_search(
        &req.vector,
        req.top_k,
        req.filter.as_ref(),
    ).await;
    let exhaustive_latency = start_exhaustive.elapsed().as_secs_f64() * 1000.0;

    // Compute recall metrics
    let recall = compute_recall(&ann_results, &ground_truth, req.top_k);
    let ndcg = compute_ndcg(&ann_results, &ground_truth, req.top_k);

    Ok(RecallResponse {
        ann_results,
        ground_truth,
        recall_at_k: recall,
        ndcg_at_k: ndcg,
        latency_ann_ms: ann_latency,
        latency_exhaustive_ms: exhaustive_latency,
    })
}
```

### Dashboard Integration

**Grafana Dashboard Configuration:**

```json
{
  "dashboard": {
    "title": "Vector Search Recall Monitoring",
    "panels": [
      {
        "title": "Recall@10 Over Time",
        "type": "graph",
        "targets": [
          {
            "expr": "avg(turbopuffer_recall_at_10)",
            "legendFormat": "Average Recall@10"
          },
          {
            "expr": "avg(turbopuffer_recall_at_10) - std(turbopuffer_recall_at_10)",
            "legendFormat": "-1 Std Dev"
          }
        ],
        "thresholds": [
          {"value": 0.90, "color": "red"},
          {"value": 0.95, "color": "yellow"}
        ]
      },
      {
        "title": "Recall by Filter Type",
        "type": "table",
        "targets": [
          {
            "expr": "avg by (filter_type) (turbopuffer_recall_at_10)"
          }
        ]
      },
      {
        "title": "Low Recall Customers",
        "type": "table",
        "targets": [
          {
            "expr": "bottomk(10, avg by (namespace) (turbopuffer_recall_at_10))"
          }
        ]
      }
    ]
  }
}
```

---

## Summary

### Key Innovations

1. **Continuous measurement** - 1% of all queries evaluated against ground truth
2. **Real-time monitoring** - Alerts trigger when recall drops below thresholds
3. **Per-customer visibility** - Individual recall dashboards for each namespace
4. **Production-grounded** - No reliance on academic benchmarks alone
5. **Actionable insights** - Recall trends inform index rebuilding decisions

### Recall Targets

| Query Type | Target Recall@10 | Alert Threshold |
|------------|------------------|-----------------|
| Unfiltered | 95%+ | 90% |
| Simple Filter | 90%+ | 85% |
| Complex Filter | 85%+ | 80% |

### Design Principles

- **What's not measured is not guaranteed** - Academic benchmarks aren't enough
- **Sample continuously** - 1% sampling provides statistical significance
- **Monitor per-customer** - Different workloads have different recall characteristics
- **Track trends, not just point-in-time** - Degradation rate matters
- **Expose to users** - Let customers verify their own recall

### Future Directions

- **Adaptive recall** - Dynamically adjust search parameters based on recent recall
- **Predictive rebuilding** - Rebuild index before recall drops below threshold
- **Filter-aware optimization** - Special handling for highly-selective filters
- **Per-query-type tuning** - Different parameters for different query patterns

---

## Appendix: Statistical Significance

### Sample Size Calculation

```
For 95% confidence interval with ±1% margin of error:

n = (Z² × p × (1-p)) / E²

Where:
- Z = 1.96 (95% confidence)
- p = 0.95 (expected recall)
- E = 0.01 (margin of error)

n = (1.96² × 0.95 × 0.05) / 0.01²
n = 1,825 samples

At 1M queries/day with 1% sampling:
- 10,000 samples/day
- Statistically significant in ~4.5 hours
```

### Confidence Interval Computation

```rust
fn compute_confidence_interval(
    recall_samples: &[f64],
    confidence: f64,
) -> (f64, f64, f64) {
    let n = recall_samples.len() as f64;
    let mean = recall_samples.iter().sum::<f64>() / n;

    let variance = recall_samples.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / (n - 1.0);

    let std_dev = variance.sqrt();
    let standard_error = std_dev / n.sqrt();

    // Z-score for confidence level
    let z_score = match confidence {
        0.90 => 1.645,
        0.95 => 1.96,
        0.99 => 2.576,
        _ => 1.96,
    };

    let margin = z_score * standard_error;

    (mean - margin, mean + margin, margin)
}

// Example:
// recall_samples = [0.94, 0.95, 0.93, 0.96, ...]
// (lower, upper, margin) = compute_confidence_interval(&samples, 0.95)
// Result: (0.942, 0.958, 0.008) = 95% ± 0.8%
```
