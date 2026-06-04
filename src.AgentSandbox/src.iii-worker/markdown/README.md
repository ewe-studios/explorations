---
title: iii-worker Documentation
---

# iii-worker Documentation

Managed worker runtime for iii — VM-based sandboxed worker execution.

## Documents

### Foundation

- [**00 — Overview**](00-overview.md) — What iii-worker is, CLI commands, crate structure, dependencies
- [**01 — Architecture**](01-architecture.md) — Layer diagram, component relationships, stdout/stderr contract

### CLI and Operations

- [**02 — CLI Surface**](02-cli-surface.md) — All 17 commands, arguments, source resolution heuristic
- [**03 — Worker Types**](03-worker-types.md) — Registry, OCI, and local workers
- [**04 — Add Pipeline**](04-add-pipeline.md) — Resolve → download → extract → configure → boot
- [**05 — Managed Ops**](05-managed-ops.md) — Binary add, bundle add, local add, restart, sync, verify

### Infrastructure

- [**06 — Sandbox Daemon**](06-sandbox-daemon.md) — VM management, overlay filesystems, exec, FS access
- [**07 — VM Lifecycle**](07-vm-lifecycle.md) — libkrun VM management, OCI adapter, states
- [**08 — Firmware**](08-firmware.md) — libkrunfw download, caching, version resolution
- [**09 — Lockfile**](09-lockfile.md) — Version pinning, iii.lock, drift detection
- [**10 — Cross-Cutting**](10-cross-cutting.md) — Testing, configuration, source watching, shell client
