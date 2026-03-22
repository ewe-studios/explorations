# iroh-sync Deep Dive

## Overview

`iroh-sync` implements CRDT-based (Conflict-Free Replicated Data Type) synchronization for multi-dimensional key-value documents. It provides eventual consistency guarantees through a range-based set reconciliation protocol.

**Version:** 0.4.1
**Repository:** https://github.com/n0-computer/iroh-sync
**License:** MIT/Apache-2.0

---

## Architecture and Design Decisions

### Core Design Philosophy

The synchronization protocol is based on **Range-Based Set Reconciliation**, as described in the paper by Aljoscha Meyer:

> "Range-based set reconciliation is a simple approach to efficiently compute the union of two sets over a network, based on recursively partitioning the sets and comparing fingerprints of the partitions to probabilistically detect whether a partition requires further work."

### Key Design Decisions

1. **Cryptographic Verification**: All entries are signed with two keypairs:
   - **Namespace Key**: Acts as a write capability token
   - **Author Key**: Provides proof of authorship

2. **Separation of Concerns**: The sync layer handles only metadata synchronization. Content data (blobs) is stored and transferred separately through `iroh-blobs`.

3. **Eventual Consistency**: The CRDT design ensures that all replicas converge to the same state regardless of update order.

4. **Multi-Version Support**: Multiple versions of the same key are retained, enabling conflict resolution and historical queries.

### Replica Model

A **Replica** is the fundamental unit of synchronization:
- Contains unlimited entries identified by (key, author, namespace)
- Entry values are 32-byte BLAKE3 hashes pointing to content in `iroh-blobs`
- Entries are signed for authenticity and integrity

### Store Architecture

The crate exposes a generic storage interface (`store::Store`) with implementations:

1. **File System Store**: Persistent storage using `redb`
2. **In-Memory Store**: For testing and ephemeral use

Both implementations use `redb` as the underlying storage engine.

---

## Key APIs and Data Structures

### Core Types

```rust
/// A replica containing entries
pub struct Replica {
    inner: Arc<RwLock<InnerReplica>>,
}

/// Entry identifier - unique per (key, namespace, author)
pub struct RecordIdentifier {
    key: Vec<u8>,
    namespace: NamespaceId,
    author: AuthorId,
}

/// A record containing content metadata
pub struct Record {
    timestamp: u64,     // Microseconds since Unix epoch
    len: u64,          // Content length in bytes
    hash: Hash,        // BLAKE3 hash of content
}

/// A signed entry
pub struct SignedEntry {
    signature: EntrySignature,
    entry: Entry,
}

/// Dual signatures for authorship and capability
pub struct EntrySignature {
    author_signature: Signature,
    namespace_signature: Signature,
}
```

### Namespace and Author Management

```rust
/// Namespace - represents a document scope
pub struct Namespace {
    priv_key: SigningKey,
    id: NamespaceId,
}

impl Namespace {
    pub fn new<R: CryptoRngCore>(rng: &mut R) -> Self { }
    pub fn id(&self) -> &NamespaceId { }
    pub fn sign(&self, msg: &[u8]) -> Signature { }
}

/// Author - represents a writer identity
pub struct Author {
    priv_key: SigningKey,
    id: AuthorId,
}

impl Author {
    pub fn new<R: CryptoRngCore>(rng: &mut R) -> Self { }
    pub fn id(&self) -> &AuthorId { }
    pub fn sign(&self, msg: &[u8]) -> Signature { }
}
```

### Replica Operations

```rust
impl Replica {
    /// Create a new replica
    pub fn new(namespace: Namespace) -> Self { }

    /// Insert a new entry
    pub fn insert(
        &self,
        key: impl AsRef<[u8]>,
        author: &Author,
        data: impl Into<Bytes>
    ) {
        // Signs and stores the entry
    }

    /// Get the latest entry for a key/author
    pub fn get_latest(
        &self,
        key: impl AsRef<[u8]>,
        author: &AuthorId
    ) -> Option<SignedEntry> { }

    /// Get all versions of an entry
    pub fn get_all<'a>(
        &'a self,
        key: impl AsRef<[u8]>,
        author: &AuthorId
    ) -> GetAllIter<'a> { }

    /// Get initial sync message
    pub fn sync_initial_message(&self) -> Message<RecordIdentifier, SignedEntry> { }

    /// Process incoming sync message
    pub fn sync_process_message(
        &self,
        message: Message<RecordIdentifier, SignedEntry>
    ) -> Option<Message<RecordIdentifier, SignedEntry>> { }
}
```

### Store Interface

```rust
pub trait Store<K, V> {
    fn get_first(&self) -> K;
    fn get(&self, key: &K) -> Option<&V>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn get_fingerprint(&self, range: &Range<K>, limit: Option<&Range<K>>) -> Fingerprint;
    fn put(&mut self, k: K, v: V);
    fn remove(&mut self, key: &K) -> Option<V>;
    fn all(&self) -> Self::AllIterator<'_>;
}
```

---

## Protocol Details

### Range-Based Set Reconciliation

The sync protocol works as follows:

1. **Initial Exchange**: Both peers exchange initial messages containing:
   - Range fingerprints (hash of all entries in range)
   - Range bounds

2. **Fingerprint Comparison**: If fingerprints match for a range, both peers have identical data for that range.

3. **Recursive Partitioning**: If fingerprints differ, the range is split and the process repeats.

4. **Entry Exchange**: At leaf ranges, actual entries are exchanged.

### Message Flow

```
Peer A                              Peer B
  |                                    |
  |--- Initial Message --------------->|
  |<--- Response Message --------------|
  |--- Refinement Message ----------->|
  |<--- Entry Exchange ----------------|
  |                                    |
  (Both replicas now synchronized)
```

### Message Types

```rust
pub enum MessagePart<K, V> {
    /// Fingerprint of a range
    RangeFingerprint(RangeFingerprint<K>),

    /// Actual entries in a range
    RangeItem(RangeItem<K, V>),
}

pub struct RangeFingerprint<K> {
    pub range: Range<K>,
    pub fingerprint: Fingerprint,
}

pub struct RangeItem<K, V> {
    pub range: Range<K>,
    pub values: Vec<(K, V)>,
    pub have_local: bool,  // Request local items if false
}
```

### Fingerprint Computation

Fingerprints use XOR of individual entry fingerprints:

```rust
impl Fingerprint {
    pub fn empty() -> Self {
        Fingerprint::new(&[][..])
    }
}

impl std::ops::BitXorAssign for Fingerprint {
    fn bitxor_assign(&mut self, rhs: Self) {
        for (a, b) in self.0.iter_mut().zip(rhs.0.iter()) {
            *a ^= b;
        }
    }
}
```

This design allows:
- Efficient computation of range fingerprints from sub-ranges
- Detection of differences through XOR comparison
- Probabilistic equality testing

### Entry Signing

Entries are signed by both the namespace and author:

```rust
impl EntrySignature {
    pub fn from_entry(
        entry: &Entry,
        namespace: &Namespace,
        author: &Author
    ) -> Self {
        let bytes = entry.to_vec();
        let namespace_signature = namespace.sign(&bytes);
        let author_signature = author.sign(&bytes);

        EntrySignature {
            author_signature,
            namespace_signature,
        }
    }
}
```

This dual-signature scheme provides:
- **Authentication**: Only authors with valid keys can create entries
- **Authorization**: Only holders of the namespace key can write to the replica
- **Non-repudiation**: Both parties are cryptographically bound to the entry

---

## Integration with Main Iroh Endpoint

### Sync Engine Integration

The sync module integrates with the main iroh endpoint through the docs engine:

```rust
pub struct Engine<D> {
    pub endpoint: Endpoint,
    pub sync: SyncHandle,
    pub default_author: DefaultAuthor,
    // ...
}

impl<D: iroh_blobs::store::Store> Engine<D> {
    pub async fn spawn(
        endpoint: Endpoint,
        gossip: Gossip,
        replica_store: crate::store::Store,
        bao_store: D,
        // ...
    ) -> anyhow::Result<Self> { }
}
```

### Gossip Protocol Integration

Sync messages are exchanged over the gossip protocol:

```rust
// Join sync swarm for a namespace
engine.start_sync(namespace_id, vec![peer_addr]).await?;

// Leave sync swarm
engine.leave(namespace_id, false).await?;

// Subscribe to sync events
let mut events = engine.subscribe().await?;
while let Some(event) = events.next().await {
    // Handle sync events
}
```

### Content Addressing

Entries reference content via BLAKE3 hashes:

```rust
// When inserting data
replica.insert("/key", &author, content_bytes);

// Content is stored separately in iroh-blobs
// Entry only stores the hash:
pub struct Record {
    hash: Hash,  // Points to blob content
    len: u64,    // Content size
    timestamp: u64,
}
```

---

## Production Usage Patterns

### Document Creation and Sync

```rust
use iroh_sync::{Namespace, Author, Replica};

// Create namespace and author
let mut rng = rand::thread_rng();
let namespace = Namespace::new(&mut rng);
let author = Author::new(&mut rng);

// Create replica
let replica = Replica::new(namespace.clone());

// Insert entries
replica.insert("/path/to/doc", &author, b"Hello, iroh!");
replica.insert("/path/to/other", &author, b"More data");

// Get entries
let entry = replica.get_latest("/path/to/doc", author.id()).unwrap();
assert!(entry.verify().is_ok());
```

### Peer Synchronization

```rust
// Alice creates and populates replica
let alice_set = ["ape", "eel", "fox", "gnu"];
for el in &alice_set {
    alice_replica.insert(el, &author, el.as_bytes());
}

// Bob creates his own entries
let bob_set = ["bee", "cat", "doe", "eel", "fox", "hog"];
for el in &bob_set {
    bob_replica.insert(el, &author, el.as_bytes());
}

// Sync replicas
let mut msg = Some(alice_replica.sync_initial_message());
while let Some(message) = msg.take() {
    if let Some(response) = bob_replica.sync_process_message(message) {
        msg = alice_replica.sync_process_message(response);
    }
}

// Both replicas now contain all entries
```

### Multi-Version Queries

```rust
// Get all versions of a key
let versions: Vec<SignedEntry> = replica
    .get_all("/key", author_id)
    .collect();

for entry in versions {
    println!("Timestamp: {}", entry.entry().record().timestamp());
    println!("Content hash: {}", entry.entry().record().content_hash());
}
```

### Storage Backend Selection

```rust
// Persistent file-based storage
let store = iroh_sync::store::fs::Store::persistent(
    "/path/to/storage".into()
)?;

// In-memory storage for testing
let store = iroh_sync::store::fs::Store::memory();
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| ed25519-dalek | 2.0.0 | Digital signatures |
| blake3 | 1.3.3 | Content hashing |
| parking_lot | 0.12.1 | Synchronization primitives |
| redb | 2.0.0 | Embedded database |
| bytes | 1.4.0 | Byte buffer management |
| rand | 0.8.5 | Cryptographic RNG |

### Notable Rust Patterns

1. **Interior Mutability**: `RwLock<InnerReplica>` allows concurrent read access with exclusive writes
2. **Reference Counting**: `Arc` enables thread-safe sharing of replica state
3. **Capability-based Security**: Namespace keys act as write capabilities
4. **Newtype Pattern**: `NamespaceId`, `AuthorId` provide type safety

### CRDT Considerations

The implementation follows CRDT principles:
- **Commutativity**: Entry order doesn't affect final state
- **Associativity**: Grouping of operations doesn't matter
- **Idempotency**: Duplicate entries are handled correctly
- **Causality**: Timestamps provide ordering for conflict resolution

### Memory Management

- Entries are cloned on access (trade-off for safety)
- `GetAllIter` uses mapped guards to avoid holding locks
- Range iterators are lazy and memory-efficient

### Potential Enhancements

1. **Compression**: Range fingerprints could use compressed representations
2. **Batching**: Bulk insert operations could reduce lock contention
3. **Async I/O**: Store operations could be fully asynchronous
4. **Indexing**: Secondary indexes for key pattern queries

---

## Summary

`iroh-sync` provides a robust CRDT-based synchronization layer with:

- **Strong Security**: Dual-signature scheme for authentication and authorization
- **Efficient Reconciliation**: Range-based fingerprinting minimizes data transfer
- **Flexible Storage**: Generic store interface with persistent and memory implementations
- **Content Addressing**: Clean separation between metadata sync and content storage

The module forms the foundation for iroh's distributed document synchronization capabilities.
