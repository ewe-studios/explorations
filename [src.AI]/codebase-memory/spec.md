---
title: "codebase-memory-mcp Documentation Spec"
---

# Project Spec: codebase-memory-mcp Documentation

## Source Codebase

- **Location**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.CodebaseMemory/codebase-memory-mcp/`
- **Repository**: https://github.com/DeusData/codebase-memory-mcp
- **License**: MIT
- **Language**: Pure C11
- **Source Lines**: 63,896 lines across 1,378 source files, 89 test files
- **Languages Supported**: 158 via vendored tree-sitter grammars
- **Build System**: Makefile (`Makefile.cbm`)
- **Tests**: 5,604 passing tests (ASan+UBSan, TSan, clang-tidy)

## What the Project Is

codebase-memory-mcp is a high-performance code intelligence engine for AI coding agents. It indexes repositories into a property graph (SQLite with FTS5), then exposes 14 MCP tools over JSON-RPC 2.0 stdio for structured codebase querying, traversal, and analysis. It supports 158 languages via tree-sitter, runs semantic similarity with an 11-signal ensemble including MinHash/LSH and nomic-embed-code vectors, and provides a Cypher query engine with 90+ token types. The Linux kernel (28M LOC, 75K files) indexes in 3 minutes.

## Documentation Goal

After reading these documents, a reader should understand:

1. The overall architecture and data flow from repository to queryable knowledge graph
2. How the three-phase parallel pipeline extracts code structure across 158 languages
3. The foundation layer (arena, slab, mimalloc, hash tables, string interning)
4. File discovery, gitignore handling, and language detection
5. Per-concern extraction (definitions, calls, imports, usages, tests, routes, etc.)
6. The SQLite store schema, FTS5 integration, and bulk write optimization
7. The Cypher query engine (lexer, recursive descent parser, planner, executor)
8. All 14 MCP tools, their inputs, outputs, and use cases
9. The 11-signal semantic similarity ensemble and MinHash/LSH algorithms
10. CLI install, update, and agent auto-configuration
11. The 3D graph visualization with Barnes-Hut layout
12. Build system, testing, security auditing, and performance tuning
13. Cross-cutting concerns (compression, memory management, logging, diagnostics)

## Documentation Structure

```
codebase-memory/
  spec.md
  markdown/
    README.md              ← Index with categorized document links
    00-overview.md         ← What it is, why it exists, quick architecture
    01-architecture.md     ← Full dependency graph, layer diagram, module map
    02-foundation-layer.md ← Arena, slab, mimalloc, hash tables, string interning
    03-discovery.md        ← File discovery, gitignore, language detection
    04-extraction.md       ← Core extraction engine across 158 languages
    05-pipeline.md         ← Three-phase parallel pipeline (22,049 lines)
    06-store.md            ← SQLite graph storage, FTS5, bulk writes
    07-cypher.md           ← Cypher query engine (90+ tokens, recursive descent)
    08-mcp-tools.md        ← 14 MCP tools over JSON-RPC 2.0 stdio
    09-semantic-similarity.md ← 11-signal ensemble, MinHash/LSH, nomic vectors
    10-cli-install.md      ← CLI install, agent auto-config, update mechanism
    11-graph-ui.md         ← 3D Barnes-Hut layout, embedded HTTP server
    12-build-test.md       ← Makefile targets, test suite, security audit
    13-cross-cutting.md    ← Compression, memory, logging, diagnostics, traces
  html/                    ← Generated HTML
  build.py                 ← Shared markdown-to-HTML builder
```

## Tasks

| Phase | Document | Status |
|-------|----------|--------|
| 1 | spec.md | DONE |
| 1 | markdown/README.md | DONE |
| 1 | markdown/00-overview.md | DONE |
| 2 | markdown/01-architecture.md | DONE |
| 2 | markdown/02-foundation-layer.md | DONE |
| 2 | markdown/03-discovery.md | DONE |
| 2 | markdown/04-extraction.md | DONE |
| 3 | markdown/05-pipeline.md | DONE |
| 3 | markdown/06-store.md | DONE |
| 3 | markdown/07-cypher.md | DONE |
| 3 | markdown/08-mcp-tools.md | DONE |
| 4 | markdown/09-semantic-similarity.md | DONE |
| 4 | markdown/10-cli-install.md | DONE |
| 4 | markdown/11-graph-ui.md | DONE |
| 4 | markdown/12-build-test.md | DONE |
| 4 | markdown/13-cross-cutting.md | DONE |
| 5 | HTML build | DONE |
| 5 | Grandfather review | DONE |

## Build System

```bash
cd "[src.AI]/codebase-memory" && python3 build.py
```

- Zero dependencies (Python 3.12+ stdlib only)
- Generates HTML with dark/light theme, mermaid diagrams, prev/next nav
- Idempotent

## Quality Requirements

All ten Iron Rules from `documentation_directive.md` apply:

1. Detailed sections with real code snippets and file path references
2. Teach key facts immediately — first paragraph is the thesis
3. Clear articulation — one idea per sentence
4. Minimum 2 mermaid diagrams per document
5. Tables, ASCII art, mermaid for visual communication
6. Generated HTML with proper navigation
7. Cross-references between all documents
8. Source path references for every implementation claim
9. At least one Aha moment per document
10. Index + prev/next navigation on every page

## Expected Outcome

A senior engineer reading these documents can:

- Understand the full architecture without reading source code
- Implement a compatible MCP client
- Write Cypher queries against the graph
- Tune indexing performance for large codebases
- Debug memory issues using the diagnostics system
- Extend the pipeline with new passes
- Integrate the 3D visualization into their own tooling

## Resume Point

If interrupted: all documents are in `markdown/`. The spec.md task table tracks progress. Re-run `python3 build.py` to regenerate HTML after any changes.
