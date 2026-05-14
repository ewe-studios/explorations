# Desktop vs CF — Feature Comparison Matrix

This document compares what works on desktop vs the CF target. The goal is to understand exactly which features are shared, which diverge, and which are unsupported on CF.

## Shared Features (Same Code)

These features work identically on both targets because they use shared code:

| Feature | Desktop | CF | Notes |
|---------|---------|-----|-------|
| Nushell closure as handler | Yes | Yes | `src/engine.rs` — same parse/run logic |
| `ArcSwap` hot-reload | Yes | Yes | Desktop: via `--watch` inotify; CF: Workspace `onChange` |
| Custom commands (`.mj`, `.bus`, `.run`, `.highlight`, `.md`) | Yes | Yes | Registered via `add_custom_commands()` |
| `{__html}` trust convention | Yes | Yes | Same escaping logic |
| Content-type inference | Yes | Yes | `src/response.rs::infer_content_type` |
| `RESPONSE_TX` oneshot | Yes | Yes | Dual-channel response model |
| Embedded Datastar JS | Yes | Yes | Served at `/datastar@1.0.1.js` |
| Admin handler swap | Yes | Yes | `PUT /admin/handler` (CF: per-user scope) |
| Event system | Yes | Partial | `Event` broadcast exists but no human/JSONL handler threads on CF |
| In-process bus (`.bus pub/sub`) | Yes | Yes | Same `Bus` implementation |

## Divergent Features

| Feature | Desktop | CF | Why |
|---------|---------|-----|-----|
| **Filesystem** | `std::fs` via `OsVfs` | Workspace (SQLite + R2) via `SnapshotVfs` | Workers has no disk |
| **`sleep`** | Blocks thread | NO-OP, capped at 64 calls/request | No sync sleep in Workers |
| **Static serving** | `tower-http::ServeDir` (async, efficient) | Workspace snapshot (sync, in-memory) | No `ServeDir` on wasm |
| **Hot-reload trigger** | `--watch` (notify crate, inotify) | Workspace `onChange` listener | Different notification substrate |
| **Path resolution** | `current_dir()` + relative | Workspace-rooted (`/path`) | No CWD on wasm |
| **Isolation** | Single process | DurableObject per user (`/u/<user>/`) | Workers scaling model |
| **Eval threading** | `spawn_eval_thread` (dedicated thread per request) | Sync eval (no thread, no tokio mpsc bridge) | No threads in wasm |
| **Response streaming** | `tokio::sync::mpsc` channel | `futures_util::stream` (async stream) | No tokio on wasm |

## Unsupported on CF

| Feature | Desktop | CF | Reason |
|---------|---------|-----|-----|
| **TLS** | rustls + HTTP/2 ALPN | N/A | Workers handles TLS at edge |
| **Reverse proxy** | `hyper::Client` forward | `"reverse proxy not supported on CF target"` | No HTTP client in `worker` crate for sync context |
| **Brotli compression** | `brotli` crate, streaming | N/A | Workers applies gzip/brotli transparently at edge |
| **Unix sockets** | Unix domain sockets | N/A | No Unix sockets on Workers |
| **`tracing` / logging** | Custom event system with human terminal UI + JSONL | `worker::console_log!` / `console_warn!` only | No terminal, no dedicated handler threads |
| **`fetch` / `http get`** | `hyper::Client` | Blocked (async, no sync HTTP in Workers) | Same blocker as reverse proxy |
| **`stor` commands** | `rusqlite` | Not yet shadowed | Needs `worker::SqlStorage` or D1 backend |
| **`ls --all`/`--long`** | Full stock `ls` | Only name/type/size | Workspace has no hidden-file convention |
| **`open` format dispatch** | All formats via MIME registry | Only `.json` auto-parsed | Other formats require explicit `\| from <fmt>` |
| **`cp --verbose`/`--interactive`** | Full `ucp` | Only `--recursive` | No TTY for interactive prompts |
| **`save --append`** | Append mode | Not implemented | Workspace has no append primitive |
| **`glob --exclude`/`--depth`** | Full stock `glob` | Custom recursive matcher, no exclusions | Simplified for Workspace |
| **`mkdir --verbose`** | Print created paths | Parsed but ignored | Simplified |
| **`mv --verbose`/`--interactive`** | Full `umv` | Read-then-write, no atomic rename | Workspace has no rename primitive |
| **`rm --trash`** | Trash integration | Permanent deletion only | No trash on Workspace |

## Feature Gates

The `cloudflare` Cargo feature (`Cargo.toml`) controls the split:

```toml
[features]
cloudflare = [
    "worker",
    "nu-command/js",        # wasm-compatible pure-data commands
    # ... other wasm-safe deps
]
```

When `cloudflare` is enabled:
- Desktop-only features are excluded (TLS, brotli, `tower-http`)
- `nu-command/js` is enabled, providing wasm-compatible versions of `date`, `random`, path utils, etc.
- `src/cf/` is compiled (`#[cfg(feature = "cloudflare")]`)
- `src/main.rs` desktop code is excluded (`#[cfg(feature = "desktop")]`)

## Build Comparison

| Target | Command | Features |
|--------|---------|----------|
| Desktop | `cargo build` | `default` (all desktop features) |
| CF | `worker-build --features cloudflare` | `cloudflare`, `--no-default-features` |
| CF dev | `mise run cf:dev` | Runs `wrangler dev` with local DO |

## Summary

The CF port is a **subset** of desktop functionality with **one key addition**: per-user DurableObject isolation with Workspace-backed filesystem. The shared code (engine, closure model, custom commands, response building) works identically on both targets thanks to the `Vfs` abstraction. The main losses are TLS, reverse proxy, and interactive command flags — all of which are edge-oriented features that the Workers runtime handles at a different layer anyway.

[← Back to Request Lifecycle](05-cf-request-lifecycle.md) | [← Back to Index](README.md)
