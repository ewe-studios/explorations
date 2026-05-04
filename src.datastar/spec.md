# Datastar Ecosystem -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.datastar/`
- **Language:** TypeScript (core library), Rust (SDK, agents, stores), Zig (HTTP server)
- **Author:** cablehead (hello@orbitinghail.dev)
- **License:** MIT (varies by sub-project)
- **Sub-projects:** 14 git clones/submodules covering Datastar, cross-stream, yoke, yoagent, http-nu, pai-sho, nushell, and more

## What This Project Is

Datastar is a lightweight reactive frontend framework (similar to Alpine.js or htmx) that uses HTML `data-*` attributes for declarative reactivity. The ecosystem around it — cross-stream (xs), yoke, yoagent, http-nu, and pai-sho — forms a complete local-first, Nushell-scriptable application stack. Datastar provides fine-grained signals, DOM morphing, SSE streaming, and a plugin architecture that lets you build interactive UIs with zero build step.

## Documentation Goal

A reader should understand:

1. How Datastar's reactive signal system works from first principles (versioned dependency graph, lazy propagation, diamond handling)
2. How the DOM morphing algorithm preserves state across patches (persistent IDs, pantry pattern, view transitions)
3. How SSE streaming connects server to client with automatic retry and signal filtering
4. How cross-stream (xs) provides a local-first event streaming store (LSM-tree indexing, content-addressable storage, Scru128 IDs)
5. How yoke and yoagent form an LLM agent harness (JSONL protocol, tool execution, context management)
6. How http-nu makes routes scriptable in Nushell with Datastar integration
7. How to replicate these systems in Rust
8. What a production-grade version of each component looks like

## Documentation Structure

```
src.datastar/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-reactive-signals.md
│   ├── 03-plugin-system.md
│   ├── 04-dom-morphing.md
│   ├── 05-sse-streaming.md
│   ├── 06-cross-stream-store.md
│   ├── 07-yoke-agent.md
│   ├── 08-http-nu.md
│   ├── 09-rust-equivalents.md
│   ├── 10-production-patterns.md
│   ├── 11-wasm-web-patterns.md
│   └── 12-algorithms-deep-dive.md
├── html/
│   ├── index.html
│   ├── styles.css
│   └── *.html
└── build.py (shared from parent)
```

## Tasks

| Phase | Task | Status |
|-------|------|--------|
| 1 | Create spec.md | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-architecture.md | DONE |
| 4 | Write 02-reactive-signals.md | DONE |
| 5 | Write 03-plugin-system.md | DONE |
| 6 | Write 04-dom-morphing.md | DONE |
| 7 | Write 05-sse-streaming.md | DONE |
| 8 | Write 06-cross-stream-store.md | DONE |
| 9 | Write 07-yoke-agent.md | DONE |
| 10 | Write 08-http-nu.md | DONE |
| 11 | Write 09-rust-equivalents.md | DONE |
| 12 | Write 10-production-patterns.md | DONE |
| 13 | Write 11-wasm-web-patterns.md | DONE |
| 14 | Write 12-algorithms-deep-dive.md | DONE |
| 15 | Write README.md (index) | DONE |
| 16 | Generate HTML with build.py | DONE |
| 17 | Grandfather Review | TODO |

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
- Build a local-first streaming store with proper indexing and CAS
- Design an LLM agent harness with JSONL protocol
- Script an HTTP server in Nushell with reactive frontend

## Resume Point

If interrupted, check the task table above for the current phase. Each document is independent once written — no inter-document state to maintain.
