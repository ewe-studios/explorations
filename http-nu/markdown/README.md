# http-nu Cloudflare Workers Port — Documentation Index

## Foundation

| Document | Description |
|----------|-------------|
| [CF Overview](00-cf-overview.md) | What the CF port is, why it exists, architecture at a glance |
| [CF Architecture](01-cf-architecture.md) | Module map, DurableObject routing, data flow diagrams |

## Module Deep Dives

| Document | Description |
|----------|-------------|
| [Vfs Abstraction](02-vfs.md) | Vfs trait, OsVfs desktop impl, thread-local dispatch |
| [SnapshotVfs](03-snapshot-vfs.md) | CF Workspace-backed Vfs, preload/drain lifecycle |
| [Shadow Commands](04-shadow-commands.md) | Layer 1/2/3 strategy, all 11 shadowed commands |
| [CF Request Lifecycle](05-cf-request-lifecycle.md) | Per-request pipeline: preload → eval → drain → persist |

## Cross-Cutting

| Document | Description |
|----------|-------------|
| [Desktop vs CF](06-desktop-vs-cf.md) | Feature comparison matrix, what works/differs |
