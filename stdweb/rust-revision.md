---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.stdweb
repository: https://github.com/koute/stdweb
revised_at: 2026-03-23
workspace: stdweb-modern
---

# Rust Revision: stdweb Ecosystem

## Overview

This document provides guidance for reproducing the key concepts from the stdweb ecosystem in modern Rust. The stdweb project pioneered many patterns in the Rust WASM space that have since been superseded by `wasm-bindgen`, `web-sys`, and `wasm-pack`. Rather than a direct port of stdweb itself (which is no longer needed), this revision focuses on the companion crates that remain architecturally interesting and the patterns that are worth preserving or modernizing.

**Sub-projects worth modernizing:**
- **picoalloc** - Already modern, actively maintained
- **speedy** - Already modern, actively maintained
- **recursion** - Already modern, actively maintained
- **object** - Already modern (gimli-rs), used by rustc
- **embed-wasm** - Pattern worth modernizing with trunk/wasm-pack
- **tracing-honeycomb** - Superseded by tracing-opentelemetry

**Sub-projects that are historical (no modern port needed):**
- **stdweb** - Superseded by wasm-bindgen + web-sys
- **cargo-web** - Superseded by wasm-pack + trunk

## Workspace Structure

```
stdweb-modern/
├── Cargo.toml                      # Workspace root
├── crates/
│   ├── embed-assets/               # Modernized embed-wasm concept
│   │   ├── embed-assets/           # Runtime crate
│   │   └── embed-assets-build/     # Build-time crate
│   ├── picoalloc/                  # Already idiomatic (fork as-is)
│   ├── speedy/                     # Already idiomatic (fork as-is)
│   ├── recursion/                  # Already idiomatic (fork as-is)
│   └── tracing-distributed/        # Modernized tracing layer
```

### Crate Breakdown

#### embed-assets (Modernized embed-wasm)
- **Purpose:** Embed WASM frontend build output in native Rust binaries
- **Type:** library (runtime) + library (build-time)
- **Public API:** `StaticAssets`, `include_assets!` macro, `AssetResponse`
- **Dependencies:** `axum` (replaces hyper), `phf`, `mime_guess`, `trunk` (replaces cargo-web)
- **Key change:** Use `trunk` instead of `cargo-web` for WASM compilation, support `axum::Response` natively

#### picoalloc (No changes needed)
- **Purpose:** Minimal memory allocator for WASM/embedded
- **Type:** library
- **Status:** Already idiomatic modern Rust with `no_std`, const generics, strict provenance
- **Note:** Version 1.1.0, actively maintained by original author

#### speedy (No changes needed)
- **Purpose:** Fast binary serialization
- **Type:** library + proc-macro
- **Status:** Already idiomatic with `no_std` support, comprehensive feature gates
- **Note:** Version 0.8.7, actively maintained

#### recursion (No changes needed)
- **Purpose:** Stack-safe recursion schemes
- **Type:** library
- **Status:** Uses GATs, zero dependencies, modern patterns
- **Note:** Version 0.5.2, uses cutting-edge Rust features

#### tracing-distributed (Modernized tracing-honeycomb)
- **Purpose:** Generic distributed tracing layer
- **Type:** library
- **Key change:** Update to modern tracing ecosystem, integrate with `tracing-opentelemetry`
- **Dependencies:** `tracing` 0.1.40+, `tracing-subscriber` 0.3+, `opentelemetry` 0.27+

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| WASM build tool | trunk | 0.21+ | Modern replacement for cargo-web |
| HTTP framework | axum | 0.8+ | Replaces hyper for embed-assets |
| Static hashing | phf | 0.11 | Perfect hash maps for asset lookup |
| MIME detection | mime_guess | 2 | Content-type inference |
| Serialization | serde + serde_json | 1.0 | Standard serialization |
| Binary serialization | speedy | 0.8 | Fast binary format (from this ecosystem) |
| Error handling | thiserror | 2.0 | Derive Error implementations |
| Tracing | tracing | 0.1.40+ | Instrumentation framework |
| OpenTelemetry | opentelemetry | 0.27+ | Distributed tracing standard |
| Async runtime | tokio | 1.0 | Async runtime for workers |
| Allocator | picoalloc | 1.1 | For WASM targets (from this ecosystem) |

## Type System Design

### Modernized embed-assets

```rust
use axum::response::Response;
use phf::Map;

/// Configuration for how root path requests are handled
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexHandling {
    /// Map requests to "/" to "index.html"
    MapRootToIndex,
    /// No special handling for root path
    None,
}

/// Static asset lookup backed by a compile-time perfect hash map.
pub struct StaticAssets {
    assets: &'static Map<&'static str, &'static [u8]>,
    index_handling: IndexHandling,
}

impl StaticAssets {
    /// Look up an asset by path and return an axum Response.
    pub fn get(&self, path: &str) -> Option<Response> {
        let path = path.strip_prefix('/').unwrap_or(path);
        let path = if path.is_empty() && self.index_handling == IndexHandling::MapRootToIndex {
            "index.html"
        } else {
            path
        };

        self.assets.get(path).map(|bytes| {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header("content-type", mime.as_ref())
                .header("content-length", bytes.len())
                .body(axum::body::Body::from(*bytes))
                .unwrap()
        })
    }
}

/// Macro to include compiled WASM assets.
/// Generates a static ASSETS: StaticAssets from build output.
#[macro_export]
macro_rules! include_assets {
    () => {
        include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

        pub static ASSETS: $crate::StaticAssets = $crate::StaticAssets::new(
            &ASSET_MAP,
            $crate::IndexHandling::MapRootToIndex,
        );
    };
}
```

### Modernized Tracing Layer

```rust
use tracing_subscriber::Layer;

/// Generic distributed tracing telemetry backend.
pub trait DistributedTelemetry: Send + Sync + 'static {
    type SpanId: Clone + Send + Sync + 'static;
    type TraceId: Clone + Send + Sync + 'static;

    fn report_span(&self, span: CompletedSpan<Self::SpanId, Self::TraceId>);
    fn report_event(&self, event: TracedEvent<Self::SpanId, Self::TraceId>);
}

/// A completed span with timing, fields, and parent context.
pub struct CompletedSpan<SpanId, TraceId> {
    pub name: &'static str,
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub parent_id: Option<SpanId>,
    pub start: std::time::Instant,
    pub duration: std::time::Duration,
    pub fields: Vec<(&'static str, serde_json::Value)>,
}

/// No-op backend for testing.
#[derive(Debug, Default)]
pub struct BlackholeTelemetry;

impl DistributedTelemetry for BlackholeTelemetry {
    type SpanId = u64;
    type TraceId = u64;

    fn report_span(&self, _span: CompletedSpan<u64, u64>) {}
    fn report_event(&self, _event: TracedEvent<u64, u64>) {}
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum EmbedAssetsError {
    #[error("trunk build failed: {0}")]
    BuildFailed(String),
    #[error("failed to walk output directory: {0}")]
    WalkError(#[from] std::io::Error),
    #[error("failed to generate asset map: {0}")]
    CodegenError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum TraceCtxError {
    #[error("no trace context registered for current span")]
    NoTraceContext,
    #[error("no active span")]
    NoActiveSpan,
}
```

## Key Rust-Specific Changes

### 1. cargo-web to trunk

**Source Pattern:** `cargo-web deploy` invoked programmatically via `CargoWebOpts::Deploy`

**Rust Translation:** Use `trunk build --release` via `std::process::Command`

**Rationale:** trunk is the actively maintained WASM build tool, supports wasm-bindgen, and does not require programmatic API access.

```rust
// embed-assets-build/src/lib.rs
pub fn compile_wasm(wasm_dir: &Path, out_dir: &Path) -> Result<(), EmbedAssetsError> {
    let status = std::process::Command::new("trunk")
        .args(["build", "--release", "--dist", out_dir.to_str().unwrap()])
        .current_dir(wasm_dir)
        .status()
        .map_err(|e| EmbedAssetsError::BuildFailed(e.to_string()))?;

    if !status.success() {
        return Err(EmbedAssetsError::BuildFailed("trunk build failed".into()));
    }

    generate_asset_map(out_dir)
}
```

### 2. hyper Response to axum Response

**Source Pattern:** `hyper::Response<Body>` with manual header insertion via `headers` crate

**Rust Translation:** `axum::response::Response` with builder pattern

**Rationale:** axum is the standard Rust web framework, and its Response type is compatible with tower and hyper.

### 3. tracing 0.1 + custom layer to tracing-opentelemetry

**Source Pattern:** Custom `TelemetryLayer` managing span lifecycle manually

**Rust Translation:** Build on `tracing-opentelemetry` for standard OTel export, keep the generic layer for custom backends

**Rationale:** OpenTelemetry has become the standard for distributed tracing, and `tracing-opentelemetry` handles the span lifecycle correctly.

### 4. failure to thiserror

**Source Pattern:** cargo-web uses `failure` crate for error handling

**Rust Translation:** `thiserror` for derive-based error types

**Rationale:** `failure` is unmaintained; `thiserror` is the standard for library error types.

## Ownership and Borrowing Strategy

Most crates in this ecosystem have straightforward ownership:

- **embed-assets:** Static data (`&'static [u8]`, `&'static Map`) - no ownership complexity, all references are `'static`
- **picoalloc:** Raw pointer management with explicit `unsafe` blocks - ownership is manual, safety guaranteed by allocator invariants
- **speedy:** Borrows data for reading (`Readable<'a, C>` with lifetime parameter), owns data for writing
- **recursion:** Uses `Vec<Option<Out>>` as an arena for intermediate values, avoiding self-referential structures

## Concurrency Model

**Approach:** Mostly single-threaded or async

- **embed-assets:** `StaticAssets` is `Send + Sync` by nature (all `&'static` references)
- **picoalloc:** Global allocator uses `static mut` with platform-specific synchronization (no Mutex - allocators cannot allocate)
- **speedy:** No concurrency concerns (serialize/deserialize are synchronous operations)
- **tracing-distributed:** Requires `Send + Sync` telemetry backends; optionally uses `parking_lot` for low-latency locking
- **recursion:** Single-threaded stack machine; experimental async feature for async recursive operations

## Memory Considerations

- **picoalloc** is the most memory-sensitive crate: it manages raw memory with a two-level bitmap, chunk headers stored inline in the managed memory, and 32-byte alignment granularity
- **speedy** uses zero-copy reading from buffers (borrowing data in place) and a circular buffer for stream reading
- **recursion** uses a heap-allocated stack (`Vec<State>`) and value array (`Vec<Option<Out>>`) to avoid call-stack overflow
- **embed-assets** embeds all WASM artifacts as static byte arrays in the binary, increasing binary size by the total size of frontend assets

## Edge Cases and Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Allocator double-free | picoalloc: UB if violated, documented as unsafe contract |
| Allocator OOM | picoalloc: Returns `None` (no panics) |
| Speedy EOF during read | `default_on_eof` attribute or `IsEof` error |
| Speedy wrong endianness | Type-level enforcement via `BigEndian`/`LittleEndian` contexts |
| Recursion stack overflow | recursion: Heap-allocated stack, bounded by available memory |
| Missing assets | embed-assets: Returns `Option<Response>`, caller handles 404 |
| Trace context missing | `TraceCtxError::NoTraceContext` |

## Code Examples

### Example: Static Asset Server with axum

```rust
use axum::{Router, extract::Path, response::IntoResponse, http::StatusCode};
use embed_assets::include_assets;

include_assets!();

async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    match ASSETS.get(&format!("/{}", path)) {
        Some(response) => response.into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", axum::routing::get(|| async {
            ASSETS.get("/").unwrap()
        }))
        .route("/*path", axum::routing::get(serve_asset));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### Example: Recursion Scheme for AST Evaluation

```rust
use recursion::{MappableFrame, PartiallyApplied, Collapsible, CollapsibleExt};

enum Expr {
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Lit(i64),
}

enum ExprFrame<A> {
    Add(A, A),
    Mul(A, A),
    Lit(i64),
}

impl MappableFrame for ExprFrame<PartiallyApplied> {
    type Frame<X> = ExprFrame<X>;
    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        match input {
            ExprFrame::Add(a, b) => ExprFrame::Add(f(a), f(b)),
            ExprFrame::Mul(a, b) => ExprFrame::Mul(f(a), f(b)),
            ExprFrame::Lit(n) => ExprFrame::Lit(n),
        }
    }
}

impl<'a> Collapsible for &'a Expr {
    type FrameToken = ExprFrame<PartiallyApplied>;
    fn into_frame(self) -> ExprFrame<Self> {
        match self {
            Expr::Add(a, b) => ExprFrame::Add(a, b),
            Expr::Mul(a, b) => ExprFrame::Mul(a, b),
            Expr::Lit(n) => ExprFrame::Lit(*n),
        }
    }
}

fn eval(expr: &Expr) -> i64 {
    expr.collapse_frames(|frame| match frame {
        ExprFrame::Add(a, b) => a + b,
        ExprFrame::Mul(a, b) => a * b,
        ExprFrame::Lit(n) => n,
    })
}
```

### Example: Speedy Binary Serialization

```rust
use speedy::{Readable, Writable, Endianness};

#[derive(Debug, PartialEq, Readable, Writable)]
struct Header {
    magic: u32,
    version: u16,
    #[speedy(length_type = u32)]
    entries: Vec<Entry>,
}

#[derive(Debug, PartialEq, Readable, Writable)]
struct Entry {
    id: u64,
    #[speedy(varint)]
    size: u64,
    #[speedy(length_type = u16)]
    name: String,
}

fn round_trip() {
    let header = Header {
        magic: 0xDEADBEEF,
        version: 1,
        entries: vec![
            Entry { id: 1, size: 1024, name: "first".into() },
            Entry { id: 2, size: 2048, name: "second".into() },
        ],
    };

    let bytes = header.write_to_vec().unwrap();
    let decoded = Header::read_from_buffer(&bytes).unwrap();
    assert_eq!(header, decoded);
}
```

## Performance Considerations

- **picoalloc:** O(1) alloc/free via bit-scanning instructions, ~2.5KB code size, suitable for hot allocation paths
- **speedy:** Zero-copy buffer reading avoids deserialization overhead; the `varint` encoding saves space for small numbers
- **recursion:** Heap stack avoids stack overflow but has allocation overhead per frame; benchmarks show competitive performance vs. natural recursion for deep trees
- **embed-assets:** PHF lookup is O(1); binary size increases proportionally to embedded asset size

## Testing Strategy

- **picoalloc:** Fuzz testing (`cargo-fuzz`), paranoid assertions for debug builds, unit tests for basic alloc/free patterns
- **speedy:** Property-based testing (`quickcheck`), round-trip tests, stream reading with randomized chunk sizes, static compile-time tests
- **recursion:** Expression evaluation tests, benchmarks for expr and list structures
- **embed-assets:** Integration test with a minimal WASM project, verify PHF map generation
- **tracing-distributed:** Use `BlackholeTelemetry` for unit testing instrumented code

## Open Considerations

- The `embed-assets` pattern could be further improved by supporting Brotli/gzip pre-compression of assets at build time
- For `tracing-distributed`, consider whether the generic layer still provides value over direct `tracing-opentelemetry` usage
- The `recursion` crate's experimental async support could benefit from structured concurrency patterns
- picoalloc's strict provenance support positions it well for future Rust memory model changes
