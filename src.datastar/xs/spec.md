# xs (cross.stream) -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.datastar/xs/`
- **Repository:** https://github.com/cablehead/xs
- **Language:** Rust
- **Version:** 0.12.1-dev
- **Crate name:** cross-stream
- **License:** MIT

## What This Project Is

xs (cross.stream) is a local-first event streaming store. It provides an append-only, ordered event log with content-addressable storage, reactive processors (actors, services, actions), and first-class Nushell integration. Built on fjall (LSM-tree) for indexing and cacache for content-addressable blob storage, with SCRU128 time-ordered IDs.

## Documentation Structure

```
src.datastar/xs/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-storage-engine.md
│   ├── 03-frame-model.md
│   ├── 04-scru128-ids.md
│   ├── 05-indexing.md
│   ├── 06-api-transport.md
│   ├── 07-processor-system.md
│   ├── 08-nushell-integration.md
│   ├── 09-cli-commands.md
│   └── 10-production-patterns.md
├── html/
│   ├── index.html
│   ├── styles.css
│   └── *.html
└── (uses parent build.py)
```
