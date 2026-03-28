---
title: "TigerBeetle: Complete Exploration"
subtitle: "Financial accounting database"
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.tigerbeetle
repository: https://github.com/tigerbeetle/tigerbeetle
explored_at: 2026-03-27
---

# TigerBeetle: Complete Exploration

## Overview

**TigerBeetle** is a distributed financial accounting database:
- **ACID compliance** - Financial-grade
- **Double-entry** - Built-in accounting
- **High performance** - 100k+ TPS
- **Deterministic** - Reproducible execution

### Key Characteristics

| Aspect | TigerBeetle |
|--------|-------------|
| **Core** | Financial ledger |
| **Consensus** | Viewstamped Replication |
| **License** | Apache 2.0 |
| **Language** | Zig |

---

## Table of Contents

1. **[Zero to DB Engineer](00-zero-to-db-engineer.md)** - Accounting DB fundamentals
2. **[Storage Engine](01-storage-engine-deep-dive.md)** - Ledger storage
3. **[Consensus](03-consensus-replication-deep-dive.md)** - Viewstamped Replication
4. **[Rust Revision](rust-revision.md)** - Translation guide
5. **[Production](production-grade.md)** - Financial deployment

---

## Architecture

```
TigerBeetle Cluster:
┌─────────────┐
│   Leader    │ <- All writes go here
├─────────────┤
│ Follower 1  │ <- Synchronous replication
│ Follower 2  │
│ Follower 3  │
└─────────────┘
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial exploration created |
