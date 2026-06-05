---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.ContentAddressable/
explored_at: 2026-06-04
documentation_goal: Document how xs uses fjall (LSM-tree), cacache-rs (content-addressable storage), and scru128 (sortable IDs), with deep dives into each underlying library.
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

How xs uses fjall, cacache-rs, and scru128 to build a high-performance append-only stream store. Covers:
- How LSM trees work (memtable, SST, compaction)
- How fjall builds on lsm-tree (WAL, keyspaces, transactions)
- How cacache-rs works (content-addressable storage)
- How scru128 generates sortable IDs and differs from Twitter Snowflake
- How xs ties them all together
- How fjall can be used in different ways
- Whether sync to S3/object storage is possible

## 3. Documentation Structure

```
src.LSMUsage/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-lsm-fjall-cacache-scru128.md
│   ├── 01-sst-format-block-structure.md
├── html/
└── build.py
```

## 4. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE |
| 1 | 00-lsm-fjall-cacache-scru128.md | DONE |
| 2 | 01-sst-format-block-structure.md | DONE |
| 3 | Grandfather review | DONE |
| 4 | Fix findings | DONE |
| 5 | Generate HTML | DONE |

Build via `python3 build.py .`. Grandfather review mandatory.
