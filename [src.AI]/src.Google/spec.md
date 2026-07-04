---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Google/
language: Go, TypeScript, Python
explored_at: 2026-07-04
---

# spec.md — src.Google Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Google/`
- **Projects:** 12 (adk-go, adk-go-example, stitch-sdk, stitch-mcp, stitchkit, Centravity, gcsfuse, design.md, generative-ai, agents-cli, knowledge-catalog, llm-wiki.md)
- **Languages:** Go, TypeScript, Python

## Documentation Structure

```
src.Google/
├── spec.md
├── build.py
├── markdown/
│   ├── README.md
│   ├── 00-overview.md          ← Project map + quick summaries
│   ├── 01-adk-go.md            ← Agent interface, LLMAgent, runner, callbacks, agent tree
│   ├── 02-stitch-sdk.md        ← Domain model, MCP transport, schema repair, adapters
│   ├── 03-centravity.md        ← ModelGateway, orchestrator, LWM, SQLite, tools
│   ├── 04-design-md.md         ← Format spec, CLI, validation, export, philosophy
│   ├── 05-gcsfuse.md           ← FUSE adapter, ML profiles, auto-optimization, config
│   └── 06-generative-ai.md     ← 30+ Gemini topics, modalities, workflows
└── html/
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-adk-go.md | DONE |
| 3 | 02-stitch-sdk.md | DONE |
| 4 | 03-centravity.md | DONE |
| 5 | 04-design-md.md | DONE |
| 6 | 05-gcsfuse.md | DONE |
| 7 | 06-generative-ai.md | DONE |
| 8 | HTML generation | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Google
python3 build.py .
```
