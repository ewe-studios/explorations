# Orbitinghail Ecosystem -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.orbitinghail/`
- **Language:** Rust (multiple crates)
- **Projects:** Fjall (KV store), LSM-Tree, Graft (sync engine), SQLSync (SQLite sync), utility crates

## What This Ecosystem Is

Orbitinghail is a collection of Rust storage projects authored primarily by @carlsverre. The core projects are:

1. **fjall** — Embedded LSM-tree KV store with multi-keyspace, SSI transactions, MVCC
2. **lsm-tree** — The underlying LSM tree implementation (used by fjall)
3. **byteview** — Zero-copy byte view types for efficient key handling
4. **value-log** — Append-only value log for large blob storage
5. **sfa** — Succinct fingerprint array for prefix search
6. **u24** — 24-bit unsigned integer type
7. **precept** — Fault injection testing framework
8. **graft** — Transactional storage engine for edge sync (uses fjall)
9. **sqlsync** — SQLite sync via custom VFS and WASM reducers
10. **sqlite-plugin** — SQLite plugin for fjall

## Documentation Structure

```
src.orbitinghail/orbitinghail/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-fjall-database.md
│   ├── 02-fjall-keyspace.md
│   ├── 03-fjall-journal.md
│   ├── 04-fjall-transactions.md
│   ├── 05-fjall-snapshots.md
│   ├── 06-fjall-recovery.md
│   ├── 07-lsm-tree.md
│   ├── 08-graft.md
│   ├── 09-sqlsync.md
│   └── 10-utility-crates.md
├── html/
│   ├── index.html
│   ├── styles.css
│   └── *.html
```

## Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Create spec.md | DONE |
| 2 | Write 00-overview.md | IN PROGRESS |
| 3 | Write 01-fjall-database.md | |
| 4 | Write 02-fjall-keyspace.md | |
| 5 | Write 03-fjall-journal.md | |
| 6 | Write 04-fjall-transactions.md | |
| 7 | Write 05-fjall-snapshots.md | |
| 8 | Write 06-fjall-recovery.md | |
| 9 | Write 07-lsm-tree.md | |
| 10 | Write 08-graft.md | |
| 11 | Write 09-sqlsync.md | |
| 12 | Write 10-utility-crates.md | |
| 13 | Write README.md (index) | |
| 14 | Generate HTML with build.py | |
| 15 | Grandfather Review | |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations
python3 build.py src.orbitinghail/orbitinghail
```

Python 3.12+ stdlib only, zero dependencies. Idempotent.
