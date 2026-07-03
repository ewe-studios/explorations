---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/Instatic/
language: TypeScript
version: 0.0.6 (pre-release)
runtime: Bun
explored_at: 2026-07-03
license: MIT
---

# spec.md — src.Instatic Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/Instatic/`
- **Language:** TypeScript
- **Version:** 0.0.6 (pre-release, no external users)
- **Runtime:** Bun (server + tooling)
- **License:** MIT
- **Package Manager:** Bun (bun.lock, bunfig.toml)
- **Source scale:** 301 core files, 752 admin files, 66 plugin files, 70+ CMS handlers

## What src.Instatic Is

A self-hosted, open-source CMS with a built-in visual editor. One Bun process serves the public website, the admin editor, the CMS API, published pages, and uploaded media, backed by either Postgres or SQLite (selected by `DATABASE_URL`). The output is plain, semantic HTML with hand-clean CSS — no framework runtimes injected into published pages.

Key differentiators:
- **Plugin system with QuickJS-WASM sandbox** — plugins run in isolated VMs with capability-gated SDK access
- **Three-layer publishing pipeline** — static baking (Layer A), in-memory LRU (Layer B), lazy-fetch holes (Layer C)
- **AI integration without SDKs** — drivers talk directly to provider REST APIs over HTTP/SSE
- **React Compiler enabled** — no hand-written useMemo/useCallback/memo
- **TypeBox-only validation** — zod is banned repo-wide
- **In-house routing** — react-router-dom is banned, custom router at src/admin/lib/routing/
- **NodeTree<TNode>** — single tree primitive for pages, visual components, slot fills

## Documentation Goal

A reader should understand:

1. Instatic's architecture: one Bun process, two off-main-thread workers, two database engines
2. The routing table: 19 routes in order, from health check to 404
3. The three-layer publishing pipeline (static bake → LRU cache → lazy holes)
4. Publisher internals: render.ts, renderNode.ts, dynamicDetection.ts, escapeProps, CSS collector
5. The plugin system: QuickJS-WASM sandbox, capability permissions, crash recovery, RPC protocol
6. Plugin host: worker pool, content projection, media, network, schedule, storage handlers
7. Plugin VM: bootstrap, ESM shim, eval, limits, marshal, types
8. The visual editor: canvas, panels, toolbar, Zustand+Mutative store
9. Canvas internals: rendering, dnd, composition, breakpoint frames, live surface
10. The AI runtime: HTTP-only drivers, multi-turn tool loop, no provider SDKs
11. Site agent: agent API, executor, tools, streaming, token runners
12. The content model: data_tables + data_rows, NodeTree, TypeBox
13. Loops, forms, media, templates, site import/export
14. Development setup: Bun, Vite, React Compiler, Playwright E2E

## Documentation Structure

```
src.Instatic/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index / table of contents
│   ├── 00-overview.md             ← What Instatic is, pre-release status, stack overview
│   ├── 01-architecture.md         ← One process, two workers, 19-route dispatcher, database
│   ├── 02-server.md               ← Hand-written router, handlers, repositories
│   ├── 03-publisher.md            ← Three-layer publishing pipeline
│   ├── 04-publisher-deep.md       ← render.ts, renderNode.ts, dynamicDetection deep dive
│   ├── 05-plugin-system.md        ← QuickJS-WASM sandbox, capabilities, lifecycle
│   ├── 06-plugin-deep.md          ← Host, VM, protocol, bootstrap, RPC deep dive
│   ├── 07-visual-editor.md        ← Canvas, panels, store, React Compiler
│   ├── 08-canvas.md               ← Canvas rendering, dnd, composition, breakpoint frames
│   ├── 09-ai-runtime.md           ← HTTP-only drivers, multi-turn tool loop, no SDKs
│   ├── 10-ai-agent.md             ← Site agent, executor, tools, streaming
│   ├── 11-content-model.md        ← data_tables, data_rows, NodeTree, TypeBox
│   ├── 12-loops-forms.md          ← Loops, forms, media, templates, site import
│   ├── 13-deployment.md           ← Docker, Postgres/SQLite, TLS, HA
│   └── 14-cross-cutting.md        ← Banned deps, patterns, testing, benchmarks
├── html/                          ← Generated HTML
│   ├── index.html
│   ├── styles.css
│   └── *.html
└── build.py                       ← Copied from shared build script
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | README.md (index) | DONE |
| 2 | 00-overview.md | DONE |
| 3 | 01-architecture.md | DONE |
| 4 | 02-server.md | DONE |
| 5 | 03-publisher.md | DONE |
| 6 | 04-plugin-system.md | DONE |
| 7 | 05-visual-editor.md | DONE |
| 8 | 06-ai-runtime.md | DONE |
| 9 | 07-content-model.md | DONE |
| 10 | 08-deployment.md | DONE |
| 11 | 09-cross-cutting.md | DONE |
| 12 | 04-publisher-deep.md | DONE |
| 13 | 06-plugin-deep.md | DONE |
| 14 | 08-canvas.md | DONE |
| 15 | 10-ai-agent.md | DONE |
| 16 | 12-loops-forms.md | DONE |
| 17 | HTML regeneration | DONE |
| 18 | Grandfather review | DONE |

## Build System

```bash
cd "[src.AI]/src.Instatic/src.Instatic"
python3 build.py src.Instatic
```

## Quality Requirements

All 10 Iron Rules from the documentation directive apply.

## Expected Outcome

A developer reading these docs can:
- Understand Instatic's architecture at a system level
- Set up local development with Bun
- Add a plugin with proper sandboxing
- Extend the publisher or router
- Deploy with Postgres or SQLite

## Resume Point

If interrupted, check `spec.md` tasks table for DONE/TODO status.
