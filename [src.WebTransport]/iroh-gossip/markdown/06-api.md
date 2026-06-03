---
title: API — GossipApi, GossipTopic, Events, Commands, and RPC
---

# API — GossipApi, GossipTopic, Events, Commands, and RPC

The API layer provides the high-level interface for applications to publish messages and consume gossip events.

## GossipApi

The `GossipApi` is the main entry point for interacting with the gossip protocol:

```rust
// iroh-gossip/src/api.rs
pub struct GossipApi {
    /// Sender to the actor's command channel.
    sender: GossipSender,
    /// Receiver for events from the actor.
    receiver: mpsc::Receiver<Event>,
}

impl GossipApi {
    /// Join a topic and get a GossipTopic handle.
    pub async fn join(&self, topic: TopicId, opts: JoinOptions) -> Result<GossipTopic> { ... }

    /// Leave a topic.
    pub async fn leave(&self, topic: TopicId) -> Result<()> { ... }
}
```

Source: `iroh-gossip/src/api.rs:1` — `GossipApi` wraps the command channel to the networking actor.

## GossipTopic

Each joined topic provides a `GossipTopic` handle:

```rust
// iroh-gossip/src/api.rs
pub struct GossipTopic {
    topic: TopicId,
    sender: GossipSender,
    receiver: GossipReceiver,
}

impl GossipTopic {
    /// Publish a message to all peers in this topic.
    pub async fn publish(&self, message: Bytes) -> Result<()> { ... }

    /// Receive the next event for this topic.
    pub async fn next(&mut self) -> Option<Event> { ... }
}
```

Source: `iroh-gossip/src/api.rs:1` — `GossipTopic` provides publish and event stream access.

## Events

```rust
// iroh-gossip/src/api.rs
pub enum Event {
    /// A message was received from a peer.
    Received {
        peer: PI,
        message: Bytes,
    },
    /// We joined a topic successfully.
    Joined {
        topic: TopicId,
        peers: Vec<PI>,
    },
    /// A peer joined the active view.
    NeighborUp {
        peer: PI,
    },
    /// A peer left the active view.
    NeighborDown {
        peer: PI,
    },
}
```

Source: `iroh-gossip/src/api.rs:1` — Four event types for application consumption.

## Commands

```rust
// iroh-gossip/src/api.rs
pub enum Command {
    /// Join a topic.
    Join(JoinRequest),
    /// Leave a topic.
    Leave(TopicId),
    /// Publish a message.
    Publish { topic: TopicId, message: Bytes },
    /// Subscribe to events.
    Subscribe { topic: TopicId },
}
```

Source: `iroh-gossip/src/api.rs:1` — Commands sent from the API to the networking actor.

## JoinOptions

```rust
// iroh-gossip/src/api.rs
pub struct JoinOptions {
    /// Bootstrap peers to connect to initially.
    pub bootstrap: Vec<PI>,
    /// Additional application data to share with peers.
    pub extra: Bytes,
}
```

Source: `iroh-gossip/src/api.rs:1` — Join options control initial bootstrap and peer data.

## Message Signing

Messages are signed with Ed25519:

```rust
// iroh-gossip/src/api.rs
pub struct Message {
    /// The message payload.
    pub payload: Bytes,
    /// Signature of the payload.
    pub signature: Signature,
    /// Author's public key.
    pub author: PublicKey,
    /// Timestamp for ordering.
    pub timestamp: u64,
}
```

Source: `iroh-gossip/src/api.rs:1` — Messages include author, signature, and timestamp for application-level ordering.

## RPC (Optional)

When the `rpc` feature is enabled, iroh-gossip provides an irpc-based RPC API:

```rust
// iroh-gossip/src/api.rs (rpc feature)
#[derive(irpc::Service)]
pub trait GossipRpc {
    async fn join(&self, req: JoinRequest) -> Result<JoinResponse>;
    async fn publish(&self, req: PublishRequest) -> Result<()>;
    async fn peers(&self, req: PeersRequest) -> Result<PeersResponse>;
}
```

Source: `iroh-gossip/src/api.rs` (cfg feature = "rpc") — irpc service definition.

## APIError

```rust
// iroh-gossip/src/api.rs
pub enum ApiError {
    /// Topic not joined.
    NotJoined,
    /// Connection to peer failed.
    ConnectFailed,
    /// Message too large.
    TooLarge(usize),
    /// Actor channel closed.
    ChannelClosed,
}
```

Source: `iroh-gossip/src/api.rs:1` — API error types.

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Protocol layers
- [Networking](../markdown/05-networking.md) — Networking layer driven by API
- [Data Flow](../markdown/09-data-flow.md) — Publish and event propagation flows
