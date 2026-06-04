---
title: s3Stream Documentation
---

# s3Stream Documentation

S3-optimized streaming storage — how to efficiently write and read streaming data on S3.

## Documents

### Performance Deep Dive

- [**00 — Overview**](00-overview.md) — What s3Stream is, why it's fast, the big picture
- [**00 — Write Path**](00-write-path.md) — WAL → LogCache → batched S3 upload
- [**01 — Read Path**](01-read-path.md) — LogCache → DataBlockCache → footer-first S3 reads
- [**02 — S3 Object Format**](02-s3-object-format.md) — Data + Index + Footer, self-describing objects
- [**03 — Caching**](03-caching.md) — LogCache merge, DataBlockCache eviction, readahead
- [**04 — Rust Design**](04-rust-design.md) — Condensed Rust implementation
