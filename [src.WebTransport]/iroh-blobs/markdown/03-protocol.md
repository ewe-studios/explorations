---
title: Protocol — Wire Format for Get, GetMany, Push, Observe
---

# Protocol — Wire Format for Get, GetMany, Push, Observe

The iroh-blobs protocol defines four request types for transferring blobs over iroh connections.

## Protocol ALPN

```rust
// iroh-blobs/src/protocol.rs
pub const ALPN: &[u8] = b"/iroh-bytes/4";
```

Source: `iroh-blobs/src/protocol.rs:1` — The ALPN used for iroh-blobs connections.

## Request Types

```rust
// iroh-blobs/src/protocol.rs
pub enum Request {
    /// Get a single blob or hash sequence.
    Get(GetRequest),
    /// Observe a blob without transferring (check existence/size).
    Observe(ObserveRequest),
    /// Reserved slots for future extensions.
    Slot2, Slot3, Slot4, Slot5, Slot6, Slot7,
    /// Push a blob to the remote (reverse direction).
    Push(PushRequest),
    /// Get multiple blobs in a single connection.
    GetMany(GetManyRequest),
}
```

Source: `iroh-blobs/src/protocol.rs:1` — 10 request variants (4 active + 6 reserved).

## GetRequest

```rust
// iroh-blobs/src/protocol.rs
pub struct GetRequest {
    /// The root hash to fetch.
    pub hash: Hash,
    /// Which ranges to fetch (can be "all" or specific ranges).
    pub ranges: ChunkRangesSeq,
}
```

Source: `iroh-blobs/src/protocol.rs:1` — `GetRequest` fetches a single blob. Format (Raw vs HashSeq) is determined by the ranges requested, not a separate field.

## GetManyRequest

```rust
// iroh-blobs/src/protocol.rs
pub struct GetManyRequest {
    /// Multiple root hashes to fetch.
    pub hashes: Vec<Hash>,
    /// Per-hash range specifications.
    pub ranges: ChunkRangesSeq,
}
```

Source: `iroh-blobs/src/protocol.rs:1` — `GetManyRequest` fetches multiple blobs. Uses `ChunkRangesSeq` (not `RangeSpecSeq`).

## PushRequest

```rust
// iroh-blobs/src/protocol.rs
pub struct PushRequest(GetRequest);
```

Source: `iroh-blobs/src/protocol.rs:1` — `PushRequest` is a newtype wrapping `GetRequest`. It reuses the same hash and ranges structure — format is implicit.

## ObserveRequest

```rust
// iroh-blobs/src/protocol.rs
pub struct ObserveRequest {
    /// The hash to check.
    pub hash: Hash,
    /// Range specification for partial existence checks.
    pub ranges: RangeSpec,
}
```

Source: `iroh-blobs/src/protocol.rs:1` — `ObserveRequest` checks blob existence with optional range filtering.

## RangeSpec and ChunkRangesSeq

```rust
// iroh-blobs/src/protocol/range_spec.rs
pub struct ChunkRangesSeq {
    /// Run-length encoded range specifications.
    specs: Vec<RangeSpec>,
}

pub struct RangeSpec {
    /// Number of chunks this spec covers.
    len: u64,
    /// The range (e.g., "all chunks", "no chunks", "every Nth chunk").
    spec: RangeSpecType,
}
```

Source: `iroh-blobs/src/protocol/range_spec.rs:1` — `ChunkRangesSeq` uses run-length encoding for efficient range specification.

### RangeSpec Types

| Type | Purpose |
|------|---------|
| `All` | Request all chunks |
| `None` | Request no chunks (header only) |
| `Sparse` | Request specific chunks at intervals |

**Aha:** The run-length encoding in `ChunkRangesSeq` is essential for large blobs. Instead of listing every chunk to fetch (millions of entries), a single `RangeSpec { len: 1000000, spec: All }` requests all 1 million chunks. For partial re-downloads, sparse specs can request "every 100th chunk" efficiently.

## Wire Format: Transfer Sequence

```
1. Client sends: Request (Get/GetMany/Push/Observe)
2. Server validates request
3. For Get/GetMany:
   a. Server sends blob header (hash, size)
   b. Server sends chunks + outboard nodes
   c. Client verifies each chunk
   d. Repeat for all requested blobs
4. For Push:
   a. Client sends blob header
   b. Client sends chunks + outboard nodes
5. For Observe:
   a. Server returns status (exists/size)
6. Connection closed
```

Source: `iroh-blobs/src/provider.rs:1` — `handle_connection` dispatches to the appropriate handler.

## Closed Error Codes

```rust
// iroh-blobs/src/protocol.rs
#[repr(u32)]
pub enum Closed {
    /// Stream was dropped without explicit close.
    StreamDropped = 0,
    /// Provider is terminating (server shutting down).
    ProviderTerminating = 1,
    /// Request was received but could not be processed.
    RequestReceived = 2,
}
```
```

Source: `iroh-blobs/src/protocol.rs:1` — Connection close reasons.

## Related Documents

- [Hash and Bao](../markdown/02-hash-and-bao.md) — BLAKE3 hashing and bao outboards
- [Get Client](../markdown/07-get-client.md) — Client FSM processing requests
- [Provider](../markdown/08-provider.md) — Server-side request handling
