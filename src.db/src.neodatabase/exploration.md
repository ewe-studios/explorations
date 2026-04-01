---
title: "Neodatabase: Complete Exploration"
subtitle: "Graph database fundamentals with Neo4j"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
repository: https://github.com/neo4j/neo4j
explored_at: 2026-03-28
status: COMPLETE
---

# Neodatabase: Complete Exploration

## Overview

**Neo4j** and graph databases for relationship-heavy data:
- **Native graph storage** - Nodes, relationships, properties
- **Cypher query language** - Declarative pattern matching
- **Graph algorithms** - Traversal, pathfinding, centrality
- **ACID transactions** - Full database reliability

### Key Characteristics

| Aspect | Neo4j |
|--------|-------|
| **Data Model** | Property graph (nodes + relationships) |
| **Query Language** | Cypher |
| **Storage** | Native graph store |
| **License** | GPL / Commercial |
| **Use Cases** | Social networks, fraud detection, knowledge graphs |

### Documents

| Document | Description |
|----------|-------------|
| [exploration.md](./exploration.md) | Overview |
| [00-zero-to-graph-engineer.md](./00-zero-to-graph-engineer.md) | Graph database fundamentals |
| [01-storage-engine-deep-dive.md](./01-storage-engine-deep-dive.md) | Graph storage internals |
| [02-query-execution-deep-dive.md](./02-query-execution-deep-dive.md) | Cypher execution |
| [03-cloning-copy-on-write-deep-dive.md](./03-cloning-copy-on-write-deep-dive.md) | Database cloning with CoW |
| [05-neon-branching-deep-dive.md](./05-neon-branching-deep-dive.md) | Neon-style instant branching |
| [rust-revision.md](./rust-revision.md) | Rust graph database options |
| [production-grade.md](./production-grade.md) | Deployment patterns |

---

## Graph Model

```
Property Graph Structure:

Nodes (entities):
- (:Person {name: "Alice", age: 30})
- (:Company {name: "Acme", founded: 1990})

Relationships (directed, typed):
- (:Person)-[:WORKS_AT]->(:Company)
- (:Person)-[:FRIENDS_WITH]->(:Person)
- (:Person)-[:LIVES_IN]->(:City)

Complete Example:
┌─────────────────────────────────────────────────────────┐
│                                                          │
│  (Alice) -[:FRIENDS_WITH]-> (Bob)                        │
│    |                       |                              │
│  [:WORKS_AT]           [:WORKS_AT]                       │
│    |                       |                              │
│    v                       v                              │
│  (Acme) -[:LOCATED_IN]-> (NYC)                            │
│                                                          │
│  Cypher Query:                                            │
│  MATCH (p:Person)-[:WORKS_AT]->(c:Company)               │
│  WHERE c.name = "Acme"                                   │
│  RETURN p.name                                            │
│                                                          │
│  Result: ["Alice", "Bob"]                                 │
└───────────────────────────────────────────────────────────┘
```

---

## Quick Start

```cypher
// Create nodes
CREATE (alice:Person {name: "Alice", age: 30})
CREATE (bob:Person {name: "Bob", age: 25})
CREATE (acme:Company {name: "Acme", founded: 1990})

// Create relationships
CREATE (alice)-[:FRIENDS_WITH]->(bob)
CREATE (alice)-[:WORKS_AT]->(acme)
CREATE (bob)-[:WORKS_AT]->(acme)

// Query: Find friends of friends
MATCH (p:Person {name: "Alice"})-[:FRIENDS_WITH*2]->(fof)
RETURN fof.name

// Query: Find company employees
MATCH (p:Person)-[:WORKS_AT]->(c:Company {name: "Acme"})
RETURN p.name, p.age
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-28 | Full exploration completed |
| 2026-03-28 | Added 00-zero-to-graph-engineer.md |
| 2026-03-28 | Added 01-storage-engine-deep-dive.md |
| 2026-03-28 | Added 02-query-execution-deep-dive.md |
| 2026-03-28 | Added 03-cloning-copy-on-write-deep-dive.md |
| 2026-03-28 | Added 05-neon-branching-deep-dive.md |
| 2026-03-28 | Added rust-revision.md |
| 2026-03-28 | Added production-grade.md |
