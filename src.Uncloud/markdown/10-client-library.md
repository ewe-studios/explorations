---
title: Client Library — pkg/client API
---

# Client Library — pkg/client API

**The client library (`pkg/client`) is the Go SDK for interacting with Uncloud — used by the CLI and available for programmatic access.**

## Architecture

Source: `pkg/client/` (15,442 LOC)

```mermaid
flowchart TB
    subgraph Client["pkg/client"]
        C1["client.go: base client"]
        C2["service.go: service operations"]
        C3["container.go: container operations"]
        C4["machine.go: machine operations"]
        C5["volume.go: volume operations"]
        C6["image.go: image operations"]
        C7["dns.go: DNS operations"]
        C8["caddy.go: Caddy operations"]
        C9["logs.go: log streaming"]
        C10["deploy/: deployment engine"]
    end

    subgraph API["Remote Machine API"]
        A1["gRPC services"]
    end

    C1 --> A1
    C2 --> A1
    C3 --> A1
    C4 --> A1
    C5 --> A1
    C6 --> A1
    C7 --> A1
    C8 --> A1
    C9 --> A1
    C10 --> A1
```

## Package Breakdown

| File | LOC | Purpose |
|------|-----|---------|
| `container.go` | 537 | Create, start, stop, remove containers |
| `image.go` | 672 | List, pull, push, remove images |
| `logmerger.go` | 317 | Merge logs from multiple containers |
| `logs.go` | 269 | Stream container logs |
| `service.go` | 386 | Deploy, update, remove services |
| `dns.go` | 228 | DNS record management |
| `volume.go` | 125 | Volume management |
| `machine.go` | 149 | Machine join/leave/list |
| `caddy.go` | 150 | Caddy config management |
| `client.go` | 98 | Base client with connection handling |
| `user.go` | 50 | User management |

## Deployment Engine

Source: `pkg/client/deploy/`

| File | Purpose |
|------|---------|
| `deploy.go` | Top-level deploy entry point |
| `container.go` | Container creation/update logic |
| `strategy.go` | Deployment strategies (rolling, start-first) |
| `resolver.go` | Placement constraint evaluation |
| `scheduler/` | Machine selection and scheduling |
| `operation/` | Deployment operation tracking |

## Log Merging

**Aha:** The client library is 15,442 LOC — larger than many complete applications. This reflects Uncloud's comprehensive API surface: every machine, service, container, volume, image, log, DNS, and Caddy operation is covered.

Source: `pkg/client/logmerger.go` (317 lines)

## Deployment Engine Flow

```mermaid
sequenceDiagram
    participant CLI as Deploy Command
    participant Client as pkg/client
    participant Resolver as Placement Resolver
    participant Scheduler as Scheduler
    participant Machine as Remote Machine

    CLI->>Client: Deploy(composeSpec)
    Client->>Resolver: Evaluate placement constraints
    Resolver-->>Client: Eligible machines
    Client->>Scheduler: Select machines by strategy
    Scheduler-->>Client: Assigned machines
    Client->>Machine: Create/update containers
    Machine-->>Client: Deployment complete
```

Merges logs from multiple containers into a single time-ordered stream — essential for multi-replica service debugging.

**Aha:** The client library is 15,442 LOC — larger than many complete applications. This reflects Uncloud's comprehensive API surface: every machine, service, container, volume, image, log, DNS, and Caddy operation is covered.

## What's Next

- [11 — Cross-Cutting](11-cross-cutting.md) — Testing, metrics, SSH exec
- [06 — CLI](06-cli.md) — Return to CLI
- [00 — Overview](00-overview.md) — Return to overview
