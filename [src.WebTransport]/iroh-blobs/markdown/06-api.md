---
title: API — Store, Blobs, Tags, Downloader, Remote
---

# API — Store, Blobs, Tags, Downloader, Remote

The API layer provides the high-level interface for blob operations.

## Store

```rust
// iroh-blobs/src/api.rs
pub struct Store<S> {
    blobs: Blobs<S>,
    tags: Tags<S>,
    // ...
}
```

Source: `iroh-blobs/src/api.rs:1` — `Store` is the main entry point, wrapping Blobs and Tags APIs.

## Blobs API

```rust
// iroh-blobs/src/api/blobs.rs
pub struct Blobs<S> {
    store: S,
}

impl<S> Blobs<S> {
    /// Add bytes directly to the store.
    pub async fn add_bytes(&self, data: Bytes) -> Result<AddResult> { ... }

    /// Add a file or directory from the filesystem.
    pub async fn add_path(&self, path: PathBuf) -> AddProgress { ... }

    /// Add a stream of bytes.
    pub async fn add_stream(&self, stream: impl Stream<Item = Bytes>) -> AddProgress { ... }

    /// Export a blob to a file.
    pub async fn export_bao(&self, hash: Hash, path: PathBuf) -> ExportBaoProgress { ... }

    /// Export ranges of a blob.
    pub async fn export_ranges(&self, hash: Hash, ranges: ChunkRanges) -> ExportProgress { ... }

    /// Observe a blob (check existence without transfer).
    pub async fn observe(&self, hash: Hash) -> ObserveProgress { ... }

    /// Import a blob from bao-encoded data.
    pub async fn import_bao(&self, hash: Hash, reader: impl Read) -> ImportBaoHandle { ... }

    /// Get a reader for a blob.
    pub async fn reader(&self, hash: Hash) -> Result<BlobReader> { ... }
}
```

Source: `iroh-blobs/src/api/blobs.rs:1` — All blob operations with progress reporting.

## BlobReader

```rust
// iroh-blobs/src/api/blobs/reader.rs
pub struct BlobReader {
    state: ReaderState, // Idle, Reading, Seeking, Poisoned
    hash: Hash,
    store: Store,
}

impl AsyncRead for BlobReader { ... }
impl AsyncSeek for BlobReader { ... }
```

Source: `iroh-blobs/src/api/blobs/reader.rs:1` — `BlobReader` implements `AsyncRead + AsyncSeek` for streaming blob content.

## Tags API

```rust
// iroh-blobs/src/api/tags.rs
pub struct Tags<S> {
    store: S,
}

impl<S> Tags<S> {
    pub async fn list(&self) -> Result<Vec<TagInfo>> { ... }
    pub async fn get(&self, tag: &Tag) -> Result<Option<Hash>> { ... }
    pub async fn set(&self, tag: Tag, hash: Hash) -> Result<()> { ... }
    pub async fn delete(&self, tag: &Tag) -> Result<()> { ... }
    pub async fn rename(&self, old: Tag, new: Tag) -> Result<()> { ... }
    pub async fn create(&self, hash: Hash) -> Result<Tag> { ... }
    pub async fn temp_tag(&self, hash: Hash) -> TempTag { ... }
}
```

Source: `iroh-blobs/src/api/tags.rs:1` — Full CRUD for named tag references.

## Downloader

```rust
// iroh-blobs/src/api/downloader.rs
pub struct Downloader<S> {
    store: S,
    endpoint: Endpoint,
    discovery: Box<dyn ContentDiscovery>,
    pool: ConnectionPool,
}
```

Source: `iroh-blobs/src/api/downloader.rs:1` — `Downloader` manages multi-node downloads with connection pooling and content discovery.

### ContentDiscovery

```rust
// iroh-blobs/src/api/downloader.rs
pub trait ContentDiscovery {
    async fn find_peers(&self, hash: &Hash) -> Vec<NodeAddr>;
}
```

Source: `iroh-blobs/src/api/downloader.rs:1` — The `ContentDiscovery` trait finds peers that have a given blob.

### SplitStrategy

```rust
// iroh-blobs/src/api/downloader.rs
pub enum SplitStrategy {
    /// Download from a single node.
    Single,
    /// Split across multiple nodes.
    Shuffled,
}
```

Source: `iroh-blobs/src/api/downloader.rs:1` — Download strategy controls whether to use one or multiple nodes.

## Remote API

```rust
// iroh-blobs/src/api/remote.rs
pub struct Remote {
    conn: Connection,
}

impl Remote {
    pub async fn get(&self, hash: Hash, format: BlobFormat) -> GetProgress { ... }
    pub async fn push(&self, hash: Hash, format: BlobFormat) -> PushProgress { ... }
}
```

Source: `iroh-blobs/src/api/remote.rs:1` — `Remote` provides single-node download/upload without store integration.

## Error Types

```rust
// iroh-blobs/src/api.rs
pub enum Error {
    /// Store error.
    Store(StoreError),
    /// Protocol error.
    Protocol(ProtocolError),
    /// Network error.
    Network(NetworkError),
}

pub enum RequestError {
    /// Remote doesn't have the blob.
    NotFound,
    /// Protocol violation.
    BadRequest,
    /// Network error.
    Io,
}
```

Source: `iroh-blobs/src/api.rs:1` — Error types for API, request, and protocol failures.

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Module map
- [Get Client](../markdown/07-get-client.md) — Client-side transfer logic
- [Tags](../markdown/10-cross-cutting.md) — Tag management and temp tags
