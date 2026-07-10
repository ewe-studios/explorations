# Wayfinder/RAC Documentation

Wayfinder/RAC ecosystem: rac-core (deterministic knowledge management), proofkeeper (autonomous verification), wayfinder-router (prompt-complexity routing), rac-benchmarks (eval suites), rac-sdk (TypeScript client), rac-ci (GitHub Actions), rac-editors (VS Code/Cursor extension).

| Attribute | Value |
|-----------|-------|
| Sub-projects | 7 |
| Source files | ~484 (Python + TypeScript) |
| Languages | Python 3.11+, TypeScript 5+ |
| Licenses | Apache-2.0 |

## Navigation

| # | Document | Purpose |
|---|----------|---------|
| 00 | [Overview](00-overview.md) | Architecture, all 7 sub-projects at a glance |
| 01 | [Architecture](01-architecture.md) | Layers, modules, data flow, Product AST, artifact types |
| 02 | [RAC Core](02-rac-core.md) | CLI, services, validation, relationships, watchkeeper |
| 03 | [Proofkeeper](03-proofkeeper.md) | Autonomous QA: drive, compile, fidelity, triage, writeback |
| 04 | [Wayfinder Router](04-wayfinder-router.md) | Prompt routing: 11-feature scorer, gateway, calibration |
| 05 | [RAC Benchmarks](05-rac-benchmarks.md) | 8 eval suites, autonomous QA, decision grounding |
| 06 | [RAC SDK](06-rac-sdk.md) | TypeScript client, error types, runner interface |
| 07 | [Knowledge Graph](07-knowledge-graph.md) | Artifact relationships, traversal, hub detection, export |
| 08 | [Deterministic Search](08-deterministic-search.md) | Resolution, retrieval, decision finding, eval benchmark |
| 09 | [Data Flow End-to-End](09-data-flow.md) | Request lifecycle: CLI, MCP, export, telemetry |
| 10 | [Error Handling](10-error-handling.md) | Exit codes, error types, refusals, recovery |
| 11 | [Explorer TUI](11-explorer-tui.md) | Textual TUI: workspace frame, views, live reload, command palette, split layout |
| 12 | [Skills & Agent Rules](12-skills-agent-rules.md) | Bundled skills, agent rules projection, OKF, drift-guarded blocks |
| 13 | [Portfolio & Gate](13-portfolio-gate.md) | Health scoring, gate enforcement, SARIF, review priorities, doctor |
| 14 | [Ingest Pipeline & OKF](14-ingest-okf.md) | DocumentConverter Protocol, MarkItDown, OKF v0.1 bundle, export modes, conformance |
| 15 | [MCP Server (Lore)](15-mcp-server.md) | FastMCP server, 5 read-only tools, budget, audit, telemetry, ping |
| 16 | [Proofkeeper Components](16-proofkeeper-components.md) | Observation, redaction, policy, model adapters, learning, triage, writeback |
| 17 | [Wayfinder Gateway](17-wayfinder-gateway.md) | Gateway proxy, streaming, cache, pricing, reliability, vkeys, calibration, judge |
| 18 | [Diff, Intent & CI](18-diff-intent-ci.md) | AST diffing, intent classification, watchkeeper, CI integration, revisions |
| 19 | [Benchmarks Deep Dive](19-benchmarks-deep.md) | Harness architecture, retrieval scoring, conformance, grounding, autonomous QA |
