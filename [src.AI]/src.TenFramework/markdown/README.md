# TEN Framework — Real-Time Multi-Modal AI Agent Platform

Graph-based message-passing architecture for building conversational AI agents. 95 pre-built extensions, 30 agent examples, C/Rust/Python/Go/Node.js.

## Documents

| # | Document | What You'll Learn |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | Architecture, extensions, agents, VAD, turn detection |
| 01 | [Architecture](01-architecture.md) | Full dependency graph, layer diagram, module map, crate structure |
| 02 | [TEN Rust Runtime](02-ten-rust-runtime.md) | ten_rust crate: graph, schema, service_hub, log, pkg_info, value_buffer, crypto |
| 03 | [TMAN CLI](03-tman-cli.md) | CLI commands, SAT solver, registry, install pipeline, templates |
| 04 | [Graph System](04-graph-system.md) | Graph model, nodes, connections, subgraphs, selectors, flattening, proxy injection |
| 05 | [Extension System](05-extension-system.md) | Package types, manifest.json, property.json, lifecycle, templates |
| 06 | [Designer API](06-designer-api.md) | HTTP endpoints, WebSockets, state management, storage, frontend serving |
| 07 | [Message Protocols](07-message-protocols.md) | cmd, data, audio_frame, video_frame, msg_conversion, compatibility |
| 08 | [Turn Detection](08-turn-detection.md) | Qwen2.5-7B model, 3-class classification, test set, performance |
| 09 | [VAD Library](09-vad-library.md) | ONNX inference, mel features, pitch estimation, language bindings |
| 10 | [Portal Playground](10-portal-playground.md) | Next.js docs site, interactive guide, blog, i18n, deployment |
| 11 | [Data Flow](11-data-flow.md) | End-to-end conversation pipeline, message routing, zero-copy audio |
| 12 | [Cross-Cutting](12-cross-cutting.md) | Logging, telemetry, crypto, configuration, environment detection |
| 13 | [SDK Bindings](13-sdk-bindings.md) | Python/Go/C++/Node.js SDKs, GIL management, async patterns, handle registry |
| 14 | [Core Extensions](14-core-extensions.md) | Default extension templates for ASR, LLM, TTS, MLLM, async |
| 15 | [Agent Server](15-agent-server.md) | Go HTTP server, worker management, Agora RTC, RAG pipeline |
| 16 | [Playground UI](16-playground-ui.md) | React playground, Redux state, RTC/RTM managers, graph editor |
| 17 | [Build System](17-build-system.md) | GN build, Cargo integration, CI/CD pipelines, Docker environment |
| 18 | [Testing](18-testing.md) | Test infrastructure, integration tests, guarder tests, CI execution |
| 19 | [Extension Examples](19-extension-examples.md) | Ollama, PCM reader, WebSocket, PIL, simple echo, agent graphs |
