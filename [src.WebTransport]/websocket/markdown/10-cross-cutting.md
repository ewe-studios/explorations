---
title: Cross-Cutting — TLS, Feature Flags, Deprecated rust-websocket
---

# Cross-Cutting Concerns — TLS, Feature Flags, Deprecated rust-websocket

## TLS Support

tungstenite-rs supports two TLS backends via feature flags:

| Feature | Backend | Use Case |
|---------|---------|----------|
| `native-tls` | native-tls (SChannel/SecureTransport/OpenSSL) | System TLS |
| `native-tls-vendored` | Vendored OpenSSL | Self-contained builds |
| `rustls-tls-native-roots` | rustls with system root certs | Modern pure-Rust TLS |
| `rustls-tls-webpki-roots` | rustls with webpki-roots | Embedded root certs |

Source: `tungstenite-rs/Cargo.toml:1` — TLS feature flags.

**Aha:** The `rustls-tls-*` features use an internal `__rustls-tls` feature that enables `rustls` and `rustls-pki-types`. The `native-tls-vendored` feature chains to `native-tls-crate/vendored`. This layered feature design keeps the default build lightweight while allowing full TLS configuration.

## rust-websocket (DEPRECATED)

```
rust-websocket v0.27.0 — DEPRECATED
```

The original WebSocket implementation for Rust is deprecated due to:
- Old tokio 0.1 dependency
- Old futures 0.1 dependency
- hyper 0.10 dependency
- Not maintained

Source: `rust-websocket/Cargo.toml:1` — Description includes "[deprecated]".

### rust-websocket Workspace

| Crate | Purpose |
|-------|---------|
| `websocket` | Main crate |
| `websocket-base` | Base implementation |

### rust-websocket Features

| Feature | Purpose |
|---------|---------|
| `sync` | Synchronous API |
| `sync-ssl` | Sync with TLS |
| `async` | Async API (tokio 0.1) |
| `async-ssl` | Async with TLS |
| `nightly` | Nightly Rust features |

Source: `rust-websocket/Cargo.toml:1`

## sunrise and sunrise-dom

TypeScript reactive libraries by Snapview (same organization as tungstenite-rs).

| Package | Version | Purpose |
|---------|---------|---------|
| `@snapview/sunrise` | 0.0.10 | Reactive spreadsheet-driven UI |
| `@snapview/sunrise-dom` | 1.0.0 | DOM bindings for Sunrise |

Source: `sunrise/package.json:1`, `sunrise-dom/package.json:1`

## websocat: Old Dependencies

websocat uses deprecated dependencies:

| Dependency | Version | Current |
|------------|---------|---------|
| tokio | 0.1 | 1.x |
| futures | 0.1 | 0.3 |
| websocket | 0.27.1 | tungstenite-rs 0.24 |

Source: `websocat/Cargo.toml:1`

## Related Documents

- [Overview](../markdown/00-overview.md) — WebSocket ecosystem
- [Architecture](../markdown/01-architecture.md) — Feature flags table
