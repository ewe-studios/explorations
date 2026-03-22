---
name: wit-bindgen
description: WebAssembly Interface Type bindings generator for component model
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/wit-bindgen/
---

# wit-bindgen - Wasm Interface Type Bindings Generator

## Overview

wit-bindgen is a **bindings generator for WebAssembly Interface Types (WIT)** that enables seamless interoperability between WebAssembly components and their hosts. It generates boilerplate code for both importing and exporting WIT-defined interfaces, supporting multiple target languages including Rust, C, C++, and more.

Key features:
- **Component model support** - Full WASM component model implementation
- **Multi-language** - Rust, C, C++, C#, JavaScript, Python bindings
- **Bidirectional** - Generate both imports and exports
- **Type-safe** - Strong typing across language boundaries
- **Async support** - Built-in async/await patterns
- **Resource handling** - Automatic lifetime management

## Directory Structure

```
wit-bindgen/
├── crates/
│   ├── wit-bindgen/             # Core generator library
│   ├── wit-bindgen-rust/        # Rust bindings generator
│   ├── wit-bindgen-c/           # C bindings generator
│   ├── wit-bindgen-cpp/         # C++ bindings generator
│   ├── wit-bindgen-csharp/      # C# bindings generator
│   ├── wit-bindgen-teavm/       # TeaVM (Java) generator
│   ├── wit-bindgen-go/          # Go bindings generator
│   ├── wit-bindgen-python/      # Python bindings generator
│   └── wit-bindgen-core/        # Shared core utilities
├── tests/
│   ├── runtime/                 # Runtime integration tests
│   ├── ui/                      # UI tests for error messages
│   └── wasm/                    # WASM test fixtures
├── build.rs                     # Build script
├── Cargo.toml
└── README.md
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    wit-bindgen Architecture                     │
└─────────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│   WIT Source     │ │  wit-bindgen     │ │   Generated      │
│   (.wit files)   │ │  (Generator)     │ │   Bindings       │
│                  │ │                  │ │                  │
│ interface fs {   │ │  ┌────────────┐  │ │  // Rust exports │
│   read: func(    │ │  │   Parser   │  │ │  impl Fs for   │
│     path: string │ │  │            │  │ │  MyComponent { │
│   ) -> bytes;    │ │  │   Type     │  │ │    fn read(    │
│   write: func(   │ │  │   Check    │  │ │      &self,    │
│     path: string,│ │  │            │  │ │      path: &str│
│     data: bytes  │ │  │   Code     │  │ │    ) -> Vec<u8>│
│   );             │ │  │   Gen      │  │ │  }             │
│ }                │ │  │            │  │ │                  │
│                  │ │  └────────────┘  │ │  // Import stubs │
│ export my-app    │ │                  │ │  extern "C" {    │
│ {                  │ │                  │ │    fn read(    │
│   use fs;          │ │                  │ │      ...       │
│ }                  │ │                  │ │  }             │
└──────────────────┘ └──────────────────┘ └──────────────────┘
```

## WIT Interface Definition Language

### Basic Interface

```wit
// Define a package and interface
package myorg:myapp;

interface filesystem {
    /// Read a file's contents
    read: func(path: string) -> result<bytes, error>;

    /// Write data to a file
    write: func(path: string, data: bytes) -> result<(), error>;

    /// Check if a path exists
    exists: func(path: string) -> bool;

    /// Delete a file
    delete: func(path: string) -> result<(), error>;
}
```

### Types and Records

```wit
interface types {
    /// A user record
    record user {
        id: u64,
        name: string,
        email: string,
        active: bool,
    }

    /// User status enumeration
    enum status {
        active,
        inactive,
        banned,
    }

    /// Result with error context
    variant operation-result {
        success(u32),
        failure(string),
        pending,
    }

    /// Complex nested type
    record config {
        users: list<user>,
        status: status,
        metadata: option<metadata>,
    }

    record metadata {
        version: string,
        created-at: u64,
        tags: list<string>,
    }
}
```

### Resources (Owned Types)

```wit
interface resources {
    /// A database connection handle
    resource connection {
        /// Execute a query
        execute: func(query: string) -> result<list<row>, error>;

        /// Begin a transaction
        begin-transaction: func() -> transaction;

        /// Close the connection
        close: func();
    }

    /// A transaction handle
    resource transaction {
        /// Commit the transaction
        commit: func() -> result<(), error>;

        /// Rollback the transaction
        rollback: func() -> result<(), error>;
    }

    /// Create a new connection
    connect: func(url: string) -> connection;
}
```

### World Definition

```wit
// Define the component's interface to the world
package myorg:myapp;

world api {
    // Import interfaces
    import filesystem;
    import types;
    import resources;

    // Import specific functions
    import logging: interface {
        log: func(level: level, message: string);
    };

    enum level {
        debug,
        info,
        warn,
        error,
    }

    // Export our main functionality
    export my-api: interface {
        process: func(input: bytes) -> bytes;
        transform: func(data: string) -> string;
    };

    // Export a resource
    export processor: resource {
        process: func(input: bytes) -> bytes;
    };
}
```

## Rust Bindings Generation

### Exporting from Rust

```rust
// WIT definition:
// interface handler {
//     handle: func(request: string) -> string;
// }
// export handler;

wit_bindgen::generate!({
    world: "handler",
});

struct MyHandler;

impl Handler for MyHandler {
    fn handle(&self, request: String) -> String {
        format!("Processed: {}", request)
    }
}

export!(MyHandler);

// Usage in lib.rs:
#[no_mangle]
pub extern "C" fn wasm_main() {
    // Component entry point
}
```

### Importing in Rust

```rust
// WIT definition:
// interface logger {
//     log: func(level: u8, message: string);
// }
// import logger;

wit_bindgen::generate!({
    world: "my-world",
    imports: {
        "logger": Logger,
    },
});

struct Logger;

impl LoggerInterface for Logger {
    fn log(level: u8, message: String) {
        println!("[{}] {}", level, message);
    }
}

// Use imported interface
pub fn do_work() {
    Logger::log(1, "Starting work".to_string());
    // ... do work ...
    Logger::log(1, "Work complete".to_string());
}
```

### Async Bindings

```rust
// WIT definition with async functions:
// interface async-handler {
//     fetch: func(url: string) -> result<bytes, error>;
//     process: func(data: bytes) -> bytes;
// }

wit_bindgen::generate!({
    world: "async-world",
    async: true,
});

struct AsyncHandler;

#[async_trait]
impl AsyncHandlerInterface for AsyncHandler {
    async fn fetch(&self, url: String) -> Result<Vec<u8>, Error> {
        // Async HTTP request
        let response = reqwest::get(&url).await?;
        Ok(response.bytes().await?.to_vec())
    }

    async fn process(&self, data: Vec<u8>) -> Vec<u8> {
        // Async processing
        tokio::task::spawn_blocking(move || {
            // CPU-intensive work
            process_data(&data)
        }).await.unwrap()
    }
}

export!(AsyncHandler);
```

### Resource Implementation

```rust
// WIT definition:
// resource database {
//     query: func(sql: string) -> list<row>;
//     close: func();
// }
// export database;

wit_bindgen::generate!({
    world: "db-world",
});

pub struct Database {
    connection: sqlx::PgPool,
}

impl Database {
    pub fn new(url: String) -> Self {
        let connection = sqlx::postgres::PgPool::connect(&url)
            .await
            .unwrap();
        Self { connection }
    }
}

impl DatabaseInterface for Database {
    fn query(&self, sql: String) -> Vec<Row> {
        sqlx::query(&sql)
            .fetch_all(&self.connection)
            .await
            .unwrap()
            .into_iter()
            .map(|r| Row { columns: r.columns() })
            .collect()
    }

    fn close(&self) {
        self.connection.close().await;
    }
}

export!(Database);
```

## CLI Usage

### Generate Bindings

```bash
# Generate Rust bindings
wit-bindgen rust my-interface.wit --out-dir ./generated

# Generate with custom options
wit-bindgen rust my-interface.wit \
    --out-dir ./generated \
    --additional-derives "serde::Serialize,serde::Deserialize" \
    --runtime-path "crate::wit_bindgen_rt"

# Generate C bindings
wit-bindgen c my-interface.wit --out-dir ./c_generated

# Generate for multiple languages
wit-bindgen rust api.wit --out-dir ./rust
wit-bindgen c api.wit --out-dir ./c
wit-bindgen cpp api.wit --out-dir ./cpp
```

### Build Integration

```toml
# Cargo.toml
[build-dependencies]
wit-bindgen-rust = "0.10"

# build.rs
fn main() {
    wit_bindgen_rust::generate(&["wit"]).unwrap();
}
```

```rust
// lib.rs
wit_bindgen::generate!({
    path: "wit",
    world: "my-world",
});

// Use generated code
use crate::my_interface::{MyType, MyFunction};
```

## Type Mappings

### WIT to Rust

```wit
// WIT Type          -> Rust Type
bool                -> bool
s8                  -> i8
u8                  -> u8
s16                 -> i16
u16                 -> u16
s32                 -> i32
u32                 -> u32
s64                 -> i64
u64                 -> u64
f32                 -> f32
f64                 -> f64
char                -> char
string              -> String
list<T>             -> Vec<T>
option<T>           -> Option<T>
result<T, E>        -> Result<T, E>
tuple<T, U>         -> (T, U)
record { ... }      -> struct { ... }
enum { ... }        -> enum { ... }
variant { ... }     -> enum { ... }
resource            -> struct with impl
```

### WIT to C

```wit
// WIT Type          -> C Type
bool                -> bool
s8/u8               -> int8_t/uint8_t
s16/u16             -> int16_t/uint16_t
s32/u32             -> int32_t/uint32_t
s64/u64             -> int64_t/uint64_t
f32/f64             -> float/double
string              -> wit_string_t (struct with ptr/len)
list<T>             -> wit_list_t (struct with ptr/len/size)
option<T>           -> struct { bool is_some; T val; }
result<T, E>        -> struct { bool is_ok; union { T ok; E err }; }
```

## Advanced Features

### Type Conversions

```rust
// Custom type conversion
wit_bindgen::generate!({
    world: "my-world",
    types: {
        "my-interface::MyType" => crate::custom::MyCustomType,
    },
});

// Implement conversion
impl From<generated::MyType> for crate::custom::MyCustomType {
    fn from(value: generated::MyType) -> Self {
        MyCustomType {
            field: value.field,
        }
    }
}
```

### Ownership Modes

```rust
// Owned resources (default)
wit_bindgen::generate!({
    world: "my-world",
    ownership: Generating,  // or Borrowing { duplicate_if_necessary: false }
});

// Borrowed resources for zero-copy
wit_bindgen::generate!({
    world: "my-world",
    ownership: Borrowing {
        duplicate_if_necessary: false,
    },
});
```

### Multiple Worlds

```rust
// Generate from multiple WIT files
wit_bindgen::generate!({
    path: "wit",
    // Combine multiple worlds
    inline: r#"
        world combined {
            import fs: interface {
                read: func(path: string) -> bytes;
            };
            export app: interface {
                run: func() -> result<(), string>;
            };
        }
    "#,
});
```

## Component Model Integration

### Producing Components

```bash
# Compile to core WASM
cargo build --target wasm32-wasi --release

# Create component
wasm-tools component new \
    target/wasm32-wasi/release/my_module.wasm \
    --adapt wasi_snapshot_preview1=wasi_adapter.wasm \
    -o component.wasm

# Or use cargo-component
cargo component new my-component
cargo component build --release
```

### Component Inspection

```bash
# Show component structure
wasm-tools component print component.wasm

# Validate component
wasm-tools component validate component.wasm

# Show types
wasm-tools component types component.wasm
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler() {
        let handler = MyHandler;
        let result = handler.handle("test".to_string());
        assert_eq!(result, "Processed: test");
    }

    #[tokio::test]
    async fn test_async_handler() {
        let handler = AsyncHandler;
        let result = handler.fetch("https://example.com".to_string()).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

```rust
// tests/integration.rs
use wasmtime::{Engine, Store, Module, Linker};
use wasmtime_wasi::WasiCtxBuilder;

#[test]
fn test_component_execution() {
    let engine = Engine::default();
    let component = Component::from_file(&engine, "component.wasm");

    let mut linker = Linker::new(&engine);

    // Link imports
    linker.root().func_wrap("logger", |msg: String| {
        println!("Log: {}", msg);
    });

    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &component).unwrap();

    // Call exported function
    let run = instance.get_func::<(), ()>(&mut store, "run").unwrap();
    run.call(&mut store, ()).unwrap();
}
```

## Related Documents

- [Wasmtime](./wasmtime-runtime-exploration.md) - Runtime support
- [WASI](./wasi-exploration.md) - System interface
- [Wasm Tools](./wasm-tools-exploration.md) - Component tooling

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/wit-bindgen/`
- wit-bindgen GitHub: https://github.com/bytecodealliance/wit-bindgen
- Component Model: https://github.com/WebAssembly/component-model
