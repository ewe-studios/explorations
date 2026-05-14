# Datastar Core Library -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.datastar/datastar/library/src/`
- **Language:** TypeScript (ES2021, DOM, DOM.Iterable)
- **Version:** 1.0.1
- **Author:** Ben Croker / starfederation
- **License:** MIT
- **Bundle size:** 11.80 KiB

## What This Project Is

Datastar is a lightweight reactive frontend framework that uses HTML `data-*` attributes for declarative reactivity — similar to Alpine.js in API shape but with a fine-grained reactive signal system (like Solid.js) and server-sent events for DOM patching (like htmx). The core library is ~38 source files organized into engine, plugins, utils, and bundles.

## Documentation Goal

A reader should understand:

1. How Datastar's reactive signal system works (ReactiveNode, Link, propagation, batching, diamond dependency resolution)
2. How the genRx expression compiler converts `data-*` attribute values to executable JS Functions
3. How the plugin system works (AttributePlugin, ActionPlugin, WatcherPlugin)
4. How every attribute plugin operates (bind, on, show, class, style, text, attr, effect, computed, init, indicator, ref, signals, json-signals, on-intersect, on-interval, on-signal-patch)
5. How the fetch action plugin handles SSE streaming with retry, form data, and signal filtering
6. How the DOM morphing algorithm preserves state (ID-set matching, pantry pattern, soft matching)
7. How watchers (patchElements, patchSignals) integrate with the engine
8. How to replicate these systems in Rust
9. How the Rust Server SDK generates typed SSE events (datastar-rust crate)
10. What a production-grade version looks like

## Documentation Structure

```
src.datastar/datastar/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-reactive-signals.md
│   ├── 03-expression-compiler.md
│   ├── 04-plugin-system.md
│   ├── 05-attribute-plugins.md
│   ├── 06-action-plugins.md
│   ├── 07-dom-morphing.md
│   ├── 08-sse-streaming.md
│   ├── 09-watchers.md
│   ├── 10-utility-systems.md
│   ├── 11-rust-equivalents.md
│   ├── 12-production-patterns.md
│   ├── 13-web-tooling.md
│   ├── 14-datastar-rust-sdk.md
│   └── 17-rendering-signals-deep-dive.md
├── html/
│   ├── index.html
│   ├── styles.css
│   └── *.html
```

## Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Create spec.md | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-reactive-signals.md | DONE |
| 5 | Write 03-expression-compiler.md | DONE |
| 6 | Write 04-plugin-system.md | DONE |
| 7 | Write 05-attribute-plugins.md | DONE |
| 8 | Write 06-action-plugins.md | DONE |
| 9 | Write 07-dom-morphing.md | DONE |
| 10 | Write 08-sse-streaming.md | DONE |
| 11 | Write 09-watchers.md | DONE |
| 12 | Write 10-utility-systems.md | DONE |
| 13 | Write 11-rust-equivalents.md | DONE |
| 14 | Write 12-production-patterns.md | DONE |
| 15 | Write 13-web-tooling.md | DONE |
| 16 | Write 14-datastar-rust-sdk.md | DONE |
| 17 | Write README.md (index) | DONE |
| 18 | Generate HTML with build.py | DONE |
| 19 | Grandfather Review + Fix Gaps | DONE |
| 20 | Merge 18-signals into 02-reactive-signals.md | DONE |
| 21 | Deepen all remaining markdown docs to line-by-line depth | DONE |
| 22 | Rebuild HTML with build.py | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations
python3 build.py src.datastar
```

Python 3.12+ stdlib only, zero dependencies. Idempotent.

## Quality Requirements

All ten Iron Rules from the documentation directive apply. Every document must have 2+ mermaid diagrams, 3+ code snippets with file paths, 1+ Aha moment, and links to 2+ related documents.

## Expected Outcome

After reading, an engineer should be able to:
- Understand Datastar's reactive architecture deeply
- Replicate the signal system, DOM morphing, and SSE patterns in Rust
- Build a custom plugin for Datastar
- Understand the expression compilation pipeline

## Resume Point

If interrupted, check the task table above for the current phase. Each document is independent once written.
