# Orbitinghail -- WASM and Web Patterns

This document covers WASM compilation, browser integration, and web deployment patterns used in the orbitinghail ecosystem, primarily through sqlsync-wasm.

**Aha:** The sqlsync WASM build uses `cdylib` + `rlib` crate types — `cdylib` produces a `.wasm` file for browser loading, and `rlib` allows other Rust crates to depend on the library. The `tsify` crate generates TypeScript type definitions automatically, so the TypeScript side sees native `interface` types instead of `any`. This is a significant DX improvement over manual type definitions.

Source: `sqlsync-wasm/src/lib.rs` — WASM bindings
Source: `sqlsync-wasm/Cargo.toml` — WASM build configuration

## WASM Build Configuration

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Crypto",
    "Window",
    "console",
]}
tsify = "0.4"
console_error_panic_hook = "0.1"

[target.'cfg(target_arch = "wasm32")'.dependencies]
worker = "0.6.7"  # Cloudflare Workers support
gloo = { git = "https://github.com/carlsverre/gloo" }  # From git, not crates.io
```

Build command:
```bash
wasm-pack build --target web
```

Output:
```
pkg/
├── sqlsync_wasm.d.ts      # TypeScript definitions (from tsify)
├── sqlsync_wasm.js         # JS glue code (from wasm-bindgen)
├── sqlsync_wasm_bg.wasm    # WASM binary
└── sqlsync_wasm_bg.wasm.d.ts
```

## TypeScript Bindings (tsify)

```rust
// Rust side
use tsify::Tsify;

#[derive(Serialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ReplicationRange {
    pub start_lsn: u64,
    pub end_lsn: u64,
}
```

Generates:
```typescript
// TypeScript side
interface ReplicationRange {
    start_lsn: bigint;
    end_lsn: bigint;
}
```

**Aha:** tsify handles the type mapping automatically — `u64` becomes `bigint` in TypeScript (since JS numbers can't represent all u64 values). This prevents subtle bugs where large LSNs lose precision in JavaScript.

## Async Operations in WASM

```rust
#[wasm_bindgen]
pub async fn open_database(volume_id: &str) -> Result<JsDatabase, JsError> {
    let db = Database::open(volume_id).await?;
    Ok(JsDatabase::new(db))
}
```

The `#[wasm_bindgen]` macro converts Rust `async fn` to a JavaScript `Promise`. The WASM thread must be the main thread for `web-sys` DOM access, so async operations use `wasm-bindgen-futures` to yield to the JS event loop.

## WebCrypto Integration

```rust
use web_sys::Crypto;

// Use browser's WebCrypto for cryptographic operations
fn random_bytes(len: usize) -> Vec<u8> {
    let crypto = web_sys::window().unwrap().crypto().unwrap();
    let mut buf = vec![0u8; len];
    crypto.get_random_values_with_u8_array(&mut buf).unwrap();
    buf
}
```

This uses the browser's CSPRNG instead of `rand`. On Cloudflare Workers, the `worker` crate provides the same API.

## Cloudflare Workers

```rust
use worker::*;

#[event(fetch)]
async fn main(_req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    // SQLite runs on the edge via WASM
    let db = Database::open("my-volume").await?;
    let rows = db.query("SELECT * FROM users").await?;
    Response::from_json(&rows)
}
```

Cloudflare Workers run WASM on the edge. sqlsync's WASM compilation allows SQLite to run directly in Workers, with the VFS backed by Workers KV or R2 storage.

## Console Error Hook

```rust
// In main() for WASM
console_error_panic_hook::set_once();
```

This routes Rust panics to `console.error` in the browser. Without it, panics produce an unhelpful "unreachable executed" error in the WASM binary. With it, you get the full panic message, file, and line number.

## Memory Management

WASM has a linear memory model. Memory grows in 64KB pages. Key considerations:

1. **No garbage collection**: Rust's ownership model handles memory. WASM doesn't GC.
2. **Memory growth**: When the heap is full, WASM requests more pages from the browser. The browser may deny this if the tab's memory limit is reached.
3. **Shared memory**: `SharedArrayBuffer` enables multi-threaded WASM, but requires specific HTTP headers (`Cross-Origin-Opener-Policy: same-origin`).

For SQLite in WASM, the database file lives in WASM linear memory. A 100MB SQLite database requires 100MB of WASM memory.

## Service Worker Integration

A service worker can cache the WASM binary for offline operation:

```javascript
// Service worker
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open('sqlsync-v1').then((cache) => {
      return cache.addAll([
        '/sqlsync_wasm_bg.wasm',
        '/sqlsync_wasm.js',
      ]);
    })
  );
});
```

This enables the SQLite database to work fully offline — the WASM binary and all data are available without a network connection.

## Replicating in Rust

For a new WASM project:

```bash
cargo generate rustwasm/wasm-pack-template
cd my-wasm-project
```

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
console_error_panic_hook = "0.1"
```

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// Set up panic hook once
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}
```

See [SQLSync](07-sqlsync.md) for the SQLite WASM integration.
See [Rust Equivalents](11-rust-equivalents.md) for general Rust patterns.
See [Architecture](01-architecture.md) for the WASM layer in the ecosystem.
