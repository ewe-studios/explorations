---
title: iroh-blobs Documentation — Index
---

# iroh-blobs Documentation

> BLAKE3-based content-addressed blob transfer scaling from kilobytes to terabytes

## Foundation

- [Overview](00-overview.html) — What iroh-blobs is, BLAKE3 verified streaming, stores
- [Architecture](01-architecture.html) — Full dependency graph, layer diagram, module map

## Protocol and Cryptography

- [Hash and Bao](02-hash-and-bao.html) — BLAKE3 hashing, bao outboards, verified streaming
- [Protocol](03-protocol.html) — Wire format: Get, GetMany, Push, Observe, range specs

## Stores

- [File Store](04-store-fs.html) — FsStore: redb metadata, partial/complete storage, GC
- [Memory Store](05-store-mem.html) — MemStore: in-memory blob storage

## API and Operations

- [API](06-api.html) — Store, Blobs, Tags, Downloader, Remote APIs
- [Get Client](07-get-client.html) — Client FSM states, blob retrieval, verification
- [Provider](08-provider.html) — Server-side: handle connections, send blobs, progress

## Cross-Cutting

- [Data Flow](09-data-flow.html) — End-to-end transfer sequences
- [Cross-Cutting](10-cross-cutting.html) — Tickets, collections, metrics, temp tags

---

Generated from source code. Every claim traces back to implementation.
