---
title: Data Flow — End-to-End Document Synchronization Sequences
---

# Data Flow — End-to-End Document Synchronization Sequences

## Local Insert Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant Engine as Engine
    participant LiveActor as LiveActor
    participant SyncHandle as SyncHandle
    participant Actor as Actor (thread)
    participant Store as redb Store
    participant Gossip as iroh-gossip

    App->>Engine: insert(author, key, value)
    Engine->>SyncHandle: InsertLocal action
    SyncHandle->>Actor: queue action
    Actor->>Store: write entry (hash, timestamp, signatures)
    Store-->>Actor: entry persisted
    Actor->>SyncHandle: event: LocalInsert
    SyncHandle->>LiveActor: notify via channel
    LiveActor->>Gossip: broadcast Op::Put(entry)
    LiveActor-->>App: LiveEvent::InsertLocal
```

Source: `iroh-docs/src/engine/live.rs:1` (LiveActor), `iroh-docs/src/actor.rs:1` (Actor).

## Peer-to-Peer Sync Flow

```mermaid
sequenceDiagram
    participant A as Node A (Alice)
    participant B as Node B (Bob)
    participant ARep as A Replica
    participant BRep as B Replica
    participant AStore as A Store
    participant BStore as B Store

    A->>B: open QUIC stream (ALPN: /iroh-sync/1)
    A->>B: Init(namespace, fingerprint(full_range))
    B->>A: fingerprint(full_range)
    alt fingerprints differ
        A->>B: Init(namespace, fingerprint(left_half))
        B->>A: fingerprint(left_half)
        alt fingerprints differ
            A->>B: Items(left_half)
            B->>B: validate + store items
        end
        A->>B: Init(namespace, fingerprint(right_half))
        B->>A: fingerprint(right_half)
        alt fingerprints differ
            B->>A: Items(right_half)
            A->>A: validate + store items
        end
    end
    A->>B: close stream
```

Source: `iroh-docs/src/net.rs:1` (Alice/Bob), `iroh-docs/src/ranger.rs:1` (fingerprint comparison).

## Gossip Propagation Flow

```mermaid
sequenceDiagram
    participant A as Node A
    participant GossipA as iroh-gossip topic A
    participant GossipB as iroh-gossip topic B
    participant B as Node B

    A->>A: insert local entry
    A->>GossipA: publish Op::Put(entry)
    GossipA->>GossipB: gossip propagation
    GossipB->>B: deliver Op::Put(entry)
    B->>B: validate entry (namespace sig, author sig, timestamp)
    B->>B: store entry
    B->>App: LiveEvent::InsertRemote
```

Source: `iroh-docs/src/engine/gossip.rs:1`.

## Content Download Flow

```mermaid
sequenceDiagram
    participant Local as Local Node
    participant Remote as Remote Node
    participant Blobs as iroh-blobs

    Local->>Remote: receive SignedEntry (content_hash)
    Local->>Local: check download policy
    alt policy matches
        Local->>Blobs: start_download(content_hash)
        Blobs->>Remote: request content by hash
        Remote-->>Blobs: stream content chunks
        Blobs->>Blobs: verify each chunk (BLAKE3)
        Blobs-->>Local: download complete
        Local->>App: LiveEvent::ContentReady
    else policy does not match
        Local->>Local: skip download (metadata only)
    end
```

Source: `iroh-docs/src/engine/live.rs:1` (download queue), `iroh-docs/src/store.rs:1` (download policies).

## Ticket-Based Join Flow

```mermaid
sequenceDiagram
    participant Publisher as Publisher
    participant Consumer as Consumer
    participant DocA as Publisher Docs
    participant DocB as Consumer Docs
    participant Engine as Engine

    Publisher->>DocA: create replica
    DocA->>Publisher: DocTicket (capability + NodeAddr)
    Publisher->>Consumer: share ticket (QR/text)
    Consumer->>DocB: import ticket
    DocB->>Engine: start_sync(namespace, Publisher)
    Engine->>Publisher: connect via QUIC
    Publisher->>Engine: accept sync
    Engine->>Engine: range-based set reconciliation
    Engine->>Consumer: replica synced
```

Source: `iroh-docs/src/ticket.rs:1` (DocTicket).

## Related Documents

- [Network](../markdown/05-network.md) — Alice/Bob protocol
- [Engine](../markdown/07-engine.md) — Live sync coordination
