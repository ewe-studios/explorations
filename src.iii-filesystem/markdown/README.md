---
title: iii-filesystem Documentation
---

# iii-filesystem Documentation

Virtual filesystem for iii worker VM sandboxes — exposes host directories to guest VMs via virtio-fs.

## Documents

### Foundation

- [**00 — Overview**](00-overview.md) — What iii-filesystem is, architecture at a glance, key design decisions
- [**01 — Architecture**](01-architecture.md) — FUSE protocol flow, VM integration, inode numbering

### Core Systems

- [**02 — PassthroughFs**](02-passthrough-fs.md) — Core struct, configuration, builder, lifecycle, FUSE operations map
- [**03 — Inode Management**](03-inode-management.md) — Dual-key lookup, Linux lookup collapse, reference counting, procfd reopen
- [**04 — File Operations**](04-file-operations.md) — Open, read, write, flush, release with zero-copy I/O
- [**05 — Directory Operations**](05-directory-operations.md) — Opendir, readdir, readdirplus, tracked leak strategy

### Specialized

- [**06 — Init Binary**](06-init-binary.md) — Virtual /init.krun file, compile-time embedding, zero-copy serving
- [**07 — Platform Abstraction**](07-platform-abstraction.md) — Errno translation, stat helpers, openat2/RESOLVE_BENEATH
- [**08 — Cross-Cutting**](08-cross-cutting.md) — Security, build system, testing, integration with iii-worker
