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
pub async fn handle_connection<S>(
    conn: Connection,
    store: &S,
    progress: EventSender,
) -> Result<()> {
    // Authorization step before accepting requests
    // ...
    handle_stream(conn, store, progress).await?;
}
```

Source: `iroh-blobs/src/provider.rs:1` — `handle_connection` includes an authorization step and dispatches through `handle_stream`. Push requests require explicit authorization via `authorize_push_request`. Reserved slots (Slot2-Slot7) fall through to `anyhow::bail!("unsupported request")`.

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
    /// A client connected to the provider.
    ClientConnected { connection: Connection },
    /// The connection was closed.
    ConnectionClosed { connection_id: u64 },
    /// A Get request was received.
    GetRequestReceived { hash: Hash },
    /// A GetMany request was received.
    GetManyRequestReceived { hashes: Vec<Hash> },
    /// A Push request was received.
    PushRequestReceived { hash: Hash },
    /// A transfer has started.
    TransferStarted { hash: Hash },
    /// Progress update during transfer.
    TransferProgress { offset: u64, size: u64 },
    /// Transfer completed successfully.
    TransferCompleted { hash: Hash, stats: TransferStats },
    /// Transfer was aborted.
    TransferAborted { hash: Hash },
}
```

Source: `iroh-blobs/src/provider.rs:1` — 9 event variants tracking the full provider lifecycle.
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
    /// Payload bytes sent (blob content + outboard).
    pub payload_bytes_sent: u64,
    /// Non-payload bytes sent (protocol overhead).
    pub other_bytes_sent: u64,
    /// Bytes read from store.
    pub bytes_read: u64,
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
