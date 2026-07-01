---
title: Flue — The Agent Harness Framework
---

# Flue Documentation

**The Agent Harness Framework** — TypeScript agents that act autonomously. Like Claude Code, but 100% headless and programmable.

## Foundation

- [Overview](00-overview.html) — What Flue is, philosophy, quickstart agents
- [Architecture](01-architecture.html) — Full dependency graph, layer diagrams, multi-target deployment

## Core Concepts

- [Agent System](02-agent-system.html) — `createAgent()`, profiles, subagents, skills
- [Harness & Sessions](03-harness-sessions.html) — `init()`, harness, session lifecycle
- [Sandbox & Tools](04-sandbox-tools.html) — Sandboxes, built-in tools, custom tools, MCP
- [Compaction](05-compaction.html) — Context-window management, LLM summarization

## Orchestration

- [Workflows](06-workflows.html) — Workflow handlers, run lifecycle, `dispatch()`
- [HTTP & WebSocket Routing](07-routing-http-websocket.html) — `flue()`, routes, protocols, error envelopes

## Deployment Targets

- [Cloudflare Target](08-cloudflare-target.html) — Durable Objects, registry, persistence
- [Node Target](09-node-target.html) — Local sandbox, process lifetime, WebSocket

## Client & Tooling

- [SDK Client](10-sdk-client.html) — `FlueClient`, HTTP, WebSocket, admin routes
- [CLI, Build & Deploy](11-cli-build-deploy.html) — `flue` CLI, build targets, connectors

## Cross-Cutting

- [Observability](12-observability.html) — `FlueEvent` system, `observe()`, OpenTelemetry
- [Session Persistence](13-session-persistence.html) — `SessionData` v5, storage, tree deletion
- [Structured Results](14-structured-results.html) — Result schemas, `finish`/`give_up` tools
- [Cross-Cutting Concerns](15-cross-cutting.html) — Error handling, provider config, usage tracking
