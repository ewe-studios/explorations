---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/crates/iii-supervisor/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document iii-supervisor — the in-VM process supervisor library for host-driven worker restart via virtio-console control channel.
---

# Spec: iii-supervisor Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `iii/crates/iii-supervisor/` |
| Language | Rust (edition 2024) |
| License | Elastic-2.0 |
| LOC | 1,201 |
| Source files | 5 |

## 2. What iii-supervisor Is

iii-supervisor is an in-VM process supervisor library consumed by iii-init (guest side) for host-driven worker restart RPCs, and by iii-worker (host side) for wire protocol types. It manages the lifecycle of the user worker subprocess: spawn, kill, respawn — with a JSON-over-virtio-console control channel.

## 3. Documentation Goal

A reader should understand:
1. The control-channel protocol: JSON requests over virtio-console
2. The child process lifecycle: spawn, kill, respawn, signal forwarding
3. The process-group isolation for killing entire worker subtrees
4. The graceful shutdown: SIGTERM → poll → SIGKILL
5. The wire protocol types shared between host and guest

## 4. Documentation Structure

```
src.iii-supervisor/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-protocol.md
│   ├── 02-child-lifecycle.md
│   ├── 03-control-channel.md
│   ├── 04-shell-protocol.md
│   ├── 05-cross-cutting.md
├── html/
└── build.py
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-protocol.md | TODO |
| 3 | 02-child-lifecycle.md | TODO |
| 4 | 03-control-channel.md | TODO |
| 5 | 04-shell-protocol.md | TODO |
| 6 | 05-cross-cutting.md | TODO |
| 7 | Grandfather review | DONE |
| 8 | Fix findings | DONE (no discrepancies — all verified) |
| 9 | Generate HTML | DONE |

## 6-9. Standard sections

Build via `python3 build.py .`. Grandfather review mandatory.
