---
title: Resource System — 30+ Backend Implementations
---

# Resource System — 30+ Backend Implementations

**The Resource interface is the contract that all 30+ backends implement — translating filesystem operations into service-specific API calls.**

## Resource Interface

Source: `typescript/packages/core/src/resource/base.ts`

```typescript
interface Resource {
  readonly kind: string
  readonly isRemote?: boolean
  readonly supportsSnapshot?: boolean
  readonly accessor?: Accessor
  readonly index?: IndexCacheStore

  open(): Promise<void>
  close(): Promise<void>

  readFile(path: PathSpec): Promise<Uint8Array>
  writeFile(path: PathSpec, data: Uint8Array): Promise<void>
  readdir(path: PathSpec): Promise<string[]>
  stat(path: PathSpec): Promise<FileStat>
  exists(path: PathSpec): Promise<boolean>
  mkdir(path: PathSpec): Promise<void>
  unlink(path: PathSpec): Promise<void>
  rename(src: PathSpec, dst: PathSpec): Promise<void>
  find(path: PathSpec, options?: FindOptions): Promise<string[]>
  // ... 15+ more methods
}
```

## Resource Categories

```mermaid
flowchart TD
    A[Resource interface] --> B[RAMResource]
    A --> C[DiskResource]
    A --> D[CloudStorage]
    A --> E[Communication]
    A --> F[Development]
    A --> G[Database]
    A --> H[Analytics]

    D --> D1[S3Resource]
    D --> D2[GCSResource]
    D --> D3[R2Resource]
    D --> D4[OCIResource]

    E --> E1[SlackResource]
    E --> E2[DiscordResource]
    E --> E3[GmailResource]

    F --> F1[GitHubResource]
    F --> F2[LinearResource]
    F --> F3[NotionResource]

    G --> G1[MongoDBResource]
    G --> G2[RedisResource]
```

## Resource Implementations by Backend

```mermaid
flowchart LR
    A[BaseResource] --> B[readFile/writeFile]
    A --> C[readdir/stat]
    A --> D[find/glob]
    B --> E[S3: GetObject/PutObject]
    B --> F[Slack: API calls]
    B --> G[GitHub: REST API]
    C --> H[MongoDB: find/insert]
    C --> I[Redis: get/set]
```

## RAMResource

Source: `typescript/packages/core/src/resource/ram/ram.ts`

In-memory filesystem — the simplest resource implementation:

| Method | Implementation |
|--------|---------------|
| `readFile` | Read from in-memory Uint8Array map |
| `writeFile` | Write to in-memory map |
| `readdir` | List keys under prefix |
| `stat` | Return size, type, mtime from metadata |

## S3Resource

Source: `typescript/packages/core/src/resource/s3/`

AWS S3 (and compatible: R2, GCS, OCI, Supabase):

| Operation | S3 API |
|-----------|--------|
| `readFile` | `GetObject` |
| `writeFile` | `PutObject` |
| `readdir` | `ListObjectsV2` with prefix |
| `stat` | `HeadObject` |
| `unlink` | `DeleteObject` |
| `mkdir` | No-op (S3 has no directories) |
| `find` | `ListObjectsV2` with filter |

## SlackResource

Source: `typescript/packages/core/src/resource/slack/`

Slack as a filesystem:

| Path Pattern | Meaning |
|-------------|---------|
| `/channels/<name>` | Channel directory |
| `/channels/<name>/messages.json` | Channel messages |
| `/users` | User list |
| `/files/<id>` | File content |

**Aha:** The `supportsSnapshot` flag on Resource enables workspace snapshot/replay with drift detection. Remote resources like S3 populate `FileStat.fingerprint` (ETag for S3, last_modified for Slack) so replay can detect if the remote file changed since the snapshot was taken.

## What's Next

- [04 — Mount System](04-mount-system.md) — Per-mount commands, ops, policies
- [06 — Ops & Commands](06-ops-commands.md) — Operation registry, command overrides
- [02 — Workspace](02-workspace.md) — Return to workspace
