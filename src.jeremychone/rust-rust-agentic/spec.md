# rust-agentic — Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-agentic/`
- **Crate name:** `agentic`
- **Language:** Rust
- **Type:** Library crate
- **Files:** 54 source files across 12 modules
- **Dependencies:** serde, serde_json, serde_with, derive_more, tokio, flume, reqwest, eventsource-stream, futures, tracing, rpc-router (for RpcId)

## What the Project Is

`rust-agentic` is an MCP (Model Context Protocol) client library implementing protocol version `2025-03-26`. It provides typed request/response/notification types for tools, resources, prompts, sampling, logging, and completion; stdio and HTTP transports; a `Client` with DashMap response queue and async runner tasks; and a sampling handler abstraction for LLM proxying.

## Documentation Structure

```
src.jeremychone/rust-rust-agentic/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   ├── 00-overview.md          ← Architecture, MCP protocol, Client lifecycle, transport system
│   ├── 01-mcp-messages.md      ← McpMessage, McpRequest, McpResponse, McpNotification, McpError, traits
│   ├── 02-mcp-types.md         ← Tools, Resources, Prompts, Sampling, Capabilities, common types
│   └── 03-mcp-transports.md    ← Client architecture, stdio/HTTP transports, trx channels, sampling handler
├── html/                       ← Generated HTML
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-mcp-messages.md | DONE |
| 4 | Write 02-mcp-types.md | DONE |
| 5 | Write 03-mcp-transports.md | DONE |
| 6 | Write README.md | DONE |
| 7 | Write spec.md | DONE |
| 8 | Generate HTML (build.py) | DONE |
| 9 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-agentic
```
