---
title: Storage — redb v2 Database, Tables, Queries, Migrations
---

# Storage — redb v2 Database, Tables, Queries, Migrations

The store module provides a persistent storage backend using redb v2, with range queries, download policies, and database migrations.

## Store Construction

```rust
// iroh-docs/src/store/fs.rs
pub struct Store {
    db: Database,
    // Transaction management
}

impl Store {
    /// In-memory store (for testing).
    pub fn memory() -> Result<Self> { ... }

    /// Persistent store at the given path.
    pub fn persistent(path: impl AsRef<Path>) -> Result<Self> { ... }
}
```

Source: `iroh-docs/src/store/fs.rs:1`.

## redb Tables

| Table | Key | Value | Purpose |
|-------|-----|-------|---------|
| `AUTHORS_TABLE` | `[u8;32]` (AuthorId) | `[u8;32]` (secret key) | Author key storage |
| `NAMESPACES_TABLE` | `[u8;32]` (NamespaceId) | `(u8, [u8;32])` (capability) | Namespace capabilities |
| `RECORDS_TABLE` | `(ns, author, key)` | `(timestamp, ns_sig, author_sig, len, hash)` | Entry records |
| `LATEST_PER_AUTHOR_TABLE` | `(ns, author)` | `(timestamp, key)` | Latest entry per author index |
| `RECORDS_BY_KEY_TABLE` | `(ns, key, author)` | `()` | By-key index for prefix queries |
| `NAMESPACE_PEERS_TABLE` | `ns` → `(nanos, peer_id)` | multimap | Peer tracking per namespace |
| `DOWNLOAD_POLICY_TABLE` | `ns` | postcard bytes | Per-namespace download policies |

Source: `iroh-docs/src/store/fs/tables.rs:1`.

## StoreInstance: ranger::Store Implementation

```rust
// iroh-docs/src/store/fs.rs
struct StoreInstance<'a> {
    namespace: NamespaceId,
    tx: WriteTransaction,
}

impl ranger::Store<SignedEntry> for StoreInstance<'_> {
    fn get_first(&self, range: &Range<RecordIdentifier>) -> Result<Option<SignedEntry>> { ... }
    fn get(&self, key: &RecordIdentifier) -> Result<Option<SignedEntry>> { ... }
    fn get_range(&self, range: &Range<RecordIdentifier>) -> Result<impl Iterator<Item = SignedEntry>> { ... }
    fn entry_put(&self, entry: SignedEntry) -> Result<()> { ... }
    fn entry_remove(&self, key: &RecordIdentifier) -> Result<()> { ... }
    fn get_fingerprint(&self, range: &Range<RecordIdentifier>) -> Result<Fingerprint> { ... }
    fn prefixes_of(&self, key: &RecordIdentifier) -> Result<Vec<RecordIdentifier>> { ... }
    fn prefixed_by(&self, prefix: &RecordIdentifier) -> Result<impl Iterator<Item = SignedEntry>> { ... }
    fn remove_prefix_filtered(&self, prefix: &RecordIdentifier, filter: impl Fn(&SignedEntry) -> bool) -> Result<()> { ... }
}
```

Source: `iroh-docs/src/store/fs.rs:1` — `StoreInstance` implements all ranger store operations using redb range queries.

## Query Builder

```rust
// iroh-docs/src/store.rs
pub struct QueryBuilder<Q> {
    author_filter: AuthorFilter,
    key_filter: KeyFilter,
    limit: Option<usize>,
    offset: Option<usize>,
    sort_by: SortBy,
    sort_direction: SortDirection,
}

// Filter types
pub enum AuthorFilter { Any, Exact(AuthorId), Prefix(Bytes) }
pub enum KeyFilter { Any, Exact(Bytes), Prefix(Bytes) }
```

Source: `iroh-docs/src/store.rs:1`.

## Download Policies

```rust
// iroh-docs/src/store.rs
pub enum DownloadPolicy {
    /// Download nothing by default.
    Nothing,
    /// Download everything by default.
    Everything,
    /// Download except specified patterns.
    EverythingExcept(Vec<FilterKind>),
    /// Download only specified patterns.
    NothingExcept(Vec<FilterKind>),
}

pub enum FilterKind {
    Exact(Bytes),
    Prefix(Bytes),
}
```

Source: `iroh-docs/src/store.rs:1` — Download policies control which content blobs are fetched during sync.

## Peer Tracking

```rust
// iroh-docs/src/store/fs.rs
pub fn register_useful_peer(&self, namespace: NamespaceId, peer: NodeId) -> Result<()> { ... }
pub fn get_sync_peers(&self, namespace: NamespaceId) -> Result<Vec<NodeId>> { ... }
```

An LRU cache of size 5 per namespace tracks the most useful peers for sync.

Source: `iroh-docs/src/store/fs.rs:1`.

## Database Migrations

| Migration | Purpose |
|-----------|---------|
| `migration_001` | Populate `latest_per_author` table from records |
| `migration_002` | Copy namespaces v1 (secret-only) to v2 (capability format) |
| `migration_003` | Delete namespaces v1 table after migration |
| `migration_004` | Populate `records_by_key` index table |

Source: `iroh-docs/src/store/fs/migrations.rs:1`.

## redb v1 → v2 Migration

```rust
// iroh-docs/src/store/fs/migrate_v1_v2.rs
pub fn migrate_v1_to_v2(v1_path: &Path, v2_path: &Path) -> Result<()> {
    // Open v1 database
    // Create v2 database in tempfile
    // Migrate all tables
    // Atomic rename
}
```

Source: `iroh-docs/src/store/fs/migrate_v1_v2.rs:1`.

## Range Iterators

```rust
// iroh-docs/src/store/fs/ranges.rs
pub struct RecordsRange {
    inner: redb::Range<'static, ...>,
}

impl Iterator for RecordsRange {
    type Item = Result<SignedEntry>;
    fn next(&mut self) -> Option<Self::Item> { ... }
}
```

Source: `iroh-docs/src/store/fs/ranges.rs:1`.

## Related Documents

- [Replica](../markdown/02-replica.md) — Data model persisted to storage
- [Sync Actor](../markdown/04-sync-actor.md) — Actor uses Store
