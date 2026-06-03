---
title: iroh-gossip Documentation — Index
---

# iroh-gossip Documentation

> Gossip messages over broadcast trees

Iroh-gossip implements epidemic broadcast trees for P2P topic-based publish-subscribe, based on the HyParView and PlumTree papers.

## Foundation

- [Overview](00-overview.html) — What iroh-gossip is, why epidemic broadcast trees
- [Architecture](01-architecture.html) — Protocol layers, state machine design, module map

## Protocol Deep Dives

- [HyParView](02-hyparview.md) — Swarm membership protocol (active/passive views)
- [PlumTree](03-plumtree.md) — Epidemic broadcast tree optimization (lazy/eager push)
- [Topic State](04-topic-state.md) — Combining HyParView + PlumTree per topic

## Networking and API

- [Networking](05-networking.md) — Connection loops, dialer, topic subscriber loop
- [API](06-api.md) — GossipApi, GossipTopic, events, commands, RPC
- [Simulation](07-simulation.md) — Discrete event simulation framework

## Cross-Cutting

- [Data Flow](09-data-flow.html) — End-to-end gossip propagation sequences
- [Cross-Cutting](10-cross-cutting.html) — Metrics, serialization, addressing, WASM

---

Generated from source code. Every claim traces back to implementation.
