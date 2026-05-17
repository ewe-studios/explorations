# connect-rust — ConnectRPC for Rust

Tower-based ConnectRPC implementation supporting Connect, gRPC, and gRPC-Web protocols. MSRV 1.88.

## Source

- **Package:** `connectrpc`
- **Language:** Rust (2024 edition)
- **182 source files across 4 workspace crates**
- **Protobuf:** Uses `buffa` for zero-copy message views

## Documentation

| Document | Description |
|----------|-------------|
| [00-overview](markdown/00-overview.md) | Architecture, protocols, workspace structure |
| [01-server-dispatch](markdown/01-server-dispatch.md) | Tower service, dynamic/monomorphic dispatch, error handling |
| [02-client-streaming](markdown/02-client-streaming.md) | HttpClient, Http2Connection, streaming, TLS |
| [03-codegen-build](markdown/03-codegen-build.md) | Code generation, protoc plugin, build.rs integration |

## Key Features

- **Three protocols** — Connect, gRPC over HTTP/2, gRPC-Web over HTTP/1.1
- **Four RPC kinds** — Unary, Server Stream, Client Stream, Bidi Stream
- **Zero-copy requests** — Handlers receive `OwnedView<RequestView<'static>>`
- **Tower middleware** — Any Tower layer composes on top
- **Honest poll_ready** — `Http2Connection` with real readiness
- **Code generation** — Traits, servers, clients from `.proto` files
