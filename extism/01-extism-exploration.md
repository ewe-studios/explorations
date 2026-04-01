---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.extism/extism
repository: git@github.com:extism/extism.git
explored_at: 2026-03-30
language: Rust, TypeScript, Go, Python, more
category: WebAssembly, Plugin Systems
---

# Extism - Exploration

## Overview

Extism is a **lightweight WebAssembly framework** for building extensible software and plugin systems. It provides a universal interface for running Wasm code anywhere (servers, edge, CLIs, IoT, browsers) with secure sandboxing, persistent memory, and host-controlled capabilities.

### Key Value Proposition

- **Universal Runtime**: Same interface everywhere
- **Plugin System**: Safe execution of untrusted code
- **Multi-Language**: 15+ SDKs, 8+ PDKs
- **Secure by Default**: Sandboxed, capability-based
- **Persistent Memory**: Plugin-scoped variables
- **Host Controlled**: HTTP, capabilities opt-in

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Extism Architecture                           │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Host Application (any language)             │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │              Extism SDK (Host)                    │  │   │
│  │  │  - Plugin loading                                 │  │   │
│  │  │  - Memory management                              │  │   │
│  │  │  - Host function registration                     │  │   │
│  │  │  - Capability enforcement                         │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  │                           │                               │   │
│  │                           │ FFI                           │   │
│  │                           ▼                               │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │          Extism Runtime (Wasmtime/Crane)          │  │   │
│  │  │  - Wasm module instantiation                      │  │   │
│  │  │  - Memory isolation                               │  │   │
│  │  │  - Host function linking                          │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  │                           │                               │   │
│  │                           │ Memory                        │   │
│  │                           ▼                               │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │          Plugin (Wasm module with PDK)            │  │   │
│  │  │  ┌─────────────────────────────────────────────┐  │  │   │
│  │  │  │   PDK (Plugin Development Kit)              │  │  │   │
│  │  │  │   - Input/Output handling                   │  │  │   │
│  │  │  │   - Config access                           │  │  │   │
│  │  │  │   - Variable storage                        │  │  │   │
│  │  │  │   - Host function calls                     │  │  │   │
│  │  │  └─────────────────────────────────────────────┘  │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
extism/
├── runtime/                  # Core runtime (Rust)
│   ├── src/
│   │   ├── plugin.rs         # Plugin management
│   │   ├── current_plugin.rs # Plugin context
│   │   ├── manifest.rs       # Plugin manifest
│   │   ├── function.rs       # Function registration
│   │   ├── memory.rs         # Memory handling
│   │   └── sdk.rs            # SDK exports
│   └── Cargo.toml
│
├── libextism/                # C library
│   ├── src/
│   │   └── extism.c
│   └── include/
│       └── extism.h
│
├── convert/                  # Type conversion traits
│
├── crates/
│   ├── extism-wasi/          # WASI support
│   └── extism-convert/       # Type conversions
│
├── manifests/
│   └── sample manifests
│
└── tests/
    ├── plugin tests
    └── host tests
```

## Core Concepts

### 1. Host and Plugin

```
Host (SDK)                          Plugin (PDK)
  │                                    │
  │ 1. Load Wasm module                │
  │───────────────────────────────────>│
  │                                    │
  │ 2. Call function with input        │
  │───────────────────────────────────>│
  │                                    │
  │ 3. Plugin reads input via PDK      │
  │                                    │
  │ 4. Plugin processes                │
  │                                    │
  │ 5. Plugin sets output via PDK      │
  │<───────────────────────────────────│
  │                                    │
  │ 6. Host receives output            │
  │<───────────────────────────────────│
```

### 2. Memory Model

```
┌─────────────────────────────────────────┐
│         Host Memory Space               │
│                                         │
│  ┌───────────────────────────────────┐ │
│  │    Plugin Memory (Wasm Linear)    │ │
│  │                                   │ │
│  │  ┌─────────────────────────────┐ │ │
│  │  │   Plugin Variables          │ │ │
│  │  │   - Persistent per-plugin   │ │ │
│  │  │   - Key-value storage       │ │ │
│  │  │   - Isolated from host      │ │ │
│  │  └─────────────────────────────┘ │ │
│  │                                   │ │
│  │  ┌─────────────────────────────┐ │ │
│  │  │   Input/Output Buffers      │ │ │
│  │  │   - Shared memory region    │ │ │
│  │  │   - Zero-copy when possible │ │ │
│  │  └─────────────────────────────┘ │ │
│  └───────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

### 3. Manifest

```json
{
  "name": "my-plugin",
  "wasm": [
    {
      "path": "plugin.wasm"
    }
  ],
  "config": {
    "environment": "production",
    "api_key": "secret"
  },
  "allowed_hosts": [
    "https://api.example.com"
  ],
  "allowed_paths": {
    "/data": "/plugin/data"
  }
}
```

## Host SDK Usage

### Rust Host

```rust
use extism::{Plugin, Manifest, Wasm, CurrentPlugin};

// Load plugin from file
let manifest = Manifest::new([Wasm::file("plugin.wasm")]);
let mut plugin = Plugin::new(&manifest, [], true).unwrap();

// Call function with string input/output
let output = plugin.call::<&str, &str>("greet", "World").unwrap();
println!("{}", output); // Hello, World!

// Call function with binary data
let input = vec![0x01, 0x02, 0x03];
let output = plugin.call::<&[u8], &[u8]>("process", &input).unwrap();

// Host function
fn log(plugin: &mut CurrentPlugin, inputs: &[Val], outputs: &mut [Val]) -> Result<(), Error> {
    let msg = plugin.memory_get_str(inputs[0].unwrap_i64() as u64)?;
    println!("Plugin: {}", msg);
    Ok(())
}

let manifest = Manifest::new([Wasm::file("plugin.wasm")])
    .with_host_function("log", log);
```

### JavaScript Host

```javascript
import { Plugin } from '@extism/extism'

// Load from URL
const plugin = await Plugin.fromUrl('plugin.wasm')

// Call function
const result = await plugin.call('greet', 'World')
console.log(result)

// Load from manifest
const manifest = {
  wasm: [{ url: 'plugin.wasm' }],
  config: { key: 'value' }
}

const plugin2 = await new Plugin(manifest)
```

### Python Host

```python
import extism

# Create plugin
plugin = extism.Plugin("plugin.wasm")

# Call function
result = plugin.call("greet", "World")
print(result)

# With config
plugin = extism.Plugin("plugin.wasm", config={"key": "value"})
```

### Go Host

```go
package main

import "github.com/extism/go-sdk"

func main() {
    manifest := extism.Manifest{
        Wasm: []extism.Wasm{{
            Path: "plugin.wasm",
        }},
    }

    plugin, err := extism.NewPlugin(manifest)
    if err != nil {
        panic(err)
    }

    result, err := plugin.Call("greet", []byte("World"))
    if err != nil {
        panic(err)
    }

    fmt.Println(string(result))
}
```

## Plugin Development

### Rust PDK

```rust
use extism_pdk::*;

// Simple function
#[plugin_fn]
pub fn greet(name: String) -> FnResult<String> {
    Ok(format!("Hello, {}!", name))
}

// With config
#[plugin_fn]
pub fn get_config() -> FnResult<String> {
    let config = config::get("environment")?;
    Ok(config.unwrap_or("development".to_string()))
}

// With variables (persistent state)
#[plugin_fn]
pub fn count() -> FnResult<u32> {
    let count = var::get::<u32>("count")?.unwrap_or(0);
    let new_count = count + 1;
    var::set("count", new_count)?;
    Ok(new_count)
}

// Host function call
#[plugin_fn]
pub fn log_message() -> FnResult<()> {
    host_fn!("log"("Hello from plugin!"))?;
    Ok(())
}

// HTTP request (if allowed)
#[plugin_fn]
pub fn fetch_data() -> FnResult<String> {
    let mut req = HttpRequest::new("https://api.example.com/data");
    let resp = http::request(req)?;
    Ok(String::from_utf8(resp.body).unwrap())
}
```

### Go PDK

```go
package main

import "github.com/extism/go-pdk"

//export greet
func greet() uint32 {
    input := pdk.Input()
    name := string(input)

    output := "Hello, " + name + "!"
    pdk.SetOutput([]byte(output))

    return 0
}

//export count
func count() uint32 {
    var count uint32
    pdk.LoadVar("count", &count)

    count++
    pdk.StoreVar("count", &count)

    return count
}
```

### AssemblyScript PDK

```typescript
import { plugin_fn } from "@extism/as-pdk";

@plugin_fn
export function greet(name: string): string {
    return `Hello, ${name}!`;
}

@plugin_fn
export function add(a: i32, b: i32): i32 {
    return a + b;
}
```

## Use Cases

### 1. Plugin System

```rust
// Host application
struct Editor {
    plugins: Vec<Plugin>,
}

impl Editor {
    fn load_plugin(&mut self, path: &str) {
        let manifest = Manifest::new([Wasm::file(path)]);
        let plugin = Plugin::new(&manifest, [], true).unwrap();
        self.plugins.push(plugin);
    }

    fn run_plugins(&mut self, text: &str) -> String {
        let mut result = text.to_string();

        for plugin in &mut self.plugins {
            if let Ok(output) = plugin.call::<&str, &str>("transform", &result) {
                result = output.to_string();
            }
        }

        result
    }
}
```

### 2. FaaS Platform

```rust
struct FaasRuntime {
    functions: HashMap<String, Plugin>,
}

impl FaasRuntime {
    fn deploy(&mut self, name: String, wasm: Vec<u8>, config: Value) {
        let manifest = Manifest::new([Wasm::memory(&wasm)])
            .with_config(config);

        let plugin = Plugin::new(&manifest, [], true).unwrap();
        self.functions.insert(name, plugin);
    }

    fn invoke(&mut self, name: &str, input: &[u8]) -> Result<Vec<u8>, Error> {
        let plugin = self.functions.get_mut(name).ok_or("Function not found")?;
        plugin.call::<&[u8], Vec<u8>>("handler", input)
    }
}
```

### 3. Code Generator

```rust
struct CodeGenerator {
    templates: HashMap<String, Plugin>,
}

impl CodeGenerator {
    fn add_template(&mut self, name: String, wasm: Vec<u8>) {
        let manifest = Manifest::new([Wasm::memory(&wasm)]);
        let plugin = Plugin::new(&manifest, [], true).unwrap();
        self.templates.insert(name, plugin);
    }

    fn generate(&mut self, template: &str, context: &Value) -> Result<String, Error> {
        let plugin = self.templates.get_mut(template).ok_or("Template not found")?;

        let input = serde_json::to_vec(context)?;
        let output = plugin.call::<&[u8], String>("render", &input)?;

        Ok(output)
    }
}
```

### 4. User-Defined Transformations

```rust
struct DataPipeline {
    transforms: Vec<Plugin>,
}

impl DataPipeline {
    fn add_transform(&mut self, wasm: Vec<u8>) -> Result<(), Error> {
        let manifest = Manifest::new([Wasm::memory(&wasm)])
            .with_allowed_host("api.transform-service.com");

        let plugin = Plugin::new(&manifest, [], true)?;
        self.transforms.push(plugin);
        Ok(())
    }

    fn process(&mut self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut result = data.to_vec();

        for plugin in &mut self.transforms {
            result = plugin.call::<&[u8], Vec<u8>>("transform", &result)?;
        }

        Ok(result)
    }
}
```

## Security Deep Dive

### Capability Model

```
Plugin Capabilities (default = none):
├── Network: allowed_hosts list
├── Filesystem: allowed_paths mapping
├── Environment: config only (no env vars)
├── Random: enabled by default
└── Host Functions: explicitly registered

Example manifest:
{
  "wasm": [{ "path": "plugin.wasm" }],
  "allowed_hosts": ["https://api.example.com"],
  "allowed_paths": { "/host/data": "/plugin/sandbox" },
  "config": { "api_key": "..." }
}
```

### Resource Limits

```rust
use extism::{Manifest, Plugin, Wasm};
use std::time::Duration;

let manifest = Manifest::new([Wasm::file("plugin.wasm")]);
let mut plugin = Plugin::new(&manifest, [], true).unwrap();

// Memory limit
plugin.set_max_memory(64 * 1024 * 1024); // 64MB

// Execution timeout
plugin.set_timeout(Duration::from_secs(5));

// Function-specific timeout
plugin.set_function_timeout("slow_func", Duration::from_secs(10));
```

### Sandboxing Guarantees

```
Plugin Isolation:
├── Separate Wasm memory space
├── No direct host memory access
├── No syscalls
├── No network (unless allowed)
├── No filesystem (unless allowed)
├── Time-limited execution
└── Memory-limited execution
```

## Performance

### Benchmarks

```
Cold Start:
├── Wasm module load: ~1-5ms
├── Instance creation: ~0.1-1ms
└── First call: ~1-10ms

Warm Execution:
├── Simple function: ~10-100μs
├── String processing: ~100μs-1ms
└── Complex computation: ~1-10ms

Memory Overhead:
├── Runtime: ~5-10MB
├── Per-plugin: ~1-5MB
└── Shared: Minimal
```

### Optimization Tips

```rust
// 1. Reuse plugin instances
let mut plugin = Plugin::new(&manifest, [], true)?;
// Call multiple times instead of recreating

// 2. Use binary format
plugin.call::<&[u8], &[u8]>("process", &binary_data)

// 3. Batch operations
plugin.call::<Vec<Item>, Vec<Result>>("batch_process", &items)

// 4. Enable caching
let config = extism::Config::default().with_cache(true);
```

---

## Related Deep Dives

- [00-zero-to-extism-engineer.md](./00-zero-to-extism-engineer.md) - Fundamentals
- [02-extism-pdk-deep-dive.md](./02-extism-pdk-deep-dive.md) - Plugin development
- [03-extism-security-deep-dive.md](./03-extism-security-deep-dive.md) - Security model
