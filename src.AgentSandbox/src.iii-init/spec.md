---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/crates/iii-init/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document iii-init — the PID 1 init binary for iii microVM workers. Covers root pivot, mount sequence, supervisor mode, shell dispatcher, and filesystem handlers.
---

# Spec: iii-init Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `iii/crates/iii-init/` |
| Language | Rust (edition 2024) |
| License | Elastic-2.0 |
| LOC | 6,429 |
| Source files | 13 |

## 2. What iii-init Is

iii-init is the PID 1 init binary that runs inside every iii microVM worker. It boots the guest environment: pivots the root filesystem off virtiofs onto tmpfs (working around a libkrun readdir bug), mounts essential Linux filesystems, overrides `/proc/meminfo` for cgroup-aware memory reporting, raises file descriptor limits, configures networking, and then execs the user worker process. In supervisor mode, it also serves a control channel for host-driven restart.

## 3. Documentation Goal

A reader should understand:
1. The boot sequence: pivot → mount → meminfo → rlimit → network → exec
2. The root pivot workaround for the libkrun virtiofs readdir bug
3. The mount sequence for essential filesystems
4. The `/proc/meminfo` override for Bun's Zig allocator
5. Supervisor mode vs legacy mode
6. The shell dispatcher for `iii worker exec`
7. The filesystem handler for native Rust ops (no shell-outs)

## 4. Documentation Structure

```
src.iii-init/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-boot-sequence.md
│   ├── 02-root-pivot.md
│   ├── 03-mount-sequence.md
│   ├── 04-supervisor.md
│   ├── 05-shell-dispatcher.md
│   ├── 06-fs-handler.md
│   ├── 07-network.md
│   ├── 08-cross-cutting.md
├── html/
└── build.py
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-boot-sequence.md | TODO |
| 3 | 02-root-pivot.md | TODO |
| 4 | 03-mount-sequence.md | TODO |
| 5 | 04-supervisor.md | TODO |
| 6 | 05-shell-dispatcher.md | TODO |
| 7 | 06-fs-handler.md | TODO |
| 8 | 07-network.md | TODO |
| 9 | 08-cross-cutting.md | TODO |
| 10 | Grandfather review | DONE |
| 11 | Fix findings | DONE (no discrepancies — all verified) |
| 12 | Generate HTML | DONE |

## 6. Build System

```bash
cd src.iii-init/
python3 build.py .
```

## 7. Quality Requirements

Follow all Iron Rules. Grandfather review mandatory.

## 8. Resume Point

Write docs in order. After all done, grandfather review, fix, generate HTML.
