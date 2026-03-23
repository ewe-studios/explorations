# Leptos Framework Exploration

## Overview

Leptos is a full-stack, isomorphic Rust web framework that uses fine-grained reactivity to build declarative user interfaces. It supports three rendering modes:

1. **Client-Side Rendering (CSR)** - Pure browser rendering with WASM
2. **Server-Side Rendering (SSR)** - HTML generation on the server
3. **Hydration** - SSR HTML + client-side interactivity

### Key Design Philosophy

- **Fine-grained reactivity**: No virtual DOM - updates target specific DOM nodes directly
- **Isomorphic server functions**: Call server-only functions from client code seamlessly
- **Modular architecture**: Each layer can be used independently
- **Performance-first**: Compile-time optimizations, minimal runtime overhead

---

## Architecture Layers

Leptos is built as a series of concentric layers, each depending on the one below:

```
┌─────────────────────────────────────────────────────────────┐
│                    cargo-leptos (Build Tool)                │
├─────────────────────────────────────────────────────────────┤
│  leptos_router  │  leptos_meta  │  leptos (Control Flow)   │
├─────────────────────────────────────────────────────────────┤
│              leptos_dom / tachys (DOM Renderer)             │
├─────────────────────────────────────────────────────────────┤
│        leptos_macro (view! and component! macros)           │
├─────────────────────────────────────────────────────────────┤
│           server_fn (Isomorphic Server Functions)           │
├─────────────────────────────────────────────────────────────┤
│     reactive_graph (Signals, Memos, Effects - Core)         │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Crates

### reactive_graph (`/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/reactive_graph`)

The heart of Leptos - a fine-grained reactive system independent of the DOM.

**Key Concepts:**
- **Signals**: Atomic mutable state (`RwSignal`, `ReadSignal`, `WriteSignal`)
- **Computations**: Derived values (`Memo`, `ArcMemo`)
- **Effects**: Side effects that run when dependencies change (`Effect`, `RenderEffect`)
- **Owner**: Manages cleanup, context, and arena allocation

**Signal Types:**
```rust
// Arena-allocated (Copy, tied to Owner lifetime)
let (read, write) = signal(0);
let rw = RwSignal::new(0);

// Reference-counted (Clone, lives as long as reference)
let (read, write) = arc_signal(0);
let rw = ArcRwSignal::new(0);

// Local-only (!Send + !Sync, stored on local arena)
let (read, write) = signal_local(0);
```

### tachys (`/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/tachys`)

The DOM rendering layer - a statically-typed view system.

**Features:**
- Type-safe HTML element construction
- Support for SSR HTML generation with streaming
- Hydration system for client-side activation
- Islands architecture support

### server_fn (`/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/server_fn`)

Isomorphic server functions - call server-only code from the client.

**How it works:**
1. Annotate function with `#[server]`
2. Function body only compiles with `ssr` feature
3. Client gets auto-generated stub that makes HTTP requests
4. Multiple encoding formats: URL-encoded, JSON, CBOR, MessagePack, etc.

```rust
#[server]
async fn read_posts(how_many: usize, query: String) -> Result<Vec<Post>, ServerFnError> {
    // Server-only code here
    Ok(posts)
}

// Can be called from client code
let posts = read_posts(3, "search".into()).await?;
```

### leptos_macro (`/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/leptos_macro`)

Procedural macros for the `view!` and `#[component]` syntax.

**view! macro:**
```rust
view! {
    <div class="container">
        <h1>"Hello, " {name} "!"</h1>
        <button on:click=handler>"Click me"</button>
        {dynamic_content}
    </div>
}
```

**Optimizations:**
- Static HTML portions compiled to `&'static str` at compile time
- Different code paths for SSR vs CSR vs hydration
- Compile-time checking of HTML structure

### hydration_context (`/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/hydration_context`)

Manages data serialization from server to client during hydration.

**SharedContext provides:**
- Unique ID generation for serialized data
- Async data streaming support
- Error boundary state transfer
- Deferred stream handling

### cargo-leptos (`/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/cargo-leptos`)

Build tool that orchestrates:
- WASM compilation for browser target
- Server binary compilation
- SASS/Tailwind integration
- Hot-reload support
- Asset optimization

---

## Rendering Modes

### CSR (Client-Side Rendering)

```rust
// Compile with: cargo leptos watch --csr
pub fn main() {
    mount_to_body(|| view! { <App/> })
}
```

- Runs entirely in browser
- WASM bundle downloaded on first load
- Full interactivity immediately after WASM loads

### SSR (Server-Side Rendering)

```rust
// Compile with: cargo leptos watch --ssr
// Server renders HTML, sends to client
```

- Server renders full HTML on each request
- No JavaScript required for content
- New full-page navigations

### Hydration (SSR + Client Interactivity)

```rust
// Server: cargo leptos watch --ssr
// Client: cargo leptos watch --hydrate
```

1. Server renders HTML string
2. Client downloads HTML + WASM
3. WASM "hydrates" by attaching event listeners
4. Subsequent navigations are client-side

### Islands Architecture

```rust
#[island]
fn Counter() -> impl IntoView {
    // Only this component is interactive on client
}

// Server component
fn Page() -> impl IntoView {
    view! {
        <StaticContent/>
        <Counter/>  // Hydrated island
        <MoreStatic/>
    }
}
```

- Only interactive components are hydrated
- Reduces WASM payload significantly
- Static content remains HTML-only

---

## Performance Characteristics

### Compile-Time Optimizations

1. **Static String Optimization**: Static HTML portions compiled to single strings
2. **Template Macro**: Pre-compiles templates for zero-overhead CSR
3. **Feature Flags**: Dead code elimination based on render mode

### Runtime Optimizations

1. **No Virtual DOM**: Direct DOM updates via reactive primitives
2. **Lazy Memos**: Computations only run when read
3. **Automatic Dependency Tracking**: No manual dependency arrays
4. **Effect Batching**: Multiple signal updates batch into single re-render

### WASM Binary Size Optimizations

1. **LTO (Link-Time Optimization)**: Enabled by default in release
2. **Code Splitting**: Via `cargo-leptos` with WASM split support
3. **Islands**: Hydrate only interactive components
4. **Lazy Routes**: Load route components on demand

---

## Reactivity System Deep Dive

### The Reactive Graph

```
Signal ──► Memo ──► Effect
  │                     │
  └─────────────────────┘
```

**Node Types:**
1. **Source Nodes**: Signals that can be directly mutated
2. **Subscriber Nodes**: Memos/Effects that depend on sources
3. **Observer**: Currently running effect/computation

### Reactive Update Flow

```rust
// 1. Create signal
let count = RwSignal::new(0);

// 2. Create dependent memo
let double = Memo::new(move |_| count.get() * 2);

// 3. Create effect
Effect::new(move |_| {
    println!("double = {}", double.get());
});

// 4. Update triggers propagation
count.set(5);  // Memo updates, effect runs
```

### Effect Types

| Type | Runs When | Use Case |
|------|-----------|----------|
| `Effect` | Source changes | General side effects |
| `RenderEffect` | Source changes, immediately | DOM updates |
| `Effect::new_isomorphic` | Source changes, works in SSR/hydrate | Universal code |

### Owner System

```rust
let owner = Owner::new();
owner.with(|| {
    // All reactive nodes created here belong to this owner
    let signal = RwSignal::new(0);
    Effect::new(move |_| {
        println!("{}", signal.get());
    });
});

// When owner is dropped:
// - All child effects are cancelled
// - All arena-allocated signals are disposed
// - Cleanup functions run
```

---

## Hydration Mechanics

### Server-Side Flow

```
Request ──► Render HTML ──► Stream Response
              │
              ▼
         Collect Resources
              │
              ▼
         Serialize State ──► Inject <script> tags
```

### Client-Side Hydration

```
HTML Loaded ──► WASM Loaded ──► Hydrate
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
              Match Nodes    Read State     Restore Effects
              from HTML      from <script>  from server
```

### Hydration Algorithm

1. Create cursor at root element
2. Each component knows how to advance cursor
3. Match reactive primitives to serialized state
4. Attach event listeners without re-rendering

```rust
// Hydration cursor walks DOM
fn hydrate(&self, cursor: &Cursor) {
    // Match existing DOM nodes
    let text_node = cursor.current()
        .dyn_ref::<Text>()
        .expect("Expected text node");

    // Bind reactive updates
    create_render_effect(move |_| {
        text_node.set_text_content(&self.value.get().to_string());
    });

    // Advance cursor
    cursor.sibling();
}
```

---

## Server Functions and RPC

### Protocol Stack

```
┌──────────────────────────────────────────────┐
│           Your #[server] function            │
├──────────────────────────────────────────────┤
│                 Protocol                     │
│           (Http / Websocket)                 │
├──────────────────────────────────────────────┤
│                 Codec                        │
│    (Json / Url / Cbor / Msgpack / etc)      │
├──────────────────────────────────────────────┤
│                 Client                       │
│          (reqwest / gloo-net)                │
├──────────────────────────────────────────────┤
│                 Server                       │
│          (axum / actix-web)                  │
└──────────────────────────────────────────────┘
```

### Serialization Flow

1. **Client → Server**: `IntoReq` trait
2. **Server Processing**: `ServerFn::run_body`
3. **Server → Client**: `IntoRes` trait

### Built-in Protocols

- **HTTP**: REST-like POST/GET endpoints
- **WebSocket**: Real-time bidirectional communication

### Built-in Codecs

- **Url**: URL-encoded form data (default)
- **Json**: JSON request/response bodies
- **Cbor**: Binary CBOR encoding
- **MessagePack**: Compact binary format
- **Postcard**: no_std-friendly serialization
- **Rkyv**: Zero-copy deserialization

---

## WASM Integration Patterns

### Browser Environment Detection

```rust
#[cfg(target_arch = "wasm32")]
use web_sys::{window, Document};

#[cfg(not(target_arch = "wasm32"))]
// Server-side implementation
```

### Executor Abstraction

Leptos uses `any_spawner` for executor-async runtime agnosticism:

```rust
// Works with tokio, wasm-bindgen-futures, glib, etc.
Executor::spawn(async { /* task */ });
Executor::spawn_local(async { /* !Send task */});
```

### WASM Bindings

```rust
// In reactive_graph for wasm
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use web_sys::console;

// Console logging from WASM
console::warn_1(&"message".into());
```

---

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/
├── leptos/                      # Main framework repo
│   ├── reactive_graph/          # Core reactive system
│   ├── tachys/                  # DOM renderer
│   ├── server_fn/               # Server functions
│   ├── hydration_context/       # SSR/hydration state
│   ├── leptos_macro/            # Procedural macros
│   ├── leptos_dom/              # DOM helpers
│   ├── leptos_server/           # Server integration
│   ├── leptos_config/           # Configuration
│   ├── leptos_hot_reload/       # Hot reload support
│   ├── router/                  # Client-side router
│   ├── meta/                    # <head> management
│   ├── integrations/            # Actix, Axum adapters
│   ├── any_spawner/             # Executor abstraction
│   ├── examples/                # Example applications
│   └── projects/                # Real-world projects
└── cargo-leptos/                # Build tool
```

---

## Key Design Decisions

### Why Arena Allocation?

Signals use arena allocation (via `slotmap`) instead of `Rc<RefCell<T>>`:

**Pros:**
- `Copy` types - easy to move into closures
- No reference counting overhead
- Automatic cleanup via `Owner`
- Better cache locality

**Cons:**
- Signals tied to reactive scope lifetime
- Need `ArcSignal` for `'static` data

### Why No Virtual DOM?

Virtual DOM requires:
1. Re-rendering entire components
2. Diffing old vs new tree
3. Patching changes

Leptos approach:
1. Components run once
2. Create actual DOM nodes
3. Effects update specific nodes

**Result:** O(1) updates instead of O(n) diffing

### Why Server Functions?

Traditional approach:
```
Client ──► REST API ◄──► Database
           (separate)
```

Server functions:
```
Client ──► #[server] fn() ◄──► Database
         (co-located)
```

Benefits:
- Single source of truth
- Type-safe end-to-end
- No API versioning needed

---

## Comparison with Other Frameworks

| Framework | Reactivity | DOM Strategy | SSR |
|-----------|------------|--------------|-----|
| Leptos | Fine-grained signals | Direct DOM | ✅ Full streaming |
| Yew | Component re-render | Virtual DOM | ❌ Limited |
| Dioxus | Fine-grained | Virtual DOM | ✅ Basic |
| Sycamore | Fine-grained | Direct DOM | ✅ Basic |

---

## Resources

- [Source Repository](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos)
- [Architecture Docs](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/ARCHITECTURE.md)
- [Examples](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/examples)
