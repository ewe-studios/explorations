# Bao-Tree Deep Dive: Merkle Tree Implementation for Verified Streaming

## Overview

`bao-tree` is a Rust implementation of a Merkle tree data structure based on the BLAKE3 hash function, designed for verified streaming of content-addressed data. It is a reimagining of the original [bao crate](https://github.com/oconnor663/bao) with significant architectural improvements.

**Version:** 0.15.1
**Repository:** https://github.com/n0-computer/bao-tree
**License:** MIT OR Apache-2.0
**Author:** Rüdiger Klaehn <rklaehn@protonmail.com>

## Core Concepts

### What is BAO?

BAO (Blake3 Authenticated Objects) is a protocol for verified streaming using BLAKE3 Merkle trees. The key innovation is that it allows verification of data integrity while streaming, without needing to buffer the entire content.

### Key Differentiators from the Original BAO Crate

1. **Runtime Configurable Chunk Groups**: Unlike the original bao crate which uses fixed 1024-byte chunks, bao-tree supports runtime-configurable chunk group sizes (powers of 2)

2. **Multi-Range Queries**: Supports requesting multiple non-overlapping ranges in a single query (e.g., `[0..1000, 5000..6000]`)

3. **First-Class Async Support**: Clean separation between sync and async I/O with maximum code sharing

4. **Flexible Outboard Formats**: Supports pre-order, post-order, and custom outboard formats

## Architecture

### Core Data Structures

#### `BaoTree`

The central specification struct that defines tree geometry:

```rust
pub struct BaoTree {
    /// Total number of bytes in the file
    size: u64,
    /// Log base 2 of the chunk group size
    block_size: BlockSize,
}
```

Key methods:
- `new(size, block_size)` - Create a new tree specification
- `root()` - Compute the root node
- `blocks()` - Number of blocks in the tree
- `chunks()` - Number of chunks
- `outboard_size()` - Size of the outboard data

#### `TreeNode`

Identifies a node in the Merkle tree using a compact u64 representation:

```rust
pub struct TreeNode(u64);
```

The encoding uses trailing ones to represent the level:
- Leaf nodes (level 0) are even numbers
- Branch nodes have trailing 1 bits corresponding to their level

Key operations:
- `level()` - Get the node's level in the tree
- `left_child()` / `right_child()` - Navigate to children
- `parent()` - Navigate to parent
- `chunk_range()` - Get the byte range covered by this node
- `post_order_offset()` - Get position in post-order traversal

#### `BlockSize`

Represents the chunk group size as a power of 2:

```rust
pub struct BlockSize(u8);
```

- `0` = 1024 bytes (no chunk grouping)
- `1` = 2048 bytes
- `2` = 4098 bytes
- `4` = 16384 bytes (recommended default)

#### `ChunkNum`

Newtype wrapper for u64 representing BLAKE3 chunk numbers:

```rust
pub struct ChunkNum(pub u64);
```

Each chunk is 1024 bytes (BLAKE3_CHUNK_SIZE).

### Tree Traversal

The crate provides several iterator types for tree traversal:

#### Post-Order Traversal
Used for creating outboards from existing data:
```rust
pub fn post_order_chunks_iter(&self) -> PostOrderChunkIter
```

#### Pre-Order Traversal (Partial)
Used for encoding/decoding with range queries:
```rust
pub fn ranges_pre_order_chunks_iter_ref(
    &self,
    ranges: &RangeSetRef<ChunkNum>,
    min_level: u8,
) -> PreOrderPartialChunkIterRef
```

### I/O Module Structure

```
io/
├── sync/      - Synchronous I/O operations
├── fsm/       - Async (tokio) I/O using Finite State Machine pattern
├── mixed/     - Experimental mixed sync/async
└── outboard/  - Outboard storage implementations
```

#### Outboard Types

Outboards store the Merkle tree hashes separately from the data:

```rust
pub struct PreOrderOutboard<T> {
    pub tree: BaoTree,
    pub root: blake3::Hash,
    pub data: T,
}
```

## Hash Computation

### Subtree Hashing

```rust
fn hash_subtree(start_chunk: u64, data: &[u8], is_root: bool) -> blake3::Hash {
    use blake3::hazmat::{ChainingValue, HasherExt};
    if is_root {
        blake3::hash(data)
    } else {
        let mut hasher = blake3::Hasher::new();
        hasher.set_input_offset(start_chunk * 1024);
        hasher.update(data);
        let non_root_hash: ChainingValue = hasher.finalize_non_root();
        blake3::Hash::from(non_root_hash)
    }
}
```

### Parent Hash Combination

```rust
fn parent_cv(left_child: &blake3::Hash, right_child: &blake3::Hash, is_root: bool) -> blake3::Hash {
    use blake3::hazmat::{merge_subtrees_non_root, merge_subtrees_root, ChainingValue, Mode};
    let left_child: ChainingValue = *left_child.as_bytes();
    let right_child: ChainingValue = *right_child.as_bytes();
    if is_root {
        merge_subtrees_root(&left_child, &right_child, Mode::Hash)
    } else {
        blake3::Hash::from(merge_subtrees_non_root(
            &left_child,
            &right_child,
            Mode::Hash,
        ))
    }
}
```

## Usage Patterns

### Basic End-to-End Example

```rust
use bao_tree::{
    io::{
        outboard::PreOrderOutboard,
        round_up_to_chunks,
        sync::{decode_ranges, encode_ranges_validated, CreateOutboard},
    },
    BlockSize, ByteRanges,
};

const BLOCK_SIZE: BlockSize = BlockSize::from_chunk_log(4); // 16 KiB

// Create outboard from file
let file = std::fs::File::open("video.mp4")?;
let ob = PreOrderOutboard::<Vec<u8>>::create(&file, BLOCK_SIZE)?;

// Encode specific byte ranges
let ranges = ByteRanges::from(0..100000);
let ranges = round_up_to_chunks(&ranges);
let mut encoded = vec![];
encode_ranges_validated(&file, &ob, &ranges, &mut encoded)?;

// Decode on receiving side
let from_server = io::Cursor::new(encoded.as_slice());
let root = ob.root;
let tree = ob.tree;
let mut decoded = std::fs::File::create("copy.mp4")?;
let mut ob = PreOrderOutboard { tree, root, data: vec![] };
decode_ranges(from_server, &ranges, &mut decoded, &mut ob)?;
```

### Range Query Operations

The crate supports complex range queries:

1. **Round up to chunks**: Converts byte ranges to chunk ranges
2. **Round up to chunk groups**: Groups chunks for efficient verification
3. **Full chunk groups**: Gets only complete chunk groups within a range

## Performance Characteristics

### Outboard Size

For a file of size N bytes with block size B:
- Number of blocks: `ceil(N / (1024 * 2^B))`
- Outboard size: `(blocks - 1) * 64` bytes (64 bytes = two 32-byte BLAKE3 hashes)

### Verification Overhead

Due to BLAKE3's speed, hash verification during streaming is not a bottleneck compared to network operations and encryption.

## Applications in the n0-computer Ecosystem

### iroh-blobs

`bao-tree` is the foundation for `iroh-blobs`, which provides:
- Content-addressed blob storage
- Verified streaming over QUIC
- Resume-capable transfers

### sendme

The `sendme` CLI tool uses `bao-tree` for:
- File/directory transfer with verification
- NAT hole punching via iroh
- Progress tracking during transfers

### dumbpipe

While `dumbpipe` focuses on raw streaming, it can use `bao-tree` for:
- Verified data transfer over custom ALPN protocols
- Integration with iroh-blobs protocol

## Design Decisions

### Why TreeNode uses u64 with Trailing Ones

This encoding allows:
- O(1) level computation via `trailing_ones()`
- Efficient parent/child navigation using bit operations
- Compact representation without additional metadata

### Why Separate Sync and Async

The design allows:
- Maximum code sharing between sync and async
- No runtime overhead from async when using sync
- Clear API boundaries for each use case

### Why Runtime Configurable Chunk Groups

- Allows tuning for different use cases (small messages vs. large files)
- Enables efficient partial verification
- Better bandwidth utilization for range queries

## Testing Strategy

The crate uses:
- **Property-based testing** with proptest for tree invariants
- **Comparison tests** against the original bao crate for compatibility
- **Integration tests** for end-to-end encoding/decoding
- **Benchmarks** for performance regression detection

## Future Directions

Potential improvements:
1. Parallel hash computation for large files
2. Incremental outboard updates for append-only workloads
3. Compression integration
4. Zero-copy optimizations for network buffers

## Related Resources

- [BAO Specification](https://github.com/oconnor663/bao/blob/master/docs/spec.md)
- [BLAKE3 Paper](https://blake3.io/blake3.pdf)
- [IPFS Thing 2023 Presentation](https://www.youtube.com/watch?v=nk4nefmguZk)
- [bao-docs/](../@formulas/src.rust/src.n0-computer/bao-docs/) - Additional resources and presentations

## Conclusion

`bao-tree` represents a production-ready implementation of verified streaming that balances performance, flexibility, and correctness. Its design makes it suitable for building decentralized storage and content distribution systems where data integrity is paramount.
