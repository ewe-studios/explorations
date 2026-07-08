---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.strukto-ai/mirage/
language: Python, TypeScript
version: 0.0.1alpha
explored_at: 2026-07-08
license: Apache-2.0
repository: github.com/strukto-ai/mirage
---

# spec.md — Mirage VFS Documentation

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.strukto-ai/mirage/`
- **Language:** Python (~2,000 files), TypeScript (~2,500 files)
- **Version:** 0.0.1alpha (Python `__init__.py`), 0.0.2-alpha (README)
- **License:** Apache 2.0
- **Author:** Strukto.ai

## What Mirage Is

Mirage is a Unified Virtual File System (VFS) for AI agents that mounts 50+ heterogeneous services (S3, Google Drive, Slack, Gmail, GitHub, Redis, MongoDB, Postgres, SSH, etc.) as a single filesystem tree. AI agents interact with every backend using standard Unix commands (`cat`, `grep`, `ls`, `find`, `wc`, `jq`). Uses an in-process tree-sitter bash parser (no real shell spawned) for sandboxed execution. Python and TypeScript SDKs provide framework integrations (OpenAI, LangChain, Pydantic AI, CAMEL, Vercel AI SDK, Mastra).

## Documentation Goal

1. Explain the unified VFS architecture and how 50+ services become a single filesystem tree
2. Document the core Python runtime: Workspace, Dispatcher, Mount, Resource, Ops, Cache
3. Deep-dive the in-process shell: tree-sitter-bash parser, AST executor, command resolution
4. Document the command system: polymorphic per-mount/per-filetype command resolution
5. Cover every resource category: object storage, Google Workspace, messaging, databases, cloud files
6. Explain the two-layer cache system (file cache + index cache, RAM + Redis backends)
7. Document snapshot/replay: serialization, drift detection, revision pins
8. Cover FUSE integration, Python/TypeScript SDKs, agent framework integrations
9. Document the TypeScript port: core, browser, node, server, agents packages
10. Cover VFP (Virtual File Protocol), observability, credential management

## Documentation Structure

```
src.strukto-ai/markdown/
├── README.md                          # Index / table of contents
├── 00-overview.md                     # What it is, philosophy, architecture at a glance [DONE]
├── 01-architecture.md                 # Full dependency graph, layers, module map
├── 02-workspace-core.md               # Workspace, Mount, MountRegistry, Dispatcher, Session
├── 03-shell-engine.md                 # tree-sitter-bash parser, AST executor, JobTable
├── 04-command-system.md               # Command registry, polymorphic resolution, per-mount/filetype
├── 05-resource-model.md               # BaseResource, Accessor, Ops, filetype handling
├── 06-cache-system.md                 # File cache (RAM/Redis), Index cache (RAM/Redis), ConsistencyPolicy
├── 07-object-storage.md               # S3, R2, GCS, OCI, Supabase, MinIO, Ceph, Wasabi, etc.
├── 08-google-workspace.md             # Gmail, Drive, Docs, Sheets, Slides
├── 09-messaging-collab.md             # Slack, Discord, Linear, Notion, Trello
├── 10-databases.md                    # MongoDB, Postgres, Redis, SSH
├── 11-snapshot-replay.md              # Serialization, drift detection, revision pins, copy
├── 12-fuse-native.md                  # FUSE adapter, native_exec, mountpoint integration
├── 13-python-sdk.md                   # Workspace API, Ops monkey-patching, Python interop
├── 14-agent-frameworks.md             # OpenAI, LangChain, Pydantic AI, CAMEL, Pydantic DeepAgents
├── 15-typescript-core.md              # TS core: workspace, shell, commands, ops, resources
├── 16-typescript-node-server.md       # Node package (Redis, SSH, disk), Server package (routers, auth)
├── 17-typescript-agents-browser.md    # Agent integrations (OpenAI, Vercel, LangChain, Mastra), Browser package
├── 18-vfp-protocol.md                 # Virtual File Protocol, methods, schema, capability system
├── 19-observability.md                # Observer, recording, history, sessions, prompts
└── 20-credential-auth.md              # Secrets management, accessor patterns, credential loading
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | 00-overview.md (existing, expand) | DONE |
| 2 | 01-architecture.md | TODO |
| 3 | 02-workspace-core.md | TODO |
| 4 | 03-shell-engine.md | TODO |
| 5 | 04-command-system.md | TODO |
| 6 | 05-resource-model.md | TODO |
| 7 | 06-cache-system.md | TODO |
| 8 | 07-object-storage.md | TODO |
| 9 | 08-google-workspace.md | TODO |
| 10 | 09-messaging-collab.md | TODO |
| 11 | 10-databases.md | TODO |
| 12 | 11-snapshot-replay.md | DONE |
| 13 | 12-fuse-native.md | DONE |
| 14 | 13-python-sdk.md | TODO |
| 15 | 14-agent-frameworks.md | TODO |
| 16 | 15-typescript-core.md | TODO |
| 17 | 16-typescript-node-server.md | TODO |
| 18 | 17-typescript-agents-browser.md | TODO |
| 19 | 18-vfp-protocol.md | TODO |
| 20 | 19-observability.md | TODO |
| 21 | 20-credential-auth.md | TODO |
| 22 | Grandfather Review | TODO |
| 23 | Build + verify HTML | TODO |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.strukto-ai/
python3 build.py src.strukto-ai
```

## Quality Requirements

All Iron Rules from documentation_directive.md apply. Minimum 2 mermaid diagrams per document. At least 1 Aha moment per document. Actual source code snippets with file paths and line numbers. Cross-references between documents. Grandfather review mandatory before marking complete.

## Expected Outcome

A reader can understand Mirage's complete architecture, implement a new resource backend, integrate with any supported agent framework, and debug production issues — without reading the source code.

## Resume Point

Start by reading the expanded 00-overview.md, then work through documents in numerical order. The architecture doc (01) is the entry point for understanding module dependencies. Resource backend docs (07-10) are independent and can be read in any order.
