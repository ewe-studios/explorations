# Willow Protocol Implementation: willow-rs and willow-store

## Overview

Willow is a family of protocols for synchronizable data stores with fine-grained permissions, efficient bandwidth usage, and support for destructive edits. The `willow-rs` project implements the Willow protocol specification in Rust.

**Repository:** https://github.com/n0-computer/willow-rs
**License:** MIT OR Apache-2.0
**Funding:** NLnet NGI0 Core Fund (Grant No. 101092990)

## Willow Protocol Background

### What is Willow?

Willow is designed as an alternative to CRDT-based synchronization with these key features:

1. **Fine-grained permissions**: Capability-based access control
2. **Destructive edits**: True deletion semantics (not just tombstones)
3. **Bandwidth efficient**: Only sync what's needed
4. **Privacy-aware**: Selective disclosure of data
5. **Memory efficient**: Bounded memory for sync operations

### Protocol Components

```
Willow Protocol Stack:
├── Data Model          - Core data structures
├── Meadowcap           - Capability system
├── Sideloading         - Offline data transfer
├── Sync Protocol       - Online synchronization
└── Store               - Data persistence (willow-store)
```

## willow-rs Architecture

### Workspace Structure

```
willow-rs/
├── data-model/         - Core Willow data structures
├── encoding/           - Binary encoding/decoding
├── meadowcap/          - Capability system implementation
├── earthstar/          - Earthstar integration
└── fuzz/               - Fuzz testing
```

### data-model

Implements the Willow Data Model specification:

#### Parameters

Willow is parameterized by:
- **M**: Maximum path length
- **N**: Maximum namespace length
- **S**: Maximum body size
- **K**: Signature scheme

```rust
pub struct WillowDataModel<const M: usize, const N: usize, const S: u64>;
```

#### Paths

Hierarchical paths similar to filesystem paths:

```rust
pub struct Path<const M: usize> {
    components: Vec<PathComponent>,
}

pub struct PathComponent {
    bytes: Vec<u8>,
}
```

Path properties:
- Ordered (lexicographic)
- Prefix-based queries
- Maximum length enforcement

#### Entries

The fundamental unit of data in Willow:

```rust
pub struct Entry<const M: usize, const N: usize, const S: u64> {
    namespace: NamespaceId<N>,
    path: Path<M>,
    entry_type: EntryType,
    timestamp: Timestamp,
    body_hash: Hash,
    // ...
}
```

Entry types:
- **Data**: Contains actual data
- **Deletion**: Marks data as deleted
- **Capability**: Permission delegation

#### Groupings

Organizing entries for efficient sync:

```rust
pub struct Grouping {
    entries: Vec<Entry>,
    proofs: Vec<CapabilityProof>,
}
```

### encoding

Binary serialization for Willow data:

#### Encoding Principles

1. **Deterministic**: Same input = same output
2. **Compact**: Minimal overhead
3. **Streamable**: Can encode/decode incrementally
4. **Self-delimiting**: Length prefixes where needed

#### Supported Encodings

```rust
// Entry encoding
pub fn encode_entry<M, N, S>(entry: &Entry<M, N, S>) -> Vec<u8>;
pub fn decode_entry<M, N, S>(bytes: &[u8]) -> Result<Entry<M, N, S>>;

// Path encoding
pub fn encode_path<M>(path: &Path<M>) -> Vec<u8>;
pub fn decode_path<M>(bytes: &[u8]) -> Result<Path<M>>;
```

### meadowcap

Capability-based access control system:

#### Capability Structure

```rust
pub struct Capability {
    /// What operations are allowed
    access: Access,
    /// Who can use this capability
    holder: PublicKey,
    /// Signature proving authorization
    signature: Signature,
}

pub enum Access {
    Read,
    Write,
    Admin,
}
```

#### Delegation

Capabilities can be delegated:

```rust
pub fn delegate(
    parent: &Capability,
    delegatee: &PublicKey,
    access: Access,
    signer: &dyn Signer,
) -> Result<Capability>;
```

#### Verification

```rust
pub fn verify_capability(
    capability: &Capability,
    root: &PublicKey,
) -> Result<Access>;
```

## willow-store: Storage Implementation

### Overview

`willow-store` provides the persistence layer for Willow data, implementing the missing Store component from the Willow specification.

**Repository:** https://github.com/n0-computer/willow-store
**Version:** 0.1.0

### Architecture

```
willow-store/
├── src/
│   ├── lib.rs          - Main module
│   ├── store.rs        - Core storage logic
│   ├── blob_seq.rs     - Sequential blob storage
│   ├── layout.rs       - Data layout
│   ├── geom.rs         - Geometry calculations
│   ├── fmt.rs          - Formatting utilities
│   └── mock_willow.rs  - Mock implementation for testing
└── examples/
    └── fs.rs           - Filesystem example
```

### Dependencies

```toml
[dependencies]
anyhow = "1.0.86"
blake3 = "1.5.1"
redb = "2.0.0"           # Embedded key-value store
zerocopy = "0.7.32"      # Zero-copy serialization
genawaiter = "0.99.1"    # Generator support
itertools = "0.13.0"
tracing = "0.1.40"
```

### Storage Design

#### Backend: redb

Uses [redb](https://github.com/cberner/redb) as the storage backend:
- ACID semantics
- Embedded (no server)
- B+ tree based
- Rust-native API

#### Data Organization

```rust
// Main tables in redb
const ENTRIES_TABLE: TableDefinition<_, _> = TableDefinition::new("entries");
const BLOBS_TABLE: TableDefinition<_, _> = TableDefinition::new("blobs");
const CAPABILITIES_TABLE: TableDefinition<_, _> = TableDefinition::new("capabilities");
```

#### Blob Storage

Sequential blob storage with reference counting:

```rust
pub struct BlobSeq {
    /// Sequence number for ordering
    seq: u64,
    /// Blob data
    data: Vec<u8>,
    /// Reference count
    refcount: u32,
}
```

### Store Operations

#### Insertion

```rust
pub async fn insert(
    &self,
    entry: Entry,
    body: impl AsRef<[u8]>,
) -> Result<InsertResult>;
```

#### Query

```rust
pub fn query(
    &self,
    namespace: &NamespaceId,
    path_prefix: &Path,
) -> impl Iterator<Item = Entry>;
```

#### Synchronization

```rust
pub fn sync_status(
    &self,
    peer: PeerId,
) -> SyncStatus;

pub fn get_missing(
    &self,
    peer_status: SyncStatus,
) -> Vec<EntryId>;
```

### Layout and Geometry

#### Layout Module

Handles physical data layout:

```rust
pub struct Layout {
    /// Entry index layout
    entries: Region,
    /// Blob data layout
    blobs: Region,
    /// Metadata
    meta: Region,
}
```

#### Geometry Module

Calculates storage geometry:

```rust
pub struct Geometry {
    /// Total capacity
    capacity: u64,
    /// Used space
    used: u64,
    /// Fragmentation ratio
    fragmentation: f32,
}
```

## Synchronization Protocol

### Sync State

```rust
pub struct SyncState {
    /// Our current state
    local: LocalState,
    /// Peer's known state
    remote: RemoteState,
    /// Pending operations
    pending: Vec<SyncOp>,
}
```

### Sync Operations

```rust
pub enum SyncOp {
    /// Send entry to peer
    SendEntry(EntryId),
    /// Request entry from peer
    RequestEntry(EntryId),
    /// Send blob data
    SendBlob(BlobId),
    /// Delete entry
    Delete(EntryId),
}
```

### Merkle Tree Integration

Uses bao-tree for efficient verification:

```rust
use bao_tree::{BaoTree, BlockSize};

pub fn compute_sync_tree(entries: &[Entry]) -> BaoTree {
    // Build Merkle tree of entries
}
```

## Integration with n0-computer Ecosystem

### iroh-sync

The iroh synchronization layer builds on willow-store:

```rust
use iroh_sync::{Store, Namespace};

// Willow-backed store
let store = WillowStore::open(path)?;
let namespace = store.get_namespace(&namespace_id)?;
```

### Capability Propagation

Meadowcap capabilities integrate with iroh's capability system:

```rust
use iroh::capabilities::Capability as IrohCapability;

impl From<MeadowcapCapability> for IrohCapability {
    fn from(cap: MeadowcapCapability) -> Self {
        // ...
    }
}
```

## Usage Example

### Basic Store Usage

```rust
use willow_store::{Store, Config};
use willow_data_model::{Entry, Path, NamespaceId};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open store
    let config = Config::default();
    let store = Store::open("/path/to/store", config)?;

    // Create namespace
    let namespace = store.create_namespace()?;

    // Insert data
    let path = Path::from_components(&["folder", "file.txt"])?;
    let entry = Entry::new(&namespace.id(), &path, EntryType::Data)?;
    store.insert(entry, b"Hello, Willow!").await?;

    // Query data
    let entries: Vec<_> = store
        .query(&namespace.id(), &Path::from_components(&["folder"])?)
        .collect();

    println!("Found {} entries", entries.len());

    Ok(())
}
```

### Capability Delegation

```rust
use willow_meadowcap::{Capability, Access, KeyPair};

// Create root capability
let root_keys = KeyPair::generate();
let root_cap = Capability::root(&root_keys.public());

// Delegate read access
let user_keys = KeyPair::generate();
let read_cap = root_cap.delegate(
    &user_keys.public(),
    Access::Read,
    &root_keys,
)?;

// Verify capability
let access = read_cap.verify(&root_keys.public())?;
assert_eq!(access, Access::Read);
```

## Performance Characteristics

### Storage Efficiency

- **Entry size**: ~200 bytes base + path length
- **Blob overhead**: Minimal (reference counted)
- **Index size**: Proportional to entry count

### Sync Efficiency

- **Delta sync**: Only transfer changed entries
- **Bloom filters**: Reduce false positives in sync queries
- **Batching**: Group operations for efficiency

### Memory Usage

- **Bounded sync**: Maximum memory bounded by configuration
- **Streaming**: Large blobs streamed, not buffered
- **Lazy loading**: Entries loaded on demand

## Testing

### Property-Based Testing

Uses proptest for invariant checking:

```rust
#[test]
fn test_entry_ordering() {
    prop_assert!(entries.are_ordered());
}
```

### Fuzz Testing

Dedicated fuzz target in `fuzz/`:

```rust
// fuzz/fuzz_targets/decode_entry.rs
fuzz_target!(|data: &[u8]| {
    let _ = decode_entry::<M, N, S>(data);
});
```

## Future Work

### Planned Features

1. **Full Sync Protocol**: Complete implementation of Willow sync
2. **Sideloading**: Offline data transfer protocol
3. **Enhanced Queries**: Complex query support
4. **Replication Strategies**: Configurable replication

### Optimizations

1. **Compression**: Optional entry compression
2. **Parallel Sync**: Concurrent sync operations
3. **Incremental Snapshots**: Point-in-time recovery
4. **Cache Layers**: Multi-level caching

## Comparison with Alternatives

| Feature | Willow | CRDTs | Git |
|---------|--------|-------|-----|
| Deletion | True | Tombstones | Manual |
| Permissions | Fine-grained | None | Coarse |
| Sync | Delta | Full state | Delta |
| Memory | Bounded | Unbounded | Bounded |

## Conclusion

Willow represents a novel approach to distributed data synchronization that addresses several limitations of existing approaches:

- **True deletion** without tombstone accumulation
- **Capability-based permissions** for fine-grained access control
- **Efficient synchronization** with bounded memory usage
- **Privacy-preserving** selective disclosure

The willow-rs and willow-store implementations provide a solid foundation for building local-first, distributed applications with strong consistency guarantees.

## Related Resources

- [Willow Protocol Specification](https://willowprotocol.org)
- [Willow Data Model Spec](https://willowprotocol.org/specs/data-model)
- [Meadowcap Spec](https://willowprotocol.org/specs/meadowcap)
- [willow-js](https://github.com/earthstar-project/willow-js) - TypeScript implementation
- [NLnet Grant](https://nlnet.nl/project/Willow/)
