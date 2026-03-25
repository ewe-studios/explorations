# Storage Architecture: Log-Structured Storage, SSTables, and Compaction

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/`

---

## Table of Contents

1. [Introduction to Log-Structured Storage](#introduction-to-log-structured-storage)
2. [Write-Ahead Log (WAL)](#write-ahead-log-wal)
3. [Memtable and Immutable Memtable](#memtable-and-immutable-memtable)
4. [SSTable Format](#sstable-format)
5. [Compaction Strategies](#compaction-strategies)
6. [ZeroFS Implementation](#zerofs-implementation)
7. [Performance Considerations](#performance-considerations)
8. [Code Examples](#code-examples)

---

## Introduction to Log-Structured Storage

### The Problem with Traditional Storage

Traditional filesystems and databases use **random I/O** for writes:

```
Traditional Write Pattern:
┌─────────────────────────────────────────┐
│  Disk Blocks                            │
│  ┌───┬───┬───┬───┬───┬───┬───┬───┐     │
│  │ A │   │ B │   │ C │   │   │ D │     │
│  └───┴───┴───┴───┴───┴───┴───┴───┘     │
│    ▲       ▲       ▲               ▲    │
│    │       │       │               │    │
│    └───────┴───────┴───────────────┘    │
│         Random seeks for each write     │
└─────────────────────────────────────────┘
```

**Problems:**
- Disk heads must seek to different locations
- SSDs have page/block write granularity
- Cloud storage (S3) has high latency per operation
- Updates require read-modify-write cycles

### The LSM Tree Solution

Log-Structured Merge (LSM) trees convert **random writes to sequential appends**:

```
LSM Write Pattern:
┌─────────────────────────────────────────┐
│  Memtable (in-memory)                   │
│  ┌─────────────────────────────────┐    │
│  │ A:value1 ← Append (fast)        │    │
│  │ B:value2 ← Append (fast)        │    │
│  │ C:value3 ← Append (fast)        │    │
│  │ D:value4 ← Append (fast)        │    │
│  └─────────────────────────────────┘    │
│              │                          │
│              │ Flush (sequential)       │
│              ▼                          │
│  ┌─────────────────────────────────┐    │
│  │  SSTable 1 (immutable)          │    │
│  │  [A, B, C, D] sorted            │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

**Benefits:**
- Sequential writes = high throughput
- In-memory buffering = low latency
- Immutable files = simple concurrency
- Cloud-native = large objects

---

## Write-Ahead Log (WAL)

### Purpose

The WAL provides **durability** before data is flushed to the main storage:

```
Write Flow:
1. Client writes data
2. Write to WAL (durability)
3. Write to Memtable (visibility)
4. Return success to client

Crash Recovery:
1. Load last checkpoint
2. Replay WAL entries
3. Reconstruct memtable
```

### WAL Structure

```rust
// Simplified WAL entry
struct WALEntry {
    sequence_number: u64,
    operation: WLOperation,
    checksum: u32,  // CRC32C or similar
}

enum WLOperation {
    Put { key: Bytes, value: Bytes },
    Delete { key: Bytes },
    Batch { entries: Vec<WLOperation> },
}
```

### WAL Formats

**Option 1: Separate WAL File**
```
/data/
├── MANIFEST          # Current state
├── 000001.wal        # Active WAL
├── 000002.wal        # Being replayed
└── 000003.sst        # Flushed SSTable
```

**Option 2: WAL in Object Storage (ZeroFS/SlateDB)**
```
s3://bucket/zerofs-data/
├── manifest.json     # Current state
├── wal/
│   ├── 000001.wal    # WAL segments
│   └── 000002.wal
└── sst/
    ├── 000001.sst    # SSTables
    └── 000002.sst
```

### WAL Trade-offs

| Aspect | With WAL | Without WAL |
|--------|----------|-------------|
| Durability | Strong (durability on write) | Weak (only on flush) |
| Write Latency | Higher (WAL write) | Lower (memory only) |
| Recovery | Fast (replay WAL) | Loss of unflushed data |
| Storage Overhead | ~2x (WAL + memtable) | 1x (memtable only) |

**ZeroFS uses WAL** for durability guarantees.

---

## Memtable and Immutable Memtable

### Memtable Structure

The memtable is an **in-memory sorted data structure**:

```rust
// Simplified memtable
struct Memtable {
    // Skip list or B-tree for sorted order
    data: BTreeMap<Bytes, ValueOrTombstone>,
    size_bytes: usize,
    sequence_number: u64,
}

enum ValueOrTombstone {
    Value(Bytes),
    Tombstone,  // Marker for deleted key
}
```

### Operations

```rust
impl Memtable {
    pub fn put(&mut self, key: Bytes, value: Bytes) {
        self.data.insert(key, ValueOrTombstone::Value(value));
        self.size_bytes += key.len() + value.len();
    }

    pub fn delete(&mut self, key: Bytes) {
        self.data.insert(key, ValueOrTombstone::Tombstone);
    }

    pub fn get(&self, key: &Bytes) -> Option<&ValueOrTombstone> {
        self.data.get(key)
    }

    pub fn should_flush(&self, threshold: usize) -> bool {
        self.size_bytes >= threshold
    }
}
```

### Immutable Memtable

When the active memtable is full, it becomes **immutable**:

```
Memtable Rotation:
┌─────────────────────────────────────────┐
│  Active Memtable                        │
│  ┌─────────────────────────────────┐    │
│  │ Receiving new writes            │    │
│  └─────────────────────────────────┘    │
│              │                          │
│              │ Full!                    │
│              ▼                          │
│  ┌─────────────────────────────────┐    │
│  │  Immutable Memtable             │    │
│  │  Being flushed to SSTable       │    │
│  └─────────────────────────────────┘    │
│              │                          │
│              │ Flush complete           │
│              ▼                          │
│  ┌─────────────────────────────────┐    │
│  │  SSTable created                │    │
│  │  Immutable memtable dropped     │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

**Why immutable?**
- No locks needed for reading
- Simple handoff to flush thread
- Consistent snapshot for reads

---

## SSTable Format

### Overview

SSTable (Sorted String Table) is an **immutable sorted file format**:

```
┌─────────────────────────────────────────┐
│           SSTable Layout                │
├─────────────────────────────────────────┤
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  Data Blocks                    │    │
│  │  ┌─────────────────────────┐    │    │
│  │  │  Block 0                │    │    │
│  │  │  [key1:val1, key2:val2] │    │    │
│  │  │  (compressed)           │    │    │
│  │  ├─────────────────────────┤    │    │
│  │  │  Block 1                │    │    │
│  │  │  [key3:val3, key4:val4] │    │    │
│  │  │  (compressed)           │    │    │
│  │  ├─────────────────────────┤    │    │
│  │  │  ...                    │    │    │
│  │  └─────────────────────────┘    │    │
│  ├─────────────────────────────────┤    │
│  │  Index Block                    │    │
│  │  - Block offsets                │    │
│  │  - First key per block          │    │
│  ├─────────────────────────────────┤    │
│  │  Bloom Filter                   │    │
│  │  - Space-efficient key lookup   │    │
│  ├─────────────────────────────────┤    │
│  │  Footer                         │    │
│  │  - Index offset                 │    │
│  │  - Bloom filter offset          │    │
│  │  - Magic number                 │    │
│  └─────────────────────────────────┘    │
│                                         │
└─────────────────────────────────────────┘
```

### Data Blocks

**Compression:**
- **LZ4**: Fast compression/decompression
- **Zstd**: Better compression ratio

```rust
struct DataBlock {
    entries: Vec<BlockEntry>,
    compression: CompressionType,
}

struct BlockEntry {
    key_overlap: u16,      // Shared prefix with previous key
    key_suffix: Bytes,     // Non-shared key portion
    value: Bytes,
}
```

**Prefix Compression:**
```
Keys without compression:
  /users/alice/profile
  /users/alice/settings
  /users/bob/profile
  /users/bob/settings

With prefix compression:
  (0, /users/alice/profile)
  (13, settings)          // 13 chars shared with previous
  (7, bob/profile)        // 7 chars shared ("/users/")
  (11, settings)          // 11 chars shared ("/users/bob/")
```

### Index Block

The index provides **efficient key lookup**:

```
Index Structure:
┌─────────────────────────────────────────┐
│  Index Entry 0                          │
│  - First key: "apple"                  │
│  - Block offset: 0                      │
├─────────────────────────────────────────┤
│  Index Entry 1                          │
│  - First key: "banana"                  │
│  - Block offset: 4096                   │
├─────────────────────────────────────────┤
│  Index Entry 2                          │
│  - First key: "cherry"                  │
│  - Block offset: 8192                   │
└─────────────────────────────────────────┘

Lookup "blueberry":
1. Binary search index
2. Find "banana" <= "blueberry" < "cherry"
3. Read Block 1
4. Binary search within block
```

### Bloom Filter

Bloom filters provide **fast key existence checks**:

```rust
// Simplified Bloom filter
struct BloomFilter {
    bits: BitSet,
    hash_functions: Vec<HashFunction>,
}

impl BloomFilter {
    pub fn add(&mut self, key: &Bytes) {
        for hash in &self.hash_functions {
            let pos = hash(key) % self.bits.len();
            self.bits.set(pos, true);
        }
    }

    pub fn might_contain(&self, key: &Bytes) -> bool {
        self.hash_functions.iter().all(|hash| {
            let pos = hash(key) % self.bits.len();
            self.bits[pos]
        })
    }
}
```

**Properties:**
- **No false negatives**: If filter says "no", key definitely not present
- **Possible false positives**: If filter says "maybe", key might not be present
- **Space efficient**: ~10 bits per key for 1% false positive rate

**Impact:**
```
Without Bloom Filter:
  Read(key) → Check all SSTables → O(n) disk reads

With Bloom Filter:
  Read(key) → Check filter → Read only relevant SSTable → O(1) disk reads
```

---

## Compaction Strategies

### Why Compaction?

Over time, LSM trees accumulate:
1. **Obsolete data**: Old versions of keys
2. **Tombstones**: Deleted keys
3. **Many small files**: Inefficient reads

Compaction merges SSTables to:
- Remove obsolete data
- Reduce file count
- Improve read performance

### Tiered Compaction

**Structure:**
```
Level 0:  [A] [B] [C]  (unsorted, overlapping)
Level 1:  [D--------E] (sorted, non-overlapping)
Level 2:  [F--------G]
```

**Process:**
```
1. L0 files accumulate
2. When threshold reached:
   - Select L0 files + overlapping L1 files
   - Merge and sort
   - Write to L1
3. Repeat for L1 → L2, etc.
```

**Pros:**
- Fast writes (just drop files)
- Lower write amplification

**Cons:**
- Higher read amplification (check multiple files)
- Space amplification (duplicate keys)

### Leveled Compaction

**Structure:**
```
Level 0:  [A] [B] [C]     (small, overlapping)
Level 1:  [D] [E] [F]     (sorted, non-overlapping)
Level 2:  [G] [H] [I]     (larger, non-overlapping)
```

**Process:**
```
1. L0 files compacted to L1
2. L1 files compacted to L2 (if overlapping)
3. Each level 10x larger than previous
```

**Pros:**
- Better read performance (one file per level)
- Lower space amplification

**Cons:**
- Higher write amplification (resort everything)

### FIFO Compaction

**Strategy:**
```
Files in chronological order:
┌─────┬─────┬─────┬─────┬─────┐
│  1  │  2  │  3  │  4  │  5  │
└─────┴─────┴─────┴─────┴─────┘
  ▲                         │
  │                         │
  └────── Drop oldest ──────┘
```

**Use case:**
- Time-series data
- Only recent data matters

### ZeroFS Compaction

ZeroFS uses **compaction with a standalone compactor option**:

```toml
# zerofs.toml
[lsm]
max_concurrent_compactions = 4
compaction_strategy = "leveled"

# Run writer without compaction
zerofs run -c zerofs.toml --no-compactor

# Run standalone compactor
zerofs compactor -c zerofs.toml
```

**Benefits:**
- Isolate compaction resources
- Run compactor on cheaper hardware
- Scale compaction independently

---

## ZeroFS Implementation

### Chunk Store

ZeroFS stores files as **32KB chunks**:

```rust
// Simplified from chunk.rs
pub struct ChunkStore {
    db: Arc<Db>,
}

impl ChunkStore {
    pub async fn write(
        &self,
        txn: &mut Transaction,
        id: InodeId,
        offset: u64,
        data: &[u8],
    ) -> Result<(), FsError> {
        let start_chunk = offset / CHUNK_SIZE as u64;
        let end_chunk = (offset + data.len() as u64 - 1) / CHUNK_SIZE as u64;

        // Load existing chunks to modify
        let existing_chunks = self.load_chunks(id, start_chunk..=end_chunk).await?;

        // Modify chunks
        for (chunk_idx, mut chunk_data) in existing_chunks {
            // Copy new data into chunk
            let write_start = /* calculate offset */;
            let write_end = /* calculate offset */;
            chunk_data[write_start..write_end]
                .copy_from_slice(&data[/* source range */]);

            // Save or delete (if all zeros)
            if chunk_data.as_ref() == ZERO_CHUNK {
                self.delete(txn, id, chunk_idx);
            } else {
                self.save(txn, id, chunk_idx, chunk_data.freeze());
            }
        }

        Ok(())
    }
}
```

### Key Encoding

ZeroFS uses **structured keys** for efficient scanning:

```rust
// Simplified from key_codec.rs
pub struct KeyCodec;

impl KeyCodec {
    pub fn chunk_key(inode: InodeId, chunk_idx: u64) -> Bytes {
        // Key format: chunk:{inode}:{chunk_idx}
        let mut buf = BytesMut::new();
        buf.put_u8(KEY_PREFIX_CHUNK);
        buf.put_u64(inode);
        buf.put_u64(chunk_idx);
        buf.freeze()
    }

    pub fn inode_key(inode: InodeId) -> Bytes {
        // Key format: inode:{inode}
        let mut buf = BytesMut::new();
        buf.put_u8(KEY_PREFIX_INODE);
        buf.put_u64(inode);
        buf.freeze()
    }

    pub fn parse_chunk_key(key: &Bytes) -> Option<u64> {
        // Extract chunk index from key
        if key[0] != KEY_PREFIX_CHUNK {
            return None;
        }
        Some(key.read_u64_at(CHUNK_IDX_OFFSET))
    }
}
```

**Key Space Layout:**
```
Key Range                          Purpose
─────────────────────────────────────────────────
inode:0 .. inode:MAX             Inode metadata
chunk:0:0 .. chunk:MAX:MAX       File chunks
dir:{inode}:{name}               Directory entries
meta:*                           Metadata (next_inode, etc.)
```

### Transaction Support

ZeroFS batches writes using **transactions**:

```rust
pub struct Transaction {
    inner: WriteBatch,
}

impl Transaction {
    pub fn put_bytes(&mut self, key: &Bytes, value: Bytes) {
        self.inner.put(key, &value);
    }

    pub fn delete_bytes(&mut self, key: &Bytes) {
        self.inner.delete(key);
    }
}

// Usage
let mut txn = db.new_transaction()?;
chunk_store.write(&mut txn, inode, offset, data).await?;
inode_store.update(&mut txn, inode, new_size).await?;
db.write_with_options(txn.into_inner(), &options).await?;
```

### Encryption Integration

Encryption happens **at the SlateDB block level**:

```rust
// BlockTransformer interface
pub trait BlockTransformer: Send + Sync {
    fn transform(&self, block: &[u8]) -> Vec<u8>;
    fn inverse(&self, block: &[u8]) -> Option<Vec<u8>>;
}

// XChaCha20 encryption
struct EncryptionTransformer {
    key: [u8; 32],
}

impl BlockTransformer for EncryptionTransformer {
    fn transform(&self, block: &[u8]) -> Vec<u8> {
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.key));
        let nonce = generate_nonce();
        let ciphertext = cipher.encrypt(&nonce, block).unwrap();

        // Prepend nonce to ciphertext
        let mut result = nonce.to_vec();
        result.extend(ciphertext);
        result
    }

    fn inverse(&self, block: &[u8]) -> Option<Vec<u8>> {
        let nonce = XNonce::from_slice(&block[..24]);
        let ciphertext = &block[24..];

        let cipher = XChaCha20Poly1305::new(Key::from_slice(&self.key));
        cipher.decrypt(nonce, ciphertext).ok()
    }
}
```

---

## Performance Considerations

### Read Amplification

**Definition:** How many disk reads per logical read

```
Read Amplification Sources:
1. Memtable check (memory, fast)
2. Immutable memtable checks (memory, fast)
3. L0 SSTables (disk, may be multiple)
4. Lower level SSTables (disk, one per level)
5. Bloom filter checks (memory, reduces 3-4)
```

**Mitigation:**
- Bloom filters (eliminate unnecessary reads)
- Caching (keep hot data in memory)
- Leveled compaction (reduce files per level)

### Write Amplification

**Definition:** How many physical writes per logical write

```
Write Amplification Sources:
1. WAL write
2. Memtable (memory, not counted)
3. Flush to L0
4. L0 → L1 compaction
5. L1 → L2 compaction
6. ... and so on
```

**Typical Values:**
- Tiered compaction: 2-4x
- Leveled compaction: 10-20x

**Mitigation:**
- Larger memtables (fewer flushes)
- Size-tiered compaction (lower levels)
- Larger SSTables (better compression)

### Space Amplification

**Definition:** How much extra storage for obsolete data

```
Space Amplification Sources:
1. Old versions of keys
2. Tombstones (not yet compacted)
3. Duplicate keys across levels
```

**Mitigation:**
- Aggressive compaction
- Tombstone GC
- Single key per level (leveled compaction)

### Cache Strategy

ZeroFS uses **multi-layered caching**:

```
┌─────────────────────────────────────────┐
│          Cache Hierarchy                 │
├─────────────────────────────────────────┤
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  Memory Block Cache             │    │
│  │  - Hot data blocks              │    │
│  │  - Configurable size (GBs)      │    │
│  └─────────────────────────────────┘    │
│              │                          │
│  ┌─────────────────────────────────┐    │
│  │  Metadata Cache                 │    │
│  │  - Inodes                       │    │
│  │  - Directory entries            │    │
│  └─────────────────────────────────┘    │
│              │                          │
│  ┌─────────────────────────────────┐    │
│  │  Disk Cache (Foyer)             │    │
│  │  - Warm data                    │    │
│  │  - SSD-backed                   │    │
│  └─────────────────────────────────┘    │
│                                         │
└─────────────────────────────────────────┘
```

---

## Code Examples

### Building an LSM Tree (Simplified)

```rust
struct SimpleLSM {
    memtable: Memtable,
    immutable_memtables: Vec<Memtable>,
    sstables: Vec<SSTable>,
    wal: WAL,
}

impl SimpleLSM {
    pub async fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        // 1. Write to WAL
        self.wal.write(WALEntry::Put { key: key.clone(), value: value.clone() }).await?;

        // 2. Write to memtable
        self.memtable.put(key, value);

        // 3. Check if flush needed
        if self.memtable.should_flush(FLUSH_THRESHOLD) {
            self.rotate_memtable().await?;
        }

        Ok(())
    }

    pub async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // 1. Check memtable
        if let Some(value) = self.memtable.get(key) {
            return Ok(value.clone());
        }

        // 2. Check immutable memtables (newest first)
        for imem in self.immutable_memtables.iter().rev() {
            if let Some(value) = imem.get(key) {
                return Ok(value.clone());
            }
        }

        // 3. Check SSTables (newest first)
        for sst in self.sstables.iter().rev() {
            if let Some(value) = sst.get(key).await? {
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    async fn rotate_memtable(&mut self) -> Result<()> {
        // Swap memtable with immutable
        let new_memtable = Memtable::new();
        let old_memtable = std::mem::replace(&mut self.memtable, new_memtable);
        self.immutable_memtables.push(old_memtable);

        // Flush in background
        tokio::spawn(self.flush_immutable());

        Ok(())
    }

    async fn flush_immutable(&mut self) -> Result<()> {
        while let Some(imem) = self.immutable_memtables.first() {
            // Create SSTable
            let sst = SSTable::build(imem).await?;
            self.sstables.push(sst);
            self.immutable_memtables.remove(0);
        }
        Ok(())
    }
}
```

### Compaction Implementation

```rust
struct Compactor {
    lsm: Arc<SimpleLSM>,
    levels: Vec<Level>,
}

impl Compactor {
    pub async fn compact_level(&mut self, level: usize) -> Result<()> {
        // Select files for compaction
        let (input_files, output_level) = self.select_compaction_input(level)?;

        // Merge files
        let mut merger = SSTableMerger::new();
        for file in &input_files {
            merger.add_file(file.clone()).await?;
        }

        // Write output
        let output_files = merger.flush(output_level).await?;

        // Update manifest atomically
        self.update_manifest(&input_files, &output_files).await?;

        // Delete old files
        for file in input_files {
            std::fs::remove_file(&file.path)?;
        }

        Ok(())
    }

    fn select_compaction_input(&self, level: usize) -> Result<(Vec<SSTable>, usize)> {
        if level == 0 {
            // L0: compact all files
            Ok((self.levels[0].files.clone(), 1))
        } else {
            // Leveled: compact overlapping files
            let (selected, target_level) = self.find_overlapping(level)?;
            Ok((selected, target_level))
        }
    }
}
```

### Bloom Filter Implementation

```rust
struct BloomFilterBuilder {
    bits_per_key: u32,
    keys: Vec<Vec<u8>>,
}

impl BloomFilterBuilder {
    pub fn new(bits_per_key: u32) -> Self {
        Self {
            bits_per_key,
            keys: Vec::new(),
        }
    }

    pub fn add_key(&mut self, key: Vec<u8>) {
        self.keys.push(key);
    }

    pub fn build(&self) -> BloomFilter {
        let num_keys = self.keys.len();
        let filter_bits = num_keys * self.bits_per_key as usize;
        let filter_bytes = (filter_bits + 7) / 8;
        let num_hashes = self.optimal_num_hashes();

        let mut filter = vec![0u8; filter_bytes];

        for key in &self.keys {
            let hashes = self.get_hash_indices(key, num_hashes, filter_bits);
            for bit_pos in hashes {
                filter[bit_pos / 8] |= 1 << (bit_pos % 8);
            }
        }

        BloomFilter {
            data: filter,
            num_hashes,
            bits: filter_bits,
        }
    }

    fn optimal_num_hashes(&self) -> usize {
        // k = (m/n) * ln(2)
        ((self.bits_per_key as f64) * 0.693) as usize
    }

    fn get_hash_indices(&self, key: &[u8], num_hashes: usize, filter_bits: usize) -> Vec<usize> {
        // Use double hashing: h(i) = h1 + i * h2
        let h1 = hash1(key) % filter_bits;
        let h2 = hash2(key) % filter_bits;
        (0..num_hashes).map(|i| (h1 + i * h2) % filter_bits).collect()
    }
}
```

---

## Summary

### Key Takeaways

1. **LSM trees** convert random writes to sequential appends, ideal for cloud storage
2. **WAL** provides durability before data is flushed
3. **Memtables** buffer writes in memory for low latency
4. **SSTables** are immutable sorted files with:
   - Compressed data blocks
   - Index for binary search
   - Bloom filters for fast existence checks
5. **Compaction** merges SSTables to:
   - Remove obsolete data
   - Reduce file count
   - Improve read performance
6. **ZeroFS** uses SlateDB with:
   - 32KB chunk storage
   - XChaCha20 encryption
   - Multi-layered caching
   - Standalone compactor option

### Further Reading

- [The Log-Structured Merge-Tree (LSM-Tree) paper](https://www.seas.harvard.edu/sites/default/files/files/archived/Nepal.pdf)
- [SlateDB Documentation](https://github.com/slatedb/slatedb)
- [LevelDB Implementation](https://github.com/google/leveldb)
- [RocksDB Tuning Guide](https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide)
