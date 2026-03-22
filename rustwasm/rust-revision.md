---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/rustwasm/
explored_at: 2026-03-22
revised_at: 2026-03-22
workspace: rust-wasm-workspace
---

# Rust Revision: Rust WebAssembly Sub-Projects

## Overview

This document consolidates the Rust WebAssembly sub-project explorations into implementation guidance for building WebAssembly applications with Rust. The revision covers the complete toolchain from compilation to optimization.

## Sub-Projects Covered

| Sub-Project | Purpose | Implementation Priority |
|-------------|---------|------------------------|
| wasm-bindgen | Rust/JS interop | Critical |
| wasm-pack | Build tooling | Critical |
| gloo | Web APIs | High |
| twiggy | Size analysis | Medium |
| walrus | WASM manipulation | Medium |
| console_error_panic_hook | Debugging | High |
| wee_alloc | Tiny allocator | Medium |
| wasm-snip | Dead code removal | Medium |
| create-wasm-app | Project template | High |

## Workspace Structure

```
rust-wasm-workspace/
├── core-crates/
│   ├── wasm-bindgen-example/   # JS interop examples
│   ├── gloo-app/               # Web API usage
│   └── panic-hook-demo/        # Debug support
├── build-tooling/
│   ├── wasm-pack-lib/          # Library template
│   └── create-wasm-app/        # App template
├── optimization/
│   ├── size-analysis/          # Twiggy profiling
│   ├── code-snipping/          # wasm-snip usage
│   └── allocator-benchmarks/   # wee_alloc testing
└── advanced/
    ├── walrus-transforms/      # WASM manipulation
    └── custom-tooling/         # Build customization
```

## Core Implementation Patterns

### wasm-bindgen Setup

```rust
// core-crates/wasm-bindgen-example/src/lib.rs
use wasm_bindgen::prelude::*;
use web_sys::{console, Document, Element, HtmlElement, Window};

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Set up panic hook for debugging
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    // Get window and document
    let window = web_sys::window().expect("No global `window` exists");
    let document = window.document().expect("Should have a document on window");

    // Create element
    let element = document.create_element("div")?;
    element.set_text_content(Some("Hello from Rust + WASM!"));

    // Add to body
    let body = document.body().expect("Document should have a body");
    body.append_child(&element)?;

    Ok(())
}

/// Export function for JavaScript to call
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

/// Function that returns JsValue
#[wasm_bindgen]
pub fn create_object() -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"name".into(), &"Rust".into()).unwrap();
    js_sys::Reflect::set(&obj, &"version".into(), &"1.0".into()).unwrap();
    obj.into()
}

/// Async function example
#[wasm_bindgen]
pub async fn fetch_data(url: String) -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or("No window")?;

    let mut opts = web_sys::RequestInit::new();
    opts.method("GET");

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)?;

    let resp = window
        .fetch_with_request(&request)
        .await?
        .json()
        .await?;

    Ok(resp)
}

/// Closure example
#[wasm_bindgen]
pub fn add_event_listener(
    element: &web_sys::Element,
    event: &str,
    callback: js_sys::Function,
) -> Result<web_sys::EventTarget, JsValue> {
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        callback.call1(&JsValue::NULL, &event).unwrap();
    }) as Box<dyn FnMut(_)>);

    let target = element.dyn_ref::<web_sys::EventTarget>()
        .ok_or("Not an EventTarget")?;

    target.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())?;

    // Forget closure to prevent dropping
    closure.forget();

    Ok(target.clone())
}
```

### Cargo.toml Configuration

```toml
# core-crates/wasm-bindgen-example/Cargo.toml
[package]
name = "wasm-bindgen-example"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
console_error_panic_hook = { version = "0.1", optional = true }
js-sys = "0.3"

[dependencies.web-sys]
version = "0.3"
features = [
    "Window",
    "Document",
    "Element",
    "HtmlElement",
    "HtmlInputElement",
    "HtmlButtonElement",
    "console",
    "Event",
    "EventTarget",
    "Request",
    "RequestInit",
    "RequestMode",
    "Response",
    "Headers",
]

[features]
default = ["console_error_panic_hook"]

[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
```

### gloo Web API Pattern

```rust
// core-crates/gloo-app/src/lib.rs
use gloo::{
    console,
    events::EventListener,
    file::FileReader,
    net::http,
    render,
    storage::LocalStorage,
    timers::Interval,
    utils::document,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console::log!("gloo app starting!");

    // Storage example
    if let Ok(storage) = LocalStorage::get::<AppState>("app_state") {
        console::log!("Loaded state:", &storage.counter);
    }

    // HTTP example
    wasm_bindgen_futures::spawn_local(async {
        match http::get("/api/data").send().await {
            Ok(response) => {
                let data: ApiResponse = response.json().await.unwrap();
                console::log!("Fetched:", &data);
            }
            Err(e) => console::error!("Fetch failed:", &e),
        }
    });

    // Timer example
    let interval = Interval::new(1000, || {
        console::log!("Tick!");
    });

    // Event listener example
    let window = document().default_view().unwrap();
    let _resize_listener = EventListener::new(&window, "resize", |event| {
        console::log!("Window resized:", event);
    });

    // File reader example
    // (Requires user input)
}

#[derive(Serialize, Deserialize)]
struct AppState {
    counter: i32,
    theme: String,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    data: Vec<String>,
    status: String,
}
```

### Build Script with wasm-pack

```rust
// build-tooling/wasm-pack-lib/build.rs
use std::process::Command;

fn main() {
    // Run wasm-pack build on release
    if std::env::var("PROFILE").unwrap() == "release" {
        let status = Command::new("wasm-pack")
            .args(&["build", "--target", "web", "--release"])
            .status()
            .unwrap();

        if !status.success() {
            panic!("wasm-pack build failed");
        }
    }
}
```

## Optimization Strategies

### Size Analysis with Twiggy

```bash
# optimization/size-analysis/analyze.sh
#!/bin/bash

# Build with debug info
wasm-pack build --release

# Analyze size
twiggy top pkg/my_module_bg.wasm > size_report.txt

# Show call graph
twiggy dominators pkg/my_module_bg.wasm > dominators.txt

# Compare versions
twiggy diff old.wasm new.wasm > size_diff.txt
```

### Code Snipping

```rust
// optimization/code-snipping/snip-config.toml
# wasm-snip configuration

[snip]
# Functions to remove
functions = [
    "my_module::debug_log",
    "my_module::trace_execution",
    "console_error_panic_hook::set_once",
]

# Regex patterns
patterns = [
    ".*::test_.*",
    ".*::bench_.*",
]
```

```bash
# Run snipping
wasm-snip pkg/my_module_bg.wasm \
    -o pkg/my_module_snipped.wasm \
    --pattern "debug_.*"

# Then optimize
wasm-opt -Oz pkg/my_module_snipped.wasm \
    -o pkg/my_module_bg.wasm
```

### Allocator Selection

```rust
// optimization/allocator-benchmarks/src/lib.rs
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[cfg(feature = "std")]
use std::alloc::System;

#[cfg(feature = "std")]
#[global_allocator]
static ALLOC: System = System;

// Benchmark different allocators
#[wasm_bindgen]
pub fn benchmark_allocations(iterations: usize) -> u128 {
    let start = web_sys::window().unwrap().performance().unwrap().now();

    for _ in 0..iterations {
        let _vec: Vec<u8> = vec![0; 1024];
    }

    let end = web_sys::window().unwrap().performance().unwrap().now();
    (end - start) as u128
}
```

## Project Templates

### Library Template

```toml
# build-tooling/wasm-pack-lib/Cargo.toml
[package]
name = "wasm-pack-lib"
version = "0.1.0"
edition = "2021"
description = "A WASM library template"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
console_error_panic_hook = { version = "0.1", optional = true }

[dev-dependencies]
wasm-bindgen-test = "0.3"

[features]
default = ["console_error_panic_hook"]

[profile.release]
opt-level = "s"
lto = true
```

### App Template

```javascript
// build-tooling/create-wasm-app/index.js
import * as wasm from "wasm-pack-lib";

// Initialize
wasm.init();

// Use exported functions
const greeting = wasm.greet("World");
console.log(greeting);

// Handle errors
try {
    wasm.risky_operation();
} catch (e) {
    console.error("WASM error:", e);
}
```

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>WASM App</title>
</head>
<body>
    <script type="module" src="index.js"></script>
</body>
</html>
```

```javascript
// webpack.config.js
const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require("path");

module.exports = {
    entry: "./index.js",
    output: {
        path: path.resolve(__dirname, "dist"),
        filename: "index.js",
    },
    mode: "development",
    experiments: {
        asyncWebAssembly: true,
    },
    plugins: [
        new CopyWebpackPlugin({
            patterns: ["index.html"],
        }),
    ],
    devServer: {
        port: 8080,
    },
};
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_greet() {
        assert_eq!(greet("Rust"), "Hello, Rust!");
    }

    #[wasm_bindgen_test(async)]
    async fn test_async_fetch() -> Result<(), JsValue> {
        let result = fetch_data("https://api.example.com/data".to_string()).await?;
        assert!(!result.is_undefined());
        Ok(())
    }
}
```

```bash
# Run tests
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
```

## Related Documents

- [wasm-bindgen](./wasm-bindgen-exploration.md) - Interop details
- [wasm-pack](./wasm-pack-exploration.md) - Build tooling
- [Twiggy](./twiggy-exploration.md) - Size analysis
- [walrus](./walrus-exploration.md) - WASM manipulation

## Sources

- wasm-bindgen Guide: https://rustwasm.github.io/docs/wasm-bindgen/
- wasm-pack: https://rustwasm.github.io/wasm-pack/
- The Rust and WebAssembly Working Group: https://github.com/rustwasm
