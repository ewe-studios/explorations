---
title: "DragonflyDB: Complete Exploration"
subtitle: "Multi-threaded in-memory datastore"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.dragonflydb
repository: https://github.com/dragonflydb/dragonfly
explored_at: 2026-03-28
status: COMPLETE
---

# DragonflyDB: Complete Exploration

## Overview

**DragonflyDB** is a modern in-memory datastore designed for 2022+ hardware:
- **Multi-threaded** - Shared-nothing architecture, utilizes all CPU cores
- **Redis-compatible** - Drop-in replacement with 185+ commands
- **Memory efficient** - Dashtable and DenseSet, 30% more efficient than Redis
- **High throughput** - 25X more throughput than Redis (3.8M QPS vs 200K)
- **Forkless snapshot** - No memory spikes during persistence

### Key Characteristics

| Aspect | DragonflyDB |
|--------|-------------|
| **Core** | In-memory key-value store |
| **Protocol** | Redis/Memcached compatible |
| **License** | BSL (Business Source License) |
| **Language** | C++ |
| **Architecture** | Shared-nothing, multi-threaded |
| **Throughput** | 3.8M QPS (c6gn.16xlarge) |
| **Latency** | <1ms P99 at peak throughput |

---

## Documents

### Core Documents

| Document | Description | Size |
|----------|-------------|------|
| [exploration.md](./exploration.md) | Architecture overview | 100 lines |
| [00-zero-to-db-engineer.md](./00-zero-to-db-engineer.md) | In-memory fundamentals, shared-nothing architecture | ~800 lines |
| [01-storage-engine-deep-dive.md](./01-storage-engine-deep-dive.md) | Dashtable, DenseSet, memory efficiency | ~900 lines |
| [02-query-execution-deep-dive.md](./02-query-execution-deep-dive.md) | VLL transaction framework, command processing | ~900 lines |
| [03-consensus-replication-deep-dive.md](./03-consensus-replication-deep-dive.md) | Replication protocol, consistency models | ~700 lines |
| [rust-revision.md](./rust-revision.md) | Valtron-based Rust translation | ~700 lines |
| [production-grade.md](./production-grade.md) | Kubernetes, Terraform, monitoring | ~800 lines |
| [04-valtron-integration.md](./04-valtron-integration.md) | Edge cache patterns for Lambda | ~600 lines |

### Key Topics Covered

1. **In-Memory Fundamentals**
   - Key-value operations (GET, SET, MGET, MSET)
   - TTL expiry and eviction
   - Cache-aside patterns
   - Shared-nothing vs shared-everything architecture

2. **Storage Engine**
   - Dashtable design (segments, buckets, slots)
   - DenseSet with pointer tagging
   - Memory overhead comparison (Redis: 32 bytes/record, Dragonfly: 18 bytes/record)
   - Forkless snapshot algorithm
   - Passive and proactive expiry

3. **Query Execution**
   - RESP protocol parsing
   - VLL transaction framework
   - Intent locks and scheduling
   - Multi-key command coordination
   - Blocking commands (BLPOP) implementation
   - Command squashing optimization

4. **Replication**
   - Full sync phase (RDB stream)
   - Journal streaming (incremental changes)
   - Consistency models (eventual, read-after-write)
   - Failure scenarios and recovery
   - Emulated cluster mode

5. **Valtron Integration**
   - Redis task iterator pattern
   - RESP serialization/deserialization
   - Lambda connection pooling
   - Cold start optimization
   - Edge cache patterns (cache-aside, rate limiting)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    DragonflyDB Architecture                  │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Listener Threads                        │    │
│  │         (TCP handling, RESP parsing)                │    │
│  └──────────────────────┬──────────────────────────────┘    │
│                         │                                    │
│                         ▼                                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Worker Threads (Shards)                 │    │
│  │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐          │    │
│  │  │Shard│ │Shard│ │Shard│ │Shard│ │Shard│  ...    │    │
│  │  │  0  │ │  1  │ │  2  │ │  3  │ │  4  │          │    │
│  │  │Dashtable│ │Dashtable│ │Dashtable│            │    │
│  │  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘          │    │
│  │         │         │         │         │             │    │
│  │         └─────────┴────┬────┴─────────┘             │    │
│  │                        │                            │    │
│  │              VLL Transaction Layer                  │    │
│  │         (Multi-key coordination)                    │    │
│  └─────────────────────────────────────────────────────┘    │
│                         │                                    │
│                         ▼                                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Memory Engine                           │    │
│  │         (Dashtable, DenseSet, allocation)           │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Persistence Layer                       │    │
│  │         (RDB snapshots, journal stream)             │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘

Performance on c6gn.16xlarge (16 cores):
- Throughput: 3.8M QPS (25X Redis)
- Latency P99: <1ms
- Memory: 30% more efficient than Redis
```

---

## Quick Start

```bash
# Docker (development)
docker run -d -p 6379:6379 --name dragonfly \
  docker.dragonflydb.io/dragonflydb/dragonfly \
  --maxmemory=4gb --cache_mode=true

# Client (Redis-compatible)
redis-cli -h localhost -p 6379

# Test commands
SET user:123 "Alice"
GET user:123
MSET key1 val1 key2 val2
MGET key1 key2

# Enable caching mode
dragonfly --cache_mode=true --maxmemory=8gb

# Production configuration
dragonfly \
  --maxmemory=14gb \
  --cache_mode=true \
  --port=6379 \
  --dir=/data \
  --dbfilename=dump.rdb \
  --logtostderr
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial exploration created |
| 2026-03-28 | Added 00-zero-to-db-engineer.md (in-memory fundamentals) |
| 2026-03-28 | Added 01-storage-engine-deep-dive.md (Dashtable, DenseSet) |
| 2026-03-28 | Added 02-query-execution-deep-dive.md (VLL transactions) |
| 2026-03-28 | Added 03-consensus-replication-deep-dive.md (Replication) |
| 2026-03-28 | Added rust-revision.md (Valtron translation) |
| 2026-03-28 | Added production-grade.md (Kubernetes, monitoring) |
| 2026-03-28 | Added 04-valtron-integration.md (Edge cache patterns) |
