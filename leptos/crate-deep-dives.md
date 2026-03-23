# Leptos Crate Deep Dives

This document provides detailed exploration of each major crate in the Leptos ecosystem.

---

## 1. reactive_graph

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/reactive_graph`

### Purpose

The core reactive system that powers Leptos. Provides signals, memos, and effects for fine-grained reactivity.

### Key Modules

```
src/
├── signal/           # Signal types (RwSignal, ReadSignal, WriteSignal)
├── computed/         # Memo, Resource, async derived values
├── effect/           # Effect, RenderEffect, EffectFunction
├── owner/            # Ownership, cleanup, context, arena
├── graph/            # Reactive graph nodes (Source, Subscriber)
├── wrappers/         # Read/write wrappers for signals
└── traits.rs         # Core traits (Get, Set, Track, Notify, etc.)
```

### Dependencies

```toml
[dependencies]
any_spawner = "0.3.0"        # Executor abstraction
slotmap = "1.1"              # Arena allocation
futures = "0.3"              # Async utilities
rustc-hash = "2.1"           # Fast hash maps
send_wrapper = "0.6"         # Thread-safe wrappers
async-lock = "3.4"           # Async primitives
```

### Key Types

| Type | Purpose |
|------|---------|
| `RwSignal<T>` | Combined read/write signal |
| `ReadSignal<T>` | Read-only handle |
| `WriteSignal<T>` | Write-only handle |
| `ArcRwSignal<T>` | Reference-counted signal |
| `Memo<T>` | Cached computed value |
| `ArcMemo<T>` | Reference-counted memo |
| `Effect` | Side effect subscriber |
| `RenderEffect` | Immediate DOM effect |
| `Resource<T>` | Async derived value |
| `Owner` | Reactive scope manager |

### Design Patterns

**Trait-Based Composition:**
```rust
// Base traits
ReadUntracked + Track → Read → With + Get
Write + Notify → Update → Set
```

**Arena Storage:**
```rust
// Signals stored in slotmap arena
// Handle is Copy, refers to arena index
pub struct RwSignal<T, S = SyncStorage> {
    handle: SlotMapKey,
    _marker: PhantomData<T>,
}
```

---

## 2. tachys

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/tachys`

### Purpose

Statically-typed view system and DOM renderer. Handles CSR, SSR, and hydration.

### Key Modules

```
src/
├── html/             # HTML element types
├── view/             # View trait and implementations
├── renderer/         # Renderer trait (DOM, SSR)
├── ssr/              # Server-side rendering
├── hydration/        # Client-side hydration
├── reactive_graph/   # Reactive view bindings
└── svg/, mathml/     # Additional namespaces
```

### Dependencies

```toml
[dependencies]
web-sys = "0.3"             # WASM DOM bindings
wasm-bindgen = "0.2"        # JS interop
futures = "0.3"             # Streaming
html-escape = "0.2"         # SSR escaping
reactive_graph = { path }   # Reactivity
```

### View Trait

```rust
pub trait View: Sized {
    type State;
    type AsyncOutput;

    fn build(self, parent: &Node, marker: Option<Node>) -> Self::State;
    fn rebuild(&self, state: &mut Self::State);
    fn into_render(self, state: Self::State) -> Self::AsyncOutput;
}
```

### SSR Streaming

```rust
pub struct StreamBuilder {
    sync_buf: String,           // Synchronous buffer
    chunks: VecDeque<StreamChunk>,
    pending: Option<ChunkFuture>,
}

// Supports out-of-order streaming
// Fallbacks rendered while waiting for async data
```

---

## 3. server_fn

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/server_fn`

### Purpose

Isomorphic server functions - write server-only code that can be called from client.

### Key Modules

```
src/
├── client/           # Client-side HTTP client
├── server/           # Server-side handlers
├── codec/            # Encodings (Json, Url, Cbor, etc.)
├── middleware/       # Tower middleware support
└── error.rs          # Error types
```

### Dependencies

```toml
[dependencies]
serde = "1.0"               # Serialization
serde_json = "1.0"          # JSON encoding
serde_qs = "0.15"           # URL encoding
gloo-net = "0.6"            # Browser HTTP
reqwest = "0.13"            # Server HTTP
inventory = "0.3"           # Function registration
const_format = "0.2"        # Const string utils
```

### Protocol Stack

```
┌─────────────────────────────┐
│    Your #[server] fn        │
├─────────────────────────────┤
│  Protocol: Http/Websocket   │
├─────────────────────────────┤
│  Codec: Json/Url/Cbor/...   │
├─────────────────────────────┤
│  Client: reqwest/gloo-net   │
├─────────────────────────────┤
│  Server: axum/actix-web     │
└─────────────────────────────┘
```

### Server Function Macro

```rust
#[server(
    prefix = "/api",
    endpoint = "read_posts",
    input = Url,      // Encoding
    output = Json,    // Response encoding
    client = Reqwest, // Client to use
)]
async fn read_posts(query: String) -> Result<Vec<Post>, ServerFnError> {
    // Server-only code
}
```

### Built-in Codecs

| Codec | Description |
|-------|-------------|
| `Url` | URL-encoded form data (default) |
| `Json` | JSON request/response |
| `Cbor` | Binary CBOR encoding |
| `MsgPack` | MessagePack binary |
| `Postcard` | no_std-friendly |
| `Rkyv` | Zero-copy deserialization |

---

## 4. leptos_macro

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/leptos_macro`

### Purpose

Procedural macros for `#[component]`, `view!`, `#[server]`, and more.

### Key Modules

```
src/
├── component.rs      # #[component] macro
├── view/             # view! macro parser
├── lib.rs            # Macro exports
└── slice.rs, memo.rs # Helper macros
```

### Dependencies

```toml
[dependencies]
syn = "2.0"                   # Rust parsing
quote = "1.0"                 # Codegen
proc-macro2 = "1.0"           # Token handling
rstml = "0.12"                # JSX-like parsing
attribute-derive = "0.10"     # Attribute parsing
leptos_hot_reload = { path }  # Hot reload support
server_fn_macro = { path }    # Server fn macros
```

### Component Macro Output

```rust
// Input
#[component]
fn Counter(initial: i32) -> impl IntoView {
    view! { <div>{initial}</div> }
}

// Output (simplified)
struct CounterProps {
    initial: i32,
}

impl Props for CounterProps { /* builder */ }

fn Counter(__props: CounterProps) -> impl IntoView {
    let initial = __props.initial;
    view! { <div>{initial}</div> }
}
```

### View Macro Optimization

```rust
// Static portions compiled to strings at compile time
view! {
    <div class="static">
        {dynamic}
    </div>
}

// Expands to:
Element::from_static(r#"<div class="static"></div>"#)
    .insert_dynamic(dynamic)
```

---

## 5. hydration_context

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/hydration_context`

### Purpose

Shared state between server and client for hydration.

### Key Types

```rust
pub trait SharedContext: Debug {
    fn is_browser(&self) -> bool;
    fn next_id(&self) -> SerializedDataId;
    fn write_async(&self, id: SerializedDataId, fut: PinnedFuture<String>);
    fn read_data(&self, id: &SerializedDataId) -> Option<String>;
    fn during_hydration(&self) -> bool;
    fn hydration_complete(&self);
    // ... error handling, streaming
}
```

### Hydration Flow

```
Server:                          Client:
  │                                │
  ├── Render HTML ─────────────►   │
  │                                │
  ├── Serialize Resources ─────►   │
  │     (in <script> tags)         │
  │                                │
  │                         Parse HTML
  │                                │
  │                         Load WASM
  │                                │
  │                         Read <script>
  │                                │
  │◄──── Hydrate State ────────────┤
  │                                │
  │◄──── Attach Listeners ─────────┤
  │                                │
                              Done
```

---

## 6. leptos_router

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/router`

### Purpose

Client-side routing with nested routes and reactive data loading.

### Key Features

- Nested routes (outlet pattern)
- Route params and query params
- Reactive route data
- Client-side navigation
- SSR route matching

### Dependencies

```toml
[dependencies]
leptos = { workspace }
url = "2.5"                    # URL parsing
percent-encoding = "2.3"       # Encoding
gloo-net = "0.6"               # Fetch for hydrate
```

### Route Definition

```rust
#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes>
                <Route path="" view=HomePage/>
                <Route path="posts/:id" view=PostPage/>
                <Route path="*" view=NotFound/>
            </Routes>
        </Router>
    }
}
```

---

## 7. any_spawner

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/any_spawner`

### Purpose

Executor-agnostic task spawning.

### Supported Executors

- `tokio` - Server async runtime
- `wasm-bindgen` - Browser event loop
- `glib` - GTK applications
- `futures-executor` - Generic executor

### API

```rust
// Spawn Send + 'static future
Executor::spawn(async { /* task */ });

// Spawn !Send future (browser)
Executor::spawn_local(async { /* task */ });

// Wait for next tick
Executor::tick().await;
```

### Implementation

```rust
static EXECUTOR_FNS: OnceLock<ExecutorFns> = OnceLock::new();

struct ExecutorFns {
    spawn: fn(PinnedFuture<()>),
    spawn_local: fn(PinnedLocalFuture<()>),
    poll_local: fn(),
}

// Set at runtime based on feature flags
Executor::init_tokio();
Executor::init_wasm_bindgen();
```

---

## 8. cargo-leptos

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/cargo-leptos`

### Purpose

Build tool for Leptos applications.

### Features

- WASM compilation
- Server binary build
- Asset optimization (CSS, JS)
- Hot reload
- Profile management
- Deployment helpers

### Dependencies

```toml
[dependencies]
tokio = "1.48"                 # Async runtime
axum = "0.8"                   # Dev server
lightningcss = "1.0"           # CSS processing
swc = "51.0"                   # JS processing
notify = "8.0"                 # File watching
cargo_metadata = "0.23"        # Cargo integration
```

### Commands

```bash
cargo leptos new              # Create new project
cargo leptos watch            # Development mode
cargo leptos build            # Production build
cargo leptos serve            # Serve production build
cargo leptos end-to-end       # E2E testing
```

---

## 9. Integration Crates

### leptos_integration_utils

Helpers for server integrations (Actix, Axum).

### leptos_actix

```rust
pub fn render_app_to_stream(
    path: String,
    app: impl Fn() -> AnyView + 'static,
) -> impl Future<Output = HttpResponse> {
    // Actix-web integration
}
```

### leptos_axum

```rust
pub fn render_app_to_stream(
    path: String,
    app: impl Fn() -> AnyView + 'static,
) -> impl Future<Output = Response<Body>> {
    // Axum integration
}
```

---

## 10. Utility Crates

| Crate | Purpose |
|-------|---------|
| `any_error` | Type-erased error handling |
| `either_of` | Multi-type Either (up to 16) |
| `oco` | Cheaply-cloned string wrapper |
| `or_poisoned` | RwLock poison handling |
| `const_str_slice_concat` | Const string utils |
| `next_tuple` | Iterator tuple helpers |

---

## Crate Dependency Graph

```
                    framework (leptos)
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
    router              meta           control_flow
        │                 │                 │
        └─────────────────┴─────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
   tachys        leptos_server      leptos_macro
        │                 │                 │
        │         ┌───────┘                 │
        │         │                         │
  hydration_ctx   server_fn                 │
        │         │                         │
        └─────────┼─────────────────────────┘
                  │
        ┌─────────┴─────────┐
        │                   │
 reactive_graph       any_spawner
        │
    owner, signal,
    effect, memo
```

---

## Resources

- Main source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos`
- Architecture docs: `ARCHITECTURE.md`
- Examples: `examples/` directory
