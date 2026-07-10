---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/
language: C/C++, Rust, Go, Python, TypeScript
explored_at: 2026-07-09
license: Apache-2.0
---

# spec.md — TEN Framework Documentation

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/`
- **Language:** C/C++ (core runtime), Rust (CLI + validation), Go/Python/Node.js (bindings)
- **LOC:** ~4.7M lines across 4 sub-projects
- **License:** Apache 2.0

## What TEN Framework Is

TEN (Transformational Extension Network) is an open-source framework for building real-time multi-modal conversational AI agents. It provides a graph-based message-passing architecture where extensions (STT, LLM, TTS, tools) connect as nodes in a directed graph. The core runtime is C for low-latency execution; the package manager and graph editor are Rust; extensions are written in Python, Go, C++, or Node.js. Includes 95 pre-built extensions, 30 agent examples, a high-performance VAD library, and a Transformer-based turn-detection model.

## Sub-Projects

1. **ten-framework** — Core runtime + 95 extensions + TMAN CLI + agent examples
2. **ten-vad** — High-performance voice activity detection (~306KB)
3. **ten-turn-detection** — Qwen2.5-7B-based semantic turn detection
4. **portal** — Documentation website (Next.js + Fumadocs)

## Documentation Structure

```
src.TenFramework/
├── spec.md, build.py, markdown/, html/
├── markdown/
│   ├── README.md                         ← Index / table of contents
│   ├── 00-overview.md                    ← What it is, why it exists, architecture at a glance
│   ├── 01-architecture.md                ← Full dependency graph, layer diagram, module map
│   ├── 02-ten-rust-runtime.md            ← ten_rust crate deep dive (graph, schema, service_hub, log, pkg_info)
│   ├── 03-tman-cli.md                    ← ten_manager CLI (commands, solver, registry, install)
│   ├── 04-graph-system.md                ← Graph model, nodes, connections, message flows, subgraphs, selectors
│   ├── 05-extension-system.md            ← Package system, manifest.json, property.json, extension types
│   ├── 06-designer-api.md                ← Designer HTTP API, WebSocket endpoints, state management
│   ├── 07-message-protocols.md           ← cmd, data, audio_frame, video_frame, msg_conversion
│   ├── 08-turn-detection.md              ← ten-turn-detection deep dive (model, test set, training)
│   ├── 09-vad-library.md                 ← ten-vad deep dive (C API, ONNX, mel features, bindings)
│   ├── 10-portal-playground.md           ← Portal UI (Next.js, graph editor, module registry)
│   ├── 11-data-flow.md                   ← End-to-end data flow with sequence diagrams
│   └── 12-cross-cutting.md               ← Logging, telemetry, crypto, configuration
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all ten_rust crate source files | DONE |
| 2 | Read all ten_manager crate source files | DONE |
| 3 | Read ten-turn-detection source files | DONE |
| 4 | Read portal UI source files | DONE |
| 5 | Read ten-vad source files | DONE |
| 6 | Write 01-architecture.md | DONE |
| 7 | Write 02-ten-rust-runtime.md | DONE |
| 8 | Write 03-tman-cli.md | DONE |
| 9 | Write 04-graph-system.md | DONE |
| 10 | Write 05-extension-system.md | DONE |
| 11 | Write 06-designer-api.md | DONE |
| 12 | Write 07-message-protocols.md | DONE |
| 13 | Write 08-turn-detection.md | DONE |
| 14 | Write 09-vad-library.md | DONE |
| 15 | Write 10-portal-playground.md | DONE |
| 16 | Write 11-data-flow.md | DONE |
| 17 | Write 12-cross-cutting.md | DONE |
| 18 | Update README.md with document list | DONE |
| 19 | Update spec.md document structure | DONE |
| 20 | Run build.py to generate HTML | DONE |
| 21 | Grandfather review — verify names, numbers, flows, coverage | DONE |

## Build System

```bash
cd "[src.AI]/" && python3 build.py src.TenFramework
```

## Quality Requirements

All 10 Iron Rules from the documentation directive apply:
1. Detailed sections with code snippets
2. Teach key facts quickly
3. Clear articulation
4. Mermaid diagrams (minimum 2 per document)
5. Good visual assets
6. Generated HTML
7. Cross-references
8. Source path references
9. Aha moments
10. Navigation: Index + Prev/Next buttons

## Expected Outcome

A reader can understand TEN Framework's architecture, implement extensions, configure graphs, use the TMAN CLI, and debug agent issues — without reading the source code.

## Resume Point

All documents written, built, and grandfather reviewed. Ready for commit.
