---
title: Sync Actor — Thread Actor + Tokio Live Actor Coordination
---

# Sync Actor — Thread Actor + Tokio Live Actor Coordination

iroh-docs uses two actors: a thread-based sync actor for sequential redb operations, and a tokio-based live actor for async network coordination.

## SyncHandle: Thread-Safe Interface

```rust
// iroh-docs/src/actor.rs
pub struct SyncHandle {
    /// Sender to the actor's async channel.
    sender: mpsc::Sender<Action>,
    /// Handle to the actor thread.
    thread: JoinHandle<()>,
}
```

Source: `iroh-docs/src/actor.rs:1` — `SyncHandle` wraps an `std::thread` running the `Actor`.

## Action Message Enum

```rust
// iroh-docs/src/actor.rs
pub enum Action {
    /// Import/export/delete authors.
    ImportAuthor { ... },
    ExportAuthor { ... },
    DeleteAuthor { ... },
    /// Import namespace capability.
    ImportNamespace { ... },
    /// Open/close replicas.
    OpenReplica { ... },
    CloseReplica { ... },
    /// Insert entries.
    InsertLocal { ... },
    InsertRemote { ... },
    /// Process sync messages from ranger.
    ProcessSyncMessage { ... },
    /// Query entries.
    Query { ... },
    /// Manage download policies.
    SetDownloadPolicy { ... },
    /// Shutdown the actor.
    Shutdown { reply: oneshot::Sender<()> },
}
```

Source: `iroh-docs/src/actor.rs:1` — All operations go through the Action channel.

## Automatic Flush

```rust
// iroh-docs/src/actor.rs
const MAX_COMMIT_DELAY: Duration = Duration::from_millis(500);
```

The actor automatically flushes pending redb transactions every 500ms. This ensures durability without requiring explicit sync calls after every operation.

Source: `iroh-docs/src/actor.rs:1`.

## OpenReplicas Tracking

```rust
// iroh-docs/src/actor.rs
struct OpenReplicas {
    /// Active replica states by namespace ID.
    replicas: HashMap<NamespaceId, ReplicaState>,
    /// Handle count per namespace (for auto-close).
    handle_count: HashMap<NamespaceId, usize>,
}
```

When the last handle for a namespace is dropped, the replica is automatically closed and its resources freed.

Source: `iroh-docs/src/actor.rs:1`.

## LiveActor: Tokio Task

```rust
// iroh-docs/src/engine/live.rs
pub struct LiveActor<D> {
    /// Sender to the live actor channel.
    sender: mpsc::Sender<ToLiveActor<D>>,
}
```

The `LiveActor` runs as a tokio task and coordinates:
- Sync connections with peers (connect/accept)
- Gossip broadcast of entry updates
- Blob downloads from remote peers
- Event subscription for applications

Source: `iroh-docs/src/engine/live.rs:1`.

## ToLiveActor Messages

```rust
// iroh-docs/src/engine/live.rs
pub enum ToLiveActor<D> {
    StartSync { origin, namespace, peer },
    Leave { namespace, peer },
    Subscribe { namespace, sender },
    HandleConnection { connection },
    NeighborUp { peer },
    NeighborDown { peer },
    IncomingSyncReport { peer, entries },
    // ... more
}
```

Source: `iroh-docs/src/engine/live.rs:1`.

## Gossip Integration

The LiveActor integrates with iroh-gossip to broadcast entry updates:

```rust
// iroh-docs/src/engine/live.rs
pub enum Op {
    /// Broadcast a new entry via gossip.
    Put(SignedEntry),
    /// Signal that content is ready for download.
    ContentReady(Hash),
    /// Broadcast a sync report (AuthorHeads).
    SyncReport { heads: AuthorHeads },
}
```

Source: `iroh-docs/src/engine/live.rs:1`.

## Drop Implementation

```rust
// iroh-docs/src/actor.rs
impl Drop for SyncHandle {
    fn drop(&mut self) {
        self.sender.send(Action::Shutdown(...));
        self.thread.join();
    }
}
```

When the `SyncHandle` is dropped, it sends a shutdown message and joins the thread, ensuring all pending operations are flushed.

Source: `iroh-docs/src/actor.rs:1`.

## Related Documents

- [Engine](../markdown/07-engine.md) — LiveActor coordination
- [Storage](../markdown/06-storage.md) — redb persistence
- [Network](../markdown/05-network.md) — Sync over QUIC streams
