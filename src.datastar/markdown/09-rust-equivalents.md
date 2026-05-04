# Datastar Ecosystem -- Rust Equivalents

This document maps every Datastar concept to its Rust equivalent, showing both the direct translation and the production-grade patterns used in real Rust codebases.

## Signal System

### TypeScript → Rust

```typescript
// TypeScript: signal with closure-based value storage
function signal<T>(initial: T): Signal<T> {
    const node = createReactiveNode();
    let value = initial;
    return Object.assign(
        () => { trackDependency(node); return value; },
        (v: T) => { if (v !== value) { value = v; propagate(node); return true; } return false; }
    ) as Signal<T>;
}
```

```rust
// Rust: arena-allocated signals with explicit typing
pub struct Signal<T> {
    node: NodeIndex,
    value: T,
}

pub struct SignalStore {
    arena: Arena<ReactiveNode>,
    effects: Vec<Effect>,
}

impl<T: PartialEq + Clone> Signal<T> {
    pub fn get(&self, store: &SignalStore) -> T {
        store.track_dependency(self.node);
        self.value.clone()
    }

    pub fn set(&mut self, value: T, store: &mut SignalStore) -> bool {
        if self.value != value {
            self.value = value;
            store.propagate(self.node);
            true
        } else {
            false
        }
    }
}
```

**Production pattern:** Use `arc-swap` for lock-free signal reads and `parking_lot::RwLock` for writes. For WASM targets, use `Cell<T>` for single-threaded signals.

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

pub struct AtomicSignal<T> {
    value: ArcSwap<T>,
    node: NodeIndex,
}

impl<T: PartialEq> AtomicSignal<T> {
    pub fn load(&self, store: &SignalStore) -> Arc<T> {
        store.track_dependency(self.node);
        self.value.load_full()
    }

    pub fn store(&self, value: T, store: &mut SignalStore) -> bool {
        let old = self.value.load();
        if *old != value {
            self.value.store(Arc::new(value));
            store.propagate(self.node);
            true
        } else {
            false
        }
    }
}
```

## DOM Morphing

### TypeScript → Rust (WASM)

```typescript
// TypeScript: direct DOM manipulation
function morphChildren(oldParent: Element, newParent: Element) {
    const pantry = new Map();
    for (const child of oldParent.children) {
        if (child.id) pantry.set(child.id, child);
    }
    // ... morph logic
}
```

```rust
// Rust WASM: web-sys DOM bindings
use web_sys::Element;
use std::collections::HashMap;

fn morph_children(old_parent: &Element, new_parent: &Element) {
    let mut pantry: HashMap<String, Element> = HashMap::new();
    let children = old_parent.children();
    for i in 0..children.length() {
        if let Some(child) = children.item(i) {
            if let Ok(id) = child.id() {
                if !id.is_empty() {
                    pantry.insert(id, child);
                }
            }
        }
    }
    // ... morph logic using web-sys DOM methods
}
```

**Production pattern:** For non-WASM Rust, implement your own DOM tree representation:

```rust
pub struct Node {
    pub kind: NodeKind,
    pub id: Option<String>,
    pub attributes: HashMap<String, String>,
    pub children: Vec<Node>,
}

pub enum NodeKind {
    Element(String),  // tag name
    Text(String),
    Comment(String),
}
```

## SSE Streaming

### TypeScript → Rust (Axum)

```typescript
// TypeScript: fetch + manual SSE parsing
const response = await fetch(url, { body, headers });
const reader = response.body.getReader();
while (true) {
    const { done, value } = await reader.read();
    // Parse SSE lines
}
```

```rust
// Rust: Axum SSE stream
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;

async fn stream_events() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        yield Ok(Event::default()
            .event("datastar-patch-elements")
            .data("<div>Updated</div>"));
    };
    Sse::new(stream).keep_alive(KeepAlive::new()
        .interval(Duration::from_secs(15))
        .text("ping"))
}
```

**Production pattern:** Use `tokio-tungstenite` for WebSocket when bidirectional communication is needed. SSE is fine for server-to-client only.

## Event Store

### Rust (cross-stream) → Production Rust

The cross-stream store already uses production-grade patterns (fjall LSM-tree, cacache CAS). For higher throughput:

```rust
// Add batching for high-throughput scenarios
pub struct BatchedAppender<'a> {
    store: &'a EventStore,
    frames: Vec<Frame>,
}

impl<'a> BatchedAppender<'a> {
    pub fn new(store: &'a EventStore) -> Self {
        Self { store, frames: Vec::with_capacity(1000) }
    }

    pub fn push(&mut self, topic: &str, payload: &[u8]) -> Result<&Frame> {
        let frame = self.store.create_frame(topic, payload)?;
        self.frames.push(frame);
        if self.frames.len() >= 1000 {
            self.flush()?;
        }
        Ok(self.frames.last().unwrap())
    }

    pub fn flush(&mut self) -> Result<()> {
        let frames = std::mem::take(&mut self.frames);
        self.store.batch_append(&frames)?;
        Ok(())
    }
}
```

## Agent Loop

### Rust (yoagent) → Production Rust

The yoagent implementation is already production-grade. For higher reliability:

```rust
// Add circuit breaker for tool execution
pub struct CircuitBreaker {
    failures: AtomicU32,
    threshold: u32,
    cooldown: Duration,
    last_failure: AtomicU64,
}

impl CircuitBreaker {
    pub async fn execute<F, T>(&self, f: F) -> Result<T>
    where F: Future<Output = Result<T>>
    {
        if self.is_open() {
            return Err(Error::CircuitOpen);
        }
        match f.await {
            Ok(result) => { self.reset(); Ok(result) }
            Err(e) => { self.record_failure(); Err(e) }
        }
    }
}
```

## HTTP Server

### Nushell (http-nu) → Production Rust

Replace Nushell scripting with compiled routes for production:

```rust
// Axum with dynamic template rendering
async fn render_template(
    State(state): State<AppState>,
    Path(template_name): Path<String>,
    Json(context): Json<serde_json::Value>,
) -> Result<Html<String>, StatusCode> {
    let html = state.templates.render(&template_name, &context)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
```

**Production pattern:** Use `askama` for compile-time template checking instead of runtime minijinja:

```rust
use askama::Template;

#[derive(Template)]
#[template(path = "search.html")]
struct SearchTemplate {
    results: Vec<SearchResult>,
    query: String,
}
```

## Cross-Reference Table

| Datastar Concept | TypeScript | Rust Equivalent | Production Pattern |
|-----------------|-----------|-----------------|-------------------|
| Signal | Closure + ReactiveNode | `ArcSwap<T>` + arena index | Lock-free atomic signal |
| Effect | effect() callback | `Effect` in store, queued flush | Tokio task channel |
| DOM morph | web-sys direct | web-sys bindings | Custom DOM tree + diff |
| SSE client | fetch + manual parse | `reqwest` + line parser | `eventsource-client` crate |
| SSE server | N/A | `axum::response::sse` | `axum-extra::sse` |
| Event store | N/A | fjall + cacache | Add batched writes |
| Agent loop | N/A | yoagent | Add circuit breaker |
| HTTP handler | Nushell closure | Axum router | Askama templates |
| Plugin system | Dynamic attribute match | `HashMap<String, Box<dyn Plugin>>` | Plugin registry with WASM loading |

See [Signal System](02-reactive-signals.md) for the TypeScript implementation details.
See [Production Patterns](10-production-patterns.md) for broader production considerations.
