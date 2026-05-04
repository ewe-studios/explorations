# Datastar Ecosystem -- What It Is and Why It Exists

Datastar is a reactive frontend framework that eliminates the build step. You write HTML with `data-*` attributes and get fine-grained reactivity, server streaming, and DOM morphing — no bundler, no framework compilation, no SPA routing. The ecosystem around it forms a complete application stack: a local-first event store (cross-stream), an LLM agent harness (yoke/yoagent), a Nushell-scriptable HTTP server (http-nu), and a P2P tunnel daemon (pai-sho).

**Aha:** Datastar doesn't use a virtual DOM or JSX compilation. It mutates the real DOM in place using a morphing algorithm that preserves element identity across patches. The signal system is hand-built — not a wrapper around Solid.js or Preact Signals — with a doubly-linked dependency graph, version-based staleness tracking, and lazy propagation. You get reactive fine-grained updates with zero framework overhead.

Source: `datastar/library/src/engine/engine.ts` — plugin application loop
Source: `datastar/library/src/engine/signals.ts` — reactive signal implementation

## Quick Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Browser (HTML)                     │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │  Signals    │  │  Plugins     │  │  Morphing  │ │
│  │  (reactive) │→→│  (bind/on/   │→→│  (patch    │ │
│  │  state)     │  │  effect/show)│  │  elements) │ │
│  └──────┬──────┘  └──────────────┘  └──────┬─────┘ │
│         │                                   │       │
│         └──────────── SSE ──────────────────┘       │
│                         ↑↓                          │
├─────────────────────────┼───────────────────────────┤
│                   Server (Rust/Zig/Nushell)         │
│  ┌────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │ http-nu    │  │  Datastar    │  │ cross-     │  │
│  │ (routing + │→→│  Rust SDK    │→→│ stream (xs)│  │
│  │ Nushell)   │  │  (events)    │  │ (event log)│  │
│  └────────────┘  └──────────────┘  └────────────┘  │
│                                      ↑              │
│                              ┌───────┴──────────┐   │
│                              │ yoke / yoagent   │   │
│                              │ (LLM agent loop) │   │
│                              └──────────────────┘   │
└──────────────────────────────────────────────────────┘
```

## The Ecosystem at a Glance

| Project | Language | Purpose |
|---------|----------|---------|
| `datastar/` | TypeScript | Core reactive framework: signals, plugins, DOM morphing, SSE |
| `datastar-rust/` | Rust | Server-side SSE event generation for Rust web frameworks |
| `datastar.http.zig/` | Zig | Zig HTTP server with Datastar SSE support |
| `xs/` (cross-stream) | Rust | Local-first append-only event streaming store |
| `yoke/` | Rust | Headless LLM agent CLI — JSONL in/out, multi-provider |
| `yoagent/` | Rust | Agent loop library: tool execution, streaming, sub-agents |
| `http-nu/` | Rust | Nushell-scriptable HTTP server with Datastar integration |
| `pai-sho/` | Rust | Persistent multi-port tunnel daemon over iroh/QUIC |
| `stacks/` | Rust/TS | Tauri desktop app with Nushell scripting |

See [Architecture](01-architecture.md) for the full dependency graph and layer diagram.
See [Reactive Signals](02-reactive-signals.md) for the signal system deep dive.
See [Cross-Stream Store](06-cross-stream-store.md) for the local-first event store.

## Philosophy

Datastar's design follows three principles:

1. **Zero build step.** No transpilation, no bundling, no framework CLI. Write HTML, load one script, go.
2. **Local-first storage.** Events live on disk via LSM-tree and content-addressable files. The server is not a database — it is a pipe.
3. **Composability over configuration.** Each plugin does one thing. Each event type is independent. Nushell closures handle routing logic.
