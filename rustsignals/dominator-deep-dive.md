---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RustSignals/rust-dominator
explored_at: 2026-03-23
---

# Dominator - Zero-Cost DOM Library Deep Dive

## Overview

Dominator is a zero-cost, ultra-high-performance declarative DOM library for Rust using FRP signals. It does NOT use a Virtual DOM - instead it uses raw DOM operations with almost no overhead.

**Key Claims:**
- As fast as Inferno (one of the fastest VDOM libraries)
- Updates are O(1) regardless of tree depth
- Everything inlined to raw DOM operations
- Scales to large applications

**Crate Info:**
- **Version:** 0.5.38
- **License:** MIT
- **Categories:** gui, web-programming, wasm

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Dominator API Layer                      │
│  (Declarative component definitions with signals)            │
├─────────────────────────────────────────────────────────────┤
│                    Signal Integration Layer                  │
│  (futures-signals integration, automatic updates)            │
├─────────────────────────────────────────────────────────────┤
│                   Raw DOM Bindings Layer                     │
│  (web-sys wrappers, direct DOM manipulation)                 │
└─────────────────────────────────────────────────────────────┘
```

## Core Abstractions

### Dom (The DOM Building Block)

```rust
pub trait Dom: Sized + 'static {
    type Message;
    type Hooks: DomHooks;

    fn into_nodes(self, document: &Document, parent: &Node) -> DomBuilder<'_, Self::Message, Self::Hooks>;
}
```

**DomBuilder:**
```rust
pub struct DomBuilder<'a, Message, Hooks: DomHooks> {
    parent: &'a Node,
    // ... internal state
}
```

### Component Pattern

```rust
pub struct Component<State, Msg> {
    state: State,
    // ...
}

impl<State, Msg> Dom for Component<State, Msg>
where
    State: /* bounds */,
{
    type Message = Msg;
    type Hooks = /* ... */;

    fn into_nodes(self, document: &Document, parent: &Node) -> DomBuilder<'_, Self::Message, Self::Hooks> {
        // Build DOM tree
    }
}
```

## Signal Integration

### signal_vec for Lists

```rust
use dominator::{html, events::EventExt};
use futures_signals::signal_vec::{MutableVec, SignalVecExt, VecDiff};

let items = MutableVec::new_with_values(vec!["Hello", "World"]);

html!("ul", {
    .children_signal_vec(items.signal_vec().map(|item| {
        html!("li", {
            .text(&item)
        })
    }))
})
```

**How it works:**
1. `MutableVec` tracks individual changes
2. `signal_vec()` creates SignalVec
3. `map()` transforms each item to Dom
4. Dominator applies only the changed nodes

### signal for Attributes/Text

```rust
let count = Mutable::new(0);

html!("div", {
    .text_signal(count.signal().map(|c| format!("Count: {}", c)))
    .attribute_signal("data-count", count.signal().map(|c| c.to_string()))
})
```

## DOM Operations

### Element Creation

```rust
// Low-level API
pub fn html<TagId>(tag: TagId, callback: impl FnOnce(DomBuilder<...>) -> DomBuilder<...>) -> Dom
where
    TagId: Into<&'static str>,
{
    // Creates element via web-sys
    let element = document.create_element(tag)?;
    // Applies all configured properties
    // Attaches to parent
}
```

### Property Bindings

```rust
// Direct property setting
pub fn text(self, text: &str) -> Self {
    self.node.set_text_content(Some(text));
    self
}

// Signal-based updates
pub fn text_signal<S>(self, signal: S) -> Self
where
    S: Signal<Item = String> + 'static,
{
    // Spawns future that updates text on signal change
    wasm_bindgen_futures::spawn_local(async move {
        signal.for_each(|text| {
            node.set_text_content(Some(&text));
            async {}
        }).await
    });
    self
}
```

## Performance Optimizations

### No VDOM Diffing

Unlike React/Vue:
- **No virtual tree creation**
- **No diff algorithm**
- **Direct DOM mutation via signals**

When a signal changes:
1. Only the affected DOM node is updated
2. No parent reconciliation needed
3. O(1) update regardless of tree size

### Stack Allocation

Most operations are stack-allocated:
- Signal transformations use `pin_project`
- No heap allocation for typical component trees
- DOM nodes are the only heap objects

### Memory Efficiency

```rust
// Traditional VDOM approach (heap heavy)
struct VNode {
    tag: String,           // heap
    props: HashMap,        // heap
    children: Vec<VNode>,  // heap
}

// Dominator approach
struct Dom {
    // Minimal state, mostly builder pattern
    // DOM nodes managed by browser
}
```

## Event Handling

### EventExt Trait

```rust
pub trait EventExt: EventTarget {
    fn event<F, E, B>(self, event_name: &'static str, callback: F) -> Self
    where
        F: FnMut(E) -> B + 'static,
        E: EventExt + AsRef<web_sys::Event>,
        B: Future<Output = ()>,
    {
        // Uses web-sys add_event_listener
        // Converts to Rust Future
    }
}
```

### Usage

```rust
html!("button", {
    .text("Click me!")
    .event(move |event: web_sys::MouseEvent| {
        wasm_bindgen_futures::spawn_local(async move {
            // Handle click
        });
        async {}
    })
})
```

## WASM Integration

### Dependencies

```toml
[dependencies]
wasm-bindgen = "0.2.48"
js-sys = "0.3.22"
wasm-bindgen-futures = "0.4.9"
web-sys = { version = "0.3.70", features = [...] }
gloo-events = "0.1.2"
futures-signals = "0.3.5"
```

### web-sys Features

Extensive feature list for full DOM access:
- `Document`, `Element`, `Node`
- `Event`, `MouseEvent`, `KeyboardEvent`
- `CssStyleDeclaration`, `CssStyleSheet`
- `ShadowRoot`, `HtmlElement`, etc.

## Component Examples

### Counter Component

```rust
fn counter() -> impl Dom {
    let count = Mutable::new(0);

    html!("div", {
        .children(&[
            html!("button", {
                .text("-")
                .event(move |_: web_sys::MouseEvent| {
                    *count.lock_mut() -= 1;
                    async {}
                })
            }),
            html!("span", {
                .text_signal(count.signal().map(|c| c.to_string()))
            }),
            html!("button", {
                .text("+")
                .event(move |_: web_sys::MouseEvent| {
                    *count.lock_mut() += 1;
                    async {}
                })
            }),
        ])
    })
}
```

### Dynamic List

```rust
fn todo_list(items: Rc<MutableVec<String>>) -> impl Dom {
    html!("ul", {
        .children_signal_vec(
            items.signal_vec()
                .map(|item| {
                    html!("li", {
                        .text(&item)
                    })
                })
        )
    })
}
```

## Routing

Dominator includes a routing module:

```rust
use dominator::routing;

routing::go_to_url("/path");
routing::route(|route: String| {
    // Handle route changes
});
```

## Animation

```rust
use dominator::animation;

animation::with_options(
    AnimationOptions {
        duration: Some(1000.0),
        easing: Some(Easing::EaseInOut),
        ..Default::default()
    },
    |progress| {
        // Update DOM based on progress
    },
);
```

## Fragment Support

```rust
use dominator::{Fragment, fragment};

fn multiple_elements() -> impl Dom {
    fragment! {
        html!("div", { .text("First") }),
        html!("div", { .text("Second") }),
    }
}
```

## Shadow DOM

```rust
html!("my-component", {
    .shadow_dom_mode(Some(ShadowRootMode::Open))
    .shadow_dom_children(&[
        html!("slot", { }),
    ])
})
```

## Comparison with Other Approaches

| Feature | Dominator | Yew | Leptos | Sycamore |
|---------|-----------|-----|--------|----------|
| VDOM | No | Yes | No (fine-grained) | No (fine-grained) |
| Signal-based | Yes | No | Yes | Yes |
| Raw DOM | Yes | No | Yes | Yes |
| WASM-first | Yes | Yes | Yes | Yes |
| futures-signals | Yes | No | No | Own signals |

## Best Practices

### 1. Use SignalVec for Lists

```rust
// GOOD - O(1) updates
.children_signal_vec(items.signal_vec().map(...))

// BAD - O(n) re-render
.children_signal(items.signal().map(|items| {
    items.iter().map(|item| html!("li", {...})).collect()
}))
```

### 2. Minimize signal_cloned

```rust
// Prefer signal() for Copy types
count.signal()

// Use signal_ref for transforms
data.signal_ref(|d| d.computed_value())

// Only use signal_cloned when necessary
complex_object.signal_cloned()
```

### 3. Event Handler Optimization

```rust
// Capture only needed data
let count = count.clone();
.event(move |_: MouseEvent| {
    *count.lock_mut() += 1;
    async {}
})
```

## Production Usage

The library has been tested on multiple large applications:
- State synchronization with server
- Complex nested components
- High-frequency updates

## Limitations

1. **WASM-only:** Designed specifically for web/WASM
2. **Learning curve:** Signal-based reactivity differs from VDOM
3. **Ecosystem:** Smaller community than Yew/Leptos
