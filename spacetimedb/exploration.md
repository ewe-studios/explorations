---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/SpacetimeDB
repository: https://github.com/clockworklabs/SpacetimeDB
explored_at: 2026-03-22
language: Rust, C#, TypeScript, C++
---

# Project Exploration: SpacetimeDB

## Overview

SpacetimeDB is a **relational database that is also a server**. It represents a fundamentally different approach to application architecture where you upload application logic directly into the database, and clients connect to it without any server in between.

### Key Value Proposition

- **No separate application server** - Your backend logic runs inside the database
- **Real-time synchronization** - State changes are automatically pushed to subscribed clients
- **Multi-language support** - Write modules in Rust, C#, TypeScript, or C++
- **In-memory performance** - All data is held in memory with commit-log durability
- **WebAssembly isolation** - User code runs in sandboxed WASM modules

### Architecture Summary

```
┌─────────────┐     ┌─────────────────────┐     ┌─────────────┐
│   Client    │────▶│   SpacetimeDB Host  │◀────│   Client    │
│  (React/TS) │     │  ┌───────────────┐  │     │  (Unity/C#) │
└─────────────┘     │  │   Your Module │  │     └─────────────┘
                    │  │  (WASM/JS)    │  │
                    │  │  - Tables     │  │
                    │  │  - Reducers   │  │
                    │  │  - Views      │  │
                    │  └───────────────┘  │
                    │  ┌───────────────┐  │
                    │  │  Commit Log   │  │
                    │  │  (Durability) │  │
                    │  └───────────────┘  │
                    └─────────────────────┘
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/SpacetimeDB/
├── crates/                          # Core Rust workspace
│   ├── auth/                        # Authentication and identity management
│   ├── bindings/                    # Rust SDK for module development
│   ├── bindings-macro/              # Procedural macros for Rust SDK
│   ├── bindings-sys/                # FFI bindings for WASM ABI
│   ├── bindings-csharp/             # C# SDK
│   ├── bindings-typescript/         # TypeScript SDK
│   ├── bindings-cpp/                # C++ SDK (Unreal Engine)
│   ├── cli/                         # CLI tool for deployment
│   ├── client-api/                  # Client connection API
│   ├── client-api-messages/         # Protocol message definitions
│   ├── codegen/                     # Code generation for SDKs
│   ├── commitlog/                   # Write-ahead log implementation
│   ├── core/                        # Core database engine
│   ├── data-structures/             # Custom data structures
│   ├── datastore/                   # Datastore abstraction layer
│   ├── durability/                  # Durability guarantees
│   ├── execution/                   # Query execution engine
│   ├── expr/                        # Expression evaluation
│   ├── lib/                         # Common library types
│   ├── memory-usage/                # Memory tracking
│   ├── metrics/                     # Prometheus metrics
│   ├── paths/                       # File path utilities
│   ├── pg/                          # PostgreSQL compatibility layer
│   ├── physical-plan/               # Physical query planning
│   ├── primitives/                  # Core primitives (TableId, ColId, etc.)
│   ├── query/                       # Query planner
│   ├── sats/                        # Spacetime Algebraic Type System
│   ├── schema/                      # Schema management
│   ├── snapshot/                    # Snapshot isolation
│   ├── sql-parser/                  # SQL parsing
│   ├── standalone/                  # Single-node deployment
│   ├── subscription/                # Subscription query engine
│   ├── table/                       # Table storage engine
│   └── update/                      # Auto-update mechanism
├── docs/                            # Documentation
├── modules/                         # Example modules
├── sdks/                            # Generated SDK outputs
└── demo/                            # Demo applications (Blackholio MMO)
```

## Core Concepts

### 1. Host
A SpacetimeDB **host** is a server that hosts databases. Many databases can run on a single host.

### 2. Database (Module)
A SpacetimeDB **database** is an application that runs on a host. It consists of:
- **Tables** - Data storage
- **Reducers** - Functions that modify state
- **Views** - Read-only computed queries
- **Procedures** - Functions with external access

### 3. Tables
Tables are in-memory relational tables with:
- Automatic indexing on primary keys
- Set semantics (no duplicate rows by default)
- B-tree and hash index support
- Multi-column indexes

### 4. Reducers
Reducers are transactional functions that:
- Run atomically (all-or-nothing)
- Can read/write tables
- Are called by clients via RPC
- Trigger automatic client updates

### 5. Identity & Authentication
- Uses OpenID Connect / JWT-based identities
- 256-bit Blake3 hash of issuer+subject
- Attached to every reducer call
- Used for authorization

## Storage Architecture

### In-Memory with Commit Log

SpacetimeDB keeps **all data in memory** for maximum performance, with durability provided by a commit log:

```
┌─────────────────────────────────────────────────────────┐
│                    In-Memory State                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   Table A    │  │   Table B    │  │   Table C    │   │
│  │  (B-Tree +   │  │  (Hash +     │  │  (B-Tree +   │   │
│  │   Hash)      │  │   B-Tree)    │  │   Hash)      │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────┘
                          │
                          │ Write
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    Commit Log (WAL)                      │
│  Segment 1  │  Segment 2  │  Segment 3  │  ...          │
│  (zstd)     │  (zstd)     │  (zstd)     │               │
└─────────────────────────────────────────────────────────┘
```

### Key Storage Components

1. **Pages** - 4KB fixed-size pages store row data
2. **B-Tree Indexes** - For range queries and sorted access
3. **Hash Indexes** - For O(1) point lookups
4. **Blob Store** - For variable-length data (strings, blobs)
5. **Commit Log** - Write-ahead log with zstd compression

### Row Storage Format

Rows are stored in a **BFLATN** (Binary Flat) format:
- Fixed-length portion stored inline in pages
- Variable-length data stored in blob store with references
- Row pointers are 64-bit (PageIndex + PageOffset)

```
Row Structure:
┌────────────────────────────────────────┐
│  Fixed Section (inline in page)        │
│  ┌──────┬──────┬──────┬────────────┐   │
│  │  id  │ name │ age  │ var_offset │   │
│  │ u64  │blob# │ u32  │   u16      │   │
│  └──────┴──────┴──────┴────────────┘   │
└────────────────────────────────────────┘
          │
          │ points to
          ▼
┌────────────────────────────────────────┐
│  Variable Section (blob store)         │
│  "John Doe" (variable length string)   │
└────────────────────────────────────────┘
```

## Client Synchronization

### Subscription System

Clients subscribe to queries, and SpacetimeDB automatically pushes updates:

```typescript
// Client subscribes to a query
ctx.subscriptionBuilder()
  .subscribe('SELECT * FROM messages WHERE channel_id = ?', [channelId])
  .onUpdate((row) => { /* handle update */ })
  .onError((err) => { /* handle error */ });
```

### Update Propagation

```
Reducer Call → Transaction → Index Update →
  → Subscription Match → Client Push
```

## Performance Characteristics

### Speed Optimizations

1. **Zero-copy reads** - Data accessed directly from pages
2. **Pointer-based navigation** - RowPointer avoids indirection
3. **Deduplication** - PointerMap prevents duplicate rows
4. **Column indexes** - Fast lookups without full table scan
5. **In-memory** - No disk I/O for reads

### Memory Layout

- Page size: 4KB
- Page header: ~32 bytes
- Fixed rows stored sequentially
- Variable data in separate blob store
- Freelist for space reuse

## Related Projects in Source

- **gungraun** - Game server using SpacetimeDB
- **omnipaxos** - Consensus protocol integration
- **Blackholio** - Reference MMO implementation
- **spacetimedb-cookbook** - Recipe examples

## Documentation References

- Main docs: `docs/docs/`
- Quickstarts available for React, Next.js, Vue, Svelte, Angular, TanStack, Remix
- Language support: Rust, C#, TypeScript, C++
