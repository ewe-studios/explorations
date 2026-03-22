---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/wasmtime/
explored_at: 2026-03-22
revised_at: 2026-03-22
workspace: wasmtime-workspace
---

# Rust Revision: Wasmtime Sub-Projects

## Overview

This document consolidates the Wasmtime sub-project explorations into implementation guidance for embedding WebAssembly in Rust applications. The revision covers runtime embedding, WASI integration, component model, and tooling.

## Sub-Projects Covered

| Sub-Project | Purpose | Implementation Priority |
|-------------|---------|------------------------|
| wasmtime-runtime | WASM execution | Critical |
| wasi | System interface | Critical |
| wit-bindgen | Component bindings | High |
| wasm-tools | Binary tooling | Medium |

## Workspace Structure

```
wasmtime-workspace/
├── runtime/
│   ├── basic-embed/            # Basic embedding
│   ├── wasi-embed/             # WASI integration
│   ├── instance-pooling/       # Performance patterns
│   └── async-runtime/          # Async execution
├── components/
│   ├── component-host/         # Component model host
│   ├── wit-component/          # WIT interface definitions
│   └── component-bindings/     # wit-bindgen generated
├── tooling/
│   ├── wasm-analyze/           # Binary analysis
│   └── wasm-transform/         # Transformations
└── examples/
    ├── plugin-system/          # Plugin architecture
    ├── sandboxed-exec/         # Sandboxed execution
    └── multi-tenant/           # Multi-tenant hosting
```

## Runtime Implementation

### Basic Embedding

```rust
// runtime/basic-embed/src/main.rs
use anyhow::Result;
use wasmtime::{Engine, Module, Store, Linker};

fn main() -> Result<()> {
    // Create engine with configuration
    let mut config = wasmtime::Config::new();
    config.wasm_reference_types(true);
    config.wasm_bulk_memory(true);

    let engine = Engine::new(&config)?;

    // Create store
    let mut store = Store::new(&engine, ());

    // Load module
    let module = Module::from_file(&engine, "module.wasm")?;

    // Create linker and add imports
    let mut linker = Linker::new(&engine);

    // Define host function
    linker.func_wrap("", "host_log", |caller: wasmtime::Caller<()>, ptr: i32, len: i32| {
        let memory = caller.get_memory("memory").unwrap();
        let mut buf = vec![0u8; len as usize];
        memory.read(&mut store, ptr as usize, &mut buf)?;
        println!("WASM: {}", String::from_utf8_lossy(&buf));
        Ok(())
    })?;

    // Instantiate
    let instance = linker.instantiate(&mut store, &module)?;

    // Call exported function
    let greet = instance.get_typed_func::<i32, ()>(&mut store, "greet")?;
    greet.call(&mut store, 0)?;

    Ok(())
}
```

### WASI Integration

```rust
// runtime/wasi-embed/src/main.rs
use anyhow::Result;
use wasmtime::{Engine, Module, Store, Linker};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use std::fs::File;
use std::io::Read;

fn main() -> Result<()> {
    let engine = Engine::default();

    // Create WASI context with capability-based security
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .env("MY_VAR", "value")?
        .preopen_dir("/host/project", "/project")?  // Only this dir accessible
        .build();

    let mut store = Store::new(&engine, wasi);

    // Create linker and add WASI
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

    // Load WASI module
    let module = Module::from_file(&engine, "wasi_module.wasm")?;

    // Instantiate
    let instance = linker.instantiate(&mut store, &module)?;

    // Call _start (WASI entry point)
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())?;

    Ok(())
}
```

### Instance Pooling

```rust
// runtime/instance-pooling/src/main.rs
use wasmtime::{Engine, Module, Store, Linker, InstanceAllocationStrategy, PoolingAllocationConfig};
use anyhow::Result;

fn main() -> Result<()> {
    let mut config = wasmtime::Config::new();

    // Configure instance pooling
    let mut pooling_config = PoolingAllocationConfig::default();
    pooling_config
        .module_limits(wasmtime::ModuleLimits {
            memory_pages: 1024,      // 64MB max
            tables: 1,
            table_elements: 10000,
            memories: 1,
            ..Default::default()
        })
        .instance_limits(wasmtime::InstanceLimits {
            count: 100,              // Max 100 concurrent instances
            ..Default::default()
        });

    config.allocation_strategy(InstanceAllocationStrategy::Pooling(pooling_config));

    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, "plugin.wasm")?;

    // Create pool of instances
    let mut instances = Vec::new();

    for i in 0..10 {
        let mut store = Store::new(&engine, i);
        let linker = Linker::new(&engine);
        let instance = linker.instantiate(&mut store, &module)?;
        instances.push((store, instance));
    }

    // Use instances concurrently
    for (mut store, instance) in &mut instances {
        let compute = instance.get_typed_func::<i32, i32>(&mut store, "compute")?;
        let result = compute.call(&mut store, 42)?;
        println!("Instance {} computed: {}", store.data(), result);
    }

    Ok(())
}
```

### Async Runtime

```rust
// runtime/async-runtime/src/main.rs
use wasmtime::{Engine, Module, Store, Linker, Config};
use anyhow::Result;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = Config::new();
    config.async_support(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let module = Module::from_file(&engine, "async_module.wasm")?;

    let linker = Linker::new(&engine);
    let instance = linker.instantiate_async(&mut store, &module).await?;

    let async_func = instance.get_typed_func::<i32, i32, _>(&mut store, "async_compute")?;

    // Call async function
    let result = async_func.call_async(&mut store, 42).await?;
    println!("Async result: {}", result);

    Ok(())
}
```

## Component Model

### Host Implementation

```rust
// components/component-host/src/main.rs
use wasmtime::{Engine, Store, Linker};
use wasmtime::component::{Component, Linker as ComponentLinker};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = wasmtime::Config::new();
    config.wasm_component_model(true);
    config.async_support(true);

    let engine = Engine::new(&config)?;
    let mut store = Store::new(&engine, ());

    let mut linker = ComponentLinker::new(&engine);

    // Import host function into component
    linker.root_import_func("", "log", |_, msg: String| {
        println!("Component logged: {}", msg);
        Ok(())
    })?;

    // Load component
    let component = Component::from_file(&engine, "component.wasm")?;

    // Instantiate
    let (instance, _) = wasmtime_wasi::bindings::Command::instantiate_async(
        &mut store,
        &component,
        &linker,
    ).await?;

    // Run component
    instance.run(&mut store).await?;

    Ok(())
}
```

### WIT Interface Definition

```wit
// components/wit-component/wit/host.wit
package myapp:host;

interface logging {
    log: func(message: string);
    error: func(message: string);
}

interface config {
    get: func(key: string) -> option<string>;
    set: func(key: string, value: string);
}

world plugin {
    import logging;
    import config;

    export init;
    export run;
    export handle-request: func(request: string) -> string;
}
```

### Generated Bindings

```rust
// components/component-bindings/src/lib.rs
// Generated by wit-bindgen

mod bindings {
    wit_bindgen::generate!({
        world: "plugin",
        path: "../wit",
    });
}

use bindings::exports::myapp::host::plugin::{Guest, RunResponse};
use bindings::myapp::host::{logging, config};

pub struct MyPlugin;

impl Guest for MyPlugin {
    fn init() {
        logging::log("Plugin initialized");
    }

    fn run() -> RunResponse {
        let config_value = config::get("my_key");
        RunResponse {
            status: "success".to_string(),
            data: format!("Config: {:?}", config_value),
        }
    }

    fn handle_request(request: String) -> String {
        logging::log(&format!("Handling request: {}", request));
        format!("Processed: {}", request)
    }
}
```

## Tooling Integration

### Binary Analysis

```rust
// tooling/wasm-analyze/src/main.rs
use wasmparser::{Parser, Payload, BinaryReaderError};
use std::fs;

fn main() -> Result<(), BinaryReaderError> {
    let wasm_bytes = fs::read("module.wasm")?;

    let mut parser = Parser::new(0);
    let mut offset = 0;

    loop {
        let (payload, consumed) = match parser.parse(&wasm_bytes[offset..], true)? {
            Some((payload, consumed)) => (payload, consumed),
            None => break,
        };

        match payload {
            Payload::Version { num, range } => {
                println!("WASM version: {}", num);
            }
            Payload::TypeSection(section) => {
                println!("Type section: {} types", section.get_count());
            }
            Payload::ImportSection(section) => {
                println!("Import section: {} imports", section.get_count());
            }
            Payload::FunctionSection(section) => {
                println!("Function section: {} functions", section.get_count());
            }
            Payload::ExportSection(section) => {
                println!("Export section: {} exports", section.get_count());
            }
            Payload::CodeSectionEntry(size) => {
                println!("Code entry: {} bytes", size);
            }
            _ => {}
        }

        offset += consumed;
        parser = Parser::new(offset);
    }

    Ok(())
}
```

### Module Transformation

```rust
// tooling/wasm-transform/src/main.rs
use walrus::{Module, ModuleConfig};
use std::collections::HashSet;

fn main() -> Result<(), walrus::Error> {
    // Parse module
    let mut module = ModuleConfig::new()
        .generate_dwarf(true)
        .parse("input.wasm")?;

    // Remove unused functions
    remove_unused_functions(&mut module);

    // Rename functions
    demangle_rust_functions(&mut module);

    // Inject logging
    inject_entry_logging(&mut module);

    // Emit transformed module
    let mut output = Vec::new();
    module.emit_wasm(&mut output)?;
    std::fs::write("output.wasm", output)?;

    Ok(())
}

fn remove_unused_functions(module: &mut Module) {
    let mut used = HashSet::new();

    // Mark exported functions
    for export in module.exports.iter() {
        if let walrus::ExportItem::Function(id) = export.item {
            used.insert(id);
        }
    }

    // Mark functions called from other functions
    for func in module.functions.iter() {
        for (_block, instr) in func.body.iter() {
            if let walrus::ir::Instr::Call { func: id } = instr {
                used.insert(*id);
            }
        }
    }

    // Remove unused
    let to_remove: Vec<_> = module
        .functions
        .iter()
        .filter(|f| !used.contains(&f.id()))
        .map(|f| f.id())
        .collect();

    for id in to_remove {
        module.functions.delete(id);
    }
}

fn demangle_rust_functions(module: &mut Module) {
    for func in module.functions.iter_mut() {
        if let Some(name) = &func.name {
            if let Ok(demangled) = rustc_demangle::try_demangle(name) {
                func.name = Some(demangled.to_string());
            }
        }
    }
}

fn inject_entry_logging(module: &mut Module) {
    let log_import = module.funcs.add_import("env", "log", module.types.add(&[], &[]));

    for func in module.functions.iter_mut() {
        if func.kind.is_import() {
            continue;
        }

        let body = func.body_mut().unwrap();
        let mut new_body = Vec::new();

        if let Some(name) = &func.name {
            // Add log at entry
            new_body.push(walrus::ir::Instr::MemoryString(name.clone()));
            new_body.push(walrus::ir::Instr::Call { func: log_import });
        }

        new_body.extend(body.iter().cloned());
        *body = new_body;
    }
}
```

## Plugin System Example

```rust
// examples/plugin-system/src/main.rs
use wasmtime::{Engine, Module, Store, Linker, Instance};
use anyhow::Result;

struct PluginHost {
    engine: Engine,
    linker: Linker<PluginContext>,
}

struct PluginContext {
    plugin_id: usize,
    data: String,
}

impl PluginHost {
    fn new() -> Result<Self> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);

        // Define plugin API
        linker.func_wrap("host", "get_data", |caller: wasmtime::Caller<PluginContext>| {
            let data = &caller.data().data;
            Ok(data.as_str().as_ptr() as i64)
        })?;

        linker.func_wrap("host", "log", |caller: wasmtime::Caller<PluginContext>, ptr: i32, len: i32| {
            let ctx = caller.data();
            println!("Plugin {} logged something", ctx.plugin_id);
            Ok(())
        })?;

        Ok(Self { engine, linker })
    }

    fn load_plugin(&self, plugin_id: usize, wasm_path: &str) -> Result<Instance> {
        let module = Module::from_file(&self.engine, wasm_path)?;
        let mut store = Store::new(&self.engine, PluginContext {
            plugin_id,
            data: format!("Plugin {} data", plugin_id),
        });

        let instance = self.linker.instantiate(&mut store, &module)?;
        Ok(instance)
    }
}

fn main() -> Result<()> {
    let host = PluginHost::new()?;

    // Load plugins
    let plugin1 = host.load_plugin(1, "plugins/plugin1.wasm")?;
    let plugin2 = host.load_plugin(2, "plugins/plugin2.wasm")?;

    // Run plugin functions
    // ...

    Ok(())
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use wasmtime::*;

    #[test]
    fn test_basic_execution() -> Result<()> {
        let engine = Engine::default();
        let mut store = Store::new(&engine, ());

        let module = Module::new(&engine, r#"
            (module
                (func (export "add") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add
                )
            )
        "#)?;

        let instance = Instance::new(&mut store, &module, &[])?;
        let add = instance.get_typed_func::<(i32, i32), i32>(&mut store, "add")?;

        assert_eq!(add.call(&mut store, (2, 3))?, 5);

        Ok(())
    }

    #[test]
    fn test_wasi_execution() -> Result<()> {
        use wasmtime_wasi::WasiCtxBuilder;

        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();

        let engine = Engine::default();
        let mut store = Store::new(&engine, wasi);

        let module = Module::from_file(&engine, "tests/wasi_test.wasm")?;

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

        let instance = linker.instantiate(&mut store, &module)?;
        let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;

        start.call(&mut store, ())?;

        Ok(())
    }
}
```

## Related Documents

- [Wasmtime Runtime](./wasmtime-runtime-exploration.md) - Runtime details
- [WASI](./wasi-exploration.md) - System interface
- [wit-bindgen](./wit-bindgen-exploration.md) - Component bindings
- [wasm-tools](./wasm-tools-exploration.md) - Binary tooling

## Sources

- Wasmtime Book: https://bytecodealliance.github.io/wasmtime/
- Component Model: https://github.com/WebAssembly/component-model
- WASI: https://wasi.dev/
