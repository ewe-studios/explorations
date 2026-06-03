---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/iroh-blobs
repository: git@github.com:n0-computer/iroh-blobs
revised_at: 2026-06-03T00:00:00Z
workspace: iroh-blobs
---

# iroh-blobs — Spec File

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/iroh-blobs`
- **Language:** Rust
- **Edition:** 2021
- **Rust Version:** 1.91
- **License:** MIT OR Apache-2.0
- **Version:** 0.91.0
- **Remote:** `git@github.com:n0-computer/iroh-blobs`

## What the Project Is

Iroh-blobs implements BLAKE3-based content-addressed blob transfer over iroh connections. It provides verified streaming from kilobytes to terabytes, with in-memory and file-based stores, collection (hash sequence) support, and a ticket-based sharing mechanism.

## Documentation Goal

After reading, a reader should understand:
1. The BLAKE3 verified streaming protocol (bao outboards)
2. The store architecture (MemStore, FsStore with redb metadata)
3. The protocol wire format (Get, GetMany, Push, Observe)
4. The client FSM states for blob transfer
5. The API layer (Store, Blobs, Tags, Downloader, Remote)
6. The garbage collection algorithm (mark-sweep)
7. The ticket format and sharing mechanism
8. Collection (hash sequence) format and wire format

## Documentation Structure

```
iroh-blobs/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-hash-and-bao.md
│   ├── 03-protocol.md
│   ├── 04-store-fs.md
│   ├── 05-store-mem.md
│   ├── 06-api.md
│   ├── 07-get-client.md
│   ├── 08-provider.md
│   ├── 09-data-flow.md
│   └── 10-cross-cutting.md
├── html/
└── build.py
```

## Tasks

| Phase | Document | Status |
|-------|----------|--------|
| Foundation | README.md | DONE |
| Foundation | 00-overview.md | DONE |
| Architecture | 01-architecture.md | DONE |
| Deep Dive | 02-hash-and-bao.md | DONE |
| Deep Dive | 03-protocol.md | DONE |
| Deep Dive | 04-store-fs.md | DONE |
| Deep Dive | 05-store-mem.md | DONE |
| Deep Dive | 06-api.md | DONE |
| Deep Dive | 07-get-client.md | DONE |
| Deep Dive | 08-provider.md | DONE |
| Cross-Cutting | 09-data-flow.md | DONE |
| Cross-Cutting | 10-cross-cutting.md | DONE |
| Grandfather Review | All documents | DONE |
| HTML Generation | build.py | DONE |

## Build System

```bash
cd iroh-blobs && python3 build.py
```

## Quality Requirements

All 10 iron rules from the markdown directive.

## Expected Outcome

After reading, a developer can use iroh-blobs for content-addressed storage, understand the verified streaming protocol, and debug store issues.

## Resume Point

Write documents in order: 00 → 01 → 02 → ... → 10. Each from actual source code, then grandfather review.
