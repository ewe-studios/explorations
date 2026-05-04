# Datastar Core Library -- Documentation

A lightweight reactive frontend framework (11.80 KiB) that uses HTML `data-*` attributes for declarative reactivity, fine-grained signals for state management, and SSE streaming for server-driven DOM updates.

## Foundation

- [Overview](00-overview.md) — What Datastar is, philosophy, architecture at a glance
- [Architecture](01-architecture.md) — Module dependency graph, layers, bundles, initialization

## Core Systems

- [Reactive Signals](02-reactive-signals.md) — ReactiveNode, Link, propagation, batching, diamond resolution, deep signals
- [Expression Compiler](03-expression-compiler.md) — genRx, signal reference rewriting, template interpolation, action invocation
- [Plugin System](04-plugin-system.md) — AttributePlugin, ActionPlugin, WatcherPlugin, modifier system, DOM scanning

## Plugin Deep Dives

- [Attribute Plugins](05-attribute-plugins.md) — All 17 attribute plugins: bind, on, show, class, style, text, attr, effect, computed, init, indicator, ref, signals, json-signals, on-intersect, on-interval, on-signal-patch
- [Action Plugins](06-action-plugins.md) — peek, setAll, toggleAll, fetch (with SSE client)

## DOM and Streaming

- [DOM Morphing](07-dom-morphing.md) — ID-set matching, pantry pattern, soft matching, script execution
- [SSE Streaming](08-sse-streaming.md) — SSE parsing pipeline, retry logic, content-type handling
- [Watchers](09-watchers.md) — patchElements and patchSignals watcher plugins

## Cross-Cutting

- [Utility Systems](10-utility-systems.md) — Case conversion, timing, path manipulation, math, DOM helpers
- [Rust Equivalents](11-rust-equivalents.md) — How to replicate Datastar's architecture in Rust
- [Production Patterns](12-production-patterns.md) — Memory management, security, performance, debugging
- [Web Tooling](13-web-tooling.md) — VS Code extension, IntelliJ plugin, SDK schema, build pipeline

## Server-Side SDK

- [Rust Server SDK](14-datastar-rust-sdk.md) — datastar crate (v0.3.2): DatastarEvent, PatchElements, PatchSignals, ExecuteScript, Axum/Rocket/Warp integrations, ReadSignals extractor
