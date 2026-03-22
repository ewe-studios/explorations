---
name: Console Error Panic Hook
description: Bridge Rust panics to console.error for better WASM debugging
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/console_error_panic_hook/
---

# Console Error Panic Hook - Debugging Bridge for WASM

## Overview

The Console Error Panic Hook is a **small but essential utility** that redirects Rust panics to JavaScript's `console.error()` when running in WebAssembly. This makes debugging WASM applications significantly easier by providing meaningful error messages in the browser's developer console instead of silent crashes or cryptic error codes.

Key features:
- **One-line setup** - Single function call to enable
- **Better error messages** - Full panic info in console
- **Source location** - File and line number included
- **Optional backtrace** - Can include stack traces
- **Zero overhead when disabled** - Compiles away in release
- **Production safe** - Can be feature-gated

## Directory Structure

```
console_error_panic_hook/
├── src/
│   ├── lib.rs              # Main implementation
│   └── imp/                # Platform-specific implementations
│       ├── web.rs          # Browser/WebAssembly implementation
│       └── node.rs         # Node.js implementation
├── example/
│   └── www/                # Example web application
├── Cargo.toml
└── README.md
```

## Installation

```toml
[dependencies]
console_error_panic_hook = "0.1.7"

[dev-dependencies]
console_error_panic_hook = "0.1.7"

[features]
default = ["console_error_panic_hook"]
release = []  # No panic hook in release
```

## Basic Usage

### Enable in lib.rs

```rust
use wasm_bindgen::prelude::*;
use console_error_panic_hook;

#[wasm_bindgen(start)]
pub fn main() {
    // Set up panic hook - MUST be called before any panicking code
    console_error_panic_hook::set_once();

    // Now panics will show in console
    // panic!("This will appear in console.error");
}
```

### Conditional Compilation

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    // Only enable in debug builds
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    // Your application code
    run_app();
}
```

### With wasm-pack

```rust
// In lib.rs or main entry point
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run() {
    // Enable panic hook
    setup_panic_hook();

    // Application logic
    start();
}

fn setup_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
```

## How It Works

### Panic Hook Registration

```rust
// From console_error_panic_hook/src/lib.rs
use std::panic;
use std::sync::Once;

static SET_HOOK: Once = Once::new();

pub fn set_once() {
    SET_HOOK.call_once(|| {
        panic::set_hook(Box::new(console_error_panic));
    });
}

fn console_error_panic(info: &panic::PanicInfo) {
    // Format the panic message
    let message = format_panic_info(info);

    // Call console.error via wasm_bindgen
    error(&message);
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}
```

### Panic Info Formatting

```rust
use std::panic::PanicInfo;

fn format_panic_info(info: &PanicInfo) -> String {
    let mut message = String::from(" panicked at ");

    // Add panic payload
    if let Some(payload) = info.payload().downcast_ref::<&str>() {
        message.push('"');
        message.push_str(payload);
        message.push('"');
    } else if let Some(payload) = info.payload().downcast_ref::<String>() {
        message.push('"');
        message.push_str(payload);
        message.push('"');
    } else {
        message.push_str("unknown");
    }

    // Add location
    if let Some(location) = info.location() {
        message.push_str(", ");
        message.push_str(location.file());
        message.push(':');
        message.push_str(&location.line().to_string());
    }

    message
}
```

## Error Message Examples

### Simple Panic

```rust
// Rust code
panic!("Something went wrong!");
```

Console output:
```javascript
panicked at "Something went wrong!", src/lib.rs:42:5
```

### Assertion Failure

```rust
// Rust code
let x = 5;
assert_eq!(x, 10);
```

Console output:
```javascript
panicked at 'assertion failed: `(left == right)`
  left: `5`,
 right: `10`', src/lib.rs:15:9
```

### Option/Result Unwrap

```rust
// Rust code
let value: Option<i32> = None;
value.unwrap();
```

Console output:
```javascript
panicked at 'called `Option::unwrap()` on a `None` value', src/lib.rs:28:12
```

## Advanced Usage

### Custom Panic Handler

```rust
use std::panic::{self, PanicInfo};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn trace(s: &str);
}

pub fn custom_panic_hook(info: &PanicInfo) {
    // Format main message
    let message = format_panic_info(info);
    error(&message);

    // Optionally add stack trace
    let backtrace = format!("{:?}", backtrace::Backtrace::new());
    trace(&backtrace);

    // Send to error reporting service
    report_error_to_server(&message, &backtrace);
}

fn report_error_to_server(message: &str, backtrace: &str) {
    // Could use web-sys to send to your error tracking
    // e.g., Sentry, LogRocket, etc.
}

pub fn set_custom_hook() {
    panic::set_hook(Box::new(custom_panic_hook));
}
```

### Integration with Error Tracking

```rust
use wasm_bindgen::JsValue;
use web_sys::Request;

pub fn setup_error_tracking(sentry_dsn: &str) {
    console_error_panic_hook::set_once();

    // Also send to Sentry
    panic::set_hook(Box::new(move |info| {
        let message = format_panic_info(info);

        // Log to console (for development)
        web_sys::console::error_1(&JsValue::from_str(&message));

        // Send to Sentry (for production)
        let _ = send_to_sentry(sentry_dsn, &message);
    }));
}

async fn send_to_sentry(dsn: &str, message: &str) -> Result<(), JsValue> {
    let body = serde_json::json!({
        "message": message,
        "level": "error",
        "platform": "javascript",
    });

    let request = Request::new_with_str_and_init(
        &format!("{}/api/envelope/", dsn),
        web_sys::RequestInit::new()
            .method("POST")
            .body(Some(&body.to_string().into())),
    )?;

    web_sys::window()
        .unwrap()
        .fetch_with_request(&request);

    Ok(())
}
```

## Feature Flags

### Development Only

```toml
# Cargo.toml
[dependencies]
console_error_panic_hook = { version = "0.1.7", optional = true }

[features]
default = ["console_error_panic_hook"]
release = []
```

```rust
// lib.rs
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    app::run();
}
```

Build commands:
```bash
# Debug build with panic hook
wasm-pack build

# Release build without panic hook
wasm-pack build --release --no-default-features --features release
```

### Cargo Profile Configuration

```toml
# Cargo.toml
[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
opt-level = "s"
lto = true
```

## Common Patterns

### With anyhow Error Handling

```rust
use anyhow::{Result, Context};
use console_error_panic_hook;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();

    if let Err(e) = run_app() {
        web_sys::console::error_1(&JsValue::from_str(&format!("Error: {:?}", e)));
    }
}

fn run_app() -> Result<()> {
    // Your application code
    initialize().context("Failed to initialize")?;
    setup_handlers().context("Failed to setup handlers")?;
    Ok(())
}
```

### With ResultExt for Better Errors

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn process_data(data: JsValue) -> Result<JsValue, JsValue> {
    // Convert and process
    let parsed: MyStruct = data.into_serde()
        .map_err(|e| format!("Failed to parse input: {:?}", e))?;

    // Process and return
    Ok(wasm_bindgen::JsValue::from_str("success"))
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_panic_hook_enabled() {
        // Set up panic hook for test
        console_error_panic_hook::set_once();

        // Verify it's set (can't directly test, but ensures coverage)
        assert!(true);
    }
}
```

### Integration Test

```rust
// tests/web.rs
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_error_handling() {
    // This should appear in console
    let result = fallible_function();
    assert!(result.is_err());
}

fn fallible_function() -> Result<(), String> {
    Err("Expected error".to_string())
}
```

## Comparison with Alternatives

| Feature | console_error_panic_hook | Custom Hook | No Hook |
|---------|-------------------------|-------------|---------|
| Setup | One line | ~20 lines | None |
| Error messages | Full panic info | Custom format | None |
| Source location | Yes | Optional | No |
| Production overhead | None (feature-gated) | Depends | None |
| Customization | Limited | Full | N/A |

## Related Documents

- [wasm-bindgen](./wasm-bindgen-exploration.md) - Rust/JS interop
- [gloo](./gloo-exploration.md) - Web utilities
- [Twiggy](./twiggy-exploration.md) - Size analysis

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/console_error_panic_hook/`
- Documentation: https://docs.rs/console_error_panic_hook/
- GitHub: https://github.com/rustwasm/console_error_panic_hook
