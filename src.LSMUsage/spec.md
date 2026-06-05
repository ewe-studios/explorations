---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressable/
explored_at: 2026-06-04
documentation_goal: >
  Comprehensive documentation of how xs uses fjall (LSM-tree database), 
  cacache-rs (content-addressable storage), and scru128 (sortable IDs) to build 
  a high-performance append-only stream store. Covers each library's internals, 
  how they integrate, scru128 vs Twitter Snowflake, alternative fjall usage 
  patterns, and S3/object storage sync possibilities.
---

# Spec: LSM Usage Documentation

## 1. Source Codebases

| Library | Location | LOC |
|---------|----------|-----|
| lsm-tree | `lsm-tree/src/` | 29,740 |
| fjall | `fjall/src/` | 12,196 |
| cacache-rs | `cacache-rs/src/` | 1,228 |
| xs | `xs/src/` | 13,083 |
| scru128 | `src.scru128/rust/src/` | 1,845 |

## 2. What This Documents

The full stack behind xs's stream store: how LSM trees work internally, how fjall wraps lsm-tree into a full database, how cacache-rs provides content-addressable storage, how scru128 generates sortable unpredictable IDs, and how xs ties them all together into an append-only stream store with TTL, GC, and hierarchical topic queries.

## 3. Documentation Structure

```
src.LSMUsage/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-lsm-tree-internals.md
│   ├── 02-fjall-database.md
│   ├── 03-cacache-rs.md
│   ├── 04-data-structures-master.md
│   ├── 04-scru128.md
│   ├── 05-xs-stream-store.md
│   ├── 06-fjall-patterns.md
│   ├── 07-s3-sync.md
├── html/
├── build.py
└── styles.css
```

## 4. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-lsm-tree-internals.md | DONE |
| 3 | 02-fjall-database.md | DONE |
| 4 | 03-cacache-rs.md | DONE |
| 5 | 04-data-structures-master.md | DONE |
| 6 | 04-scru128.md | DONE |
| 7 | 05-xs-stream-store.md | DONE |
| 8 | 06-fjall-patterns.md | DONE |
| 9 | 07-s3-sync.md | DONE |
| 10 | Grandfather review | DONE |
| 11 | Fix findings | DONE |
| 12 | Generate HTML | DONE |

## 5. Quality Requirements

Follow all Iron Rules from the documentation directive. Grandfather review mandatory.

## 6. Resume Point

Continue writing markdown documents in order. After all documents written, run grandfather review, fix, generate HTML.
