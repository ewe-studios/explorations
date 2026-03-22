# Iroh Core Modules - Exploration Index

This directory contains comprehensive deep-dive explorations of the core iroh ecosystem modules. These documents cover the architecture, APIs, protocols, and production usage patterns of each module.

---

## Critical Core Modules

### 1. [iroh-blobs](./iroh-blobs-deep-dive.md) - Blob Storage and Syncing
**Version:** 0.91.0 | **Status:** CRITICAL

The foundational content-addressed blob storage system using BLAKE3 verified streaming.

**Key Topics:**
- Content-addressed storage with BLAKE3 Merkle trees
- Efficient range requests with chunk-level granularity
- File system and memory store backends
- GetRequest/ChunkRangesSeq for precise data retrieval
- Integration with iroh-blobs for content storage

**Key Files:**
- `src/lib.rs` - Main module entry point
- `src/api.rs` - User-facing API
- `src/protocol.rs` - Wire protocol definition
- `src/store/fs.rs` - File system store implementation

---

### 2. [iroh-sync](./iroh-sync-deep-dive.md) - CRDT Sync Protocol
**Version:** 0.4.1 | **Status:** CRITICAL

Range-based set reconciliation protocol for eventual consistency.

**Key Topics:**
- Range-based set reconciliation algorithm
- Dual-signature scheme (namespace + author)
- Replica model with content-addressed entries
- Store trait with redb implementations
- Multi-version support for conflict resolution

**Key Files:**
- `src/lib.rs` - Main entry point
- `src/sync.rs` - Core sync types and Replica
- `src/ranger.rs` - Range-based reconciliation

---

### 3. [iroh-gossip](./iroh-gossip-deep-dive.md) - Gossip Protocol
**Version:** 0.90.0 | **Status:** CRITICAL

HyParView membership + PlumTree gossip broadcasting.

**Key Topics:**
- HyParView swarm membership protocol
- PlumTree eager/lazy push strategy
- Topic-based message scoping
- IO-less state machine design
- Built-in simulation framework

**Key Files:**
- `src/lib.rs` - Module entry
- `src/proto.rs` - Protocol definitions
- `src/proto/state.rs` - Protocol state machine
- `src/proto/hyparview.rs` - Membership protocol
- `src/proto/plumtree.rs` - Gossip protocol
- `src/net.rs` - Network integration
- `DESIGN.md` - Architecture documentation

---

### 4. [iroh-docs](./iroh-docs-deep-dive.md) - Document Database
**Version:** 0.35.0 | **Status:** CRITICAL

Distributed document database built on iroh-sync and iroh-blobs.

**Key Topics:**
- Engine-based live sync coordination
- Default author management
- Content protection from GC
- Gossip integration for real-time updates
- Store abstractions with migrations

**Key Files:**
- `src/lib.rs` - Module entry
- `src/engine.rs` - Sync engine
- `src/engine/live.rs` - Live sync actor
- `src/store/fs.rs` - File system store
- `src/protocol.rs` - Network protocol

---

## Supporting Modules

### 5. [iroh-car](./iroh-car-deep-dive.md) - Content-Addressed Archives
**Status:** Supporting

IPLD CAR format implementation for data import/export.

**Key Topics:**
- CARv1 format support
- Async streaming reader/writer
- IPFS interoperability
- Length-prefixed block encoding

---

### 6. [iroh-metrics](./iroh-metrics-deep-dive.md) - Metrics Collection
**Version:** 0.35 | **Status:** Supporting

Prometheus-compatible metrics with feature-gated overhead.

**Key Topics:**
- Counter and Gauge metric types
- Derive macros for metric groups
- Zero-overhead when disabled
- Prometheus output format

---

### 7. [iroh-io](./iroh-io-deep-dive.md) - I/O Utilities
**Version:** 0.6.1 | **Status:** Supporting

Async I/O traits for files, memory, and HTTP resources.

**Key Topics:**
- AsyncSliceReader/Writer traits
- Position-explicit I/O model
- Non-Send futures for local executors
- HTTP Range request support

---

### 8. [iroh-ffi](./iroh-ffi-deep-dive.md) - FFI Bindings (UniFFI)
**Version:** 0.35.0 | **Status:** Language Bindings

UniFFI-based bindings for Kotlin, Swift, Python, Ruby.

**Key Topics:**
- Multi-language binding generation
- Type-safe FFI with UniFFI
- Async support in foreign languages
- Callback interfaces

---

### 9. [iroh-c-ffi](./iroh-c-ffi-deep-dive.md) - C FFI Bindings
**Version:** 0.90.0 | **Status:** Language Bindings

safer-ffi based C-compatible bindings.

**Key Topics:**
- Automatic header generation
- C ABI compatibility
- Explicit memory management
- Error code pattern

---

### 10. [iroh-dns-server](./iroh-dns-server-deep-dive.md) - DNS Server
**Version:** 0.1.0 | **Status:** Infrastructure

PKARR relay and DNS server for node discovery.

**Key Topics:**
- PKARR protocol implementation
- DNS server with hickory
- Rate limiting with governor
- ACME TLS certificate management
- redb persistence

---

### 11. [iroh-relay](./iroh-relay-deep-dive.md) - Relay Server
**Version:** 0.34.1 | **Status:** Infrastructure

DERP-based relay for hole punching and fallback.

**Key Topics:**
- DERP wire protocol
- HTTP/HTTPS/QUIC transport
- STUN server support
- Connection forwarding
- Network connectivity reporting

---

## Module Integration Diagram

```
                                    ┌─────────────────┐
                                    │   Application   │
                                    └────────┬────────┘
                                             │
              ┌──────────────────────────────┼──────────────────────────────┐
              │                              │                              │
              ▼                              ▼                              ▼
    ┌─────────────────┐            ┌─────────────────┐            ┌─────────────────┐
    │   iroh-docs     │            │  iroh-blobs     │            │  iroh-gossip    │
    │  (Documents)    │◄──────────►│   (Storage)     │            │   (Broadcast)   │
    └────────┬────────┘            └────────┬────────┘            └────────┬────────┘
             │                              │                              │
             │         ┌────────────────────┼────────────────────┐         │
             │         │                    │                    │         │
             ▼         ▼                    ▼                    ▼         ▼
    ┌─────────────────────────────────────────────────────────────────────────┐
    │                           iroh-sync                                    │
    │                    (CRDT Sync Protocol)                                │
    └─────────────────────────────────────────────────────────────────────────┘
             │                              │
             ▼                              ▼
    ┌─────────────────┐            ┌─────────────────┐
    │ iroh-dns-server │            │  iroh-relay     │
    │   (Discovery)   │            │  (Hole Punch)   │
    └─────────────────┘            └─────────────────┘
             │                              │
             └──────────────┬───────────────┘
                            │
                            ▼
                   ┌─────────────────┐
                   │  iroh endpoint  │
                   │  (Network Core) │
                   └─────────────────┘
                            │
              ┌─────────────┼─────────────┐
              │             │             │
              ▼             ▼             ▼
    ┌─────────────────┐ ┌─────────┐ ┌─────────────┐
    │   iroh-io       │ │metrics  │ │  FFI layers │
    │   (I/O utils)   │ │         │ │(UniFFI/C)   │
    └─────────────────┘ └─────────┘ └─────────────┘
```

---

## Cross-Module Dependencies

| Module | Depends On | Used By |
|--------|-----------|---------|
| iroh-blobs | iroh-io, iroh-metrics | iroh-docs |
| iroh-sync | - | iroh-docs |
| iroh-gossip | iroh-metrics | iroh-docs |
| iroh-docs | iroh-blobs, iroh-sync, iroh-gossip | - |
| iroh-car | - | (standalone) |
| iroh-metrics | - | All modules |
| iroh-io | - | iroh-blobs |
| iroh-ffi | iroh-docs, iroh-blobs | External (Kotlin/Swift/Python) |
| iroh-c-ffi | iroh | External (C/C++) |
| iroh-dns-server | - | iroh discovery |
| iroh-relay | - | iroh networking |

---

## Production Readiness Summary

| Module | Production Ready | Notes |
|--------|-----------------|-------|
| iroh-blobs | Yes | Mature, widely used |
| iroh-sync | Yes | Core CRDT implementation |
| iroh-gossip | Yes | Production-tested |
| iroh-docs | Yes | Full engine implementation |
| iroh-car | Yes | Standard format support |
| iroh-metrics | Yes | Feature-gated overhead |
| iroh-io | Yes | Well-tested utilities |
| iroh-ffi | Yes | Active development |
| iroh-c-ffi | Yes | Active development |
| iroh-dns-server | Yes | Deployed in production |
| iroh-relay | Yes | Multiple deployments |

---

## Related Documents in Parent Directory

- `nat-traversal.md` - NAT traversal strategies
- `p2p-ground-up.md` - P2P networking from first principles
- `iroh-ids.md` - Identifier systems in iroh
- `cryptography-keys.md` - Cryptographic primitives
- `QUIC_RPC_DEEP_DIVE.md` - RPC over QUIC
- `BAO_TREE_DEEP_DIVE.md` - BLAKE3 Merkle trees
- `WILLOW_PROTOCOL_DEEP_DIVE.md` - Willow protocol foundation

---

## Exploration Date

**Generated:** 2026-03-22
**Based on:** Current main branch versions
