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

## Navigation

Start here:

- **[00-overview](00-overview.md)** — What Instatic is, pre-release status, stack overview
- **[01-architecture](01-architecture.md)** — One process, two workers, routing table, database adapters

Core systems:

- **[02-server](02-server.md)** — Hand-written router with 15 ordered routes, handlers, repositories
- **[03-publisher](03-publisher.md)** — Three-layer publishing: static bake → LRU cache → lazy holes
- **[04-plugin-system](04-plugin-system.md)** — QuickJS-WASM sandbox, capability permissions, crash recovery
- **[05-visual-editor](05-visual-editor.md)** — Canvas, panels, Zustand+Mutative store, React Compiler

Advanced:

- **[06-ai-runtime](06-ai-runtime.md)** — HTTP-only AI drivers, multi-turn tool loop, no provider SDKs
- **[07-content-model](07-content-model.md)** — data_tables, data_rows, NodeTree<TNode>, TypeBox validation

Operations:

- **[08-deployment](08-deployment.md)** — Docker, Postgres/SQLite setup, TLS, multi-instance HA
- **[09-cross-cutting](09-cross-cutting.md)** — Banned dependencies, patterns, testing strategy
