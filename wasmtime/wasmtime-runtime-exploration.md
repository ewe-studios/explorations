---
name: Wasmtime
description: Production-ready WebAssembly runtime implementing the WASI standard
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/wasmtime/
---

# Wasmtime - WebAssembly Runtime

## Overview

Wasmtime is a **standalone WebAssembly runtime** designed for embedding WebAssembly in applications or using it as a CLI tool. It's part of the Bytecode Alliance and implements the latest WebAssembly and WASI standards.

Key features:
- **Standalone runtime** - No browser required
- **WASI support** - Full system interface implementation
- **Multiple backends** - Cranelift (default), Winch, and external support
- **Instance pooling** - Efficient resource reuse
- **Fuel-based metering** - Prevent infinite loops and DoS
- **Async support** - Native async/await integration
- **Component model** - Early support for WASM components

## Directory Structure

```
wasmtime/
├── src/
│   ├── lib.rs              # Main library entry
│   ├── engine.rs           # Compilation engine
│   ├── module.rs           # Module representation
│   ├── instance.rs         # Instance management
│   ├── store.rs            # Store for GC roots
│   ├── func.rs             # Function definitions
│   ├── memory.rs           # Linear memory
│   ├── table.rs            # Function/data tables
│   ├── externals.rs        # External imports/exports
│   ├── linker.rs           # Module linking
│   ├── component/          # Component model support
│   ├── runtime/            # Runtime internals
│   └── config.rs           # Configuration options
├── Cargo.toml
├── README.md
└── examples/
    └── *.rs                # Example embeddings
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Wasmtime Embedding                           │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Host Application (Rust/C/C++/etc.)                     │   │
│  │                                                          │   │
│  │  Engine  ──────────────────────────────────────────┐    │   │
│  │    │                                               │    │   │
│  │    ▼                                               │    │   │
│  │  Config (optimization, features, backends)         │    │   │
│  └────┼───────────────────────────────────────────────┘    │   │
│       │                                                     │   │
│       ▼                                                     │   │
│  ┌─────────────────┐  ┌─────────────────┐                  │   │
│  │    Module       │  │    Linker       │                  │   │
│  │  (compiled      │  │  (name          │                  │   │
│  │   bytecode)     │  │   resolution)   │                  │   │
│  └────────┬────────┘  └────────┬────────┘                  │   │
│           │                    │                            │   │
│           └────────┬───────────┘                            │   │
│                    │                                        │   │
│                    ▼                                        │   │
│           ┌─────────────────┐                              │   │
│           │     Store       │                              │   │
│           │  (GC roots,     │                              │   │
│           │   fuel, limits) │                              │   │
│           └────────┬────────┘                              │   │
│                    │                                        │   │
│                    ▼                                        │   │
│           ┌─────────────────┐                              │   │
│           │    Instance     │                              │   │
│           │  (exports,      │                              │   │
│           │   memory)       │                              │   │
│           └─────────────────┘                              │   │
└─────────────────────────────────────────────────────────────────┘
```

## Core Concepts

### Engine

The `Engine` is the global compilation and configuration context:

```rust
use wasmtime::{Engine, Config};

let mut config = Config::new();
config.strategy(wasmtime::Strategy::Cranelift);
config.cranelift_opt_level(wasmtime::OptLevel::Speed);
config.wasm_reference_types(true);
config.wasm_simd(true);

let engine = Engine::new(&config)?;
```

### Module

A `Module` represents compiled WebAssembly:

```rust
use wasmtime::{Module, Engine};

// Compile from bytes
let engine = Engine::default();
let module = Module::new(&engine, r#"
    (module
        (func (export "add") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.add
        )
    )
"#)?;

// Or load from file
let module = Module::from_file(&engine, "module.wasm")?;

// Serialize compiled module
let bytes = module.serialize()?;
let module = unsafe { Module::deserialize(&engine, bytes)? };
```

### Store

A `Store` holds GC roots and instance data:

```rust
use wasmtime::{Store, Engine};

let engine = Engine::default();

// Store with basic data
let mut store = Store::new(&engine, 42);

// Access store data
println!("Store data: {}", *store.data());
*store.data_mut() = 100;

// Store with complex state
struct HostState {
    memory_limit: usize,
    stdout: Stdout,
}

let state = HostState {
    memory_limit: 100_000,
    stdout: std::io::stdout(),
};

let mut store = Store::new(&engine, state);
```

### Instance

An `Instance` is a running module:

```rust
use wasmtime::{Instance, Module, Store, Func};

let engine = Engine::default();
let mut store = Store::new(&engine, ());
let module = Module::new(&engine, wasm_bytes)?;

// Create instance
let instance = Instance::new(&mut store, &module, &[])?;

// Get exported function
let add = instance.get_func(&mut store, "add")
    .unwrap()
    .typed::<(i32, i32), i32>(&store)?;

// Call function
let result = add.call(&mut store, (2, 3))?;
assert_eq!(result, 5);
```

### Linker

The `Linker` resolves imports by name:

```rust
use wasmtime::{Linker, Module, Instance, Func};

let engine = Engine::default();
let mut store = Store::new(&engine, ());
let module = Module::new(&engine, wasm_bytes)?;

// Create linker
let mut linker = Linker::new(&engine);

// Define host function
linker.func_wrap("", "host_log", |caller: Caller<'_, ()>, val: i32| {
    println!("Host received: {}", val);
    Ok(())
})?;

// Link module
let instance = linker.instantiate(&mut store, &module)?;
```

## WASI Support

### WASI Configuration

```rust
use wasmtime_wasi::{WasiCtxBuilder, WasiCtx};
use wasmtime::{Store, Module, Linker};

// Build WASI context
let wasi = WasiCtxBuilder::new()
    .inherit_stdio()
    .inherit_args()?
    .env("MY_VAR", "value")?
    .preopen_dir("/host/path", "/guest/path")?
    .build();

let mut store = Store::new(&engine, wasi);

// Link WASI functions
let mut linker = Linker::new(&engine);
wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

// Instantiate WASI module
let module = Module::new(&engine, wasi_module_bytes)?;
let instance = linker.instantiate(&mut store, &module)?;
```

### Filesystem Sandboxing

```rust
use wasmtime_wasi::{WasiCtxBuilder, DirPerms, FilePerms};

let wasi = WasiCtxBuilder::new()
    // Read-only directory
    .preopen_dir_with_perms(
        "/host/data",
        "/data",
        DirPerms::READ,
        FilePerms::READ,
    )?
    // Writable directory
    .preopen_dir_with_perms(
        "/host/output",
        "/output",
        DirPerms::READ_WRITE,
        FilePerms::READ_WRITE,
    )?
    .build();
```

## Advanced Features

### Instance Pooling

```rust
use wasmtime::{Config, Engine, Module, InstancePoolingStrategy};

let mut config = Config::new();
config.strategy(wasmtime::Strategy::Cranelift);

// Enable instance pooling
config.instance_preimage_limit(1000);

let engine = Engine::new(&config)?;

// Pooling reduces instantiation overhead
let module = Module::new(&engine, wasm_bytes)?;

// Create pooled instances
for _ in 0..100 {
    let instance = Instance::new(&mut store, &module, &[])?;
    // Instance created from pool
}
```

### Fuel-based Metering

```rust
use wasmtime::{Config, Store, FuelConsumable};

let mut config = Config::new();
config.consume_fuel(true);

let engine = Engine::new(&config)?;
let mut store = Store::new(&engine, ());

// Set initial fuel
store.set_fuel(10_000)?;

// Execute with fuel limit
let result = func.call(&mut store, args);

// Check remaining fuel
let remaining = store.get_fuel()?;
println!("Fuel consumed: {}", 10_000 - remaining);

// Handle fuel exhaustion
if let Err(e) = result {
    if e.to_string().contains("all fuel consumed") {
        println!("Execution limit reached");
    }
}
```

### Async Support

```rust
use wasmtime::{Config, Engine, Store};

let mut config = Config::new();
config.async_support(true);

let engine = Engine::new(&config)?;

// Create async store
let mut store = Store::new(&engine, ());

// Call function asynchronously
let func = instance.get_func(&mut store, "compute")
    .unwrap()
    .typed::<i32, i32>(&store)?;

let result = func.call_async(&mut store, 42).await?;
```

### Multi-value Support

```rust
// WASM function returning multiple values
let wasm = r#"
    (module
        (func (export "divmod")
            (param i32 i32)
            (result i32 i32)
            local.get 0
            local.get 1
            i32.div_u
            local.get 0
            local.get 1
            i32.rem_u
        )
    )
"#;

let func = instance.get_func(&mut store, "divmod")
    .unwrap()
    .typed::<(i32, i32), (i32, i32)>(&store)?;

let (quotient, remainder) = func.call(&mut store, (17, 5))?;
assert_eq!(quotient, 3);
assert_eq!(remainder, 2);
```

### References Types

```rust
let mut config = Config::new();
config.wasm_reference_types(true);

let engine = Engine::new(&config)?;

// WASM with externref
let wasm = r#"
    (module
        (import "env" "store_ref" (func (param externref)))
        (func (export "create"))
    )
"#;

let mut store = Store::new(&engine, ());

// Pass reference to WASM
let host_obj = String::from("Hello");
let func = instance.get_func(&mut store, "store_ref")
    .unwrap();

func.call(&mut store, [Val::ExternRef(Some(ExternRef::new(host_obj)))])?;
```

## Component Model

```rust
use wasmtime::{component::{Component, Linker}, Store};

let engine = Engine::default();
let mut store = Store::new(&engine, ());

// Load WASM component
let component = Component::from_file(&engine, "component.wasm")?;

// Create linker for components
let mut linker = Linker::new(&engine);

// Bind imports
linker.root().func_wrap("host_func", |x: i32| Ok(x + 1))?;

// Instantiate component
let instance = linker.instantiate(&mut store, &component)?;

// Get exported function
let run = instance.get_func::<(i32,), (i32,)>(&mut store, "run")?;
let result = run.call(&mut store, (42,))?;
```

## Embedding Example

```rust
use wasmtime::*;

fn main() -> Result<()> {
    // Create engine with config
    let mut config = Config::new();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config)?;

    // Create store with host state
    struct Host {
        counter: u32,
    }
    let mut store = Store::new(&engine, Host { counter: 0 });

    // Define host functions
    let mut linker = Linker::new(&engine);
    linker.func_wrap("env", "get_counter", |caller: Caller<'_, Host>| {
        Ok(caller.data().counter)
    })?;
    linker.func_wrap("env", "increment", |mut caller: Caller<'_, Host>| {
        caller.data_mut().counter += 1;
        Ok(())
    })?;

    // Load module
    let module = Module::new(&engine, r#"
        (module
            (import "env" "get_counter" (func (result i32)))
            (import "env" "increment" (func))
            (func (export "compute")
                call $increment
                call $get_counter
            )
        )
    "#)?;

    // Instantiate and run
    let instance = linker.instantiate(&mut store, &module)?;
    let compute = instance.get_func::<(), i32>(&mut store, "compute")?;
    let result = compute.call(&mut store, ())?;

    println!("Result: {}", result);
    Ok(())
}
```

## Performance Considerations

### Compilation Caching

```rust
use wasmtime::{Config, Engine};

let mut config = Config::new();

// Enable compilation cache
unsafe {
    config.cache_config_load_default()?;
}

let engine = Engine::new(&config)?;

// Subsequent compilations use cache
let module = Module::new(&engine, wasm_bytes)?; // Fast if cached
```

### Memory Allocation

```rust
use wasmtime::{Config, MemoryCreator, MemoryType};

struct CustomMemoryCreator;

impl MemoryCreator for CustomMemoryCreator {
    fn new_memory(
        &self,
        ty: MemoryType,
    ) -> Result<Box<dyn wasmtime::Memory>> {
        // Custom memory allocation (e.g., huge pages, NUMA-aware)
        Ok(Box::new(CustomMemory::new(ty)))
    }
}

let mut config = Config::new();
config.memory_creator(CustomMemoryCreator);
```

## Related Documents

- [WASI](./wasi-exploration.md) - WASI implementation details
- [Wasm Tools](./wasm-tools-exploration.md) - WASM tooling
- [Wit Bindgen](./wit-bindgen-exploration.md) - Interface bindings

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/wasmtime/`
- Wasmtime Documentation: https://docs.wasmtime.dev/
- Bytecode Alliance: https://bytecodealliance.org/
