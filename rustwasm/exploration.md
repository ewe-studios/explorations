---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm
repository: https://github.com/rustwasm
explored_at: 2026-03-22
language: Rust, JavaScript, TypeScript, WebAssembly
---

# Project Exploration: rustwasm (Rust and WebAssembly Working Group)

## Overview

The **Rust and WebAssembly Working Group** (rustwasm) is a community-driven initiative that builds tools, documentation, and libraries for using Rust with WebAssembly. The working group produces a cohesive ecosystem of tools that make Rust+Wasm development productive, reliable, and performant.

### Key Value Proposition

- **One-stop-shop tooling** - wasm-pack handles building, testing, and publishing
- **Ergonomic bindings** - wasm-bindgen eliminates manual JS glue code
- **Modular toolkit** - Gloo provides reusable browser API wrappers
- **Optimization tools** - Twiggy for code size profiling, wasm-snip for dead code elimination
- **Production-ready** - Battle-tested by the community since 2018

### Example Usage

```rust
// src/lib.rs - Using wasm-bindgen to export Rust to JavaScript
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) {
    web_sys::console::log_1(&format!("Hello, {}!", name).into());
}

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

```javascript
// JavaScript usage
import { greet, add } from "./pkg";

greet("World!");
console.log(add(2, 3)); // 5
```

```bash
# Build with wasm-pack
wasm-pack build --target web
wasm-pack publish
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/
├── wasm-bindgen/                    # Core bindings tool
│   ├── crates/
│   │   ├── wasm-bindgen/            # Main crate
│   │   ├── js-sys/                  # JS global bindings
│   │   ├── web-sys/                 # Web API bindings
│   │   ├── wasm-bindgen-futures/    # Async/Promise support
│   │   └── wasm-bindgen-macro/      # Proc macros
│   └── cli/                         # Command-line tool
│
├── wasm-pack/                       # Build and publish tool
│   ├── src/
│   │   ├── build/                   # Build command
│   │   ├── test/                    # Test runner
│   │   ├── pack/                    # Packaging
│   │   └── publish/                 # Publishing to npm
│   └── Cargo.toml
│
├── gloo/                            # Modular toolkit
│   ├── crates/
│   │   ├── console/                 # console.* APIs
│   │   ├── timers/                  # setTimeout, setInterval
│   │   ├── events/                  # Event listeners
│   │   ├── file/                    # File/Blob APIs
│   │   ├── history/                 # History API
│   │   ├── dialogs/                 # alert, confirm, prompt
│   │   └── worker/                  # Web Workers
│   └── website/                     # Documentation
│
├── twiggy/                          # Code size profiler
│   ├── analyze/                     # Analysis logic
│   ├── cli/                         # CLI interface
│   └── guide/                       # Documentation
│
├── console_error_panic_hook/        # Panic hook for debugging
│   ├── src/
│   │   └── lib.rs                   # Panic hook implementation
│   └── tests/
│
├── wasm-snip/                       # Dead code elimination
│   ├── src/
│   │   └── lib.rs                   # Function replacement
│   └── README.md
│
├── book/                            # Rust and WebAssembly book
│   ├── src/
│   │   ├── tutorial/                # Game of Life tutorial
│   │   ├── reference/               # Reference docs
│   │   └── concepts/                # Background concepts
│   └── book.toml
│
├── create-wasm-app/                 # Project scaffolding
│   ├── .bin/
│   │   └── create-wasm-app.js       # Scaffold script
│   └── templates/
│
├── binary-install/                  # Binary downloader
│   ├── src/
│   │   └── lib.rs                   # Download and cache
│   └── tests/
│
├── wee_alloc/                       # Tiny allocator for Wasm
│   ├── src/
│   │   └── lib.rs                   # Allocator implementation
│   └── README.md
│
├── walrus/                          # Wasm utility library
│   ├── src/
│   │   ├── module.rs                # Module representation
│   │   ├── ir/                      # Intermediate representation
│   │   └── passes/                  # Optimization passes
│   └── README.md
│
└── wasm-tracing-allocator/          # Memory tracing
    └── src/
        └── lib.rs                   # Tracing allocator
```

## Core Projects

### 1. wasm-bindgen

**The glue between Rust and JavaScript**

wasm-bindgen is the foundational crate that enables high-level interactions between Rust-compiled WebAssembly modules and JavaScript.

#### How It Works

```rust
// 1. Import JavaScript functions into Rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_name = "fetch")]
    fn fetch(input: &str) -> JsPromise;
}

// 2. Export Rust functions to JavaScript
#[wasm_bindgen]
pub fn process_data(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    // Process and return
    Ok(data.to_vec())
}

// 3. Export structs with methods
#[wasm_bindgen]
pub struct Processor {
    data: Vec<u8>,
}

#[wasm_bindgen]
impl Processor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Processor {
        Processor { data: Vec::new() }
    }

    #[wasm_bindgen]
    pub fn add(&mut self, byte: u8) {
        self.data.push(byte);
    }

    #[wasm_bindgen]
    pub fn get(&self, index: usize) -> Option<u8> {
        self.data.get(index).copied()
    }
}
```

#### Generated JavaScript

```javascript
// wasm-bindgen automatically generates:
import { Processor, process_data } from "./module.js";

const proc = new Processor();
proc.add(42);
console.log(proc.get(0)); // 42
```

#### Type Conversions

| Rust Type | JavaScript Type | Notes |
|-----------|-----------------|-------|
| `i8`, `u8`, `i16`, `u16` | Number | Safe integer range |
| `i32`, `u32`, `i64`, `u64` | BigInt or Number | Depends on config |
| `f32`, `f64` | Number | IEEE 754 |
| `bool` | Boolean | Direct mapping |
| `String`, `&str` | String | UTF-8 conversion |
| `Vec<T>` | Array | Element conversion |
| `Option<T>` | T or null/undefined | Nullable |
| `Result<T, E>` | T or throw | Error handling |
| `Closure<dyn Fn()>` | Function | Callback |
| `JsValue` | any | Escape hatch |

### 2. wasm-pack

**The build tool for Rust + Wasm**

wasm-pack is a one-stop shop for building, testing, and publishing Rust-generated WebAssembly.

#### Commands

```bash
# Create new project from template
wasm-pack new my-project

# Build for web (default target)
wasm-pack build

# Build with release optimizations
wasm-pack build --release

# Build for Node.js
wasm-pack build --target nodejs

# Build for bundlers (webpack, rollup, etc.)
wasm-pack build --target bundler

# Run tests in headless browser
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome

# Package for publishing
wasm-pack pack

# Publish to npm
wasm-pack publish
```

#### Build Output

```
pkg/
├── my_module.js          # JavaScript bindings
├── my_module_bg.js       # Internal bindings
├── my_module_bg.wasm     # Compiled Wasm
├── my_module.d.ts        # TypeScript types
├── package.json          # npm package manifest
└── README.md             # Generated readme
```

#### package.json Generated

```json
{
  "name": "my-wasm-module",
  "version": "0.1.0",
  "files": [
    "my_module_bg.wasm",
    "my_module.js",
    "my_module_bg.js",
    "my_module.d.ts"
  ],
  "module": "my_module.js",
  "types": "my_module.d.ts",
  "sideEffects": ["./my_module.js"],
  "keywords": ["wasm"]
}
```

### 3. Gloo

**A modular toolkit for Web APIs**

Gloo is a collection of ergonomic Rust wrappers around browser APIs, designed to be modular and composable.

#### Console API

```rust
use gloo_console::{log, info, warn, error};

log!("Simple log");
info!("Info message");
warn!("Warning!");
error!("Error occurred");

// Log multiple values
let obj = js_sys::Object::new();
log!("Object:", obj);

// Table display
gloo_console::table(&data);

// Timer
let timer = gloo_console::timer::Timer::new("loading");
// ... do work ...
timer.end(); // "loading: 234.56ms"
```

#### Events API

```rust
use gloo::events::EventListener;
use wasm_bindgen::JsCast;

let button: web_sys::HtmlButtonElement = document
    .query_selector("#my-button")?
    .unwrap()
    .dyn_into()?;

// Add event listener
let listener = EventListener::new(&button, "click", |event| {
    console_log!("Button clicked!");
});

// Listener automatically removed when dropped
// To keep alive:
listener.forget();
```

#### Timers API

```rust
use gloo::timers::callback::{Timeout, Interval};

// One-shot timeout
let timeout = Timeout::new(1000, || {
    console_log!("One second passed!");
});
timeout.forget(); // Keep alive

// Repeating interval
let interval = Interval::new(100, |count| {
    console_log!("Tick {}", count);
});
interval.forget();
```

#### File API

```rust
use gloo_file::{Blob, FileReader, callbacks::FileReader as FileReaderCb};

// Read a file
let file_reader = FileReader::new().unwrap();
let handle = file_reader.read_as_text(
    &file,
    gloo_file::buffer::FileType::Plain,
    move |result| {
        match result {
            Ok(content) => console_log!("Content: {}", content),
            Err(e) => console_error!("Error: {:?}", e),
        }
    }
);

// Cancel if needed
file_reader.abort(&handle);
```

#### History API

```rust
use gloo_history::{History, MemoryHistory, BrowserHistory};

// Browser history
let history = BrowserHistory::new();
history.push("/page1");
history.back();
history.forward();

// Listen to changes
let _listener = history.listen(|location| {
    console_log!("Navigated to: {:?}", location);
});
```

### 4. Twiggy

**A code size profiler for WebAssembly**

Twiggy analyzes Wasm binaries to understand code size and optimize binaries.

#### Commands

```bash
# Install
cargo install twiggy

# Show top functions by size
twiggy top my_module.wasm

# Show dominator tree (who calls whom)
twiggy dominators my_module.wasm

# Show monomorphizations
twiggy monos my_module.wasm

# Diff two binaries
twiggy diff old.wasm new.wasm

# Export to CSV
twiggy top my_module.wasm > sizes.csv
```

#### Example Output

```
$ twiggy top my_module.wasm

 Shallow Bytes │ Shallow % │ Who?
───────────────┼───────────┼──────────────────────────
         10240 │     15.6% │ "my_heavy_function"
          8192 │     12.5% │ "serde::serialize"
          4096 │      6.2% │ "wasm_bindgen::describe"
          2048 │      3.1% │ "__wbindgen_malloc"
          1024 │      1.5% │ (panic formatting)
           512 │      0.7% │ (other)
         39321 │     59.6% │ ... and 124 others
         65536 │    100.0% │ Σ [130 Total Functions]
```

#### Dominator Analysis

```
$ twiggy dominators my_module.wasm

 Dominator Bytes │ Dominator % │ Who?
─────────────────┼─────────────┼──────────────────────────
           65536 │       100% │ "my_module.wasm"
           32768 │        50% │ ├─ "heavy_computation"
           16384 │        25% │ │  ├─ "inner_loop"
            8192 │        12% │ │  │  ├─ "helper_func"
            4096 │         6% │ │  │  │  ├─ "tiny_func"
           16384 │        25% │ ├─ "serde_serialize"
            8192 │        12% │ │  ├─ "serialize_struct"
           16384 │        25% │ └─ "wasm_bindgen"
```

### 5. console_error_panic_hook

**Debug panics in WebAssembly**

This crate forwards Rust panics to `console.error` for better debugging.

#### Usage

```rust
// Option 1: Set panic hook manually
use std::panic;

fn init() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    // ... rest of init
}

// Option 2: Set once (recommended)
fn my_function() {
    console_error_panic_hook::set_once();
    // ... rest of function
}

// Option 3: Use cfg feature flag
#[cfg(target_arch = "wasm32")]
console_error_panic_hook::set_once();
```

#### Without Panic Hook

```
RuntimeError: Unreachable executed
    at my_module.wasm:0x1234
```

#### With Panic Hook

```
Error: panicked at 'index out of bounds: the len is 10 but the index is 20', src/lib.rs:42:5

Stack:
Error
    at http://localhost:8080/my_module.js:456:19
    at my_module.wasm:0x1234
```

### 6. wasm-snip

**"Black hole" dead code elimination**

wasm-snip replaces specified functions with `unreachable` instructions, allowing the Wasm optimizer to remove dead code.

#### Usage

```bash
# Snip a specific function
wasm-snip my_module.wasm -f "my_unused_function" -o my_module.snipped.wasm

# Snip all functions matching pattern
wasm-snip my_module.wasm -f "debug_*" -o my_module.snipped.wasm

# Snip panic formatting (saves ~20KB)
wasm-snip my_module.wasm \
    -f "__rustc_hashbrown_fmt" \
    -f "core::fmt::Arguments::new_v1" \
    -o my_module.snipped.wasm
```

#### Before and After

```
# Before
$ twiggy top my_module.wasm | grep panic
  20480 │     31.2% │ "core::panicking::panic"
   8192 │     12.5% │ "core::fmt::Arguments::new_v1"

# After snipping
$ twiggy top my_module.snipped.wasm | grep panic
  (no matches - panic code eliminated!)
```

### 7. wee_alloc

**A tiny allocator for WebAssembly**

wee_alloc is a specialized memory allocator designed to minimize code size for Wasm modules.

#### Usage

```rust
// Cargo.toml
[dependencies]
wee_alloc = "0.4"

// lib.rs
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Reduces binary size by ~5-10KB compared to dlmalloc
```

#### Trade-offs

| Aspect | wee_alloc | dlmalloc (default) |
|--------|-----------|-------------------|
| Code size | ~2KB | ~10-15KB |
| Speed | Slower | Faster |
| Fragmentation | Higher | Lower |
| Features | Minimal | Full-featured |

### 8. walrus

**A library for Wasm transformation**

walrus is used to read, transform, and write Wasm modules programmatically.

#### Usage

```rust
use walrus::{Module, ModuleConfig};

// Load a Wasm module
let mut module = ModuleConfig::new()
    .parse(&wasm_bytes)?;

// Iterate over functions
for func in module.funcs.iter() {
    println!("Function: {:?}", func.name());
}

// Remove unused functions
module.remove_unused_functions();

// Write back
let output = module.emit_wasm();
```

## Tool Workflows

### Development Workflow

```bash
# 1. Create new project
wasm-pack new my-wasm-app
cd my-wasm-app

# 2. Develop with watch mode
wasm-pack build --dev --watch

# 3. Run tests
wasm-pack test --headless --firefox

# 4. Profile code size
twiggy top pkg/my_wasm_app_bg.wasm

# 5. Optimize
wasm-pack build --release

# 6. Check size again
twiggy top pkg/my_wasm_app_bg.wasm
```

### Publishing Workflow

```bash
# 1. Build for npm
wasm-pack build --target bundler --release

# 2. Run tests one more time
wasm-pack test --headless --chrome

# 3. Check package contents
wasm-pack pack

# 4. Publish to npm
wasm-pack publish
```

### CI/CD Integration

```yaml
# .github/workflows/wasm.yml
name: Wasm CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Test
        run: wasm-pack test --headless --firefox
      - name: Build
        run: wasm-pack build --release
      - name: Check size
        run: |
          cargo install twiggy
          twiggy top pkg/*.wasm | head -20
```

## Book and Documentation

The Rust and WebAssembly book provides comprehensive guides:

### Tutorial: Game of Life

A complete tutorial building Conway's Game of Life:

```rust
// Implementing the Game of Life universe
pub struct Universe {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
}

#[wasm_bindgen]
impl Universe {
    pub fn new() -> Universe {
        // Initialize universe
    }

    pub fn tick(&mut self) {
        // Compute next generation
    }

    pub fn cells(&self) -> *const Cell {
        self.cells.as_ptr()
    }
}
```

### Key Topics Covered

- **Background and Concepts**: What is WebAssembly, why Rust?
- **Setup**: Installing toolchain, creating projects
- **Tutorial**: Building Game of Life step-by-step
- **Reference**:
  - Code size optimization
  - Debugging techniques
  - Time profiling
  - Deploying to production
  - JavaScript FFI patterns
  - Project templates

## Performance Characteristics

### Binary Sizes

| Feature | Approximate Size | Notes |
|---------|------------------|-------|
| Empty wasm-bindgen module | ~2KB | Minimal overhead |
| With console_error_panic_hook | +5KB | Debug support |
| With serde | +30-50KB | Serialization |
| With web-sys DOM APIs | +20-40KB | Browser APIs |
| Default allocator (dlmalloc) | ~10KB | Can use wee_alloc instead |
| Panic formatting | ~15KB | Can be snipped |

### Runtime Performance

| Operation | Relative Speed | Notes |
|-----------|---------------|-------|
| Integer arithmetic | ~1x native | Near-native speed |
| Floating point | ~1x native | IEEE 754 |
| Memory access | ~1.1x | Bounds checking overhead |
| JS→Rust call | ~100ns | Boundary crossing cost |
| Rust→JS call | ~100ns | Boundary crossing cost |
| String conversion | O(n) | UTF-8 encoding |

## Ecosystem Crates

### Official Ecosystem

| Crate | Purpose |
|-------|---------|
| `wasm-bindgen` | Core bindings |
| `js-sys` | JavaScript global bindings |
| `web-sys` | Web browser API bindings |
| `wasm-bindgen-futures` | Async/Promise support |
| `console_error_panic_hook` | Panic debugging |
| `gloo-*` | Modular browser APIs |
| `wasm-pack` | Build tool |
| `twiggy` | Size profiler |

### Community Ecosystem

| Crate | Purpose |
|-------|---------|
| `wasm-logger` | Logging to browser console |
| `console_log` | Simpler console logging |
| `serde-wasm-bindgen` | Serde + wasm-bindgen integration |
| `js-sys` | JavaScript builtins |
| `wasm-bindgen-test` | Testing framework |
| `rollup-plugin-wasm-bindgen` | Rollup integration |
| `wasm-pack-plugin` | Webpack integration |

## Trade-offs

| Aspect | Decision | Trade-off |
|--------|----------|-----------|
| wasm-bindgen | Auto-generated glue | Slight runtime overhead |
| wasm-pack | Opinionated workflow | Less flexibility |
| Gloo | Modular design | More dependencies |
| web-sys | Comprehensive bindings | Large crate size |
| dlmalloc | Default allocator | Larger than wee_alloc |
| Panic formatting | Included by default | Can snip for size |

## Best Practices

### 1. Minimize Boundary Crossings

```rust
// Bad: Many crossings
for item in items {
    process_js_item(item)?;  // Each call crosses boundary
}

// Good: Batch processing
let results = process_all_items(items)?;  // Single crossing
```

### 2. Use `[wasm_bindgen(inline_js)]` for Small Helpers

```rust
#[wasm_bindgen(inline_js = r#"
export function now() { return performance.now(); }
"#)]
extern "C" {
    fn now() -> f64;
}
```

### 3. Optimize for Size Early

```toml
# Cargo.toml
[profile.release]
opt-level = "s"   # Optimize for size
lto = true        # Link-time optimization
codegen-units = 1 # Better optimizations
```

### 4. Use console_error_panic_hook in Development

```rust
#[cfg(debug_assertions)]
console_error_panic_hook::set_once();
```

### 5. Profile with Twiggy

```bash
# Always check what's in your binary
twiggy top pkg/*.wasm
twiggy dominators pkg/*.wasm
```

## Related Projects in Source Directory

- **binary-install** - Binary downloader used by wasm-pack
- **book** - The Rust and WebAssembly book
- **console_error_panic_hook** - Panic hook for debugging
- **create-wasm-app** - Project scaffolding
- **gloo** - Modular browser API toolkit
- **hello-wasm-bindgen** - Minimal example
- **twiggy** - Code size profiler
- **walrus** - Wasm transformation library
- **wasm-bindgen** - Core bindings
- **wasm-pack** - Build and publish tool
- **wasm-snip** - Dead code elimination
- **wasm-tracing-allocator** - Memory tracing
- **wee_alloc** - Tiny allocator
- **weedle** - WebIDL parser

## References

- **Main Documentation**: https://rustwasm.github.io/docs
- **wasm-bindgen Guide**: https://rustwasm.github.io/docs/wasm-bindgen/
- **wasm-pack Guide**: https://rustwasm.github.io/docs/wasm-pack/
- **Rust and WebAssembly Book**: https://rustwasm.github.io/docs/book/
- **Gloo Documentation**: https://docs.rs/gloo
- **Discord Community**: https://discord.gg/xMZ7CCY
- **GitHub Organization**: https://github.com/rustwasm
