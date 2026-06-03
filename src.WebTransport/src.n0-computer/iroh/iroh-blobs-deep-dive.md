# iroh-blobs Deep Dive

## Overview

`iroh-blobs` is the foundational blob storage and synchronization component of the iroh ecosystem. It implements a content-addressed blob storage system using BLAKE3 verified streaming, enabling efficient, verifiable transfer of arbitrary-sized data over QUIC connections.

**Version:** 0.91.0
**Repository:** https://github.com/n0-computer/iroh-blobs
**License:** MIT OR Apache-2.0

---

## Architecture and Design Decisions

### Core Design Goals

1. **Data Integrity First**: The protocol is designed to be paranoid about data integrity. All data is validated using BLAKE3 hashes both on the provider and getter side. Data integrity is considered more important than performance.

2. **Unbounded Scale**: The system supports blobs of arbitrary size (up to terabytes) and collections with unlimited numbers of links. No component requires the entire blob or collection to be in memory at once.

3. **Efficient Range Requests**: The protocol supports efficient range requests with a worst-case overhead of about two chunk groups per range. The minimum granularity is a chunk group of 16KiB or 16 BLAKE3 chunks.

4. **Multi-Blob Efficiency**: For transferring multiple tiny blobs, the protocol supports grouping them into collections to avoid the overhead of separate round-trips for each blob.

### Content-Addressed Storage Model

The blob store uses BLAKE3 hashes as content identifiers. This provides:
- **Deduplication**: Identical content always produces the same hash
- **Verification**: Data can be verified against its hash during transfer
- **Merkle Trees**: Large files are structured as Merkle trees for efficient partial verification

### Bao-Tree Integration

The system uses the `bao-tree` crate for BLAKE3 verified streaming. Bao-tree extends standard BLAKE3 by:
- Combining multiple BLAKE3 chunks into chunk groups (16KiB) for efficiency
- Building Merkle trees that enable partial verification
- Supporting efficient range requests without downloading entire files

### Store Architecture

The blob store follows an actor-based architecture with two main actors:

1. **Main Actor**: Handles user commands and owns handles for hashes currently being worked on. It forwards commands to the database actor or creates hash contexts and spawns tasks.

2. **Database Actor**: Stores metadata about each hash, including:
   - Inlined data for small files
   - Outboard data (Merkle tree proofs)
   - Tags for blob identification

### Storage Backends

The crate provides two storage implementations:

1. **File System Store (`store::fs`)**: Persistent storage backed by:
   - A directory-based file system for blob data
   - A `redb` database for metadata
   - Support for partial blobs and progressive validation

2. **Memory Store (`store::mem`)**: Ephemeral in-memory storage for testing and temporary use cases.

---

## Key APIs and Data Structures

### Core Types

```rust
/// The main hash type - a 32-byte BLAKE3 hash
pub struct Hash([u8; 32]);

/// Combines a hash with its format (raw blob or hash sequence)
pub struct HashAndFormat {
    pub hash: Hash,
    pub format: BlobFormat,
}

/// Specifies whether content is a single blob or a sequence
pub enum BlobFormat {
    Raw,      // Single blob
    HashSeq,  // Sequence of blobs
}
```

### The Store API

```rust
/// Main entry point for blob operations
pub struct Store {
    client: ApiClient,
}

impl Store {
    /// Blob operations (import, export, delete, list)
    pub fn blobs(&self) -> &blobs::Blobs { }

    /// Tag management
    pub fn tags(&self) -> &Tags { }

    /// Remote fetching from single nodes
    pub fn remote(&self) -> &remote::Remote { }

    /// Complex multi-source downloads
    pub fn downloader(&self, endpoint: &Endpoint) -> downloader::Downloader { }
}
```

### GetRequest - Range Specification

The `GetRequest` type is central to the protocol, allowing precise specification of what data to retrieve:

```rust
pub struct GetRequest {
    pub hash: Hash,
    pub ranges: ChunkRangesSeq,
}

impl GetRequest {
    /// Request entire blob
    pub fn blob(hash: impl Into<Hash>) -> Self { }

    /// Request hash sequence and all children
    pub fn all(hash: impl Into<Hash>) -> Self { }

    /// Request specific byte ranges
    pub fn builder() -> GetRequestBuilder { }
}

// Example: Request specific byte ranges
let request = GetRequest::builder()
    .root(ChunkRanges::bytes(..1000) | ChunkRanges::bytes(10000..11000))
    .build(hash);
```

### ChunkRangesSeq

`ChunkRangesSeq` provides efficient encoding of range requests:
- Uses run-length encoding to remove repeating elements
- Supports infinite sequences for unknown-size collections
- Encodes alternating intervals of selected/non-selected chunks
- Compact wire representation using Postcard serialization

### TempTag System

Temporary tags provide automatic lifecycle management for blobs:

```rust
/// Temporary tag that automatically cleans up on drop
pub struct TempTag {
    // ...
}

impl Drop for TempTag {
    fn drop(&mut self) {
        // Automatically delete tagged blob
    }
}
```

---

## Protocol Details

### Wire Protocol (ALPN: `/iroh-bytes/4`)

The protocol operates over QUIC streams with the following message types:

```rust
pub enum Request {
    Get(GetRequest),           // Fetch blob/collection
    GetMany(GetManyRequest),   // Fetch multiple blobs
    Observe(ObserveRequest),   // Monitor blob availability
    Push(PushRequest),         // Push data to provider
}
```

### Request/Response Flow

1. **Connection Setup**: Getter connects to provider via QUIC with ALPN `/iroh-bytes/4`
2. **Request Send**: Getter sends serialized `GetRequest` (max 100MiB)
3. **Data Transfer**: Provider responds with BAO-encoded bytes
4. **Verification**: Getter validates data against Merkle tree
5. **Stream Close**: Provider closes stream (or connection on error)

### Error Handling

Connection and stream errors use standardized error codes:

```rust
pub enum Closed {
    StreamDropped = 0,         // Stream was dropped
    ProviderTerminating = 1,   // Provider shutting down
    RequestReceived = 2,       // Request complete, more data invalid
}
```

### Hash Sequences

For collections, blobs are organized as hash sequences:
- First element in `ChunkRangesSeq` refers to the hash sequence itself
- Subsequent elements refer to child blobs by index
- Infinite sequences supported via `build_open()` for unknown child counts

---

## Integration with Main Iroh Endpoint

### Protocol Handler

```rust
pub trait BlobsProtocol {
    /// Accept incoming blob requests
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError>;

    /// Graceful shutdown
    async fn shutdown(&self);
}
```

### Router Integration

```rust
// Register blobs protocol with iroh router
let router = Router::builder(endpoint)
    .accept(BlobsProtocol::ALPN, blobs_protocol.clone())
    .build()
    .await?;
```

### Discovery Integration

Blobs can be fetched from nodes discovered through iroh's discovery mechanisms:
- DNS-based discovery
- Local network discovery
- Relay-based discovery

### Content Protection

The blob store integrates with the docs engine for content protection:

```rust
/// Protect blob content from garbage collection
pub fn protect_cb(&self) -> ProtectCb {
    let sync = self.sync.clone();
    Box::new(move |live| {
        Box::pin(async move {
            let doc_hashes = sync.content_hashes().await.unwrap();
            for hash in doc_hashes {
                live.insert(hash.unwrap());
            }
        })
    })
}
```

---

## Production Usage Patterns

### Basic Blob Operations

```rust
// Import a file
let temp_tag = store.blobs().import_path(path, temp_tag()).await?;
let hash = temp_tag.hash();

// Export a blob to file
store.blobs().export_bao(hash, &mut file).await?;

// Check blob availability
let status = store.blobs().blob_status(hash).await?;
```

### Streaming Imports

For large files, use streaming import:

```rust
let (sender, receiver) = mpsc::channel(1);
let temp_tag = store.blobs()
    .import_byte_stream(Box::pin(receiver), temp_tag())
    .await?;

// Stream data
sender.send(chunk).await?;
```

### Multi-Source Downloads

Use the downloader for complex scenarios:

```rust
let downloader = store.downloader(&endpoint);
downloader
    .queue(hash, GetRequest::all(hash))
    .add_source(node_addr)
    .await?;
downloader.run().await?;
```

### Tag Management

```rust
// Create persistent tag
let tag = store.tags().set_tag("my-blob", hash).await?;

// List all tags
let tags: Vec<_> = store.tags().list_tags().await?.collect();

// Auto-cleanup with temp tags
let temp_tag = store.blobs().import_bytes(data, temp_tag()).await?;
// Tag automatically deleted when dropped
```

### Garbage Collection

The file store runs automatic GC:
- Protected blobs (via `protect_cb`) are retained
- Unreferenced blobs are cleaned up periodically
- Partial blobs are retained for resumable downloads

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| bao-tree | 0.15.1 | BLAKE3 verified streaming |
| bytes | 1.x | Byte buffer management |
| redb | 2.4 | Embedded key-value store |
| quinn | 0.14.0 | QUIC implementation |
| postcard | 1.1.1 | Serialization |
| tokio | 1.43.0 | Async runtime |
| irpc | 0.5.0 | RPC framework |

### Notable Rust Patterns

1. **Type-Safe Range Building**: Builder pattern for `GetRequest` with compile-time range validation
2. **Zero-Copy Operations**: Extensive use of `Bytes` for zero-copy buffer management
3. **Newtype Patterns**: `Hash`, `TempTag`, `Tag` provide type safety
4. **Result Aliases**: Custom error types with `Snafu` for ergonomic error handling

### Async Architecture

- Uses Tokio for async runtime
- Actor model with message passing between main and database actors
- Task spawning for long-running operations (imports, exports)
- Channel-based communication with backpressure

### Memory Management

- Reference counting via `Arc` for shared state
- Weak references for cleanup notifications
- Explicit protection handles to prevent GC during operations
- Linear I/O model - futures must be polled to completion

### Potential Enhancements

1. **Streaming Exports**: Current export could benefit from more streaming options
2. **Parallel Fetching**: Enhanced multi-source parallel chunk fetching
3. **Compression**: Optional compression layer for transfers
4. **Encryption**: At-rest encryption support for sensitive blobs

---

## Summary

`iroh-blobs` provides a production-ready, content-addressed blob storage system with:

- **Verifiable transfers** using BLAKE3 Merkle trees
- **Efficient range requests** with minimal overhead
- **Flexible storage** with file system and memory backends
- **Actor-based architecture** for concurrent operations
- **Integration hooks** for the broader iroh ecosystem

The module exemplifies careful design trade-offs prioritizing data integrity while maintaining practical performance characteristics for real-world usage.
