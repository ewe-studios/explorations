---
title: Memory Store — MemStore and ReadonlyMemStore
---

# Memory Store — MemStore and ReadonlyMemStore

The in-memory stores provide ephemeral blob storage for testing and short-lived applications.

## MemStore

```rust
// iroh-blobs/src/store/mem.rs
pub struct MemStore {
    actor: Actor,
}
```

Source: `iroh-blobs/src/store/mem.rs:1` — `MemStore` wraps an actor that manages blob storage in memory.

### BaoFileStorage in Memory

```rust
// iroh-blobs/src/store/mem.rs
pub enum BaoFileStorage {
    /// Partial storage: incomplete blob with some chunks.
    Partial(BaoFilePartialStorage),
    /// Complete storage: full blob with all chunks verified.
    Complete(BaoFileCompleteStorage),
}

pub struct BaoFileCompleteStorage {
    pub outboard: Bytes,
    pub data: Bytes,
    pub size: u64,
}

pub struct BaoFilePartialStorage {
    pub outboard: Bytes,
    pub data: SparseMemFile,
    pub size_info: SizeInfo,
    pub bitfield: Bitfield,
}
```

Source: `iroh-blobs/src/store/mem.rs:1` — Complete storage holds full data and outboard in `Bytes`. Partial storage uses `SparseMemFile` for sparse chunk storage.

### SparseMemFile

```rust
// iroh-blobs/src/store/util/sparse_mem_file.rs
pub struct SparseMemFile {
    /// Chunks stored so far.
    chunks: BTreeMap<u64, Bytes>,
    /// Valid ranges tracking.
    valid_ranges: RangeSet,
}
```

Source: `iroh-blobs/src/store/util/sparse_mem_file.rs:1` — `SparseMemFile` stores chunks in a BTreeMap indexed by chunk number, tracking valid ranges.

## ReadonlyMemStore

```rust
// iroh-blobs/src/store/readonly_mem.rs
pub struct ReadonlyMemStore {
    blobs: HashMap<Hash, BaoFileCompleteStorage>,
}
```

Source: `iroh-blobs/src/store/readonly_mem.rs:1` — `ReadonlyMemStore` is an immutable store created from an iterator of blobs. It supports reads but not writes.

### Creation

```rust
// iroh-blobs/src/store/readonly_mem.rs
impl ReadonlyMemStore {
    pub fn new(blobs: impl IntoIterator<Item = (Hash, BaoFileCompleteStorage)>) -> Self { ... }
}
```

Useful for pre-loading test data or serving static content.

## Bitfield

```rust
// iroh-blobs/src/api/proto/bitfield.rs
pub struct Bitfield {
    /// Tracks which chunks have been validated.
    chunks: RangeSet,
}
```

Source: `iroh-blobs/src/api/proto/bitfield.rs:1` — `Bitfield` tracks which chunks of a partial blob have been received and verified.

## PartialMemStorage

```rust
// iroh-blobs/src/store/util/partial_mem_storage.rs
pub struct PartialMemStorage {
    pub data: Bytes,
    pub outboard: Bytes,
    pub size: u64,
    pub bitfield: Bitfield,
}
```

Source: `iroh-blobs/src/store/util/partial_mem_storage.rs:1` — Simplified partial storage for small blobs that fit in memory.

## When to Use MemStore

| Scenario | Store |
|----------|-------|
| Unit tests | MemStore |
| Short-lived CLI tool | MemStore |
| Pre-loaded static content | ReadonlyMemStore |
| Production server | FsStore |

## Related Documents

- [File Store](../markdown/04-store-fs.md) — Production file-based store
- [Architecture](../markdown/01-architecture.md) — Module map
