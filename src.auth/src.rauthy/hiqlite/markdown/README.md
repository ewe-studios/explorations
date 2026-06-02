# hiqlite Documentation

Embeddable SQLite with Raft consensus.

## Document Index

| # | Document | Description |
|---|----------|-------------|
| 00 | [Overview](00-overview.html) | Philosophy, features |
| 01 | [Architecture](01-architecture.html) | Raft, WAL, SQLite |
| 02 | [Raft](02-raft.html) | Raft consensus |
| 03 | [WAL](03-wal.html) | WAL storage |
| 04 | [SQLite](04-sqlite.html) | SQLite integration |
| 05 | [Network](05-network.html) | WebSocket networking |
| 06 | [Queries](06-queries.html) | Query handling |
| 07 | [Backup](07-backup.html) | Backup/restore |
| 08 | [Deployment](08-deployment.html) | Configuration |

## Quick Links

- **Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/src.rauthy/hiqlite/`
- **Repository:** https://github.com/sebadob/hiqlite.git
- **Architecture:** [ARCHITECTURE.md](https://github.com/sebadob/hiqlite/blob/main/ARCHITECTURE.md)

## What is hiqlite?

An embeddable SQLite database with Raft consensus:

```rust
use hiqlite::{NodeConfig, start_node};

// Start Raft node
let node = start_node(NodeConfig {
    node_id: 1,
    nodes: vec![
        "192.168.1.10:2380",
        "192.168.1.11:2380",
        "192.168.1.12:2380",
    ],
    data_dir: "/data/hiqlite",
}).await?;

// Execute query (replicated)
node.execute("INSERT INTO users (name) VALUES ('Alice')").await?;

// Local read
let users: Vec<User> = node.query_as("SELECT * FROM users").await?;
```

## Features

| Feature | Description |
|---------|-------------|
| **Raft** | Strong consistency |
| **SQLite** | Local, fast reads |
| **HA** | Automatic failover |
| **Self-heal** | Crash recovery |
| **Cache** | KV stores |
| **Backup** | Encrypted to S3 |

## Next Steps

Start with [Overview →](00-overview.html).
