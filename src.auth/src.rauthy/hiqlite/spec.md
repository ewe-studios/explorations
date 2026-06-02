# hiqlite — Spec

## Source Codebase Location

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.auth/src.rauthy/hiqlite/`
- **Repository:** https://github.com/sebadob/hiqlite.git
- **Language:** Rust
- **License:** Apache-2.0
- **Author:** Sebastian Dobe

## What This Project Is

Hiqlite is an embeddable SQLite database with Raft consensus for high availability. It provides:

- **Strong consistency** — Raft ensures all nodes agree
- **High availability** — Automatic leader failover
- **Self-healing** — Recovery from crashes and data loss
- **Embedded** — No separate database process
- **Fast** — Up to 24.5k inserts/s
- **KV caches** — In-memory with disk persistence

## Documentation Goal

After reading this documentation, an engineer should understand:

1. The Raft architecture and consensus mechanism
2. The WAL (Write-Ahead Log) storage
3. The SQLite state machine
4. The networking layer (WebSockets)
5. Query routing and execution
6. Backup and restore
7. KV cache system
8. Distributed locks and counters
9. Configuration and deployment
10. Performance tuning

## Documentation Structure

```
src.auth/src.rauthy/hiqlite/
├── spec.md                      ← This file
├── exploration.md               ← Original exploration
├── markdown/
│   ├── README.md                ← Index
│   ├── 00-overview.md           ← Philosophy, features
│   ├── 01-architecture.md       ← Raft, WAL, SQLite
│   ├── 02-raft.md               ─ Raft consensus
│   ├── 03-wal.md                ─ WAL storage
│   ├── 04-sqlite.md             ─ SQLite integration
│   ├── 05-network.md              ─ WebSocket networking
│   ├── 06-queries.md              ─ Query handling
│   ├── 07-backup.md               ─ Backup/restore
│   └── 08-deployment.md             ─ Configuration
├── html/
└── (uses ../../../build.py)
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Via exploration |
| 2 | Create spec.md | DONE | This file |
| 3 | Write README.md | DONE | Index |
| 3 | Write 00-overview.md | DONE | Philosophy, features |
| 3 | Write 01-architecture.md | DONE | Raft, WAL, SQLite |
| 3 | Write 02-raft.md | DONE | Raft consensus |
| 3 | Write 03-wal.md | DONE | WAL storage |
| 3 | Write 04-sqlite.md | DONE | SQLite integration |
| 3 | Write 05-network.md | DONE | WebSocket networking |
| 3 | Write 06-queries.md | DONE | Query handling |
| 3 | Write 07-backup.md | DONE | Backup/restore |
| 3 | Write 08-deployment.md | DONE | Configuration |
| 4 | Generate HTML | DONE | All 9 documents generated |
| 5 | Grandfather review | TODO | Verify against source |

## Build System

**Script:** `../../../build.py`

```bash
python3 build.py src.auth/src.rauthy/hiqlite
```

## Quality Requirements

All documents must meet the Iron Rules from the markdown directive.

## Resume Point

Resume from the last uncompleted task in the Tasks table.
