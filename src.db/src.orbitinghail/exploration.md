---
title: "OrbitingHail: Complete Exploration"
subtitle: "SQLSync and distributed SQL"
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.orbitinghail
repository: https://github.com/orbitinghail
explored_at: 2026-03-27
---

# OrbitingHail: Complete Exploration

## Overview

**OrbitingHail** projects focus on distributed SQL:
- **SQLSync** - CRDT-based SQL sync
- **Graft** - Distributed database
- **Splinter** - Database federation

---

## Table of Contents

1. **[Zero to DB Engineer](00-zero-to-db-engineer.md)** - Distributed SQL
2. **[Storage Engine](01-storage-engine-deep-dive.md)** - SQLSync internals
3. **[Rust Revision](rust-revision.md)** - Translation guide
4. **[Production](production-grade.md)** - Deployment
5. **[Valtron Integration](04-valtron-integration.md)** - Valtron patterns

---

## SQLSync Protocol

```
SQLSync uses CRDTs for conflict-free replication:
- Last-write-wins for scalars
- Add-wins for sets
- Custom CRDTs for complex types
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial exploration created |
| 2026-03-28 | Added 00-zero-to-db-engineer.md |
| 2026-03-28 | Added 01-storage-engine-deep-dive.md |
| 2026-03-28 | Added rust-revision.md |
| 2026-03-28 | Added production-grade.md |
| 2026-03-28 | Added 04-valtron-integration.md |
