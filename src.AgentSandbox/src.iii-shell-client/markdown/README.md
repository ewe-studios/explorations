---
title: iii-shell-client + iii-shell-proto Documentation
---

# iii-shell-client + iii-shell-proto Documentation

Wire protocol and async client for `iii worker exec` — multiplexed command execution in VM sandboxes.

## Documents

- [**00 — Overview**](00-overview.md) — Wire protocol, shell client, key constants
- [**01 — Wire Protocol**](01-wire-protocol.md) — Frame format, encoding, decoding, multiplexing
- [**02 — Shell Client**](02-shell-client.md) — Async pipe-mode client, OutputSink
- [**03 — Filesystem Ops**](03-filesystem-ops.md) — FsRequest, FsResult, FsEntry, FsMatch
- [**04 — Cross-Cutting**](04-cross-cutting.md) — Base64 encoding, protocol consumers
