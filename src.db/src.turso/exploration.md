---
title: "Turso/libSQL: Complete Exploration"
subtitle: "Embedded replicas and serverless SQLite"
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.turso
repository: https://github.com/tursodatabase/libsql
explored_at: 2026-03-27
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

## Table of Contents

1. **[Zero to DB Engineer](00-zero-to-db-engineer.md)** - SQLite fundamentals
2. **[Storage Engine](01-storage-engine-deep-dive.md)** - WAL, B-tree
3. **[Query Execution](02-query-execution-deep-dive.md)** - SQLite VM
4. **[Replication](03-consensus-replication-deep-dive.md)** - Embedded replicas
5. **[Rust Revision](rust-revision.md)** - Translation guide
6. **[Production](production-grade.md)** - Deployment
7. **[Valtron Integration](04-valtron-integration.md)** - Lambda

---

## Architecture

```
┌─────────────┐    ┌─────────────┐
│   Client    │ -> │   Turso     │
│   (SDK)     │ <- │   Server    │
└─────────────┘    └─────────────┘
                          │
                          │ WAL sync
                          ▼
                   ┌─────────────┐
                   │  Embedded   │
                   │  Replica    │
                   │  (local)    │
                   └─────────────┘
```

---

## Running libSQL

```bash
# Server mode
sqld --enable-http-sync

# Client (Rust)
use libsql::Database;
let db = Database::open("file.db").await?;
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial exploration created |
