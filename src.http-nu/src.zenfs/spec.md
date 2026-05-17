# zenfs — Spec

## Source

- **Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.http-nu/src.zenfs/`
- **Language:** TypeScript (@zenfs/core v2.5.6)
- **Package:** `@zenfs/core` — emulates Node.js `fs` API in browsers and other environments
- **178 TypeScript source files across 12 packages**
- **License:** LGPL-3.0-or-later

## What It Is

A cross-platform virtual filesystem library for TypeScript that emulates the Node.js `fs` API. Uses a pluggable backend system where different storage mechanisms (in-memory, HTTP fetch, IndexedDB, real node:fs, worker ports, SharedArrayBuffer) can be mounted at different paths, creating a unified VFS.

## Documentation Goal

1. Understand the VFS architecture (VFS layer, Backend layer, Internal layer)
2. Understand the Backend system (InMemory, Fetch, Passthrough, Port, SingleBuffer, CopyOnWrite)
3. Understand the Store abstraction (key-value store with transactions)
4. Understand the Inode format (4 KiB, version 5, memium struct decorators)
5. Understand the Node.js API compatibility (sync, async, promises, streams, Dir)
6. Understand the RPC system for Worker/MessagePort communication

## Structure

```
src.zenfs/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-vfs-architecture.md
│   └── 02-backends-stores.md
├── html/
│   └── ... (generated)
```

## Tasks

| Task | Status |
|------|--------|
| Read all 178 source files | DONE |
| Write spec.md | DONE |
| Write 00-overview.md | DONE |
| Write 01-vfs-architecture.md | DONE |
| Write 02-backends-stores.md | DONE |
| Write README.md | DONE |
| Generate HTML via build.py | DONE |
| Grandfather review | DONE |

## Build System

```bash
python3 build.py src.http-nu/src.zenfs
```

## Quality Requirements

Per documentation_directive.md — minimum 2 mermaid diagrams, 3 code snippets with file paths, 1 Aha moment per document. All names/numbers/flows verified against source.

## Expected Outcome

A reader can understand how to embed a Node.js-compatible filesystem in the browser, configure multiple storage backends, and leverage the transaction system for atomic operations.

## Resume Point

Spec written. Need to write 3 markdown docs + README + build + review.
