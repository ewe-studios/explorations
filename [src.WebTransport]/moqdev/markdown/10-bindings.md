---
title: Bindings — Go, Swift, C FFI, JavaScript, Python
---

# Bindings — Go, Swift, C FFI, JavaScript, Python

MoQ provides bindings for multiple languages through FFI and native implementations.

## C FFI (libmoq)

```rust
// moq/rs/libmoq/src/
#[no_mangle]
pub extern "C" fn moq_connect(...) { ... }
```

Source: `moq/rs/libmoq/src/` — `staticlib` with `cbindgen`-generated C header.

## Go Bindings

| Package | Version | Method |
|---------|---------|--------|
| `moq-go` | v0.2.15 | FFI via cgo |
| `moq-go-ffi` | — | FFI mirror |

Source: `moq-go/` — Go bindings.

## Swift Bindings

| Package | Version | Method |
|---------|---------|--------|
| `moq-swift` | v0.3.0 | Swift Package Manager |
| `moq-swift-ffi` | — | FFI mirror |

Source: `moq-swift/Package.swift:1` — Swift package definition.

## JavaScript/TypeScript

`@moq/web-transport` — Node ESM + WASM bindings.

Source: `web-transport/rs/web-transport-node/` — Node.js bindings.

## Python

Python bindings via `moq-ffi` with UniFFI.

Source: `moq/py/` — Python bindings.

## Smoke Tests

Cross-language smoke tests in 7 languages:

| Language | Directory |
|----------|-----------|
| C | `smoke/clients/c/` |
| Go | `smoke/clients/go/` |
| JavaScript (native) | `smoke/clients/js-native/` |
| JavaScript (browser) | `smoke/clients/js/` |
| Kotlin | `smoke/clients/kotlin/` |
| Python | `smoke/clients/python/` |
| Swift | `smoke/clients/swift/` |

Source: `smoke/clients/` — Smoke test implementations.

**Aha:** The smoke tests cover 7 languages ensuring the MoQ protocol works consistently across all bindings. The C and Kotlin clients are often overlooked in FFI setups but are critical for production integrations (Android/Kotlin apps, C-based media tools).

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Module map
- [Overview](../markdown/00-overview.md) — Ecosystem overview
