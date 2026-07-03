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

## What src.Instatic Is

A self-hosted, open-source CMS with a built-in visual editor. One Bun process serves the public website, the admin editor, the CMS API, published pages, and uploaded media, backed by either Postgres or SQLite (selected by `DATABASE_URL`). The output is plain, semantic HTML with hand-clean CSS — no framework runtimes injected into published pages.

Key differentiators:
- **Plugin system with QuickJS-WASM sandbox** — plugins run in isolated VMs with capability-gated SDK access
- **Three-layer publishing pipeline** — static baking (Layer A), in-memory LRU (Layer B), lazy-fetch holes (Layer C)
- **AI integration without SDKs** — drivers talk directly to provider REST APIs over HTTP/SSE
- **React Compiler enabled** — no hand-written useMemo/useCallback/memo
- **TypeBox-only validation** — zod is banned repo-wide
- **In-house routing** — react-router-dom is banned, custom router at src/admin/lib/routing/

## Documentation Goal

A reader should understand:

1. Instatic's architecture: one Bun process, two off-main-thread workers, two database engines
2. The routing table: 15 routes in order, from health check to 404
3. The three-layer publishing pipeline (static bake → LRU cache → lazy holes)
4. The plugin system: QuickJS-WASM sandbox, capability permissions, crash recovery
5. The visual editor: canvas, panels, toolbar, Zustand+Mutative store
6. The content model: data_tables + data_rows, no separate pages table
7. The AI runtime: HTTP-only drivers, multi-turn tool loop, no provider SDKs
8. The tree primitive: NodeTree<TNode> for pages, visual components, slot fills
9. Development setup: Bun, Vite, React Compiler, Playwright E2E

## Documentation Structure

```
src.Instatic/
├── spec.md
├── markdown/
│   ├── README.md                  ← Index / table of contents
│   ├── 00-overview.md             ← What Instatic is, pre-release status, stack overview
│   ├── 01-architecture.md         ← One process, two workers, routing, database
│   ├── 02-server.md               ← Hand-written router, handlers, repositories
│   ├── 03-publisher.md            ← Three-layer publishing pipeline
│   ├── 04-plugin-system.md        ← QuickJS-WASM sandbox, capabilities, lifecycle
│   ├── 05-visual-editor.md        ← Canvas, panels, store, React Compiler
│   ├── 06-ai-runtime.md           ← HTTP-only drivers, tool loop, no SDKs
│   ├── 07-content-model.md        ← data_tables, data_rows, NodeTree, TypeBox
│   ├── 08-deployment.md           ← Docker, Postgres/SQLite, TLS, HA
│   └── 09-cross-cutting.md        ← Banned deps, patterns, testing
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
| 12 | HTML generation | DONE |
| 13 | Grandfather review | DONE (fixed route count 15→19) |

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

If interrupted, check `spec.md` tasks table for DONE status.
