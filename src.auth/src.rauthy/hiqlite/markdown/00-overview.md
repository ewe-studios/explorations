---
title: hiqlite Overview
next: 01-architecture.md
---

# hiqlite Overview

Embeddable SQLite with Raft consensus.

## Philosophy

**SQLite is fast — let's make it distributed.**

### The Problem

- SQLite is single-node only
- Need HA for production
- External databases add complexity
- Existing solutions are async or server-based

### The Solution

**Aha:** Embed Raft consensus directly in the application.

```mermaid
flowchart TB
    subgraph Traditional
        APP1[App + SQLite] 
        DB[External DB]
    end

    subgraph Hiqlite
        APP2[App + SQLite + Raft]
    end

    Traditional --> DB
    Hiqlite --> Hiqlite
```

**Keep SQLite's strengths:**
- Local, fast reads (no network)
- No separate process
- Zero external dependencies

**Add HA features:**
- Raft consensus
- Automatic failover
- Self-healing

## Key Features

### 1. Embeddable

```rust
// SQLite is embedded
let node = start_node(config).await?;

// Local reads (no network)
let users = node.query_as::<User>("SELECT * FROM users").await?;
```

### 2. Raft Consensus

```mermaid
flowchart TB
    subgraph Cluster["3-Node Cluster"]
        N1[Node 1]
        N2[Node 2]
        N3[Node 3]
    end

    N1 <-->|Raft| N2
    N2 <-->|Raft| N3
    N3 <-->|Raft| N1
```

**Properties:**
- Strong consistency
- Automatic leader election
- Fault tolerant (tolerates (n-1)/2 failures)

### 3. Self-Healing

- **Crash recovery** — Rebuild from WAL + snapshots
- **Data loss** — Sync from other nodes
- **Split brain** — Automatic detection

### 4. High Performance

| Metric | Value |
|--------|-------|
| Inserts/s | 24.5k (M2 SSD) |
| Inserts/s | 16.5k (SATA SSD) |
| Cache ops/s | ~500k (memory) |

**Aha:** Near physical disk limits despite single SQLite writer.

### 5. KV Cache

```rust
// In-memory KV cache
node.cache_set("key", "value", Some(Duration::from_secs(300))).await?;

// Disk-backed (rebuilds after restart)
let value: Option<String> = node.cache_get("key").await?;
```

### 6. Encrypted Backups

```rust
// Backup to S3
node.backup_to_s3(
    "s3://bucket/backups/db-2025-01-15.sql",
    &encryption_key,
).await?;
```

## Architecture

```mermaid
flowchart TB
    subgraph App["Application"]
        API[API Layer]
    end

    subgraph Hiqlite["hiqlite"]
        RAFT[Raft Layer]
        SQLITE[SQLite]
        WAL[hiqlite-wal]
        CACHE[KV Cache]
    end

    subgraph Network["Network"]
        WS[WebSocket]
    end

    API --> RAFT
    RAFT --> SQLITE
    RAFT --> WAL
    RAFT --> CACHE
    RAFT <-->|WebSocket| Network
```

## Use Cases

| Use Case | Benefit |
|----------|---------|
| **Identity Provider** | HA for rauthy |
| **Microservices** | Embedded DB per service |
| **Edge Computing** | Distributed at edge |
| **IoT** | Local + replicated |

## Next Steps

Continue to [Architecture →](01-architecture.html).
