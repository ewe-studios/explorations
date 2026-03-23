# SSR and Hydration Deep Dive

## Overview

Leptos supports multiple rendering modes, with Server-Side Rendering (SSR) and Hydration being key for production web applications. This document explains the internals.

---

## Rendering Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **CSR** | Client-Side Rendering | Dashboards, SPAs |
| **SSR** | Server-Side Rendering | SEO, initial load |
| **Hydration** | SSR + Client Interactivity | Full-stack apps |
| **Islands** | Partial Hydration | Content-heavy sites |

---

## SSR Architecture

### Server-Side Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      HTTP Request                            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Router Matching                          │
│              (Match path to route tree)                    │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                 Load Route Data                            │
│           (Resources, Server Functions)                     │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Render Components to HTML                       │
│     (Static portions optimized as &'static str)            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Collect Async Resources                         │
│           (Serialize to JSON for hydration)                 │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│           Stream HTML Response with <script> tags           │
│     (Contains serialized state for client hydration)        │
└─────────────────────────────────────────────────────────────┘
```

### HTML Stream Structure

```html
<!DOCTYPE html>
<html>
<head>
    <!-- Static head content -->
</head>
<body>
    <!-- Main HTML content -->
    <div id="app">
        <div class="container">
            <h1>Server Rendered</h1>
        </div>
    </div>

    <!-- Hydration scripts -->
    <script id="__LEPTOS_RESOURCE_DATA__" type="application/json">
        {"resource_0": {"data": [...]}, "resource_1": {...}}
    </script>

    <script id="__LEPTOS_CHUNKS__" type="application/json">
        {"chunks": [...]}
    </script>

    <!-- WASM loader -->
    <script type="module">
        import init from "/pkg/app.js";
        init().then(() => {
            // WASM loaded, start hydration
        });
    </script>
</body>
</html>
```

---

## SSR Implementation

### StreamBuilder

```rust
// tachys/src/ssr/mod.rs

pub struct StreamBuilder {
    sync_buf: String,                    // Synchronous HTML buffer
    chunks: VecDeque<StreamChunk>,       // Async chunks
    pending: Option<ChunkFuture>,        // Pending async work
    pending_ooo: VecDeque<PinnedFuture<OooChunk>>,  // Out-of-order chunks
    id: Option<Vec<u16>>,                // Chunk identifier
}

impl StreamBuilder {
    /// Push synchronous HTML
    pub fn push_sync(&mut self, string: &str) {
        self.sync_buf.push_str(string);
    }

    /// Push async block
    pub fn push_async(
        &mut self,
        fut: impl Future<Output = VecDeque<StreamChunk>> + Send + 'static,
    ) {
        // Flush sync buffer
        let sync = mem::take(&mut self.sync_buf);
        if !sync.is_empty() {
            self.chunks.push_back(StreamChunk::Sync(sync));
        }

        // Add async chunk
        self.chunks.push_back(StreamChunk::Async {
            chunks: Box::pin(fut),
        });
    }

    /// Take all ready chunks
    pub fn take_chunks(&mut self) -> VecDeque<StreamChunk> {
        let sync = mem::take(&mut self.sync_buf);
        if !sync.is_empty() {
            self.chunks.push_back(StreamChunk::Sync(sync));
        }
        mem::take(&mut self.chunks)
    }
}
```

### Stream Chunk Types

```rust
pub enum StreamChunk {
    /// Synchronous HTML string
    Sync(String),

    /// Async chunk (waiting for data)
    Async {
        chunks: PinnedFuture<VecDeque<StreamChunk>>,
    },

    /// Out-of-order streaming chunk
    Ooo {
        id: Vec<u16>,
        html: String,
    },
}
```

### Component SSR

```rust
impl RenderHtml for VNode {
    fn to_html_with_buf(
        &self,
        buf: &mut String,
        position: &mut Position,
        escape: bool,
        mark_branches: bool,
        extra_attrs: Vec<AnyAttribute>,
    ) {
        match self {
            VNode::Element(el) => {
                // Build opening tag with attributes
                buf.push_str(&format!("<{}", el.tag));

                for attr in &el.attrs {
                    buf.push_str(&attr.to_html());
                }

                buf.push('>');

                // Render children
                for child in &el.children {
                    child.to_html_with_buf(
                        buf, position, escape, mark_branches, vec![]
                    );
                }

                // Closing tag
                buf.push_str(&format!("</{}>", el.tag));
            }

            VNode::Text(text) => {
                if escape {
                    buf.push_str(&html_escape::text(&text.text));
                } else {
                    buf.push_str(&text.text);
                }
            }

            VNode::Dynamic(view_fn) => {
                // For reactive content
                let view = view_fn();
                view.to_html_with_buf(
                    buf, position, escape, mark_branches, vec![]
                );
            }
        }
    }
}
```

---

## Hydration System

### Client-Side Hydration Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    HTML Loaded                               │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  WASM Download                               │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│             Parse Hydration Data                            │
│        (Read <script> tags with serialized state)           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Create Hydration Cursor                        │
│            (Start at root DOM node)                         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│           Walk DOM and Bind Reactivity                       │
│      (Match components to existing nodes)                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│            Restore Resource State                           │
│         (Deserialize from JSON)                             │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Hydration Complete                             │
│          (App is now interactive)                           │
└─────────────────────────────────────────────────────────────┘
```

### Hydration Cursor

```rust
// tachys/src/hydration.rs

pub struct Cursor(Rc<RefCell<Node>>);

impl Cursor {
    pub fn new(root: Element) -> Self {
        Self(Rc::new(RefCell::new(root.into())))
    }

    pub fn current(&self) -> Node {
        self.0.borrow().clone()
    }

    /// Advance to first child
    pub fn child(&self) {
        let mut inner = self.0.borrow_mut();
        if let Some(child) = first_child(&inner) {
            *inner = child;
        }
    }

    /// Advance to next sibling
    pub fn sibling(&self) {
        let mut inner = self.0.borrow_mut();
        if let Some(sibling) = next_sibling(&inner) {
            *inner = sibling;
        }
    }

    /// Move to parent
    pub fn parent(&self) {
        let mut inner = self.0.borrow_mut();
        if let Some(parent) = get_parent(&inner) {
            *inner = parent;
        }
    }

    /// Find next placeholder (comment marker)
    pub fn next_placeholder(&self, position: &PositionState) -> Placeholder {
        self.advance_to_placeholder(position);
        let marker = self.current();
        Placeholder::cast_from(marker).unwrap()
    }
}
```

### Hydration Markers

Leptos uses comment markers to identify dynamic content:

```html
<!--ssr-open-->
<div class="static">
    <!--ssr-text-node-0-->
    Server Text
    <!--/ssr-text-node-0-->
</div>
<!--/ssr-open-->

<!--ssr-suspense-0-->
<!--ssr-fallback-->
Loading...
<!--/ssr-fallback-->
<!--/ssr-suspense-0-->
```

### Cursor Walking Example

```rust
impl Hydrate for VNode {
    type State = VNodeState;

    fn hydrate(
        self,
        cursor: &Cursor,
        _position: &PositionState,
    ) -> Self::State {
        match self {
            VNode::Element(el) => {
                // Check cursor is at correct element
                let current = cursor.current();
                assert!(current.is_element());

                // Create state
                let state = VNodeState::Element {
                    element: current.dyn_into::<Element>().unwrap(),
                    children: vec![],
                };

                // Descend into children
                cursor.child();
                for child in el.children {
                    let child_state = child.hydrate(cursor, &PositionState::default());
                    state.children.push(child_state);
                    cursor.sibling();
                }
                cursor.parent();

                state
            }

            VNode::Text(text) => {
                // Get text node from cursor
                let current = cursor.current();
                let text_node = current.dyn_ref::<Text>().unwrap();

                // Create state with reactive binding
                VNodeState::Text(TextState {
                    node: text_node.clone(),
                    value: text.text,
                })
            }

            VNode::Dynamic(_) => {
                // Find placeholder comment
                let placeholder = cursor.next_placeholder(&PositionState::FirstChild);

                // Will be replaced by reactive content
                VNodeState::Placeholder(placeholder)
            }
        }
    }
}
```

---

## Resource Hydration

### Server-Side Resource Collection

```rust
// reactive_graph/src/computed/resource.rs

impl<T> Resource<T> {
    /// On server, serialize resource for hydration
    pub fn to_hydration(&self) -> Option<String> {
        let value = self.value.get();
        serde_json::to_string(&value).ok()
    }
}

// In SSR context
let resources = vec![
    (resource_id_0, resource_0.to_hydration()),
    (resource_id_1, resource_1.to_hydration()),
];

let json = serde_json::to_string(&resources).unwrap();
```

### Client-Side Resource Restoration

```rust
// hydration_context/src/hydrate.rs

pub struct HydrationContext {
    serialized_data: Arc<RwLock<HashMap<SerializedDataId, String>>>,
}

impl SharedContext for HydrationContext {
    fn read_data(&self, id: &SerializedDataId) -> Option<String> {
        // Read data that was serialized from server
        self.serialized_data.read().unwrap().get(id).cloned()
    }
}

// Resource reads from hydration context
impl<T> Resource<T> {
    fn hydrate_value(&self, id: &SerializedDataId) {
        if let Some(json) = SharedContext::read_data(id) {
            let value: Option<T> = serde_json::from_str(&json).unwrap();
            self.value.set(value);
        }
    }
}
```

---

## Suspense and Streaming

### Out-of-Order Streaming

```rust
// leptos/src/suspense_component.rs

impl RenderHtml for SuspenseBoundary {
    fn to_html_stream(&self) -> impl Stream<Item = String> {
        let mut stream = StreamBuilder::new();

        // Push fallback immediately
        stream.push_fallback(
            &self.fallback,
            &mut position,
            true,
            vec![],
        );

        // Stream actual content async
        stream.push_async(async move {
            // Wait for resources
            let content = self.children.to_html();

            // Return chunk with ID for OOO replacement
            vec![StreamChunk::Ooo {
                id: self.chunk_id.clone(),
                html: content,
            }]
        });

        stream
    }
}
```

### Client-Side OOO Replacement

```javascript
// hydration/islands_routing.js

window.handleOooChunk = (id, html) => {
    const placeholder = document.querySelector(
        `[data-ooo-chunk="${id}"]`
    );

    if (placeholder) {
        const template = document.createElement('template');
        template.innerHTML = html;

        // Replace placeholder with actual content
        placeholder.parentNode.replaceChild(
            template.content,
            placeholder
        );

        // Hydrate new content
        hydrateNode(template.content);
    }
};
```

---

## Islands Architecture

### Island Component Structure

```rust
// Server component (not hydrated)
#[component]
fn BlogPost() -> impl IntoView {
    view! {
        <article>
            <h1>{title}</h1>
            <Content/>  // Static HTML

            // Interactive island (hydrated)
            <CommentSection post_id={id}/>
        </article>
    }
}

// Island component (hydrated)
#[island]
fn CommentSection(post_id: String) -> impl IntoView {
    let (comments, set_comments) = signal(Vec::new());

    view! {
        <div>
            <For
                each=move || comments.get()
                key=|c| c.id
                children=|comment| view! { <Comment {...comment}/> }
            />
        </div>
    }
}
```

### Island Hydration Script

```javascript
// hydration/island_script.js

// Parse island metadata from HTML
const islandMetadata = JSON.parse(
    document.getElementById('__LEPTOS_ISLANDS__').textContent
);

// Load and hydrate each island
for (const island of islandMetadata) {
    const element = document.querySelector(
        `[data-island="${island.id}"]`
    );

    // Create reactive scope for island
    const owner = Owner::new();
    owner.with(() => {
        // Mount island component
        const component = createIslandComponent(island.name, island.props);
        mount(component, element);
    });
}
```

---

## Error Handling

### Error Boundaries in SSR

```rust
// leptos/src/error_boundary.rs

impl RenderHtml for ErrorBoundary {
    fn to_html_stream(&self) -> impl Stream<Item = String> {
        // Check for errors during rendering
        match self.children.render() {
            Ok(content) => content.to_html_stream(),
            Err(error) => {
                // Render fallback with error state
                let error_id = register_error(&error);
                self.fallback.render_error(error_id).to_html_stream()
            }
        }
    }
}
```

### Error Serialization

```rust
// hydration_context/src/ssr.rs

impl SharedContext for SsrContext {
    fn register_error(
        &self,
        boundary_id: SerializedDataId,
        error_id: ErrorId,
        error: Error,
    ) {
        // Store error for client hydration
        self.errors.write().unwrap().insert(
            boundary_id,
            (error_id, error.to_string()),
        );
    }
}
```

---

## Performance Optimizations

### Static String Optimization

```rust
// View macro output

// ❌ Without optimization (many allocations)
Element::new("div")
    .attr("class", "container")
    .child(
        Element::new("h1")
            .child("Hello")
    )

// ✅ With optimization (single string)
Element::from_static(r#"<div class="container"><h1>Hello</h1></div>"#)
```

### Streaming Modes

| Mode | Description |
|------|-------------|
| **In-Order** | Wait for all data, then stream HTML |
| **Out-of-Order** | Stream HTML immediately, replace async parts |
| **Progressive** | Stream chunks as they complete |

### Resource Blocking

```rust
// Block HTML stream until critical resources load
let blocking_resources = vec![resource_a, resource_b];

let stream = if !blocking_resources.is_empty() {
    // Wait for blocking resources
    future::join_all(blocking_resources)
        .map(|_| render_html())
        .boxed()
} else {
    // Stream immediately
    render_html().boxed()
};
```

---

## Debugging

### Hydration Mismatches

Common causes:
1. **Different content**: Server and client render different HTML
2. **Missing markers**: Comment markers not found
3. **Resource timing**: Resources not loaded before hydration

```rust
#[cfg(debug_assertions)]
fn debug_hydration() {
    // Compare expected vs actual node types
    if current.node_type() != expected {
        console::warn_1(&"Hydration mismatch!".into());
    }

    // Log cursor position
    log_node_hierarchy(cursor.current());
}
```

---

## Resources

- [tachys/src/ssr](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/tachys/src/ssr)
- [tachys/src/hydration](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/tachys/src/hydration)
- [hydration_context](/home/darkvoid/Boxxed/@formulas/src.rust/src.leptos/leptos/hydration_context)
