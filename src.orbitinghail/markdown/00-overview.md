# Orbitinghail Rust Projects -- What They Are and Why They Exist

This is a collection of 18 interconnected Rust projects from orbitinghail (hello@orbitinghail.dev), centered around one theme: **local-first storage with offline sync**. The core building block is **fjall** — a K.I.S.S. LSM-tree KV database — extended by **graft** for syncable storage (local Fjall + remote S3), **splinter-rs** for compressed bitmaps, and **sqlsync** for offline-first SQLite sync with WASM browser support.

**Aha:** The entire ecosystem is designed around append-only writes. LSM-trees never update in place — they append to a memtable, flush to immutable SSTables, and compact in the background. Graft extends this by appending to a log with LSNs (Log Sequence Numbers) and syncing segments to S3. Even the remote storage uses append-only object paths with CBE (Complement Big-Endian) encoding so that lexicographic ordering of object keys matches reverse-chronological ordering of commits. Append-only is the simplest write pattern that enables crash safety, replication, and time-travel queries simultaneously.

## The Ecosystem at a Glance

| Project | Purpose | Key Dependency |
|---------|---------|---------------|
| `fjall/` | Embeddable KV database (LSM-tree) | lsm-tree, byteview |
| `lsm-tree/` | Core LSM-tree implementation | byteview, u24, quick_cache |
| `byteview/` | Zero-copy immutable byte slice | — |
| `u24/` | 24-bit unsigned integer | zerocopy |
| `value-log/` | Value log for key-value separation | byteview |
| `sfa/` | Simple File-based Archive | — |
| `graft/` | Syncable storage engine | fjall, OpenDAL |
| `splinter-rs/` | Compressed bitmap for sparse u32 sets | bitvec, u24, crc64fast |
| `sqlsync/` | Offline-first SQLite sync | sqlite-vfs, WASM |
| `tantivy/` | Full-text search engine | sstable, stacker, bitpacker |
| `quickwit/` | Distributed search engine | tantivy, DataFusion, AWS SDK |
| `chitchat/` | Gossip protocol for cluster membership | — |
| `precept/` | Fault injection testing framework | — |

## Philosophy

1. **Append-only everywhere.** Writes are always appends. Reads are prefix scans. Compaction is background maintenance, not an in-place update.
2. **Local-first, remote-sync.** The local store is authoritative. Remote is a replica. Sync is eventual and conflict-free through LSN ordering.
3. **Crash-safe by default.** WAL journaling, atomic writes with preconditions, and checksums on every block.
4. **Zero-copy where possible.** `byteview` provides `&[u8]` slices without allocation. `u24` packs 3 bytes into 4-byte alignment. `splinter-rs` compresses bitmaps for sparse sets.

See [Architecture](01-architecture.md) for the full dependency graph.
See [LSM-Tree](02-lsm-tree.md) for the core storage algorithm.
