---
title: Replica — The Data Model: SignedEntry, Record, RecordIdentifier
---

# Replica — The Data Model: SignedEntry, Record, RecordIdentifier

The Replica is the local representation of a synchronizable key-value store.

## Entry Identity

Each entry is uniquely identified by a `RecordIdentifier`:

```rust
// iroh-docs/src/sync.rs
pub struct RecordIdentifier {
    namespace: NamespaceId,
    author: AuthorId,
    key: Bytes,
}
```

The identifier is a composite key: `(NamespaceId, AuthorId, key_bytes)`. This triple enables efficient prefix queries (all entries in a namespace, all entries by an author, all entries with a key prefix).

Source: `iroh-docs/src/sync.rs:1`.

## Entry Structure

```rust
// iroh-docs/src/sync.rs
pub struct SignedEntry {
    /// The record identifier (namespace, author, key).
    id: RecordIdentifier,
    /// The record data (hash, length, timestamp).
    record: Record,
    /// Dual signatures (author + namespace).
    signature: EntrySignature,
}

pub struct Record {
    /// BLAKE3 hash of the content (32 bytes).
    content_hash: Hash,
    /// Content length in bytes.
    content_len: u64,
    /// Lamport timestamp for ordering.
    timestamp: u64,
}
```

Source: `iroh-docs/src/sync.rs:1` — `SignedEntry`, `Record`.

**Key insight:** The `timestamp` is a Lamport clock, not a wall-clock time. Each new entry increments the timestamp, enabling conflict resolution: higher timestamps win. This is essential for multi-writer scenarios where two authors may write to the same key concurrently.

## Entry Validation

```rust
// iroh-docs/src/sync.rs
pub fn validate_entry(entry: &SignedEntry) -> Result<()> {
    // 1. Verify namespace signature
    // 2. Verify author signature
    // 3. Check timestamp is within bounds (not older than latest entry for this author)
    // 4. Validate content hash format
}
```

Source: `iroh-docs/src/sync.rs:1` — `validate_entry()` performs all checks before accepting an entry.

## Capability (Write/Read Access)

```rust
// iroh-docs/src/sync.rs
pub enum CapabilityKind {
    /// Full write access (has namespace secret key).
    Write,
    /// Read-only access (has namespace public key only).
    Read,
}
```

Source: `iroh-docs/src/sync.rs:1` — `CapabilityKind` determines write vs. read access.

## ContentStatus

```rust
// iroh-docs/src/sync.rs
pub enum ContentStatus {
    /// Content is fully available in the local blob store.
    Complete,
    /// Content is partially available (some chunks downloaded).
    Incomplete,
    /// Content is not available (not yet downloaded).
    Missing,
}
```

Source: `iroh-docs/src/sync.rs:1` — `ContentStatus` tracks blob availability.

## Replica Operations

```rust
// iroh-docs/src/sync.rs
impl Replica {
    /// Insert a new entry (creates a SignedEntry).
    pub fn insert(&self, author: &Author, key: Bytes, value: Bytes) -> Result<SignedEntry> { ... }

    /// Get latest entries, optionally filtered by author/key.
    pub fn get_latest_for_each_author(&self) -> impl Iterator<Item = SignedEntry> { ... }

    /// Get entries matching a prefix.
    pub fn get_by_prefix(&self, prefix: &[u8]) -> impl Iterator<Item = SignedEntry> { ... }

    /// Remove entries matching a prefix.
    pub fn remove_prefix(&self, prefix: &[u8]) -> Result<()> { ... }

    /// Get the fingerprint for range-based sync.
    pub fn get_fingerprint(&self, range: &Range<RecordIdentifier>) -> Fingerprint { ... }
}
```

Source: `iroh-docs/src/sync.rs:1` — Replica implements `ranger::Store<SignedEntry>`.

## AuthorHeads

```rust
// iroh-docs/src/heads.rs
pub struct AuthorHeads {
    /// Latest timestamp per author.
    heads: HashMap<AuthorId, u64>,
}
```

`AuthorHeads` tracks the latest entry timestamp per author. Used during sync to quickly determine which authors have new entries:

```rust
// Find authors where this node has newer entries than the peer
let updates = local_heads.has_news_for(&remote_heads);
```

Source: `iroh-docs/src/heads.rs:1`.

## Events

```rust
// iroh-docs/src/sync.rs
pub enum Event {
    /// A local insert (created by this node).
    LocalInsert { entry: SignedEntry },
    /// A remote insert (received from a peer).
    RemoteInsert { entry: SignedEntry },
}
```

Source: `iroh-docs/src/sync.rs:1` — Events notify the application of entry changes.

## Related Documents

- [Ranger](../markdown/03-ranger.md) — Set reconciliation algorithm
- [Storage](../markdown/06-storage.md) — redb persistence
- [Keys](../markdown/08-keys.md) — Author and Namespace keys
