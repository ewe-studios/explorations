---
title: iroh Documentation — Index
---

# iroh Documentation

> **less net work for networks**

Iroh gives you an API for dialing by public key. You say "connect to that phone", iroh will find and maintain the fastest connection for you, regardless of where it is.

## Foundation

- [Overview](00-overview.html) — What iroh is, why it exists, architecture at a glance
- [Architecture](01-architecture.html) — Full dependency graph, layer diagram, module map

## Deep Dives

- [Endpoint](02-endpoint.html) — The main connection manager: bind, connect, accept
- [Protocol Dispatch](03-protocol.html) — Router and ProtocolHandler: ALPN-based protocol registration
- [Address Lookup](04-address-lookup.html) — DNS, Pkarr, and Memory address resolution services
- [Network Report](05-net_report.html) — Probes, reports, NAT detection, relay selection
- [TLS Layer](06-tls.html) — Raw public key TLS: RFC 7250, Ed25519 verification
- [Socket Layer](07-socket.html) — Transports, RemoteMap, path selection, hole-punching
- [Relay Server](08-iroh-relay.html) — Relay server and client architecture

## Cross-Cutting

- [Data Flow](09-data-flow.html) — End-to-end flows with sequence diagrams
- [Cross-Cutting Concerns](10-cross-cutting.html) — WASM, portmapper, metrics, runtime, custom transports

---

Generated from source code. Every claim traces back to implementation.
