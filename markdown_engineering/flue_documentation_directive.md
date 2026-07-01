# Flue Documentation Directive

A comprehensive guide for documenting **Flue** — the Agent Harness Framework. This directive adapts the general `documentation_directive.md` for Flue's specific architecture and teaching needs.

## What Flue Is

Flue is a TypeScript framework for building headless, programmable AI agents. It wraps `pi-agent-core` into a full runtime with HTTP/WebSocket APIs, sandbox isolation, session persistence, structured results, workflow orchestration, and multi-target deployment (Node.js, Cloudflare Workers). It is **not** an AI SDK — it is a runtime framework like Astro or Next.js, but for agents.

## Core Concepts to Document

1. **Agents** — defined in `agents/<name>.ts` via `createAgent()`, addressable at `/agents/:name/:id`
2. **Harnesses** — configured environments returned by `init()`, holding model, sandbox, tools, skills
3. **Sessions** — conversation state containers within a harness, with message history, compaction, and persistence
4. **Sandboxes** — isolated execution environments (virtual/just-bash, local, remote via connectors)
5. **Tools** — LLM-callable functions (built-in read/write/edit/bash/grep/glob/task, custom, MCP)
6. **Skills** — markdown-defined agent capabilities discovered from `.agents/skills/` or statically imported
7. **Workflows** — orchestration handlers exported from `.flue/workflows/`, invoked via HTTP/CLI
8. **Dispatch** — asynchronous input delivery to agent sessions via `dispatch()`
9. **Compaction** — automatic context-window management via LLM summarization
10. **Multi-target** — same code deploys to Node.js or Cloudflare Workers + Durable Objects

## Architecture Deep Dives

| Area | Key Files |
|------|-----------|
| Runtime core | `packages/runtime/src/runtime/flue-app.ts`, `routing.ts` |
| Agent definition | `packages/runtime/src/agent-definition.ts`, `agent.ts` |
| Session engine | `packages/runtime/src/session.ts` (2271 lines) |
| Sandbox system | `packages/runtime/src/sandbox.ts`, `agent.ts` (tool creation) |
| Compaction | `packages/runtime/src/compaction.ts`, session.ts `runCompaction()` |
| Cloudflare target | `packages/runtime/src/cloudflare/` (14 files) |
| Node target | `packages/runtime/src/node/` (5 files) |
| SDK client | `packages/sdk/src/client.ts`, `public/` |
| CLI | `packages/cli/src/lib/` |
| OpenTelemetry | `packages/opentelemetry/src/index.ts` |

## Documentation Structure

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

## Aha Moments to Surface

1. **Flue agents are Claude Code without the TUI** — same tools (read/write/edit/bash/grep/glob), same AGENTS.md discovery, same skills system, but 100% headless and programmable
2. **Session history is a tree, not a list** — `SessionHistory` uses parent-linked entries with `leafId` for branch tracking, enabling task sessions to persist as parent-owned subtrees
3. **Compaction is an LLM summarization pass, not a truncation** — it uses the same agent harness to summarize old messages, persists the cost on `CompactionEntry.usage`, and folds it into the triggering call's usage
4. **The bash tool has dual timeout enforcement** — `params.timeout` (LLM-visible, recoverable exit 124) + `AbortSignal.timeout` (local backstop for connectors that ignore both)
5. **Workflows and agents share the same route handler body** — `flue-app.ts` routes both through `handleAgentRequest` / `handleWorkflowRequest` with the same middleware chain
6. **Cloudflare persistence is automatic** — session data lives in Durable Objects; no database config needed. Node defaults to in-memory unless you supply a `SessionStore`
7. **Result schemas use `finish`/`give_up` injection** — the framework injects two hidden tools into the harness, loops until one is called, with a 32-attempt defense-in-depth ceiling
8. **Sandbox connectors are duck-typed** — `SandboxApi` interface means any provider (E2B, Daytona, Modal, Boxd, Mirage) implements the same 8 methods, zero provider branching in core

## Quality Standards

All Iron Rules from `documentation_directive.md` apply. Additional Flue-specific requirements:

- Every API surface in `types.ts` that is exported must appear in documentation
- Every built-in tool (read, write, edit, bash, grep, glob, task) must be documented with its exact parameter schema and limits (MAX_READ_LINES=2000, MAX_READ_BYTES=50KB, etc.)
- Every `FlueEvent` type variant must be explained in the observability document
- Cloudflare vs Node differences must be explicit — never say "on some platforms"
- Connector documentation must reference the `connectors/` directory spec files
