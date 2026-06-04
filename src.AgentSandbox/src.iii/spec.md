---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/
repository: git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: |
  Document the entire iii ecosystem — the Rust engine, SDK packages, worker ecosystem,
  and all 9 subprojects — so a junior engineer can understand the architecture, data flows,
  and how to build on top of iii without reading source code.
---

# Spec: iii Ecosystem Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/` |
| Repository | `git@github.com:iii-hq/iii` (main monorepo) + 8 additional repos |
| Languages | Rust (engine, workers, SDK), TypeScript (Node SDK, console, harness), Python (SDK, examples) |
| License | Elastic License 2.0 (engine), Apache 2.0 (SDKs, CLI, workers, docs) |
| Engine LOC | ~85,000 lines of Rust |
| Subprojects | 9 (iii engine, workers, agentmemory, spec-forge, cli-tooling, examples, iii-cloud-cli, iii-cookbook, skills-and-validation) |

## 2. What iii Is

iii is a **process communication engine** that collapses distributed backend infrastructure — queues, cron, HTTP endpoints, state management, observability, agents, and sandboxes — into a single runtime. Workers (processes) register with the iii engine over WebSocket, declare functions and triggers, and communicate through a unified function/trigger protocol. The architecture follows a **Worker → Function → Trigger** mental model where any worker can call any function from any other worker at runtime, with no integration code required.

## 3. Documentation Goal

A reader should understand:

1. The Worker → Function → Trigger mental model and why it eliminates integration code
2. The engine architecture: how the Rust engine routes messages, manages registries, and handles WebSocket connections
3. The WebSocket protocol: message types, binary frame handling for telemetry, and connection lifecycle
4. The function registry: registration, ownership tracking, scope-based reload, and invocation with OTEL tracing
5. The trigger system: built-in types (http, cron, durable:subscriber, state, stream, subscribe), schema validation, and custom triggers
6. The worker system: trait, in-process vs external workers, hot reload, RBAC sessions, and adapter pattern
7. The observability system: 6,101-line OTEL integration, metrics, traces, logs, and the separate /otel WebSocket endpoint
8. SDK design: how Node.js, Python, and Rust SDKs abstract the WebSocket protocol
9. The worker ecosystem: 17+ workers across Rust, TypeScript, and Python
10. Specialized tools: agentmemory (persistent memory for AI agents), spec-forge (UI spec generation), skills-and-validation (documentation validation)
11. Console architecture: React frontend + Rust backend with embedded assets and WebSocket proxy
12. Data flows: end-to-end function invocation, durable workflows, streaming, and telemetry pipelines

## 4. Documentation Structure

```
src.iii/
├── spec.md                          ← This file (project tracker)
├── markdown/
│   ├── README.md                    ← Index with categorized document links
│   ├── 00-overview.md               ← What iii is, philosophy, architecture at a glance
│   ├── 01-architecture.md           ← Full dependency graph, layer diagrams, module map
│   ├── 02-engine-core.md            ← Engine struct, registries, message routing, lifecycle
│   ├── 03-protocol-websocket.md     ← Message types, binary frames, connection lifecycle
│   ├── 04-workers-system.md         ← Worker trait, in-process vs external, hot reload, RBAC
│   ├── 05-functions-triggers.md     ← Function registry, trigger types, schema validation
│   ├── 06-observability.md          ← OTEL integration, metrics, traces, logs
│   ├── 07-sdk-packages.md           ← Node.js, Python, Rust SDK deep dives
│   ├── 08-ecosystem-workers.md      ← 17+ workers: shell, database, storage, MCP, harness, etc.
│   ├── 09-agentmemory.md            ← AI agent memory: observation, compression, search, consolidation
│   ├── 10-spec-forge.md             ← UI spec generation: caching, streaming, validation
│   ├── 11-cli-tooling.md            ← Project scaffolding: templates, runtime detection, telemetry
│   ├── 12-skills-validation.md      ← Documentation validation: rendering, three-layer checks
│   ├── 13-examples.md               ← Example patterns: human-in-loop, chat agent, todo, property search
│   ├── 14-data-flow.md              ← End-to-end flows: invocation, durable, streaming, telemetry
│   └── 15-cross-cutting.md          ← Security, configuration, testing, CI/CD, console
├── html/
│   ├── index.html                   ← Auto-generated index
│   ├── styles.css                   ← Shared CSS
│   └── *.html                       ← Generated from markdown
├── exploration.md                   ← Existing exploration (preserved)
└── build.py                         ← Shared markdown → HTML script
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md (index) | DONE |
| 1 | 00-overview.md | DONE |
| 2 | 01-architecture.md | DONE |
| 3 | 02-engine-core.md | DONE |
| 4 | 03-protocol-websocket.md | DONE |
| 5 | 04-workers-system.md | DONE |
| 6 | 05-functions-triggers.md | DONE |
| 7 | 06-observability.md | DONE |
| 8 | 07-sdk-packages.md | DONE |
| 9 | 08-ecosystem-workers.md | DONE |
| 10 | 09-agentmemory.md | DONE |
| 11 | 10-spec-forge.md | DONE |
| 12 | 11-cli-tooling.md | DONE |
| 13 | 12-skills-validation.md | DONE |
| 14 | 13-examples.md | DONE |
| 15 | 14-data-flow.md | DONE |
| 16 | 15-cross-cutting.md | DONE |
| 17 | Grandfather review | DONE |
| 18 | Fix grandfather findings | DONE |
| 19 | Generate HTML | DONE |

## 6. Build System

```bash
cd src.iii/
python3 build.py                    # build all
python3 build.py .                  # build this project
```

Single Python 3.12+ stdlib-only build script at `build.py` (copied from root documentation/ directory).

## 7. Quality Requirements

Follow all Iron Rules from the documentation directive:
1. Detailed sections with code snippets (actual function signatures, file paths)
2. Teach key facts quickly (first sentence = thesis)
3. Clear articulation (one idea per sentence)
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, ASCII, code blocks)
6. Generated HTML with navigation (index, prev/next, theme toggle)
7. Cross-references (no orphan pages, link to 2+ related docs)
8. Source path references (file:line for every implementation claim)
9. Aha moments (non-obvious design decisions, clever insights)
10. Navigation bar on every page

## 8. Expected Outcome

After reading this documentation, an engineer should be able to:
- Explain the iii architecture to another engineer
- Write a custom iii worker in Rust, TypeScript, or Python
- Understand how function invocation, triggers, and durable queues work
- Debug WebSocket protocol issues using the message type reference
- Configure workers, adapters, and hot reload
- Understand how observability data flows through the system
- Deploy iii applications using the CLI tooling

## 9. Resume Point

If interrupted, continue writing markdown documents in order (00 → 01 → ...). The spec.md task table tracks progress. After all documents are written, run grandfather review, fix findings, then generate HTML.
