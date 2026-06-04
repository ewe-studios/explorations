---
title: Network — Alice/Bob Sync Protocol Over QUIC Streams
---

# Network — Alice/Bob Sync Protocol Over QUIC Streams

The network module implements the Alice/Bob sync protocol over iroh QUIC bidirectional streams.

## Protocol ALPN

```rust
// iroh-docs/src/net.rs
pub const ALPN: &[u8] = b"/iroh-sync/1";
```

Source: `iroh-docs/src/net.rs:1`.

## Alice (Initiator) Protocol

```rust
// iroh-docs/src/net.rs
pub async fn connect_and_sync<S>(
    conn: &Connection,
    replica: &Replica<S>,
    config: SyncConfig,
) -> Result<SyncFinished>
```

Alice:
1. Opens a bidirectional QUIC stream
2. Sends the namespace ID to sync
3. Runs the ranger sync algorithm as the initiator
4. Returns `SyncFinished` with timing and outcome data

Source: `iroh-docs/src/net.rs:1` — `connect_and_sync()`.

## Bob (Acceptor) Protocol

```rust
// iroh-docs/src/net.rs
pub async fn handle_connection<S>(
    conn: &Connection,
    store: &S,
    accept: impl Fn(&NamespaceId) -> AcceptOutcome,
) -> Result<SyncFinished>
```

Bob:
1. Accepts the incoming bidirectional stream
2. Reads the namespace ID
3. Calls the `accept` callback to authorize the sync
4. Runs the ranger sync algorithm as the acceptor
5. Returns `SyncFinished`

Source: `iroh-docs/src/net.rs:1` — `handle_connection()`.

## SyncCodec Wire Protocol

```rust
// iroh-docs/src/net/codec.rs
pub enum Message {
    /// Initialize sync with namespace and ranger message.
    Init(NamespaceId, ranger::Message),
    /// Continue sync with ranger message.
    Sync(ranger::Message),
    /// Abort sync with reason.
    Abort(AbortReason),
}
```

Messages are length-prefixed and postcard-encoded.

Source: `iroh-docs/src/net/codec.rs:1`.

## Abort Reasons

```rust
// iroh-docs/src/net.rs
pub enum AbortReason {
    /// Namespace not found on this node.
    NotFound,
    /// Already syncing this namespace with this peer.
    AlreadySyncing,
    /// Internal server error.
    InternalServerError,
}
```

Source: `iroh-docs/src/net.rs:1`.

## SyncFinished Result

```rust
// iroh-docs/src/net.rs
pub struct SyncFinished {
    /// The namespace that was synced.
    pub namespace: NamespaceId,
    /// The peer that was synced with.
    pub peer: NodeId,
    /// The sync outcome (entries sent/received).
    pub outcome: SyncOutcome,
    /// Timing information.
    pub timings: SyncTimings,
}
```

Source: `iroh-docs/src/net.rs:1`.

## run_alice / BobState

```rust
// iroh-docs/src/net/codec.rs
pub async fn run_alice<S>(
    codec: &mut SyncCodec,
    replica: &Replica<S>,
    config: SyncConfig,
) -> Result<SyncOutcome>

struct BobState {
    // State machine for acceptor protocol
    namespace: NamespaceId,
    // ...
}
```

Source: `iroh-docs/src/net/codec.rs:1` — `run_alice()` and `BobState` implement the sync state machines.

## Integration Tests

The codec module includes extensive integration tests with simulated duplex streams:
- Basic sync with matching entries
- Sync with differing entries
- Sync with partial overlap
- Abort handling (NotFound, AlreadySyncing)

Source: `iroh-docs/src/net/codec.rs` — `#[cfg(test)]` module.

## Related Documents

- [Ranger](../markdown/03-ranger.md) — Sync algorithm
- [Engine](../markdown/07-engine.md) — Live sync coordination
