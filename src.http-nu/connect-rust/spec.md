# connect-rust — Spec

## Source

- **Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.http-nu/src.connect-protocol/connect-rust/`
- **Language:** Rust (MSRV 1.88, 2024 edition)
- **Package:** `connectrpc` — Tower-based ConnectRPC implementation
- **182 Rust source files across 4 workspace crates**
- **Protobuf:** Uses `buffa` library for zero-copy message views

## What It Is

A Tower-based ConnectRPC implementation for Rust, providing server and client runtimes supporting the Connect, gRPC over HTTP/2, and gRPC-Web protocols. Uses the `buffa` protobuf library for zero-copy message views.

## Documentation Goal

1. Understand the Tower service architecture and dual dispatch strategy
2. Understand the client implementation (HttpClient, Http2Connection, streaming)
3. Understand the code generation (connectrpc-codegen, protoc plugin)
4. Understand the build integration (connectrpc-build)
5. Understand the protocol handling (Connect, gRPC, gRPC-Web)
6. Understand zero-copy request/response design

## Structure

```
connect-rust/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-server-dispatch.md
│   ├── 02-client-streaming.md
│   └── 03-codegen-build.md
├── html/
│   └── ... (generated)
```

## Tasks

| Task | Status |
|------|--------|
| Read all 182 source files | DONE |
| Write spec.md | DONE |
| Write 00-overview.md | DONE |
| Write 01-server-dispatch.md | DONE |
| Write 02-client-streaming.md | DONE |
| Write 03-codegen-build.md | DONE |
| Write README.md | DONE |
| Generate HTML via build.py | DONE |
| Grandfather review | DONE |

## Build System

```bash
python3 build.py src.http-nu/connect-rust
```

## Quality Requirements

Per documentation_directive.md — minimum 2 mermaid diagrams, 3 code snippets with file paths, 1 Aha moment per document. All names/numbers/flows verified against source.

## Expected Outcome

A reader can understand how to build a ConnectRPC server and client in Rust, leverage Tower middleware, generate zero-copy view handlers, and support all three wire protocols.

## Resume Point

Spec written. Need to write 4 markdown docs + README + build + review.
