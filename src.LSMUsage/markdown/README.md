---
title: LSM Usage Documentation
---

# LSM Usage Documentation

Deep dive into LSM trees, fjall, cacache-rs, scru128, and xs — with comprehensive data structure implementations from scratch in Rust.

## Documents

### Foundations

- [**00 — Overview**](00-overview.md) — The full stack, why it works, source libraries
- [**01 — LSM Tree Internals**](01-lsm-tree-internals.md) — Memtable (SkipList), SST files, blocks, prefix compression, bloom filters, compaction
- [**02 — fjall Database**](02-fjall-database.md) — WAL/journal, keyspaces, cross-keyspace batches, OCC transactions, recovery
- [**03 — cacache-rs**](03-cacache-rs.md) — Content-addressable storage, index layer, content layer, atomic writes, integrity verification

### Data Structures Master Guide

- [**04 — Data Structures Master**](04-data-structures-master.md) — SkipList, SST files, Bloom filters, block indexes, WAL, content-addressable storage, scru128 — with full Rust implementations from scratch, edge cases, and disk storage patterns

### scru128 IDs

- [**04 — scru128**](04-scru128.md) — Sortable, unpredictable IDs, comparison with Twitter Snowflake, implementation from scratch

### Integration

- [**05 — xs Stream Store**](05-xs-stream-store.md) — How xs ties fjall, cacache-rs, and scru128 together into an append-only stream store
- [**06 — fjall Patterns**](06-fjall-patterns.md) — Alternative usage patterns: simple KV, multi-tenant, event log, session store, config store, time-series
- [**07 — S3 Sync**](07-s3-sync.md) — Syncing fjall and cacache to S3/object storage, architecture, challenges, alternatives
