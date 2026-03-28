---
title: "Neo4j/Neodatabase: Complete Exploration"
subtitle: "Graph database fundamentals"
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.db/src.neodatabase
explored_at: 2026-03-27
---

# Neo4j/Neodatabase: Complete Exploration

## Overview

**Neo4j** and related graph databases:
- **Native graph storage** - Nodes, relationships
- **Cypher query language** - Pattern matching
- **Graph algorithms** - Traversal, pathfinding

---

## Table of Contents

1. **[Zero to DB Engineer](00-zero-to-db-engineer.md)** - Graph DB fundamentals
2. **[Storage Engine](01-storage-engine-deep-dive.md)** - Graph storage
3. **[Query Execution](02-query-execution-deep-dive.md)** - Cypher, traversal
4. **[Rust Revision](rust-revision.md)** - Translation guide

---

## Graph Model

```
Graph Structure:
(Alice) -[FRIEND]-> (Bob)
   |                    |
[WORKS_AT]         [LIVES_IN]
   |                    |
   v                    v
(Company) -[LOCATED_IN]-> (NYC)
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial exploration created |
