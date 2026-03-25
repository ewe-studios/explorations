# Query Engine: Quickwit and Distributed Search

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/quickwit/`

---

## Table of Contents

1. [Introduction to Distributed Search](#introduction-to-distributed-search)
2. [Search Engine Fundamentals](#search-engine-fundamentals)
3. [Quickwit Architecture](#quickwit-architecture)
4. [Inverted Indexes](#inverted-indexes)
5. [Distributed Query Execution](#distributed-query-execution)
6. [Aggregation](#aggregation)
7. [Object Storage Native Design](#object-storage-native-design)
8. [Code Examples](#code-examples)

---

## Introduction to Distributed Search

### The Problem

Traditional search engines face challenges at scale:

- **Data volume**: Terabytes to petabytes of logs, documents
- **Query latency**: Sub-second response times expected
- **Concurrency**: Hundreds of concurrent users
- **Cost**: Storage and compute expenses
- **Operations**: Complex cluster management

### Quickwit Solution

**Quickwit** is a cloud-native search engine:

```
Quickwit Design Goals:
- Object storage native (S3, GCS, Azure)
- Log analytics focused
- Sub-second search latency
- Simple operations (no complex cluster)
- Cost-effective (storage/compute分离)
```

### Use Cases

| Use Case | Description |
|----------|-------------|
| **Log Analytics** | Centralized logging, troubleshooting |
| **Observability** | Metrics, traces, events correlation |
| **Full-Text Search** | Document search, e-commerce |
| **Security Analytics** | SIEM, threat detection |

---

## Search Engine Fundamentals

### Inverted Index

The core data structure for full-text search:

```
Document Collection:
Doc 1: "The quick brown fox"
Doc 2: "The lazy dog"
Doc 3: "The fox jumps"

Inverted Index:
┌─────────────────────────────────────────┐
│  Term       │  Postings List           │
├─────────────────────────────────────────┤
│  "the"      │  [1, 2, 3]               │
│  "quick"    │  [1]                     │
│  "brown"    │  [1]                     │
│  "fox"      │  [1, 3]                  │
│  "lazy"     │  [2]                     │
│  "dog"      │  [2]                     │
│  "jumps"    │  [3]                     │
└─────────────────────────────────────────┘

Query "fox": Look up "fox" → [1, 3] → Docs 1, 3
```

### Posting List with Positions

```
Enhanced Index with Positions:
Term: "fox"
Postings: [
  { doc_id: 1, positions: [3], freq: 1 },
  { doc_id: 3, positions: [1], freq: 1 },
]

Phrase Query "quick fox":
1. Find "quick": positions [1] in doc 1
2. Find "fox": positions [3] in doc 1
3. Check adjacency: position 1 and 3? No (gap of 2)
4. Result: No match for exact phrase
```

### BM25 Scoring

```
BM25 (Best Matching 25) scoring formula:

score(d, q) = Σ (IDF(ti) * (f(ti,d) * (k1 + 1)) / (f(ti,d) + k1 * (1 - b + b * |d|/avgdl)))

Where:
- f(ti,d): Term frequency in document
- |d|: Document length
- avgdl: Average document length
- k1, b: Tuning parameters
- IDF(ti): Inverse document frequency

Simplified:
- TF: How often term appears in doc
- IDF: How rare term is across corpus
- Length normalization: Penalize long docs
```

---

## Quickwit Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Quickwit Architecture                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────┐                                       │
│  │  Indexer        │  Ingest and index documents          │
│  │  - Parse        │                                       │
│  │  - Transform    │                                       │
│  │  - Index        │                                       │
│  └────────┬────────┘                                       │
│           │                                                  │
│           ▼                                                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Object Storage (S3, GCS, Azure)          │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐               │   │
│  │  │ Split 1 │  │ Split 2 │  │ Split N │  Immutable   │   │
│  │  │ (index) │  │ (index) │  │ (index) │  segments    │   │
│  │  └─────────┘  └─────────┘  └─────────┘               │   │
│  └──────────────────────────────────────────────────────┘   │
│           │                                                  │
│           ▼                                                  │
│  ┌─────────────────┐                                       │
│  │  Searcher       │  Execute queries                      │
│  │  - Parse query  │                                       │
│  │  - Fetch splits │                                       │
│  │  - Aggregate    │                                       │
│  └────────┬────────┘                                       │
│           │                                                  │
│           ▼                                                  │
│  ┌─────────────────┐                                       │
│  │  Metastore      │  Index metadata                       │
│  │  (SQLite/PG)    │  - Split locations                    │
│  │                 │  - Schema                             │
│  │                 │  - Retention policies                 │
│  └─────────────────┘                                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Actor Model

Quickwit uses an actor-based concurrency model:

```rust
// From quickwit-actors
pub trait Actor: Sized + Send + Sync + 'static {
    type ObservableState: Clone;

    // Process a message
    async fn process_message(
        &mut self,
        message: Self::Message,
        ctx: &ActorContext<Self::Message>,
    ) -> Result<(), ActorProcessingError>;

    // Optional: finalization hook
    async fn finalize(&mut self, _ctx: &ActorContext<Self::Message>) -> Result<(), ActorProcessingError> {
        Ok(())
    }
}

// Actor lifecycle:
// Spawn → Process messages → (Supervisor monitors) → Finalize → Exit
```

### Universe (Actor Runtime)

```rust
use quickwit_actors::{Universe, Actor, ActorContext, Mailbox};

struct PingActor {
    ping_count: usize,
}

impl Actor for PingActor {
    type ObservableState = usize;
    type Message = Ping;

    fn observable_state(&self) -> Self::ObservableState {
        self.ping_count
    }

    async fn process_message(
        &mut self,
        _message: Ping,
        _ctx: &ActorContext<Ping>,
    ) -> Result<(), ActorProcessingError> {
        self.ping_count += 1;
        Ok(())
    }
}

#[derive(Debug)]
struct Ping;

// Usage
let universe = Universe::new();
let (mailbox, handle) = universe.spawn_builder().spawn(PingActor { ping_count: 0 });
mailbox.send_message(Ping).await?;
```

---

## Inverted Indexes

### Index Structure

```
┌─────────────────────────────────────────┐
│         Quickwit Index Structure        │
├─────────────────────────────────────────┤
│                                         │
│  Split (immutable segment)              │
│  ┌─────────────────────────────────┐    │
│  │  Split Meta                     │    │
│  │  - Split ID                     │    │
│  │  - Time range                   │    │
│  │  - Doc count                    │    │
│  │  - Footer offset                │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │  Inverted Index                 │    │
│  │  - Term dictionary              │    │
│  │  - Posting lists                │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │  Stored Fields                  │    │
│  │  - Document store               │    │
│  │  - Columnar storage             │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │  Fast Fields                    │    │
│  │  - Columnar numeric fields      │    │
│  │  - For aggregations             │    │
│  └─────────────────────────────────┘    │
│  ┌─────────────────────────────────┐    │
│  │  Footer                         │    │
│  │  - Checksums                    │    │
│  │  - Index pointers               │    │
│  └─────────────────────────────────┘    │
│                                         │
└─────────────────────────────────────────┘
```

### Term Dictionary

```
FST (Finite State Transducer) for term dictionary:

Term Dictionary (FST):
┌─────────────────────────────────────────┐
│  apple      → offset 0x0000             │
│  application → offset 0x0042            │
│  banana     → offset 0x0089             │
│  band       → offset 0x00C1             │
│  bandana    → offset 0x00F8             │
└─────────────────────────────────────────┘

FST Compression:
- Shared prefixes stored once
- "band" and "bandana" share "band" prefix
- O(1) lookup, compact representation
```

### Posting List Encoding

```rust
// From tantivy (Quickwit's underlying library)
struct PostingList {
    doc_ids: Vec<u32>,      // Delta-encoded
    term_freqs: Vec<u32>,   // Optional
    positions: Vec<u32>,    // Optional, for phrase queries
}

// Delta encoding:
// Doc IDs: [1, 5, 12, 15] → Deltas: [1, 4, 7, 3]
// Smaller numbers = better compression

// Bitpacking compression:
// If all deltas < 8, use 3 bits per delta
// 32 deltas fit in 12 bytes (vs 128 bytes uncompressed)
```

### Field Norms

```
Field norms for scoring:
- Document length per field
- Used for BM25 length normalization
- Stored as single byte (quantized)

Quantization:
norm_code = floor(255 * (1.0 / sqrt(doc_length)))

Reconstruction:
approx_length = (255 / norm_code)²
```

---

## Distributed Query Execution

### Query Flow

```
┌─────────────────────────────────────────────────────────────┐
│                Distributed Query Execution                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Client Query: "error AND timeout from 2024-01-01"         │
│       │                                                      │
│       ▼                                                      │
│  ┌─────────────────────────┐                               │
│  │  Search Coordinator     │                               │
│  │  1. Parse query         │                               │
│  │  2. Query metastore    │                               │
│  │  3. Identify splits     │                               │
│  └────────────┬────────────┘                               │
│               │                                              │
│       ┌───────┴───────┬──────────────┐                      │
│       ▼               ▼              ▼                      │
│  ┌─────────┐    ┌─────────┐   ┌─────────┐                  │
│  │Searcher │    │Searcher │   │Searcher │  Parallel        │
│  │   A     │    │   B     │   │   C     │  Execution       │
│  │         │    │         │   │         │                  │
│  │ Fetch   │    │ Fetch   │   │ Fetch   │                  │
│  │ splits  │    │ splits  │   │ splits  │                  │
│  │         │    │         │   │         │                  │
│  │ Execute │    │ Execute │   │ Execute │                  │
│  │ query   │    │ query   │   │ query   │                  │
│  │         │    │         │   │         │                  │
│  │ Partial │    │ Partial │   │ Partial │                  │
│  │ results │    │ results │   │ results │                  │
│  └────┬────┘    └────┬────┘   └────┬────┘                  │
│       │              │             │                        │
│       └──────────────┴─────────────┘                        │
│                      │                                       │
│                      ▼                                       │
│            ┌─────────────────┐                              │
│            │  Merge Results  │                              │
│            │  - Combine hits │                              │
│            │  - Re-sort      │                              │
│            │  - Aggregate    │                              │
│            └────────┬────────┘                              │
│                     │                                        │
│                     ▼                                        │
│              Final Results                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Split Pruning

```rust
// Time-based pruning
fn prune_splits_by_time(
    splits: &[Split],
    time_range: Option<Range<i64>>,
) -> Vec<Split> {
    splits.iter()
        .filter(|split| {
            match time_range {
                Some(range) => {
                    split.time_range.overlaps(&range)
                }
                None => true,
            }
        })
        .cloned()
        .collect()
}

// Term-based pruning using bloom filters
fn prune_splits_by_terms(
    splits: &[Split],
    query_terms: &[Term],
) -> Vec<Split> {
    splits.iter()
        .filter(|split| {
            // Check bloom filter for each term
            query_terms.iter().all(|term| {
                split.bloom_filter.might_contain(term)
            })
        })
        .cloned()
        .collect()
}
```

### Parallel Search

```rust
use tokio::task::JoinSet;

async fn search_parallel(
    searchers: &[Searcher],
    query: &Query,
) -> Result<SearchResults> {
    let mut join_set = JoinSet::new();

    // Spawn search tasks
    for searcher in searchers {
        let query_clone = query.clone();
        let searcher_clone = searcher.clone();

        join_set.spawn(async move {
            searcher_clone.search(&query_clone).await
        });
    }

    // Collect results
    let mut all_results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        all_results.push(result??);
    }

    // Merge results
    Ok(merge_results(all_results))
}
```

---

## Aggregation

### Aggregation Types

```rust
// Quickwit/Tantivy aggregations
enum Aggregation {
    // Metric aggregations
    Min { field: String },
    Max { field: String },
    Avg { field: String },
    Sum { field: String },
    Count,

    // Bucket aggregations
    Terms {
        field: String,
        size: usize,
    },
    Range {
        field: String,
        ranges: Vec<Range<f64>>,
    },
    DateHistogram {
        field: String,
        interval: Duration,
    },

    // Pipeline aggregations
    MovingAvg { buckets_path: String },
    CumulativeSum { buckets_path: String },
}
```

### Columnar Storage for Fast Fields

```
Fast Fields (columnar storage):
┌─────────────────────────────────────────┐
│  Doc ID  │  timestamp  │  response_time │
├─────────────────────────────────────────┤
│    0     │  1704067200 │     125        │
│    1     │  1704067201 │     89         │
│    2     │  1704067202 │     234        │
│    3     │  1704067203 │     156        │
│    ...   │     ...     │      ...       │
└─────────────────────────────────────────┘

Columnar layout:
- All timestamps together
- All response_times together
- Efficient for aggregations
- Skip irrelevant columns
```

### Aggregation Execution

```rust
async fn execute_aggregation(
    searcher: &Searcher,
    query: &Query,
    aggregations: &[Aggregation],
) -> Result<AggregationResults> {
    // 1. Find matching documents
    let doc_ids = searcher.search(query).await?;

    // 2. Load fast field values
    let mut agg_state = AggregationState::new(aggregations);

    for doc_id in doc_ids {
        let values = searcher.load_fast_fields(doc_id).await?;

        // 3. Update aggregation state
        for agg in aggregations {
            match agg {
                Aggregation::Min { field } => {
                    agg_state.update_min(field, values[field]);
                }
                Aggregation::Terms { field, size } => {
                    agg_state.update_terms(field, values[field], *size);
                }
                // ... other aggregation types
            }
        }
    }

    // 4. Finalize and return
    Ok(agg_state.finalize())
}
```

---

## Object Storage Native Design

### Split Format

```
Split stored on S3:
s3://bucket/index-id/splits/{split_id}.split

Split file structure:
┌─────────────────────────────────────────┐
│  Header                                 │
│  - Magic number                         │
│  - Version                              │
│  - Compression type                     │
├─────────────────────────────────────────┤
│  Term Dictionary (FST)                  │
├─────────────────────────────────────────┤
│  Posting Lists (compressed)             │
├─────────────────────────────────────────┤
│  Stored Fields                          │
├─────────────────────────────────────────┤
│  Fast Fields (columnar)                 │
├─────────────────────────────────────────┤
│  Footer                                 │
│  - Checksums                            │
│  - Index offsets                        │
└─────────────────────────────────────────┘

Typical split size: 100MB - 1GB
```

### Metastore

```rust
// Metastore stores split metadata
struct SplitMetadata {
    split_id: String,
    index_id: String,
    partition_id: u64,
    source_id: String,
    num_docs: usize,
    uncompressed_docs_size_in_bytes: u64,
    time_range: Option<Range<i64>>,
    create_timestamp: i64,
    maturity: Maturity,
    footer_offsets: Range<u64>,
}

// Metastore operations
trait Metastore: Send + Sync + 'static {
    // Index management
    async fn create_index(&self, index_metadata: IndexMetadata) -> MetastoreResult<()>;
    async fn delete_index(&self, index_id: &str) -> MetastoreResult<()>;

    // Split management
    async fn add_splits(&self, index_id: &str, splits: Vec<SplitMetadata>) -> MetastoreResult<()>;
    async fn list_splits(&self, query: SplitQuery) -> MetastoreResult<Vec<SplitMetadata>>;
    async fn mark_splits_for_deletion(&self, index_id: &str, split_ids: &[&str]) -> MetastoreResult<()>;
    async fn delete_splits(&self, index_id: &str, split_ids: &[&str]) -> MetastoreResult<()>;
}
```

### Cache Strategy

```rust
// Local cache for hot splits
struct SplitCache {
    cache_dir: PathBuf,
    max_size: u64,
    current_size: AtomicU64,
}

impl SplitCache {
    async fn get(&self, split_id: &str) -> Option<SplitHandle> {
        // Check local cache first
        if let Ok(file) = File::open(self.cache_path(split_id)).await {
            return Some(SplitHandle::Local(file));
        }

        // Download from object storage
        let split = self.download_from_object_storage(split_id).await?;
        self.add_to_cache(split_id, &split).await;
        Some(SplitHandle::Local(split))
    }

    async fn download_from_object_storage(&self, split_id: &str) -> Option<File> {
        // Download from S3/GCS/Azure
        // ...
    }
}
```

---

## Code Examples

### Basic Search Example

```rust
use quickwit_search::{SearchRequest, SearchService};
use quickwit_metastore::Metastore;

async fn search_logs(
    search_service: &dyn SearchService,
    index_id: &str,
    query: &str,
    time_range: Option<Range<i64>>,
) -> Result<SearchResponse> {
    let search_request = SearchRequest {
        index_id: index_id.to_string(),
        query: query.to_string(),
        time_range,
        max_hits: 100,
        start_offset: 0,
        sort_by_field: Some(SortField {
            field: "timestamp".to_string(),
            order: SortOrder::Desc,
        }),
        ..Default::default()
    };

    search_service.root_search(search_request).await
}

// Usage
let response = search_logs(
    &search_service,
    "otel-logs-v0_6",
    "severity_text:ERROR AND body:*timeout*",
    Some(now - Duration::from_hours(1)..now),
).await?;

for hit in response.hits {
    println!("{}: {}", hit.timestamp, hit.json);
}
```

### Aggregation Example

```rust
use quickwit_proto::search::{SearchRequest, AggregationRequest};
use tantivy::aggregation::{Aggregation, TermsAggregation};

async fn error_rate_by_service(
    search_service: &dyn SearchService,
    index_id: &str,
    time_range: Range<i64>,
) -> Result<AggregationResponse> {
    let search_request = SearchRequest {
        index_id: index_id.to_string(),
        query: "*".to_string(),
        time_range: Some(time_range),
        max_hits: 0,  // We only want aggregations
        aggregation_request: Some(AggregationRequest {
            aggs: vec![
                // Bucket by service
                Aggregation {
                    name: "services".to_string(),
                    agg_type: AggregationType::Terms(TermsAggregation {
                        field: "service_name".to_string(),
                        size: 20,
                    }),
                    aggs: vec![
                        // Count total per service
                        Aggregation {
                            name: "total".to_string(),
                            agg_type: AggregationType::Count,
                            aggs: vec![],
                        },
                        // Count errors per service
                        Aggregation {
                            name: "errors".to_string(),
                            agg_type: AggregationType::Count,
                            filter: Some("severity_text:ERROR".to_string()),
                            aggs: vec![],
                        },
                    ],
                },
            ],
        }),
        ..Default::default()
    };

    let response = search_service.root_search(search_request).await?;

    // Calculate error rates
    let agg_result = response.aggregation.unwrap();
    for bucket in agg_result["services"]["buckets"].as_array().unwrap() {
        let service = bucket["key"].as_str().unwrap();
        let total = bucket["total"]["value"].as_u64().unwrap();
        let errors = bucket["errors"]["value"].as_u64().unwrap();
        let error_rate = errors as f64 / total as f64 * 100.0;
        println!("{}: {:.2}% error rate ({}/{})", service, error_rate, errors, total);
    }

    Ok(())
}
```

### Index Configuration

```yaml
# index-config.yaml
version: 0.6
index_id: otel-logs
index_uri: s3://my-bucket/quickwit/otel-logs

doc_mapping:
  mode: lenient
  field_mappings:
    - name: timestamp
      type: datetime
      input_formats:
        - unix_timestamp
      output_format: unix_timestamp_secs
      fast: true

    - name: severity_text
      type: text
      tokenizer: raw
      fast: true

    - name: body
      type: text
      tokenizer: default
      record: position
      fieldnorms: true

    - name: resource_attributes
      type: json
      tokenizer: default

search_settings:
  default_search_fields: [severity_text, body]

retention:
  period: 30 days
  schedule: daily
```

---

## Summary

### Key Takeaways

1. **Quickwit** is cloud-native search designed for object storage
2. **Actor model** provides clean concurrency and fault tolerance
3. **Inverted indexes** with FST dictionaries and compressed posting lists
4. **Distributed query execution** with parallel search and result merging
5. **Columnar fast fields** enable efficient aggregations
6. **Object storage native** design:
   - Immutable splits
   - Metastore for metadata
   - Local caching for performance

### Further Reading

- [Quickwit Documentation](https://quickwit.io/docs/)
- [Tantivy Documentation](https://github.com/quickwit-oss/tantivy)
- [Lucene Paper](https://lucene.apache.org/core/)
- [The Power of Columnar Storage](https://github.com/quickwit-oss/tantivy/blob/main/doc/fastfield.md)
