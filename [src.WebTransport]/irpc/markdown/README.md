---
title: irpc Documentation — Index
---

# irpc Documentation

> Minimal streaming RPC library for iroh and Quinn QUIC

irpc provides lightweight RPC abstraction over QUIC streams, transparently supporting both local in-process and remote network communication.

## Foundation

- [Overview](00-overview.html) — What irpc is, design goals, interaction patterns
- [Architecture](01-architecture.html) — Layer diagram, module map, feature flags

## Core API

- [Service](02-service.html) — Service trait, RpcMessage, Channels
- [Channels](03-channels.html) — oneshot, mpsc, none channel types
- [Client](04-client.html) — Client API, WithChannels, request handling

## Transport

- [RPC Transport](05-rpc-transport.html) — Quinn/iroh QUIC streams, serialization
- [Local](06-local.html) — In-process transport via mpsc channels

## Derive Macro

- [Derive Macro](07-derive-macro.html) — #[rpc_requests] procedural macro

## Cross-Cutting

- [Data Flow](09-data-flow.html) — Request/response sequences
- [Cross-Cutting](10-cross-cutting.html) — irpc-derive, irpc-iroh, features, error handling

---

Generated from source code.
