# simple-fs — Documentation

**Source:** `src/` — 39 Rust files. Version 0.12.0-WIP. MIT OR Apache-2.0.

simple-fs is a Rust crate providing a simple and convenient API for file system access. It centers around `SPath` — a UTF-8 guaranteed, POSIX-normalized path wrapper — and provides: glob-filtered file listing, safe delete/trash, byte-range span reading, debounced file watching, and feature-gated JSON/TOML/binary serialization.

## Documentation

- [Overview](00-overview.md) — Architecture at a glance, quick start, key types, feature flags
- [Architecture](01-architecture.md) — Module structure, error model, feature-gated architecture
- [SPath](02-spath.md) — UTF-8 normalized path wrapper, normalization, collapse, MIME detection, transformers
- [Listing](03-listing.md) — Glob grouping algorithm, WalkDir prefix pruning, sort_by_globs
- [Spans, Safer, Watch](04-spans-safer-watch.md) — Byte-range reading, line spans, CSV spans, safer remove/trash, file watching
- [Features](05-features.md) — JSON, TOML, binary numbers, pretty_size formatting

## Feature Flags

| Feature | Enables | Dependencies |
|---------|---------|--------------|
| `with-json` | `load_json`, `save_json`, NDJSON | serde, serde_json |
| `with-toml` | `load_toml`, `save_toml` | serde, toml |
| `bin-nums` | Binary numeric load/save | byteorder |
| `full` | All of the above | — |
