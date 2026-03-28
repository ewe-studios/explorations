# Rust Revision: Complete Translation Guide for workerd

**Created:** 2026-03-27

**Status:** Comprehensive translation strategy

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Crate Structure](#crate-structure)
3. [Dependency Mapping](#dependency-mapping)
4. [Type System Translation](#type-system-translation)
5. [Core Components](#core-components)
6. [Memory Management](#memory-management)
7. [Async Model Translation](#async-model-translation)
8. [JSG Equivalent](#jsg-equivalent)
9. [Implementation Roadmap](#implementation-roadmap)

---

## Executive Summary

Translating workerd from C++/KJ to Rust requires careful consideration of:

- **V8 → QuickJS/Wasmtime**: JavaScript/WASM runtime replacement
- **KJ → Tokio/Standard**: Async and type system replacement
- **Cap'n Proto**: Keep (has Rust bindings)
- **JSG → Custom macros**: Binding layer recreation

### Estimated Effort

| Component | LOC (C++) | Estimated Rust LOC | Effort |
|-----------|-----------|-------------------|--------|
| Core runtime | 50,000 | 35,000 | High |
| JSG bindings | 40,000 | 25,000 | High |
| API implementations | 150,000 | 100,000 | Very High |
| I/O layer | 80,000 | 60,000 | High |
| **Total** | **320,000** | **220,000** | **12-18 months** |

---

## Crate Structure

```
workerd-rust/
├── Cargo.toml                    # Workspace definition
├── workerd-core/                 # Core types and traits
│   ├── src/
│   │   ├── lib.rs
│   │   ├── isolate.rs           # JavaScript isolate
│   │   ├── context.rs           # Execution context
│   │   ├── module.rs            # Module system
│   │   └── limits.rs            # Resource limits
│
├── workerd-jsg/                  # JavaScript bindings (rquickjs)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── resource.rs          # Resource types
│   │   ├── promise.rs           # Promise handling
│   │   ├── memory.rs            # GC tracking
│   │   └── macros.rs            # JSG-like macros
│
├── workerd-isolate/              # V8/QuickJS isolation
│   ├── src/
│   │   ├── lib.rs
│   │   ├── isolate.rs           # Isolate management
│   │   ├── lock.rs              # Locking semantics
│   │   └── gc.rs                # Garbage collection
│
├── workerd-actor/                # Durable Objects
│   ├── src/
│   │   ├── lib.rs
│   │   ├── actor.rs             # Actor lifecycle
│   │   ├── cache.rs             # LRU cache
│   │   ├── storage.rs           # SQLite storage
│   │   └── gate.rs              # Input/Output gates
│
├── workerd-api/                  # Web API implementations
│   ├── src/
│   │   ├── lib.rs
│   │   ├── fetch.rs             # fetch() API
│   │   ├── request.rs           # Request object
│   │   ├── response.rs          # Response object
│   │   ├── headers.rs           # Headers object
│   │   ├── streams.rs           # Streams API
│   │   └── websocket.rs         # WebSocket API
│
├── workerd-rpc/                  # Cap'n Proto RPC
│   ├── src/
│   │   ├── lib.rs
│   │   ├── service.rs           # Service bindings
│   │   └── transport.rs         # RPC transport
│
├── workerd-streams/              # Streams implementation
│   ├── src/
│   │   ├── lib.rs
│   │   ├── readable.rs          # ReadableStream
│   │   ├── writable.rs          # WritableStream
│   │   ├── transform.rs         # TransformStream
│   │   └── pipe.rs              # Piping logic
│
├── workerd-http/                 # HTTP implementation
│   ├── src/
│   │   ├── lib.rs
│   │   ├── client.rs            # HTTP client
│   │   ├── server.rs            # HTTP server
│   │   └── tls.rs               # TLS (rustls)
│
├── workerd-server/               # Server binary
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs            # Config parsing
│   │   └── sockets.rs           # Socket management
│
└── workerd-util/                 # Utilities
    ├── src/
    │   ├── lib.rs
    │   ├── sqlite.rs            # SQLite wrapper
    │   ├── uuid.rs              # UUID generation
    │   └── state_machine.rs     # State machine DSL
```

---

## Dependency Mapping

### Core Dependencies

```toml
# workerd-rust/Cargo.toml

[workspace]
members = [
    "workerd-core",
    "workerd-jsg",
    "workerd-isolate",
    "workerd-actor",
    "workerd-api",
    "workerd-rpc",
    "workerd-streams",
    "workerd-http",
    "workerd-server",
    "workerd-util",
]

[workspace.dependencies]
# JavaScript runtime
rquickjs = { version = "0.5", features = ["full"] }
# Alternative: deno_core = "0.280"

# WebAssembly
wasmtime = "18.0"
wasi-common = "18.0"

# Async runtime
tokio = { version = "1.36", features = ["full"] }
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
capnp = "0.19"
capnpc = "0.19"
capnp-rpc = "0.19"

# HTTP
hyper = { version = "1.2", features = ["full"] }
http = "1.1"
http-body = "1.0"
tokio-rustls = "0.25"
rustls = "0.22"
webpki-roots = "0.26"

# Storage
rusqlite = { version = "0.31", features = ["bundled"] }

# Utilities
uuid = { version = "1.7", features = ["v4"] }
bytes = "1.5"
pin-project-lite = "0.2"
thiserror = "1.0"
anyhow = "1.0"

# Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### KJ → Rust Type Mapping

| KJ Type | Rust Equivalent | Notes |
|---------|-----------------|-------|
| `kj::String` | `String` | Owned string |
| `kj::StringPtr` | `&str` | String slice |
| `kj::Array<T>` | `Vec<T>` | Growable array |
| `kj::ArrayPtr<T>` | `&[T]` | Slice |
| `kj::Own<T>` | `Box<T>` | Owned pointer |
| `kj::Rc<T>` | `Arc<T>` | Thread-safe RC |
| `kj::Maybe<T>` | `Option<T>` | Optional |
| `kj::OneOf<T...>` | `enum` | Sum type |
| `kj::Function<T>` | `Box<dyn Fn>` | Function trait |
| `kj::Promise<T>` | `Pin<Box<dyn Future>>` | Async |
| `kj::Exception` | `anyhow::Error` | Error type |

---

## Type System Translation

### JSG Resource Types → Rust

**C++ (JSG):**
```cpp
// api/http.h
class Request: public jsg::Object {
 public:
  kj::StringPtr getMethod();
  kj::String getUrl();
  jsg::Ref<Headers> getHeaders();

  JSG_RESOURCE_TYPE(Request) {
    JSG_READONLY_PROTOTYPE_PROPERTY(method, getMethod);
    JSG_READONLY_PROTOTYPE_PROPERTY(url, getUrl);
    JSG_READONLY_PROTOTYPE_PROPERTY(headers, getHeaders);
  }
};
```

**Rust (rquickjs):**
```rust
// workerd-api/src/request.rs

use rquickjs::{class::Trace, Class, Ctx, Object, Result, Value};
use std::sync::Arc;

#[rquickjs::class]
pub struct Request {
    method: String,
    url: String,
    headers: Arc<Headers>,
}

#[rquickjs::methods]
impl Request {
    #[qjs(get)]
    pub fn method(&self) -> &str {
        &self.method
    }

    #[qjs(get)]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[qjs(get)]
    pub fn headers(&self) -> Arc<Headers> {
        self.headers.clone()
    }

    #[qjs(constructor)]
    pub fn new(ctx: Ctx, input: Value, init: Option<Object>) -> Result<Self> {
        // Constructor logic
    }
}

impl Trace for Request {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a>) {
        // GC tracing if needed
    }
}
```

### JSG Macros → Rust Attribute Macros

```rust
// workerd-jsg/src/macros.rs

// JSG_RESOURCE_TYPE equivalent
pub use workerd_jsg_macros::jsg_resource;

// Usage
#[jsg_resource]
impl Request {
    #[jsg_method]
    pub fn array_buffer(&self, ctx: Ctx) -> Result<ArrayBuffer> {
        // ...
    }

    #[jsg_property(get)]
    pub fn body(&self) -> Option<ReadableStream> {
        // ...
    }

    #[jsg_static]
    pub fn redirect(ctx: Ctx, url: String, status: u16) -> Result<Response> {
        // ...
    }
}
```

---

## Core Components

### Isolate Management

```rust
// workerd-isolate/src/isolate.rs

use rquickjs::{Runtime, Context, Module};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

pub struct Isolate {
    runtime: Runtime,
    context: Context,
    limits: IsolateLimits,
}

pub struct IsolateLock {
    semaphore: Arc<Semaphore>,
    mutex: Arc<Mutex<()>>,
}

impl Isolate {
    pub fn new(limits: IsolateLimits) -> Result<Self, Error> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;

        Ok(Self {
            runtime,
            context,
            limits,
        })
    }

    pub async fn acquire_lock(&self) -> IsolateGuard {
        // Fair locking like workerd's AsyncLock
        let permit = self.semaphore.acquire().await.unwrap();
        let guard = self.mutex.lock().unwrap();

        IsolateGuard { _permit: permit, _guard: guard }
    }

    pub fn execute<F, R>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&Context) -> Result<R, rquickjs::Error>,
    {
        self.context.with(|ctx| f(&ctx).map_err(Error::from))
    }
}

pub struct IsolateGuard<'a> {
    _permit: tokio::sync::SemaphorePermit<'a>,
    _guard: std::sync::MutexGuard<'a, ()>,
}
```

### Module System

```rust
// workerd-core/src/module.rs

use std::collections::HashMap;
use std::sync::Arc;

pub enum ModuleType {
    Esm(Vec<u8>),      // Compiled JS
    Wasm(Vec<u8>),     // WASM binary
    Json(serde_json::Value),
    Text(String),
    Data(Vec<u8>),
}

pub struct ModuleRegistry {
    modules: HashMap<String, Arc<ModuleInfo>>,
    cache: ModuleCache,
}

pub struct ModuleInfo {
    module_type: ModuleType,
    specifiers: Vec<String>,  // Import specifiers
    exports: Vec<String>,     // Export names
}

impl ModuleRegistry {
    pub fn resolve(&self, specifier: &str, referrer: Option<&str>) -> Result<String, ResolveError> {
        // Module resolution algorithm
    }

    pub fn compile(&mut self, specifier: &str, source: &[u8]) -> Result<Arc<ModuleInfo>, CompileError> {
        // Compile and cache
    }
}
```

---

## Memory Management

### GC Handle Management

```rust
// workerd-isolate/src/gc.rs

use rquickjs::{Persistent, Value, Ctx};
use std::cell::RefCell;
use std::collections::HashMap;

pub struct GcHandleStore {
    handles: RefCell<HashMap<usize, GcHandle>>,
    next_id: RefCell<usize>,
}

enum GcHandle {
    Value(Persistent<Value>),
    Object(Persistent<rquickjs::Object>),
    Function(Persistent<rquickjs::Function>),
}

impl GcHandleStore {
    pub fn store(&self, ctx: &Ctx, value: Value) -> GcToken {
        let id = *self.next_id.borrow();
        *self.next_id.borrow_mut() += 1;

        let persistent = Persistent::new(ctx, value);
        self.handles.borrow_mut()
            .insert(id, GcHandle::Value(persistent));

        GcToken(id)
    }

    pub fn get(&self, ctx: &Ctx, token: GcToken) -> Option<Value> {
        self.handles.borrow()
            .get(&token.0)
            .map(|h| match h {
                GcHandle::Value(p) => p.get(ctx),
                _ => unreachable!(),
            })
    }

    pub fn remove(&self, token: GcToken) {
        self.handles.borrow_mut().remove(&token.0);
    }
}

#[derive(Clone, Copy)]
pub struct GcToken(usize);
```

---

## Async Model Translation

### KJ Promises → Rust Futures

```rust
// workerd-core/src/promise.rs

use std::future::Future;
use std::pin::Pin;
use tokio::sync::oneshot;

pub struct Promise<T> {
    inner: Pin<Box<dyn Future<Output = Result<T, Error>> + Send>>,
}

impl<T> Promise<T> {
    pub async fn then<F, U>(self, f: F) -> Promise<U>
    where
        F: FnOnce(T) -> Promise<U> + Send + 'static,
        U: Send + 'static,
    {
        Promise {
            inner: Box::pin(async move {
                let value = self.inner.await?;
                f(value).inner.await
            }),
        }
    }

    pub async fn join_all<I>(promises: I) -> Vec<Result<T, Error>>
    where
        I: IntoIterator<Item = Promise<T>>,
    {
        futures::future::join_all(
            promises.into_iter().map(|p| p.inner)
        ).await
    }
}

// TaskSet equivalent
pub struct TaskSet {
    handles: tokio::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl TaskSet {
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let handle = tokio::spawn(future);
        futures::executor::block_on(async {
            self.handles.lock().await.push(handle);
        });
    }
}
```

---

## JSG Equivalent

### Full JSG Macro Implementation

```rust
// workerd-jsg-macros/src/lib.rs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, ItemFn};

#[proc_macro_attribute]
pub fn jsg_resource(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let impl_item = parse_macro_input!(item as ItemImpl);

    let name = &impl_item.self_ty;

    // Generate TypeScript definition
    let ts_def = generate_ts_def(&impl_item);

    // Generate wrapper code
    let expanded = quote! {
        #impl_item

        impl JsgResource for #name {
            const NAME: &'static str = stringify!(#name);

            fn init_class(ctx: &Ctx<'_>, global: &Object) -> Result<()> {
                let ctor = Class::<#name>::define(global)?;
                // ... class initialization
                Ok(())
            }
        }

        #ts_def
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn jsg_method(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let fn_item = parse_macro_input!(item as ItemFn);
    // Generate method binding
    // ...
    TokenStream::from(fn_item)
}

#[proc_macro_attribute]
pub fn jsg_property(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Generate property getter/setter
    // ...
    TokenStream::from(item)
}
```

---

## Implementation Roadmap

### Phase 1: Foundation (Months 1-3)

- [ ] `workerd-core` - Core types and traits
- [ ] `workerd-jsg` - Basic binding macros
- [ ] `workerd-isolate` - Isolate management with rquickjs
- [ ] Basic module loading

### Phase 2: APIs (Months 4-8)

- [ ] `workerd-api` - Fetch, Request, Response, Headers
- [ ] `workerd-streams` - ReadableStream, WritableStream
- [ ] `workerd-http` - HTTP client/server
- [ ] `workerd-rpc` - Cap'n Proto RPC

### Phase 3: Actor Model (Months 9-12)

- [ ] `workerd-actor` - Durable Objects
- [ ] SQLite storage backend
- [ ] LRU cache implementation
- [ ] Input/Output gates

### Phase 4: Production (Months 13-18)

- [ ] Performance optimization
- [ ] Testing and compatibility
- [ ] Documentation
- [ ] Production deployment

---

## Key Challenges

### Challenge 1: V8 vs QuickJS

| Aspect | V8 | QuickJS |
|--------|-----|---------|
| Performance | Excellent | Good |
| Memory | High | Low |
| Features | Full ES2024 | ES2020 |
| Embedding | Complex | Simple |

**Mitigation:** Consider `deno_core` (V8-based) for production.

### Challenge 2: GC Integration

Rust's ownership model conflicts with GC:
- Need `Persistent` handles for long-lived refs
- Manual GC root management
- Risk of memory leaks

**Solution:** Use RAII patterns with explicit `GcHandleStore`.

### Challenge 3: Async Bridging

KJ async ≠ Tokio:
- Different cancellation models
- Different error handling
- Different task spawning

**Solution:** Create adapter layer with unified `Promise<T>` type.

---

## References

- [rquickjs Documentation](https://docs.rs/rquickjs/)
- [Deno Core](https://docs.rs/deno_core/)
- [Cap'n Proto Rust](https://capnproto.org/capnp-rust/)
- [Tokio Documentation](https://tokio.rs/)
