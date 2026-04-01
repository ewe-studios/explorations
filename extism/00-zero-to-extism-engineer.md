---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/extism
explored_at: 2026-03-30
prerequisites: Basic programming knowledge, WebAssembly concepts helpful
---

# Zero to Extism Engineer - Complete Fundamentals

## Table of Contents

1. [What is Extism?](#what-is-extism)
2. [Why Extism?](#why-extism)
3. [Installation](#installation)
4. [Your First Plugin](#your-first-plugin)
5. [Host Application](#host-application)
6. [Memory Model](#memory-model)
7. [Host Functions](#host-functions)
8. [Configuration](#configuration)
9. [Security Model](#security-model)

## What is Extism?

Extism is a **lightweight WebAssembly framework** for building extensible software and plugin systems. It provides a universal interface for running Wasm code on servers, edge, CLIs, IoT, browsers, and everything in between.

### The Problem Extism Solves

**Without Extism:**
```
User wants plugins → You need sandboxing
                    → Isolate untrusted code
                    → Handle multiple languages
                    → Manage memory safely
Complexity: Security vulnerabilities, language barriers, memory bugs
```

**With Extism:**
```
User writes plugin (any language) → Compiles to Wasm
                                  → Extism runs it safely
                                  → Host controls capabilities
Simplicity: Universal runtime, secure by default, language agnostic
```

### Key Concepts

| Term | Definition |
|------|------------|
| **Host** | The application that loads and runs plugins |
| **Plugin** | A WebAssembly module with Extism PDK |
| **PDK** | Plug-in Development Kit (language-specific) |
| **SDK** | Host SDK to load and run plugins |
| **Memory** | Shared memory between host and plugin |
| **Host Functions** | Functions the host exposes to plugins |

## Why Extism?

### Benefits

1. **Language Agnostic**: Plugins in Rust, Go, Python, C#, etc.
2. **Secure by Default**: Sandboxed, untrusted code isolation
3. **Universal**: Same interface everywhere (server, edge, browser)
4. **Lightweight**: Minimal runtime overhead
5. **Persistent Memory**: Plugin-scoped variables
6. **Host Controlled**: HTTP, filesystem, capabilities opt-in

### When to Use Extism

**Good fit:**
- Plugin/extensibility systems
- FaaS platforms
- Code generators
- User-provided transformations
- Multi-tenant compute
- Secure code execution

**Not recommended:**
- Simple function calls (use native)
- Performance-critical tight loops
- When all code is trusted

## Installation

### Host SDK (Rust Example)

```bash
cargo add extism
```

### Plugin PDK (Rust Example)

```bash
cargo add extism-pdk
```

### Other Languages

**JavaScript/Node:**
```bash
npm install @extism/extism
```

**Python:**
```bash
pip install extism
```

**Go:**
```bash
go get github.com/extism/go-sdk
```

**Supported SDKs:** Rust, JavaScript, Elixir, Go, Haskell, Java, .NET, OCaml, Perl, PHP, Python, Ruby, Zig, C, C++

**Supported PDKs:** Rust, JavaScript, Go, Haskell, AssemblyScript, .NET, C, Zig

## Your First Plugin

### Rust Plugin

```rust
// plugin/src/lib.rs
use extism_pdk::*;

#[plugin_fn]
pub fn greet(name: String) -> FnResult<String> {
    Ok(format!("Hello, {}!", name))
}

#[plugin_fn]
pub fn add(a: i32, b: i32) -> FnResult<i32> {
    Ok(a + b)
}
```

### Build to Wasm

```bash
# Install Wasm target
rustup target add wasm32-unknown-unknown

# Build
cargo build --release --target wasm32-unknown-unknown

# Output: target/wasm32-unknown-unknown/release/plugin.wasm
```

### AssemblyScript Plugin

```typescript
// plugin.ts
import { plugin_fn } from "@extism/as-pdk";

@plugin_fn
export function greet(name: string): string {
    return `Hello, ${name}!`;
}
```

### Go Plugin

```go
// plugin.go
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
```

## Host Application

### Rust Host

```rust
// host.rs
use extism::{Plugin, Manifest, Wasm};

fn main() {
    // Load plugin
    let manifest = Manifest::new([Wasm::file("plugin.wasm")]);
    let mut plugin = Plugin::new(&manifest, [], true).unwrap();

    // Call function
    let result = plugin.call::<&str, &str>("greet", "World").unwrap();
    println!("{}", result); // Hello, World!

    // Call with different function
    let sum = plugin.call::<(i32, i32), i32>("add", (5, 3)).unwrap();
    println!("{}", sum); // 8
}
```

### JavaScript Host

```javascript
// host.js
import { Plugin } from '@extism/extism';

const plugin = await Plugin.fromUrl('plugin.wasm');

const result = await plugin.call('greet', 'World');
console.log(result); // Hello, World!
```

### Python Host

```python
# host.py
import extism

plugin = extism.Plugin("plugin.wasm")
result = plugin.call("greet", "World")
print(result)  # Hello, World!
```

## Memory Model

### How It Works

```
┌─────────────────────────────────────┐
│           Host Memory               │
│  ┌─────────────────────────────┐   │
│  │     Plugin Memory (Wasm)    │   │
│  │  ┌───────────────────────┐  │   │
│  │  │   Plugin Variables    │  │   │
│  │  │   - Persistent state  │  │   │
│  │  │   - Isolated per call │  │   │
│  │  └───────────────────────┘  │   │
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

### Plugin Variables

```rust
// In plugin
use extism_pdk::*;

#[plugin_fn]
pub fn count() -> FnResult<u32> {
    // Get current count
    let count = var::get::<u32>("count")?.unwrap_or(0);

    // Increment and store
    let new_count = count + 1;
    var::set("count", new_count)?;

    Ok(new_count)
}
```

### Host Memory Access

```rust
// Host can access plugin memory
let ptr = plugin.call::<&str, Ptr>("greet", "World")?;

// Read from plugin memory
let output = plugin.memory_get::<&str>(ptr)?;
```

## Host Functions

### Defining Host Functions

```rust
use extism::{CurrentPlugin, Error, MemoryHandle};

// Host function that plugin can call
fn log_message(plugin: &mut CurrentPlugin, inputs: &[Val], outputs: &mut [Val]) -> Result<(), Error> {
    let msg = plugin.memory_get_str(inputs[0].unwrap_i64() as u64)?;
    println!("Plugin says: {}", msg);
    Ok(())
}

// Register with plugin
let manifest = Manifest::new([Wasm::file("plugin.wasm")])
    .with_host_function("log", log_message);

let plugin = Plugin::new(&manifest, [], true)?;
```

### Calling Host Functions from Plugin

```rust
// In plugin
use extism_pdk::*;

#[plugin_fn]
pub fn do_work() -> FnResult<()> {
    // Call host function
    let output = host_fn!("log"("Working..."));
    Ok(())
}
```

### Built-in Host Functions

```rust
// HTTP (if enabled)
let response = host_fn!("http_get"("https://api.example.com"));

// Random (if enabled)
let random = host_fn!("random"());

// Sleep (if enabled)
host_fn!("sleep"(1000));
```

## Configuration

### Plugin Config

```rust
// Host provides config
let manifest = Manifest::new([Wasm::file("plugin.wasm")])
    .with_config_option("environment", "production")
    .with_config_option("api_key", "secret");

let plugin = Plugin::new(&manifest, [], true)?;
```

### Reading Config in Plugin

```rust
use extism_pdk::*;

#[plugin_fn]
pub fn get_config() -> FnResult<String> {
    let env = config::get("environment").unwrap_or("development");
    Ok(format!("Running in {} mode", env))
}
```

### JSON Config

```rust
// Host
let config = serde_json::json!({
    "database": "postgres",
    "pool_size": 10
});

let manifest = Manifest::new([Wasm::file("plugin.wasm")])
    .with_config(config);
```

```rust
// Plugin
#[derive(serde::Deserialize)]
struct DbConfig {
    database: String,
    pool_size: u32,
}

#[plugin_fn]
pub fn connect() -> FnResult<String> {
    let config: DbConfig = config::get().unwrap();
    Ok(format!("Connecting to {} with pool {}",
               config.database, config.pool_size))
}
```

## Security Model

### Capability-Based Security

```rust
// Host controls what plugin can do
let manifest = Manifest::new([Wasm::file("plugin.wasm")])
    // Allow specific HTTP endpoints
    .with_allowed_host("api.example.com")
    // No filesystem access by default
    // No network by default
    ;

let plugin = Plugin::new(&manifest, [], true)?;
```

### Resource Limits

```rust
use extism::{Manifest, Wasm, Plugin};
use std::time::Duration;

let manifest = Manifest::new([Wasm::file("plugin.wasm")]);

let mut plugin = Plugin::new(&manifest, [], true)?;

// Set memory limit
plugin.set_max_memory(1024 * 1024 * 64); // 64MB

// Set execution timeout
plugin.set_timeout(Duration::from_secs(5));
```

### Sandboxing

```
Plugin runs in sandbox:
├── Isolated memory space
├── No direct filesystem access
├── No network unless explicitly allowed
├── No host functions unless exposed
└── Execution time limits
```

---

**Next Steps:**
- [01-extism-exploration.md](./01-extism-exploration.md) - Full architecture
- [02-extism-pdk-deep-dive.md](./02-extism-pdk-deep-dive.md) - Plugin development
- [03-extism-security-deep-dive.md](./03-extism-security-deep-dive.md) - Security model
