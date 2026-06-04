---
title: Caching — File Cache, Index Cache, Redis Integration
---

# Caching — File Cache, Index Cache, Redis Integration

**Mirage has a two-tier caching system: file cache for content and index cache for directory listings — both supporting RAM and Redis backends.**

## File Cache

Source: `typescript/packages/core/src/cache/file/`

```mermaid
flowchart TD
    A[readFile request] --> B{In file cache?}
    B -->|Yes| C{ConsistencyPolicy?}
    B -->|No| D[Fetch from resource]
    C -->|lazy| E[Serve cached content]
    C -->|always| F[Fetch fresh, update cache]
    D --> G[Store in cache]
    F --> G
    E --> H[Return content]
    G --> H
```

### Cache Configuration

```typescript
const ws = new Workspace(resources, {
  cache_limit: "512MB",  // LRU eviction when exceeded
  cache: new RAMFileCacheStore({ maxSize: 512 * 1024 * 1024 }),
})
```

| Parameter | Purpose | Default |
|-----------|---------|---------|
| `cache_limit` | Max cache size (string or bytes) | `"512MB"` |
| `cache` | Cache store (RAM or Redis) | RAM |

## Index Cache

Source: `typescript/packages/core/src/cache/index/`

Caches directory listings and metadata — avoids repeated `ListObjectsV2` calls to S3 or API calls to Slack:

```typescript
ws.setIndex(new IndexConfig({
  ttl: 300,  // 5 minute TTL
  store: new RedisIndexCacheStore({ url: 'redis://localhost' }),
}))
```

| Store | Purpose |
|-------|---------|
| `RAMIndexCacheStore` | In-memory index cache |
| `RedisIndexCacheStore` | Redis-backed index cache |

## Redis Integration

Source: `typescript/packages/core/src/cache/file/redis.ts`

For distributed deployments, Mirage supports Redis for both file and index caching:

| Feature | Redis Implementation |
|---------|--------------------|
| File cache | Store file content as Redis keys |
| Index cache | Store directory listings with TTL |
| LRU eviction | Redis maxmemory policy |

## Cache Eviction (LRU)

```mermaid
flowchart TD
    A[New cache entry] --> B{Cache under limit?}
    B -->|Yes| C[Insert entry]
    B -->|No| D[Evict LRU entry]
    D --> E{Cache under limit?}
    E -->|No| D
    E -->|Yes| C
    C --> F[Return content]
```

## Cache Store Backends

```mermaid
flowchart LR
    A[Cache Interface] --> B[RAMFileCacheStore]
    A --> C[RedisFileCacheStore]
    A --> D[RAMIndexCacheStore]
    A --> E[RedisIndexCacheStore]
```

**Aha:** The consistency policy interacts with caching — `lazy` mode serves from cache even if stale, while `always` mode fetches fresh content and updates the cache.

## Cache Keys

Cache keys are derived from the full path:

| Resource | Cache Key |
|----------|-----------|
| RAM | `/data/file.txt` |
| S3 | `/s3/bucket/key → ETag` |
| Slack | `/slack/general/messages.json → timestamp` |

## What's Next

- [10 — FUSE & CLI](10-fuse-cli.md) — FUSE mount, CLI commands
- [08 — Snapshot & Replay](08-snapshot-replay.md) — Return to snapshot
- [02 — Workspace](02-workspace.md) — Return to workspace
