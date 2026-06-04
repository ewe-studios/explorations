---
title: Architecture — Core Abstractions, Dependency Graph
---

# Architecture — Core Abstractions, Dependency Graph

**Mirage is organized around four core abstractions: Workspace, Resource, Mount, and Operation.**

## Core Abstractions

```mermaid
classDiagram
    class Workspace {
        +MountRegistry mounts
        +execute(command)
        +snapshot()
        +load(snapshot)
    }
    class Mount {
        +prefix: string
        +resource: Resource
        +mode: MountMode
        +consistency: ConsistencyPolicy
        +dispatch(op, path)
    }
    class Resource {
        <<interface>>
        +readFile(path)
        +writeFile(path, data)
        +readdir(path)
        +stat(path)
        +find(path, options)
    }
    class OpsRegistry {
        +register(name, fn)
        +dispatch(op, path)
    }

    Workspace --> Mount
    Mount --> Resource
    Mount --> OpsRegistry
```

## Key Types

**Aha:** The `PathSpec` type is the core of path resolution — every filesystem operation starts by parsing a path into a PathSpec, which identifies the mount prefix, the resource-local path, and whether the path crosses mount boundaries. This is how `cp /s3/data /github/repo/data` knows to read from S3 and write to GitHub.

Source: `typescript/packages/core/src/types.ts`

| Type | Purpose |
|------|---------|
| `PathSpec` | Parsed filesystem path with mount resolution |
| `FileStat` | File metadata (size, type, fingerprint, revision) |
| `FileType` | `file`, `directory`, `symlink` |
| `MountMode` | `read`, `write`, `exec` |
| `ConsistencyPolicy` | `lazy` (read-through cache) or `always` (sync every read) |
| `DriftPolicy` | `strict` (error on mismatch) or `off` (skip checks) |
| `ResourceName` | Enum of 30+ supported backends |

## Dependency Flow

```mermaid
flowchart LR
    A[Agent command] --> B[Shell Parser]
    B --> C[Execution Tree]
    C --> D[Dispatcher]
    D --> E{Cross-mount?}
    E -->|Yes| F[Cross-mount handler]
    E -->|No| G[Single-mount dispatch]
    F --> H[Resource ops]
    G --> H
    H --> I[Remote API]
```

## Package Dependencies

| Package | Depends On |
|---------|-----------|
| `@struktoai/mirage-core` | None (base package) |
| `@struktoai/mirage-node` | core, tree-sitter, FUSE bindings |
| `@struktoai/mirage-browser` | core, OPFS API |
| `@struktoai/mirage-server` | core, Express/Fastify |
| `@struktoai/mirage-agents` | core, OpenAI/LangChain/Mastra SDKs |
| `@struktoai/mirage-cli` | core, commander |

## What's Next

- [02 — Workspace](02-workspace.md) — The Workspace class, mount management
- [03 — Resource System](03-resource-system.md) — Resource interface, 30+ backends
- [00 — Overview](00-overview.md) — Return to overview
