# Floneum Desktop UI Deep Dive

## Overview

The Floneum desktop application is a visual graph editor built with **Dioxus**, a React-like UI framework for Rust. It provides an intuitive interface for creating AI workflows by connecting plugin nodes.

**Source Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.floneum/floneum/floneum/floneum/`

## Application Structure

```
floneum/ (UI application)
├── src/
│   ├── main.rs           # Application entry point
│   ├── graph.rs          # Visual graph rendering
│   ├── node.rs           # Node component
│   ├── edge.rs           # Edge/connection component
│   ├── connection.rs     # Connection rendering
│   ├── input.rs          # Node input ports
│   ├── output.rs         # Node output ports
│   ├── node_value.rs     # Value handling
│   ├── sidebar.rs        # Plugin sidebar
│   ├── plugin_search.rs  # Plugin search functionality
│   ├── current_node.rs   # Currently selected node info
│   ├── window.rs         # Window configuration
│   ├── icons.rs          # UI icons
│   └── theme.rs          # Styling theme
└── public/
    └── tailwind.css      # Tailwind styles
```

## Application Entry Point

```rust
fn main() {
    // Setup tracing
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    let log_path = directories::ProjectDirs::from("com", "floneum", "floneum")
        .unwrap()
        .data_dir()
        .join("debug.log");
    std::fs::create_dir_all(log_path.parent().unwrap()).unwrap();
    let file = File::create(log_path).unwrap();
    let debug_log = tracing_subscriber::fmt::layer().with_writer(std::sync::Arc::new(file));

    let logger = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::ERROR.into())
                .from_env_lossy(),
        )
        .pretty()
        .finish();

    logger.with(debug_log).init();

    // Create window config
    let config = match make_config() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to make config: {:?}", err);
            return;
        }
    };

    dioxus::prelude::LaunchBuilder::new()
        .with_cfg(config)
        .launch(App);
}
```

## Application State

```rust
#[derive(Default)]
pub struct ApplicationState {
    graph: VisualGraph,
    currently_focused: Option<FocusedNodeInfo>,
    resource_storage: ResourceStorage,
    plugins: HashMap<String, Plugin>,
}

impl ApplicationState {
    async fn insert_plugin(&mut self, name: &str) -> Result<()> {
        match self.get_plugin(name) {
            Some(plugin) => {
                let instance = plugin.instance().await?;
                self.graph.create_node(instance)?;
                Ok(())
            }
            None => Err(anyhow::anyhow!("Plugin not found")),
        }
    }

    async fn add_plugin(&mut self, plugin: Plugin) -> Result<()> {
        let name = plugin.name().await?;
        self.plugins.insert(name.clone(), plugin);
        Ok(())
    }

    pub(crate) fn clear(&mut self) {
        self.graph.clear();
        self.currently_focused = None;
        self.resource_storage.clear();
    }
}
```

## Visual Graph System

### Graph Data Structure

```rust
pub struct VisualGraphInner {
    pub graph: StableGraph<Signal<Node>, Signal<Edge>>,
    pub connections: Slab<ConnectionProps>,
    pub currently_dragging: Option<CurrentlyDragging>,
    pub pan_pos: Point2D<f32, f32>,
    pub zoom: f32,
}
```

Using `petgraph::stable_graph::StableGraph` provides:
- Stable node indices even after removals
- Efficient graph traversal
- Edge direction for data flow

### Creating Nodes

```rust
impl VisualGraph {
    pub fn create_node(&self, instance: PluginInstance) -> anyhow::Result<()> {
        let position = self.scale_screen_pos(PagePoint::new(0., 0.));
        let mut inner_mut = self.inner;
        let mut inner = inner_mut.write();

        // Create input signals
        let mut inputs = Vec::new();
        for input in &instance.metadata().inputs {
            inputs.push(Signal::new_in_scope(
                NodeInput::new(input.clone(), vec![input.ty.create(instance.resources())?]),
                self.inner.origin_scope(),
            ));
        }

        // Create output signals
        let mut outputs = Vec::new();
        for output in &instance.metadata().outputs {
            outputs.push(Signal::new_in_scope(
                NodeOutput {
                    definition: output.clone(),
                    value: output.ty.create(instance.resources())?,
                    rendered_size: None,
                },
                self.inner.origin_scope(),
            ));
        }

        let node = Signal::new_in_scope(
            Node {
                instance,
                position,
                running: false,
                queued: false,
                error: None,
                rendered_size: None,
                id: Default::default(),
                inputs,
                outputs,
            },
            ScopeId::ROOT,
        );
        let idx = inner.graph.add_node(node);
        inner.graph[idx].write().id = idx;

        Ok(())
    }
}
```

### Graph Execution

The graph handles execution with dependency tracking:

```rust
fn should_run_node(&self, id: NodeIndex) -> bool {
    let graph = self.inner.read();

    // Check if node is already running
    if graph.graph[id].read().running {
        return false;
    }

    // Traverse back through inputs to check dependencies
    let mut visited = HashSet::default();
    visited.insert(id);
    let mut should_visit = Vec::new();

    for input in graph.graph.edges_directed(id, petgraph::Direction::Incoming) {
        let source = input.source();
        should_visit.push(source);
        visited.insert(source);
    }

    while let Some(new_id) = should_visit.pop() {
        let node = graph.graph[new_id].read();
        if node.running || node.queued {
            return false;  // Wait for dependencies
        }
        // Continue traversing...
    }

    true
}

pub fn run_node(&self, mut node: Signal<Node>) {
    let current_node_id = node.read().id;

    if self.set_input_nodes(current_node_id) {
        let inputs = node.read().inputs.iter().map(|i| i.read().value()).collect();

        node.write().running = true;
        node.write().queued = true;

        let graph = self.inner;
        spawn(async move {
            let result = node.write().instance.run(inputs).await;

            match result.as_deref() {
                Some(Ok(result)) => {
                    // Copy outputs
                    for (out, current) in result.iter().zip(node.write().outputs.iter()) {
                        current.write_unchecked().value.clone_from(out);
                    }

                    // Queue downstream nodes
                    for edge in graph.read().graph.edges_directed(current_node_id, petgraph::Direction::Outgoing) {
                        let new_node_id = edge.target();
                        graph.read().graph[new_node_id].write().queued = true;
                    }
                }
                Some(Err(err)) => {
                    node.write().error = Some(err.to_string());
                }
                None => {}
            }

            node.write().running = false;
            node.write().queued = false;
        });
    }
}
```

### Connection Management

```rust
pub fn connect(
    &mut self,
    input_id: NodeIndex,
    output_id: NodeIndex,
    edge: Signal<Edge>,
) {
    if !self.check_connection_validity(input_id, output_id, edge) {
        return;
    }

    let mut current_graph = self.inner.write();

    // Remove existing connections to this input
    let mut edges_to_remove = Vec::new();
    {
        let input_index = edge.read().end;
        for edge in current_graph.graph.edges_directed(output_id, petgraph::Direction::Incoming) {
            if edge.weight().read().end == input_index {
                edges_to_remove.push(edge.id());
            }
        }
        for edge in edges_to_remove {
            current_graph.graph.remove_edge(edge);
        }
    }

    current_graph.graph.add_edge(input_id, output_id, edge);
}

pub fn check_connection_validity(
    &self,
    input_id: NodeIndex,
    output_id: NodeIndex,
    edge: Signal<Edge>,
) -> bool {
    let edge = edge.read();
    let graph = self.inner.read();

    let input = graph.graph[input_id].read().output_type(edge.start).unwrap();
    let output = graph.graph[output_id].read().input_type(edge.end).unwrap();

    input.compatible(&output)
}
```

## FlowView Component

The main graph canvas:

```rust
pub fn FlowView(mut props: FlowViewProps) -> Element {
    use_context_provider(|| props.graph);
    let mut graph = props.graph.inner;
    let current_graph = graph.read();

    let pan_pos = current_graph.pan_pos;
    let zoom = current_graph.zoom;

    let transform = format!(
        "matrix({} {} {} {} {} {})",
        1. * zoom, 0., 0., 1. * zoom, pan_pos.x, pan_pos.y
    );

    rsx! {
        div {
            position: "relative",
            width: "100%",
            height: "100%",
            onmousemove: move |evt| props.graph.update_mouse(&evt),

            // Zoom controls
            div { button { onclick: |_| zoom_in(), "+" } }
            div { button { onclick: |_| zoom_out(), "-" } }

            // Render nodes
            for id in current_graph.graph.node_identifiers() {
                Node { key: "{id:?}", node: current_graph.graph[id] }
            }

            // SVG layer for edges
            svg {
                width: "100%",
                height: "100%",
                onmousedown: move |evt| start_pan(evt),
                onmousemove: move |evt| handle_pan(evt),
                onmouseup: move |_| end_pan(),

                g { transform: "{transform}",
                    // Render edges
                    for edge_ref in current_graph.graph.edge_references() {
                        NodeConnection {
                            start: current_graph.graph[edge_ref.target()],
                            connection: current_graph.graph[edge_ref.id()],
                            end: current_graph.graph[edge_ref.source()]
                        }
                    }

                    // Dragging connection preview
                    if let Some(CurrentlyDragging::Connection(drag)) = &current_graph.currently_dragging {
                        CurrentlyDragging { from: drag.from, to: drag.to, ... }
                    }
                }
            }
        }
    }
}
```

## Interaction Handling

### Panning

```rust
let mut drag_start_pos = use_signal(|| Option::<Point2D<f32, f32>>::None);
let mut drag_pan_pos = use_signal(|| Option::<Point2D<f32, f32>>::None);

onmousedown: move |evt| {
    let pos = evt.element_coordinates();
    drag_start_pos.set(Some(Point2D::new(pos.x as f32, pos.y as f32)));
    drag_pan_pos.set(Some(pan_pos));
},

onmousemove: move |evt| {
    if let (Some(drag_start), Some(drag_pan)) = (drag_start_pos(), drag_pan_pos()) {
        let end_pos = Point2D::new(evt.element_coordinates().x as f32, ...);
        let diff = end_pos - drag_start;
        graph.with_mut(|g| {
            g.pan_pos.x = drag_pan.x + diff.x;
            g.pan_pos.y = drag_pan.y + diff.y;
        });
    }
},
```

### Zooming

```rust
button {
    onclick: move |_| {
        let new_zoom = zoom * 1.1;
        graph.with_mut(|graph| graph.zoom = new_zoom);
    },
    "+"
}
```

### Node Dragging

```rust
pub fn start_dragging_node(&mut self, evt: &MouseData, node: Signal<Node>) {
    let mut inner = self.inner.write();
    inner.currently_dragging = Some(CurrentlyDragging::Node(NodeDragInfo {
        element_offset: evt.element_coordinates().cast().cast_unit(),
        node,
    }));
}

pub fn update_mouse(&mut self, evt: &MouseData) {
    let new_pos = self.scale_screen_pos(evt.page_coordinates());

    match &mut inner.currently_dragging {
        Some(CurrentlyDragging::Node(drag)) => {
            let mut node = drag.node.write();
            node.position.x = new_pos.x - drag.element_offset.x;
            node.position.y = new_pos.y - drag.element_offset.y;
        }
        Some(CurrentlyDragging::Connection(drag)) => {
            let mut to = drag.to.write();
            *to = new_pos;
        }
        _ => {}
    }
}
```

## Input/Output Handling

### Setting Node Inputs

```rust
pub fn set_input_nodes(&self, id: NodeIndex) -> bool {
    if !self.should_run_node(id) {
        return false;
    }

    let graph = self.inner.read();
    let inputs = &graph.graph[id].read().inputs;

    for input in graph.graph.edges_directed(id, petgraph::Direction::Incoming) {
        let source = input.source();
        let edge = input.weight().read();
        let start_index = edge.start;
        let end_index = edge.end;

        let source_node = graph.graph[source].read();
        let value = source_node.outputs[start_index].read().as_input();

        let mut target_input = inputs[end_index.index];
        let mut input_value = target_input.write();
        input_value.set_connection(end_index.ty, value);
    }

    true
}
```

### Type Compatibility

```rust
pub fn compatible(&self, other: &ValueType) -> bool {
    match (self, other) {
        (ValueType::Single(a), ValueType::Single(b)) => a.compatible(b),
        (ValueType::Many(a), ValueType::Many(b)) => a.compatible(b),
        (ValueType::Single(a), ValueType::Many(b)) => a.compatible(b),
        (ValueType::Many(_), ValueType::Single(_)) => false,
        _ => false,
    }
}

pub fn compatible(&self, other: &PrimitiveValueType) -> bool {
    match (self, other) {
        // Any type is compatible with Any
        (PrimitiveValueType::Any, _) | (_, PrimitiveValueType::Any) => true,
        // Same types are compatible
        (a, b) if a == b => true,
        // Number and Float are compatible
        (PrimitiveValueType::Number, PrimitiveValueType::Float) => true,
        (PrimitiveValueType::Float, PrimitiveValueType::Number) => true,
        _ => false,
    }
}
```

## Plugin Search Sidebar

The sidebar allows users to search and add plugins:

```rust
pub fn Sidebar() -> Element {
    let package_manager = use_package_manager();
    let state = use_application_state();
    let mut search_query = use_signal(|| String::new());

    let filtered_plugins = package_manager.as_ref().map(|pm| {
        let query = search_query.read().to_lowercase();
        pm.entries()
            .iter()
            .filter(|entry| {
                entry.meta()
                    .map(|m| m.name.to_lowercase().contains(&query))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>()
    });

    rsx! {
        div { class: "sidebar",
            input {
                placeholder: "Search plugins...",
                value: search_query.read().clone(),
                oninput: move |e| search_query.set(e.value()),
            }

            for plugin in filtered_plugins.unwrap_or_default() {
                PluginEntry {
                    plugin,
                    on_click: move |_| {
                        spawn(async move {
                            state.write().insert_plugin(plugin.meta().unwrap().name.as_ref()).await;
                        });
                    }
                }
            }
        }
    }
}
```

## Current Node Info

Display information about the selected node:

```rust
pub struct FocusedNodeInfo {
    pub node: Signal<Node>,
    pub logs: Signal<Vec<String>>,
}

pub fn CurrentNodeInfo() -> Element {
    let focused = use_application_state().read().currently_focused.clone();

    if let Some(info) = focused {
        let node = info.node.read();
        let logs = node.instance.read_logs();

        rsx! {
            div { class: "node-info",
                h3 { "{node.instance.metadata().name}" }
                p { "{node.instance.metadata().description}" }

                // Display logs
                div { class: "logs",
                    for log in logs.iter() {
                        p { "{log}" }
                    }
                }

                // Input/Output values
                for input in node.inputs.iter() {
                    InputDisplay { input }
                }
                for output in node.outputs.iter() {
                    OutputDisplay { output }
                }
            }
        }
    } else {
        rsx! { div { "Select a node to view details" } }
    }
}
```

## Value Type Colors

Type-based color coding for connections:

```rust
impl ValueType {
    pub fn color(&self) -> &'static str {
        match self {
            ValueType::Single(p) | ValueType::Many(p) => match p {
                PrimitiveValueType::Number => "#FF6B6B",  // Red
                PrimitiveValueType::Float => "#4ECDC4",   // Teal
                PrimitiveValueType::Text => "#45B7D1",    // Blue
                PrimitiveValueType::Boolean => "#96CEB4", // Green
                PrimitiveValueType::Model => "#FFEEAD",   // Yellow
                PrimitiveValueType::Embedding => "#D4A5A5", // Pink
                PrimitiveValueType::Database => "#9B59B6", // Purple
                PrimitiveValueType::Page => "#3498DB",    // Dark Blue
                PrimitiveValueType::Node => "#E74C3C",    // Red
                _ => "#95A5A6",                           // Gray
            },
        }
    }
}
```

## Window Configuration

```rust
pub fn make_config() -> Result<dioxus::prelude::Config> {
    use dioxus::prelude::*;

    let window_config = WindowConfig::default()
        .with_title("Floneum")
        .with_resizable(true)
        .with_decorations(true)
        .with_transparent(false);

    Ok(Config::default().with_window(window_config))
}
```

## Styling

The UI uses Tailwind CSS for styling. The main stylesheet is generated from `input.css`:

```bash
npx tailwindcss -i ./input.css -o ./public/tailwind.css --watch
```

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `dioxus` | UI framework |
| `dioxus-desktop` | Desktop rendering |
| `petgraph` | Graph data structure |
| `slab` | Efficient indexed storage |
| `tracing-subscriber` | Logging |
| `directories` | Platform-specific paths |
| `anyhow` | Error handling |

## Build Process

```bash
# Watch Tailwind CSS
npx tailwindcss -i ./input.css -o ./public/tailwind.css --watch

# Build and run
cargo run --release --target <target-triple>
```
