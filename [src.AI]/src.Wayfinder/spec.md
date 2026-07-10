---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder/
language: Python, TypeScript
version: rac-core 0.27.x, proofkeeper 2026.07.1, wayfinder-router 0.1.x
explored_at: 2026-07-09
license: Apache-2.0
---

# spec.md — Wayfinder/RAC Documentation

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder/`
- **Languages:** Python 3.11+ (rac-core, wayfinder-router, rac-benchmarks), TypeScript 5+ (proofkeeper, rac-sdk)
- **Source scale:** 7 sub-projects, ~484 source files
- **Author:** Tom Ballard (itsthelore)
- **License:** Apache-2.0

## What Wayfinder/RAC Is

RAC (Requirements As Code) is a deterministic knowledge-management engine that captures product requirements, architecture decisions, designs, roadmaps, and prompts as typed Markdown files with YAML frontmatter. It serves them read-only to AI coding agents over MCP. The ecosystem includes proofkeeper (autonomous verification), wayfinder-router (deterministic prompt routing), CI integrations, and SDKs. Every component is deterministic: no LLM calls in the engine, no randomness, no network. Same inputs produce byte-identical outputs.

## Documentation Goal

A reader should understand:
1. What RAC is and why it exists (deterministic knowledge management for AI agents)
2. The layered architecture of rac-core (CLI, services, core, output, MCP)
3. The artifact type system (Requirement, Decision, Roadmap, Prompt, Design)
4. How proofkeeper drives products autonomously and compiles sessions to Playwright tests
5. How wayfinder-router scores prompt complexity with 11 features and routes to models
6. How rac-benchmarks evaluates retrieval and grounding quality
7. How the TypeScript SDK wraps the rac CLI
8. How the knowledge graph is built and traversed
9. How search and resolution work deterministically
10. How errors are handled across all layers
11. How the Explorer TUI provides a keyboard-first terminal interface
12. How bundled skills and agent-rules projection work
13. How portfolio health scoring and gate enforcement operate
14. How document ingestion and OKF export function
15. How the MCP server serves artifacts to AI agents
16. How proofkeeper components (observation, redaction, policy, adapters) work
17. How the Wayfinder gateway handles streaming, caching, pricing, and reliability
18. How AST diffing, intent classification, and CI integration function
19. How benchmark harness architecture and scoring mechanics work

## Documentation Structure

```
src.Wayfinder/
├── spec.md
├── markdown/
│   ├── README.md                       ← Index / table of contents
│   ├── 00-overview.md                  ← What Wayfinder/RAC is, philosophy, component map
│   ├── 01-architecture.md              ← Layers, modules, Product AST, artifact types
│   ├── 02-rac-core.md                  ← CLI commands, services, validation
│   ├── 03-proofkeeper.md               ← Autonomous QA: drive, compile, fidelity, triage, writeback
│   ├── 04-wayfinder-router.md          ← Prompt routing: 11-feature scorer, gateway, calibration
│   ├── 05-rac-benchmarks.md            ← 8 eval suites, autonomous QA, decision grounding
│   ├── 06-rac-sdk.md                   ← TypeScript client
│   ├── 07-knowledge-graph.md           ← Relationships, traversal, hub detection, export
│   ├── 08-deterministic-search.md      ← Resolution, retrieval, decision finding, eval benchmark
│   ├── 09-data-flow.md                 ← End-to-end request lifecycle
│   ├── 10-error-handling.md            ← Exit codes, error types, refusals, recovery
│   ├── 11-explorer-tui.md              ← Textual TUI: workspace frame, views, live reload, commands
│   ├── 12-skills-agent-rules.md        ← Bundled skills, agent-rules projection, drift-guarded blocks
│   ├── 13-portfolio-gate.md            ← Health scoring, enforcement policy, SARIF, doctor
│   ├── 14-ingest-okf.md                ← Document conversion pipeline, OKF format, output renderers
│   ├── 15-mcp-server.md                ← FastMCP server, 5 tools, budget, audit, telemetry
│   ├── 16-proofkeeper-components.md    ← Observation, redaction, policy, adapters, learning, triage
│   ├── 17-wayfinder-gateway.md         ← Gateway proxy, streaming, cache, pricing, reliability, vkeys
│   ├── 18-diff-intent-ci.md            ← AST diff, intent classification, watchkeeper, CI actions
│   └── 19-benchmarks-deep.md           ← Harness architecture, retrieval scoring, grounding, autonomous QA
└── html/                               ← Generated HTML
```

## Tasks

| # | Document | Status |
|---|----------|--------|
| 1 | README.md | DONE |
| 2 | 00-overview.md | DONE |
| 3 | 01-architecture.md | DONE |
| 4 | 02-rac-core.md | DONE |
| 5 | 03-proofkeeper.md | DONE |
| 6 | 04-wayfinder-router.md | DONE |
| 7 | 05-rac-benchmarks.md | DONE |
| 8 | 06-rac-sdk.md | DONE |
| 9 | 07-knowledge-graph.md | DONE |
| 10 | 08-deterministic-search.md | DONE |
| 11 | 09-data-flow.md | DONE |
| 12 | 10-error-handling.md | DONE |
| 13 | 11-explorer-tui.md | DONE |
| 14 | 12-skills-agent-rules.md | DONE |
| 15 | 13-portfolio-gate.md | DONE |
| 16 | 14-ingest-okf.md | DONE |
| 17 | 15-mcp-server.md | DONE |
| 18 | 16-proofkeeper-components.md | DONE |
| 19 | 17-wayfinder-gateway.md | DONE |
| 20 | 18-diff-intent-ci.md | DONE |
| 21 | 19-benchmarks-deep.md | DONE |
| 22 | HTML generation | DONE |
| 23 | Grandfather review | DONE |

## Build System

```bash
cd [src.AI] && python3 build.py src.Wayfinder   # build all Wayfinder docs
```

Dependencies: Python 3.12+ stdlib only (no third-party packages).

## Expected Outcome

After reading these documents, a developer can:
- Invoke any rac subcommand and interpret its exit code
- Author correctly-typed artifact files with proper frontmatter
- Understand the proofkeeper autonomous QA loop (drive, compile, fidelity, triage, writeback)
- Configure wayfinder-router with custom weights, tiers, or classifiers
- Use the TypeScript SDK to integrate RAC into editor extensions
- Trace the knowledge graph from any artifact
- Search and resolve artifact IDs deterministically
- Handle errors and refusals appropriately in scripts and CI
- Navigate the Explorer TUI efficiently with keyboard shortcuts
- Install and use bundled skills for AI agent workflows
- Configure gate enforcement policies for CI
- Ingest DOCX/PDF/HTML documents into RAC format
- Use the MCP server to serve artifacts to AI agents
- Understand proofkeeper's observation, redaction, and policy systems
- Configure the Wayfinder gateway with streaming, caching, and reliability
- Use AST diffing and intent classification for corpus review
- Run and interpret benchmark results

## Resume Point

If interrupted, the next step is HTML generation (`python3 build.py src.Wayfinder`) followed by grandfather review: verify every name, number, flow, and coverage claim against the source code at `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder/`.
