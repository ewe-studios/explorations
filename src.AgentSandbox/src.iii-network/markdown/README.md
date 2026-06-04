---
title: iii-network Documentation
---

# iii-network Documentation

Userspace TCP/IP networking for iii worker VM sandboxes using smoltcp.

## Documents

- [**00 — Overview**](00-overview.md) — What iii-network is, crate structure, key design decisions
- [**01 — Architecture**](01-architecture.md) — Shared memory, device, poll loop
- [**02 — Stack Poll Loop**](02-stack-poll-loop.md) — Frame classification, smoltcp integration
- [**03 — TCP Proxy**](03-tcp-proxy.md) — Guest ↔ host TCP bridging
- [**04 — DNS Interceptor**](04-dns-interceptor.md) — Guest DNS hijack
- [**05 — UDP Relay**](05-udp-relay.md) — Non-DNS UDP outside smoltcp
- [**06 — Cross-Cutting**](06-cross-cutting.md) — Backend, network orchestrator, threading
