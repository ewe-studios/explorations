---
title: iii Ecosystem Documentation
---

# iii Ecosystem Documentation

## Foundation

- [**00 — Overview**](00-overview.md) — What iii is, the Worker → Function → Trigger mental model, and why it eliminates integration code
- [**01 — Architecture**](01-architecture.md) — Full dependency graph, layer diagrams, engine structure, and module map
- [**02 — Engine Core**](02-engine-core.md) — Engine struct, registries, message routing, and lifecycle management
- [**03 — Protocol & WebSocket**](03-protocol-websocket.md) — Message types, binary telemetry frames, and connection lifecycle

## Core Systems

- [**04 — Workers System**](04-workers-system.md) — Worker trait, in-process vs external, hot reload, RBAC sessions, and adapter pattern
- [**05 — Functions & Triggers**](05-functions-triggers.md) — Function registry, trigger types, schema validation, and invocation flow
- [**06 — Observability**](06-observability.md) — OTEL integration (6,101 lines), metrics, traces, logs, and the /otel endpoint
- [**07 — SDK Packages**](07-sdk-packages.md) — Node.js, Python, and Rust SDK deep dives with registration patterns

## Ecosystem Workers & Tools

- [**08 — Ecosystem Workers**](08-ecosystem-workers.md) — 17+ workers: shell, database, storage, MCP, ACP, harness, console, and more
- [**09 — AgentMemory**](09-agentmemory.md) — AI agent memory: observation pipeline, compression, triple-stream search, and 4-tier consolidation
- [**10 — SpecForge**](10-spec-forge.md) — UI spec generation: dual-tier caching, JSONL streaming, validation, and collaborative sessions
- [**11 — CLI Tooling**](11-cli-tooling.md) — Project scaffolding: templates, runtime detection, telemetry, and multi-product architecture
- [**12 — Skills & Validation**](12-skills-validation.md) — Documentation validation: dual-audience rendering, three-layer checks, and Vale prose linting
- [**13 — Examples**](13-examples.md) — Example patterns: human-in-loop, chat agent, todo API, and property search agent

## Cross-Cutting

- [**14 — Data Flow**](14-data-flow.md) — End-to-end flows: invocation, durable workflows, streaming, and telemetry pipelines
- [**15 — Cross-Cutting Concerns**](15-cross-cutting.md) — Security model, configuration system, testing strategy, CI/CD, and console architecture
