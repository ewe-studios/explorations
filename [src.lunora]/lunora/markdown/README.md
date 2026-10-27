---
title: lunora Documentation Index
---

# lunora Documentation

> **Type-safe, real-time backend on your own Cloudflare account.** A Durable Object is the database and also the broadcaster.

The set starts with a table, "How to read these docs", in the overview. It tells you which page explains each Cloudflare idea. This set explains how Lunora uses every Cloudflare service, how it shards, how its change log and R2 archive work, and what is good and bad about the design. All claims trace to files in the repository `anolilab/lunora` (commit `50b6008`).

## Foundation

- [Overview](00-overview.md): what Lunora is, packages, request flow
- [Architecture](01-architecture.md): layers, host contracts, code generation
- [Cloudflare services](02-cloudflare-services.md): every service, how and why

## Core Deep Dives

- [Worker runtime](03-worker-runtime.md): request gate, routing, batching
- [Sharding design](04-sharding-design.md): names, registry, fan-out, limits
- [The ShardDO](05-shard-durable-object.md): one shard in detail
- [Change log](06-change-log-cdc.md): `__cdc_log`, epoch, `_commitSeq`, retention
- [Replication and read scaling](07-replication-and-read-scaling.md): replicas, relays, bookmarks, caches
- [Storage tiers and archive](08-storage-tiers-and-archive.md): R2 segments, crash windows
- [D1 and Hyperdrive](09-d1-global-and-hyperdrive.md): global tables

## Cross-Cutting

- [Realtime engine](10-realtime-engine.md): subscriptions, pokes, hibernation
- [Alarms, queues, workflows](11-alarms-queues-workflows.md): background work
- [Tricks and optimizations](12-optimizations-and-tricks.md): 56 entries
- [Migrations and schema](13-migrations-and-schema.md)
- [Testing, observability, deployment](14-testing-observability-deployment.md)

## Verdict

- [Critical assessment](15-critical-assessment.md): ranked strengths, weaknesses, risks
- [Glossary](16-glossary.md)

---

Generated from source code. Every claim traces back to implementation.
