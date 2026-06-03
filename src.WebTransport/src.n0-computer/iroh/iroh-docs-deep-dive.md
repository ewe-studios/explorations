# iroh-docs Deep Dive

## Overview

`iroh-docs` implements a distributed document database built on top of `iroh-sync` and `iroh-blobs`. It provides multi-dimensional key-value documents with efficient synchronization, authorship tracking, and content-addressed storage.

**Version:** 0.35.0
**Repository:** https://github.com/n0-computer/iroh-docs
**License:** MIT/Apache-2.0

---

## Architecture and Design Decisions

### Layered Architecture

The docs module is built in layers:

```
┌─────────────────────────────────────────┐
│           Engine (Live Sync)            │
├─────────────────────────────────────────┤
│           Protocol Handlers             │
├─────────────────────────────────────────┤
│              Net (ALPN)                 │
├─────────────────────────────────────────┤
│    iroh-sync (CRDT Sync Protocol)       │
├─────────────────────────────────────────┤
│    iroh-blobs (Content Storage)         │
├─────────────────────────────────────────┤
│         iroh-gossip (Broadcast)         │
└─────────────────────────────────────────┘
```

### Core Design Decisions

1. **Separation of Metadata and Content**: Following iroh-sync's design, docs store only metadata (keys, authors, timestamps, content hashes) while actual content lives in iroh-blobs.

2. **Actor-Based Sync Engine**: The `Engine` uses actors for managing:
   - Live sync coordination
   - Gossip swarm membership
   - Content download scheduling

3. **Default Author Pattern**: Each node maintains a persistent default author for local document operations.

4. **Content Protection**: Active documents protect their content from garbage collection through callback mechanisms.

5. **Store Abstraction**: Documents are persisted using a generic `Store` trait with redb-based implementations.

### Replica Model

A **Replica** (internally called a "document") is:
- Identified by a `NamespaceId`
- Contains entries keyed by (path, author)
- Entries point to content via BLAKE3 hashes
- Entries are signed for authenticity

### Sync Architecture

The sync system operates at multiple levels:

1. **Set Reconciliation**: iroh-sync's range-based protocol for metadata exchange
2. **Gossip Broadcast**: Real-time propagation of new entries
3. **Content Download**: Lazy fetching of blob content on demand

---

## Key APIs and Data Structures

### Core Types

```rust
/// Document namespace identifier
pub type NamespaceId = iroh_sync::NamespaceId;

/// Author identifier
pub type AuthorId = iroh_sync::AuthorId;

/// Document entry containing metadata
pub struct Entry {
    id: RecordIdentifier,
    record: Record,
}

/// Content status tracking
pub enum EntryStatus {
    Complete,    // Content fully available
    Partial,     // Some content available
    Missing,     // No content available
}

pub enum ContentStatus {
    Complete { hash: Hash },
    Partial { hash: Hash, available: ChunkRanges },
    Missing,
}
```

### The Sync Engine

```rust
/// Main entry point for document operations
pub struct Engine<D> {
    pub endpoint: Endpoint,
    pub sync: SyncHandle,
    pub default_author: DefaultAuthor,
    blob_store: D,
    // ...
}

impl<D: iroh_blobs::store::Store> Engine<D> {
    /// Start the sync engine
    pub async fn spawn(
        endpoint: Endpoint,
        gossip: Gossip,
        replica_store: crate::store::Store,
        bao_store: D,
        downloader: Downloader,
        default_author_storage: DefaultAuthorStorage,
        local_pool_handle: LocalPoolHandle,
    ) -> anyhow::Result<Self>;

    /// Start syncing a document
    pub async fn start_sync(
        &self,
        namespace: NamespaceId,
        peers: Vec<NodeAddr>
    ) -> Result<()>;

    /// Stop syncing a document
    pub async fn leave(
        &self,
        namespace: NamespaceId,
        kill_subscribers: bool
    ) -> Result<()>;

    /// Subscribe to document events
    pub async fn subscribe(
        &self,
        namespace: Option<NamespaceId>
    ) -> Result<impl Stream<Item = SyncEvent>>;

    /// Get content protection callback
    pub fn protect_cb(&self) -> ProtectCb;
}
```

### Document Operations

```rust
/// Handle for document operations
pub struct DocHandle {
    // ...
}

impl DocHandle {
    /// Insert content at a key
    pub async fn set_bytes(
        &self,
        key: impl AsRef<[u8]>,
        content: impl Into<Bytes>
    ) -> Result<()>;

    /// Get latest entry for a key
    pub async fn get_latest(
        &self,
        key: impl AsRef<[u8]>
    ) -> Result<Option<Entry>>;

    /// Get content for an entry
    pub async fn get_content(
        &self,
        key: impl AsRef<[u8]>
    ) -> Result<Option<Bytes>>;

    /// Get all entries matching a key pattern
    pub async fn get_many(
        &self,
        key_prefix: impl AsRef<[u8]>
    ) -> Result<Vec<Entry>>;

    /// Get content status
    pub async fn content_status(
        &self,
        key: impl AsRef<[u8]>
    ) -> Result<ContentStatus>;
}
```

### Author Management

```rust
/// Default author persistence
pub enum DefaultAuthor {
    Persistent(AuthorId),
    Ephemeral(Author),
}

impl DefaultAuthor {
    /// Load or create default author
    pub async fn load(
        storage: DefaultAuthorStorage,
        sync: &SyncHandle
    ) -> Result<Self>;

    /// Get author ID
    pub fn author_id(&self) -> AuthorId;
}

/// Storage for default author
pub enum DefaultAuthorStorage {
    Persistent(PathBuf),
    Ephemeral,
}
```

### Store Interface

```rust
/// Document store trait
pub trait Store: Clone + Send + Sync + 'static {
    /// Open a replica
    fn open_replica(&self, namespace: &NamespaceId) -> Result<Replica>;

    /// List all replicas
    fn list_replicas(&self) -> Result<Vec<(NamespaceId, AuthorId)>>;

    /// Import a replica from another store
    fn import_replica(&self, replica: Replica) -> Result<NamespaceId>;

    /// Remove a replica
    fn remove_replica(&self, namespace: &NamespaceId) -> Result<()>;
}
```

### Events

```rust
/// Sync and document events
pub enum SyncEvent {
    /// New entry received
    Insert {
        namespace: NamespaceId,
        author: AuthorId,
        key: Vec<u8>,
        entry: Entry,
    },

    /// Content status changed
    ContentReady {
        namespace: NamespaceId,
        hash: Hash,
    },

    /// Sync started with peer
    SyncStarted {
        namespace: NamespaceId,
        peer: NodeId,
    },

    /// Sync completed with peer
    SyncFinished {
        namespace: NamespaceId,
        peer: NodeId,
    },
}
```

---

## Protocol Details

### Network Protocol (ALPN: `/iroh-docs/1`)

The docs protocol handles document synchronization:

```rust
pub const ALPN: &[u8] = b"/iroh-docs/1";

/// Protocol messages
pub enum ProtocolMessage {
    /// Initial sync request
    SyncRequest {
        namespace: NamespaceId,
        from: Option<RecordIdentifier>,
    },

    /// Sync response with entries
    SyncResponse {
        entries: Vec<(RecordIdentifier, SignedEntry)>,
        done: bool,
    },

    /// Gossip forwarded message
    GossipForward {
        topic: TopicId,
        message: Bytes,
    },
}
```

### Sync Protocol Flow

```
Peer A                              Peer B
  |                                    |
  |--- SyncRequest (namespace) ------->|
  |                                    |
  |<-- SyncResponse (entries) ---------|
  |<-- SyncResponse (more entries) ----|
  |<-- SyncResponse (done=true) -------|
  |                                    |
  |--- GossipForward (update) -------->|
  |                                    |
  (Both replicas synchronized)
```

### Content Download Flow

```
1. Entry received via sync/gossip
         │
         v
2. Check local blob store
         │
    +----+----+
    |         |
   Yes       No
    |         |
    v         v
 Complete  Queue download
            │
            v
       Request from peer
            │
            v
       Validate & store
            │
            v
       Emit ContentReady
```

### Gossip Integration

Documents use gossip for real-time updates:

```rust
// When entry is inserted locally
async fn broadcast_update(
    namespace: NamespaceId,
    entry: &SignedEntry,
) {
    let message = encode_entry(entry);
    gossip.broadcast(namespace.into(), message).await.ok();
}

// When receiving gossip message
async fn handle_gossip(
    namespace: NamespaceId,
    data: Bytes,
) {
    if let Ok(entry) = decode_entry(&data) {
        if entry.verify().is_ok() {
            store.insert(namespace, entry).await.ok();
        }
    }
}
```

### Ticket-Based Sharing

Documents can be shared via tickets:

```rust
/// Document sharing ticket
pub struct DocTicket {
    namespace: NamespaceId,
    peers: Vec<NodeAddr>,
}

impl DocTicket {
    /// Create ticket for document
    pub fn new(namespace: NamespaceId, peers: Vec<NodeAddr>) -> Self;

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self>;

    /// Serialize to string
    pub fn to_string(&self) -> String;
}
```

---

## Integration with Main Iroh Endpoint

### Router Integration

```rust
// Create docs protocol handler
let docs_protocol = DocsProtocol::new(
    engine.clone(),
    gossip.clone(),
);

// Register with router
let router = Router::builder(endpoint)
    .accept(DocsProtocol::ALPN, docs_protocol.clone())
    .accept(GOSSIP_ALPN, gossip.clone())
    .build()
    .await?;
```

### Engine Initialization

```rust
// Full initialization sequence
let endpoint = Endpoint::builder()
    .relay_mode(RelayMode::Custom(relay_map))
    .bind()
    .await?;

let gossip = Gossip::builder().spawn(endpoint.clone());

let bao_store = iroh_blobs::store::fs::Store::persistent(
    blobs_path.clone()
)?;

let replica_store = crate::store::Store::fs(
    docs_path.clone()
)?;

let downloader = bao_store.downloader();

let engine = Engine::spawn(
    endpoint.clone(),
    gossip.clone(),
    replica_store,
    bao_store,
    downloader,
    DefaultAuthorStorage::Persistent(authors_path),
    LocalPoolHandle::default(),
).await?;
```

### Content Protection

```rust
// Protect document content from GC
let protect_cb = engine.protect_cb();

// Register with blob store
bao_store.set_protect_cb(protect_cb).await?;

// Content is now protected while document is active
```

---

## Production Usage Patterns

### Document Creation and Sync

```rust
use iroh_docs::{Engine, Namespace, Author};

// Create document
let mut rng = rand::thread_rng();
let namespace = Namespace::new(&mut rng);
let namespace_id = namespace.id();

// Open document handle
let doc = engine.open(namespace_id).await?;

// Insert content
doc.set_bytes("/hello.txt", b"Hello, iroh!".to_vec()).await?;
doc.set_bytes("/data.bin", large_data).await?;

// Start syncing with peers
let peers = discovery.get_peers().await?;
engine.start_sync(namespace_id, peers).await?;

// Monitor sync progress
let mut events = engine.subscribe(Some(namespace_id)).await?;
while let Some(event) = events.next().await {
    match event {
        SyncEvent::SyncFinished { namespace, peer } => {
            println!("Synced with {:?}", peer);
        }
        SyncEvent::ContentReady { hash, .. } => {
            println!("Content available: {}", hash);
        }
        _ => {}
    }
}
```

### Query Patterns

```rust
// Get single entry
let entry = doc.get_latest("/path/to/doc").await?;
if let Some(entry) = entry {
    let content = doc.get_content("/path/to/doc").await?;
}

// Get all entries under prefix
let entries: Vec<Entry> = doc
    .get_many("/prefix/")
    .await?;

// Get entries by author
let entries: Vec<Entry> = doc
    .get_by_author(author_id)
    .await?;

// Range queries
let entries: Vec<Entry> = doc
    .get_range("/start".."/end")
    .await?;
```

### Collaborative Editing

```rust
// Multiple authors can edit same document
let doc = engine.open(namespace_id).await?;

// Local edits
doc.set_bytes("/shared.txt", b"My edit".to_vec()).await?;

// Edits automatically sync to peers
// Conflicts resolved via CRDT semantics
// Last-write-wins based on timestamps

// Get all versions of a key
let versions = doc.get_all("/shared.txt").await?;
for entry in versions {
    println!("Version by {:?} at {}",
        entry.id().author(),
        entry.record().timestamp()
    );
}
```

### Document Discovery

```rust
// Advertise document availability
gossip.join(namespace_id.into(), vec![]).await?;

// Listen for document requests
let mut requests = gossip.subscribe(namespace_id.into()).await?;
while let Ok(event) = requests.recv().await {
    if let Event::Received(data) = event {
        // Handle document request
        let request: DocRequest = decode(&data)?;
        let entries = doc.get_many(request.prefix).await?;
        // Send entries...
    }
}
```

### Backup and Export

```rust
// Export document to CAR file
let car_path = "/backup.car";
let file = File::create(car_path).await?;
doc.export_car(file).await?;

// Import from CAR file
let file = File::open("/backup.car").await?;
let namespace_id = engine.import_car(file).await?;
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| iroh-sync | 0.4.1 | CRDT synchronization |
| iroh-blobs | 0.35.0 | Content storage |
| iroh-gossip | 0.35.0 | Gossip protocol |
| redb | 2.0.0 | Embedded database |
| ed25519-dalek | 2.0.0 | Signatures |
| tokio | 1.x | Async runtime |
| tokio-util | 0.7.12 | Async utilities |
| futures-lite | 2.3.0 | Future utilities |
| postcard | 1.x | Serialization |

### Notable Rust Patterns

1. **Actor Model**: `LiveActor` manages sync state with message passing
2. **Callback Pattern**: `ContentStatusCallback` for GC protection
3. **Stream-based Events**: Async streams for event subscription
4. **Type-Safe Storage**: Generic store trait with concrete implementations

### Concurrency Model

- Multi-actor architecture:
  - `LiveActor`: Coordinates sync operations
  - `GossipActor`: Manages gossip swarms
  - `SyncHandle`: Thread-safe handle to replica store
- Channels for inter-actor communication
- `Arc` for shared state access
- `RwLock` for read-heavy data structures

### Memory Management

- Lazy content loading - entries loaded on demand
- Content streaming for large blobs
- Reference counting for shared resources
- Explicit content protection via callbacks

### Error Handling

```rust
/// Document errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Store error: {0}")]
    Store(#[from] StoreError),

    #[error("Sync error: {0}")]
    Sync(#[from] SyncError),

    #[error("Content not available: {0}")]
    ContentMissing(Hash),

    #[error("Invalid signature")]
    InvalidSignature,
}
```

### Potential Enhancements

1. **Indexing**: Secondary indexes for complex queries
2. **Compression**: Optional entry compression
3. **Batch Operations**: Atomic multi-key operations
4. **Query Language**: Structured query interface
5. **Encryption**: Per-document encryption support

---

## Store Implementation Details

### File System Store

```rust
/// Persistent file-based document store
pub mod fs {
    /// Main store struct
    pub struct Store {
        db: redb::Database,
        path: PathBuf,
    }

    impl Store {
        /// Create persistent store
        pub fn persistent(path: PathBuf) -> Result<Self>;

        /// Create in-memory store
        pub fn memory() -> Result<Self>;
    }
}
```

### Database Schema

The store uses multiple redb tables:

```rust
/// Table definitions
mod tables {
    /// Entries by (namespace, author, key)
    pub const ENTRIES: TableDefinition<(&[u8], &[u8], &[u8]), &[u8]>;

    /// Latest entries by (namespace, key)
    pub const LATEST: TableDefinition<(&[u8], &[u8]), (&[u8], u64)>;

    /// Authors
    pub const AUTHORS: TableDefinition<&[u8], &[u8]>;
}
```

### Migrations

The store supports schema migrations:

```rust
mod migrations {
    /// Migrate from v1 to v2 schema
    pub fn migrate_v1_v2(db: &redb::Database) -> Result<()>;
}
```

---

## Summary

`iroh-docs` provides a distributed document database with:

- **CRDT-Based Sync**: Eventual consistency with conflict resolution
- **Content Addressing**: Efficient deduplication via iroh-blobs
- **Real-Time Updates**: Gossip-based propagation of changes
- **Authorship Tracking**: Cryptographic proof of authorship
- **Flexible Storage**: Pluggable storage backends

The module enables collaborative, offline-first applications with strong consistency guarantees.
