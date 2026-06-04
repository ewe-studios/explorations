---
title: iii-supervisor Documentation
---

# iii-supervisor Documentation

In-VM process supervisor library — host-driven worker restart via virtio-console control channel.

## Documents

- [**00 — Overview**](00-overview.md) — What iii-supervisor is, crate structure, key constants
- [**01 — Protocol**](01-protocol.md) — Control-channel wire protocol, JSON line format, port discovery
- [**02 — Child Lifecycle**](02-child-lifecycle.md) — Spawn, kill, respawn, process groups, graceful termination
- [**03 — Control Channel**](03-control-channel.md) — Serve loop, request dispatch, on_dispatch hook
- [**04 — Shell Protocol**](04-shell-protocol.md) — Shell protocol re-export from iii-shell-proto
- [**05 — Cross-Cutting**](05-cross-cutting.md) — Testing, mutex poison recovery, regression tests
