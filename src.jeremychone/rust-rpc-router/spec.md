# rust-rpc-router — Spec

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.jeremychone/rust-rpc-router/`
- **Crate name:** `rpc-router`
- **Language:** Rust
- **Type:** Library crate (with companion `rpc-router-macros` proc-macro crate)
- **Files:** 32 source files across 7 modules
- **Dependencies:** serde, serde_json, uuid, bs58, base64, data-encoding, bitflags, derive_more, futures, serde_with

## What the Project Is

`rpc-router` is a JSON-RPC 2.0 compliant router that routes method calls to async handler functions. It provides a typed handler system with dependency injection via `Resources` (a type-map borrowed from `http` crate's extensions), parameter parsing via `IntoParams`, and full JSON-RPC request/response/notification parsing with validation.

## Documentation Structure

```
src.jeremychone/rust-rpc-router/
├── spec.md                     ← This file
├── markdown/
│   ├── README.md               ← Index
│   ├── 00-overview.md          ← Architecture, router, handler system, resources
│   ├── 01-rpc-messages.md      ← RpcRequest, RpcNotification, parsing, errors
│   └── 02-rpc-response.md      ← RpcResponse, RpcError, serialization
├── html/                       ← Generated HTML
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read all source files | DONE |
| 2 | Write 00-overview.md | DONE |
| 3 | Write 01-rpc-messages.md | DONE |
| 4 | Write 02-rpc-response.md | DONE |
| 5 | Write README.md | DONE |
| 6 | Write spec.md | DONE |
| 7 | Generate HTML (build.py) | DONE |
| 8 | Grandfather review | DONE |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations && python3 build.py src.jeremychone/rust-rpc-router
```
