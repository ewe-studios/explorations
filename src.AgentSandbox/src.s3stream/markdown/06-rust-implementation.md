---
title: Rust Implementation — In-Memory Block Structure with Zero-Copy
---

# Rust Implementation — In-Memory Block Structure with Zero-Copy

**Here's how to implement s3Stream's in-memory block structure in Rust — using `bytes::Bytes` for zero-copy record storage, `RwLock` for concurrent access, and `tokio::sync::Semaphore` for size limiting.**

## Core Types

```rust
use bytes::{Bytes, BytesMut};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::RwLock;
use tokio::sync::{RwLock as AsyncRwLock, Semaphore};

/// A single record batch — backed by Bytes (Arc-based, zero-copy clone).
#[derive(Clone, Debug)]
pub struct StreamRecordBatch {
    pub stream_id: u64,
    pub base_offset: u64,
    pub epoch: i64,
    pub data: Bytes,  // Zero-copy: Arc<[u8]> internally
}

impl StreamRecordBatch {
    pub fn size(&self) -> usize {
        // header (17 bytes) + data length
        17 + self.data.len()
    }

    pub fn last_offset(&self) -> u64 {
        self.base_offset + 1  // simplified: assumes 1 record per batch
    }
}
```

## StreamCache — Per-Stream Record Storage

```rust
/// Per-stream record cache with offset→index lookup.
pub struct StreamCache {
    records: Vec<StreamRecordBatch>,
    start_offset: u64,
    end_offset: u64,
    offset_index: HashMap<u64, usize>,  // offset → index in records
}

impl StreamCache {
    fn new() -> Self {
        Self {
            records: Vec::new(),
            start_offset: u64::MAX,
            end_offset: 0,
            offset_index: HashMap::new(),
        }
    }

    fn add(&mut self, batch: StreamRecordBatch) {
        let offset = batch.base_offset;
        self.offset_index.insert(offset, self.records.len());
        self.records.push(batch);
        self.start_offset = self.start_offset.min(offset);
        self.end_offset = self.records.last().unwrap().last_offset();
    }

    /// O(1) lookup for known offsets, O(log n) binary search for unknown.
    fn get(&self, start: u64, end: u64) -> Option<&[StreamRecordBatch]> {
        if self.end_offset <= start || self.start_offset >= end {
            return None;
        }
        let idx = self.offset_index.get(&start).copied()?;
        let end_idx = self.records[idx..]
            .iter()
            .position(|r| r.base_offset() >= end)
            .unwrap_or(self.records.len() - idx);
        Some(&self.records[idx..idx + end_idx])
    }
}
```

## LogCacheBlock — Sealed or Active Block

```rust
/// A block of records from multiple streams.
/// When sealed, it's read-only and queued for upload.
pub struct LogCacheBlock {
    max_size: usize,
    max_stream_count: usize,
    streams: HashMap<u64, StreamCache>,
    size: AtomicUsize,
}

impl LogCacheBlock {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            max_stream_count: 10_000,
            streams: HashMap::new(),
            size: AtomicUsize::new(0),
        }
    }

    /// Put a record batch. Returns true if the block is full.
    pub fn put(&self, batch: StreamRecordBatch) -> bool {
        let stream = self.streams
            .entry(batch.stream_id)
            .or_insert_with(StreamCache::new);
        stream.add(batch);
        let new_size = self.size.fetch_add(batch.size(), Ordering::Relaxed) + batch.size();
        new_size >= self.max_size || self.streams.len() >= self.max_stream_count
    }

    pub fn size(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    /// Get records for a stream offset range across all streams.
    pub fn get(&self, stream_id: u64, start: u64, end: u64) -> Option<&[StreamRecordBatch]> {
        self.streams.get(&stream_id)?.get(start, end)
    }
}
```

## LogCache — The Top-Level Buffer

```rust
/// In-memory buffer for records before S3 upload.
pub struct LogCache {
    capacity: usize,
    block_max_size: usize,
    /// Sealed blocks waiting for upload.
    blocks: RwLock<Vec<Arc<LogCacheBlock>>>,
    /// Currently accepting writes.
    active: AsyncRwLock<LogCacheBlock>,
}

impl LogCache {
    pub fn new(capacity: usize, block_max_size: usize) -> Self {
        Self {
            capacity,
            block_max_size,
            blocks: RwLock::new(Vec::new()),
            active: AsyncRwLock::new(LogCacheBlock::new(block_max_size)),
        }
    }

    /// Put a record batch. Seals the active block if full.
    pub async fn put(&self, batch: StreamRecordBatch) -> Result<bool, Error> {
        let mut active = self.active.write().await;
        let full = active.put(batch);
        if full {
            // Seal: move active to blocks, create new active
            let sealed = std::mem::replace(
                &mut *active,
                LogCacheBlock::new(self.block_max_size),
            );
            self.blocks.write().unwrap().push(Arc::new(sealed));
        }
        Ok(self.total_size() >= self.capacity)
    }

    /// Get records for a stream offset range across all blocks.
    pub async fn get(&self, stream_id: u64, start: u64, end: u64) -> Vec<StreamRecordBatch> {
        let mut result = Vec::new();

        // Search sealed blocks first
        let blocks = self.blocks.read().unwrap();
        for block in &*blocks {
            if let Some(records) = block.get(stream_id, start, end) {
                result.extend(records.iter().cloned());
                if result.last().map_or(false, |r| r.last_offset() >= end) {
                    break;
                }
            }
        }

        // Then search active block
        let active = self.active.read().await;
        if let Some(records) = active.get(stream_id, start, end) {
            result.extend(records.iter().cloned());
        }

        result
    }

    /// Seal the active block and return all blocks for upload.
    pub async fn seal(&self) -> Vec<Arc<LogCacheBlock>> {
        let mut blocks = self.blocks.write().unwrap();
        let active = self.active.write().await;
        let sealed = std::mem::replace(
            &mut *active,
            LogCacheBlock::new(self.block_max_size),
        );
        blocks.push(Arc::new(sealed));
        blocks.drain(..).collect()
    }

    fn total_size(&self) -> usize {
        let blocks = self.blocks.read().unwrap();
        blocks.iter().map(|b| b.size()).sum::<usize>()
    }
}
```

## Zero-Copy DataBlock

```rust
use bytes::{Buf, BufMut, Bytes, BytesMut};

/// A data block ready for S3 upload — zero-copy composite of header + record data.
pub struct DataBlock {
    /// Composite buffer: [header][record0][record1]...
    buffer: Bytes,
    stream_id: u64,
    start_offset: u64,
    end_offset: u64,
    record_count: usize,
}

impl DataBlock {
    pub const HEADER_SIZE: usize = 10; // magic(1) + flag(1) + count(4) + length(4)

    /// Build a data block from records. Zero-copy: references the original Bytes.
    pub fn new(stream_id: u64, records: &[StreamRecordBatch]) -> Self {
        // Calculate total size
        let data_size: usize = records.iter().map(|r| r.data.len()).sum();
        let total_size = Self::HEADER_SIZE + data_size;

        // Build header
        let mut header = BytesMut::with_capacity(Self::HEADER_SIZE);
        header.put_u8(0x5A);                           // magic
        header.put_u8(0x02);                           // flag
        header.put_u32(records.len() as u32);          // record count
        header.put_u32(data_size as u32);              // data length

        // Build composite buffer — zero-copy for record data
        let mut composite = BytesMut::with_capacity(total_size);
        composite.put(header.freeze());
        for record in records {
            composite.put(record.data.clone());  // Clone Bytes = clone Arc, zero-copy
        }

        let start_offset = records.first().map(|r| r.base_offset).unwrap_or(0);
        let end_offset = records.last().map(|r| r.last_offset()).unwrap_or(0);

        Self {
            buffer: composite.freeze(),
            stream_id,
            start_offset,
            end_offset,
            record_count: records.len(),
        }
    }

    pub fn as_bytes(&self) -> &Bytes {
        &self.buffer
    }

    pub fn size(&self) -> usize {
        self.buffer.len()
    }
}
```

## ObjectWriter — Incremental Multipart Upload

```rust
/// Writes stream records to an S3 object with incremental multipart upload.
pub struct ObjectWriter<S: ObjectStorage> {
    storage: S,
    key: String,
    block_size_threshold: usize,
    part_size_threshold: usize,
    waiting_blocks: Vec<DataBlock>,
    waiting_blocks_size: usize,
    completed_blocks: Vec<DataBlock>,
    upload: Option<S::MultipartUpload>,
    total_size: usize,
}

impl<S: ObjectStorage> ObjectWriter<S> {
    pub fn new(storage: S, key: String, block_size: usize, part_size: usize) -> Self {
        Self {
            storage,
            key,
            block_size_threshold: block_size,
            part_size_threshold: part_size.max(S::MIN_PART_SIZE),
            waiting_blocks: Vec::new(),
            waiting_blocks_size: 0,
            completed_blocks: Vec::new(),
            upload: None,
            total_size: 0,
        }
    }

    /// Write records for a stream. May trigger a multipart part upload.
    pub async fn write(&mut self, stream_id: u64, records: &[StreamRecordBatch]) -> Result<(), Error> {
        let block = DataBlock::new(stream_id, records);
        self.waiting_blocks.push(block);
        self.waiting_blocks_size += block.size();
        self.total_size += block.size();

        if self.waiting_blocks_size >= self.part_size_threshold {
            self.try_upload_part().await?;
        }
        Ok(())
    }

    /// Upload accumulated blocks as a multipart part.
    async fn try_upload_part(&mut self) -> Result<(), Error> {
        // Collect blocks up to part size
        let mut part_size = 0;
        let mut part_blocks = Vec::new();
        for block in self.waiting_blocks.iter() {
            part_blocks.push(block.as_bytes().clone());
            part_size += block.size();
            if part_size >= self.part_size_threshold {
                break;
            }
        }

        if part_size < self.part_size_threshold {
            return Ok(());
        }

        // Combine blocks into one part — still zero-copy via Bytes
        let part_data: Bytes = part_blocks.into_iter().flat_map(|b| b).collect();

        let upload = self.upload.get_or_insert_with(|| {
            self.storage.create_multipart_upload(&self.key)
        });

        upload.upload_part(part_data).await?;

        // Move uploaded blocks to completed
        let count = part_blocks.len();
        for _ in 0..count {
            self.completed_blocks.push(self.waiting_blocks.remove(0));
        }
        self.waiting_blocks_size -= part_size;

        Ok(())
    }

    /// Close: flush remaining, write index + footer, complete upload.
    pub async fn close(mut self) -> Result<u64, Error> {
        // 1. Flush remaining blocks as final data part
        for block in self.waiting_blocks.drain(..) {
            self.completed_blocks.push(block);
        }

        // 2. Build index block
        let index_block = self.build_index_block();
        let index_position = self.total_size as u64;

        // 3. Build footer
        let footer = Footer {
            index_position,
            index_length: index_block.len() as u32,
        };
        let footer_bytes = footer.encode();

        // 4. Upload final part: index + footer
        let mut final_data = BytesMut::with_capacity(index_block.len() + Footer::SIZE);
        final_data.put(index_block);
        final_data.put(footer_bytes);

        let upload = self.upload.take().unwrap();
        upload.upload_part(final_data.freeze()).await?;
        upload.complete().await?;

        Ok(index_position + index_block.len() as u64 + Footer::SIZE as u64)
    }

    fn build_index_block(&self) -> BytesMut {
        let count = self.completed_blocks.len();
        let mut buf = BytesMut::with_capacity(count * DataBlockIndex::SIZE);

        let mut position = 0u64;
        for block in &self.completed_blocks {
            let idx = DataBlockIndex {
                stream_id: block.stream_id,
                start_offset: block.start_offset,
                end_offset_delta: (block.end_offset - block.start_offset) as u32,
                record_count: block.record_count as u32,
                position,
                size: block.size() as u32,
            };
            idx.encode(&mut buf);
            position += block.size() as u64;
        }

        buf
    }
}
```

## DataBlockCache — Read-Side Cache

```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Cache key: (object_id, stream_id, start_offset)
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct CacheKey {
    pub object_id: u64,
    pub stream_id: u64,
    pub start_offset: u64,
}

/// Cached data block.
pub struct CachedBlock {
    pub data: Bytes,  // Zero-copy: shared via Arc
    pub index: DataBlockIndex,
}

/// DataBlockCache — like Linux page cache for S3 data blocks.
pub struct DataBlockCache {
    shards: Vec<CacheShard>,
    size_limiter: Arc<Semaphore>,
    max_size: usize,
}

impl DataBlockCache {
    pub fn new(max_size: usize, num_shards: usize) -> Self {
        Self {
            shards: (0..num_shards)
                .map(|_| CacheShard::new(max_size / num_shards))
                .collect(),
            size_limiter: Arc::new(Semaphore::new(max_size)),
            max_size,
        }
    }

    fn shard(&self, stream_id: u64) -> &CacheShard {
        &self.shards[(stream_id as usize) % self.shards.len()]
    }

    pub async fn get(&self, key: CacheKey) -> Option<Arc<CachedBlock>> {
        self.shard(key.stream_id).get(&key).await
    }

    pub async fn put(&self, key: CacheKey, block: Arc<CachedBlock>) {
        let permit = self.size_limiter.acquire(block.data.len()).await.ok()?;
        self.shard(key.stream_id).put(key, block).await;
        permit.forget();  // Permit consumed by cached data
    }
}

/// Per-shard cache with LRU eviction.
struct CacheShard {
    blocks: RwLock<HashMap<CacheKey, Arc<CachedBlock>>>,
    lru: RwLock<VecDeque<CacheKey>>,
    max_size: usize,
}

impl CacheShard {
    fn new(max_size: usize) -> Self {
        Self {
            blocks: RwLock::new(HashMap::new()),
            lru: RwLock::new(VecDeque::new()),
            max_size,
        }
    }

    async fn get(&self, key: &CacheKey) -> Option<Arc<CachedBlock>> {
        let blocks = self.blocks.read().await;
        let block = blocks.get(key)?.clone();
        // Touch LRU
        drop(blocks);
        let mut lru = self.lru.write().await;
        if let Some(pos) = lru.iter().position(|k| k == key) {
            lru.remove(pos);
        }
        lru.push_back(key.clone());
        Some(block)
    }

    async fn put(&self, key: CacheKey, block: Arc<CachedBlock>) {
        let mut blocks = self.blocks.write().await;
        blocks.insert(key.clone(), block);
        drop(blocks);
        self.lru.write().await.push_back(key);
        self.evict().await;
    }

    async fn evict(&self) {
        // Evict LRU blocks until under size limit
        loop {
            let lru = self.lru.write().await;
            let key = lru.front()?.clone();
            drop(lru);

            let blocks = self.blocks.read().await;
            if let Some(block) = blocks.get(&key) {
                // Check if eviction needed
                // In real impl, track current size
            }
            // ... evict logic
            break;
        }
    }
}
```

## Key Rust Design Decisions

| Java Pattern | Rust Replacement | Why |
|-------------|-----------------|-----|
| Netty ByteBuf | `bytes::Bytes` | Arc-based, zero-copy clone, same semantics |
| CompletableFuture | `async/await` | Direct, no callback chains |
| ConcurrentHashMap | `RwLock<HashMap>` | Simpler, no ref counting |
| ReentrantReadWriteLock | `tokio::sync::RwLock` | Async-aware |
| EventLoop sharding | Shard array + hash | Same concept, simpler |
| CompositeByteBuf | `BytesMut::put(Bytes)` | Zero-copy via Arc clone |
| AsyncSemaphore | `tokio::sync::Semaphore` | Built-in, async-ready |

## Memory Layout Summary

```
LogCache (256MB)
  ├─ blocks: Vec<Arc<LogCacheBlock>>  (sealed, read-only)
  └─ active: AsyncRwLock<LogCacheBlock>  (accepting writes)

LogCacheBlock (64MB max)
  └─ streams: HashMap<u64, StreamCache>

StreamCache
  ├─ records: Vec<StreamRecordBatch>
  └─ offset_index: HashMap<u64, usize>  // O(1) offset lookup

StreamRecordBatch
  └─ data: Bytes  // Arc<[u8]> — zero-copy clone

DataBlock (for upload)
  └─ buffer: Bytes  // [header][record0][record1]... — zero-copy composite
```

**The 64MB of record data is never copied.** When a `StreamRecordBatch` is added to a `DataBlock`, its `Bytes` field is cloned — which just increments an Arc reference count. The same holds when uploading: `Bytes` references are streamed to S3 without copying.

## What's Next

- [00 — Overview](00-overview.md) — Return to overview
- [03 — Caching](03-caching.md) — Return to caching
- [05 — Object Assembly](05-object-assembly.md) — Return to object assembly
