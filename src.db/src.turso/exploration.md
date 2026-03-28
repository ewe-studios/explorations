---
title: "Turso/libSQL: Complete Exploration"
subtitle: "Embedded replicas and serverless SQLite"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.turso
repository: https://github.com/tursodatabase/libsql
explored_at: 2026-03-27
status: COMPLETE
---

# Turso/libSQL: Complete Exploration

## Overview

**Turso/libSQL** is a fork of SQLite that adds:
- **Embedded replicas** - Local copies of remote database
- **Serverless** - HTTP-based access
- **Sync protocol** - Efficient replication
- **Wasm support** - Run in browsers

### Key Characteristics

| Aspect | Turso/libSQL |
|--------|--------------|
| **Core** | SQLite fork with extensions |
| **Replication** | Embedded replicas, WAL sync |
| **License** | MIT/Apache |
| **Language** | Rust, C, Go, TS bindings |

---

## Documents

### Core Documents

| Document | Description | Size |
|----------|-------------|------|
| [exploration.md](./exploration.md) | Architecture overview | 100 lines |
| [00-zero-to-db-engineer.md](./00-zero-to-db-engineer.md) | SQLite fundamentals, WAL, ACID, consistency | ~800 lines |
| [01-storage-engine-deep-dive.md](./01-storage-engine-deep-dive.md) | File format, B-tree, WAL structure | ~700 lines |
| [02-query-execution-deep-dive.md](./02-query-execution-deep-dive.md) | VDBE bytecode, query planning | ~900 lines |
| [03-consensus-replication-deep-dive.md](./03-consensus-replication-deep-dive.md) | Sync protocol, consistency models | ~500 lines |
| [rust-revision.md](./rust-revision.md) | Valtron-based Rust translation | ~500 lines |
| [production-grade.md](./production-grade.md) | Deployment, monitoring, backup | ~600 lines |
| [04-valtron-integration.md](./04-valtron-integration.md) | Lambda deployment without async | ~500 lines |

### Key Topics Covered

1. **SQLite Fundamentals**
   - B-Tree data structure and page layout
   - ACID transactions explained
   - WAL vs rollback journal
   - Embedded replica architecture

2. **Storage Engine**
   - 100-byte database header
   - Page structure (leaf, interior)
   - WAL frame format (24-byte header + page data)
   - Checkpoint process
   - Memory-mapped I/O

3. **Query Execution**
   - SQL tokenizer and parser
   - AST generation
   - VDBE bytecode compilation
   - Query planning and optimization
   - Prepared statements

4. **Replication**
   - Sync protocol (HTTP/gRPC)
   - Consistency models (strong, eventual, read-after-write)
   - Conflict resolution strategies
   - Failure scenarios and recovery

5. **Valtron Integration**
   - Task iterator pattern (no async)
   - Lambda handler implementation
   - HTTP effect handling
   - Cold start optimization

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    libSQL Architecture                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐      │
│  │   Client    │ -> │   Primary   │ -> │   Embedded  │      │
│  │   (SDK)     │ <- │   Server    │ <- │   Replica   │      │
│  └─────────────┘    └─────────────┘    └─────────────┘      │
│                            │                                  │
│                            │ WAL sync via HTTP               │
│                            ▼                                  │
│                   ┌─────────────────┐                        │
│                   │   WAL Frames    │                        │
│                   │   (propagated   │                        │
│                   │    to replicas) │                        │
│                   └─────────────────┘                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Quick Start

```bash
# Server mode
sqld --enable-http-sync

# Client (Rust - original libsql)
use libsql::Database;
let db = Database::open("file.db").await?;

# Client (Valtron - no async)
let db = Database::open("file.db")?;
let results = db.execute("SELECT * FROM users", &[])?;
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial exploration created |
| 2026-03-28 | Added 00-zero-to-db-engineer.md (SQLite fundamentals) |
| 2026-03-28 | Added 01-storage-engine-deep-dive.md (WAL, B-tree) |
| 2026-03-28 | Added 02-query-execution-deep-dive.md (VDBE bytecode) |
| 2026-03-28 | Added 03-consensus-replication-deep-dive.md (Sync protocol) |
| 2026-03-28 | Added rust-revision.md (Valtron translation) |
| 2026-03-28 | Added production-grade.md (Deployment) |
| 2026-03-28 | Added 04-valtron-integration.md (Lambda) |
