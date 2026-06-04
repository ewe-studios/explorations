---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.turso/agentfs/
repository: git@github.com:tursodatabase/agentfs.git
explored_at: 2026-06-04
documentation_goal: Document AgentFS — SQLite-backed virtual filesystem for AI agents with FUSE/NFS mounting, OverlayFS, and SDK support.
---

# Spec: AgentFS Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `@formulas/src.rust/src.turso/agentfs/` |
| Language | Rust (SDK + CLI + Sandbox), TypeScript (SDK), Python (SDK) |
| License | MIT |
| LOC (Rust) | 17,489 |
| Rust source files | ~40 |
| SDK languages | TypeScript, Python, Rust |

## 2. What AgentFS Is

AgentFS is a SQLite-backed virtual filesystem designed for AI agent state management. It provides a POSIX-like filesystem, key-value store, and toolcall audit trail — all stored in a single SQLite database file. Mountable via FUSE (Linux) or NFS (macOS), with OverlayFS for copy-on-write isolation.

## 3. Documentation Goal

A reader should understand:
1. The SQLite-backed VFS design: tables, chunks, inodes
2. The syscall interception model (reverie) vs FUSE mounting
3. The OverlayFS copy-on-write implementation
4. The SDK trait: FileSystem with HostFS, AgentFS, OverlayFS
5. The KV store and toolcall audit trail
6. Cross-platform mounting (FUSE vs NFS)

## 4. Documentation Structure

```
src.agentfs/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-sqlite-vfs.md
│   ├── 02-syscall-interception.md
│   ├── 03-overlayfs.md
│   ├── 04-fuse-mount.md
│   ├── 05-sdk.md
│   ├── 06-cross-cutting.md
├── html/
└── build.py
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-sqlite-vfs.md | TODO |
| 3 | 02-syscall-interception.md | TODO |
| 4 | 03-overlayfs.md | TODO |
| 5 | 04-fuse-mount.md | TODO |
| 6 | 05-sdk.md | TODO |
| 7 | 06-cross-cutting.md | TODO |
| 8 | Grandfather review | TODO |
| 9 | Fix findings | TODO |
| 10 | Generate HTML | TODO |

## 6-9. Standard sections

Build via `python3 build.py .`. Grandfather review mandatory.
