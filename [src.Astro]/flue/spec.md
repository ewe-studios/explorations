---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Astro/flue/
repository: github.com/withastro/flue
explored_at: 2026-07-01
documentation_goal: Document Flue — the Agent Harness Framework. A TypeScript runtime for building headless, programmable AI agents with HTTP/WebSocket APIs, sandbox isolation, session persistence, structured results, and multi-target deployment (Node.js, Cloudflare Workers).
---

# Spec: Flue Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `src.Astro/flue/` |
| Languages | TypeScript (monorepo, pnpm workspace) |
| License | Apache-2.0 |
| Version | 0.9.1 |
| Package Manager | pnpm@11.1.1 |
| Node Engine | >=22.18.0 |

### Packages

| Package | Version | Purpose |
|---------|---------|---------|
| `@flue/runtime` | 0.9.1 | Core runtime: harness, sessions, tools, sandbox, Cloudflare+Node adapters |
| `@flue/sdk` | 0.9.1 | Client SDK: HTTP client, WebSocket sockets, admin routes (deprecated exports) |
| `@flue/cli` | 0.9.1 | CLI binary: `flue dev`, `flue build`, `flue run`, `flue add`, `flue connect` |
| `@flue/connectors` | 0.3.7 | Connector metadata (private package) |
| `@flue/opentelemetry` | 0.9.1 | OpenTelemetry adapter for FlueEvent observability |

### Key Dependencies

| Dependency | Purpose |
|------------|---------|
| `@earendil-works/pi-agent-core` | Agent harness engine (Agent class, tool execution) |
| `@earendil-works/pi-ai` | Model provider abstraction, streaming, types |
| `hono` | HTTP framework (routing, middleware) |
| `hono-openapi` | OpenAPI schema generation |
| `valibot` | Runtime validation (agent profiles, schemas) |
| `just-bash` | In-process bash execution for virtual sandbox |
| `@modelcontextprotocol/sdk` | MCP server connection support |
| `turbo` | Monorepo build orchestration |

## 2. What Flue Is

Flue is **The Agent Harness Framework** — a TypeScript runtime for building headless, programmable AI agents. If you know how to use Claude Code, you already know the basics of building agents with Flue. The same tools (read, write, edit, bash, grep, glob), the same AGENTS.md discovery system, the same skills mechanism — but without a TUI, without a GUI, and without requiring a human operator. It's like Claude Code, but 100% headless and programmable.

Flue is not another AI SDK. It's a proper runtime-agnostic framework — think Astro or Next.js, but for agents. Write once, build, and deploy anywhere (Node.js, Cloudflare Workers, GitHub Actions, GitLab CI/CD).

## 3. Documentation Goal

A reader should understand:
1. The agent/harness/session mental model and how they compose
2. How `createAgent()` works, including profiles, subagents, and skills
3. The session lifecycle: prompt, skill, task, shell, compact, delete
4. The sandbox system: virtual (just-bash), local, remote connectors
5. Built-in tools: read, write, edit, bash, grep, glob, task — with exact limits
6. Compaction: automatic context-window management via LLM summarization
7. Workflows: handlers, run lifecycle, dispatch for async delivery
8. HTTP routing: flue() sub-app, agent/workflow/run routes, WebSocket protocols
9. Cloudflare deployment: Durable Objects, session persistence, registry
10. Node deployment: local sandbox, in-memory sessions, WebSocket
11. SDK client: FlueClient for HTTP, WebSocket, admin routes
12. CLI: dev, build, run, connect, add connectors
13. Observability: FlueEvent system, observe(), OpenTelemetry adapter
14. Session persistence: SessionData v5, storage, tree deletion
15. Structured results: result schemas, finish/give_up injection, retry loop
16. Cross-cutting: error handling, provider config, usage tracking

## 4. Documentation Structure

```
[src.Astro]/flue/
├── spec.md
├── markdown/
│   ├── README.md                       ← Index
│   ├── 00-overview.md                  ← What Flue is, philosophy, quickstart
│   ├── 01-architecture.md              ← Full dependency graph, layers, targets
│   ├── 02-agent-system.md              ← createAgent, profiles, subagents, skills
│   ├── 03-harness-sessions.md          ← init(), harness, session lifecycle
│   ├── 04-sandbox-tools.md             ← Sandboxes, built-in tools, custom tools, MCP
│   ├── 05-compaction.md                ← Context management, summarization, overflow
│   ├── 06-workflows.md                 ← Workflow handlers, run lifecycle, dispatch
│   ├── 07-routing-http-websocket.md    ← flue(), routes, protocols, error envelopes
│   ├── 08-cloudflare-target.md         ← Durable Objects, registry, persistence
│   ├── 09-node-target.md               ← Local sandbox, process lifetime, WebSocket
│   ├── 10-sdk-client.md                ← FlueClient, HTTP, WebSocket, admin routes
│   ├── 11-cli-build-deploy.md          ← flue CLI, build targets, connectors
│   ├── 12-observability.md             ← FlueEvent system, observe(), OpenTelemetry
│   ├── 13-session-persistence.md       ← SessionData, versioning, storage, tree deletion
│   ├── 14-structured-results.md        ← result schemas, finish/give_up tools, retry loop
│   ├── 15-cross-cutting.md             ← Error handling, provider config, usage tracking
├── html/
└── build.py
```

## 5. Tasks

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-architecture.md | TODO |
| 3 | 02-agent-system.md | TODO |
| 4 | 03-harness-sessions.md | TODO |
| 5 | 04-sandbox-tools.md | TODO |
| 6 | 05-compaction.md | TODO |
| 7 | 06-workflows.md | TODO |
| 8 | 07-routing-http-websocket.md | TODO |
| 9 | 08-cloudflare-target.md | TODO |
| 10 | 09-node-target.md | TODO |
| 11 | 10-sdk-client.md | TODO |
| 12 | 11-cli-build-deploy.md | TODO |
| 13 | 12-observability.md | TODO |
| 14 | 13-session-persistence.md | TODO |
| 15 | 14-structured-results.md | TODO |
| 16 | 15-cross-cutting.md | TODO |

## 6. Build System

```bash
cd [src.Astro]/flue && python3 build.py          # build all
python3 build.py flue                              # build this project
```

Shared `build.py` from repository root. Python 3.12+ stdlib only. Generates HTML with dark/light theme, Mermaid diagrams, prev/next navigation, index page.

## 7. Quality Requirements

All ten Iron Rules from `documentation_directive.md` apply:
1. Detailed sections with code snippets
2. Teach key facts, principles, and ideas quickly
3. Clear articulation
4. Mermaid diagrams (minimum 2 per document)
5. Good visual assets (tables, ASCII art, code blocks)
6. Generated HTML with navigation
7. Cross-references (no orphan pages)
8. Source path references
9. Aha moments (at least 1 per document)
10. Navigation: index + prev/next buttons

## 8. Expected Outcome

After reading, a developer should be able to:
- Write a Flue agent with proper model, sandbox, tools, and skills configuration
- Deploy to Node.js or Cloudflare Workers
- Build a workflow with multiple harnesses and session orchestration
- Use the SDK client to invoke agents and inspect workflow runs
- Connect MCP servers as tools
- Configure compaction for long-running sessions
- Use structured results with valibot schemas
- Add custom tools and subagent profiles
- Set up observability with OpenTelemetry

## 9. Resume Point

If interrupted, check the spec.md Tasks table for the next TODO document. Documents should be written in order (00 → 01 → 02 → ...) since later documents reference earlier ones.
