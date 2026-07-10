# Instatic — Self-Hosted CMS with Visual Editor

Single Bun process serving public site, admin editor, CMS API, and published pages. Outputs plain HTML/CSS — no framework in published output.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/` — 1,750+ TypeScript files, Bun runtime, MIT

## Documents

### Foundation

| # | Document | What You'll Master |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | Single-process architecture, core engine, 3-layer publishing, key decisions |
| 01 | [Core Engine](01-architecture.md) | NodeTree, mutations, module engine, 3-layer pipeline, Visual Components |
| 02 | [Server](02-server.md) | Hand-written router, 19 routes, handlers, repositories |
| 08 | [Deployment](08-deployment.md) | Docker, Postgres/SQLite, TLS with Caddy, multi-instance HA |
| 22 | [Security & Auth](22-security-auth.md) | CSRF defense, 27 capabilities, rate limiting, session management |

### Publishing & Content

| # | Document | What You'll Master |
|---|----------|-------------------|
| 03 | [Publisher](03-publisher.md) | Static artifacts, LRU cache, hole runtime, CSS concatenation order |
| 04 | [Publisher Deep Dive](04-publisher-deep.md) | render.ts, renderNode.ts, dynamicDetection, escapeProps, CSS collector |
| 11 | [Content Model](11-content-model.md) | data_tables, data_rows, NodeTree, TypeBox validation |
| 12 | [Loops & Forms](12-loops-forms.md) | Content loops, form handling, media, templates |
| 21 | [Forms System](21-forms-system.md) | Challenge-response anti-spam, rate limiting, native CMS forms |

### Plugin System

| # | Document | What You'll Master |
|---|----------|-------------------|
| 04 | [Plugin System](04-plugin-system.md) | QuickJS-WASM sandbox, 23 capabilities, Bun.Worker isolation |
| 06 | [Plugin Deep Dive](06-plugin-deep.md) | Host, VM, protocol, bootstrap, RPC, crash recovery |

### Visual Editor

| # | Document | What You'll Master |
|---|----------|-------------------|
| 05 | [Visual Editor](05-visual-editor.md) | Canvas, panels, toolbar, Zustand+Mutative store, React Compiler |
| 08 | [Canvas](08-canvas.md) | Canvas rendering, dnd, composition, breakpoint frames, live surface |
| 13 | [Editor Store](13-editor-store.md) | 14-slice Zustand architecture, patch-based undo, Mutative middleware |
| 19 | [Visual Components](19-visual-components.md) | VC definitions, slot outlets, sync, parameter overrides |

### AI Integration

| # | Document | What You'll Master |
|---|----------|-------------------|
| 06 | [AI Runtime](06-ai-runtime.md) | HTTP-only drivers, shared tool loop, 35 tools, dual-endpoint bridge |
| 10 | [AI Agent](10-ai-agent.md) | Site agent, executor, tools, streaming, token runners |

### Modules & CSS

| # | Document | What You'll Master |
|---|----------|-------------------|
| 14 | [Module System](14-module-system.md) | 15 base blocks, pure render, TypeBox schemas, prop escaping |
| 15 | [Media Pipeline](15-media-pipeline.md) | Upload, variants, storage adapters, responsive image sizes |
| 20 | [Framework CSS](20-framework-css.md) | Fluid typography, token system, generated utilities, three-layer CSS bundle |

### Admin & Infrastructure

| # | Document | What You'll Master |
|---|----------|-------------------|
| 09 | [Cross-Cutting](09-cross-cutting.md) | Banned deps, patterns, testing, benchmarks |
| 16 | [Admin SPA](16-admin-spa.md) | Admin shell, workspaces, navigation, Spotlight command palette |
| 23 | [Site Import/Export](23-site-import-export.md) | HTML import pipeline, asset rewriting, CMS transfer, templates |

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Instatic
python3 build.py .
```
