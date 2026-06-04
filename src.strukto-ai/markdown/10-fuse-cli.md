---
title: FUSE & CLI — Real Filesystem Mount, Command-Line Interface
---

# FUSE & CLI — Real Filesystem Mount, Command-Line Interface

**Mirage can mount itself as a real FUSE filesystem on Linux/macOS — giving coding agents direct filesystem access to all mounted resources through familiar bash.**

## FUSE Integration

Source: `python/mirage/fuse/`

```mermaid
flowchart TB
    subgraph Agent["Coding Agent (Claude Code, Codex)"]
        A1["bash: cat /s3/logs/app.json"]
    end

    subgraph FUSE["FUSE Mount"]
        F1["mirage-fuse daemon"]
        F2["Workspace instance"]
    end

    subgraph Backends["Remote Services"]
        B1["S3"]
        B2["Slack"]
        B3["GitHub"]
    end

    A1 --> F1
    F1 --> F2
    F2 --> B1
    F2 --> B2
    F2 --> B3
```

### FUSE Operations

| FUSE Op | Mirage Translation |
|---------|-------------------|
| `open()` | Resource.readFile() |
| `read()` | Stream from resource |
| `write()` | Resource.writeFile() |
| `readdir()` | Resource.readdir() |
| `getattr()` | Resource.stat() |
| `unlink()` | Resource.unlink() |
| `mkdir()` | Resource.mkdir() |

## CLI Commands

Source: `typescript/packages/cli/` and `python/mirage/cli/`

| Command | Purpose |
|---------|---------|
| `mirage init` | Initialize workspace config |
| `mirage start` | Start FUSE daemon |
| `mirage stop` | Stop FUSE daemon |
| `mirage status` | Show FUSE status |
| `mirage execute <cmd>` | Execute command in workspace |
| `mirage snapshot` | Create snapshot |
| `mirage load` | Load snapshot |

## FUSE Operation Flow

```mermaid
sequenceDiagram
    participant Process as Any Process
    participant Kernel as Linux VFS
    participant FUSE as mirage-fuse daemon
    participant WS as Workspace
    participant Resource as Remote Resource

    Process->>Kernel: open("/mnt/mirage/s3/file.txt")
    Kernel->>FUSE: FUSE_OPEN
    FUSE->>WS: resolve path → /s3/file.txt
    WS->>Resource: stat(path)
    Resource-->>WS: FileStat
    WS-->>FUSE: OK
    FUSE-->>Kernel: File handle
    Kernel-->>Process: fd

    Process->>Kernel: read(fd, buf, 4096)
    Kernel->>FUSE: FUSE_READ
    FUSE->>WS: readFile(path, offset, size)
    WS->>Resource: GetObject with range
    Resource-->>WS: bytes
    WS-->>FUSE: IOResult
    FUSE-->>Kernel: Data
    Kernel-->>Process: Data in buf
```

## CLI Architecture

```mermaid
flowchart TD
    A[mirage CLI] --> B[init]
    A --> C[start/stop/status]
    A --> D[execute]
    A --> E[snapshot/load]
    C --> F[FUSE daemon]
    D --> G[Workspace.execute]
    E --> H[Snapshot persistence]
```

**Aha:** When plugged into coding agents like Claude Code, Mirage runs as a FUSE daemon in the background. The agent's `cat`, `grep`, `sed` commands go through the kernel VFS layer into Mirage's FUSE handler, which dispatches to the appropriate resource. The agent has no idea it's talking to S3, Slack, or GitHub — it just sees files.

## Node.js Server

Source: `typescript/packages/server/`

HTTP server that exposes Mirage as a REST API:

| Endpoint | Purpose |
|----------|---------|
| `POST /execute` | Run a command |
| `GET /ls/:path` | List directory |
| `GET /cat/:path` | Read file |
| `POST /write/:path` | Write file |
| `POST /snapshot` | Create snapshot |
| `POST /load` | Load snapshot |

## What's Next

- [11 — Agent Integrations](11-agent-integrations.md) — OpenAI, LangChain, Mastra
- [12 — Python SDK](12-python-sdk.md) — Python API
- [00 — Overview](00-overview.md) — Return to overview
