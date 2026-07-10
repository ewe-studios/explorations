# TEN Framework Spec — Documentation Project Tracker

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.TenFramework/`
- **Language:** Rust (CLI + tooling), C (core runtime), Python/Go/C++/Node.js (extensions)
- **License:** Apache 2.0
- **Scale:** ~4.7M lines total, 95+ pre-built extensions, 30 agent examples

## What the Project Is

TEN (Transformational Extension Network) is an open-source framework for building real-time multi-modal conversational AI agents. It uses a graph-based message-passing architecture: extensions (STT, LLM, TTS, tools) connect as nodes in a directed graph, and messages (commands, data, audio frames, video frames) flow along edges. The C core runtime handles sub-millisecond message routing. A Rust CLI (`tman`) manages packages with SAT-based dependency resolution and provides a visual graph editor. Extensions are written in Python, Go, C++, or Node.js.

## Documentation Goal

After reading this documentation, a developer should be able to:
1. Understand TEN's architecture, extension system, and graph-based message passing
2. Build and install extensions using any supported language SDK
3. Configure and deploy agents using the CLI and Designer
4. Integrate VAD and turn detection for voice agents
5. Use the playground for real-time agent interaction
6. Contribute to the TEN Framework codebase

## Documentation Structure

```
markdown/
├── README.md                    ← Index / table of contents
├── 00-overview.md               ← Architecture, extensions, agents, VAD at a glance
├── 01-architecture.md           ← Full dependency graph, layer diagram, module map
├── 02-ten-rust-runtime.md       ← ten_rust crate: graph, schema, service_hub, log, pkg_info
├── 03-tman-cli.md               ← CLI commands, SAT solver, registry, install pipeline
├── 04-graph-system.md           ← Graph model, nodes, connections, subgraphs, selectors
├── 05-extension-system.md       ← Package types, manifest.json, property.json, lifecycle
├── 06-designer-api.md           ← HTTP endpoints, WebSockets, state management
├── 07-message-protocols.md      ← cmd, data, audio_frame, video_frame, msg_conversion
├── 08-turn-detection.md         ← Qwen2.5-7B model, 3-class classification
├── 09-vad-library.md            ← ONNX inference, mel features, pitch estimation
├── 10-portal-playground.md      ← Next.js docs site, interactive guides, i18n
├── 11-data-flow.md              ← End-to-end conversation pipeline, zero-copy audio
├── 12-cross-cutting.md          ← Logging, telemetry, crypto, configuration
├── 13-sdk-bindings.md           ← Python/Go/C++/Node.js SDKs, extension APIs
├── 14-core-extensions.md        ← Default extension templates (ASR, LLM, TTS, MLLM)
├── 15-agent-server.md           ← Go server, worker management, Agora RTC
├── 16-playground-ui.md          ← React playground, Redux, RTC/RTM managers
├── 17-build-system.md           ← GN build system, Cargo, CI/CD pipelines
├── 18-testing.md                ← Test infrastructure, integration tests, test utilities
└── 19-extension-examples.md     ← Example extensions catalog (ollama, pcm, websocket, PIL)
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | 00-overview.md — Architecture overview | DONE |
| 2 | 01-architecture.md — Layers, modules, dependency graph | DONE |
| 3 | 02-ten-rust-runtime.md — ten_rust crate deep dive | DONE |
| 4 | 03-tman-cli.md — CLI commands and SAT solver | DONE |
| 5 | 04-graph-system.md — Graph model, nodes, connections | DONE |
| 6 | 05-extension-system.md — Packages, manifest, property | DONE |
| 7 | 06-designer-api.md — Designer HTTP API | DONE |
| 8 | 07-message-protocols.md — Message types and conversions | DONE |
| 9 | 08-turn-detection.md — Semantic turn detection | DONE |
| 10 | 09-vad-library.md — VAD with ONNX inference | DONE |
| 11 | 10-portal-playground.md — Portal documentation site | DONE |
| 12 | 11-data-flow.md — End-to-end data flow | DONE |
| 13 | 12-cross-cutting.md — Logging, telemetry, crypto | DONE |
| 14 | 13-sdk-bindings.md — Python/Go/C++/Node.js SDKs | DONE |
| 15 | 14-core-extensions.md — Default extension templates | DONE |
| 16 | 15-agent-server.md — Go server, worker management | DONE |
| 17 | 16-playground-ui.md — React playground, Redux, RTC | DONE |
| 18 | 17-build-system.md — GN build, Cargo, CI/CD | DONE |
| 19 | 18-testing.md — Test infrastructure and utilities | DONE |
| 20 | 19-extension-examples.md — Example extensions catalog | DONE |

## Build System

```bash
cd [src.AI]/ && python3 build.py src.TenFramework
```

**Script:** `documentation/build.py`
**Dependencies:** None (Python 3.12+ stdlib only)

## Quality Requirements

The Iron Rules from `documentation_directive.md`:
1. Detailed sections with actual code snippets
2. Teach key facts quickly — first paragraph is the thesis
3. Clear articulation — one idea per sentence
4. Minimum 2 mermaid diagrams per document
5. Good visual assets — tables, mermaid, code blocks
6. Generated HTML with navigation
7. Cross-references between documents
8. Source path references with file paths and line numbers
9. At least 1 Aha moment per document
10. Consistent navigation with prev/next buttons

## Expected Outcome

A reader can understand the TEN Framework deeply enough to build extensions, configure agents, and contribute to the codebase without reading the source.

## Resume Point

All 20 documents are complete. Run grandfather review to verify all claims against source.
