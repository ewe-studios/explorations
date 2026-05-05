# http-nu -- Overview

## What Is http-nu?

http-nu is a performant HTTP server where every route handler is a Nushell closure. You write a script that returns a closure, and http-nu runs it for every request. It's designed to be the web server for the Datastar/xs ecosystem — with first-class support for SSE streaming, reactive frontends, and event-driven backends.

## Quick Start

```nushell
# Minimal server: echo request back as JSON
http-nu :3000 -c '{|req| $req | to json}'

# From a script file
http-nu :3000 serve.nu

# With cross.stream store + Datastar
http-nu --store ./store :3000 serve.nu
```

### Example serve.nu

```nushell
{|req|
    match [$req.method, $req.path] {
        ["GET", "/"] => {
            {status: 200, headers: {"content-type": "text/html"}, body: "<h1>Hello!</h1>"}
        }
        ["GET", "/api/data"] => {
            {body: ({items: [1, 2, 3]} | to json)}
        }
        _ => {status: 404, body: "not found"}
    }
}
```

## Key Features

| Feature | Description |
|---------|-------------|
| Nushell scripting | Route handlers as closures with full Nushell power |
| Hot reload | `--watch` reloads script on file change (zero downtime) |
| cross.stream | `--store` enables `.cat`, `.append`, `.cas` commands |
| Datastar stdlib | Built-in `datastar` module for SSE streaming to reactive frontends |
| TLS | `--tls cert.pem` for HTTPS |
| Brotli compression | Automatic response compression |
| Static files | tower-http ServeDir for static assets |
| Templates | MiniJinja templates with auto-escaping |
| Syntax highlighting | syntect-powered code highlighting |
| Markdown rendering | pulldown-cmark for server-side markdown |
| Plugins | Load Nushell plugins (`--plugin path`) |
| Logging | Human-readable or JSONL structured logs |

## Design Philosophy

1. **Script-first** — Business logic in Nushell. Rust handles the performance-critical plumbing.
2. **Composable** — Integrates with xs for persistence, yoke for AI, Datastar for frontend.
3. **Zero-config** — Sensible defaults. One binary, one script, one command.
4. **Hot reload** — Edit your script, server picks up changes automatically.
5. **Production-ready** — TLS, compression, graceful shutdown, structured logging.

## Key Dependencies

| Crate | Version | Role |
|-------|---------|------|
| hyper | 1 | HTTP/1.1 server |
| tokio | 1 | Async runtime |
| nu-* | 0.112.1 | Embedded Nushell |
| cross-stream | 0.12.0 | Event store integration (optional) |
| rustls/tokio-rustls | latest | TLS |
| minijinja | 2 | Template engine |
| syntect | 5.3 | Syntax highlighting |
| pulldown-cmark | 0.12 | Markdown → HTML |
| tower-http | 0.6 | Static file serving |
| brotli | 8 | Response compression |
| notify | 8 | File watching for hot reload |
| arc-swap | 1.7 | Lock-free engine hot-swap |
| scru128 | 3 | Request IDs |

## Module Layout

```rust
pub mod commands;     // Custom Nushell commands for HTTP context
pub mod compression;  // Brotli response compression
pub mod engine;       // Nushell engine setup and script loading
pub mod handler;      // HTTP request handler (routes to Nushell)
pub mod listener;     // TCP/TLS/UDS listener abstraction
pub mod logging;      // Structured logging (human/JSONL)
pub mod request;      // HTTP request → Nushell Value conversion
pub mod response;     // Nushell Value → HTTP response conversion
pub mod stdlib;       // Built-in Nushell modules (datastar, html, http, router)
pub mod store;        // cross.stream Store wrapper
pub mod worker;       // Worker pool for request processing
```
