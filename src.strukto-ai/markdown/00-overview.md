---
title: Mirage — Unified Virtual File System for AI Agents
---

# Mirage — Unified Virtual File System for AI Agents

**Mirage is a Unified Virtual File System for AI Agents — a single tree that mounts services and data sources like S3, Google Drive, Slack, Gmail, and Redis side-by-side as one filesystem.**

## What It Does

```mermaid
flowchart TB
    subgraph Agent["AI Agent (Claude, GPT, etc.)"]
        A1["bash commands: cat, grep, cp, sed"]
    end

    subgraph Mirage["Mirage Virtual Filesystem"]
        W1["Workspace (single root /)"]
        M1["Mount: /data (RAM)"]
        M2["Mount: /s3 (S3)"]
        M3["Mount: /slack (Slack)"]
        M4["Mount: /github (GitHub)"]
        M5["Mount: /gmail (Gmail)"]
    end

    subgraph Backends["Remote Services"]
        B1["AWS S3"]
        B2["Slack API"]
        B3["GitHub API"]
        B4["Google APIs"]
        B5["Redis"]
    end

    A1 --> W1
    W1 --> M1
    W1 --> M2
    W1 --> M3
    W1 --> M4
    W1 --> M5
    M2 --> B1
    M3 --> B2
    M4 --> B3
    M5 --> B4
    M5 --> B5
```

**Aha:** The key insight is that AI agents already know bash — they don't need to learn a new API per service. Mirage translates a single `grep` command into service-specific operations: for S3 it downloads and searches, for Slack it reads channel messages, for Gmail it searches emails. The agent sees one filesystem underneath.

## Supported Backends (30+)

| Category | Resources |
|----------|-----------|
| Storage | RAM, Disk, S3/R2/GCS/OCI/Supabase, Dropbox, Box |
| Google Workspace | GDrive, GDocs, GSheets, GSlides, Gmail |
| Communication | Slack, Discord, Telegram, Email |
| Development | GitHub, GitHub CI, Linear, Notion, Trello |
| Database | MongoDB, Redis, Postgres |
| Analytics | Langfuse, PostHog, Vercel |
| Research | Semantic Scholar (papers, authors) |
| Browser | OPFS (Origin Private File System) |

## TypeScript SDK Architecture

```mermaid
flowchart TB
    subgraph TS["TypeScript Packages (239,957 LOC)"]
        C1["core (154,277): VFS engine"]
        N1["node (55,888): Node.js runtime"]
        B1["browser (19,133): Browser runtime"]
        S1["server (4,758): HTTP server"]
        A1["agents (3,090): Framework integrations"]
        CL1["cli (2,335): CLI commands"]
    end

    C1 --> N1
    C1 --> B1
    C1 --> S1
    C1 --> A1
    C1 --> CL1
```

## Python SDK Architecture

Source: `python/mirage/` (222,702 LOC)

| Module | Purpose |
|--------|---------|
| `workspace/` | Workspace, Mount, MountRegistry, Dispatcher |
| `resource/` | 30+ resource implementations |
| `commands/` | Built-in commands (cat, grep, sed, awk, etc.) |
| `shell/` | Shell parser, job table |
| `cache/` | File and index caching |
| `ops/` | Operation registry with per-resource overrides |
| `fuse/` | FUSE filesystem mount |
| `cli/` | Command-line interface |

## What's Next

- [01 — Architecture](01-architecture.md) — Full dependency graph, core abstractions
- [02 — Workspace](02-workspace.md) — The Workspace class, mount management
- [03 — Resource System](03-resource-system.md) — Resource interface, 30+ backends
