---
title: Data Flow — End-to-End Gossip Propagation Sequences
---

# Data Flow — End-to-End Gossip Propagation Sequences

This document traces the complete data flow from application publish to message delivery across the swarm.

## Initial Join Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant GossipApi as GossipApi
    participant Actor as Actor (net.rs)
    participant Dialer as Dialer
    participant Bootstrap as Bootstrap Peer
    participant HyPar as HyParView State
    participant Plum as PlumTree State
    participant Peers as Swarm Peers

    App->>GossipApi: join(topic_id, bootstrap)
    GossipApi->>Actor: Command::Join
    Actor->>HyPar: InEvent::PeerUp(bootstrap)
    HyPar-->>Actor: OutEvents: ConnectTo(more peers)
    Actor->>Dialer: connect(more peers)
    Dialer->>Peers: QUIC connections
    Peers-->>Actor: Connection established
    Actor->>HyPar: InEvent::PeerUp for each
    HyPar-->>Actor: Neighbor exchanges (shuffle, join)
    Actor->>GossipApi: Event::Joined(topic_id, peers)
    GossipApi-->>App: GossipTopic handle
```

Source: `iroh-gossip/src/net.rs:1` (Actor join flow), `iroh-gossip/src/proto/hyparview.rs:1` (HyParView join).

## Message Broadcast Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant Topic as GossipTopic
    participant Actor as Actor
    participant Plum as PlumTree State
    participant Eager as Eager Peers (tree)
    participant Lazy as Lazy Peers (backup)

    App->>Topic: publish(message)
    Topic->>Actor: Command::Publish
    Actor->>Plum: InEvent::Broadcast(message)
    Plum-->>Actor: OutEvents:
        SendMessage(eager_peer_1, Gossip(msg))
        SendMessage(eager_peer_2, Gossip(msg))
        SendMessage(lazy_peer_1, IHave(msg_id))
        SendMessage(lazy_peer_2, IHave(msg_id))
    Actor->>Eager: Gossip(message_id, payload)
    Actor->>Lazy: IHave(message_id)
    Eager->>Eager: deliver to application
    Eager->>Eager: forward to their eager peers
    Lazy->>Lazy: already received? (check cache)
    alt not yet received
        Lazy->>Actor: IWant(message_id)
        Actor->>Plum: InEvent::RecvMessage(IWant)
        Plum-->>Actor: OutEvent: SendMessage(Lazy, Gossip)
        Actor->>Lazy: Gossip(message_id, payload)
        Lazy->>Lazy: deliver to application
    else already received
        Lazy->>Lazy: ignore (duplicate)
    end
```

Source: `iroh-gossip/src/proto/plumtree.rs:1` (PlumTree broadcast flow), `iroh-gossip/src/api.rs:1` (publish).

**Aha:** The broadcast flow shows why PlumTree achieves O(N) message complexity. Each message is sent eagerly along tree edges (exactly N-1 sends for N peers) and lazily along non-tree edges (only if the tree path fails). The IHave/IWant exchange ensures delivery even when tree edges fail.

## Peer Discovery and Connection Flow

```mermaid
sequenceDiagram
    participant A as Peer A
    participant B as Peer B
    participant HyPar as HyParView State
    participant Actor as Actor
    participant Dialer as Dialer

    A->>HyPar: shuffle timer fires
    HyPar-->>Actor: OutEvent: SendMessage(B, Shuffle([C, D]))
    Actor->>B: Shuffle([C, D])
    B->>HyPar: InEvent: RecvMessage(Shuffle)
    HyPar-->>Actor: OutEvent: SendMessage(B, ShuffleReply([E, F]))
    Actor->>A: ShuffleReply([E, F])
    A->>HyPar: InEvent: RecvMessage(ShuffleReply)
    HyPar-->>Actor: OutEvent: ConnectTo(E), ConnectTo(F)
    Actor->>Dialer: dial(E), dial(F)
    Dialer->>E: QUIC connection
    Dialer->>F: QUIC connection
    E-->>A: connected (new active view peer)
    F-->>A: connected (new active view peer)
```

Source: `iroh-gossip/src/proto/hyparview.rs:1` (shuffle protocol), `iroh-gossip/src/net.rs:1` (dialer).

## Gossip Topic Subscription Flow

```mermaid
sequenceDiagram
    participant Sub as New Subscriber
    participant Topic as Existing Topic State
    participant Actor as Actor
    participant Pub as Publisher

    Sub->>Actor: join(topic_id)
    Actor->>Topic: InEvent::PeerUp(Sub)
    Topic->>Actor: OutEvent: ConnectTo(Sub)
    Actor->>Sub: establish connection
    Sub->>Topic: InEvent::PeerUp complete
    Topic-->>Actor: OutEvent: EmitEvent(Joined)
    Actor->>Sub: Event::Joined
    Pub->>Topic: publish(new_message)
    Topic-->>Actor: OutEvents: forward to Sub (now in tree)
    Actor->>Sub: forward new_message
    Sub-->>Sub: deliver to application
```

Source: `iroh-gossip/src/proto/topic.rs:1` (topic state), `iroh-gossip/src/net.rs:1` (topic subscriber loop).

## Peer Disconnect and Recovery Flow

```mermaid
sequenceDiagram
    participant A as Peer A
    participant B as Peer B (failing)
    participant Actor as Actor
    participant HyPar as HyParView State
    participant Passive as Passive View

    B->>A: QUIC connection drops
    Actor->>HyPar: InEvent::PeerDown(B)
    HyPar->>HyPar: remove B from active view
    HyPar->>Passive: promote peer C from passive view
    HyPar-->>Actor: OutEvent: ConnectTo(C)
    Actor->>Dialer: dial(C)
    Dialer->>C: QUIC connection
    C-->>Actor: connected
    Actor->>HyPar: InEvent::PeerUp(C)
    Note over A,C: Active view restored, tree reformed
```

Source: `iroh-gossip/src/proto/hyparview.rs:1` (peer disconnect handling), `iroh-gossip/src/proto/topic.rs:1` (recovery).

## Related Documents

- [HyParView](../markdown/02-hyparview.md) — Membership protocol flows
- [PlumTree](../markdown/03-plumtree.md) — Broadcast tree propagation
- [API](../markdown/06-api.md) — Application-facing API
