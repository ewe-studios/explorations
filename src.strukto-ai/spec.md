---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.strukto-ai/mirage/
repository: github.com/strukto-ai/mirage
explored_at: 2026-06-04
documentation_goal: Document Mirage — a Unified Virtual File System for AI Agents with 30+ service mounts, Python+TS SDKs, and bash-level agent interaction.
---

# Spec: Mirage Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `src.strukto-ai/mirage/` |
| Languages | TypeScript (239,957 LOC), Python (222,702 LOC) |
| License | Apache-2.0 |
| Total LOC | 462,659 |

### TypeScript Packages (239,957 LOC)

| Package | LOC | Purpose |
|---------|-----|---------|
| `core` | 154,277 | Core VFS engine — Workspace, Mount, Resource, shell parser |
| `node` | 55,888 | Node.js runtime — CLI, server, FUSE mount |
| `browser` | 19,133 | Browser runtime — OPFS, web workers |
| `server` | 4,758 | HTTP server for agent workspace |
| `agents` | 3,090 | Framework integrations (OpenAI, LangChain, Mastra) |
| `cli` | 2,335 | CLI commands |

### Python Package (222,702 LOC)

| Module | LOC | Purpose |
|--------|-----|---------|
| `core/` | ~50,000 | Core resources (S3, GDrive, Slack, GitHub, etc.) |
| `workspace/` | ~40,000 | Workspace, mount, executor, dispatcher |
| `commands/` | ~30,000 | Built-in commands (cat, grep, sed, awk, etc.) |
| `resource/` | ~25,000 | Resource implementations |
| `shell/` | ~15,000 | Shell parser, job table |
| `cache/` | ~15,000 | File and index caching |
| `fuse/` | ~5,000 | FUSE filesystem mount |

## 2. What Mirage Is

Mirage is a **Unified Virtual File System for AI Agents** — a single tree that mounts services and data sources like S3, Google Drive, Slack, Gmail, and Redis side-by-side as one filesystem. AI agents reach every backend with the same handful of Unix-like tools, and pipelines compose across services as naturally as on a local disk.

## 3. Documentation Goal

A reader should understand:
1. The Workspace abstraction — single root with heterogeneous mounts
2. The Resource interface — how 30+ backends speak filesystem semantics
3. The Mount system — per-mount commands, ops, consistency policies
4. The shell parser — tree-sitter based bash parsing
5. Cross-mount operations — cp, mv, diff across different backends
6. The ops registry — per-resource, per-filetype command overrides
7. Snapshot & replay — workspace serialization with drift detection
8. The caching system — file cache, index cache, Redis integration
9. FUSE integration — mounting Mirage as a real filesystem
10. Agent framework integrations — OpenAI, LangChain, Mastra

## 4. Documentation Structure

```
src.strukto-ai/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-workspace.md
│   ├── 03-resource-system.md
│   ├── 04-mount-system.md
│   ├── 05-shell-parser.md
│   ├── 06-ops-commands.md
│   ├── 07-cross-mount.md
│   ├── 08-snapshot-replay.md
│   ├── 09-caching.md
│   ├── 10-fuse-cli.md
│   ├── 11-agent-integrations.md
│   ├── 12-python-sdk.md
│   ├── 13-cross-cutting.md
├── html/
└── build.py
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-architecture.md | TODO |
| 3 | 02-workspace.md | TODO |
| 4 | 03-resource-system.md | TODO |
| 5 | 04-mount-system.md | TODO |
| 6 | 05-shell-parser.md | TODO |
| 7 | 06-ops-commands.md | TODO |
| 8 | 07-cross-mount.md | TODO |
| 9 | 08-snapshot-replay.md | TODO |
| 10 | 09-caching.md | TODO |
| 11 | 10-fuse-cli.md | TODO |
| 12 | 11-agent-integrations.md | TODO |
| 13 | 12-python-sdk.md | TODO |
| 14 | 13-cross-cutting.md | TODO |
| 15 | Grandfather review | TODO |
| 16 | Fix findings | TODO |
| 17 | Generate HTML | TODO |

## 6-9. Standard sections

Build via `python3 build.py .`. Grandfather review mandatory.
