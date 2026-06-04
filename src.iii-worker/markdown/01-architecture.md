---
title: Architecture — Dependency Graph, Layers, Component Relationships
---

# Architecture — Dependency Graph, Layers, Component Relationships

**iii-worker is organized as three logical layers: CLI surface, core operations, and VM infrastructure.** This document covers the full dependency graph and component relationships.

## Layer Diagram

```mermaid
graph TB
    subgraph Layer1["Layer 1: CLI Surface (cli/)"]
        A1["app.rs: Cli, Commands enum"]
        A2["managed.rs: 6,469 lines"]
        A3["status.rs: 1,697 lines"]
        A4["sandbox.rs: 749 lines"]
        A5["init.rs: 557 lines"]
    end

    subgraph Layer2["Layer 2: Core Operations (core/)"]
        B1["add.rs: worker installation"]
        B2["remove.rs: config removal"]
        B3["start/stop/list.rs"]
        B4["update/sync/verify/clear.rs"]
        B5["types.rs: WorkerSource, AddOptions"]
        B6["error.rs: WorkerOpError"]
        B7["project.rs: ProjectCtx"]
        B8["events.rs: EventSink"]
    end

    subgraph Layer3["Layer 3: VM Infrastructure"]
        C1["worker_manager/libkrun.rs: 999 lines"]
        C2["worker_manager/oci.rs: 1,490 lines"]
        C3["vm_boot.rs: 1,199 lines"]
        C4["sandbox_daemon/: 7,595 lines"]
        C5["firmware/: libkrunfw download"]
    end

    subgraph Layer4["Layer 4: External"]
        D1["Workers Registry API"]
        D2["OCI Registries"]
        D3["iii Engine (WebSocket)"]
        D4["iii-supervisor (process supervisor)"]
    end

    A1 --> A2
    A2 --> B1
    B1 --> C1
    B1 --> C2
    B1 --> C4
    B1 --> D1
    C1 --> D4
    C2 --> D2
    C4 --> D3
```

## Component Relationships

```mermaid
flowchart LR
    subgraph Add["add Pipeline"]
        A1["resolve source"] --> A2["download"]
        A2 --> A3["extract"]
        A3 --> A4["configure"]
        A4 --> A5["boot VM"]
    end

    subgraph VM["VM Infrastructure"]
        V1["libkrun VM"] --> V2["iii-filesystem VFS"]
        V1 --> V3["iii-network stack"]
        V1 --> V4["firmware (libkrunfw)"]
    end

    subgraph Sandbox["Sandbox Daemon"]
        S1["create VM"] --> S2["overlay mount"]
        S2 --> S3["exec commands"]
        S3 --> S4["fs access"]
        S4 --> S5["reap stopped"]
    end

    Add --> VM
    Add --> Sandbox
```

## CLI → Core → Infrastructure Mapping

| CLI Command | Core Module | Infrastructure |
|-------------|------------|----------------|
| `iii worker add` | `core::add::run` | registry + worker_manager + sandbox_daemon |
| `iii worker remove` | `core::remove::run` | config_file + engine |
| `iii worker start` | `core::start::run` | worker_manager + sandbox_daemon |
| `iii worker stop` | `core::stop::run` | sandbox_daemon |
| `iii worker list` | `core::list::run` | config_file + sandbox_daemon |
| `iii worker exec` | — | shell_client + shell_relay |
| `iii worker status` | — | sandbox_daemon |
| `iii worker sync` | — | lockfile + registry |
| `iii worker update` | `core::update::run` | lockfile + registry |
| `iii worker clear` | `core::clear::run` | filesystem |
| `iii worker reinstall` | `core::add::run` (--force) | registry + worker_manager |
| `iii worker verify` | — | lockfile + filesystem |

## Stdout/Stderr Contract

Source: `cli/managed.rs:9-24`

**Aha:** Every managed command follows a strict contract: stdout contains ONLY machine-readable output (the worker name on success), and stderr contains ALL human-facing output (progress, status, errors). This means scripts can pipe `iii worker start foo` and get just the worker name, while users see rich progress on stderr.

```
stdout: worker_name\n
stderr: • downloading pdfkit
stderr:   ✓ resolved to binary v1.2.3
stderr:   ✓ ready in 3.2s
```

## What's Next

- [02 — CLI Surface](02-cli-surface.md) — All commands, arguments, and the stdout/stderr contract
- [03 — Worker Types](03-worker-types.md) — Registry, OCI, and local workers
- [04 — Add Pipeline](04-add-pipeline.md) — The add flow: resolve → download → extract → configure → boot
