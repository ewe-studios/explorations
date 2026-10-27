---
location: https://github.com/anolilab/lunora (shallow clone, commit 50b6008)
repository: git@github.com:anolilab/lunora.git
explored_at: 2026-10-27T00:00:00Z
language: TypeScript (pnpm monorepo, 59 packages)
---

# Project Exploration: Lunora, a Cloudflare backend where the Durable Object is the database

## Overview

Lunora is a type-safe, real-time backend for your own Cloudflare account. A Durable Object (a single-threaded Cloudflare object with its own SQLite storage) is the database. The same object holds the WebSocket connections of the clients that read the data. So the writer is also the broadcaster.

Lunora is alpha software. Its license is the Functional Source License identifier FSL-1.1-Apache-2.0. The central class `ShardDO` has 18,454 lines. See [Overview](markdown/00-overview.md).

```mermaid
flowchart LR
    C["Client"] --> W["Worker: gate, route, batch"]
    W --> S["ShardDO: one SQLite database, one writer, sockets"]
    W --> REG["ShardRegistryDO"]
    S --> CDC["__cdc_log"]
    CDC --> R2["R2: cdc segments"]
    S --> D1["D1 or Hyperdrive: global tables"]
    S --> Q["Queues, Workflows, Scheduler"]
```

## Cloudflare services used

Durable Objects (with SQLite and WebSocket hibernation) and Workers form the core. Lunora also uses D1, Hyperdrive, R2 and Workers KV (a key-value store). It uses Queues, Workflows, Cron Triggers and Analytics Engine. It uses Pipelines with R2 SQL (Structured Query Language), Vectorize, Workers AI (artificial intelligence) with AI Gateway, and service bindings. The page [Cloudflare services](markdown/02-cloudflare-services.md) has one section for each.

## Sharding in one minute

The shard is a Durable Object chosen by name. A table marked `.shardBy(field)` lives in the object named by that field value. Other tables live in the root shard `__root__`. A single `ShardRegistryDO` lists the shards for fan-out. The caller supplies the shard key, and I found no server check that it matches the row. There is no resharding, only a manual export and import. See [Sharding design](markdown/04-sharding-design.md).

## Change log and archive

Every shard keeps `__cdc_log` with a sequence number (`_commitSeq`) and an epoch. A sweep copies old rows into R2 as `cdc/{shard}/{epoch}/{16-digit last seq}.json`, advances a watermark, and only then deletes. A restore is detected as a timeline fork. See [Change log](markdown/06-change-log-cdc.md) and [Storage tiers](markdown/08-storage-tiers-and-archive.md).

```mermaid
sequenceDiagram
    participant S as Sweep
    participant R as R2
    participant W as Watermark
    participant L as Log
    S->>R: put segment
    R-->>S: resolved
    S->>W: advance
    S->>L: trim up to watermark
```

## Concepts explained for a reader who does not know Cloudflare

The pages explain each Cloudflare idea at the place that owns it. The overview has a table that lists where each concept lives.

| Concept | Where |
|---------|-------|
| Durable Object: actor model, gates, `blockConcurrencyWhile`, SQLite transactions, alarms, WebSocket hibernation, point-in-time recovery, location hints, jurisdictions, why it is a natural shard | [Cloudflare services](markdown/02-cloudflare-services.md) |
| D1 read replicas, Sessions application programming interface (API), bookmark, `first-unconstrained`, `first-primary`, read-your-writes | [D1 and global tables](markdown/09-d1-global-and-hyperdrive.md) |
| Consistency words: monotonic reads, eventual consistency, replication lag, watermark, epoch, commit sequence, idempotency, outbox, fork after restore | [Replication and read scaling](markdown/07-replication-and-read-scaling.md) |
| Workers isolates and limits, R2, Workers KV, Queues, Workflows, Cron Triggers, Analytics Engine, Hyperdrive, Vectorize, Workers AI, Pipelines, service bindings | [Cloudflare services](markdown/02-cloudflare-services.md) |

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Shard (D1 session per request)
    participant D as D1
    C->>S: write a global row
    S->>D: write, then read session.getBookmark()
    S-->>C: response with header x-d1-bookmark
    C->>S: later read with x-d1-bookmark
    S->>D: withSession(bookmark): a replica waits until it reaches the bookmark
    D-->>S: rows that include the earlier write
```

## Document map

| Topic | Document |
|-------|----------|
| Overview | [00](markdown/00-overview.md) |
| Architecture | [01](markdown/01-architecture.md) |
| Cloudflare services | [02](markdown/02-cloudflare-services.md) |
| Worker runtime | [03](markdown/03-worker-runtime.md) |
| Sharding | [04](markdown/04-sharding-design.md) |
| ShardDO | [05](markdown/05-shard-durable-object.md) |
| Change log | [06](markdown/06-change-log-cdc.md) |
| Replication and read scaling | [07](markdown/07-replication-and-read-scaling.md) |
| Storage tiers and archive | [08](markdown/08-storage-tiers-and-archive.md) |
| D1 and Hyperdrive | [09](markdown/09-d1-global-and-hyperdrive.md) |
| Realtime | [10](markdown/10-realtime-engine.md) |
| Alarms, queues, workflows | [11](markdown/11-alarms-queues-workflows.md) |
| 56 tricks | [12](markdown/12-optimizations-and-tricks.md) |
| Migrations | [13](markdown/13-migrations-and-schema.md) |
| Testing and deployment | [14](markdown/14-testing-observability-deployment.md) |
| Critical assessment | [15](markdown/15-critical-assessment.md) |
| Glossary | [16](markdown/16-glossary.md) |

## The five most important tricks

1. Archive first, destroy second, with a watermark in SQLite and the epoch in the R2 prefix.
2. A gate wrapper that never rejects, because a rejection from `blockConcurrencyWhile` resets the object.
3. A schedule outbox written inside the transaction, so a crash cannot lose a deferred job.
4. Fork detection with outside witnesses (consumer, replica, archive), because SQLite cannot report a restore.
5. Range-gated invalidation with a per-flush diff cache for realtime fan-out.

## Verdict

Strong core idea, careful handling of Cloudflare edge cases. The weak points sit outside one object. They are caller-chosen shard keys, no resharding, single registry and scheduler objects, and a log archive ceiling of about 166 changes per second per shard. The default request log adds write amplification. **Aha: the best parts (gate, outbox, archive) all come from treating the platform's failure modes as first-class.** See [Critical assessment](markdown/15-critical-assessment.md).

## Differences between code and docs

- The code has 59 packages. `AGENTS.md` says 58.
- The docs say routing is by schema. In the code the client passes `shardKey`.
- The docs say actions run in the Worker. In the code they run on the shard.
- The sink comment says the Analytics Engine blob limit is 5,120 bytes. The writer in `bindings` uses 16 kilobytes (KB), which matches the Cloudflare limit.
- The queue delay check is 12 hours. The code comment calls it the Cloudflare ceiling. Cloudflare lists 24 hours.
- `architecture.mdx` says D1 replicas converge within about 24 hours. The Cloudflare D1 docs give no such bound.
- `limits.mdx` says environment variables share 5 KB in total. Cloudflare limits each variable to 5 KB.

## Grandfather review

Done on 2026-10-27. Every `file:line` citation in chapters 00 to 15 was printed by a script and read against the claim. Several wrong line numbers and facts were fixed (for example 34 workerd tests, not 37; `platform-node` uses `better-sqlite3`). Numbers were cross-checked with grep. Mermaid count is at least 2 in every chapter. Abbreviations are spelled out.

Round 2 (concept explanations) added the sections that explain Cloudflare ideas. Each Cloudflare claim names the file of `cloudflare/cloudflare-docs` (commit `3edd18a`, 2026-10-27) that supports it. Each new Lunora name and `file:line` citation was checked against the clone at commit `50b6008`. Mermaid diagrams were parsed with Mermaid 11. Wrong or unsupported statements that round 2 found are listed in "Differences between code and docs" above. The Cloudflare Pipelines, R2 SQL and R2 Data Catalog products are now named Basin Pipelines, Basin SQL and Basin Catalog in the Cloudflare docs.

Not verified:

- Nothing was run. The clone was treated as untrusted, so all claims come from reading code.
- Row-write amplification per request (chapter 12, entry 56) is an inference, not a measurement.
- Cloudflare limits and behavior were checked against a shallow clone of `cloudflare/cloudflare-docs` (round 2). Not found there: the compound `SELECT` limit of 5 terms, and the `startAfter` option of the R2 binding `list` call. See [Cloudflare services](markdown/02-cloudflare-services.md) and [Storage tiers](markdown/08-storage-tiers-and-archive.md).
- The Node and `celld` host rows of the capability matrix were counted from the matrix, not tested.
- Chapter 16 (glossary) citations were checked less closely than chapters 00 to 15.
- The limits that Lunora's `limits.mdx` quotes (for example 50-byte `LIKE` patterns on D1) were checked only where the Cloudflare docs list them.
