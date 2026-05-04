# Datastar Ecosystem -- Documentation Index

## Foundation

- [Overview](00-overview.md) — What Datastar is, the ecosystem, philosophy
- [Architecture](01-architecture.md) — Layer diagram, dependency graph, entry points, technology stack
- [Reactive Signals](02-reactive-signals.md) — Signal system, dependency graph, dirty propagation, deep proxy
- [Plugin System](03-plugin-system.md) — Attribute/Action/Watcher plugins, expression compilation
- [DOM Morphing](04-dom-morphing.md) — Morphing algorithm, persistent IDs, pantry pattern, view transitions
- [SSE Streaming](05-sse-streaming.md) — SSE protocol, client parser, server generation (Rust/Zig), retry

## Deep Dives

- [Cross-Stream Store](06-cross-stream-store.md) — Event store, Scru128 IDs, CAS, TTL, read API
- [Yoke Agent](07-yoke-agent.md) — JSONL protocol, agent loop, tool system, session persistence
- [HTTP-NU Server](08-http-nu.md) — Nushell scripting, custom commands, route handlers, compression
- [Rust Equivalents](09-rust-equivalents.md) — TypeScript → Rust mapping, production patterns
- [Production Patterns](10-production-patterns.md) — Reliability, observability, scaling, security
- [WASM and Web Patterns](11-wasm-web-patterns.md) — WASM candidates, build config, memory management
- [Algorithms Deep Dive](12-algorithms-deep-dive.md) — Signal propagation, morphing complexity, CAS verification
