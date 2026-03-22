---
name: Wasm-Bindgen
description: Bridge between WebAssembly and JavaScript enabling bidirectional calls
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/wasm-bindgen/
---

# Wasm-Bindgen - Rust/JavaScript Interoperability

## Overview

Wasm-Bindgen is the **fundamental tool** for Rust ↔ JavaScript interoperability in WebAssembly. It generates the glue code needed to call exported Rust functions from JavaScript and import JavaScript APIs into Rust.

Key features:
- **Automatic type conversion** - Rust types to/from JS types
- **Closure support** - Pass Rust closures to JavaScript
- **JsValue wrapper** - Dynamic JavaScript value handling
- **DOM bindings** - Access web APIs directly from Rust
- **Async/await support** - Promise-based async patterns
- **Weak references** - Proper memory management across the boundary

## Directory Structure

```
wasm-bindgen/
├── crates/
│   ├── wasm-bindgen/           # Core runtime library
│   ├── wasm-bindgen-macro/     # Procedural macros
│   ├── wasm-bindgen-macro-support/ # Macro implementation
│   ├── wasm-bindgen-backend/   # Code generation backend
│   ├── wasm-bindgen-cli/       # Command-line tool
│   ├── wasm-bindgen-shared/    # Shared types between crates
│   ├── wasm-bindgen-test/      # Testing framework
│   └── web-sys/                # Web API bindings
├── examples/
│   ├── todo-app/               # TodoMVC example
│   ├── wasm-audio/             # Audio processing demo
│   ├── webrtc/                 # WebRTC example
│   └── ...
├── guides/
│   └── reference/              # Documentation
├── Cargo.toml
└── README.md
```

## Core Concepts

### Basic Export

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

JavaScript usage:
```javascript
import * as wasm from './pkg/wasm_bindgen.js';

wasm.add(2, 3); // 5
wasm.greet("World"); // "Hello, World!"
```

### Importing JavaScript

```rust
use wasm_bindgen::prelude::*;

// Import global JavaScript function
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn hello() {
    log("Hello from Rust!");
}

// Import specific module
#[wasm_bindgen(module = "/js/utils.js")]
extern "C" {
    fn helper_function(x: i32) -> i32;
}
```

### JsValue - Dynamic JavaScript Values

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn process_js_value(val: &JsValue) {
    // Check type
    if val.is_string() {
        let s = val.as_string().unwrap();
        console_log!("Got string: {}", s);
    } else if val.is_number() {
        let n = val.as_f64().unwrap();
        console_log!("Got number: {}", n);
    } else if val.is_object() {
        console_log!("Got object");
    }

    // Create JsValues
    let string_val = JsValue::from_str("Hello");
    let number_val = JsValue::from_f64(42.0);
    let bool_val = JsValue::from_bool(true);
    let null_val = JsValue::NULL;
    let undefined_val = JsValue::UNDEFINED;
}
```

### Working with Objects

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub fn process_object(obj: &JsValue) {
    // Get properties
    let name = js_sys::Reflect::get(obj, &JsValue::from_str("name"))
        .unwrap()
        .as_string()
        .unwrap();

    // Set properties
    js_sys::Reflect::set(
        obj,
        &JsValue::from_str("processed"),
        &JsValue::from_bool(true),
    ).unwrap();

    // Type-safe casting
    if let Some(element) = obj.dyn_ref::<web_sys::HtmlElement>() {
        element.set_inner_text("Modified from Rust");
    }
}
```

### Closures

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use web_sys::{Document, Window};

#[wasm_bindgen]
pub fn setup_click_handler() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    // Create closure
    let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        console_log!("Clicked at ({}, {})", event.client_x(), event.client_y());
    }) as Box<dyn FnMut(_)>);

    // Get element
    let element = document
        .get_element_by_id("my-button")
        .unwrap();

    // Add event listener
    element
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .set_onclick(Some(closure.as_ref().unchecked_ref()));

    // Forget closure (prevents dropping)
    closure.forget();
}

// One-time closure
#[wasm_bindgen]
pub fn fetch_data() {
    let closure = Closure::once(|data: JsValue| {
        console_log!("Data received: {:?}", data);
    });

    // Pass to JavaScript, will be dropped after call
    js_sys::Promise::resolve(&JsValue::from_str("test"))
        .then(closure.as_ref().unchecked_ref());

    closure.forget(); // Or manage lifetime
}
```

### Async/Await

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};
use js_sys::Promise;

#[wasm_bindgen]
pub async fn fetch_json(url: String) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into()?;

    let json = JsFuture::from(resp.json()?).await?;
    Ok(json)
}

// Using with Promise
#[wasm_bindgen]
pub fn async_operation() -> Promise {
    wasm_bindgen_futures::future_to_promise(async {
        // Async Rust code
        let result = do_async_work().await;

        // Return to JavaScript
        match result {
            Ok(value) => Ok(JsValue::from(value)),
            Err(err) => Err(JsValue::from_str(&err.to_string())),
        }
    })
}
```

## Web API Bindings (web-sys)

### DOM Manipulation

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, Window};

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global window");
    let document = window.document().expect("no document");

    // Create element
    let div = document.create_element("div")?;
    div.set_inner_html("<h1>Hello from Rust!</h1>");

    // Get body
    let body = document.body().expect("no body");
    body.append_child(&div)?;

    // Query selector
    if let Some(element) = document.query_selector("#app")? {
        element.set_inner_content("Content loaded");
    }

    Ok(())
}
```

### Event Handling

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlButtonElement};

#[wasm_bindgen]
pub struct App {
    count: i32,
    closure: Option<Closure<dyn FnMut(Event)>>,
}

#[wasm_bindgen]
impl App {
    pub fn new() -> App {
        App { count: 0, closure: None }
    }

    pub fn setup_counter(&mut self) {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let button = document
            .get_element_by_id("counter-btn")
            .unwrap()
            .dyn_into::<HtmlButtonElement>()
            .unwrap();

        // Create closure that captures self reference
        let closure = Closure::wrap(Box::new(move |event: Event| {
            event.prevent_default();
            console_log!("Button clicked!");
        }) as Box<dyn FnMut(_)>);

        button.set_onclick(Some(closure.as_ref().unchecked_ref()));
        self.closure = Some(closure);
    }
}
```

### Canvas Rendering

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

#[wasm_bindgen]
pub fn draw_circle(x: i32, y: i32, radius: i32) -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .get_element_by_id("my_canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()?;

    let ctx = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()?;

    // Draw circle
    ctx.begin_path();
    ctx.arc(x as f64, y as f64, radius as f64, 0.0, 2.0 * std::f64::consts::PI)?;
    ctx.set_fill_style(&JsValue::from_str("blue"));
    ctx.fill();

    Ok(())
}
```

## Advanced Patterns

### Structs with Methods

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Point {
    x: f64,
    y: f64,
}

#[wasm_bindgen]
impl Point {
    #[wasm_bindgen(constructor)]
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn x(&self) -> f64 {
        self.x
    }

    pub fn y(&self) -> f64 {
        self.y
    }

    pub fn distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }
}
```

JavaScript usage:
```javascript
const p1 = new Point(0, 0);
const p2 = new Point(3, 4);
p1.distance(p2); // 5
p1.translate(1, 1);
```

### Inheritance and Traits

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = EventTarget)]
    pub type EventTarget;

    #[wasm_bindgen(js_name = HTMLElement)]
    pub type HtmlElement;

    // Inheritance
    #[wasm_bindgen(method, structural, js_name = addEventListener)]
    pub fn add_event_listener(
        this: &EventTarget,
        event_name: &str,
        callback: &js_sys::Function,
    );

    // Cast from EventTarget to HtmlElement
    #[wasm_bindgen(method, getter)]
    pub fn inner_html(this: &HtmlElement) -> String;
}

// Runtime type check
pub fn process_node(node: &web_sys::Node) {
    if let Some(element) = node.dyn_ref::<web_sys::HtmlElement>() {
        // Safe to use as HtmlElement
    }
}
```

### Memory Management

```rust
use wasm_bindgen::prelude::*;

// Explicit memory cleanup
#[wasm_bindgen]
pub struct LargeBuffer {
    data: Vec<u8>,
}

#[wasm_bindgen]
impl LargeBuffer {
    pub fn new(size: usize) -> LargeBuffer {
        LargeBuffer {
            data: vec![0; size],
        }
    }

    // Explicit cleanup method
    pub fn clear(&mut self) {
        self.data.clear();
        self.data.shrink_to_fit();
    }
}

// Weak references for callbacks
#[wasm_bindgen]
pub struct CallbackManager {
    callbacks: Vec<wasm_bindgen::closure::Closure<dyn FnMut()>>,
}

impl CallbackManager {
    pub fn add_callback<F>(&mut self, callback: F)
    where
        F: FnMut() + 'static,
    {
        let closure = wasm_bindgen::closure::Closure::wrap(
            Box::new(callback) as Box<dyn FnMut()>
        );
        self.callbacks.push(closure);
    }
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
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }

    #[wasm_bindgen_test]
    async fn test_async() {
        let result = async_operation().await;
        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_dom() {
        let document = web_sys::window().unwrap().document().unwrap();
        let element = document.create_element("div").unwrap();
        element.set_inner_html("test");
        assert_eq!(element.inner_html(), "test");
    }
}
```

Run tests:
```bash
wasm-pack test --headless --firefox
```

## Performance Considerations

### Minimizing Boundary Crossings

```rust
// Bad: many boundary crossings
#[wasm_bindgen]
pub fn process_array_bad(arr: &js_sys::Array) -> js_sys::Array {
    let result = js_sys::Array::new();
    for i in 0..arr.length() {
        let val = arr.get(i);
        let processed = process_value(&val);
        result.push(&processed);
    }
    result
}

// Good: single conversion
#[wasm_bindgen]
pub fn process_array_good(arr: &js_sys::Array) -> js_sys::Array {
    // Convert to Vec, process in Rust, convert back
    let vec: Vec<f64> = arr.iter().filter_map(|v| v.as_f64()).collect();
    let result: Vec<JsValue> = vec.into_iter()
        .map(|v| {
            let processed = v * 2.0 + 1.0;
            JsValue::from_f64(processed)
        })
        .collect();
    result.into_iter().collect()
}
```

### Serialization Optimization

```rust
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};

#[derive(Serialize, Deserialize)]
pub struct Data {
    name: String,
    values: Vec<f64>,
}

// Efficient serialization
#[wasm_bindgen]
pub fn process_data(json: &JsValue) -> Result<JsValue, JsValue> {
    let data: Data = from_value(json.clone())?;

    // Process in Rust
    let result = Data {
        name: data.name.to_uppercase(),
        values: data.values.iter().map(|v| v * 2.0).collect(),
    };

    Ok(to_value(&result)?)
}
```

## Related Documents

- [Wasm-Pack](./wasm-pack-exploration.md) - Build tooling
- [Gloo](./gloo-exploration.md) - Idiomatic web APIs
- [Console Error Panic Hook](./console-error-panic-hook-exploration.md) - Error handling

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/wasm-bindgen/`
- Wasm-Bindgen Guide: https://rustwasm.github.io/wasm-bindgen/
- API Documentation: https://docs.rs/wasm-bindgen/
