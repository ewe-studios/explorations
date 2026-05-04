# Orbitinghail Rust Projects -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.orbitinghail/`
- **Language:** Rust
- **Author:** orbitinghail (hello@orbitinghail.dev)
- **License:** MIT/Apache-2.0 (varies by sub-project)
- **Sub-projects:** 18 distinct Rust projects covering LSM-tree storage, syncable databases, compressed bitmaps, SQLite sync, full-text search, and distributed systems

## What This Project Is

A collection of interconnected Rust projects centered around storage engines and offline-first data synchronization. The core is **fjall** — an embeddable LSM-tree KV database — extended by **graft** for syncable storage with remote S3 support, **splinter-rs** for compressed bitmaps, **sqlsync** for offline-first SQLite sync with WASM, and production-grade systems like **tantivy** (search engine) and **quickwit** (distributed search).

## Documentation Goal

A reader should understand:

1. How LSM-tree databases work (memtable, SSTables, compaction, bloom filters)
2. How graft provides syncable storage with local Fjall + remote S3/OpenDAL
3. How compressed bitmaps (splinter-rs) enable efficient set operations
4. How sqlsync syncs SQLite databases over the wire with WASM support
5. How filesystem storage formats work (SSTable layout, segment format, commit hashes)
6. How data validation works (XXH3 checksums, BLAKE3 commits, CRC64, ZStd checksums)
7. How S3/remote storage is optimized (HTTP/1 only, CBE encoding, atomic writes, concurrent operations)
8. How to replicate these patterns in your own Rust projects
9. What a production-grade storage system looks like

## Documentation Structure

```
src.orbitinghail/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-lsm-tree.md
│   ├── 03-fjall-database.md
│   ├── 04-graft-storage.md
│   ├── 05-remote-sync.md
│   ├── 06-splinter-bitmap.md
│   ├── 07-sqlsync.md
│   ├── 08-storage-formats.md
│   ├── 09-checksums-validation.md
│   ├── 10-s3-remote-optimizations.md
│   ├── 11-rust-equivalents.md
│   ├── 12-production-patterns.md
│   └── 13-wasm-web-patterns.md
├── html/
│   ├── index.html
│   ├── styles.css
│   └── *.html
└── build.py (shared from parent)
```

## Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Create spec.md | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-lsm-tree.md | DONE |
| 5 | Write 03-fjall-database.md | DONE |
| 6 | Write 04-graft-storage.md | DONE |
| 7 | Write 05-remote-sync.md | DONE |
| 8 | Write 06-splinter-bitmap.md | DONE |
| 9 | Write 07-sqlsync.md | DONE |
| 10 | Write 08-storage-formats.md | DONE |
| 11 | Write 09-checksums-validation.md | DONE |
| 12 | Write 10-s3-remote-optimizations.md | DONE |
| 13 | Write 11-rust-equivalents.md | DONE |
| 14 | Write 12-production-patterns.md | DONE |
| 15 | Write 13-wasm-web-patterns.md | DONE |
| 16 | Write README.md (index) | DONE |
| 17 | Generate HTML with build.py | DONE |
| 18 | Grandfather Review | TODO |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations
python3 build.py src.orbitinghail
```

## Quality Requirements

All ten Iron Rules from the documentation directive apply.

## Expected Outcome

After reading, an engineer should be able to understand LSM-tree internals, build a syncable storage system, implement compressed bitmaps, sync SQLite with WASM, and optimize S3 storage patterns.

## Resume Point

Check the task table above for the current phase.
