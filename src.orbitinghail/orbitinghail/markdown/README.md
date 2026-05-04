# Orbitinghail Rust Projects -- Documentation Index

## Foundation

- [Overview](00-overview.md) — What the projects are, ecosystem map, philosophy
- [Architecture](01-architecture.md) — Layer diagram, dependency graph, entry points, technology stack
- [LSM-Tree](02-lsm-tree.md) — Memtable, SSTables, bloom filters, compaction strategies
- [Fjall Database](03-fjall-database.md) — KV database, WAL, transactions, MVCC
- [Graft Storage](04-graft-storage.md) — Page-oriented syncable storage, snapshots, LEAP oracle
- [Remote Sync](05-remote-sync.md) — S3 sync process, CBE encoding, segment builder, idempotency

## Deep Dives

- [Splinter Bitmap](06-splinter-bitmap.md) — Compressed bitmap, u24, CRC64
- [SQLSync](07-sqlsync.md) — Offline-first SQLite, WASM, replication protocol
- [Storage Formats](08-storage-formats.md) — SSTable, SFA, WAL, segment layouts
- [Checksums and Validation](09-checksums-validation.md) — XXH3, BLAKE3, CRC64, ZStd integrity
- [S3 Remote Optimizations](10-s3-remote-optimizations.md) — HTTP/1, DNS caching, atomic writes, Range reads
- [Rust Equivalents](11-rust-equivalents.md) — Usage patterns, idiomatic Rust, production alternatives
- [Production Patterns](12-production-patterns.md) — Durability, recovery, monitoring, scaling, compaction tuning
- [WASM and Web Patterns](13-wasm-web-patterns.md) — WASM build, tsify, WebCrypto, Cloudflare Workers
