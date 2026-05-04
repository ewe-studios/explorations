# Datastar -- Rust Equivalents

Translating Datastar's TypeScript architecture to Rust requires mapping JavaScript concepts (Proxy, Function constructor, MutationObserver, DOM API) to Rust equivalents.

**Aha:** The signal system is the easiest part to translate — ReactiveNode is essentially a struct with version tracking and a propagation algorithm that maps cleanly to Rust. The hardest parts are the DOM morphing (requires a DOM abstraction) and the expression compiler (requires a JS runtime or a custom expression parser).

## Signal System → Rust

The ReactiveNode/Link architecture translates almost directly:

```rust
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone)]
struct Link {
    node: Option<Rc<ReactiveNode>>,
    source: Option<Rc<ReactiveNode>>,
    next_sub: Option<Link>,
    prev_sub: Option<Link>,
    next_rt: Option<Link>,
    prev_rt: Option<Link>,
}

struct ReactiveNode {
    links: Option<Vec<Link>>,
    flags: ReactiveFlags,
    version: u64,
    last_checked_version: u64,
    value: serde_json::Value,
    computed: Option<Box<dyn Fn() -> serde_json::Value>>,
    effect: Option<Box<dyn Fn()>>,
}

bitflags::bitflags! {
    struct ReactiveFlags: u8 {
        const Mutable = 1 << 0;
        const Watching = 1 << 1;
        const RecursedCheck = 1 << 2;
        const Recursed = 1 << 3;
        const Dirty = 1 << 4;
        const Pending = 1 << 5;
    }
}

fn signal<T: Into<serde_json::Value>>(value: T) -> Rc<ReactiveNode> {
    Rc::new(ReactiveNode {
        flags: ReactiveFlags::Mutable,
        value: value.into(),
        ..Default::default()
    })
}

fn get(node: &Rc<ReactiveNode>) -> serde_json::Value {
    // Create dependency link if in tracking scope
    // Return node.value.clone()
}

fn set(node: &Rc<ReactiveNode>, value: serde_json::Value) {
    node.value.set(value);
    propagate(node);
}
```

Key differences from TypeScript:

| TypeScript | Rust | Why |
|-----------|------|-----|
| `class ReactiveNode` | `struct ReactiveNode` + `Rc<RefCell<>>` | Shared ownership + interior mutability |
| `Link.unlink()` method | `fn unlink(link: &mut Link)` | Free function or trait impl |
| `Map<string, Signal>` | `HashMap<String, Rc<ReactiveNode>>` | Same semantics |
| Lazy `checkDirty()` | Same algorithm | Pure algorithm, no runtime difference |

## Expression Compiler → Rust

This is the hardest part. TypeScript's `new Function()` has no direct Rust equivalent. Options:

### Option A: Embed a JS runtime (quickjs-rs, deno_core)

```rust
use quickjs_rust::JSRuntime;

fn compile_expression(expr: &str) -> impl Fn(&SignalStore) -> serde_json::Value {
    let rewritten = rewrite_signal_refs(expr);  // $count → $['count']
    move |store: &SignalStore| {
        let mut ctx = runtime.create_context();
        ctx.set("$", store.to_js_value());
        ctx.eval(&rewritten).unwrap()
    }
}
```

**Pros:** Full JS semantics, easy to implement.
**Cons:** Adds ~2MB binary size, WASM-unfriendly, runtime overhead.

### Option B: Parse and evaluate a mini-expression language

```rust
enum Expr {
    SignalRef(String),
    Literal(serde_json::Value),
    BinaryOp(Box<Expr>, Operator, Box<Expr>),
    MemberAccess(Box<Expr>, String),
    Call(String, Vec<Expr>),
    Template(Vec<Expr>),
}

fn parse(expr: &str) -> Result<Expr, ParseError> {
    // Hand-written or tree-sitter parser
}

fn eval(expr: &Expr, store: &SignalStore) -> serde_json::Value {
    match expr {
        Expr::SignalRef(name) => store.get(name),
        Expr::BinaryOp(left, op, right) => {
            let l = eval(left, store);
            let r = eval(right, store);
            apply_op(&l, op, &r)
        }
        // ...
    }
}
```

**Pros:** No JS runtime, fast, WASM-compatible, safe.
**Cons:** Limited to a subset of JS expressions, needs a parser.

### Option C: Compile to WASM at build time

```rust
// Build step: parse expressions, compile to WASM bytecode
// Runtime: load and execute WASM bytecode
```

**Pros:** Maximum performance, sandboxed.
**Cons:** Complex build pipeline, requires WASM runtime.

## DOM Morphing → Rust (Web/WASM)

For WASM targets (web-sys):

```rust
use wasm_bindgen::JsCast;
use web_sys::{Element, Node, Document, DocumentFragment};

fn morph(old_elt: &Element, new_content: &DocumentFragment) {
    // Same algorithm as TypeScript, using web_sys DOM APIs
    let old_ids = collect_ids(old_elt);
    let persistent_ids = find_persistent_ids(old_elt, new_content);
    let id_map = build_id_map(old_elt, &persistent_ids);
    morph_children(old_elt, new_content, &id_map, &persistent_ids);
}
```

The morph algorithm itself is algorithmically identical — it just uses `web_sys` DOM APIs instead of browser-native `document.querySelectorAll`.

For headless Rust (no DOM):

```rust
// Represent DOM as a tree of nodes
struct HtmlNode {
    tag: String,
    attributes: HashMap<String, String>,
    children: Vec<HtmlNode>,
    id: Option<String>,
}

fn morph_tree(old: &mut HtmlNode, new: &HtmlNode) {
    // Same ID-set matching algorithm
    // But operates on in-memory tree, not browser DOM
}
```

## Plugin System → Rust

```rust
use std::collections::HashMap;

type ApplyFn = Box<dyn Fn(PluginContext) -> Box<dyn FnOnce()>>;

struct AttributePlugin {
    name: String,
    requirement: Requirement,
    apply: ApplyFn,
}

struct PluginRegistry {
    attribute_plugins: HashMap<String, AttributePlugin>,
    action_plugins: HashMap<String, ActionPlugin>,
    watcher_plugins: HashMap<String, WatcherPlugin>,
}
```

Plugins register at startup:

```rust
fn register_builtin_plugins(registry: &mut PluginRegistry) {
    registry.register_attribute(AttributePlugin {
        name: "bind".into(),
        requirement: Requirement::Exclusive,
        apply: Box::new(bind_apply),
    });
    // ...
}
```

## SSE Streaming → Rust

For client-side (WASM):

```rust
use web_sys::Response;
use wasm_streams::ReadableStream;

async fn fetch_event_source(url: &str) -> Result<impl Stream<Item = SseEvent>> {
    let resp = gloo_net::http::Request::get(url).send().await?;
    let stream = ReadableStream::from(resp.body().unwrap());
    parse_sse_stream(stream)
}
```

For server-side (native):

```rust
use axum::response::sse::{Sse, Event};
use futures::stream;

async fn sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    Sse::new(stream::iter(vec![
        Ok(Event::default().event("datastar-patch-elements").data("<div>Hello</div>")),
    ]))
}
```

## Batched Updates → Rust

```rust
struct Batch {
    pending: Vec<Rc<ReactiveNode>>,
    in_progress: bool,
}

fn begin_batch() { /* increment counter */ }

fn end_batch() {
    // Process all pending nodes in order
    while let Some(node) = batch.pending.pop() {
        if node.flags.contains(Dirty) {
            update_node(&node);
        }
    }
}
```

## Key Challenges

| Challenge | TypeScript Solution | Rust Challenge |
|-----------|-------------------|----------------|
| Shared mutable state | Closures capture by reference | `Rc<RefCell<>>` or `Arc<Mutex<>>` |
| Dynamic function compilation | `new Function()` | Requires parser or embedded JS runtime |
| DOM mutation | MutationObserver | `web_sys` for WASM, no equivalent for native |
| Event dispatch | CustomEvent on document | Event system needed (tokio::sync::broadcast?) |
| Garbage collection | Automatic | Manual cleanup via Drop trait |
| Async/await | Native | tokio or async-std runtime needed |

See [Production Patterns](12-production-patterns.md) for production-grade considerations.
See [Web Tooling](13-web-tooling.md) for the IDE integration story.
