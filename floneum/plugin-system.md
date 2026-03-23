# Floneum Plugin System Deep Dive

## Overview

The plugin system is the heart of Floneum. It provides a secure, isolated environment for running community-created plugins using WebAssembly with the component model.

**Source Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.floneum/floneum/floneum/plugin/`

## Architecture

### Core Components

```
plugin/
├── src/
│   ├── lib.rs          # WIT bindgen setup
│   ├── host.rs         # Host implementation (WASI, linking)
│   ├── plugin.rs       # Plugin loading and instance management
│   ├── resource.rs     # Typed resource storage system
│   ├── llm.rs          # Text generation model handling
│   ├── embedding.rs    # Embedding model handling
│   ├── embedding_db.rs # Vector database implementation
│   ├── node.rs         # Browser DOM node handling
│   ├── page.rs         # Browser page/tab handling
│   └── proxies.rs      # Generated proxy code (25KB)
└── wit/
    └── plugin.wit      # WIT interface definitions
```

### Plugin Loading Flow

```rust
// 1. Load plugin from path
pub fn load_plugin(path: &Path, resources: ResourceStorage) -> Plugin {
    let module = PackageIndexEntry::new(path.into(), None, None);
    load_plugin_from_source(module, resources)
}

// 2. Plugin structure (lazy loading)
pub struct Plugin {
    shared: SharedPluginState,
    source: PackageIndexEntry,
    component: OnceCell<Component>,      // Lazy-loaded WASM component
    definition: OnceCell<Definition>,    // Plugin metadata
    metadata: OnceCell<PluginMetadata>,
}

// 3. Component loading (on-demand compilation)
async fn component(&self) -> anyhow::Result<&Component> {
    let bytes = self.source.wasm_bytes().await?;

    // Convert WASM module to component using WASI adapter
    let component = ComponentEncoder::default()
        .module(bytes.as_slice())?
        .validate(true)
        .adapter(
            "wasi_snapshot_preview1",
            include_bytes!("../wasi_snapshot_preview1.wasm"),
        )
        .unwrap()
        .encode()?;

    Component::from_binary(&ENGINE, &component)
}
```

### Plugin Instance Creation

When a user adds a plugin node to the graph:

```rust
pub async fn instance(&self) -> anyhow::Result<PluginInstance> {
    let (mut store, world) = self.create_world().await?;
    let definition = self.definition().await?;

    // Create channels for async communication
    let (input_sender, mut input_receiver) =
        broadcast::channel::<Vec<Vec<PrimitiveValue>>>(100);
    let (output_sender, output_receiver) = broadcast::channel(100);

    // Spawn async task to handle plugin execution
    tokio::spawn(async move {
        loop {
            let Ok(inputs) = input_receiver.recv().await else {
                break;
            };
            let outputs = world.interface0.call_run(&mut store, &inputs).await;
            if output_sender.send(Arc::new(outputs)).is_err() {
                break;
            }
        }
    });

    Ok(PluginInstance {
        source: self.source.clone(),
        sender: input_sender,
        receiver: output_receiver,
        metadata: definition.clone(),
        shared_plugin_state: self.shared.clone(),
    })
}
```

### Running a Plugin

```rust
pub fn run(
    &self,
    inputs: Vec<Vec<PrimitiveValue>>,
) -> impl Future<Output = Option<Arc<Result<Vec<Vec<PrimitiveValue>>, Error>>>> + 'static {
    let sender = self.sender.clone();
    let mut receiver = self.receiver.resubscribe();
    async move {
        let _ = sender.send(inputs);
        receiver.recv().await.ok()
    }
}
```

## Host Implementation

### Wasmtime Engine Configuration

```rust
pub(crate) static ENGINE: Lazy<Engine> = Lazy::new(|| {
    let mut config = Config::new();
    config.wasm_component_model(true).async_support(true);
    Engine::new(&config).unwrap()
});
```

### Linker Setup

```rust
pub(crate) static LINKER: Lazy<Linker<State>> = Lazy::new(|| {
    let mut linker = Linker::new(&ENGINE);
    Both::add_to_linker(&mut linker, |x| x).unwrap();
    Command::add_to_linker(&mut linker, |x| x).unwrap();
    linker
});
```

### Host State

```rust
pub struct State {
    pub(crate) shared: SharedPluginState,
    pub(crate) plugin_state: HashMap<Vec<u8>, Vec<u8>>,  // Plugin KV store
    pub(crate) table: ResourceTable,
    pub(crate) ctx: WasiCtx,
}

pub struct SharedPluginState {
    pub(crate) logs: Arc<RwLock<Vec<String>>>,
    pub(crate) resources: ResourceStorage,
}
```

### WASI Sandbox Configuration

```rust
impl State {
    pub fn new(shared: SharedPluginState) -> Self {
        let sandbox = Path::new("./sandbox");
        std::fs::create_dir_all(sandbox).unwrap();

        let mut ctx = WasiCtxBuilder::new();
        let ctx_builder = ctx
            .inherit_stderr()
            .inherit_stdin()
            .inherit_stdio()
            .inherit_stdout()
            .preopened_dir(sandbox, "./", DirPerms::all(), FilePerms::all())
            .unwrap();

        let table = ResourceTable::new();
        let ctx = ctx_builder.build();

        State {
            plugin_state: Default::default(),
            shared,
            table,
            ctx,
        }
    }
}
```

## Resource Management System

### Resource Storage

The resource system provides type-safe access to shared resources (models, browser pages, etc.):

```rust
type ResourceMap = Arc<RwLock<HashMap<TypeId, Slab<Box<dyn Any + Send + Sync>>>>>;

#[derive(Default, Clone)]
pub struct ResourceStorage {
    map: ResourceMap,
}

impl ResourceStorage {
    pub(crate) fn insert<T: Send + Sync + 'static>(&self, item: T) -> Resource<T> {
        let ty_id = TypeId::of::<T>();
        let mut binding = self.map.write();
        let slab = binding.entry(ty_id).or_default();
        let id = slab.insert(Box::new(item));
        Resource {
            index: id,
            owned: true,
            phantom: PhantomData,
        }
    }

    pub(crate) fn get<T: Send + Sync + 'static>(
        &self,
        key: Resource<T>,
    ) -> Option<MappedRwLockReadGuard<'_, T>> {
        RwLockReadGuard::try_map(self.map.read(), |r| {
            r.get(&TypeId::of::<T>())
                .and_then(|slab| slab.get(key.index))
                .and_then(|any| any.downcast_ref())
        }).ok()
    }
}
```

### Resource Types

| Resource Type | Description |
|--------------|-------------|
| `LazyTextGenerationModel` | LLM (Llama, Phi, Mistral, etc.) |
| `LazyTextEmbeddingModel` | BERT embedding model |
| `VectorDBWithDocuments` | Vector database with document storage |
| `Arc<Tab>` | Headless Chrome browser tab |
| `AnyNodeRef` | DOM element reference |

## Host Capabilities Implementation

### Text Generation (LLM)

```rust
pub(crate) enum LazyTextGenerationModel {
    Uninitialized(main::types::ModelType),
    Initialized(ConcreteTextGenerationModel),
}

pub(crate) enum ConcreteTextGenerationModel {
    Llama(Arc<Llama>),
    Phi(Arc<Phi>),
}

// Lazy initialization with download handler
async fn initialize(
    &self,
) -> impl Future<Output = anyhow::Result<ConcreteTextGenerationModel>> {
    let model_type = match self {
        LazyTextGenerationModel::Uninitialized(ty) => Some(*ty),
        _ => None,
    };

    async move {
        let model_type = model_type.ok_or(...)?;
        let builder = model_type.llm_builder();

        match builder {
            LlmBuilder::Llama(builder) => {
                let model = builder.build_with_loading_handler(progress).await?;
                Ok(ConcreteTextGenerationModel::Llama(Arc::new(model)))
            }
            LlmBuilder::Phi(builder) => {
                let model = builder.build_with_loading_handler(progress).await?;
                Ok(ConcreteTextGenerationModel::Phi(Arc::new(model)))
            }
        }
    }
}
```

### Inference

```rust
pub(crate) async fn impl_infer(
    &self,
    self_: TextGenerationModelResource,
    input: String,
    max_tokens: Option<u32>,
    stop_on: Option<String>,
) -> wasmtime::Result<String> {
    let index = self_.into();
    let model = self.initialize_model(index).await?;

    match model {
        ConcreteTextGenerationModel::Llama(model) => Ok(model
            .generate_text(&input)
            .with_max_length(max_tokens.unwrap_or(u32::MAX))
            .with_stop_on(stop_on)
            .await?),
        ConcreteTextGenerationModel::Phi(model) => Ok(model
            .generate_text(&input)
            .with_max_length(max_tokens.unwrap_or(u32::MAX))
            .with_stop_on(stop_on)
            .await?),
    }
}
```

### Structured Generation

```rust
pub(crate) async fn impl_infer_structured(
    &self,
    self_: TextGenerationModelResource,
    input: String,
    regex: String,
) -> wasmtime::Result<String> {
    let structure = RegexParser::new(&regex)?;
    let index = self_.into();
    let model = self.initialize_model(index).await?;

    match model {
        ConcreteTextGenerationModel::Llama(model) => {
            Ok(model.stream_structured_text(&input, structure).await?)
        }
        ConcreteTextGenerationModel::Phi(model) => {
            Ok(model.stream_structured_text(&input, structure).await?)
        }
    }
}
```

### Vector Database

```rust
pub(crate) struct VectorDBWithDocuments {
    db: Lazy<Result<VectorDB<UnknownVectorSpace>, Arc<heed::Error>>>,
    documents: Vec<Option<Document>>,
}

impl VectorDBWithDocuments {
    pub fn add_embedding(
        &mut self,
        embedding: Embedding,
        document: Document,
    ) -> anyhow::Result<()> {
        let id = self.db.as_ref().map_err(Clone::clone)?
            .add_embedding(embedding.vector.into())?;

        if id.0 as usize >= self.documents.len() {
            self.documents.resize(id.0 as usize + 1, None);
        }
        self.documents[id.0 as usize] = Some(document);
        Ok(())
    }

    pub fn get_closest(
        &self,
        embedding: Embedding,
        count: usize,
    ) -> anyhow::Result<Vec<(f32, &Document)>> {
        let results = self.db.as_ref().map_err(Clone::clone)?
            .get_closest(embedding.vector.into(), count)?;

        Ok(results.into_iter()
            .filter_map(|result| {
                let id = result.value;
                let distance = result.distance;
                let document = self.documents[id.0 as usize].as_ref()?;
                Some((distance, document))
            })
            .collect())
    }
}
```

### Browser Automation

```rust
// Find element in page
pub(crate) async fn impl_find_in_current_page(
    &self,
    self_: PageResource,
    query: String,
) -> wasmtime::Result<NodeResource> {
    let (node_id, page_id) = {
        let index = self_.into();
        let node = self.get(index).ok_or(...)?;
        (node.node_id, node.page_id)
    };

    let node_id = {
        let page = Resource::from_index_borrowed(page_id);
        let tab = self.get(page).ok_or(...)?;
        let node = headless_chrome::Element::new(&tab, node_id)?;
        let child = node.find_element(&query)?;
        child.node_id
    };

    let child = self.insert(AnyNodeRef { page_id, node_id });
    Ok(NodeResource { id: child.index() as u64, owned: true })
}
```

## WIT Interface Definitions

Full interface from `wit/plugin.wit`:

```wit
package plugins:main;

interface imports {
  store: func(key: list<u8>, value: list<u8>);
  load: func(key: list<u8>) -> list<u8>;
  unload: func(key: list<u8>);
  log-to-user: func(information: string);
}

interface types {
  // HTTP, browser, models, embeddings defined here
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

## Plugin Development Interface

### Rust Adapter

The `rust_adapter` provides ergonomic Rust bindings:

```rust
// lib.rs
pub use floneum_rust_macro::export_plugin;
mod helpers;
pub use helpers::*;

wit_bindgen::generate!({
    path: "../wit",
    world: "plugin-world",
    pub_export_macro: true,
    default_bindings_module: "::floneum_rust",
});
```

### Helper Types

The adapter provides wrapper types with RAII resource management:

```rust
pub struct TextGenerationModel {
    model: TextGenerationModelResource,
}

impl TextGenerationModel {
    pub fn new(model: ModelType) -> Self {
        let model = create_model(model);
        Self { model }
    }

    pub fn infer(&self, input: &str, max_tokens: Option<u32>, stop_on: Option<&str>) -> String {
        infer(self.model, input, max_tokens, stop_on)
    }
}

impl Drop for TextGenerationModel {
    fn drop(&mut self) {
        drop_model(self.model);
    }
}
```

### Procedural Macro

The `export_plugin` macro:
1. Parses function signature for types
2. Extracts doc comments as description
3. Parses examples from doc comments
4. Generates `Guest` trait implementation
5. Creates `Definition` struct

```rust
#[proc_macro_attribute]
pub fn export_plugin(args: TokenStream, input: TokenStream) -> TokenStream {
    // Parse function
    let mut input = parse_macro_input!(input as ItemFn);

    // Extract description from doc comments
    let mut description = String::new();
    for attr in &input.attrs {
        if attr.path().is_ident("doc") {
            // Extract doc comment content
        }
    }

    // Parse input/output types
    // Generate Guest implementation

    TokenStream::from(quote! {
        ::floneum_rust::export!(Plugin);

        #input

        pub struct Plugin;

        impl Guest for Plugin {
            fn structure() -> Definition { /* ... */ }
            fn run(input: Vec<Vec<PrimitiveValue>>) -> Vec<Vec<PrimitiveValue>> { /* ... */ }
        }
    })
}
```

## Security Model

### Isolation Guarantees

1. **Memory Isolation:** Each plugin runs in separate WASM memory
2. **Filesystem Access:** Limited to `./sandbox` directory
3. **Capability-Based Security:** Host controls which imports are available
4. **Resource Tracking:** All resources are owned by host, plugins hold borrowed references

### Resource Cleanup

Resources are automatically cleaned up via:
- RAII Drop implementations for wrapper types
- Reference counting for shared resources
- Explicit `drop_*` host functions for WASM resources

## Performance Considerations

### Optimization Strategies

1. **Lazy Model Loading:** Models only load when first used
2. **Model Caching:** Shared models across plugin instances
3. **Async Execution:** Non-blocking plugin execution
4. **WASM Caching:** Compiled modules cached per plugin type

### Memory Management

- Resources stored in Slab for O(1) allocation/deallocation
- Type-erased storage with downcast for type safety
- Reference counting for shared model instances

## Built-in Plugins

The `plugins/` directory contains 37 built-in plugins:

| Category | Plugins |
|----------|---------|
| LLM | `generate_text`, `generate_structured_text` |
| Embeddings | `embedding`, `embedding_db`, `add_embedding` |
| Logic | `if_statement`, `and`, `or`, `not`, `equals`, `contains` |
| Math | `add`, `subtract`, `multiply`, `divide`, `power`, `calculate` |
| List | `new_list`, `add_to_list`, `join`, `split`, `slice`, `length` |
| Browser | `find_node`, `click_node`, `type_in_node`, `navigate_to`, `node_text` |
| File | `read_from_file`, `write_to_file` |
| Web | `get_article`, `read_rss`, `search`, `search_engine` |
| Format | `format`, `number`, `string` |
