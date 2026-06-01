# mirage — Spec

## Source Codebase Location

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.strukto-ai/mirage/`
- **Repository:** https://github.com/strukto-ai/mirage.git
- **Languages:** Python (primary), TypeScript
- **Version:** 0.0.2a0 (Python)
- **Author:** Zecheng Zhang (zecheng@strukto.ai)
- **License:** Apache-2.0

## What This Project Is

Mirage is a **Unified Virtual File System for AI Agents**: a single tree that mounts services and data sources like S3, Google Drive, Slack, Gmail, Redis, GitHub, and more side-by-side as one filesystem.

AI agents use familiar Unix-like tools (`cat`, `grep`, `cp`, `ls`) to interact with every backend service. Any LLM that knows bash can use Mirage with zero new vocabulary.

### Key Features

- **One filesystem, every backend** — Mount S3, GDrive, Slack, Gmail, GitHub, MongoDB, Postgres, Redis, SSH, and more under a single root
- **Familiar bash tools** — Agents reuse standard Unix commands across all services
- **Portable workspaces** — Clone, snapshot, and version environments
- **Multi-language SDKs** — Python and TypeScript with framework integrations
- **Works with major frameworks** — OpenAI Agents, LangChain, Pydantic AI, CAMEL, OpenHands

## Documentation Goal

After reading this documentation, an engineer should understand:

1. The virtual file system architecture (VFS → Dispatcher → Resources)
2. How resources are mounted and accessed
3. The command dispatch system (bash → VFS operations)
4. The Python SDK structure and async patterns
5. The TypeScript SDK and browser support
6. The FUSE integration for host filesystem mounting
7. The server mode for remote access
8. How to add new resource types
9. The caching and observation layers
10. Framework integrations (OpenAI, LangChain, etc.)

## Documentation Structure

```
src.strukto-ai/mirage/
├── spec.md                      ← This file
├── exploration.md               ← Original exploration
├── grandfather-review.md        ← Verification report
├── markdown/
│   ├── README.md                ← Index
│   ├── 00-overview.md           ← Philosophy, quick start
│   ├── 01-architecture.md       ← VFS, dispatcher, resources
│   ├── 02-python-sdk.md         ← Python implementation
│   ├── 03-typescript-sdk.md     ← TypeScript implementation
│   ├── 04-resources.md          ← Built-in resource types
│   ├── 05-commands.md           ← Command dispatch
│   ├── 06-fuse-server.md        ← FUSE and server modes
│   ├── 07-frameworks.md           ← Framework integrations
│   └── 08-extending.md            ← Adding new resources
├── html/
└── (uses ../../build.py)
```

## Tasks

| Phase | Document | Status | Notes |
|-------|----------|--------|-------|
| 1 | Read source code | DONE | Via README and exploration |
| 2 | Create spec.md | DONE | This file |
| 3 | Write README.md | DONE | Index with navigation |
| 3 | Write 00-overview.md | DONE | Philosophy, quick start |
| 3 | Write 01-architecture.md | DONE | VFS, dispatcher |
| 3 | Write 02-python-sdk.md | DONE | Python implementation |
| 3 | Write 03-typescript-sdk.md | DONE | TypeScript SDK |
| 3 | Write 04-resources.md | DONE | Built-in resources |
| 3 | Write 05-commands.md | DONE | Command dispatch |
| 3 | Write 06-fuse-server.md | DONE | FUSE and server |
| 3 | Write 07-frameworks.md | DONE | Framework integrations |
| 3 | Write 08-extending.md | DONE | Adding resources |
| 4 | Generate HTML | DONE | All 9 documents generated |
| 5 | Grandfather review | DONE | ✅ All issues fixed — see grandfather-review.md |

## Build System

**Script:** `../../build.py`

```bash
python3 build.py src.strukto-ai/mirage
```

## Quality Requirements

All documents must meet the Iron Rules from the markdown directive.

## Resume Point

Resume from the last uncompleted task in the Tasks table.
