---
title: LSM Usage Documentation
---

# LSM Usage Documentation

Deep dive into how xs uses fjall (LSM-tree database), cacache-rs (content-addressable storage), and scru128 (sortable IDs) to build a high-performance append-only stream store.

## Documents

### Core Concepts

- [**00 — LSM, fjall, cacache, scru128**](00-lsm-fjall-cacache-scru128.md) — How each library works, how xs uses them, scru128 vs Snowflake
- [**01 — SST Format & Block Structure**](01-sst-format-block-structure.md) — SST file format, block structure, bloom filters, xs's fjall configuration

## Key Takeaways

1. **LSM trees** turn random writes into sequential writes via memtables and SST files
2. **fjall** wraps lsm-tree with a WAL (journal), keyspaces, and transactions
3. **cacache-rs** provides content-addressable storage with integrity guarantees
4. **scru128** provides sortable, unpredictable IDs — unlike Snowflake, no machine ID needed
5. **xs** uses fjall for metadata (frames) and cacache for content (bytes), with scru128 IDs for ordering
