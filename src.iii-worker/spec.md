---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/crates/iii-worker/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document the iii-worker managed runtime — VM-based sandboxed worker execution, OCI registry, firmware management, lifecycle, and the CLI surface.
---

# Spec: iii-worker Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `iii/crates/iii-worker/` |
| Language | Rust (edition 2024) |
| License | Elastic-2.0 |
| LOC | 42,998 (55,932 including tests) |
| Source files | 43 .rs files |

## 2. What iii-worker Is

iii-worker is the managed worker runtime for iii. It manages the full lifecycle of sandboxed workers: downloading from the workers registry or OCI images, configuring, starting/stopping VMs (via msb_krun/libkrun), and providing CLI commands (`iii worker add`, `start`, `stop`, `list`, `exec`, `status`, `sync`, `verify`, `update`, `clear`, `reinstall`, `remove`). Workers run in isolated krun VMs with their own filesystem, network, and process namespace.

## 3. Documentation Goal

A reader should understand:
1. The CLI surface: all commands and their arguments
2. The three worker types: registry, OCI, and local
3. The add pipeline: resolve → download → extract → configure → boot
4. The sandbox daemon: VM management, overlay filesystems, exec, FS access
5. The worker manager: libkrun VM lifecycle, OCI image pulling
6. Firmware management: libkrunfw download, caching, version resolution
7. The lockfile system (iii.lock): version pinning and drift detection
8. Source watching for local development
9. The stdout/stderr contract for scriptability
10. Integration with iii-filesystem, iii-network, iii-supervisor

## 4. Documentation Structure

```
src.iii-worker/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-cli-surface.md
│   ├── 03-worker-types.md
│   ├── 04-add-pipeline.md
│   ├── 05-managed-ops.md
│   ├── 06-sandbox-daemon.md
│   ├── 07-vm-lifecycle.md
│   ├── 08-firmware.md
│   ├── 09-lockfile.md
│   ├── 10-cross-cutting.md
├── html/
├── exploration.md
└── build.py
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-architecture.md | TODO |
| 3 | 02-cli-surface.md | TODO |
| 4 | 03-worker-types.md | TODO |
| 5 | 04-add-pipeline.md | TODO |
| 6 | 05-managed-ops.md | TODO |
| 7 | 06-sandbox-daemon.md | TODO |
| 8 | 07-vm-lifecycle.md | TODO |
| 9 | 08-firmware.md | TODO |
| 10 | 09-lockfile.md | TODO |
| 11 | 10-cross-cutting.md | TODO |
| 12 | Grandfather review | DONE |
| 13 | Fix findings | DONE (fixed: test count 39→34) |
| 14 | Generate HTML | DONE |

## 6. Build System

```bash
cd src.iii-worker/
python3 build.py .
```

## 7. Quality Requirements

Follow all Iron Rules from the documentation directive. Grandfather review mandatory.

## 8. Expected Outcome

An engineer can understand how iii manages worker VMs, the CLI workflow, and the sandbox architecture.

## 9. Resume Point

Continue writing markdown documents in order. After all documents written, run grandfather review, fix, generate HTML.
