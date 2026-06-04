---
title: iroh-docs Documentation — Index
---

# iroh-docs Documentation

> Multi-dimensional key-value documents with efficient P2P synchronization

iroh-docs provides replicas with (namespace, author, key) entries, dual-signed content-addressed values, and range-based set reconciliation over iroh QUIC connections.

## Foundation

- [Overview](00-overview.html) — What iroh-docs is, data model, sync algorithm
- [Architecture](01-architecture.html) — Layer diagram, module map, dependency graph

## Core Protocol

- [Replica](02-replica.html) — The data model: SignedEntry, Record, RecordIdentifier
- [Ranger](03-ranger.html) — Range-based set reconciliation algorithm
- [Sync Actor](04-sync-actor.html) — Thread actor + tokio live actor coordination

## Network and Storage

- [Network](05-network.html) — Alice/Bob sync protocol over QUIC streams
- [Storage](06-storage.html) — redb v2 store, tables, queries, migrations
- [Engine](07-engine.html) — Live sync coordination, gossip integration

## Keys and Cross-Cutting

- [Keys](08-keys.html) — Author and Namespace cryptographic keys
- [Data Flow](09-data-flow.html) — End-to-end sync sequences
- [Cross-Cutting](10-cross-cutting.html) — RPC, CLI, tickets, metrics, download policies

---

Generated from source code. Every claim traces back to implementation.
