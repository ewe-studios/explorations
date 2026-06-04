---
title: AgentFS Documentation
---

# AgentFS Documentation

SQLite-backed virtual filesystem for AI agent state management.

## Documents

- [**00 — Overview**](00-overview.md) — What AgentFS is, crate structure, key design decisions
- [**01 — SQLite VFS**](01-sqlite-vfs.md) — Tables, 4KB chunks, inode mapping
- [**02 — Syscall Interception**](02-syscall-interception.md) — FUSE mount, reverie sandbox, mount table
- [**03 — VFS Setup Comparison**](03-vfs-setup-comparison.md) — iii-filesystem vs AgentFS: how they set up the VFS
- [**04 — OverlayFS**](04-overlayfs.md) — Copy-on-write implementation
- [**05 — SDK**](05-sdk.md) — TypeScript, Python, Rust SDKs
- [**06 — Cross-Cutting**](06-cross-cutting.md) — Mount strategies, dependencies, testing
