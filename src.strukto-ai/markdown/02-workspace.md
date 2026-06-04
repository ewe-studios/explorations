---
title: Workspace — Single Root, Heterogeneous Mounts
---

# Workspace — Single Root, Heterogeneous Mounts

**The Workspace is the central abstraction — a single root filesystem with heterogeneous mounts, each backed by a different Resource.**

## Workspace Class

Source: `typescript/packages/core/src/workspace/workspace.ts`

```typescript
const ws = new Workspace({
  '/data': new RAMResource(),
  '/s3': new S3Resource({ bucket: 'logs' }),
  '/slack': new SlackResource({}),
  '/github': new GitHubResource({}),
})
```

### Constructor Parameters

| Parameter | Purpose | Default |
|-----------|---------|---------|
| `resources` | Dict of mount path → Resource | Required |
| `cache_limit` | File cache size limit | `"512MB"` |
| `cache` | Cache config (RAM or Redis) | RAM |
| `consistency` | Default consistency policy | `lazy` |
| `drift` | Drift detection policy | `off` |

## Execution Flow

```mermaid
sequenceDiagram
    participant Agent as AI Agent
    participant WS as Workspace
    participant Parser as Shell Parser
    participant Executor as Execution Engine
    participant Mount as Mount
    participant Resource as Resource

    Agent->>WS: execute("cat /s3/logs/app.json")
    WS->>Parser: parse(command)
    Parser-->>WS: Execution Tree
    WS->>Executor: run_command_tree(tree)
    Executor->>Mount: resolve path → /s3
    Mount->>Resource: readFile(path)
    Resource-->>Mount: Uint8Array
    Mount-->>Executor: IOResult
    Executor-->>WS: output + exit code
    WS-->>Agent: stdout text
```

## Session Flow

```mermaid
sequenceDiagram
    participant A1 as Agent 1
    participant A2 as Agent 2
    participant SM as SessionManager
    participant WS as Workspace

    A1->>SM: create_session(agent_id="agent-a")
    SM-->>A1: session_id="sess-1"
    A2->>SM: create_session(agent_id="agent-b")
    SM-->>A2: session_id="sess-2"
    A1->>WS: execute("cat /data/file.txt", session="sess-1")
    WS-->>A1: output (recorded in sess-1 history)
    A2->>WS: execute("ls /s3/", session="sess-2")
    WS-->>A2: output (recorded in sess-2 history)
```

**Aha:** Sessions enable multiple agents to share the same Workspace without interfering with each other's command history or state.

## Mount Registry

Source: `typescript/packages/core/src/workspace/mount/registry.ts`

The `MountRegistry` manages all mounts in the workspace:

| Method | Purpose |
|--------|---------|
| `register(mount)` | Add a mount to the workspace |
| `mountFor(path)` | Find the mount that owns a path |
| `list()` | List all mounts |
| `unregister(prefix)` | Remove a mount |

Path resolution walks mounts longest-prefix-first — `/s3/logs/app.json` matches `/s3` before `/`.

## What's Next

- [03 — Resource System](03-resource-system.md) — Resource interface, 30+ backends
- [04 — Mount System](04-mount-system.md) — Per-mount commands, ops, policies
- [01 — Architecture](01-architecture.md) — Return to architecture
