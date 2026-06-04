---
title: iii-init Documentation
---

# iii-init Documentation

PID 1 init binary for iii microVM workers — boots the guest environment and supervises the user worker process.

## Documents

### Foundation

- [**00 — Overview**](00-overview.md) — What iii-init is, boot sequence, crate structure, dependencies
- [**01 — Boot Sequence**](01-boot-sequence.md) — Step-by-step PID 1 initialization walkthrough

### Core Systems

- [**02 — Root Pivot**](02-root-pivot.md) — The virtiofs readdir workaround: tmpfs root + bind mounts
- [**03 — Mount Sequence**](03-mount-sequence.md) — Essential Linux filesystem mounts, /proc/meminfo override
- [**04 — Supervisor**](04-supervisor.md) — PID-1 supervision: legacy vs supervisor mode, signal forwarding
- [**05 — Shell Dispatcher**](05-shell-dispatcher.md) — virtio-console shell channel, multiplexed exec sessions
- [**06 — FS Handler**](06-fs-handler.md) — Native filesystem operations: no shell-outs, sync-only
- [**07 — Network**](07-network.md) — Network configuration and DNS setup inside the VM

### Cross-Cutting

- [**08 — Cross-Cutting**](08-cross-cutting.md) — Cross-compilation, testing, parse module
