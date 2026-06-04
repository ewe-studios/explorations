---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/crates/iii-network/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document iii-network — userspace TCP/IP networking for iii worker VM sandboxes using smoltcp.
---

# Spec: iii-network Documentation

## 1. Source

| Property | Value |
|----------|-------|
| Location | `iii/crates/iii-network/` |
| Language | Rust (edition 2024) |
| License | Elastic-2.0 |
| LOC | 2,661 |
| Source files | 12 |

## 2. What iii-network Is

iii-network provides a userspace TCP/IP stack for iii worker VM sandboxes using smoltcp. It bridges guest ethernet frames from libkrun's NetWorker thread through a shared-memory queue to a smoltcp poll thread, which services TCP connections via tokio proxy tasks.

## 3-9. Standard sections

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-architecture.md | TODO |
| 3 | 02-stack-poll-loop.md | TODO |
| 4 | 03-tcp-proxy.md | TODO |
| 5 | 04-dns-interceptor.md | TODO |
| 6 | 05-udp-relay.md | TODO |
| 7 | 06-cross-cutting.md | TODO |
| 8 | Grandfather review | DONE |
| 9 | Fix findings | DONE (no discrepancies — all verified) |
| 10 | Generate HTML | DONE |

Build via `python3 build.py .`. Grandfather review mandatory.
