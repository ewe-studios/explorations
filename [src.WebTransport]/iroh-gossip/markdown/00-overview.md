---
title: Overview — What iroh-gossip Is and Why Epidemic Broadcast Trees
---

# Overview — What iroh-gossip Is and Why Epidemic Broadcast Trees

Iroh-gossip is a P2P gossip protocol for broadcasting messages to all peers subscribed to a topic, using epidemic broadcast trees for efficient dissemination.

## The Problem: Broadcasting to a P2P Swarm

When a swarm of peers subscribes to a topic, how do you ensure every message reaches every subscriber?

```
Naive flooding:                    Epidemic broadcast tree:

  A → B → C → D                   A ──eager──▶ B
  A → C (duplicate!)                 │           │
  A → D (duplicate!)              lazy▼       eager▼
  O(n²) messages per broadcast      C ─◄─────── D
                                     (pruned lazy edge)
                                     O(n) messages per broadcast
```

**Key insight:** Epidemic broadcast trees (PlumTree) reduce message complexity from O(n²) to O(n) by building a spanning tree where only tree edges use eager push. Non-tree edges use lazy push (IHave/IWant) as backup for tree failures.

## Two Protocols, One State Machine

Iroh-gossip combines two academic papers:

| Protocol | Paper | Role |
|----------|-------|------|
| **HyParView** | [HyParView: Maintenance of Partial Topology Views](https://asc.di.fct.unl.pt/~jleitao/pdf/dsn07-leitao.pdf) | Peer membership: who are my neighbors? |
| **PlumTree** | [Epidemic Broadcast Trees (PlumTree)](https://asc.di.fct.unl.pt/~jleitao/pdf/srds07-leitao.pdf) | Message dissemination: how do I reach everyone? |

HyParView manages the **partial view** — each peer maintains an active view (direct connections) and passive view (backup connections). PlumTree uses this topology to build an **epidemic broadcast tree** that reaches every peer with minimal redundancy.

## Architecture at a Glance

```mermaid
flowchart TD
    subgraph "Application"
        App[GossipTopic.publish(bytes)]
    end

    subgraph "API (api.rs)"
        GossipApi[GossipApi]
        GossipTopic[GossipTopic]
        Events[Event stream]
    end

    subgraph "Networking (net.rs)"
        Actor[Actor: main event loop]
        ConnLoop[Connection loop per peer]
        Dialer[Dialer: outbound connections]
    end

    subgraph "Protocol State (proto/)"
        TopicState[Topic state: HyParView + PlumTree]
        HyParView[HyParView: membership]
        PlumTree[PlumTree: broadcast tree]
    end

    App --> GossipTopic
    GossipTopic --> GossipApi
    GossipApi --> Actor
    Actor --> TopicState
    TopicState --> HyParView
    TopicState --> PlumTree
    Actor --> ConnLoop
    Actor --> Dialer
    ConnLoop --> TopicState
    Dialer --> TopicState
```

## The IO-Less State Machine Design

The `proto/` module is a pure state machine with no I/O. It accepts `InEvent`s and produces `OutEvent`s:

```
InEvent → handle() → State mutation + OutEvents
                              ↓
                    net module processes OutEvents
                    (send messages, set timers, dial peers)
                              ↓
                    Results become new InEvents
```

Source: `iroh-gossip/src/proto/state.rs:1` — `State<PI, R>` processes events deterministically.

**Aha:** The IO-less design means the protocol can be tested deterministically — feed it events, check the output, no mocks or async needed. The simulation framework (`proto/sim.rs`) runs 1000-node networks in a single process using this exact property.

## Module Structure

| Module | Lines | Purpose |
|--------|-------|---------|
| `proto/` | — | IO-less protocol state machine |
| `proto/hyparview.rs` | 764 | HyParView membership protocol |
| `proto/plumtree.rs` | 910 | PlumTree epidemic broadcast tree |
| `proto/topic.rs` | 363 | Combined per-topic state |
| `proto/state.rs` | 381 | Top-level event dispatch |
| `proto/util.rs` | 532 | IndexSet, TimerMap, TimeBoundCache |
| `proto/sim.rs` | 1141 | Discrete event simulation framework |
| `net.rs` | 1977 | Iroh-based networking |
| `net/address_lookup.rs` | 175 | Gossip-specific address lookup |
| `net/util.rs` | 435 | Stream utilities, frame encoding, timers |
| `api.rs` | 535 | High-level API: GossipApi, GossipTopic |
| `metrics.rs` | 35 | Prometheus counters |

## Feature Flags

| Feature | Default | Purpose |
|---------|---------|---------|
| `net` | ✅ | Iroh-based networking |
| `metrics` | ✅ | Prometheus metrics |
| `rpc` | — | irpc-based RPC API |
| `rpc-tls-ring` | — | Ring crypto for RPC |
| `rpc-tls-aws-lc-rs` | — | AWS LC-RS for RPC |
| `test-utils` | — | Test utilities (ChaCha RNG) |
| `simulator` | — | CLI simulation binary |
| `examples` | — | Example binaries |

Source: `iroh-gossip/Cargo.toml:features`

## Quick Start

```rust
// iroh-gossip/examples/setup.rs
let endpoint = iroh::Endpoint::bind().await?;
let gossip = Gossip::builder()
    .spawn(endpoint.clone())?;
let router = iroh::protocol::Router::builder(endpoint)
    .accept(iroh_gossip::ALPN.to_vec(), gossip.clone())
    .spawn()
    .await?;
```

Source: `iroh-gossip/examples/setup.rs:1-21`

## Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `iroh` | =1.0.0-rc.1 | Networking (via `net` feature) |
| `iroh-base` | =1.0.0-rc.1 | PublicKey, EndpointId types |
| `irpc` | 0.16.0 | RPC system (via `rpc` feature) |
| `postcard` | 1 | Serialization (no_std compatible) |
| `blake3` | 1.8 | Topic ID hashing |
| `ed25519-dalek` | =3.0.0-pre.7 | Message signing |
| `rand` | 0.10.1 | Random peer selection |
| `bytes` | 1.7 | Message payloads |
| `iroh-metrics` | =1.0.0-rc.0 | Metrics collection |

Source: `iroh-gossip/Cargo.toml:dependencies`

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Protocol layers and state machine design
- [HyParView](../markdown/02-hyparview.md) — Swarm membership protocol
- [PlumTree](../markdown/03-plumtree.md) — Epidemic broadcast tree optimization
