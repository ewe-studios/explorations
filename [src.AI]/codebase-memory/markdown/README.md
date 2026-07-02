---
title: "codebase-memory-mcp Documentation"
---

# codebase-memory-mcp Documentation

High-performance code intelligence engine for AI coding agents. Pure C11, 156 languages, 14 MCP tools, knowledge graph with Cypher queries.

## Foundation

- **[00-overview](00-overview.html)** — What it is, why it exists, quick architecture, getting started
- **[01-architecture](01-architecture.html)** — Full dependency graph, layer diagram, module map, data flow
- **[02-foundation-layer](02-foundation-layer.html)** — Arena, slab, mimalloc, hash tables, string interning, logging

## Core Pipeline

- **[03-discovery](03-discovery.html)** — File discovery, gitignore handling, language detection, user configuration
- **[04-extraction](04-extraction.html)** -- Core extraction engine, tree-sitter AST, 156 language grammars
- **[05-pipeline](05-pipeline.html)** -- Three-phase parallel pipeline, 22 passes, worker pool, registry resolution

## Storage and Querying

- **[06-store](06-store.html)** — SQLite graph storage, FTS5, bulk write optimization, Louvain clustering
- **[07-cypher](07-cypher.html)** — Cypher query engine, 90+ tokens, recursive descent parser, planner, executor

## Interface

- **[08-mcp-tools](08-mcp-tools.html)** — 14 MCP tools over JSON-RPC 2.0 stdio, inputs, outputs, examples
- **[10-cli-install](10-cli-install.html)** -- CLI install, update, 9-agent auto-configuration, environment variables
- **[11-graph-ui](11-graph-ui.html)** — 3D Barnes-Hut layout, embedded HTTP server, frontend visualization

## Advanced

- **[09-semantic-similarity](09-semantic-similarity.html)** — 11-signal ensemble, MinHash/LSH, nomic-embed-code vectors
- **[12-build-test](12-build-test.html)** — Makefile targets, 5,604 tests, security audit, performance tuning
- **[13-cross-cutting](13-cross-cutting.html)** — LZ4/Zstd compression, memory management, logging, diagnostics, traces

---

**Repository**: [github.com/DeusData/codebase-memory-mcp](https://github.com/DeusData/codebase-memory-mcp)  
**Language**: Pure C11  
**Lines**: 63,896 across 119 source files  
**Tests**: 152 test files  
**Languages**: 156 via vendored tree-sitter grammars
