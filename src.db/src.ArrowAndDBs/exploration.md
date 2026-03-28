---
title: "ArrowAndDBs: Complete Exploration"
subtitle: "Columnar analytics databases"
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.ArrowAndDBs
explored_at: 2026-03-27
---

# ArrowAndDBs: Complete Exploration

## Overview

Collection of columnar analytics databases:
- **Apache Arrow** - In-memory columnar format
- **DuckDB** - Embedded OLAP
- **DataFusion** - Query engine
- **Polars** - DataFrame library
- **Iceberg** - Table format

### Components

| Project | Purpose |
|---------|---------|
| Arrow | Columnar memory format |
| DuckDB | Embedded analytical DB |
| DataFusion | SQL query engine |
| Polars | Rust/Python DataFrame |
| Iceberg | Table format for lakes |

---

## Table of Contents

1. **[Zero to DB Engineer](00-zero-to-db-engineer.md)** - OLAP fundamentals
2. **[Storage Engine](01-storage-engine-deep-dive.md)** - Columnar storage
3. **[Query Execution](02-query-execution-deep-dive.md)** - Vectorized execution
4. **[Rust Revision](rust-revision.md)** - Arrow/DataFusion in Rust
5. **[Production](production-grade.md)** - Deployment

---

## Arrow Columnar Format

```
Arrow Record Batch:
┌─────────────────────────────────┐
│ Schema                          │
│ - name: UTF8                    │
│ - age: INT64                    │
│ - score: FLOAT64                │
├─────────────────────────────────┤
│ Column: name                    │
│ [Alice, Bob, Charlie, ...]      │
├─────────────────────────────────┤
│ Column: age                     │
│ [30, 25, 35, ...]               │
├─────────────────────────────────┤
│ Column: score                   │
│ [95.5, 87.3, 92.1, ...]         │
└─────────────────────────────────┘
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial exploration created |
