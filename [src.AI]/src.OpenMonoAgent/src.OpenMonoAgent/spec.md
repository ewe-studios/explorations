---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.OpenMonoAgent/OpenMonoAgent.ai/
language: C# (.NET 10)
version: (git HEAD)
explored_at: 2026-07-03
license: MIT
---

# spec.md — src.OpenMonoAgent Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.OpenMonoAgent/OpenMonoAgent.ai/`
- **Language:** C# (.NET 10)
- **License:** MIT
- **Projects:** OpenMono.Cli (145 files), OpenMono.Tests (45 files)
- **Total:** 190 C# files

## What src.OpenMonoAgent Is

OpenMono is a .NET 10 CLI agentic coding tool that runs a local conversation loop against a llama.cpp inference server (or any OpenAI-compatible endpoint). It features 20+ built-in tools, MCP tool integration, LSP support, Roslyn code analysis, permission engine, JSONL session management, and TUI rendering.

## Documentation Structure

```
src.OpenMonoAgent/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index
│   ├── 00-overview.md             ← What OpenMono is, architecture
│   ├── 01-cli.md                  ← CLI commands, configuration
│   ├── 02-conversation-loop.md    ← Agent loop, LLM client, streaming
│   ├── 03-tools.md                ← 20+ built-in tools, MCP, LSP, Roslyn
│   ├── 04-permissions.md          ← Permission engine, auto-allow rules
│   ├── 05-session.md              ← JSONL session management, compaction
│   ├── 06-tui.md                  ← Terminal UI, rendering
│   ├── 07-playbooks.md            ← Playbook system, examples
│   └── 08-deployment.md           ← Docker setup, Caddy gateway, services
├── html/
└── build.py
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | README.md (index) | TODO |
| 2 | 00-overview.md | TODO |
| 3 | 01-cli.md | TODO |
| 4 | 02-conversation-loop.md | TODO |
| 5 | 03-tools.md | TODO |
| 6 | 04-permissions.md | TODO |
| 7 | 05-session.md | TODO |
| 8 | 06-tui.md | TODO |
| 9 | 07-playbooks.md | TODO |
| 10 | 08-deployment.md | TODO |
| 11 | HTML generation | TODO |
| 12 | Grandfather review | TODO |

## Build System

```bash
cd "[src.AI]/src.OpenMonoAgent/src.OpenMonoAgent"
python3 build.py src.OpenMonoAgent
```

## Expected Outcome

A developer reading these docs understands OpenMono's architecture, how to configure it, how the agent loop works, and how to extend it with tools and playbooks.
