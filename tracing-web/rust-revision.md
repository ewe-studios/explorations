# Rust Replication Plan: Building a tracing-web Clone

This document outlines **how to build a similar library in Rust**, covering best practices, architecture decisions, and production considerations.

## Project Goal

Build a `tracing` subscriber layer that outputs to browser consoles and Performance API when compiled to WebAssembly.

## Phase 1: Project Setup

### Cargo.toml Configuration

```toml
[package]
name = "tracing-web"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
keywords = ["web", "wasm", "tracing", "log"]
categories = ["development-tools::debugging", "wasm", "web-programming"]
description = "A tracing compatible subscriber layer for web platforms."

[dependencies]
# Core tracing types (minimal dependencies)
tracing-core = { version = "0.1", default-features = false }

# Subscriber infrastructure
tracing-subscriber = { version = "0.3", default-features = false, features = ["fmt"] }

# WASM bindings
wasm-bindgen = { version = "0.2", default-features = false }
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console", "Performance"], default-features = false }

[dev-dependencies]
tracing = "0.1"
wasm-bindgen-test = "0.3"

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

### Directory Structure

```
tracing-web/
├── Cargo.toml
├── README.md
├── CHANGELOG.md
├── LICENSE-APACHE
├── LICENSE-MIT
├── src/
│   ├── lib.rs
│   ├── console_writer.rs
│   └── performance_layer.rs
└── examples/
    └── trace-web-app/
        ├── Cargo.toml
        ├── index.html
        └── src/main.rs
```

## Phase 2: Core Implementation

### Step 1: FFI Bindings (performance_layer.rs)

```rust
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use js_sys::{Object, Reflect, JsString};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = _fakeGlobal)]
    type Global;

    #[wasm_bindgen()]
    type Performance;

    #[wasm_bindgen(static_method_of = Global, js_class = "globalThis", getter)]
    fn performance() -> Performance;

    #[wasm_bindgen(method, catch, js_name = "mark")]
    fn mark(this: &Performance, name: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = "mark")]
    fn mark_detailed(
        this: &Performance,
        name: &str,
        details: &JsValue,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(method, catch, js_name = "measure")]
    fn measure(
        this: &Performance,
        name: &str,
        start: &str,
        end: &str,
    ) -> Result<(), JsValue>;
}

// Thread-local cached handle
thread_local! {
    static PERF: Performance = Global::performance();
}
```

**Best Practice:** Cache FFI handles in `thread_local!` to avoid repeated lookups.

### Step 2: Console Writer (console_writer.rs)

```rust
use std::io::{self, Write};
use tracing_core::Level;
use tracing_subscriber::fmt::MakeWriter;
use wasm_bindgen::JsValue;
use web_sys::console;

pub struct ConsoleWriter {
    buffer: Vec<u8>,
    level: Level,
    log_fn: fn(&str),
}

impl Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(()) // Flush on drop
    }
}

impl Drop for ConsoleWriter {
    fn drop(&mut self) {
        let message = String::from_utf8_lossy(&self.buffer);
        (self.log_fn)(message.as_ref());
    }
}

pub struct MakeConsoleWriter;

impl<'a> MakeWriter<'a> for MakeConsoleWriter {
    type Writer = ConsoleWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ConsoleWriter {
            buffer: Vec::new(),
            level: Level::TRACE,
            log_fn: |msg| console::log_1(&JsValue::from(msg)),
        }
    }

    fn make_writer_for(&'a self, meta: &tracing_core::Metadata<'_>) -> Self::Writer {
        let log_fn = match *meta.level() {
            Level::TRACE | Level::DEBUG => |msg| console::debug_1(&JsValue::from(msg)),
            Level::INFO => |msg| console::info_1(&JsValue::from(msg)),
            Level::WARN => |msg| console::warn_1(&JsValue::from(msg)),
            Level::ERROR => |msg| console::error_1(&JsValue::from(msg)),
        };

        ConsoleWriter {
            buffer: Vec::new(),
            level: *meta.level(),
            log_fn,
        }
    }
}
```

### Step 3: Performance Layer (performance_layer.rs)

```rust
use std::marker::PhantomData;
use tracing_core::{span, Subscriber};
use tracing_subscriber::{layer::{Context, Layer}, registry::LookupSpan};

pub struct PerformanceLayer<S> {
    _marker: PhantomData<S>,
}

impl<S> Layer<S> for PerformanceLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let name = format!("{} [{}]: enter", span.name(), id.into_u64());
        let _ = PERF.with(|p| p.mark(&name));
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let name = format!("{} [{}]: enter", span.name(), id.into_u64());
        let _ = PERF.with(|p| p.mark(&name));
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("span not found");
        let span_id = id.into_u64();
        let enter_name = format!("{} [{}]: enter", span.name(), span_id);
        let exit_name = format!("{} [{}]: exit", span.name(), span_id);
        let measure_name = format!("{} [{}]: measure", span.name(), span_id);

        let _ = PERF.with(|p| {
            p.mark(&exit_name)?;
            p.measure(&measure_name, &enter_name, &exit_name)
        });
    }
}

pub fn performance_layer<S>() -> PerformanceLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    PerformanceLayer {
        _marker: PhantomData,
    }
}
```

### Step 4: Library Entry Point (lib.rs)

```rust
//! A tracing compatible subscriber layer for web platforms.
//!
//! # Example
//!
//! ```rust,no_run
//! use tracing_web::{MakeConsoleWriter, performance_layer};
//! use tracing_subscriber::prelude::*;
//!
//! let console_layer = tracing_subscriber::fmt::layer()
//!     .with_writer(MakeConsoleWriter);
//!
//! let perf_layer = performance_layer();
//!
//! tracing_subscriber::registry()
//!     .with(console_layer)
//!     .with(perf_layer)
//!     .init();
//! ```

mod console_writer;
mod performance_layer;

pub use console_writer::{ConsoleWriter, MakeConsoleWriter};
pub use performance_layer::{performance_layer, PerformanceLayer};
```

## Phase 3: Example Application

### examples/trace-web-app/Cargo.toml

```toml
[package]
name = "trace-web-app"
version = "0.1.0"
edition = "2021"

[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-web = { path = "../.." }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"

[dependencies.web-sys]
version = "0.3"
features = ["Window", "Document", "Element"]
```

### examples/trace-web-app/src/main.rs

```rust
use tracing_subscriber::prelude::*;
use tracing_web::{MakeConsoleWriter, performance_layer};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn main() {
    // Initialize tracing
    let console_layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_ansi(false)
        .with_writer(MakeConsoleWriter);

    let perf_layer = performance_layer();

    tracing_subscriber::registry()
        .with(console_layer)
        .with(perf_layer)
        .init();

    tracing::info!("Application started");

    // Create a span
    let span = tracing::info_span!("my_operation", id = 42);
    let _guard = span.enter();

    tracing::debug!("Inside span");
    tracing::warn!("This is a warning");

    // Nested span
    let inner = tracing::debug_span!("inner_operation");
    inner.in_scope(|| {
        tracing::trace!("Deep inside");
    });

    tracing::error!("Something went wrong!");
}
```

### examples/trace-web-app/index.html

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Tracing Web Example</title>
</head>
<body>
    <p>Open browser DevTools to see tracing output</p>
    <script type="module">
        import init from "./pkg/trace_web_app.js";
        init();
    </script>
</body>
</html>
```

## Phase 4: Build and Test

### Building for WASM

```bash
# Add WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack

# Build the example
cd examples/trace-web-app
wasm-pack build --target web --dev

# Or build library
cd ../..
wasm-pack build --target web
```

### Running the Example

```bash
# Simple HTTP server
cd examples/trace-web-app
python -m http.server 8080

# Open http://localhost:8080 in browser
# Open DevTools Console to see output
```

## Best Practices

### 1. Minimize Dependencies

```toml
# Good: Only what's needed
tracing-core = { version = "0.1", default-features = false }

# Avoid: Pulling in std when not needed
tracing = "0.1"  # This includes more features
```

### 2. Use Type-Level Dispatch

```rust
// Instead of Box<dyn Trait>
trait LogImpl {
    fn log(msg: &str);
}

struct InfoLevel;
impl LogImpl for InfoLevel {
    fn log(msg: &str) { console::info_1(&msg.into()); }
}

// Returns concrete fn pointer, not trait object
fn get_logger() -> fn(&str) {
    InfoLevel::log
}
```

### 3. Handle Errors Gracefully

```rust
// Browser APIs are best-effort
let _ = PERF.with(|p| p.mark(&name));

// Don't crash on FFI failures
```

### 4. Use PhantomData Correctly

```rust
// Consumer of S should be contravariant
pub struct Layer<S> {
    _marker: PhantomData<fn(S)>,  // Contravariant
}

// Producer of S should be covariant
pub struct Container<S> {
    _marker: PhantomData<S>,  // Covariant
}
```

### 5. Intern Strings for FFI

```rust
// Good: Interned
let name = JsValue::from(wasm_bindgen::intern("static_string"));

// Bad: New allocation each time
let name = JsValue::from("static_string".to_string());
```

## Production Considerations

### 1. Feature Flags

```toml
[features]
default = []

# Enable pretty console output with CSS
pretty-console = []

# Enable performance marks
performance = []

# Reduce code size for production
optimize-size = []
```

### 2. Conditional Compilation

```rust
#[cfg(feature = "performance")]
fn emit_performance_mark(name: &str) {
    // Implementation
}

#[cfg(not(feature = "performance"))]
fn emit_performance_mark(_name: &str) {
    // No-op
}
```

### 3. Level Filtering at Compile Time

```rust
// In Cargo.toml
[dependencies.tracing]
version = "0.1"
features = ["max_level_info", "release_max_level_warn"]
```

This removes trace/debug calls in release builds.

### 4. Non-Blocking Considerations

For high-throughput scenarios:

```rust
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;

pub struct AsyncConsoleWriter {
    tx: Sender<String>,
}

impl AsyncConsoleWriter {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for msg in rx {
                console::log_1(&JsValue::from(&msg));
            }
        });

        Self { tx }
    }
}

impl Write for AsyncConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let msg = String::from_utf8_lossy(buf).to_string();
        let _ = self.tx.send(msg);
        Ok(buf.len())
    }
}
```

### 5. Code Size Optimization

```toml
# In .cargo/config.toml
[target.wasm32-unknown-unknown]
rustflags = [
    "-C", "opt-level=s",
    "-C", "lto=fat",
]

# In Cargo.toml
[profile.release]
lto = true
codegen-units = 1
```

### 6. Browser Compatibility

```rust
// Feature detection
fn performance_available() -> bool {
    !Global::performance().is_undefined()
}

// Graceful degradation
if performance_available() {
    // Use performance API
} else {
    // Fall back to console only
}
```

## Common Pitfalls

### 1. Forgetting `default-features = false`

```toml
# Bad: Pulls in unnecessary dependencies
tracing-core = "0.1"

# Good: Minimal
tracing-core = { version = "0.1", default-features = false }
```

### 2. Not Handling UTF-8 Correctly

```rust
// ConsoleWriter must handle partial UTF-8 sequences
impl Drop for ConsoleWriter {
    fn drop(&mut self) {
        // Use from_utf8_lossy, not from_utf8
        let message = String::from_utf8_lossy(&self.buffer);
        (self.log_fn)(message.as_ref());
    }
}
```

### 3. Blocking on FFI

```rust
// Bad: Synchronous FFI in hot path
fn on_event(&self, event: &Event) {
    let msg = format_event(event);
    console::log_1(&msg.into());  // Blocks
}

// Better: Buffer and batch
fn on_event(&self, event: &Event) {
    self.buffer.push(format_event(event));
    if self.buffer.len() >= BATCH_SIZE {
        self.flush();
    }
}
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_writer_write() {
        let mut writer = ConsoleWriter {
            buffer: Vec::new(),
            level: Level::INFO,
            log_fn: |_| {},
        };

        writer.write_all(b"Hello").unwrap();
        assert_eq!(writer.buffer, b"Hello");
    }
}
```

### WASM Tests

```rust
#[cfg(target_arch = "wasm32")]
mod wasm_tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_performance_layer() {
        let layer = performance_layer();
        // Test implementation
    }
}
```

## Comparison with Existing Solutions

| Feature | tracing-web | tracing-console | tracing-opentelemetry |
|---------|-------------|-----------------|----------------------|
| Browser Console | Yes | No | No |
| Performance API | Yes | No | No |
| Distributed Tracing | No | No | Yes |
| Server-side | No | Yes | Yes |
| WASM Optimized | Yes | No | Partial |

## Future Enhancements

1. **OpenTelemetry Export** - Add OTLP export capability
2. **Structured JSON Output** - Machine-readable console output
3. **Sampling** - Configurable trace sampling
4. **Context Propagation** - W3C trace context support
5. **Metrics Export** - Integration with web-vitals

## References

- [tokio-rs/tracing](https://github.com/tokio-rs/tracing)
- [wasm-bindgen Guide](https://rustwasm.github.io/docs/wasm-bindgen/)
- [Web Performance API](https://developer.mozilla.org/en-US/docs/Web/API/Performance)
- [tracing-subscriber Documentation](https://docs.rs/tracing-subscriber)
