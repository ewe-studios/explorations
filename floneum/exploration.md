# Floneum Exploration

## Overview

**Floneum** is a graph editor for AI workflows with a focus on community-made plugins, local AI, and safety. It implements a flow-based programming model where nodes (plugins) process and pass data through connections in a visual graph editor.

**Source Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.floneum/floneum/floneum/`

## Architecture Summary

Floneum consists of several key components:

```
floneum/
├── floneum/           # Main desktop UI application (Dioxus-based)
├── floneum-cli/       # CLI for building plugins
├── plugin/            # WebAssembly plugin host runtime
├── floneumite/        # Package index/registry client
├── rust_adapter/      # Rust bindings for plugin development
├── rust_macro/        # Procedural macros for plugin exports
├── plugins/           # Built-in plugins (37 plugins)
└── wit/               # WIT interface definitions
```

## Core Concepts

### 1. Flow-Based Programming Model

Floneum uses a directed graph where:
- **Nodes** are plugin instances that process data
- **Edges** connect outputs of one node to inputs of another
- **Data flows** through the graph as `PrimitiveValue` types

The graph execution is handled in `floneum/src/graph.rs`:
- Nodes can be marked as `running` or `queued`
- Dependencies are resolved by traversing incoming edges
- When a node completes, it triggers downstream nodes to run

### 2. WebAssembly Plugin System

Plugins are compiled to WebAssembly (WASM) and run in an isolated sandbox using **Wasmtime** with the component model.

**Key Security Features:**
- Plugins run in isolated WASI sandboxes
- Limited filesystem access (only `./sandbox` directory)
- Controlled access to host capabilities (LLMs, embeddings, browser automation)
- Resources are tracked and cleaned up via RAII

**WIT Interface (`wit/plugin.wit`):**

The WebAssembly Interface Types (WIT) define three worlds:

1. **`exports` world** - Plugin exports (`structure()` and `run()`)
2. **`plugin-world`** - Full plugin with host imports
3. **`both`** - Combined interface

**Host Capabilities Exposed to Plugins:**

```wit
interface imports {
  // Key-value storage for plugin state
  store: func(key: list<u8>, value: list<u8>);
  load: func(key: list<u8>) -> list<u8>;
  unload: func(key: list<u8>);

  // User-facing logging
  log-to-user: func(information: string);
}

interface types {
  // HTTP requests
  get-request: func(url: string, headers: list<header>) -> string;

  // Browser automation (headless Chrome)
  create-page: func(mode: browser-mode, url: string) -> page-resource;
  find-in-current-page: func(page: page-resource, selector: string) -> node-resource;
  screenshot-browser: func(page: page-resource) -> list<u8>;
  page-html: func(page: page-resource) -> string;

  // DOM manipulation
  get-element-text: func(node: node-resource) -> string;
  click-element: func(node: node-resource);
  type-into-element: func(node: node-resource, keys: string);
  get-element-outer-html: func(node: node-resource) -> string;
  find-child-of-element: func(node: node-resource, selector: string) -> node-resource;

  // Embedding models and vector databases
  create-embedding-model: func(ty: embedding-model-type) -> embedding-model-resource;
  get-embedding: func(model: embedding-model-resource, document: string) -> embedding;
  create-embedding-db: func(embeddings: list<embedding>, documents: list<string>) -> embedding-db-resource;
  find-closest-documents: func(db: embedding-db-resource, search: embedding, count: u32) -> list<string>;

  // Text generation (LLM)
  create-model: func(ty: model-type) -> text-generation-model-resource;
  infer: func(model: text-generation-model-resource, input: string, ...) -> string;
  infer-structured: func(model: text-generation-model-resource, input: string, regex: string) -> string;
}
```

### 3. Data Flow Between Nodes

**Value Types (`PrimitiveValue`):**

```rust
enum PrimitiveValue {
    Model(TextGenerationModelResource),
    EmbeddingModel(EmbeddingModelResource),
    ModelType(ModelType),
    EmbeddingModelType(EmbeddingModelType),
    Database(EmbeddingDbResource),
    Number(i64),
    Float(f64),
    Text(String),
    File(String),
    Folder(String),
    Embedding(Vec<f32>),
    Boolean(bool),
    Page(PageResource),
    Node(NodeResource),
}
```

**IO Definition Types:**

```rust
enum ValueType {
    Single(PrimitiveValueType),  // Single value
    Many(PrimitiveValueType),    // Vector of values
}

enum PrimitiveValueType {
    Number, Float, Text, File, Folder,
    Embedding, Database, Model, EmbeddingModel,
    ModelType, EmbeddingModelType, Boolean,
    Page, Node, Any
}
```

**Data Flow Process:**

1. User connects node outputs to node inputs via the visual editor
2. When a node is triggered, `set_input_nodes()` resolves incoming edge values
3. Input values are extracted and passed to the plugin's `run()` function
4. Output values are stored and propagated to connected downstream nodes
5. Downstream nodes are marked as `queued` and will run when their turn comes

### 4. Visual Programming Interface

Built with **Dioxus** (a React-like Rust UI framework):

**Key UI Components:**
- `FlowView` - Main graph canvas with pan/zoom
- `VisualGraph` - Graph state management using `petgraph::StableGraph`
- `Node` - Individual plugin instance display
- `Edge` / `Connection` - Visual connections between nodes
- `Sidebar` - Plugin browser/search

**Graph State:**
```rust
struct VisualGraphInner {
    pub graph: StableGraph<Signal<Node>, Signal<Edge>>,
    pub connections: Slab<ConnectionProps>,
    pub currently_dragging: Option<CurrentlyDragging>,
    pub pan_pos: Point2D<f32, f32>,
    pub zoom: f32,
}
```

**Interaction Handling:**
- Mouse events for panning canvas
- Node dragging with offset calculation
- Connection creation via drag-from-output/drop-on-input
- Type compatibility checking for connections

### 5. Plugin Development Model

**Rust Plugin Template:**

```rust
use floneum_rust::*;

#[export_plugin]
/// Plugin description here.
fn plugin_name(input1: Type1, input2: Type2) -> ReturnType {
    // Plugin logic
}
```

**The `#[export_plugin]` macro:**
- Parses function signature for input/output types
- Extracts doc comments for description
- Parses `### Examples` section for example code
- Generates `Guest` trait implementation
- Creates `Definition` struct with metadata

**Example from `generate_text` plugin:**
```rust
#[export_plugin]
/// Calls a large language model to generate text.
fn generate_text(model: ModelType, text: String, max_size: i64) -> String {
    if !TextGenerationModel::model_downloaded(model) {
        log_to_user("downloading model... This could take several minutes");
    }

    let session = TextGenerationModel::new(model);
    let mut response = session.infer(&text, (max_size != 0).then_some(max_size as u32), None);
    response += "\n";
    response
}
```

### 6. Package Management (floneumite)

**Floneumite** manages plugin discovery and installation:

- Searches GitHub repos with topic `floneum-v{CURRENT_BINDING_VERSION}`
- Fetches `dist/floneum.toml` for package metadata
- Downloads WASM binaries to `~/.local/share/floneum/v{version}/packages/`
- Maintains local index with 3-day cache timeout
- Supports automatic updates

**Package Structure:**
```toml
[package]
name = "floneum-example"
version = "0.1.0"
binding_version = "3"  # Must match CURRENT_BINDING_VERSION

[[packages]]
name = "example-plugin"
package_version = "0.1.0"
```

### 7. Model Support (Kalosm Integration)

Floneum integrates with **Kalosm** (a Rust ML framework) for local AI:

**Text Generation Models:**
- Llama family (7B, 13B, 70B variants)
- Mistral (7B Instruct)
- Zephyr (7B Alpha/Beta)
- Phi (1, 1.5, 2)
- Solar (10.7B)
- TinyLlama (1.1B)

**Embedding Models:**
- BERT (via Candle)

**Model Management:**
- Lazy initialization with download-on-first-use
- Progress callbacks during download
- Models are cached and reused across plugin instances
- Structured generation via regex-constrained decoding

## Sub-Projects

| Sub-Project | Purpose | Key Files |
|-------------|---------|-----------|
| `floneum` | Desktop UI application | `src/main.rs`, `src/graph.rs` |
| `plugin` | WASM host runtime | `src/host.rs`, `src/plugin.rs` |
| `floneumite` | Package registry client | `src/index.rs` |
| `rust_adapter` | Rust plugin bindings | `src/lib.rs`, `src/helpers.rs` |
| `rust_macro` | Plugin export macros | `src/lib.rs` |
| `floneum-cli` | Plugin build CLI | `src/main.rs` |

## Key Dependencies

| Dependency | Purpose |
|------------|---------|
| `wasmtime` | WebAssembly runtime with component model |
| `wasmtime-wasi` | WASI implementation for sandboxing |
| `wit-component` | WIT interface processing |
| `dioxus` | Cross-platform UI framework |
| `petgraph` | Graph data structures |
| `kalosm` | Local ML models (LLM, embeddings) |
| `headless_chrome` | Browser automation |
| `heed` | Embedded vector database (LMDB-based) |
| `candle` | ML inference engine (via Kalosm) |

## Security Considerations

1. **Sandboxed Execution:** Plugins run in WASI sandbox with limited filesystem
2. **Resource Isolation:** Each plugin instance has its own resource table
3. **Type Safety:** WIT ensures type-safe communication between host and plugins
4. **Controlled Capabilities:** Host decides which capabilities to expose

## Performance Characteristics

- **Plugin Loading:** WASM modules are compiled once, instantiated per-node
- **Model Loading:** Models use lazy initialization with shared caching
- **Graph Execution:** Async execution with dependency tracking
- **UI Responsiveness:** Heavy operations run in background tasks

## Related Projects in Source Tree

The `src.floneum` directory also contains:
- **Candle** - Hugging Face's ML framework (upstream mirror)
- **Kalosm interfaces** - ML abstraction layer
- Various AI model implementations
