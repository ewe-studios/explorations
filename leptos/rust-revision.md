# Rust Revision Guide: Building a Leptos-like Framework

## Overview

This guide explains how to reproduce Leptos's functionality in Rust at a production level. It covers the core architectural patterns, design decisions, and implementation strategies.

---

## 1. Project Structure

Create a workspace with modular crates:

```
my-framework/
├── Cargo.toml                 # Workspace root
├── reactive-core/             # Reactivity system
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── signal.rs
│       ├── effect.rs
│       ├── memo.rs
│       └── owner.rs
├── renderer/                  # DOM/View rendering
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── dom.rs
│       ├── ssr.rs
│       └── hydration.rs
├── macros/                    # Procedural macros
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── component.rs
│       └── view.rs
├── server-fns/                # Isomorphic server functions
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── codec.rs
│       └── protocol.rs
└── framework/                 # Main user-facing crate
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        └── control_flow.rs
```

### Root Cargo.toml

```toml
[workspace]
resolver = "2"
members = [
    "reactive-core",
    "renderer",
    "macros",
    "server-fns",
    "framework",
]

[workspace.dependencies]
# Internal crates
reactive-core = { path = "./reactive-core" }
renderer = { path = "./renderer" }

# Shared dependencies
slotmap = "1.0"
futures = "0.3"
thiserror = "2.0"
syn = "2.0"
quote = "1.0"
proc-macro2 = "1.0"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["rt", "macros"] }

[profile.release]
codegen-units = 1
lto = true
opt-level = 'z'
```

---

## 2. Reactive Core Implementation

### 2.1 Signal Types

```rust
// reactive-core/src/signal.rs

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock},
};

use slotmap::{new_key_type, SlotMap};

use crate::{
    effect::{EffectInner, Subscriber},
    owner::Owner,
};

new_key_type! { pub struct SignalKey; }

/// Arena-allocated reactive signal
pub struct Signal<T> {
    key: SignalKey,
    _marker: std::marker::PhantomData<T>,
}

impl<T: 'static> Signal<T> {
    pub fn new(value: T) -> (ReadSignal<T>, WriteSignal<T>)
    where
        T: Send + Sync,
    {
        let inner = ArcSignalInner {
            value: RwLock::new(value),
            subscribers: RwLock::new(Vec::new()),
        };

        let arc = Arc::new(inner);
        (
            ReadSignal { inner: Arc::clone(&arc) },
            WriteSignal { inner: arc },
        )
    }
}

/// Read handle for a signal
#[derive(Clone)]
pub struct ReadSignal<T> {
    inner: Arc<ArcSignalInner<T>>,
}

struct ArcSignalInner<T> {
    value: RwLock<T>,
    subscribers: RwLock<Vec<Arc<dyn Subscriber + Send + Sync>>>,
}

impl<T: Clone + Send + Sync + 'static> ReadSignal<T> {
    /// Track this signal in the current reactive scope
    pub fn get(&self) -> T {
        // Register with current effect if exists
        if let Some(observer) = Observer::current() {
            self.add_subscriber(observer);
        }

        // Return cloned value
        self.inner.value.read().unwrap().clone()
    }

    fn add_subscriber(&self, subscriber: Arc<dyn Subscriber + Send + Sync>) {
        self.inner.subscribers.write().unwrap().push(subscriber);
    }
}

/// Write handle for a signal
pub struct WriteSignal<T> {
    inner: Arc<ArcSignalInner<T>>,
}

impl<T: Send + Sync + 'static> WriteSignal<T> {
    pub fn set(&self, value: T) {
        // Update value
        *self.inner.value.write().unwrap() = value;

        // Notify all subscribers
        let subscribers = self.inner.subscribers.read().unwrap().clone();
        for sub in subscribers {
            sub.mark_dirty();
        }
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut lock = self.inner.value.write().unwrap();
        f(&mut *lock);
        drop(lock);

        // Notify subscribers
        let subscribers = self.inner.subscribers.read().unwrap().clone();
        for sub in subscribers {
            sub.mark_dirty();
        }
    }
}
```

### 2.2 Effect System

```rust
// reactive-core/src/effect.rs

use std::{
    cell::RefCell,
    sync::{Arc, RwLock},
};

use crate::signal::AnySource;

thread_local! {
    /// Current active observer (effect or memo)
    static OBSERVER: RefCell<Option<Arc<dyn Subscriber>>> = RefCell::new(None);
}

pub trait Subscriber: Send + Sync {
    fn add_source(&self, source: AnySource);
    fn mark_dirty(&self);
}

pub struct Effect {
    inner: Arc<RwLock<EffectInner>>,
}

struct EffectInner {
    callback: Box<dyn Fn() + Send + Sync>,
    sources: Vec<AnySource>,
    dirty: bool,
}

impl Effect {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        let inner = Arc::new(RwLock::new(EffectInner {
            callback: Box::new(callback),
            sources: Vec::new(),
            dirty: true,
        }));

        // Schedule first run
        Self::schedule_run(Arc::clone(&inner));

        Self { inner }
    }

    fn schedule_run(inner: Arc<RwLock<EffectInner>>) {
        // Use async executor (tokio, wasm-bindgen-futures, etc.)
        spawn(async move {
            // Clear previous sources
            {
                let mut lock = inner.write().unwrap();
                for source in lock.sources.drain(..) {
                    source.remove_subscriber();
                }
            }

            // Set as current observer
            OBSERVER.with(|obs| {
                *obs.borrow_mut() = Some(Arc::clone(&inner) as Arc<dyn Subscriber>);
            });

            // Run callback
            {
                let lock = inner.read().unwrap();
                (lock.callback)();
            }

            // Clear observer
            OBSERVER.with(|obs| *obs.borrow_mut() = None);

            // Mark as clean
            inner.write().unwrap().dirty = false;
        });
    }
}

impl Subscriber for EffectInner {
    fn add_source(&self, source: AnySource) {
        // Cast to mutable to add to sources
        // In real impl, use unsafe or interior mutability
    }

    fn mark_dirty(&self) {
        self.dirty = true;
        Self::schedule_run(Arc::clone(
            &Arc::new(RwLock::new(self.clone()))
        ));
    }
}
```

### 2.3 Memo Implementation

```rust
// reactive-core/src/memo.rs

use std::sync::{Arc, RwLock};

use crate::{effect::Subscriber, signal::AnySource, owner::Observer};

pub struct Memo<T> {
    inner: Arc<RwLock<MemoInner<T>>>,
}

struct MemoInner<T> {
    value: Option<T>,
    dirty: bool,
    compute: Box<dyn Fn(Option<&T>) -> T + Send + Sync>,
    sources: Vec<AnySource>,
    subscribers: Vec<Arc<dyn Subscriber + Send + Sync>>,
}

impl<T: Clone + Send + Sync + 'static> Memo<T> {
    pub fn new<F>(compute: F) -> Self
    where
        F: Fn(Option<&T>) -> T + Send + Sync + 'static,
    {
        let inner = Arc::new(RwLock::new(MemoInner {
            value: None,
            dirty: true,
            compute: Box::new(compute),
            sources: Vec::new(),
            subscribers: Vec::new(),
        }));

        Self { inner }
    }

    pub fn get(&self) -> T {
        // Register current observer as subscriber
        if let Some(observer) = Observer::current() {
            self.add_subscriber(observer);
        }

        // Check if recomputation needed
        self.ensure_computed();

        // Return cached value
        self.inner.read().unwrap().value.clone().unwrap()
    }

    fn ensure_computed(&self) {
        let mut inner = self.inner.write().unwrap();

        if inner.dirty {
            // Clear old sources
            for source in inner.sources.drain(..) {
                source.remove_subscriber();
            }

            // Set self as current observer for dependency tracking
            Observer::set_current(Arc::clone(&self.inner) as Arc<dyn Subscriber>);

            // Get old value for computation
            let old_value = inner.value.as_ref();

            // Run computation
            let new_value = (inner.compute)(old_value);

            // Clear observer
            Observer::clear_current();

            // Only update if value changed
            if let Some(ref old) = inner.value {
                if new_value != *old {
                    inner.value = Some(new_value);
                    inner.dirty = false;

                    // Mark subscribers dirty
                    let subscribers = inner.subscribers.clone();
                    for sub in subscribers {
                        sub.mark_dirty();
                    }
                }
            } else {
                inner.value = Some(new_value);
                inner.dirty = false;
            }
        }
    }

    fn add_subscriber(&self, subscriber: Arc<dyn Subscriber + Send + Sync>) {
        self.inner.write().unwrap().subscribers.push(subscriber);
    }
}

impl<T> Subscriber for MemoInner<T> {
    fn add_source(&self, source: AnySource) {
        self.sources.push(source);
    }

    fn mark_dirty(&self) {
        self.dirty = true;
        // Don't recompute yet - wait for get() call (lazy evaluation)
    }
}
```

### 2.4 Owner System

```rust
// reactive-core/src/owner.rs

use std::{
    cell::RefCell,
    sync::{Arc, Weak, RwLock},
};

thread_local! {
    static CURRENT_OWNER: RefCell<Option<Weak<RwLock<OwnerInner>>>> = RefCell::new(None);
}

pub struct Owner {
    inner: Arc<RwLock<OwnerInner>>,
}

struct OwnerInner {
    parent: Option<Weak<RwLock<OwnerInner>>>,
    children: Vec<Weak<RwLock<OwnerInner>>>,
    cleanups: Vec<Box<dyn FnOnce()>>,
    disposed: bool,
}

impl Owner {
    pub fn new() -> Self {
        let inner = Arc::new(RwLock::new(OwnerInner {
            parent: None,
            children: Vec::new(),
            cleanups: Vec::new(),
            disposed: false,
        }));

        Self { inner }
    }

    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Set as current owner
        CURRENT_OWNER.with(|current| {
            *current.borrow_mut() = Some(Arc::downgrade(&self.inner));
        });

        let result = f();

        // Clear current owner
        CURRENT_OWNER.with(|current| *current.borrow_mut() = None);

        result
    }

    pub fn on_cleanup<F>(f: F)
    where
        F: FnOnce() + 'static,
    {
        CURRENT_OWNER.with(|current| {
            if let Some(owner) = current.borrow().as_ref().and_then(|w| w.upgrade()) {
                owner.write().unwrap().cleanups.push(Box::new(f));
            }
        });
    }

    pub fn dispose(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.disposed = true;

        // Run cleanups
        for cleanup in inner.cleanups.drain(..) {
            cleanup();
        }

        // Dispose children
        for child in inner.children.drain(..) {
            if let Some(child) = child.upgrade() {
                Owner { inner: child }.dispose();
            }
        }
    }
}

/// Get current observer (effect/memo being evaluated)
pub struct Observer;

impl Observer {
    pub fn current() -> Option<Arc<dyn Subscriber>> {
        // In real impl, use thread-local storage
        None
    }

    pub fn set_current(observer: Arc<dyn Subscriber>) {
        // Set in thread-local storage
    }

    pub fn clear_current() {
        // Clear thread-local storage
    }
}
```

---

## 3. View System

### 3.1 IntoView Trait

```rust
// renderer/src/lib.rs

use std::rc::Rc;
use web_sys::Node;

pub trait IntoView {
    fn into_view(self) -> VNode;
}

pub enum VNode {
    Element(VElement),
    Text(VText),
    Fragment(Vec<VNode>),
    Dynamic(Box<dyn Fn() -> VNode>),
}

pub struct VElement {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<VNode>,
}

pub struct VText {
    pub text: String,
}

impl IntoView for String {
    fn into_view(self) -> VNode {
        VNode::Text(VText { text: self })
    }
}

impl IntoView for &str {
    fn into_view(self) -> VNode {
        VNode::Text(VText { text: self.to_string() })
    }
}

impl IntoView for i32 {
    fn into_view(self) -> VNode {
        VNode::Text(VText { text: self.to_string() })
    }
}
```

### 3.2 DOM Rendering

```rust
// renderer/src/dom.rs

use web_sys::{Document, Element, Node, Window, window};

pub fn create_element(tag: &str) -> Element {
    let document = Document::from(window().unwrap().document().unwrap());
    document.create_element(tag).unwrap()
}

pub fn create_text_node(text: &str) -> Node {
    let document = Document::from(window().unwrap().document().unwrap());
    document.create_text_node(text).into()
}

pub fn mount(node: &VNode, parent: &Node) {
    match node {
        VNode::Element(el) => {
            let element = create_element(&el.tag);

            // Set attributes
            for (key, value) in &el.attrs {
                element.set_attribute(key, value).unwrap();
            }

            // Mount children
            for child in &el.children {
                mount(child, &element.into());
            }

            parent.append_child(&element.into()).unwrap();
        }
        VNode::Text(text) => {
            let text_node = create_text_node(&text.text);
            parent.append_child(&text_node).unwrap();
        }
        VNode::Fragment(children) => {
            for child in children {
                mount(child, parent);
            }
        }
        VNode::Dynamic(render_fn) => {
            mount(&render_fn(), parent);
        }
    }
}

pub fn mount_to_body(view: impl Fn() -> VNode + 'static) {
    let document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // Create effect to re-render when signals change
    // In real impl, use reactive system
    mount(&view(), &body);
}
```

---

## 4. Procedural Macros

### 4.1 Component Macro

```rust
// macros/src/component.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, PatType};

pub fn component_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_body = &input_fn.block;

    // Extract props from function parameters
    let props: Vec<_> = input_fn
        .sig
        .inputs
        .iter()
        .filter_map(|param| {
            if let syn::FnArg::Typed(PatType { pat, ty, .. }) = param {
                Some((pat.clone(), ty.clone()))
            } else {
                None
            }
        })
        .collect();

    // Generate props struct
    let props_struct_name = format_ident!("{}_Props", fn_name);

    let output = quote! {
        #[derive(Clone)]
        struct #props_struct_name {
            #(#props),*
        }

        fn #fn_name(props: #props_struct_name) -> impl IntoView {
            #fn_body
        }
    };

    output.into()
}
```

### 4.2 View Macro

```rust
// macros/src/view.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Expr};

enum ViewNode {
    Element {
        tag: String,
        attrs: Vec<(String, Expr)>,
        children: Vec<ViewNode>,
    },
    Text(Expr),
    Interpolation(Expr),
}

pub fn view_impl(input: TokenStream) -> TokenStream {
    let input_expr = parse_macro_input!(input as Expr);

    // Parse JSX-like syntax into ViewNode tree
    // (This is simplified - real impl needs full parser)

    let output = quote! {
        {
            // Generate DOM building code
            let __el = create_element("div");
            // ... set attributes, mount children
            __el.into_view()
        }
    };

    output.into()
}
```

---

## 5. Server Functions

### 5.1 Server Function Trait

```rust
// server-fns/src/lib.rs

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerFnError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Server error: {0}")]
    Server(String),
}

pub trait ServerFn: Sized {
    type Output: Serialize + for<'de> Deserialize<'de>;

    fn url() -> &'static str;
    fn method() -> &'static str;

    fn run_body(self) -> impl Future<Output = Result<Self::Output, ServerFnError>>;
}

pub trait Client {
    fn send_request<S: ServerFn>(
        func: S,
        data: Vec<u8>,
    ) -> impl Future<Output = Result<Vec<u8>, ServerFnError>>;
}
```

### 5.2 Server Function Macro

```rust
// server-fns/src/macro.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn server_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_body = &input_fn.block;

    // Extract function parameters
    let params = &input_fn.sig.inputs;

    // Generate server function struct and impl
    let output = quote! {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct #fn_name {
            // Fields from function parameters
        }

        impl ServerFn for #fn_name {
            type Output = /* return type */;

            fn url() -> &'static str {
                concat!("/", stringify!(#fn_name))
            }

            fn method() -> &'static str {
                "POST"
            }

            #[cfg(feature = "ssr")]
            fn run_body(self) -> impl Future<Output = Result<Self::Output, ServerFnError>> {
                // Server-side implementation
                async move {
                    #fn_body
                }
            }

            #[cfg(not(feature = "ssr"))]
            fn run_body(self) -> impl Future<Output = Result<Self::Output, ServerFnError>> {
                // Client stub - makes HTTP request
                async move {
                    // Serialize arguments
                    let data = serde_json::to_vec(&self)?;

                    // Send request
                    let response = Client::send_request(self, data).await?;

                    // Deserialize response
                    Ok(serde_json::from_slice(&response)?)
                }
            }
        }

        // Helper function for easy calling
        async fn #fn_name(/* params */) -> Result</* Output */, ServerFnError> {
            #fn_name { /* fields */ }.run_body().await
        }
    };

    output.into()
}
```

---

## 6. Hydration System

### 6.1 Shared Context

```rust
// renderer/src/hydration.rs

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SerializedDataId(usize);

pub trait SharedContext: Send + Sync {
    fn is_browser(&self) -> bool;
    fn next_id(&self) -> SerializedDataId;
    fn write_async(&self, id: SerializedDataId, fut: Pin<Box<dyn Future<Output = String> + Send>>);
    fn read_data(&self, id: &SerializedDataId) -> Option<String>;
    fn during_hydration(&self) -> bool;
    fn hydration_complete(&self);
}

/// Server-side context
pub struct SsrContext {
    next_id: AtomicUsize,
    data: Arc<RwLock<HashMap<SerializedDataId, String>>>,
}

impl SharedContext for SsrContext {
    fn is_browser(&self) -> bool {
        false
    }

    fn next_id(&self) -> SerializedDataId {
        SerializedDataId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    fn write_async(&self, id: SerializedDataId, fut: Pin<Box<dyn Future<Output = String> + Send>>) {
        // Store future result for serialization
    }

    fn read_data(&self, _id: &SerializedDataId) -> Option<String> {
        None  // Server doesn't read data
    }

    fn during_hydration(&self) -> bool {
        false
    }

    fn hydration_complete(&self) {}
}

/// Client-side (hydration) context
pub struct HydrateContext {
    data: Arc<RwLock<HashMap<SerializedDataId, String>>>,
    is_hydrating: AtomicBool,
}

impl HydrateContext {
    pub fn new(initial_data: HashMap<SerializedDataId, String>) -> Self {
        Self {
            data: Arc::new(RwLock::new(initial_data)),
            is_hydrating: AtomicBool::new(true),
        }
    }
}

impl SharedContext for HydrateContext {
    fn is_browser(&self) -> bool {
        true
    }

    fn next_id(&self) -> SerializedDataId {
        // In browser, IDs are pre-generated
        unimplemented!()
    }

    fn read_data(&self, id: &SerializedDataId) -> Option<String> {
        self.data.read().unwrap().get(id).cloned()
    }

    fn during_hydration(&self) -> bool {
        self.is_hydrating.load(Ordering::Relaxed)
    }

    fn hydration_complete(&self) {
        self.is_hydrating.store(false, Ordering::Relaxed);
    }
}
```

### 6.2 SSR HTML Generation

```rust
// renderer/src/ssr.rs

use futures::{Stream, StreamExt};
use std::pin::Pin;

pub trait RenderHtml {
    fn to_html(&self) -> String;
    fn to_html_stream(&self) -> Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl RenderHtml for VNode {
    fn to_html(&self) -> String {
        match self {
            VNode::Element(el) => {
                let mut html = String::new();
                html.push_str(&format!("<{}", el.tag));

                for (key, value) in &el.attrs {
                    html.push_str(&format!(" {}=\"{}\"", key, value));
                }

                html.push('>');

                for child in &el.children {
                    html.push_str(&child.to_html());
                }

                html.push_str(&format!("</{}>", el.tag));
                html
            }
            VNode::Text(text) => {
                // Escape HTML
                text.text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            }
            VNode::Fragment(children) => {
                children.iter().map(|c| c.to_html()).collect()
            }
            VNode::Dynamic(render_fn) => render_fn().to_html(),
        }
    }

    fn to_html_stream(&self) -> Pin<Box<dyn Stream<Item = String> + Send>> {
        // For streaming SSR
        Box::pin(futures::stream::once(async move {
            self.to_html()
        }))
    }
}
```

---

## 7. Integration Example

### 7.1 Putting It All Together

```rust
// framework/src/lib.rs

pub use reactive_core::{signal, Effect, Memo, Owner};
pub use renderer::{mount_to_body, view, IntoView, VNode};
pub use server_fns::server;

#[macro_export]
macro_rules! view {
    ($($tt:tt)*) => {
        $crate::macros::view_impl!($($tt)*)
    };
}

// Example counter component
pub fn simple_counter(initial_value: i32) -> impl IntoView {
    let (count, set_count) = signal(initial_value);

    let increment = move |_| {
        set_count.update(|c| *c += 1);
    };

    let decrement = move |_| {
        set_count.update(|c| *c -= 1);
    };

    view! {
        <div>
            <button on:click=decrement>"-1"</button>
            <span>{move || count.get().to_string()}</span>
            <button on:click=increment>"+1"</button>
        </div>
    }
}

pub fn main() {
    mount_to_body(|| simple_counter(0).into_view());
}
```

---

## 8. Performance Optimizations

### 8.1 Static String Optimization

```rust
// Compile static view portions to strings
const STATIC_TEMPLATE: &str = r#"<div class="container"><button>Click</button></div>"#;

// Rather than building element by element
```

### 8.2 Event Delegation

```rust
// Single event listener at document level
pub fn add_delegated_listener(event_type: &str) {
    let handler = Closure::wrap(Box::new(move |e: Event| {
        // Find target and dispatch
    }) as Box<dyn FnMut(_)>);

    document()
        .add_event_listener(event_type, handler.as_ref())
        .unwrap();
    handler.forget();
}
```

### 8.3 Memoization with Selectors

```rust
pub struct Selector<T> {
    value: T,
    subscribers: Vec<Arc<dyn Fn(&T, &T) -> bool>>,
}

impl<T: PartialEq> Selector<T> {
    pub fn new(initial: T) -> Self {
        Self {
            value: initial,
            subscribers: Vec::new(),
        }
    }

    pub fn update(&mut self, new_value: T) {
        if self.value != new_value {
            let old = &self.value;
            for should_notify in &self.subscribers {
                if should_notify(old, &new_value) {
                    // Notify this subscriber
                }
            }
            self.value = new_value;
        }
    }
}
```

---

## 9. Testing Strategy

### 9.1 Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_reactivity() {
        let owner = Owner::new();
        owner.set();

        let (count, set_count) = signal(0);
        let doubled = Memo::new(move |_| count.get() * 2);

        assert_eq!(doubled.get(), 0);
        set_count.set(5);
        assert_eq!(doubled.get(), 10);
    }
}
```

### 9.2 Integration Testing

```rust
#[cfg(test)]
mod integration {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_component_rendering() {
        let app = simple_counter(0);
        let view = app.into_view();
        // Assert DOM structure
    }
}
```

---

## 10. Key Takeaways

1. **Modular Architecture**: Split into focused crates (reactivity, rendering, macros)
2. **Arena Allocation**: Use `slotmap` for efficient signal storage
3. **Reactive Graph**: Automatic dependency tracking via thread-local observer
4. **Lazy Memos**: Only compute when read, only notify on change
5. **SSR Support**: Serialize reactive state, hydrate on client
6. **Macro Ergonomics**: `view!` and `#[component]` for DX
7. **Type Safety**: Use Rust's type system for compile-time guarantees

---

## Resources

- [Leptos Source](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos)
- [SolidJS](https://solidjs.com) (reactivity inspiration)
- [Sycamore](https://sycamore-rs.netlify.app) (similar approach)
- [Dioxus](https://dioxuslabs.com) (alternative Rust framework)
