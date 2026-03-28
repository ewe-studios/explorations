# Database Explorations Index

This directory contains comprehensive explorations for database projects from `/home/darkvoid/Boxxed/@formulas/src.rust/src.db/`.

## Completed Explorations

### Full Explorations (8 files each)

| Database | Description | Location |
|----------|-------------|----------|
| **SpacetimeDB** | In-memory database with realtime sync | [src.db/src.spacetime/](src.spacetime/) |
| **SurrealDB** | Multi-model: document + graph + relational | [src.db/src.surrealdb/](src.surrealdb/) |

### Partial Explorations (1 file each)

| Database | Description | Location |
|----------|-------------|----------|
| **Turso/libSQL** | Embedded replicas, serverless SQLite | [src.db/src.turso/](src.turso/) |
| **Delta Lake** | ACID transactions for data lakes | [src.db/src.delta/](src.delta/) |
| **ArrowAndDBs** | Columnar analytics (Arrow, DuckDB, DataFusion) | [src.db/src.ArrowAndDBs/](src.ArrowAndDBs/) |
| **DragonflyDB** | High-performance Redis alternative | [src.db/src.dragonflydb/](src.dragonflydb/) |
| **Gimli-rs** | DWARF debugging parser | [src.db/src.gimli-rs/](src.gimli-rs/) |
| **GoatPlatform** | Real-time database (goatdb, sqlsync) | [src.db/src.goatplatform/](src.goatplatform/) |
| **OrbitingHail** | SQLSync, distributed SQL | [src.db/src.orbitinghail/](src.orbitinghail/) |
| **TigerBeetle** | Financial accounting database | [src.db/src.tigerbeetle/](src.tigerbeetle/) |
| **Neodatabase** | Neo4j graph database | [src.db/src.neodatabase/](src.neodatabase/) |

---

## File Structure Template

Each database exploration includes:

```
[database]/
├── exploration.md              # Architecture overview
├── 00-zero-to-db-engineer.md   # First principles explainer
├── 01-storage-engine-deep-dive.md   # Storage internals
├── 02-query-execution-deep-dive.md  # Query processing
├── 03-consensus-replication-deep-dive.md  # Distributed systems
├── rust-revision.md            # Rust translation guide
├── production-grade.md         # Production deployment
└── 04-valtron-integration.md   # Lambda deployment (no async/tokio)
```

---

## Key Topics Covered

### Database Fundamentals
- Storage engines (B-trees, LSM trees, in-memory)
- Query execution and optimization
- Index implementations
- Transaction processing

### Distributed Systems
- Consensus protocols (Raft, Paxos)
- Replication strategies
- Leader election
- Conflict resolution

### Rust Implementation
- valtron executor pattern (no async/await)
- TaskIterator for query execution
- Memory management
- Error handling

### Production Deployment
- Performance optimization
- Monitoring and observability
- Backup and recovery
- High availability

### Serverless Integration
- AWS Lambda deployment
- State persistence (S3, DynamoDB)
- Cold start optimization
- Connection pooling

---

## Source Repositories

| Database | Repository |
|----------|------------|
| SpacetimeDB | https://github.com/clockworklabs/SpacetimeDB |
| SurrealDB | https://github.com/surrealdb/surrealdb |
| Turso/libSQL | https://github.com/tursodatabase/libsql |
| Delta Lake | https://github.com/delta-io/delta |
| Apache Arrow | https://github.com/apache/arrow |
| DuckDB | https://github.com/duckdb/duckdb |
| DataFusion | https://github.com/apache/datafusion |
| DragonflyDB | https://github.com/dragonflydb/dragonfly |
| Gimli-rs | https://github.com/gimli-rs/gimli |
| TigerBeetle | https://github.com/tigerbeetle/tigerbeetle |

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial database explorations index created |
| 2026-03-27 | SpacetimeDB: Complete 8-file exploration |
| 2026-03-27 | SurrealDB: Complete 8-file exploration |
| 2026-03-27 | Partial explorations for 9 additional databases |

---

*This index is a living document. Additional database explorations will be added.*
