---
title: SST Format, Block Structure, and xs Usage Patterns
---

# SST Format, Block Structure, and xs Usage Patterns

**This document covers the SST (Sorted String Table) file format used by lsm-tree, the block-level structure with compression and checksums, bloom filters, and how xs specifically configures fjall for its stream store workload.**

## SST File Layout

Source: `lsm-tree/src/table/` (SST = Sorted String Table)

An SST file is an immutable, sorted list of key-value pairs split into compressed blocks:

```
SST File:
  ┌──────────────────────────────────────────────────┐
  │ DATA BLOCK 0 (compressed key-value pairs)        │
  ├──────────────────────────────────────────────────┤
  │ DATA BLOCK 1                                     │
  ├──────────────────────────────────────────────────┤
  │ ...                                              │
  ├──────────────────────────────────────────────────┤
  │ DATA BLOCK N                                     │
  ├──────────────────────────────────────────────────┤
  │ BLOOM FILTER (skip non-existent keys)            │
  ├──────────────────────────────────────────────────┤
  │ BLOCK INDEX (binary or hash index)               │
  ├──────────────────────────────────────────────────┤
  │ META BLOCK (compression, format version)         │
  ├──────────────────────────────────────────────────┤
  │ RESTART INTERVALS (full keys for binary search)  │
  ├──────────────────────────────────────────────────┤
  │ FOOTER (checksum, magic number)                  │
  └──────────────────────────────────────────────────┘
```

## Block Structure

Source: `lsm-tree/src/table/block/`

Each data block has:

```
Block:
  ┌─────────────────────────────────────┐
  │ Header (17 bytes):                  │
  │   block_type    (1 byte)            │
  │   checksum      (16 bytes, 128-bit) │
  │   data_length   (4 bytes)           │
  │   uncompressed_length (4 bytes)     │
  ├─────────────────────────────────────┤
  │ Data (compressed key-value pairs)   │
  │   [key1→val1][key2→val2]...         │
  ├─────────────────────────────────────┤
  │ Trailer (restart points)            │
  └─────────────────────────────────────┘
```

### Block Header

```rust
pub struct Header {
    pub block_type: BlockType,       // 1 byte: data, index, filter, or meta
    pub checksum: Checksum,           // 16 bytes: 128-bit hash (xxHash128)
    pub data_length: u32,             // 4 bytes: compressed data size
    pub uncompressed_length: u32,     // 4 bytes: original data size
}
```

**Aha:** lsm-tree uses **xxHash128** for block checksums, not CRC32 or SHA-256. xxHash is a non-cryptographic hash that's extremely fast (can hash GB/s) while still providing excellent collision resistance. For a storage engine where you're checksumming millions of blocks, this matters.

### Prefix Compression

Within a data block, keys use prefix compression. Since keys are sorted, consecutive keys share prefixes:

```
Block data (logical):
  "user:1"        → "Alice"
  "user:2"        → "Bob"
  "user:3"        → "Charlie"
  "user:profile:1" → "Profile A"

Block data (encoded):
  [6]"user:1"     → "Alice"     (full key)
  [1]"2"          → "Bob"       (shared 5 bytes)
  [1]"3"          → "Charlie"   (shared 5 bytes)
  [11]"user:profile:1" → "Profile A" (full key - restart point)
```

**Restart intervals**: Every N keys (default 16), a full key is stored. This allows binary search within the block without decompressing all keys.

### Compression

Source: `lsm-tree/src/compression.rs`

Blocks can be compressed with LZ4:

```rust
pub enum CompressionType {
    None,
    #[cfg(feature = "lz4")]
    Lz4,
}
```

LZ4 is chosen for its decompression speed — decompression is typically faster than memory bandwidth, so reading compressed data can be faster than reading uncompressed data.

## Block Index

Source: `lsm-tree/src/table/block_index/`

The block index maps keys to block handles (offset + size in the file). There are two index types:

### Binary Index (Default)

A sorted list of key-block_handle pairs. Binary search finds the block containing a key.

```
Index Block:
  [key1] → BlockHandle(offset=0, size=1024)
  [key16] → BlockHandle(offset=1024, size=1024)
  [key32] → BlockHandle(offset=2048, size=1024)
  ...
```

### Hash Index

Source: `lsm-tree/src/table/block_index/hash_index/`

A hash table mapping key hashes to block handles. Faster for point lookups, but doesn't support range scans.

```rust
// Configurable: hash_ratio = keys_per_bucket_entry
// Default: 1.0 (one entry per key)
// xs uses: 8.0 for point-read keyspace, 0.0 for prefix-scan keyspace
```

**Aha:** xs tunes this per keyspace. The `stream` keyspace (point reads by scru128 ID) uses `hash_ratio = 8.0` — a hash index with 8 keys per bucket. The `idx_topic` keyspace (prefix scans) uses `hash_ratio = 0.0` — no hash index, just binary index for range scans.

## Bloom Filters

Source: `lsm-tree/src/table/filter/`

Bloom filters let the LSM tree quickly determine if a key **definitely does not exist** in an SST file. This avoids reading data blocks for non-existent keys.

```rust
pub struct BloomFilterBuilder {
    bits_per_key: f64,  // default: 10.0 (1% false positive rate)
}
```

**How it works:**
1. For each key in the SST, add it to the Bloom filter (set bits in a bit array)
2. On lookup, check the Bloom filter:
   - If bits are NOT all set → key definitely doesn't exist → skip this file
   - If bits ARE all set → key might exist → read the data block

**False positive rate:** With 10 bits per key, the false positive rate is ~1%. This means 99% of lookups for non-existent keys skip the file entirely.

## How xs Configures fjall

Source: `xs/src/store/mod.rs`

### Database Configuration

```rust
let db = Database::builder(path.join("fjall"))
    .cache_size(32 * 1024 * 1024)  // 32 MiB block cache
    .worker_threads(1)              // Single worker thread
    .open()?;
```

**32 MiB cache:** For a stream store, the working set is typically the most recent frames. A 32 MiB cache holds thousands of recent frames.

**1 worker thread:** xs does most I/O in its own threads (gc, broadcast). The fjall background thread is just for compaction.

### Stream Keyspace (Point Reads)

```rust
KeyspaceCreateOptions::default()
    .max_memtable_size(8 * 1024 * 1024)     // 8 MiB memtable
    .data_block_size_policy(BlockSizePolicy::all(16 * 1024))  // 16 KiB blocks
    .data_block_hash_ratio_policy(HashRatioPolicy::all(8.0))  // Hash index
    .expect_point_read_hits(true);
```

**8 MiB memtable:** Large enough to batch many appends before flushing, but small enough to fit in memory.

**16 KiB blocks:** Small blocks for fast random access. When reading a single frame by scru128 ID, only one 16 KiB block needs to be read.

**Hash index (ratio 8.0):** For point reads, a hash index is faster than binary search. 8 keys per bucket means the index is 8x smaller than a full binary index.

### Topic Index Keyspace (Prefix Scans)

```rust
KeyspaceCreateOptions::default()
    .max_memtable_size(8 * 1024 * 1024)
    .data_block_size_policy(BlockSizePolicy::all(16 * 1024))
    .data_block_hash_ratio_policy(HashRatioPolicy::all(0.0))  // No hash index
    .expect_point_read_hits(true);
```

**Hash ratio 0.0:** No hash index — the topic index is primarily used for prefix scans (e.g., "user.*" wildcard queries), where a binary index is more efficient.

### Append Operation

```rust
pub fn append(&self, mut frame: Frame) -> Result<Frame, Error> {
    let _guard = self.append_lock.lock().unwrap();  // Serialize

    frame.id = scru128::new();  // Sortable ID

    if frame.ttl != Some(TTL::Ephemeral) {
        self.insert_frame(&frame)?;
    }

    let _ = self.broadcast_tx.send(frame.clone());
    Ok(frame)
}

fn insert_frame(&self, frame: &Frame) -> Result<(), Error> {
    let encoded = serde_json::to_vec(&frame)?;
    let topic_key = idx_topic_key_from_frame(frame)?;
    let prefix_keys = idx_topic_prefix_keys(&frame.topic, &frame.id);

    let mut batch = self.db.batch();
    batch.insert(&self.stream, frame.id.as_bytes(), encoded);
    batch.insert(&self.idx_topic, topic_key, b"");
    for prefix_key in &prefix_keys {
        batch.insert(&self.idx_topic, prefix_key, b"");
    }
    batch.commit()?;
    self.db.persist(PersistMode::SyncAll)?;
    Ok(())
}
```

**Aha:** Each append is a **cross-keyspace atomic batch** — the frame goes into `stream`, the topic index goes into `idx_topic`, and prefix index keys also go into `idx_topic`. All are committed atomically via the journal.

**PersistMode::SyncAll:** After every append, the journal is synced to disk. This means every frame is durable immediately. For a stream store, this is the right trade-off — you don't want to lose messages.

### Range Queries

```rust
fn iter_frames(&self, start: Option<(&Scru128Id, bool)>) -> Box<dyn Iterator<Item = Frame> + '_> {
    let range = match start {
        Some((id, true)) => (Bound::Included(id.as_bytes().to_vec()), Bound::Unbounded),
        Some((id, false)) => (Bound::Excluded(id.as_bytes().to_vec()), Bound::Unbounded),
        None => (Bound::Unbounded, Bound::Unbounded),
    };

    Box::new(self.stream.range(range).filter_map(|guard| {
        let (key, value) = guard.into_inner().ok()?;
        Some(deserialize_frame((key, value)))
    }))
}
```

**Reverse iteration:** For `read --last N` (get the N most recent frames), xs iterates the stream keyspace in reverse order. Because scru128 IDs are sortable by time, reverse iteration gives you frames in reverse chronological order.

### TTL and Garbage Collection

```rust
enum GCTask {
    Remove(Scru128Id),
    CheckLastTTL { topic: String, keep: u32 },
    Drain(tokio::sync::oneshot::Sender<()>),
}
```

**TTL types:**
- `TTL::Time(ttl)`: Frame expires at a specific time
- `TTL::Last(n)`: Keep only the last N frames for this topic
- `TTL::Ephemeral`: Don't store the frame (only broadcast)

The GC worker runs on a dedicated thread and processes GCTask messages from the append path.

## How fjall Differs from RocksDB

| Feature | RocksDB | fjall |
|---------|---------|-------|
| Language | C++ | 100% Rust |
| Memory safety | Manual | Safe |
| Keyspaces | Column families | Keyspaces |
| Transactions | No built-in | OCC + single-writer |
| Compaction | Leveled, universal, FIFO | Leveled, tiered, FIFO, movedown, pulldown |
| Blob support | BlobDB | Built-in blob tree |
| API | C API + bindings | Native Rust |

**Aha:** fjall is designed to be simpler than RocksDB. It has fewer configuration knobs, which means fewer ways to misconfigure it. For xs's workload (append-only stream with point reads and prefix scans), fjall's defaults are already well-tuned.

## What's Next

- [00 — LSM/fjall/cacache/scru128](00-lsm-fjall-cacache-scru128.md) — Return to overview
- [01 — Architecture](01-architecture.md) — Return to architecture
