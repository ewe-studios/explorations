---
name: Gloo
description: Idiomatic Rust wrappers for Web APIs designed for WASM and web development
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/gloo/
---

# Gloo - Idiomatic Rust Web APIs

## Overview

Gloo is a **collection of ergonomic, idiomatic Rust wrappers for Web APIs** designed specifically for WebAssembly and modern web development. It provides a modular set of crates that make working with browser APIs feel natural in Rust, with proper error handling, async support, and type safety.

Key features:
- **Modular design** - Use only what you need
- **Idiomatic Rust** - Proper error handling, no raw JS values
- **Async first** - Futures-based APIs
- **Type safe** - Compile-time guarantees
- **Zero-cost abstractions** - Minimal overhead over raw JS
- **WASM optimized** - Designed for wasm-bindgen ecosystem

## Directory Structure

```
gloo/
├── crates/
│   ├── gloo/                    # Re-export crate (all-in-one)
│   ├── gloo-utils/              # Core utilities (js-sys, web-sys)
│   ├── gloo-console/            # Console API wrapper
│   ├── gloo-dialogs/            # Alert, confirm, prompt
│   ├── gloo-events/             # Event handling utilities
│   ├── gloo-file/               # File API (FileReader, Blob)
│   ├── gloo-history/            # Browser history API
│   ├── gloo-net/                # HTTP (fetch), WebSocket
│   ├── gloo-render/             # requestAnimationFrame
│   ├── gloo-storage/            # localStorage, sessionStorage
│   ├── gloo-timers/             # setTimeout, setInterval
│   ├── gloo-worker/             # Web Workers
│   ├── gloo-history/            # History API
│   └── gloo-events/             # Event listeners
├── examples/
├── Cargo.toml
└── README.md
```

## Core Crates

### gloo-utils

```rust
use gloo_utils::document;
use gloo_utils::window;
use gloo_utils::errors::JsError;

// Access global objects safely
let doc = document();
let win = window();

// Type-safe access to properties
let title = doc.title();
let width = win.inner_width().unwrap();
```

### gloo-console

```rust
use gloo_console::{log, info, warn, error, debug, table};

// Basic logging
log("Hello from Rust!");
info!("Information: {}", 42);
warn!("Warning message");
error!("Error: {:?}", some_error);

// Structured logging
let data = serde_json::json!({
    "user": "alice",
    "action": "login",
});
log!("User action:", data);

// Table view
let users = vec![
    serde_json::json!({ "name": "Alice", "age": 30 }),
    serde_json::json!({ "name": "Bob", "age": 25 }),
];
table(&users);

// Time tracking
use gloo_console::profile;

profile!("expensive_operation");
// ... do work ...
profile_end!("expensive_operation");

// Assert
use gloo_console::assert;
assert!(condition, "Condition must be true");
```

### gloo-timers

```rust
use gloo_timers::callback::Timeout;
use gloo_timers::future::{TimeoutFuture, sleep};

// Callback-based timeout
let timeout = Timeout::new(1000, || {
    console_log!("One second elapsed!");
});
// Timeout automatically cancels when dropped

// Future-based (async/await)
async fn wait_one_second() {
    sleep(1000).await;
    console_log!("One second elapsed!");
}

// Interval
use gloo_timers::callback::Interval;

let interval = Interval::new(1000, || {
    console_log!("Tick!");
});
// Interval automatically cancels when dropped

// Clear interval manually
drop(interval);

// Async interval with stream
use gloo_timers::future::IntervalStream;
use futures::StreamExt;

let mut interval = IntervalStream::new(1000);
while let Some(_) = interval.next().await {
    console_log!("Tick!");
}
```

### gloo-events

```rust
use gloo_events::{EventListener, EventListenerClosure};
use web_sys::{Element, Event};

// Simple event listener
let listener = EventListener::new(
    &element,
    "click",
    |event: &web_sys::MouseEvent| {
        console_log!("Clicked at ({}, {})", event.client_x(), event.client_y());
    }
);
// Automatically removes listener when dropped

// Manual control
let mut listener = EventListener::new(&element, "click", handler);
listener.forget();  // Don't auto-remove
listener.remove();  // Manually remove

// Multiple events
use gloo_events::EventListenerGroup;

let mut group = EventListenerGroup::new();
group.register(&element1, "click", handler1);
group.register(&element2, "mouseover", handler2);
group.register(&element3, "keydown", handler3);
// All listeners removed when group is dropped

// Custom events
use gloo_events::EventTarget;

let target = EventTarget::new();
target.add_event_listener("custom", |event: &Event| {
    console_log!("Custom event!");
});
target.dispatch_event(&Event::new("custom").unwrap());
```

### gloo-file

```rust
use gloo_file::{File, Blob, FileReader};
use gloo_file::callbacks::{FileReader as FileReaderCb, ReadAsDataUrl};
use gloo_file::future::{FileReader as FileReaderFut, ReadAsText};

// File input handling
let input: web_sys::HtmlInputElement = /* ... */;
let files: Vec<File> = input.files()
    .unwrap()
    .iter()
    .collect();

// Read file as text (callback)
let file = files[0].clone();
let reader = FileReaderCb::new(move |result| {
    match result {
        Ok(content) => console_log!("File content: {}", content),
        Err(e) => console_error!("Error reading file: {:?}", e),
    }
});
reader.read_as_data_url(file);

// Read file as text (async/await)
let file = files[0].clone();
let content = FileReaderFut::new(file)
    .read_as_text()
    .await
    .unwrap();
console_log!("Content: {}", content);

// Blob operations
let blob = Blob::new(&[file.as_bytes()]);
let size = blob.size();
let mime_type = blob.type_();

// Slice blob
let sliced = blob.slice(0, 1024).unwrap();

// Download blob
use gloo_file::download;

download::blob(&blob, "filename.txt");
download::text("Hello, World!", "hello.txt");
download::json(&data, "data.json")?;
```

### gloo-net

```rust
use gloo_net::http::Request;
use gloo_net::websocket::{WebSocket, Message};

// HTTP Fetch API
use gloo_net::http::{Request, Method, Headers};

// GET request
let response = Request::get("https://api.example.com/data")
    .send()
    .await?;

if response.ok() {
    let data: serde_json::Value = response.json().await?;
}

// POST with JSON body
let response = Request::post("https://api.example.com/users")
    .header("Content-Type", "application/json")
    .json(&serde_json::json!({
        "name": "Alice",
        "email": "alice@example.com",
    }))?
    .send()
    .await?;

// Custom headers
let mut headers = Headers::new();
headers.append("Authorization", "Bearer token123");
headers.append("Accept", "application/json");

let response = Request::get("https://api.example.com/protected")
    .headers(headers)
    .send()
    .await?;

// WebSocket
let mut ws = WebSocket::open("wss://echo.example.com")?;

// Send message
ws.send(Message::Text("Hello, Server!".to_string()))?;

// Receive messages
while let Some(msg) = ws.next().await {
    match msg? {
        Message::Text(text) => console_log!("Received: {}", text),
        Message::Bytes(data) => console_log!("Received bytes: {:?}", data),
    }
}

// WebSocket with reconnection
use gloo_net::websocket::futures::WebSocket as WsFutures;

let ws = WsFutures::open("wss://example.com").await?;
```

### gloo-storage

```rust
use gloo_storage::{Storage, LocalStorage, SessionStorage};
use gloo_storage::errors::StorageError;

// LocalStorage
let name: String = LocalStorage::get("user_name")?;
LocalStorage::set("user_name", "Alice")?;
LocalStorage::remove("user_name");
LocalStorage::clear()?;

// SessionStorage
SessionStorage::set("session_id", "abc123")?;
let id: String = SessionStorage::get("session_id")?;

// Custom types with serde
#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

let user = User { id: 1, name: "Alice".to_string() };
LocalStorage::set("user", &user)?;
let user: User = LocalStorage::get("user")?;

// Handle missing keys
let maybe_name: Option<String> = LocalStorage::get("optional_key")?;

// Handle errors
match LocalStorage::get::<String>("key") {
    Ok(value) => console_log!("Got: {}", value),
    Err(StorageError::KeyNotFound(_)) => console_log!("Key not found"),
    Err(StorageError::DeserializationError(e)) => console_error!("Bad data: {}", e),
    Err(e) => console_error!("Storage error: {:?}", e),
}
```

### gloo-dialogs

```rust
use gloo_dialogs::{alert, confirm, prompt};

// Alert
alert("Operation completed!");

// Confirm
if confirm("Are you sure?") {
    // User clicked OK
} else {
    // User clicked Cancel
}

// Prompt
if let Some(name) = prompt("What is your name?", "Enter name here") {
    console_log!("Hello, {}!", name);
}

// Custom prompts not supported (browser limitation)
```

### gloo-render

```rust
use gloo_render::{request_animation_frame, AnimationFrame};
use gloo_render::futures::AnimationFrameFuture;

// Callback-based
let id = request_animation_frame(|timestamp| {
    console_log!("Frame at {} ms", timestamp);
    // Schedule next frame
    request_animation_frame(callback);
});

// Cancel animation frame
gloo_render::cancel_animation_frame(id);

// Future-based
async fn animate() {
    loop {
        let timestamp = AnimationFrameFuture::new().await;
        console_log!("Frame at {} ms", timestamp);
        // Break condition
        if should_stop() {
            break;
        }
    }
}

// Animation loop with state
struct Animation {
    rotation: f32,
}

impl Animation {
    fn new() -> Self {
        Self { rotation: 0.0 }
    }

    fn start(mut self) {
        wasm_bindgen_futures::spawn_local(async move {
            loop {
                AnimationFrameFuture::new().await;
                self.rotation += 0.1;
                self.render();
            }
        });
    }

    fn render(&self) {
        // Render with current rotation
    }
}
```

### gloo-worker

```rust
use gloo_worker::{Worker, Bridged, Registrable};
use serde::{Serialize, Deserialize};

// Define worker message types
#[derive(Serialize, Deserialize)]
enum Input {
    Compute(u64),
    Ping,
}

#[derive(Serialize, Deserialize)]
enum Output {
    Result(u64),
    Pong,
    Progress(f32),
}

// Define worker
pub struct ComputeWorker;

impl Worker for ComputeWorker {
    type Message = ();
    type Input = Input;
    type Output = Output;

    fn create(scope: &WorkerScope<Self>) -> Self {
        ComputeWorker
    }

    fn update(&mut self, scope: &WorkerScope<Self>, msg: Self::Message) {
        // Handle internal messages
    }

    fn received(&mut self, scope: &WorkerScope<Self>, msg: Self::Input) {
        match msg {
            Input::Compute(n) => {
                // Long-running computation
                let result = fibonacci(n);
                scope.send_message(Output::Result(result));
            }
            Input::Ping => {
                scope.send_message(Output::Pong);
            }
        }
    }
}

// Use worker from main thread
let worker = ComputeWorker::bridged(scope);
worker.send(Input::Compute(42));

// Receive output
let handler = scope.callback(|msg: Output| {
    match msg {
        Output::Result(n) => Msg::ComputationDone(n),
        Output::Pong => Msg::Pong,
        Output::Progress(p) => Msg::ProgressUpdate(p),
    }
});
```

## Integration Patterns

### Yew Integration

```rust
use yew::prelude::*;
use gloo_console::log;
use gloo_timers::callback::Timeout;
use gloo_storage::{Storage, LocalStorage};

#[function_component(App)]
fn app() -> Html {
    let count = use_state(|| 0);

    // Load from localStorage on mount
    {
        let count = count.clone();
        use_effect_with((), move |_| {
            if let Ok(saved) = LocalStorage::get::<u32>("count") {
                count.set(saved);
            }
            || {
                // Cleanup: save on unmount
                LocalStorage::set("count", *count).ok();
            }
        });
    }

    // Debounced save
    {
        let count = count.clone();
        use_effect_with(*count, move |value| {
            let timeout = Timeout::new(1000, move || {
                LocalStorage::set("count", *value).ok();
                log!("Saved count: {}", value);
            });
            move || drop(timeout)  // Cancel if count changes
        });
    }

    html! {
        <button onclick={count.clone().callback(|c| c + 1)}>
            { format!("Count: {}", count) }
        </button>
    }
}
```

### Leptos Integration

```rust
use leptos::*;
use gloo_net::http::Request;
use gloo_storage::{Storage, LocalStorage};

#[component]
fn DataLoader() -> impl IntoView {
    let (data, set_data) = create_signal::<Option<String>>(None);
    let (loading, set_loading) = create_signal(false);

    let load_data = move || {
        set_loading.set(true);
        spawn_local(async move {
            let response = Request::get("/api/data")
                .send()
                .await;

            match response {
                Ok(r) => {
                    let text = r.text().await.unwrap_or_default();
                    set_data.set(Some(text));
                    LocalStorage::set("cached_data", &text).ok();
                }
                Err(_) => {
                    // Load from cache
                    let cached: Option<String> = LocalStorage::get("cached_data").ok();
                    set_data.set(cached);
                }
            }
            set_loading.set(false);
        });
    };

    view! {
        <button on:click=move |_| load_data()>
            {if loading() { "Loading..." } else { "Load Data" }}
        </button>
        <Show when=move || data.get().is_some()>
            <p>{data.get().unwrap_or_default()}</p>
        </Show>
    }
}
```

## Related Documents

- [wasm-bindgen](./wasm-bindgen-exploration.md) - Rust/JS interop
- [wasm-pack](./wasm-pack-exploration.md) - Build tooling

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/gloo/`
- Gloo Documentation: https://gloo.netlify.app/
- Gloo GitHub: https://github.com/rustwasm/gloo
