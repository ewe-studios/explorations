---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/crates/iii-filesystem/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document the iii-filesystem VFS layer — how it exposes host directories to guest VMs via virtio-fs, with init binary injection, inode tracking, and cross-platform (Linux/macOS) support.
---

# Spec: iii-filesystem Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `iii/crates/iii-filesystem/` |
| Language | Rust (edition 2024) |
| License | Elastic-2.0 |
| LOC | 4,421 |
| Source files | 17 |

## 2. What iii-filesystem Is

iii-filesystem provides filesystem backends for iii worker VM sandboxes. The primary backend is `PassthroughFs`, which exposes a host directory to a guest VM via virtio-fs, with optional init binary injection at `/init.krun`. It implements the `DynFileSystem` trait from `msb_krun`, mapping guest FUSE operations to the host filesystem via syscalls.

## 3. Documentation Goal

A reader should understand:
1. How PassthroughFs maps guest FUSE operations to host syscalls
2. The inode table with dual-key lookup (FUSE inode + host identity)
3. Linux lookup collapse optimization (open + statx = 2 syscalls vs 4)
4. init.krun virtual file injection via memfd/tmpfile
5. Zero-copy I/O via ZeroCopyWriter/ZeroCopyReader
6. Cross-platform differences (Linux vs macOS) and flag translation
7. Security: RESOLVE_BENEATH containment, name validation, symlink rejection
8. Cache policy negotiation (Never/Auto/Always)
9. The leaked readdir buffer strategy with destroy reclamation

## 4. Documentation Structure

```
src.iii-filesystem/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-passthrough-fs.md
│   ├── 03-inode-management.md
│   ├── 04-file-operations.md
│   ├── 05-directory-operations.md
│   ├── 06-init-binary.md
│   ├── 07-platform-abstraction.md
│   ├── 08-cross-cutting.md
├── html/
├── exploration.md
└── build.py
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-architecture.md | DONE |
| 3 | 02-passthrough-fs.md | DONE |
| 4 | 03-inode-management.md | DONE |
| 5 | 04-file-operations.md | DONE |
| 6 | 05-directory-operations.md | DONE |
| 7 | 06-init-binary.md | DONE |
| 8 | 07-platform-abstraction.md | DONE |
| 9 | 08-cross-cutting.md | DONE |
| 10 | Grandfather review | DONE |
| 11 | Fix findings | DONE |
| 12 | Generate HTML | DONE |

## 6. Build System

```bash
cd src.iii-filesystem/
python3 build.py .
```

## 7. Quality Requirements

Follow all Iron Rules from the documentation directive. Grandfather review mandatory.

## 8. Expected Outcome

An engineer can understand how iii exposes host directories to guest VMs via virtio-fs, the security model, cross-platform differences, and the VFS architecture.

## 9. Resume Point

Continue writing markdown documents in order. After all documents written, run grandfather review, fix, generate HTML.
