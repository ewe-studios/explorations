# src.Instatic Documentation

A self-hosted, open-source CMS with a built-in visual editor, powered by Bun.

## Project Overview

| Attribute | Value |
|-----------|-------|
| Version | 0.0.6 (pre-release) |
| Runtime | Bun |
| Language | TypeScript |
| Database | Postgres or SQLite |
| License | MIT |
| Source Scale | 301 core files, 752 admin files, 66 plugin files |

## Navigation

Start here:

- **[00-overview](00-overview.md)** — What Instatic is, pre-release status, stack overview
- **[01-architecture](01-architecture.md)** — One process, two workers, 19-route dispatcher, database

Core systems:

- **[02-server](02-server.md)** — Hand-written router with 19 ordered routes, handlers, repositories
- **[03-publisher](03-publisher.md)** — Three-layer publishing: static bake → LRU cache → lazy holes
- **[04-publisher-deep](04-publisher-deep.md)** — render.ts, renderNode.ts, dynamicDetection deep dive
- **[05-plugin-system](05-plugin-system.md)** — QuickJS-WASM sandbox, capability permissions, crash recovery
- **[06-plugin-deep](06-plugin-deep.md)** — Host, VM, protocol, bootstrap, RPC deep dive
- **[07-visual-editor](07-visual-editor.md)** — Canvas, panels, Zustand+Mutative store, React Compiler
- **[08-canvas](08-canvas.md)** — Canvas rendering, dnd, composition, breakpoint frames

Advanced:

- **[09-ai-runtime](09-ai-runtime.md)** — HTTP-only AI drivers, multi-turn tool loop, no provider SDKs
- **[10-ai-agent](10-ai-agent.md)** — Site agent, executor, tools, streaming
- **[11-content-model](11-content-model.md)** — data_tables, data_rows, NodeTree<TNode>, TypeBox validation

Operations:

- **[12-loops-forms](12-loops-forms.md)** — Loops, forms, media, templates, site import
- **[13-deployment](13-deployment.md)** — Docker, Postgres/SQLite setup, TLS, multi-instance HA
- **[14-cross-cutting](14-cross-cutting.md)** — Banned dependencies, patterns, testing, benchmarks
