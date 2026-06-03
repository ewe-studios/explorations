---
title: Cross-Cutting — Tickets, Collections, Metrics, Temp Tags
---

# Cross-Cutting Concerns — Tickets, Collections, Metrics, Temp Tags

These concerns span multiple modules in iroh-blobs.

## BlobTicket

```rust
// iroh-blobs/src/ticket.rs
pub struct BlobTicket {
    addr: NodeAddr,
    hash: Hash,
    format: BlobFormat,
}
```

Source: `iroh-blobs/src/ticket.rs:1` — `BlobTicket` serializes to postcard/base32 for easy sharing.

### Serialization

Tickets are encoded as base32 strings:

```
BlobTicket → postcard → base32 → "ticket string"
```

Share tickets via QR code, clipboard, or any text channel.

## Collection

```rust
// iroh-blobs/src/format/collection.rs
pub struct Collection {
    blobs: Vec<(String, Hash)>,  // (name, hash) pairs
}
```

Source: `iroh-blobs/src/format/collection.rs:1` — A collection groups named blobs under a single root hash.

### Collection Wire Format

```
CollectionMeta (serialized with postcard):
  ├── version: u64
  ├── blobs: Vec<(String, Hash)>
```

The collection itself is stored as a HashSeq blob, where each child is a named blob.

## TempTag

```rust
// iroh-blobs/src/util/temp_tag.rs
pub struct TempTag {
    hash: Hash,
    counter: Arc<AtomicUsize>,
}
```

Source: `iroh-blobs/src/util/temp_tag.rs:1` — `TempTag` protects content from garbage collection while it's in use. When the `TempTag` is dropped, the counter decrements and GC can eventually reclaim the blob if no other references exist.

### TempTag Lifecycle

```
import bytes → create TempTag → use blob → drop TempTag → GC can reclaim
```

## Metrics

```rust
// iroh-blobs/src/metrics.rs
pub struct Metrics {
    pub download_bytes_total: Counter,
    pub download_time_total: Counter,
    pub downloads_success: Counter,
    pub downloads_error: Counter,
    pub downloads_notfound: Counter,
    pub downloader_tick_main: Counter,
    pub downloader_tick_rx: Counter,
    pub downloader_tick_discovery: Counter,
    pub downloader_tick_dialer: Counter,
    pub downloader_tick_connections: Counter,
}
```

Source: `iroh-blobs/src/metrics.rs:1` — 10 Prometheus counters for download tracking.

## BlobsProtocol ALPN

```rust
// iroh-blobs/src/net_protocol.rs
pub const ALPN: &[u8] = b"/iroh-blobs/1";
```

Source: `iroh-blobs/src/net_protocol.rs:1` — Registered with the iroh Router.

## Blob Status

```rust
// iroh-blobs/src/api/proto.rs
pub enum BlobStatus {
    /// Blob is fully stored.
    Complete { size: u64 },
    /// Blob is partially stored.
    Partial { size: Option<u64> },
    /// Blob is not stored at all.
    NotFound,
}
```

Source: `iroh-blobs/src/api/proto.rs:1` — Three status variants for blob existence queries.

## Examples

| Example | Description |
|---------|-------------|
| `transfer.rs` | Simple file send/receive via BlobTicket |
| `get-blob.rs` | Fetch blob from remote without store |
| `mdns-discovery.rs` | Local network file transfer with mDNS |
| `custom-protocol.rs` | Text search over blobs with custom protocol |
| `random_store.rs` | Provide/request random blobs |

Source: `iroh-blobs/examples/`

## Related Documents

- [Overview](../markdown/00-overview.md) — What iroh-blobs is
- [Protocol](../markdown/03-protocol.md) — Wire format
- [API](../markdown/06-api.md) — High-level API
