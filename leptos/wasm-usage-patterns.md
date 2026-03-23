# WASM Usage Patterns in Leptos

## Overview

Leptos compiles to WebAssembly (WASM) for client-side execution, enabling Rust code to run in the browser. This document covers how Leptos utilizes WASM, compilation patterns, and runtime behaviors.

---

## WASM Compilation Target

### Target Configuration

```toml
# .cargo/config.toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "link-args=-z stack-size=1048576"]

# rust-toolchain.toml
[toolchain]
channel = "nightly-2025-06-07"  # Specific nightly for WASM features
```

### Build Process

```bash
# cargo-leptos orchestrates the build
cargo leptos watch

# Under the hood:
# 1. Compile to WASM
cargo build --target wasm32-unknown-unknown --release

# 2. Process with wasm-bindgen
wasm-bindgen target/wasm32-unknown-unknown/release/app.wasm \
  --out-dir pkg \
  --target web

# 3. Optimize with wasm-opt (via binaryen)
wasm-opt -Oz pkg/app_bg.wasm -o pkg/app_optimized.wasm
```

---

## WASM Bindings

### web-sys Integration

Leptos uses `web-sys` for DOM bindings:

```rust
// Conditional compilation
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_sys::{Element, Node, Text, Document, Window};

#[cfg(not(target_arch = "wasm32"))]
// Server-side stub types
use crate::stub::{Element, Node, Text};
```

### JS Value Handling

```rust
use wasm_bindgen::JsValue;

// Error handling from JS
impl UnwrapOrDebug for Result<T, JsValue> {
    fn or_debug(self, el: &Node, name: &'static str) {
        #[cfg(any(debug_assertions, leptos_debuginfo))]
        {
            if let Err(err) = self {
                web_sys::console::warn_3(
                    &JsValue::from_str(&format!("[WARNING] Error at {name}")),
                    el,
                    &err,
                );
            }
        }
    }
}
```

---

## Executor Abstraction

### any_spawner Crate

Leptos needs to work with different async runtimes:
- **Browser**: `wasm-bindgen-futures`
- **Server**: `tokio`, `async-std`
- **Native GUI**: `glib`

```rust
// any_spawner/src/lib.rs

// Type aliases
type SpawnFn = fn(PinnedFuture<()>);
type SpawnLocalFn = fn(PinnedLocalFuture<()>);
type PollLocalFn = fn();

static EXECUTOR_FNS: OnceLock<ExecutorFns> = OnceLock::new();

pub struct Executor;

impl Executor {
    // Spawn Send + 'static future
    pub fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
        let pinned_fut = Box::pin(fut);
        if let Some(fns) = EXECUTOR_FNS.get() {
            (fns.spawn)(pinned_fut);
        } else {
            handle_uninitialized_spawn(pinned_fut);
        }
    }

    // Spawn !Send future (used in WASM)
    pub fn spawn_local(fut: impl Future<Output = ()> + 'static) {
        let pinned_fut = Box::pin(fut);
        if let Some(fns) = EXECUTOR_FNS.get() {
            (fns.spawn_local)(pinned_fut);
        } else {
            handle_uninitialized_spawn_local(pinned_fut);
        }
    }
}
```

### WASM Executor Initialization

```rust
// In browser entry point
#[cfg(target_arch = "wasm32")]
pub fn init_wasm() {
    Executor::init_wasm_bindgen();
    // Now Executor::spawn uses wasm-bindgen-futures
}

// wasm-bindgen executor
impl Executor {
    pub fn init_wasm_bindgen() -> Result<(), ExecutorError> {
        EXECUTOR_FNS.get_or_try_init(|| Ok(ExecutorFns {
            spawn: |fut| {
                // wasm-bindgen-futures spawns on JS event loop
                wasm_bindgen_futures::spawn_local(fut);
            },
            spawn_local: |fut| {
                wasm_bindgen_futures::spawn_local(fut);
            },
            poll_local: no_op_poll,
        }))?;
        Ok(())
    }
}
```

---

## Hydration in WASM

### WASM-Specific Hydration Flow

```rust
// hydration_context/src/hydrate.rs

pub struct HydrationContext {
    // Data sent from server
    serialized_data: Arc<RwLock<HashMap<SerializedDataId, String>>>,
    // Track which chunks are being hydrated
    hydration_state: Arc<RwLock<HydrationState>>,
}

impl SharedContext for HydrationContext {
    fn read_data(&self, id: &SerializedDataId) -> Option<String> {
        // Read data that was serialized from server
        self.serialized_data
            .read()
            .unwrap()
            .get(id)
            .cloned()
    }

    fn during_hydration(&self) -> bool {
        // True while WASM is hydrating
        *self.hydration_state.read().unwrap() == HydrationState::Hydrating
    }

    fn hydration_complete(&self) {
        // Signal hydration is done
        *self.hydration_state.write().unwrap() = HydrationState::Complete;
    }
}
```

### Hydration Script Injection

```rust
// Server-side: inject hydration data
fn serialize_resources(resources: Vec<Resource>) -> String {
    let mut script = r#"<script id="__LEPTOS_RESOURCE_DATA__" type="application/json">"#.to_string();
    script.push_str(&serde_json::to_string(&resources).unwrap());
    script.push_str("</script>");
    script
}

// Client-side: read hydration data
#[wasm_bindgen(inline_js = r#"
    export function getHydrationData() {
        const el = document.getElementById('__LEPTOS_RESOURCE_DATA__');
        return el ? JSON.parse(el.textContent) : null;
    }
"#)]
extern "C" {
    pub fn get_hydration_data() -> JsValue;
}
```

---

## DOM Operations via WASM

### Renderer Trait

```rust
// tachys/src/renderer/mod.rs

pub trait Renderer: Clone + Sized + 'static {
    type Node: Clone + Debug;
    type Element: AsRef<Self::Node> + Clone;
    type Text: AsRef<Self::Node> + Clone;
    type Placeholder: AsRef<Self::Node> + Clone;

    // DOM creation
    fn create_element(tag: &str) -> Self::Element;
    fn create_text_node(text: &str) -> Self::Text;
    fn create_comment(text: &str) -> Self::Node;

    // DOM manipulation
    fn insert_child_before(parent: &Self::Element, child: &Self::Node, reference: Option<&Self::Node>);
    fn remove_child(parent: &Self::Element, child: &Self::Node);
    fn replace_child(parent: &Self::Element, old: &Self::Node, new_node: &Self::Node);

    // Event handling
    fn add_event_listener(el: &Self::Element, event: &str, handler: Box<dyn FnMut()>);
}
```

### DOM Renderer Implementation

```rust
// tachys/src/renderer/dom.rs

#[derive(Clone, Copy, Debug)]
pub struct Dom;

impl Renderer for Dom {
    type Node = web_sys::Node;
    type Element = web_sys::Element;
    type Text = web_sys::Text;
    type Placeholder = web_sys::Comment;

    fn create_element(tag: &str) -> Self::Element {
        let document = document();
        document.create_element(tag).unwrap()
    }

    fn insert_child_before(
        parent: &Self::Element,
        child: &Self::Node,
        reference: Option<&Self::Node>,
    ) {
        match reference {
            Some(reference) => {
                parent.insert_before(child, Some(reference)).unwrap();
            }
            None => {
                parent.append_child(child).unwrap();
            }
        }
    }

    fn add_event_listener(
        el: &Self::Element,
        event: &str,
        handler: Box<dyn FnMut()>,
    ) {
        // Convert Rust closure to JS closure
        let closure = Closure::wrap(handler as Box<dyn FnMut()>);
        el.add_event_listener_with_callback(event, closure.as_ref())
            .unwrap();
        closure.forget();  // Leak closure for 'static lifetime
    }
}
```

---

## WASM Binary Size Optimization

### Release Profile Configuration

```toml
# leptos/Cargo.toml
[profile.release]
codegen-units = 1  # Better optimization
lto = true         # Link-time optimization
opt-level = 'z'    # Optimize for size
```

### WASM-Specific Optimizations

```toml
# Cargo.toml for WASM target
[profile.release-wasm]
inherits = "release"
lto = true
opt-level = 'z'
strip = true       # Remove debug symbols
panic = "abort"    # Smaller than unwind
```

### Code Splitting

```rust
// cargo-leptos supports WASM splitting
// Split by route
lazy_routes! {
    "/" => HomeRoute,
    "/about" => AboutRoute,
    "/dashboard" => DashboardRoute,  // Loaded on demand
}

// Dynamic imports
async fn load_dashboard() {
    // Fetch additional WASM blob
    let dashboard = wasm_pack::import("/dashboard.wasm").await;
}
```

### Binary Size Comparison

| Optimization | Size | Reduction |
|-------------|------|-----------|
| Debug build | 15 MB | - |
| Release build | 1.2 MB | 92% |
| + LTO | 900 KB | 25% |
| + opt-level z | 700 KB | 22% |
| + Islands | 350 KB | 50% |
| + Code splitting | 150 KB initial | 57% |

---

## Thread Model in WASM

### Single-Threaded (Default)

```rust
// wasm-bindgen runs on main thread
// All JS/WASM interaction is !Send

// Local-only signals (faster)
let local_signal = RwSignal::new_local(0);

// spawn_local for !Send futures
Executor::spawn_local(async {
    // Runs on main thread
    dom_manipulation().await;
});
```

### Multi-Threaded WASM (Experimental)

```rust
// With --features=wasm-bindgen/atomics
// Compile with: RUSTFLAGS="-C target-feature=+atomics,+bulk-memory"

// Thread-safe signals
let shared = ArcRwSignal::new(0);

// Spawn on thread pool
Executor::spawn(async move {
    // Runs on Web Worker
    compute_intensive().await;
});
```

---

## Console Logging from WASM

### Isomorphic Logging

```rust
// leptos_dom/src/logging.rs

/// Log to browser console or terminal
pub fn log(text: impl Display) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&text.to_string().into());

    #[cfg(not(target_arch = "wasm32"))]
    println!("{}", text);
}

/// Warn in browser console or stderr
pub fn warn(text: impl Display) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&text.to_string().into());

    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{}", text);
}
```

### Panic Hook

```rust
// Browser entry point
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Better panic messages in browser
    console_error_panic_hook::set_once();

    // Initialize logging
    console_log::init_with_level(log::Level::Debug)?;

    // Run app
    leptos::mount_to_body(App);
    Ok(())
}
```

---

## Event Handling

### Native Events

```rust
// tachys/src/html/event.rs

use wasm_bindgen::JsCast;
use web_sys::{Event, MouseEvent, KeyboardEvent};

pub trait EventDescriptor: Clone {
    type EventType: JsCast;

    fn name(&self) -> &str;
}

#[derive(Clone)]
pub struct Click;

impl EventDescriptor for Click {
    type EventType = MouseEvent;

    fn name(&self) -> &str {
        "click"
    }
}

// Add event listener
fn add_event<E: EventDescriptor>(
    el: &web_sys::Element,
    _event: E,
    handler: impl Fn(E::EventType) + 'static,
) {
    let listener = move |event: Event| {
        // Cast to specific event type
        let typed_event = event.dyn_into::<E::EventType>().unwrap();
        handler(typed_event);
    };

    let closure = Closure::wrap(Box::new(listener) as Box<dyn FnMut(_)>);
    el.add_event_listener_with_callback(
        &_event.name(),
        closure.as_ref(),
    ).unwrap();
    closure.forget();
}
```

### Event Delegation

```rust
// Optional feature for performance
#[cfg(feature = "delegation")]
pub fn add_delegated_event(event: &str) {
    // Add single listener to document
    let document = document();
    let handler = Closure::wrap(Box::new(move |e: Event| {
        // Find target and dispatch
        let target = e.target().unwrap();
        dispatch_event(event, &target);
    }) as Box<dyn FnMut(_)>);

    document
        .add_event_listener_with_callback(event, handler.as_ref())
        .unwrap();
    handler.forget();
}
```

---

## Request Animation Frame

### Reactive Animations

```rust
// leptos_dom/src/helpers/raf.rs

use wasm_bindgen::prelude::*;
use web_sys::window;

pub fn request_animation_frame(f: &Closure<dyn FnMut()>) -> i32 {
    window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap()
}

pub fn request_animation_frame_callback<F>(cb: F) -> RaFrameCallback
where
    F: FnMut(f64) + 'static,
{
    let closure = Closure::wrap(Box::new(cb) as Box<dyn FnMut(f64)>);
    let id = request_animation_frame(&closure);
    RaFrameCallback { closure, id }
}

// Usage in component
let mut frame = request_animation_frame_callback(|timestamp| {
    // Update animation state
    position.set(calculate_position(timestamp));
});
```

---

## LocalStorage/SessionStorage

### Browser-Only APIs

```rust
// leptos/src/prelude.rs

use web_sys::{Storage, window};

pub fn local_storage() -> Option<Storage> {
    #[cfg(target_arch = "wasm32")]
    {
        window()
            .and_then(|w| w.local_storage().ok())
            .flatten()
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

pub fn get_from_storage<T: serde::de::DeserializeOwned>(
    key: &str,
) -> Option<T> {
    local_storage()
        .and_then(|s| s.get(key).ok().flatten())
        .and_then(|json| serde_json::from_str(&json).ok())
}

pub fn set_storage<T: serde::Serialize>(key: &str, value: &T) {
    if let Some(storage) = local_storage() {
        let json = serde_json::to_string(value).unwrap();
        storage.set(key, &json).ok();
    }
}
```

---

## Fetch API Integration

### gloo-net for HTTP

```rust
// server_fn/src/client.rs

#[cfg(feature = "browser")]
use gloo_net::http::Request;

pub async fn send_request(
    url: String,
    data: Vec<u8>,
) -> Result<Vec<u8>, ServerFnError> {
    #[cfg(target_arch = "wasm32")]
    {
        // Use browser Fetch API via gloo-net
        let response = Request::post(&url)
            .body(data)
            .map_err(|e| ServerFnError::Request(e.to_string()))?
            .send()
            .await
            .map_err(|e| ServerFnError::Request(e.to_string()))?;

        response
            .binary()
            .await
            .map_err(|e| ServerFnError::Response(e.to_string()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Use reqwest on server
        reqwest::Client::new()
            .post(&url)
            .body(data)
            .send()
            .await?
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(Into::into)
    }
}
```

---

## WASM Memory Management

### Arena Allocation

```rust
// reactive_graph uses slotmap for arena allocation
// This avoids individual heap allocations for signals

use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct ReactiveNodeKey; }

pub struct ReactiveArena {
    nodes: SlotMap<ReactiveNodeKey, ReactiveNode>,
}

impl ReactiveArena {
    pub fn insert(&mut self, node: ReactiveNode) -> ReactiveNodeKey {
        self.nodes.insert(node)
    }

    pub fn remove(&mut self, key: ReactiveNodeKey) {
        self.nodes.remove(key);
    }
}
```

### Garbage Collection Strategy

```rust
// No GC in WASM - manual cleanup via Owner

impl Owner {
    pub fn dispose(&self) {
        let inner = self.inner.write().or_poisoned();

        // 1. Cancel all child effects
        for effect in &inner.effects {
            effect.cancel();
        }

        // 2. Drop all arena items
        for (_key, item) in inner.arena.drain() {
            drop(item);
        }

        // 3. Run cleanup callbacks
        for cleanup in inner.cleanups.drain(..) {
            cleanup();
        }
    }
}
```

---

## Debugging WASM

### Source Maps

```toml
# Cargo.toml
[profile.dev]
debug = true  # Include debug info for source maps

# wasm-bindgen generates .wasm.map files
# Browser devtools can show original Rust source
```

### Console Trace

```rust
#[cfg(target_arch = "wasm32")]
fn debug_trace(message: &str) {
    web_sys::console::trace_0();
    web_sys::console::log_1(&message.into());
}
```

---

## Resources

- [wasm-bindgen Guide](https://rustwasm.github.io/wasm-bindgen/)
- [Trunk Build Tool](https://trunkrs.dev/)
- [cargo-leptos](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/cargo-leptos)
