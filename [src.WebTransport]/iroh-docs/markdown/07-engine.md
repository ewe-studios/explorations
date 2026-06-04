---
title: Engine — Live Sync Coordination and Gossip Integration
---

# Engine — Live Sync Coordination and Gossip Integration

The Engine coordinates the sync thread, live tokio actor, and gossip integration for real-time document synchronization.

## Engine Structure

```rust
// iroh-docs/src/engine.rs
pub struct Engine<D> {
    /// Background sync thread handle.
    sync_handle: SyncHandle,
    /// Tokio live actor.
    live_actor: LiveActor<D>,
    /// Gossip state manager.
    gossip_state: GossipState,
    /// Blobs store for content.
    blobs: Blobs,
}
```

Source: `iroh-docs/src/engine.rs:1`.

## Engine Builder

```rust
// iroh-docs/src/engine.rs
impl Engine<Live> {
    pub fn start_sync(&self, namespace: NamespaceId, peer: NodeId) -> Result<()> { ... }
    pub fn leave(&self, namespace: NamespaceId, peer: NodeId) -> Result<()> { ... }
    pub fn subscribe(&self, namespace: NamespaceId) -> Result<mpsc::Receiver<LiveEvent>> { ... }
    pub async fn handle_connection(&self, conn: Connection) -> Result<()> { ... }
    pub async fn shutdown(self) -> Result<()> { ... }
}
```

Source: `iroh-docs/src/engine.rs:1`.

## LiveEvent

```rust
// iroh-docs/src/engine.rs
pub enum LiveEvent {
    /// A local entry was inserted.
    InsertLocal { namespace, entry },
    /// A remote entry was inserted via sync.
    InsertRemote { namespace, entry, from },
    /// Downloaded content is ready.
    ContentReady { namespace, hash },
    /// Pending content became available.
    PendingContentReady { namespace },
    /// A neighbor joined a gossip swarm.
    NeighborUp { namespace, peer },
    /// A neighbor left a gossip swarm.
    NeighborDown { namespace, peer },
    /// A sync operation finished.
    SyncFinished { namespace, peer, outcome },
}
```

Source: `iroh-docs/src/engine.rs:1`.

## LiveActor Coordination

```rust
// iroh-docs/src/engine/live.rs
impl LiveActor {
    /// Sync with a peer via connect.
    async fn sync_with_peer(&mut self, namespace, peer) -> Result<()> { ... }

    /// Handle sync completion: register peer, broadcast report, manage downloads.
    async fn on_sync_finished(&mut self, result: &SyncFinished) -> Result<()> { ... }

    /// Handle replica events: broadcast inserts, queue downloads.
    async fn on_replica_event(&mut self, event: &Event) -> Result<()> { ... }

    /// Start downloading content from peers.
    async fn start_download(&mut self, hash: Hash, namespace: NamespaceId) -> Result<()> { ... }
}
```

Source: `iroh-docs/src/engine/live.rs:1`.

## QueuedHashes

```rust
// iroh-docs/src/engine/live.rs
struct QueuedHashes {
    /// Pending downloads by hash.
    by_hash: HashMap<Hash, Vec<NamespaceId>>,
    /// Pending downloads by namespace.
    by_namespace: HashMap<NamespaceId, VecDeque<Hash>>,
}
```

Tracks pending blob downloads to avoid duplicate download requests.

Source: `iroh-docs/src/engine/live.rs:1`.

## GossipState

```rust
// iroh-docs/src/engine/gossip.rs
struct GossipState {
    /// Active gossip subscriptions per namespace.
    subscriptions: HashMap<NamespaceId, GossipTopic>,
}
```

Manages joining/leaving gossip topics for document namespaces.

Source: `iroh-docs/src/engine/gossip.rs:1`.

## Gossip Message Processing

```rust
// iroh-docs/src/engine/gossip.rs
async fn receive_loop(&mut self, mut topic: GossipTopic) -> Result<()> {
    while let Some(event) = topic.next().await {
        match event {
            GossipEvent::Received(peer, data) => {
                let op: Op = postcard::from_bytes(&data)?;
                match op {
                    Op::Put(entry) => { /* insert remote entry */ }
                    Op::ContentReady(hash) => { /* mark content ready */ }
                    Op::SyncReport { heads } => { /* process sync report */ }
                }
            }
        }
    }
}
```

Source: `iroh-docs/src/engine/gossip.rs:1`.

## SyncState Machine

```rust
// iroh-docs/src/engine/state.rs
enum SyncReason {
    DirectJoin,
    NewNeighbor,
    SyncReport,
    Resync,
}

enum SyncState {
    Idle,
    Running { start_time: Instant, origin: Origin },
}
```

Prevents double-syncing and tracks resync requests.

Source: `iroh-docs/src/engine/state.rs:1`.

## Deterministic Tie-Breaking

When both peers try to sync simultaneously, the sync direction is determined by comparing node IDs:

```rust
// iroh-docs/src/engine/state.rs
fn expected_sync_direction(self_id: NodeId, peer_id: NodeId) -> Direction {
    if self_id < peer_id { Connect } else { Accept }
}
```

Source: `iroh-docs/src/engine/state.rs:1`.

## Default Author

```rust
// iroh-docs/src/engine.rs
pub enum DefaultAuthorStorage {
    /// In-memory (reset on restart).
    Mem,
    /// Persistent file-based storage.
    Persistent(PathBuf),
}
```

Source: `iroh-docs/src/engine.rs:1`.

## Related Documents

- [Sync Actor](../markdown/04-sync-actor.md) — Thread actor used by Engine
- [Network](../markdown/05-network.md) — Sync over QUIC streams
- [Keys](../markdown/08-keys.md) — Author and Namespace keys
