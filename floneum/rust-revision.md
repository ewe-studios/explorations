# Reproducing Floneum in Rust - Production Guide

## Overview

This guide explains how to reproduce Floneum's functionality in Rust at a production level. Floneum combines flow-based programming, WebAssembly plugins, and local AI into a cohesive system.

## Architecture Components to Reproduce

1. **WebAssembly Plugin Host** - Wasmtime-based runtime with component model
2. **WIT Interface Definitions** - Interface types for host-plugin communication
3. **Resource Management** - Type-safe resource storage and borrowing
4. **Flow-Based Graph Engine** - Directed graph with dependency resolution
5. **Visual UI** - Dioxus-based graph editor (optional, could use egui/iced)
6. **Package Management** - Plugin discovery and distribution
7. **ML Integration** - Local LLM and embedding model support

## 1. Setting Up the Plugin Host

### Core Dependencies

```toml
[dependencies]
# WebAssembly runtime
wasmtime = { version = "15", features = ["component-model"] }
wasmtime-wasi = { version = "15" }
wit-component = "0.19"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Error handling
anyhow = "1.0"

# Utilities
parking_lot = "0.12"
tracing = "0.1"
once_cell = "1.18"
```

### Engine and Linker Setup

```rust
use wasmtime::{Config, Engine, component::{Component, Linker, ResourceTable}};
use wasmtime_wasi::{WasiCtxBuilder, WasiCtx, WasiView, DirPerms, FilePerms};
use once_cell::sync::Lazy;

// Global engine with component model support
static ENGINE: Lazy<Engine> = Lazy::new(|| {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.async_support(true);
    Engine::new(&config).unwrap()
});

// Global linker for all plugins
static LINKER: Lazy<Linker<PluginState>> = Lazy::new(|| {
    let mut linker = Linker::new(&ENGINE);
    // Add your host interfaces here
    // Both::add_to_linker(&mut linker, |x| x).unwrap();
    linker
});
```

### Plugin State Structure

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct SharedPluginState {
    pub logs: Arc<RwLock<Vec<String>>>,
    pub resources: ResourceStorage,
}

pub struct PluginState {
    pub shared: SharedPluginState,
    pub plugin_kv: HashMap<Vec<u8>, Vec<u8>>,
    pub table: ResourceTable,
    pub ctx: WasiCtx,
}

impl PluginState {
    pub fn new(shared: SharedPluginState) -> Self {
        let sandbox = std::path::Path::new("./sandbox");
        std::fs::create_dir_all(sandbox).unwrap();

        let ctx = WasiCtxBuilder::new()
            .inherit_stdio()
            .preopened_dir(sandbox, "./", DirPerms::all(), FilePerms::all())
            .unwrap()
            .build();

        Self {
            shared,
            plugin_kv: Default::default(),
            table: ResourceTable::new(),
            ctx,
        }
    }
}

impl WasiView for PluginState {
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }
}
```

## 2. WIT Interface Definitions

Create a `wit/plugin.wit` file:

```wit
package myapp:plugins;

interface imports {
    // Plugin state persistence
    store: func(key: list<u8>, value: list<u8>);
    load: func(key: list<u8>) -> list<u8>;
    unload: func(key: list<u8>);

    // Logging
    log-to-user: func(message: string);
}

interface types {
    // Value types for data flow
    variant primitive-value {
        number(s64),
        float(f64),
        text(string),
        boolean(bool),
        embedding(list<float32>),
        // Add more as needed
    }

    // IO definitions
    record io-definition {
        name: string,
        ty: value-type,
    }

    variant value-type {
        single(primitive-value-type),
        many(primitive-value-type),
    }

    enum primitive-value-type {
        number,
        float,
        text,
        boolean,
        embedding,
        any,
    }

    // Plugin metadata
    record definition {
        name: string,
        description: string,
        inputs: list<io-definition>,
        outputs: list<io-definition>,
    }
}

interface definitions {
    use types.{definition, primitive-value};
    structure: func() -> definition;
    run: func(inputs: list<list<primitive-value>>) -> list<list<primitive-value>>;
}

world plugin-world {
    export definitions;
    import imports;
    import types;
}
```

Generate bindings:

```rust
// In lib.rs
wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin-world",
    async: true,
});
```

## 3. Resource Management System

### Type-Erased Resource Storage

```rust
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::Arc;

type ResourceMap = Arc<RwLock<HashMap<TypeId, Slab<Box<dyn Any + Send + Sync>>>>>;

#[derive(Default, Clone)]
pub struct ResourceStorage {
    map: ResourceMap,
}

#[derive(Clone, Copy)]
pub struct Resource<T> {
    index: usize,
    owned: bool,
    phantom: PhantomData<T>,
}

impl ResourceStorage {
    pub fn insert<T: Send + Sync + 'static>(&self, item: T) -> Resource<T> {
        let ty_id = TypeId::of::<T>();
        let mut binding = self.map.write();
        let slab = binding.entry(ty_id).or_insert_with(Slab::new);
        let id = slab.insert(Box::new(item));
        Resource {
            index: id,
            owned: true,
            phantom: PhantomData,
        }
    }

    pub fn get<T: Send + Sync + 'static>(
        &self,
        key: Resource<T>,
    ) -> Option<impl Deref<Target = T> + '_> {
        RwLockReadGuard::try_map(self.map.read(), |r| {
            r.get(&TypeId::of::<T>())
                .and_then(|slab| slab.get(key.index))
                .and_then(|any| any.downcast_ref())
        }).ok()
    }

    pub fn get_mut<T: Send + Sync + 'static>(
        &self,
        key: Resource<T>,
    ) -> Option<impl DerefMut<Target = T> + '_> {
        RwLockWriteGuard::try_map(self.map.write(), |r| {
            r.get_mut(&TypeId::of::<T>())
                .and_then(|slab| slab.get_mut(key.index))
                .and_then(|any| any.downcast_mut())
        }).ok()
    }

    pub fn drop_key<T: 'static>(&self, key: Resource<T>) {
        assert!(key.owned);
        if let Some(slab) = self.map.write().get_mut(&TypeId::of::<T>()) {
            slab.remove(key.index);
        }
    }
}
```

## 4. Plugin Loading and Execution

### Plugin Structure

```rust
use wasmtime::component::Component;
use wit_component::ComponentEncoder;
use std::path::Path;
use once_cell::sync::OnceCell;

pub struct Plugin {
    shared: SharedPluginState,
    component: OnceCell<Component>,
    wasm_path: PathBuf,
}

impl Plugin {
    pub fn load(path: impl AsRef<Path>, shared: SharedPluginState) -> Self {
        Self {
            shared,
            wasm_path: path.as_ref().to_path_buf(),
            component: OnceCell::new(),
        }
    }

    async fn component(&self) -> anyhow::Result<&Component> {
        if let Some(comp) = self.component.get() {
            return Ok(comp);
        }

        let bytes = tokio::fs::read(&self.wasm_path).await?;

        // Convert WASM module to component
        let component = ComponentEncoder::default()
            .module(&bytes)?
            .validate(true)
            .adapter("wasi_snapshot_preview1", include_bytes!("wasi_snapshot_preview1.wasm"))?
            .encode()?;

        let component = Component::from_binary(&ENGINE, &component)?;
        let _ = self.component.set(component);
        Ok(self.component.get().unwrap())
    }

    pub async fn create_instance(&self) -> anyhow::Result<PluginInstance> {
        let state = PluginState::new(self.shared.clone());
        let mut store = Store::new(&ENGINE, state);
        let component = self.component().await?;

        let (instance, _bindings) = PluginWorld::instantiate_async(&mut store, component, &LINKER).await?;

        Ok(PluginInstance {
            store: Arc::new(Mutex::new(store)),
            instance,
        })
    }
}
```

### Plugin Instance

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct PluginInstance {
    store: Arc<Mutex<Store<PluginState>>>,
    instance: PluginWorld,
}

impl PluginInstance {
    pub async fn run(&self, inputs: Vec<Vec<PrimitiveValue>>) -> anyhow::Result<Vec<Vec<PrimitiveValue>>> {
        let mut store = self.store.lock().await;
        let outputs = self.instance.call_run(&mut store, &inputs).await?;
        Ok(outputs)
    }

    pub async fn get_structure(&self) -> anyhow::Result<Definition> {
        let mut store = self.store.lock().await;
        Ok(self.instance.call_structure(&mut store).await?)
    }
}
```

## 5. Flow-Based Graph Engine

### Graph Data Structure

```rust
use petgraph::stable_graph::{StableGraph, NodeIndex};
use std::collections::HashSet;

pub type Graph = StableGraph<Signal<Node>, Signal<Edge>>;

pub struct Node {
    pub instance: PluginInstance,
    pub position: Point2D<f32, f32>,
    pub running: bool,
    pub queued: bool,
    pub inputs: Vec<Signal<NodeInput>>,
    pub outputs: Vec<Signal<NodeOutput>>,
    pub id: NodeIndex,
}

pub struct Edge {
    pub start: OutputIndex,  // (output_index)
    pub end: InputIndex,     // (input_index)
}

pub struct VisualGraph {
    graph: Signal<Graph>,
}
```

### Dependency Resolution

```rust
impl VisualGraph {
    fn should_run_node(&self, id: NodeIndex) -> bool {
        let graph = self.graph.read();

        // Check if already running
        if graph[id].read().running {
            return false;
        }

        // Check all upstream dependencies
        let mut visited = HashSet::new();
        let mut to_visit = Vec::new();

        for edge in graph.edges_directed(id, petgraph::Direction::Incoming) {
            to_visit.push(edge.source());
            visited.insert(edge.source());
        }

        while let Some(node_id) = to_visit.pop() {
            let node = graph[node_id].read();
            if node.running || node.queued {
                return false;  // Wait for dependency
            }
            for edge in graph.edges_directed(node_id, petgraph::Direction::Incoming) {
                if visited.insert(edge.source()) {
                    to_visit.push(edge.source());
                }
            }
        }

        true
    }

    fn set_input_nodes(&self, id: NodeIndex) -> bool {
        if !self.should_run_node(id) {
            return false;
        }

        let graph = self.graph.read();

        for edge in graph.edges_directed(id, petgraph::Direction::Incoming) {
            let source_id = edge.source();
            let edge_data = edge.weight().read();
            let source_node = graph[source_id].read();
            let target_node = graph[id].read();

            // Get value from source output
            let value = source_node.outputs[edge_data.start.index].read().value();

            // Set value on target input
            let mut target_input = target_node.inputs[edge_data.end.index].write();
            target_input.set_value(value);
        }

        true
    }

    pub fn run_node(&self, id: NodeIndex) {
        if !self.set_input_nodes(id) {
            return;
        }

        let node = self.graph.read()[id].clone();
        let inputs: Vec<Vec<_>> = node.inputs.iter()
            .map(|i| i.read().value())
            .collect();

        node.write().running = true;
        node.write().queued = true;

        tokio::spawn(async move {
            let result = node.read().instance.run(inputs).await;

            match result {
                Ok(outputs) => {
                    // Set outputs and trigger downstream nodes
                    for (output, value) in node.write().outputs.iter().zip(outputs) {
                        output.write().set_value(value);
                    }

                    // Queue downstream nodes
                    let graph = /* get graph */;
                    for edge in graph.edges_directed(id, petgraph::Direction::Outgoing) {
                        graph[edge.target()].write().queued = true;
                    }
                }
                Err(e) => {
                    node.write().error = Some(e.to_string());
                }
            }

            node.write().running = false;
            node.write().queued = false;
        });
    }
}
```

### Connection Management

```rust
impl VisualGraph {
    pub fn connect(&mut self, from: NodeIndex, to: NodeIndex, edge: Signal<Edge>) {
        if !self.is_compatible(from, to, &edge.read()) {
            return;
        }

        let mut graph = self.graph.write();

        // Remove existing connection to this input
        let input_idx = edge.read().end.index;
        let edges_to_remove: Vec<_> = graph
            .edges_directed(to, petgraph::Direction::Incoming)
            .filter(|e| e.weight().read().end.index == input_idx)
            .map(|e| e.id())
            .collect();

        for edge_id in edges_to_remove {
            graph.remove_edge(edge_id);
        }

        graph.add_edge(from, to, edge);
    }

    fn is_compatible(&self, from: NodeIndex, to: NodeIndex, edge: &Edge) -> bool {
        let graph = self.graph.read();
        let output_type = graph[from].read().output_type(edge.start.index);
        let input_type = graph[to].read().input_type(edge.end.index);

        output_type.compatible(&input_type)
    }
}
```

## 6. Value Types and Type System

```rust
#[derive(Clone, Debug)]
pub enum PrimitiveValue {
    Number(i64),
    Float(f64),
    Text(String),
    Boolean(bool),
    Embedding(Vec<f32>),
    // Add custom types as needed
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveValueType {
    Number,
    Float,
    Text,
    Boolean,
    Embedding,
    Any,
}

#[derive(Clone, Debug)]
pub enum ValueType {
    Single(PrimitiveValueType),
    Many(PrimitiveValueType),
}

impl ValueType {
    pub fn compatible(&self, other: &ValueType) -> bool {
        match (self, other) {
            (ValueType::Single(a), ValueType::Single(b)) => a.compatible(b),
            (ValueType::Many(a), ValueType::Many(b)) => a.compatible(b),
            (ValueType::Single(a), ValueType::Many(b)) => a.compatible(b),
            (ValueType::Many(_), ValueType::Single(_)) => false,
            _ => false,
        }
    }
}

impl PrimitiveValueType {
    pub fn compatible(&self, other: &Self) -> bool {
        match (self, other) {
            (PrimitiveValueType::Any, _) | (_, PrimitiveValueType::Any) => true,
            (a, b) if a == b => true,
            // Number and Float are interchangeable
            (PrimitiveValueType::Number, PrimitiveValueType::Float) => true,
            (PrimitiveValueType::Float, PrimitiveValueType::Number) => true,
            _ => false,
        }
    }
}
```

## 7. Plugin Development Framework

### Macro for Plugin Export

```rust
// In your plugin macro crate
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};
use quote::quote;

#[proc_macro_attribute]
pub fn export_plugin(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);
    let func_name = &func.sig.ident;

    // Extract doc comments for description
    // Parse function signature for types
    // Generate Guest implementation

    quote! {
        pub struct Plugin;

        impl Guest for Plugin {
            fn structure() -> Definition {
                Definition {
                    name: stringify!(#func_name),
                    description: "",  // Extract from doc comments
                    inputs: vec![],   // Parse from function params
                    outputs: vec![],  // Parse from return type
                }
            }

            fn run(input: Vec<Vec<PrimitiveValue>>) -> Vec<Vec<PrimitiveValue>> {
                // Extract inputs and call function
                #func
            }
        }

        export!(Plugin);
    }.into()
}
```

### Plugin Template

```rust
// Plugin source (src/lib.rs)
use floneum_rust::*;

#[export_plugin]
/// My plugin description
fn my_plugin(input1: String, input2: i64) -> String {
    format!("Received: {} with number {}", input1, input2)
}
```

### Build Script

```rust
// build.rs
use std::process::Command;

fn main() {
    // Build to WASM
    Command::new("cargo")
        .args(&["build", "--release", "--target", "wasm32-wasi"])
        .status()
        .unwrap();

    // Convert to component (optional, can be done by host)
    // Or use cargo-component if available
}
```

## 8. ML Integration (Optional)

### Using Candle for Local Models

```toml
[dependencies]
candle-core = "0.3"
candle-nn = "0.3"
candle-transformers = "0.3"
hf-hub = "0.3"
tokenizers = "0.15"
```

### Text Generation Model Wrapper

```rust
use candle::{Tensor, DType, Device};
use candle_transformers::models::llama::{Llama, LlamaConfig};
use hf_hub::{Repo, RepoType, Api};
use tokenizers::Tokenizer;

pub struct TextGenerationModel {
    model: Llama,
    tokenizer: Tokenizer,
    device: Device,
}

impl TextGenerationModel {
    pub async fn load(model_id: &str) -> anyhow::Result<Self> {
        let api = Api::new()?;
        let repo = api.repo(Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            "main".to_string(),
        ));

        let config_path = repo.get("config.json")?;
        let tokenizer_path = repo.get("tokenizer.json")?;
        let weights_path = repo.get("model.safetensors")?;

        let config = std::fs::read_to_string(config_path)?;
        let config: LlamaConfig = serde_json::from_str(&config)?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)?;
        let device = Device::Cpu;  // or Cuda if available

        let model = Llama::load(&weights_path, config, &device)?;

        Ok(Self { model, tokenizer, device })
    }

    pub async fn generate(&self, prompt: &str, max_tokens: usize) -> anyhow::Result<String> {
        let tokens = self.tokenizer.encode(prompt, true)?;
        let tokens = tokens.get_ids();

        // Run inference
        let mut generated = Vec::new();
        for &token in &tokens {
            generated.push(token);
        }

        for _ in 0..max_tokens {
            let input_tensor = Tensor::new(&generated, &self.device)?;
            let logits = self.model.forward(&input_tensor, generated.len() - 1)?;

            // Sample from logits (greedy for simplicity)
            let next_token = logits.argmax(1)?.get(0)?.try_into()?;
            generated.push(next_token);

            if next_token == self.tokenizer.token_to_id("</s>").unwrap_or(2) {
                break;
            }
        }

        let text = self.tokenizer.decode(&generated, true)?;
        Ok(text)
    }
}
```

## 9. UI Options

### Option A: Dioxus (React-like)

```rust
use dioxus::prelude::*;

fn App() -> Element {
    let mut zoom = use_signal(|| 1.0);
    let mut pan = use_signal(|| Point2D::new(0.0, 0.0));

    rsx! {
        div {
            width: "100%",
            height: "100%",
            onwheel: move |e| {
                let delta = e.delta_y();
                zoom.with(|z| *z *= if delta < 0.0 { 1.1 } else { 0.9 });
            },

            svg {
                g { transform: "scale({zoom}) translate({pan})",
                    // Render nodes and edges
                }
            }
        }
    }
}
```

### Option B: egui (Immediate Mode)

```rust
use egui::{Context, Pos2, Response, Sense, Ui};

pub fn graph_editor(ui: &mut Ui, graph: &mut Graph) {
    let (rect, response) = ui.allocate_exact_size(
        ui.available_size(),
        Sense::click_and_drag(),
    );

    if response.dragged() {
        // Pan logic
    }

    // Draw nodes
    for node in graph.nodes() {
        // Render node
    }

    // Draw edges
    for edge in graph.edges() {
        // Render bezier curve
    }
}
```

## 10. Project Structure

```
my-flow-engine/
├── Cargo.toml
├── host/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── plugin.rs
│       ├── graph.rs
│       └── resources.rs
├── plugin-sdk/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── macros.rs
├── wit/
│   └── plugin.wit
└── plugins/
    ├── example-plugin/
    │   ├── Cargo.toml
    │   └── src/
    │       └── lib.rs
    └── ...
```

## Key Design Decisions

1. **Component Model:** Use WASM component model for type-safe interfaces
2. **Resource Ownership:** Host owns all resources, plugins borrow
3. **Async Execution:** All plugin execution is async and non-blocking
4. **Type Safety:** WIT provides compile-time type checking across boundaries
5. **Sandboxing:** WASI limits plugin access to system resources

## Production Considerations

### Security
- Validate all plugin inputs/outputs
- Limit resource usage (memory, CPU time)
- Implement plugin timeouts
- Audit plugin code before loading

### Performance
- Cache compiled WASM modules
- Pool model instances
- Use GPU acceleration when available
- Batch similar operations

### Reliability
- Implement circuit breakers
- Graceful degradation
- Comprehensive logging
- Health monitoring

### Extensibility
- Plugin versioning
- Hot-reload support
- Schema evolution
- Backward compatibility
