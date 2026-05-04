# OpenUI Ecosystem -- Documentation Index

## Foundation

- [Overview](00-overview.md) — What OpenUI is, ecosystem map, philosophy
- [Architecture](01-architecture.md) — Layer diagram, dependency graph, entry points, technology stack
- [Lang Core](02-lang-core.md) — Lexer (36 token types), AST, discriminated unions
- [Streaming Parser](03-streaming-parser.md) — Watermark mechanism, incremental parsing, edit/merge
- [Materializer](04-materializer.md) — Schema-aware lowering, positional-to-named prop mapping
- [Evaluator](05-evaluator.md) — AST interpreter, builtins, lazy Each, action plans

## Deep Dives

- [React Renderer](06-react-renderer.md) — Renderer component, useOpenUIState, error boundary
- [Component Library](07-component-library.md) — 60+ components, form handling, tool provider
- [OpenClaw Plugin](08-openclaw-plugin.md) — Server plugin, tool registration, stores
- [Gateway Socket](09-gateway-socket.md) — WebSocket RPC, challenge auth, reconnection
- [Storage Patterns](10-storage-patterns.md) — JSON files, SQLite, localStorage, atomic writes
- [Rust Equivalents](11-rust-equivalents.md) — Parser/materializer/evaluator in Rust, production alternatives
- [Production Patterns](12-production-patterns.md) — Streaming reliability, LLM error handling, scaling
- [WASM and Web Patterns](13-wasm-web-patterns.md) — Stream adapters, edge deployment, WASM candidates
