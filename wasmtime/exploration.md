---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime
repository: https://github.com/bytecodealliance/wasmtime
explored_at: 2026-03-22
language: Rust, C, C++, Python, .NET, Go, Ruby
---

# Project Exploration: wasmtime (Bytecode Alliance)

## Overview

**Wasmtime** is a standalone runtime for WebAssembly, developed by the Bytecode Alliance. It executes WebAssembly modules outside of the browser using the Cranelift JIT compiler for fast, secure, and configurable execution. The wasmtime ecosystem includes wasm-tools (low-level Wasm manipulation) and wit-bindgen (Component Model bindings generation).

### Key Value Proposition

- **Fast execution** - Cranelift code generator with optimizing compilation
- **Secure by design** - Careful review, fuzzing, formal verification efforts
- **Highly configurable** - From tiny embedded to massive server deployments
- **WASI support** - Rich host environment interaction APIs
- **Standards compliant** - Passes official WebAssembly test suite
- **Multi-language** - Rust, C/C++, Python, .NET, Go, Ruby embeddings

### Example Usage

```bash
# Install wasmtime
curl https://wasmtime.dev/install.sh -sSf | bash

# Compile Rust to WASM
rustup target add wasm32-wasip1
rustc hello.rs --target wasm32-wasip1

# Run with wasmtime
wasmtime hello.wasm
# Hello, world!
```

```rust
// Embed wasmtime in Rust
use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "hello.wasm")?;
    let store = Store::new(&engine, ());
    let instance = Instance::new(&store, &module, &[])?;

    let run = instance.get_typed_func::<(), ()>(&store, "run")?;
    run.call(())?;

    Ok(())
}
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/
├── wasmtime/                        # Main runtime project
│   ├── .github/
│   ├── ci/
│   ├── crates/
│   │   ├── wasmtime/                # Main runtime crate
│   │   ├── wasi-common/             # WASI implementation
│   │   ├── wasi-nn/                 # WASI neural network extension
│   │   ├── wasi-threads/            # WASI threading support
│   │   ├── wiggle/                  # WASI interface generator
│   │   ├── cranelift/               # JIT compiler
│   │   │   ├── codegen/             # Code generation
│   │   │   ├── frontend/            # IR frontend
│   │   │   ├── interpreter/         # Cranelift interpreter
│   │   │   ├── jit/                 # JIT compilation
│   │   │   ├── object/              # Object file handling
│   │   │   ├── bforest/             # B-forest data structure
│   │   │   ├── entity/              # Entity management
│   │   │   ├── filetests/           # File-based tests
│   │   │   ├── fuzzgen/             # Fuzzing generator
│   │   │   ├── isle/                # ISLE pattern language
│   │   │   └── reader/              # CLIF reader
│   │   ├── fuzzing/                 # Fuzzing infrastructure
│   │   ├── jit-debug/               # JIT debugging support
│   │   ├── c-api/                   # C API bindings
│   │   └── test-programs/           # Test programs
│   ├── docs/
│   │   ├── WASI-*.md                # WASI documentation
│   │   ├── cli*.md                  # CLI documentation
│   │   └── contributing-*.md        # Contributing guides
│   ├── ADOPTERS.md
│   ├── CONTRIBUTING.md
│   ├── README.md
│   ├── RELEASES.md
│   └── SECURITY.md
│
├── wasm-tools/                      # Wasm manipulation tools
│   ├── crates/
│   │   ├── wasmparser/              # Wasm binary parser
│   │   ├── wat/                     # WAT text format parser
│   │   ├── wast/                    # WAST AST
│   │   ├── wasmprinter/             # Binary to text printer
│   │   ├── wasm-smith/              # Test case generator
│   │   ├── wasm-mutate/             # Test case mutator
│   │   ├── wasm-shrink/             # Test case shrinker
│   │   ├── wasm-encoder/            # Binary module generator
│   │   ├── wit-parser/              # WIT interface parser
│   │   ├── wit-encoder/             # WIT generator
│   │   ├── wit-component/           # Component creation
│   │   ├── wit-smith/               # WIT test generator
│   │   └── wasm-metadata/           # Metadata handling
│   ├── CONTRIBUTING.md
│   └── README.md
│
├── wit-bindgen/                     # Component Model bindings
│   ├── crates/
│   │   ├── wit-bindgen/             # Core bindings generator
│   │   ├── wit-bindgen-core/        # Core functionality
│   │   ├── wit-bindgen-rust/        # Rust guest bindings
│   │   ├── wit-bindgen-c/           # C guest bindings
│   │   └── wit-bindgen-cli/         # CLI tool
│   ├── tests/
│   │   ├── rust/                    # Rust test cases
│   │   ├── c/                       # C test cases
│   │   └── hosts/                   # Host implementations
│   └── README.md
│
├── WASI/                            # WebAssembly System Interface
│   ├── legacy/                      # Legacy WASI (wasi_snapshot_preview1)
│   ├── proposals/                   # WASI proposals
│   └── README.md
│
├── WASI-Virt/                       # WASI virtual filesystem
│   └── README.md
│
├── wasi-rs/                         # WASI Rust bindings
│   └── README.md
│
├── wasm-micro-runtime/              # Micro runtime (WAMR)
│   └── README.md
│
├── jco/                             # JavaScript component tools
│   └── README.md
│
└── wrpc/                            # RPC over WASM
    └── README.md
```

## Core Projects

### 1. Wasmtime Runtime

**The flagship WebAssembly runtime**

Wasmtime is a production-ready WebAssembly runtime that can be used as a standalone CLI tool or embedded in applications.

#### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Wasmtime Architecture                         │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    Embedder API                             │ │
│  │  - Rust, C, Python, .NET, Go, Ruby                         │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                  wasmtime::Runtime                          │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │ │
│  │  │   Engine     │  │   Module     │  │   Store      │     │ │
│  │  │  (config)    │  │ (compiled)   │  │  (instances) │     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                  Cranelift JIT Compiler                     │ │
│  │  - IR generation                                            │ │
│  │  - Optimization passes                                      │ │
│  │  - Native code emission                                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│                              ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                   WASI Layer                                │ │
│  │  - wasi-common                                              │ │
│  │  - wasi-nn (neural networks)                                │ │
│  │  - wasi-threads                                             │ │
│  │  - wiggle (interface generator)                             │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

#### Key Components

**Engine** - Compilation and configuration management:
```rust
let mut config = Config::new();
config.strategy(Strategy::Cranelift);
config.cranelift_debug_verifier(true);
config.wasm_reference_types(true);

let engine = Engine::new(&config)?;
```

**Module** - Compiled WebAssembly code:
```rust
// Compile from file
let module = Module::from_file(&engine, "path/to/file.wasm")?;

// Compile from bytes
let wasm_bytes = std::fs::read("module.wasm")?;
let module = Module::new(&engine, &wasm_bytes)?;

// Pre-compile for faster instantiation
let compiled = module.serialize()?;
std::fs::write("module.cwasm", compiled)?;
```

**Store** - Instance management and state:
```rust
// Store holds GC roots and instance state
let mut store = Store::new(&engine, MyState { /* ... */ });

// Add resources to the store
let resource = store.insert(MyResource { /* ... */ });
```

**Instance** - Running module with imports/exports:
```rust
let instance = Instance::new(&mut store, &module, &imports)?;

// Get exported functions
let run = instance.get_typed_func::<i32, i32>(&mut store, "run")?;
let result = run.call(&mut store, 42)?;

// Get exported memory
let memory = instance.get_memory(&store, "memory")
    .expect("module has no memory");
```

#### Cranelift Compiler

Cranelift is a code generator designed for speed of compilation rather than peak performance:

```
Source Wasm → Wasm IR → Cranelift IR → Machine Code

Optimization passes:
- Constant folding
- Dead code elimination
- Register allocation
- Instruction selection (ISLE)
- Branch optimization
```

**ISLE (Instruction Selection and Lowering Expressions)**:
```isle
;; Example ISLE rule for x86 addition
(defrule add_i32 (add x y)
  (let (result (Machine.reg))
    (emit (X86.add_rr result x y))))
```

#### WASI Support

WASI (WebAssembly System Interface) provides system-level APIs:

```rust
use wasmtime_wasi::{Wasi, WasiCtxBuilder};

let ctx = WasiCtxBuilder::new()
    .inherit_stdio()
    .inherit_args()?
    .build();

let wasi = Wasi::new(&store, ctx);
wasi.add_to_linker(&mut linker)?;
```

**WASI APIs supported:**
- Filesystem I/O
- Network sockets
- Clocks and timers
- Random numbers
- Environment variables
- Command-line arguments
- Exit codes

### 2. wasm-tools

**Low-level WebAssembly module manipulation**

wasm-tools provides CLI and Rust libraries for working with WebAssembly modules at the binary level.

#### CLI Commands

| Command | Description | Rust Crate |
|---------|-------------|------------|
| `validate` | Validate Wasm module | `wasmparser` |
| `print` | Binary to text (WAT) | `wasmprinter` |
| `parse` | Text (WAT) to binary | `wat`, `wast` |
| `smith` | Generate test modules | `wasm-smith` |
| `mutate` | Mutate test cases | `wasm-mutate` |
| `shrink` | Minimize test cases | `wasm-shrink` |
| `strip` | Remove custom sections | - |
| `demangle` | Demangle Rust/C++ symbols | - |
| `objdump` | Print section headers | - |
| `dump` | Debug binary info | - |

#### Component Model Commands

```bash
# Extract WIT interface from component
wasm-tools component wit component.wasm

# Convert WIT to binary
wasm-tools component wit ./wit --wasm

# Create component from core Wasm
wasm-tools component new my-core.wasm -o my-component.wasm

# Adapt WASI preview1 to preview2
wasm-tools component new my-core.wasm \
    --adapt wasi_snapshot_preview1.reactor.wasm \
    -o my-component.wasm
```

#### Library Usage

```rust
use wasmparser::{Parser, Payload};

fn parse_wasm(bytes: &[u8]) -> Result<()> {
    for payload in Parser::new(0).parse_all(bytes) {
        match payload? {
            Payload::Version { encoding, .. } => {
                println!("Wasm version: {:?}", encoding);
            }
            Payload::SectionStart(offset) => {
                println!("Section at offset: {}", offset);
            }
            Payload::TypeSection(s) => {
                for ty in s {
                    println!("Type: {:?}", ty?);
                }
            }
            // ... handle other sections
        }
    }
    Ok(())
}
```

#### wasm-smith (Test Generation)

```rust
use wasm_smith::{Config, Module};

struct MyConfig;

impl Config for MyConfig {
    fn max_types(&self) -> usize { 100 }
    fn max_functions(&self) -> usize { 100 }
    fn allow_start_export(&self) -> bool { true }
    // ... other config options
}

let mut rng = rand::thread_rng();
let module = Module::new(MyConfig, &mut rng)?;
let wasm_bytes = module.to_bytes();
```

#### wasm-mutate (Test Mutation)

```rust
use wasm_mutate::{ModuleInfo, Mutator};

let input = std::fs::read("test.wasm")?;
let info = ModuleInfo::new(&input)?;
let mutator = Mutator::new();

for mutated in mutator.iter(&input, &info, seed)? {
    let mutated_wasm = mutated?;
    // Test with mutated Wasm
}
```

#### wasm-shrink (Test Case Reduction)

```rust
use wasm_shrink::WasmShrinker;

let shrinker = WasmShrinker::default();
let shrunk = shrinker.shrink(
    &original_wasm,
    &test_program,
    |wasm| {
        // Predicate: does this Wasm still trigger the bug?
        run_test(wasm).is_err()
    }
)?;

println!("Shrunk from {} to {} bytes",
    original_wasm.len(), shrunk.len());
```

### 3. wit-bindgen

**Component Model bindings generator**

wit-bindgen generates language bindings for WebAssembly Component Model interfaces defined in WIT (WebAssembly Interface Types).

#### WIT Language

WIT is an interface definition language for the Component Model:

```wit
package example:my-app;

interface types {
    record user {
        id: u64,
        name: string,
        email: string,
    }

    enum error {
        not-found,
        unauthorized,
        invalid-input,
    }
}

interface database {
    use types.{user, error};

    get-user: func(id: u64) -> result<user, error>;
    create-user: func(name: string, email: string) -> result<user, error>;
    delete-user: func(id: u64) -> result<(), error>;
}

world my-app {
    import database;

    export run: func() -> result<(), error>;
}
```

#### Rust Guest Bindings

```rust
// src/lib.rs
wit_bindgen::generate!({
    world: "my-app",
});

use exports::my_app::run::Guest;

struct MyApp;

impl Guest for MyApp {
    fn run() -> Result<(), Error> {
        let db = imports::database::instance();

        let user = db.create_user(
            "Alice".to_string(),
            "alice@example.com".to_string()
        )?;

        println!("Created user: {}", user.name);

        Ok(())
    }
}

export!(MyApp);
```

#### C Guest Bindings

```c
// my-component.c
#include "my-app.h"

void my_app_run(my_app_error_t *err) {
    database_user_t user;

    database_create_user(
        &(string_t) {.ptr = "Bob", .len = 3},
        &(string_t) {.ptr = "bob@example.com", .len = 15},
        &user,
        err
    );

    if (err->is_err) return;

    printf("Created user: %.*s\n",
           (int)user.name.len, user.name.ptr);
}
```

#### Rust Host Bindings (wasmtime::component::bindgen!)

```rust
use wasmtime::component::{bindgen, Component, Linker};
use wasmtime::{Config, Engine};

bindgen!({
    path: "wit",
    world: "my-app",
    async: true,
});

#[tokio::main]
async fn main() -> wasmtime::Result<()> {
    let engine = Engine::new(Config::new().wasm_component_model(true))?;
    let component = Component::from_file(&engine, "my-component.wasm")?;

    let mut linker = Linker::new(&engine);
    MyApp::add_to_linker(&mut linker)?;

    let (instance, _) = MyApp::instantiate_async(
        &mut linker,
        component,
        Store::new(&engine, ())
    ).await?;

    instance.run().await?;

    Ok(())
}
```

#### CLI Usage

```bash
# Generate Rust bindings
wit-bindgen rust ./wit

# Generate C bindings
wit-bindgen c ./wit

# Generate for specific world
wit-bindgen rust ./wit --world my-app

# Generate with stub implementations
wit-bindgen rust ./wit --generate-stub
```

### 4. WASI (WebAssembly System Interface)

**System interface for WebAssembly**

WASI provides a modular system interface for WebAssembly modules to interact with the host system securely.

#### WASI Preview Versions

| Version | Status | Notes |
|---------|--------|-------|
| `wasi_snapshot_preview1` | Legacy | Original monolithic API |
| `wasi_preview1` | Supported | Component Model compatible |
| `wasi_preview2` | In development | Modular capability-based |

#### WASI Modules

```
wasi-filesystem      - Filesystem operations
wasi-sockets         - Network sockets
wasi-clocks          - Clocks and timers
wasi-random          - Random number generation
wasi-environment     - Environment variables
wasi-exit            - Process exit
wasi-http            - HTTP client/server
wasi-keyvalue        - Key-value storage
wasi-blobstore       - Blob storage
```

#### Example: WASI Filesystem

```rust
use wasi::filesystem::types::*;

// Open a file
let fd = open_at(
    AT_FDCWD,
    PathFlags::empty(),
    "hello.txt".to_string(),
    OpenFlags::CREATE,
    ReadWrite::ReadWrite,
    ModeFlags::empty(),
)?;

// Write to file
let content = b"Hello, WASI!";
write(fd, content)?;

// Read from file
let mut buffer = vec![0u8; 1024];
let n = read(fd, &mut buffer)?;
println!("Read: {}", String::from_utf8_lossy(&buffer[..n]));

// Close file
fdclose(fd)?;
```

### 5. WASI-Virt

**Virtual filesystem for WASI**

WASI-Virt provides a virtual filesystem layer for WASI, enabling sandboxed file access.

```rust
use wasi_virt::{VirtFileSystem, VirtConfig};

let mut config = VirtConfig::new();
config.mount("/app", "/host/path/to/app");
config.mount("/data", "/host/path/to/data");

let vfs = VirtFileSystem::new(config)?;

// Now use vfs with WASI
```

### 6. jco (JavaScript Component Tools)

**Component tools for JavaScript**

```bash
# Install jco
npm install -g @bytecodealliance/jco

# Translate component to JS
jco transpile my-component.wasm -o ./output

# Run component in Node.js
node ./output/my-component.js
```

### 7. wrpc

**RPC over WebAssembly**

```rust
use wrpc_runtime_wasmtime;

// Define RPC interface
wrpc_interface_rpc::bindgen!({
    path: "wit",
    world: "rpc-service",
});

// Implement service
struct MyService;

impl RpcService for MyService {
    async fn handle_request(&self, req: Request) -> Result<Response> {
        // Handle RPC request
        Ok(Response { /* ... */ })
    }
}
```

## Language Embeddings

### Rust

```rust
use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "module.wasm")?;

    let mut linker = Linker::new(&engine);
    linker.func_wrap("", "host_log", |msg: String| {
        println!("Wasm says: {}", msg);
    })?;

    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;

    let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
    run.call(&mut store, ())?;

    Ok(())
}
```

### C/C++

```c
#include <wasm.h>
#include <wasmtime.h>

wasm_engine_t* engine = wasm_engine_new();
wasm_store_t* store = wasm_store_new(engine);

wasm_byte_vec_t wasm;
wasm_byte_vec_new_from_file(&wasm, "module.wasm");

wasm_module_t* module = wasm_module_new(store, &wasm);

wasm_instance_t* instance = wasm_instance_new(store, module, imports, NULL);

// Get and call exports...
```

### Python

```python
from wasmtime import Store, Module, Instance, Engine

engine = Engine()
store = Store(engine)
module = Module.from_file(engine, "module.wasm")
instance = Instance(store, module, [])

run = instance.exports(store)["run"]
run(store)
```

### .NET

```csharp
using Wasmtime;

using var engine = new Engine();
using var module = Module.FromFile(engine, "module.wasm");
using var linker = new Linker(engine);

linker.Define("", "host_log", (string msg) => {
    Console.WriteLine($"Wasm says: {msg}");
});

using var store = new Store(engine);
var instance = linker.Instantiate(store, module);

var run = instance.GetFunction("run");
run?.Invoke();
```

## Performance Characteristics

### Compilation Speed

| Mode | Speed | Use Case |
|------|-------|----------|
| JIT (Cranelift) | Fast | General purpose |
| AOT (Precompiled) | Instant | Production deployment |
| Lazy JIT | Deferred | Large modules |

### Runtime Performance

| Operation | Relative Speed |
|-----------|---------------|
| Native call | 1.0x |
| Wasm → Wasm call | 1.1x |
| Host → Wasm call | 1.2x |
| Wasm → Host call | 1.3x |
| Memory access | 1.1x (bounds check) |

### Memory Usage

| Component | Approximate Size |
|-----------|-----------------|
| Engine | ~10 MB |
| Compiled module | 1-10 MB |
| Instance | ~1 MB base |
| Wasm linear memory | Configurable (64KB pages) |

## Build and Configuration

### Config Options

```rust
let mut config = Config::new();

// Compilation strategy
config.strategy(Strategy::Cranelift);

// Optimization level
config.cranelift_opt_level(OptLevel::SpeedAndSize);

// Debugging
config.cranelift_debug_verifier(true);
config.generate_address_map(true);

// Wasm features
config.wasm_reference_types(true);
config.wasm_multi_value(true);
config.wasm_multi_memory(true);
config.wasm_simd(true);
config.wasm_relaxed_simd(true);
config.wasm_memory64(true);

// Resource limits
config.max_wasm_memory(4 * 1024 * 1024 * 1024); // 4GB
config.max_instances(1000);

// Caching
config.cache_config_load_default()?;
```

### Caching

```bash
# Configure cache
wasmtime config new

# Cache location (default)
# Linux: ~/.cache/wasmtime
# macOS: ~/Library/Caches/wasmtime
# Windows: %LOCALAPPDATA%\wasmtime
```

## Security Model

### Sandboxing

- **Memory isolation** - Each instance has isolated linear memory
- **Capability-based security** - WASI requires explicit capabilities
- **No implicit host access** - Imports must be explicitly provided
- **Spectre mitigations** - Built-in Spectre v1/v4 protections

### Resource Limits

```rust
config.max_wasm_memory(256 * 1024 * 1024); // 256MB per instance
config.max_memory_pages(4096); // 4096 pages
config.max_table_elements(10000);
config.max_instances(100);
```

### Fuzzing

Wasmtime is continuously fuzzed via Google's OSS-Fuzz:

```bash
# Run fuzz targets locally
cd wasmtime
cargo fuzz run validate
cargo fuzz run exec
cargo fuzz run compile
```

## Use Cases

### 1. Plugin Systems

```rust
struct PluginHost {
    engine: Engine,
    linker: Linker<PluginState>,
}

impl PluginHost {
    fn load_plugin(&self, wasm_path: &str) -> Result<Plugin> {
        let module = Module::from_file(&self.engine, wasm_path)?;
        let mut store = Store::new(&self.engine, PluginState::new());
        let instance = self.linker.instantiate(&mut store, &module)?;
        Ok(Plugin { store, instance })
    }
}
```

### 2. Serverless/Edge Computing

```rust
// Pre-compile modules for fast instantiation
let engine = Engine::new(Config::new().cache_config_load_default()?)?;
let module = Module::from_file(&engine, "function.cwasm")?;

// Fast instantiation per request
fn handle_request(module: &Module) -> Result<Response> {
    let mut store = Store::new(module.engine(), RequestState::new());
    let instance = Instance::new(&mut store, module, &[])?;
    let handler = instance.get_typed_func::<Request, Response>(&mut store, "handle")?;
    Ok(handler.call(&mut store, request)?)
}
```

### 3. Smart Contracts

```rust
struct ContractRuntime {
    engine: Engine,
    state: ContractState,
}

impl ContractRuntime {
    fn execute(&mut self, contract: &Module, method: &str, args: &[Val]) -> Result<Vec<Val>> {
        let mut store = Store::new(&self.engine, &mut self.state);
        let instance = Instance::new(&mut store, contract, &[])?;
        let func = instance.get_func(&mut store, method)
            .ok_or_else(|| anyhow!("Method not found"))?;
        func.call(&mut store, args)
    }
}
```

## Trade-offs

| Aspect | Decision | Trade-off |
|--------|----------|-----------|
| Cranelift | Fast compilation | Slightly slower code than LLVM |
| JIT default | Fast startup | AOT available for latency-sensitive |
| WASI preview1 | Backwards compatible | Moving to modular preview2 |
| Rust-first | Best Rust support | Other languages may lag |
| Component Model | Future-proof | Still evolving specification |

## Best Practices

### 1. Use Pre-compilation for Production

```rust
// Compile once
let compiled = module.serialize()?;
std::fs::write("module.cwasm", &compiled)?;

// Load quickly later
let module = Module::deserialize(&engine, "module.cwasm")?;
```

### 2. Reuse Engines

```rust
// Bad: Create engine per request
fn handle() {
    let engine = Engine::default();
    // ...
}

// Good: Share engine across requests
static ENGINE: OnceLock<Engine> = OnceLock::new();
fn get_engine() -> &'static Engine {
    ENGINE.get_or_init(Engine::default)
}
```

### 3. Use Typed Functions

```rust
// Untyped (more error-prone)
let func = instance.get_func(&store, "add")?;
let results = func.call(&mut store, &[Val::I32(2), Val::I32(3)])?;

// Typed (type-safe)
let add = instance.get_typed_func::<(i32, i32), i32>(&store, "add")?;
let result = add.call(&mut store, (2, 3))?;
```

### 4. Handle Traps Gracefully

```rust
match func.call(&mut store, args) {
    Ok(results) => { /* Success */ }
    Err(e) if e.is::<Trap>() => { /* Wasm trap */ }
    Err(e) => { /* Other error */ }
}
```

## Related Projects in Source Directory

- **WASI** - WebAssembly System Interface specifications
- **WASI-Virt** - Virtual filesystem for WASI
- **wasi-rs** - WASI Rust bindings
- **wasm-micro-runtime** - Embedded Wasm runtime (WAMR)
- **wasm-tools** - Wasm manipulation CLI and libraries
- **wit-bindgen** - Component Model bindings generator
- **jco** - JavaScript component tools
- **wrpc** - RPC over WebAssembly

## References

- **Wasmtime Guide**: https://docs.wasmtime.dev/
- **API Documentation**: https://docs.rs/wasmtime
- **GitHub**: https://github.com/bytecodealliance/wasmtime
- **Website**: https://wasmtime.dev/
- **Bytecode Alliance**: https://bytecodealliance.org/
- **WASI Specification**: https://wasi.dev/
- **Component Model**: https://github.com/WebAssembly/component-model
- **Zulip Chat**: https://bytecodealliance.zulipchat.com/#narrow/stream/217126-wasmtime
