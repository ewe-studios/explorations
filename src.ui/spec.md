# OpenUI Ecosystem -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ui/`
- **Language:** TypeScript/JavaScript (monorepo)
- **Author:** OpenUI contributors
- **License:** MIT
- **Sub-projects:** 12+ sub-projects including openui core framework, openclaw-ui integration, examples, voice agent, analytics, CLI tooling, plus src.nexuio (agent platforms) and src.OrvaStudios (creative tools)

## What This Project Is

OpenUI is a full-stack Generative UI framework for building AI-powered chat and copilot interfaces. The core innovation is **OpenUI Lang** — a compact, streaming-first DSL that LLMs emit as structured UI markup. The markup is progressively parsed, schema-aware materialized, and rendered as React components. The ecosystem includes an OpenClaw agent integration (server-side plugin + Next.js client), 20+ example apps, and voice/streamlit integrations.

## Documentation Goal

A reader should understand:

1. How OpenUI Lang works (lexer, streaming parser, AST, materializer, evaluator)
2. How the progressive streaming parser handles incomplete tokens during LLM streaming
3. How the materializer resolves references and maps positional args via JSON Schema
4. How the React renderer integrates with the framework-agnostic core
5. How the component library provides 60+ prebuilt UI components
6. How the OpenClaw integration works (gateway socket, engine, plugin system)
7. How storage works (JSON files, SQLite, localStorage) — formats, validation, limits
8. How to replicate these patterns in Rust
9. What a production-grade generative UI system looks like

## Documentation Structure

```
src.ui/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-lang-core.md
│   ├── 03-streaming-parser.md
│   ├── 04-materializer.md
│   ├── 05-evaluator.md
│   ├── 06-react-renderer.md
│   ├── 07-component-library.md
│   ├── 08-openclaw-plugin.md
│   ├── 09-gateway-socket.md
│   ├── 10-storage-patterns.md
│   ├── 11-rust-equivalents.md
│   ├── 12-production-patterns.md
│   └── 13-wasm-web-patterns.md
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
| 4 | Write 02-lang-core.md | DONE |
| 5 | Write 03-streaming-parser.md | DONE |
| 6 | Write 04-materializer.md | DONE |
| 7 | Write 05-evaluator.md | DONE |
| 8 | Write 06-react-renderer.md | DONE |
| 9 | Write 07-component-library.md | DONE |
| 10 | Write 08-openclaw-plugin.md | DONE |
| 11 | Write 09-gateway-socket.md | DONE |
| 12 | Write 10-storage-patterns.md | DONE |
| 13 | Write 11-rust-equivalents.md | DONE |
| 14 | Write 12-production-patterns.md | DONE |
| 15 | Write 13-wasm-web-patterns.md | DONE |
| 16 | Write README.md (index) | DONE |
| 17 | Generate HTML with build.py | DONE |
| 18 | Grandfather Review | DONE |
| 19 | Write 14-nexuio-ecosystem.md | DONE |
| 20 | Write 15-orvastudios-ecosystem.md | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations
python3 build.py src.ui
```

## Quality Requirements

All ten Iron Rules from the documentation directive apply.

## Expected Outcome

After reading, an engineer should be able to understand the OpenUI Lang parser, build a streaming-first generative UI system, replicate patterns in Rust, and design production-grade copilot interfaces.

## Resume Point

Check the task table above for the current phase.
