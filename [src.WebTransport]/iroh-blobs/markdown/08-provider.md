---
title: Provider — Server-Side Blob Transfer Handler
---

# Provider — Server-Side Blob Transfer Handler

The provider handles incoming iroh-blobs connections and serves blob content to requesting clients.

## Protocol Handler

```rust
// iroh-blobs/src/net_protocol.rs
pub struct BlobsProtocol<S> {
    store: S,
}

impl<S> ProtocolHandler for BlobsProtocol<S> {
    async fn accept(&self, connection: Connection) -> Result<()> {
        handle_connection(connection, &self.store).await
    }
}
```

Source: `iroh-blobs/src/net_protocol.rs:1` — `BlobsProtocol` implements iroh's `ProtocolHandler`, dispatching to `handle_connection`.

## Connection Handler

```rust
// iroh-blobs/src/provider.rs
pub async fn handle_connection<S>(conn: Connection, store: &S) -> Result<()> {
    let request = read_request(&mut conn).await?;
    match request {
        Request::Get(req) => handle_get(conn, req, store).await,
        Request::GetMany(req) => handle_get_many(conn, req, store).await,
        Request::Push(req) => handle_push(conn, req, store).await,
        Request::Observe(req) => handle_observe(conn, req, store).await,
    }
}
```

Source: `iroh-blobs/src/provider.rs:1` — `handle_connection` reads the request and dispatches to the appropriate handler.

## handle_get

Serves a single blob to the client:

1. Read the blob from store (content + outboard)
2. Send blob header (hash, size)
3. Stream chunks according to the requested ranges
4. Close connection

Source: `iroh-blobs/src/provider.rs:1` — `handle_get` serves a GetRequest.

## handle_get_many

Serves multiple blobs in a single connection:

1. Read all requested hashes
2. For each hash:
   - Read blob from store
   - Send header
   - Stream chunks
3. Close connection

Source: `iroh-blobs/src/provider.rs:1` — `handle_get_many` batches multiple blob transfers.

## handle_push

Receives a blob FROM the client:

1. Read blob header
2. Receive chunks + outboard
3. Verify each chunk
4. Store blob
5. Send completion acknowledgment

Source: `iroh-blobs/src/provider.rs:1` — `handle_push` is the reverse of `handle_get`.

## handle_observe

Checks blob existence without transferring:

1. Look up hash in store
2. Return status: complete/partial/not found
3. Return size if available
4. Close connection

Source: `iroh-blobs/src/provider.rs:1` — `handle_observe` is a lightweight existence check.

## Progress Reporting

```rust
// iroh-blobs/src/provider.rs
pub struct ProgressWriter {
    tx: mpsc::Sender<Event>,
}

pub struct ProgressReader {
    rx: mpsc::Receiver<Event>,
}
```

Source: `iroh-blobs/src/provider.rs:1` — Progress events are sent through a channel during transfers.

## Event Types

```rust
// iroh-blobs/src/provider.rs
pub enum Event {
    /// Client connected.
    Connected {
        connection_id: u64,
    },
    /// Transfer started.
    TransferStarted {
        hash: Hash,
    },
    /// Chunk sent.
    ChunkSent {
        offset: u64,
        size: u64,
    },
    /// Transfer completed.
    TransferCompleted {
        hash: Hash,
        stats: TransferStats,
    },
    /// Transfer aborted.
    TransferAborted {
        hash: Hash,
        error: String,
    },
}
```

Source: `iroh-blobs/src/provider.rs:1` — Events track the lifecycle of each transfer.

## TransferStats

```rust
// iroh-blobs/src/provider.rs
pub struct TransferStats {
    /// Bytes sent.
    pub bytes_sent: u64,
    /// Chunks sent.
    pub chunks_sent: u64,
    /// Transfer duration.
    pub duration: Duration,
}
```

Source: `iroh-blobs/src/provider.rs:1` — Per-transfer statistics.

## send_blob

```rust
// iroh-blobs/src/provider.rs
pub async fn send_blob<S>(conn: &mut Connection, store: &S, hash: Hash) -> Result<TransferStats> {
    // Read blob from store
    // Send header
    // Stream chunks with outboard
    // Return stats
}
```

Source: `iroh-blobs/src/provider.rs:1` — `send_blob` is the core blob sending function used by all handlers.

## Related Documents

- [Protocol](../markdown/03-protocol.md) — Request format
- [Get Client](../markdown/07-get-client.md) — Client-side counterpart
- [Data Flow](../markdown/09-data-flow.md) — Complete transfer sequence
