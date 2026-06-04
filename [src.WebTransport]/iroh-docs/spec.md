---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/iroh-docs
repository: git@github.com:n0-computer/iroh-docs
revised_at: 2026-06-03T00:00:00Z
workspace: iroh-docs
---

# iroh-docs — Spec File

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/iroh-docs`
- **Language:** Rust
- **Edition:** 2021
- **Rust Version:** 1.81
- **License:** MIT OR Apache-2.0
- **Version:** 0.35.0
- **Remote:** `git@github.com:n0-computer/iroh-docs`

## What the Project Is

iroh-docs provides multi-dimensional key-value documents with efficient synchronization over iroh P2P connections. Replicas contain entries identified by (namespace, author, key), with content-addressed values stored via iroh-blobs. Sync uses range-based set reconciliation based on Aljoscha Meyer's paper.

## Documentation Goal

After reading, a reader should understand:
1. The replica data model: Namespace → Author → Key → Entry → Hash
2. Dual signing: namespace key (write capability) + author key (authorship)
3. Range-based set reconciliation algorithm (ranger)
4. The sync actor model (thread + tokio live actor)
5. The network protocol (Alice/Bob sync over QUIC streams)
6. The storage layer (redb v2, tables, migrations)
7. The engine (live sync coordination, gossip integration)
8. Download policies for content-addressed blobs
9. The CLI and RPC interfaces

## Documentation Structure

```
iroh-docs/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-replica.md
│   ├── 03-ranger.md
│   ├── 04-sync-actor.md
│   ├── 05-network.md
│   ├── 06-storage.md
│   ├── 07-engine.md
│   ├── 08-keys.md
│   ├── 09-data-flow.md
│   └── 10-cross-cutting.md
├── html/
└── build.py
```

## Tasks — All DONE

## Build System

```bash
cd iroh-docs && python3 build.py
```

## Quality Requirements

All 10 iron rules.

## Expected Outcome

After reading, a developer can use iroh-docs for multi-peer document sync, understand the set reconciliation algorithm, and debug sync issues.

## Resume Point

Write documents in order.
