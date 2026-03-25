# Designing Inverted Indexes in a KV-Store on Object Storage - Deep Dive

## Executive Summary

Turbopuffer's FTS v2 introduces a new inverted index structure that achieves **10x smaller indexes** and dramatically better throughput. The key innovation is partitioning posting lists into fixed-size blocks optimized for object storage. This post explains the design decisions and tradeoffs.

---

## Background: What is an Inverted Index?

### The Basic Concept

An inverted index maps terms (words, values) to the documents that contain them:

```
Document Collection:
┌──────┬─────────────────────────────────────┐
│ DocID│ Content                             │
├──────┼─────────────────────────────────────┤
│ 1    │ "adrien wrote the documentation"    │
│ 2    │ "morgan reviewed adrien's code"     │
│ 3    │ "nathan and morgan deployed"        │
│ 4    │ "simon wrote tests"                 │
└──────┴─────────────────────────────────────┘

Inverted Index:
┌──────────┬─────────────────────────────┐
│ Term     │ Posting List (DocIDs)       │
├──────────┼─────────────────────────────┤
│ adrien   │ [1, 2]                      │
│ morgan   │ [2, 3]                      │
│ nathan   │ [3]                         │
│ simon    │ [4]                         │
│ wrote    │ [1, 4]                      │
│ code     │ [2]                         │
└──────────┴─────────────────────────────┘
```

### Posting List Structure

In its simplest form:

```rust
type InvertedIndex = HashMap<Term, PostingList>;
type PostingList = Vec<(DocId, Weight)>; // sorted by DocId
```

For full-text search (BM25), weights represent term frequency:

```
Term: "adrien"
Posting List: [(doc_id=1, weight=0.8), (doc_id=2, weight=0.3)]
                            ↑                       ↑
                    appears once,           appears once,
                    high TF-IDF             lower TF-IDF
```

---

## The Challenge: Object Storage Constraints

### Turbopuffer's Storage Architecture

Turbopuffer stores data in an **LSM-tree** (Log-Structured Merge tree) on object storage:

```
LSM Tree Structure:
┌─────────────────────────────────────────────────────────────┐
│  MemTable (in-memory)                                       │
│  ┌─────┬─────┬─────┬─────┬─────┐                           │
│  │ ab  │ cd  │ fg  │ ij  │ mn  │ ...                       │
│  └─────┴─────┴─────┴─────┴─────┘                           │
└─────────────────────────────────────────────────────────────┘
                          │ flush
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  SSTable L1 (small files, ~100MB each)                      │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐                           │
│  │ ab  │ │ cd  │ │ fg  │ │ ... │                           │
│  └─────┘ └─────┘ └─────┘ └─────┘                           │
└─────────────────────────────────────────────────────────────┘
                          │ compact
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  SSTable L2 (large files, ~1GB each)                        │
│  ┌─────────────┬─────────────┬─────────────┐               │
│  │  ab..gh     │  ij..op     │  qr..zz     │               │
│  └─────────────┴─────────────┴─────────────┘               │
└─────────────────────────────────────────────────────────────┘
                          │ store
                          ▼
                  Object Storage (S3/GCS)
```

### The Design Question

**How do we map inverted index data structures to KV pairs?**

The answer determines:
- Write amplification (how much extra data is written on updates)
- Read amplification (how much extra data is read on queries)
- Storage efficiency (how much overhead per entry)

---

## Design Option 1: One KV Per Posting List

### The Simplest Approach

```
Key: term (e.g., "adrien")
Value: entire posting list (all doc IDs + weights)

┌─────────────────────────────────────────────────────────────┐
│  Object Storage Keys                                         │
├─────────────────────────────────────────────────────────────┤
│  namespace/idx/text/adrien → [1, 2, 5, 7, 9, 15, ...]       │
│  namespace/idx/text/morgan → [3, 4, 5, 9, 12, ...]          │
│  namespace/idx/text/nathan → [1, 2, 3, 4, 6, 7, 8, ...]     │
└─────────────────────────────────────────────────────────────┘
```

### Pros

- **Simple**: One lookup per term
- **Query efficient**: Fetch exactly what you need

### Cons

```
Problem: Write Amplification

When adding doc 1000 to "adrien":
1. Read entire posting list (might be 1MB+)
2. Add new entry
3. Rewrite entire posting list

For popular terms with millions of postings:
- Terrible write amplification
- Expensive compaction (rewriting GB per compact)
- Not feasible at scale
```

---

## Design Option 2: One KV Per Posting

### The Opposite Extreme

```
Key: (term, doc_id)
Value: weight

┌─────────────────────────────────────────────────────────────┐
│  Object Storage Keys                                         │
├─────────────────────────────────────────────────────────────┤
│  namespace/idx/text/adrien/1 → 0.8                          │
│  namespace/idx/text/adrien/2 → 0.3                          │
│  namespace/idx/text/adrien/5 → 0.5                          │
│  namespace/idx/text/morgan/3 → 0.6                          │
│  ...                                                         │
└─────────────────────────────────────────────────────────────┘
```

### Pros

- **Update efficient**: Each write is tiny
- **Compaction friendly**: LSM handles small KV pairs well

### Cons

```
Problem 1: Key Overhead

Each KV stores the full key prefix:
- Key: "namespace/idx/text/adrien/1" (30+ bytes)
- Value: 4 bytes (float weight)
- Overhead: 88%+ wasted space on metadata

For 1 billion postings:
- 30GB+ just in key prefixes!
```

```
Problem 2: Query Inefficiency

To fetch posting list for "adrien":
- Must scan/list all keys with prefix "adrien/"
- Potentially millions of LIST/GET operations
- Latency nightmare
```

---

## Design Option 3: Partitioned Posting Lists (The Solution)

### The Middle Ground

```
Partition each posting list into BLOCKS:

Term: "adrien" with 1 million postings
┌─────────────────────────────────────────────────────────────┐
│  Block 0: docs 1-10000      → Key: adrien/block/0          │
│  Block 1: docs 10001-20000  → Key: adrien/block/1          │
│  ...                                                         │
│  Block 99: docs 990001-1M   → Key: adrien/block/99         │
└─────────────────────────────────────────────────────────────┘
```

### Block Structure

```rust
struct PostingBlock {
    /// Block metadata
    header: BlockHeader {
        term: String,
        block_id: u32,
        doc_id_start: DocId,
        doc_id_end: DocId,
        entry_count: u32,
        max_weight: f32,      // For block-max optimization
        compressed_size: u32,
    },

    /// Compressed posting data
    /// Using delta encoding + varint compression
    doc_id_deltas: Vec<u32>,  // Δ = current - previous
    weights: Vec<f32>,        // Or compressed weights
}
```

### Compression Techniques

```
Original Postings:
DocIDs:  [100, 105, 108, 200, 201, 500, 505]
Weights: [0.8, 0.3, 0.5, 0.2, 0.9, 0.1, 0.4]

Delta Encoding (DocIDs):
Deltas:  [100, 5, 3, 92, 1, 299, 5]
         ↑ first is absolute, rest are deltas

Varint Compression:
- Small numbers use fewer bytes
- 5 → 1 byte (0b00000101)
- 299 → 2 bytes (0b10010101 0b00000010)

Result:
- Original: 7 × 4 bytes = 28 bytes (u32)
- Compressed: ~12 bytes
- Savings: 57% reduction
```

---

## FTS v1 Design: Partition by Vector Cluster

### Background: Turbopuffer's Vector Index

Turbopuffer uses a **clustering-based vector index** (SPFresh-inspired):

```
Vector Clusters:
┌─────────────────────────────────────────────────────────────┐
│                      Centroid 0                              │
│                    (in DRAM)                                 │
│              ┌───────────┴───────────┐                      │
│              ▼                       ▼                       │
│     ┌─────────────┐          ┌─────────────┐                │
│     │ Cluster 0   │          │ Cluster 1   │                │
│     │ docs: 1-10K │          │ docs: 10K-20K│               │
│     │ (on SSD)    │          │ (on SSD)    │                │
│     └─────────────┘          └─────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

### FTS v1 Index Alignment

FTS v1 aligned posting list partitions with vector cluster boundaries:

```
┌─────────────────────────────────────────────────────────────┐
│  FTS v1 Partitioning                                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Term: "adrien"                                             │
│  ┌────────────────┬────────────────┬────────────────┐      │
│  │ Cluster 0      │ Cluster 1      │ Cluster 2      │      │
│  │ docs [1, 5, 9] │ docs [12, 15]  │ docs [25, 30]  │      │
│  │ (aligned with  │ (aligned with  │ (aligned with  │      │
│  │  vector cluster│  vector cluster│  vector cluster│      │
│  │  boundary)     │  boundary)     │  boundary)     │      │
│  └────────────────┴────────────────┴────────────────┘      │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Key Format:
namespace/idx/text/{term}/cluster_{id}
```

### Limitations

```
Problem: Irregular Partition Sizes

Vector clusters have variable sizes:
- Cluster 0: 5000 docs
- Cluster 1: 15000 docs
- Cluster 2: 3000 docs

Posting list sizes vary wildly:
- Common term in large cluster: 10MB+
- Rare term in small cluster: 100 bytes

Results in:
- Inefficient block sizes
- Unpredictable read patterns
- Suboptimal cache utilization
```

---

## FTS v2 Design: Fixed-Size Posting Blocks

### The New Approach

FTS v2 uses **fixed-size blocks** regardless of vector cluster boundaries:

```
┌─────────────────────────────────────────────────────────────┐
│  FTS v2 Fixed-Size Blocks                                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Term: "adrien" (2 million postings)                        │
│                                                             │
│  ┌──────────┬──────────┬──────────┬──────────┐            │
│  │ Block 0  │ Block 1  │   ...    │ Block N  │            │
│  │ 10K docs │ 10K docs │          │ 10K docs │            │
│  │ ~400KB   │ ~400KB   │          │ ~400KB   │            │
│  └──────────┴──────────┴──────────┴──────────┘            │
│                                                             │
│  Target block size: 256KB - 1MB (configurable)             │
│  Documents per block: ~10K (varies by compression)         │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Key Format:
namespace/idx/text/{term}/{block_id}
```

### Block Metadata Index

```rust
/// Stored separately for quick lookup
struct PostingListIndex {
    term: Term,
    total_docs: u64,
    blocks: Vec<BlockMeta>,
}

struct BlockMeta {
    block_id: u32,
    doc_id_start: DocId,
    doc_id_end: DocId,
    byte_offset: u64,      // In combined object
    byte_size: u32,
    max_weight: f32,       // For block-max WAND/MAXSCORE
    min_doc_id: DocId,     // For pruning
}
```

### Key Innovations

**1. Predictable Block Sizes**
```
All blocks are 256KB - 1MB:
- Predictable memory footprint
- Efficient prefetching
- Optimal for SSD page cache
- Consistent decompression latency
```

**2. Vectorized Batch Processing**
```
Blocks sized for SIMD processing:
- Load entire block into L3 cache
- Process 8-16 documents per SIMD instruction
- Amortize decompression overhead across many docs
```

**3. Efficient Compaction**
```
During LSM compaction:
- Merge adjacent blocks (combine small ones)
- Split oversized blocks
- Rewrite only affected blocks, not entire posting list
```

---

## Query Execution with Fixed-Size Blocks

### Block Retrieval Strategy

```rust
async fn execute_query(
    term: &str,
    doc_filter: Option<Bitmap>,
) -> Result<PostingList> {
    // 1. Fetch block metadata (small, cached)
    let index = fetch_block_index(term).await?;

    // 2. Identify relevant blocks
    let relevant_blocks = index.blocks.iter()
        .filter(|block| {
            // Prune blocks that can't contribute
            if let Some(max_weight) = block.max_weight {
                if max_weight < score_threshold {
                    return false;
                }
            }
            true
        })
        .collect();

    // 3. Fetch blocks in parallel (batch GET requests)
    let blocks = fetch_blocks_in_parallel(&relevant_blocks).await?;

    // 4. Decompress and merge
    let mut posting_list = PostingList::new();
    for block in blocks {
        let docs = block.decompress();
        posting_list.extend(docs);
    }

    Ok(posting_list)
}
```

### Parallel Fetch Optimization

```
Query: "adrien" OR "morgan" OR "nathan"

Without parallelization:
1. Fetch "adrien" blocks (100ms)
2. Fetch "morgan" blocks (100ms)
3. Fetch "nathan" blocks (100ms)
Total: 300ms

With parallelization:
1. Issue all GET requests concurrently
2. Wait for all to complete (~100ms)
Total: ~100ms (3x improvement)
```

### Block-Max Optimization

```rust
/// Using pre-computed block max scores for pruning
fn should_fetch_block(
    block_max: f32,
    current_threshold: f32,
) -> bool {
    // If block's max possible score < threshold, skip it
    block_max >= current_threshold
}

// During query execution:
for block in blocks {
    if !should_fetch_block(block.max_weight, threshold) {
        continue; // Skip fetch entirely!
    }

    // Only fetch blocks that can contribute
    let docs = fetch_block(block.id);
    // ... process docs
}
```

---

## Performance Comparison

### Index Size

| Version | Index Size (5M docs) | Compression Ratio |
|---------|---------------------|-------------------|
| FTS v1 | ~10 GB | 3x |
| FTS v2 | ~1 GB | 30x |

### Query Latency (p90, hot cache)

| Query Type | FTS v1 | FTS v2 | Improvement |
|------------|--------|--------|-------------|
| Single term | 50ms | 5ms | 10x |
| Multi-term (5) | 150ms | 10ms | 15x |
| Block-max WAND | 200ms | 10ms | 20x |

### Write Amplification

| Operation | FTS v1 | FTS v2 |
|-----------|--------|--------|
| Add 1 doc | ~10KB rewrite | ~400B (1 block) |
| Delete 1 doc | ~10KB rewrite | ~400B (tombstone) |
| Compaction | GB/s | 100MB/s |

---

## Implementation Details

### Block Encoding Format

```rust
/// Physical layout of a posting block on disk
struct EncodedPostingBlock {
    /// Magic number for validation
    magic: u32,              // 4 bytes

    /// Version for compatibility
    version: u16,            // 2 bytes

    /// Number of postings in block
    count: u16,              // 2 bytes

    /// Compressed doc IDs (delta + varint)
    doc_ids_compressed: Vec<u8>,

    /// Compressed weights (optional - can be computed)
    weights_compressed: Vec<u8>,

    /// CRC32 checksum
    checksum: u32,           // 4 bytes
}

impl EncodedPostingBlock {
    fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend(&self.magic.to_le_bytes());
        buf.extend(&self.version.to_le_bytes());
        buf.extend(&self.count.to_le_bytes());
        buf.extend(&self.doc_ids_compressed);
        buf.extend(&self.weights_compressed);
        buf.extend(&self.checksum.to_le_bytes());
        buf
    }

    fn decode(data: &[u8]) -> Result<Self> {
        // Parse header
        let magic = u32::from_le_bytes(data[0..4].try_into()?);
        let version = u16::from_le_bytes(data[4..6].try_into()?);
        let count = u16::from_le_bytes(data[6..8].try_into()?);

        // Decompress body
        // ...

        Ok(Self { ... })
    }
}
```

### Compression Implementation

```rust
/// Delta encoding + varint compression for doc IDs
fn compress_doc_ids(doc_ids: &[DocId]) -> Vec<u8> {
    let mut compressed = Vec::new();

    let mut prev = 0u32;
    for &doc_id in doc_ids {
        let delta = doc_id - prev;
        prev = doc_id;

        // Varint encoding
        let mut value = delta;
        while value >= 0x80 {
            compressed.push((value as u8) | 0x80); // continuation bit
            value >>= 7;
        }
        compressed.push(value as u8);
    }

    compressed
}

/// Decompression
fn decompress_doc_ids(compressed: &[u8]) -> Vec<DocId> {
    let mut doc_ids = Vec::new();
    let mut prev = 0u32;
    let mut offset = 0;

    while offset < compressed.len() {
        let mut delta = 0u32;
        let mut shift = 0;

        loop {
            let byte = compressed[offset];
            offset += 1;

            delta |= ((byte & 0x7F) as u32) << shift;

            if byte < 0x80 {
                break; // Last byte
            }
            shift += 7;
        }

        prev += delta;
        doc_ids.push(prev);
    }

    doc_ids
}
```

### Caching Strategy

```rust
/// Multi-level cache for posting blocks
struct PostingCache {
    /// L1: Hot blocks (frequently accessed terms)
    l1_cache: MokaCache<BlockKey, CompressedBlock>,

    /// L2: Warm blocks (recent access)
    l2_cache: MokaCache<BlockKey, CompressedBlock>,

    /// L3: DRAM buffer (prefetched blocks)
    l3_buffer: DashMap<BlockKey, CompressedBlock>,
}

impl PostingCache {
    async fn get(&self, key: &BlockKey) -> Option<CompressedBlock> {
        // Check L1 first (fastest)
        if let Some(block) = self.l1_cache.get(key) {
            return Some(block);
        }

        // Check L2
        if let Some(block) = self.l2_cache.get(key) {
            self.l1_cache.insert(key.clone(), block.clone());
            return Some(block);
        }

        // Fetch from storage
        let block = fetch_block_from_storage(key).await?;

        // Insert into cache hierarchy
        if is_hot_term(key.term) {
            self.l1_cache.insert(key.clone(), block.clone());
        } else {
            self.l2_cache.insert(key.clone(), block.clone());
        }

        Some(block)
    }
}
```

---

## Tradeoffs and Lessons Learned

### What We Tried That Didn't Work

**Attempt 1: Variable-Size Blocks**
```
Initial idea: Let blocks grow naturally based on document distribution

Problem:
- Some blocks became 10MB+ (popular terms)
- Decompression latency was unpredictable
- Memory pressure from large blocks

Solution: Fixed-size blocks with explicit splitting
```

**Attempt 2: Global Block Index**
```
Initial idea: Single index for all terms

Problem:
- Index itself became GBs
- Had to paginate index lookups (ironic!)

Solution: Per-term index objects (small, cacheable)
```

**Attempt 3: Aggressive Deduplication**
```
Initial idea: Deduplicate common weight patterns

Problem:
- Dedup overhead exceeded savings
- Added complexity to decompression

Solution: Simple compression only (delta + varint)
```

### Design Principles

1. **Fixed sizes enable predictable performance**
   - Know exactly how much data you're fetching
   - Plan cache capacity accurately
   - Parallelize efficiently

2. **Metadata should be separate from data**
   - Index structures are read frequently
   - Data blocks are read selectively
   - Different caching strategies for each

3. **Compression is about access patterns, not just size**
   - Delta encoding enables SIMD-friendly sequential access
   - Block boundaries align with query parallelism
   - Trade some compression ratio for speed

4. **Object storage is the bottleneck**
   - Minimize round-trips
   - Batch reads when possible
   - Cache aggressively

---

## Summary

### Key Takeaways

1. **Partitioned posting lists** amortize per-KV overhead while keeping updates efficient

2. **Fixed-size blocks** (256KB - 1MB) enable predictable performance and efficient vectorized processing

3. **Block-max optimization** allows skipping entire blocks that can't contribute to results

4. **LSM-friendly design** ensures compaction remains efficient at scale

5. **Delta encoding + varint compression** achieves 10-30x compression while maintaining fast decompression

### Performance Achieved

- **10x smaller indexes** compared to FTS v1
- **20x faster queries** with block-max WAND
- **25x lower write amplification** on updates

### Future Improvements

- **Adaptive block sizes**: Larger blocks for cold data, smaller for hot
- **Columnar compression**: Separate encoding for different posting attributes
- **GPU decompression**: Offload decompression to GPU for massive parallelism

---

## Appendix: Comparison with Other Systems

### Apache Lucene

```
Lucene uses similar block-based approach:
- Fixed-size blocks (~8KB)
- Frame of reference (FOR) compression
- Block-max WAND support

Differences:
- Lucene optimized for local filesystem
- turbopuffer optimized for object storage
- Larger blocks better for network latency
```

### Elasticsearch

```
Elasticsearch (built on Lucene):
- Inherits Lucene's block structure
- Adds distributed sharding
- Segment-based organization

turbopuffer differences:
- Object storage native (no local FS)
- Simpler deployment model
- Tighter integration with vector search
```

### Pinecone/Weaviate

```
Managed vector databases with filtering:
- Often use post-filtering (lower recall)
- Some use partitioned indexes
- Varying approaches to block storage

turbopuffer advantage:
- Native filtering with high recall
- Object storage cost benefits
- Proven at billion-scale
```
