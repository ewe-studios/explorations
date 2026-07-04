---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.strukto-ai/mirage/
language: Python, TypeScript
version: 0.0.2-alpha
explored_at: 2026-07-04
license: Apache-2.0
repository: github.com/strukto-ai/mirage
---

# spec.md — Mirage VFS Documentation

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.strukto-ai/mirage/`
- **Language:** Python (~2,000 files), TypeScript (~2,500 files)
- **Version:** 0.0.2-alpha
- **License:** Apache 2.0

## What Mirage Is

Mirage is a Unified Virtual File System (VFS) for AI agents that mounts 50+ heterogeneous services (S3, Google Drive, Slack, Gmail, GitHub, Redis, MongoDB, Postgres, SSH, etc.) as a single filesystem tree. AI agents interact with every backend using standard Unix commands (`cat`, `grep`, `ls`, `find`, `wc`, `jq`). Uses an in-process tree-sitter bash parser (no real shell spawned) for sandboxed execution.

## Documentation

```
src.strukto-ai/
├── spec.md, build.py, markdown/, html/
```
