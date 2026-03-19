---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.cloudflare/Others/workers-rs
repository: https://github.com/cloudflare/workers-rs
revised_at: 2026-03-19
---

# workers-rs Deep Dive: Rust SDK for Cloudflare Workers

## Overview

workers-rs is the Rust SDK for Cloudflare Workers, providing idiomatic Rust bindings for the Workers runtime. It enables developers to write Workers in Rust with full access to the Workers API surface.

## Workspace Structure

```
workers-rs/
├── worker/                    # Main library crate
│   ├── src/
│   │   ├── lib.rs             # Public API exports
│   │   ├── env.rs             # Environment bindings (Env, Secret, Var)
│   │   ├── request.rs         # Request handling
│   │   ├── response.rs        # Response handling
│   │   ├── router.rs          # HTTP router
│   │   ├── durable/           # Durable Objects
│   │   ├── kv.rs              # KV namespace bindings
│   │   ├── r2/                # R2 object storage
│   │   ├── queue/             # Queues (feature-gated)
│   │   ├── d1/                # D1 database (feature-gated)
│   │   ├── websocket.rs       # WebSocket handling
│   │   ├── cache.rs           # Cache API
│   │   ├── http/              # HTTP crate integration
│   │   └── send.rs            # Send helpers for async
│   └── Cargo.toml
├── worker-macros/             # Procedural macros
│   ├── src/
│   │   ├── lib.rs             # Macro exports
│   │   ├── event.rs           # #[event] macro
│   │   ├── durable_object.rs  # #[durable_object] macro
│   │   └── send.rs            # #[send] macro
│   └── Cargo.toml
├── worker-sys/                # wasm-bindgen sys bindings
├── worker-kv/                 # KV client library
├── worker-build/              # Build tooling
├── examples/                  # Example Workers
│   ├── rpc-server/            # RPC server example
│   ├── rpc-client/            # RPC client example
│   ├── axum/                  # Axum framework integration
│   └── ...
└── Cargo.toml                 # Workspace root
```

## Feature Flags

```toml
[features]
default = []
http = ["dep:http", "dep:http-body"]           # http::Request/Response types
queue = []                                      # Queue handler support
d1 = []                                         # D1 database bindings
rpc = []                                        # RPC support
```

### HTTP Feature

The `http` feature replaces custom types with standard `http` crate types:

```rust
// With http feature enabled:
pub type HttpRequest = ::http::Request<worker::Body>;
pub type HttpResponse = ::http::Response<worker::Body>;

// Body implements http_body::Body
pub use http::body::Body;  // Wraps web_sys::ReadableStream
```

**Benefits:**
- Compatibility with ecosystem crates (axum, hyper, etc.)
- Standard types for headers, methods, status codes
- Easier integration with existing Rust HTTP code

## Event Handler Macros

### #[event(fetch)] - HTTP Request Handler

```rust
use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    Response::ok("Hello World")
}
```

**Expanded signature:**
```rust
#[wasm_bindgen]
pub async fn main(
    req: web_sys::Request,
    env: Env,
    ctx: Context
) -> Result<web_sys::Response, Box<dyn std::error::Error>>
```

### #[event(scheduled)] - Cron Handler

```rust
#[event(scheduled)]
pub async fn main(event: ScheduledEvent, env: Env, ctx: ScheduleContext) -> Result<()> {
    console_log!("Running scheduled job: {}", event.cron());
    Ok(())
}
```

### #[event(queue)] - Queue Consumer (feature-gated)

```rust
#[event(queue)]
pub async fn main(message_batch: MessageBatch<MyType>, env: Env, ctx: Context) -> Result<()> {
    for msg in message_batch.messages() {
        console_log!("Processing: {:?}", msg.body());
    }
    Ok(())
}
```

### #[event(start)] - WASM Start Function

```rust
#[event(start)]
pub fn main() {
    // Runs once when Worker starts
    initialize_cache();
}
```

### respond_with_errors Attribute

```rust
// Automatic error-to-response conversion
#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    // On error, returns 500 with error message in status text
    my_handler(req, env).await
}
```

## Core Types

### Env - Environment Bindings

```rust
pub struct Env {
    inner: worker_sys::Env,
}

impl Env {
    /// Get a binding (KV, R2, D1, etc.)
    pub fn get_binding(&self, name: &str) -> Result<JsValue>;

    /// Get a secret value
    pub fn secret(&self, name: &str) -> Result<Secret>;

    /// Get a variable
    pub fn var(&self, name: &str) -> Result<Var>;

    /// Get KV namespace
    pub fn kv(&self, name: &str) -> Result<KvStore>;

    /// Get R2 bucket
    pub fn bucket(&self, name: &str) -> Result<R2Bucket>;

    /// Get D1 database
    #[cfg(feature = "d1")]
    pub fn db(&self, name: &str) -> Result<D1Database>;
}
```

### Secret - Secure Environment Variables

```rust
pub struct Secret {
    inner: worker_sys::Secret,
}

impl Secret {
    /// Get secret value as string
    pub fn value(&self) -> String;

    /// Get as JsValue
    pub fn inner(&self) -> JsValue;
}
```

### Request - HTTP Request Wrapper

```rust
pub struct Request {
    inner: web_sys::Request,
    body: Option<Body>,
    parsed_headers: Headers,
    cf: Option<Cf>,
    method: Method,
    url: Url,
}

impl Request {
    pub fn method(&self) -> Method;
    pub fn url(&self) -> &Url;
    pub fn headers(&self) -> &Headers;
    pub fn cf(&self) -> &Cf;

    /// Get body as text
    pub async fn text(&mut self) -> Result<String>;

    /// Get body as JSON
    pub async fn json<T: DeserializeOwned>(&mut self) -> Result<T>;

    /// Get body as bytes
    pub async fn bytes(&mut self) -> Result<Vec<u8>>;

    /// Get form data
    pub async fn form_data(&mut self) -> Result<FormDataProvider>;

    /// Get protobuf
    pub async fn protobuf(&mut self) -> Result<Vec<u8>>;
}
```

### Cf - Cloudflare-Specific Request Properties

```rust
pub struct Cf {
    inner: JsValue,
}

impl Cf {
    /// ASN information
    pub fn asn(&self) -> Option<u32>;

    /// Country code
    pub fn country(&self) -> Option<String>;

    /// HTTP protocol (e.g., "HTTP/2")
    pub fn http_protocol(&self) -> Option<String>;

    /// Request priority
    pub fn request_priority(&self) -> Option<String>;

    /// TLS client auth (if mTLS enabled)
    pub fn tls_client_auth(&self) -> Option<TlsClientAuth>;

    /// Edge region
    pub fn edge_request_region(&self) -> Option<String>;
}

pub struct TlsClientAuth {
    pub cert_issuer_dn_legacy: String,
    pub cert_issuer_dn: String,
    pub cert_issuer_serial: String,
    pub cert_subject_dn_legacy: String,
    pub cert_subject_dn: String,
    pub cert_verified: String,
    pub cert_not_before: String,
    pub cert_not_after: String,
    pub cert_subject_alt_name: String,
    pub cert_presented: String,
    pub cert_fingerprint: String,
}
```

### Response - HTTP Response Builder

```rust
pub struct Response {
    inner: web_sys::Response,
    body: Option<ResponseBody>,
}

impl Response {
    /// Simple response with status code
    pub fn ok(text: impl Into<String>) -> Result<Self>;
    pub fn error(text: impl Into<String>, status: u16) -> Result<Self>;

    /// Builder pattern
    pub fn builder() -> ResponseBuilder;

    /// JSON response
    pub fn json<T: Serialize>(value: &T) -> Result<Self>;

    /// Redirect response
    pub fn redirect(url: &str, status: u16) -> Result<Self>;

    /// WebSocket response
    pub fn websocket(ws: &WebSocket, protocol: Option<&str>) -> Result<Self>;
}

pub struct ResponseBuilder {
    status: u16,
    headers: Headers,
    encode_body: EncodeBody,
}

impl ResponseBuilder {
    pub fn status(mut self, status: u16) -> Self;
    pub fn header(mut self, key: &str, value: &str) -> Self;
    pub fn encode(mut self, encode: EncodeBody) -> Self;
    pub fn body(mut self, body: impl Into<ResponseBody>) -> Result<Response>;
}
```

## Router - HTTP Path Routing

```rust
pub struct Router<T = ()> {
    routes: Vec<Route<T>>,
}

impl<T> Router<T> {
    pub fn new() -> Self;

    /// Add GET route
    pub fn get(mut self, path: &str, handler: fn(Request, RouteContext<T>) -> Result<Response>) -> Self;

    /// Add POST route
    pub fn post(mut self, path: &str, handler: fn(Request, RouteContext<T>) -> Result<Response>) -> Self;

    /// Add route with method
    pub fn on(mut self, method: Method, path: &str, handler: fn(Request, RouteContext<T>) -> Result<Response>) -> Self;

    /// Run router
    pub async fn run(self, req: Request, env: Env) -> Result<Response> {
        // Match path and method, invoke handler
    }
}

pub struct RouteContext<T> {
    request: Request,
    env: Env,
    data: T,
    params: RouteParams,
}

impl<T> RouteContext<T> {
    pub fn param(&self, name: &str) -> Option<&str>;
    pub fn data(&self) -> &T;
    pub fn env(&self) -> &Env;
    pub fn request(&self) -> &Request;
}
```

**Usage Example:**

```rust
#[event(fetch)]
async fn main(req: Request, env: Env) -> Result<Response> {
    let router = Router::new()
        .get("/", |_, _| Response::ok("Home"))
        .get("/user/:id", |_, ctx| {
            let id = ctx.param("id").unwrap();
            Response::ok(format!("User {}", id))
        })
        .post("/api/data", |mut req, _| async move {
            let json: MyData = req.json().await?;
            Response::json(&json)
        });

    router.run(req, env).await
}
```

## Durable Objects

### Definition

```rust
use worker::*;

#[durable_object]
pub struct Counter {
    count: u32,
    state: State,
    env: Env,
}

#[durable_object]
impl DurableObject for Counter {
    fn new(state: State, env: Env) -> Self {
        Self { count: 0, state, env }
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        match req.path().as_str() {
            "/increment" => {
                self.count += 1;
                self.state.storage().put("count", self.count).await?;
                Response::ok(self.count.to_string())
            }
            "/get" => {
                let count = self.state.storage().get("count").await?;
                Response::ok(count)
            }
            _ => Response::error("Not found", 404)
        }
    }
}
```

### Usage from Worker

```rust
#[event(fetch)]
async fn main(req: Request, env: Env) -> Result<Response> {
    // Get Durable Object namespace
    let namespace = env.durable_object("COUNTER")?;

    // Get or create stub by ID
    let stub = namespace.id_from_name("counter-1")?.get_stub()?;

    // Forward request to DO
    stub.fetch_with_request(req).await
}
```

## KV Store Bindings

```rust
pub struct KvStore {
    inner: worker_sys::KvNamespace,
}

impl KvStore {
    /// Get value
    pub async fn get(&self, key: &str) -> Result<Option<KvValue>>;

    /// Get with metadata
    pub async fn get_with_metadata<T: DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<(KvValue, Option<T>)>>;

    /// Put value
    pub async fn put(&self, key: &str, value: impl Into<KvValue>) -> Result<()>;

    /// Put with expiration
    pub async fn put_with_expiration(
        &self,
        key: &str,
        value: impl Into<KvValue>,
        expiration: u64,
    ) -> Result<()>;

    /// Delete key
    pub async fn delete(&self, key: &str) -> Result<()>;

    /// List keys
    pub async fn list(&self) -> Result<KvList>;
}
```

## R2 Object Storage

```rust
pub struct R2Bucket {
    inner: worker_sys::R2Bucket,
}

impl R2Bucket {
    /// Get object
    pub async fn get(&self, key: &str) -> Result<Option<R2Object>>;

    /// Put object
    pub async fn put(&self, key: &str, value: impl Into<R2PutOptions>) -> Result<R2Object>;

    /// Delete object
    pub async fn delete(&self, key: &str) -> Result<()>;

    /// List objects
    pub async fn list(&self, options: R2ListOptions) -> Result<R2Objects>;
}

pub struct R2Object {
    pub key: String,
    pub size: u64,
    pub uploaded: js_sys::Date,
    pub http_metadata: Option<R2HttpMetadata>,
    pub custom_metadata: Option<std::collections::HashMap<String, String>>,
}
```

## D1 Database (feature-gated)

```rust
#[cfg(feature = "d1")]
pub struct D1Database {
    inner: worker_sys::D1Database,
}

#[cfg(feature = "d1")]
impl D1Database {
    /// Execute query
    pub async fn exec(&self, query: &str) -> Result<D1Result>;

    /// Execute with parameters
    pub async fn exec_with_params(
        &self,
        query: &str,
        params: &[JsValue],
    ) -> Result<D1Result>;

    /// First row
    pub async fn first<T: DeserializeOwned>(&self, query: &str) -> Result<Option<T>>;

    /// All rows
    pub async fn all<T: DeserializeOwned>(&self, query: &str) -> Result<Vec<T>>;
}

/// Macro for prepared statements
#[cfg(feature = "d1")]
#[macro_export]
macro_rules! query {
    ($db:expr, $query:literal, $($args:expr),*) => {
        $db.exec_with_params($query, &[$($args.into()),*])
    };
}

// Usage
let result = query!(db, "SELECT * FROM users WHERE id = ?", user_id);
```

## Queue Consumer (feature-gated)

```rust
#[cfg(feature = "queue")]
pub struct MessageBatch<T> {
    inner: worker_sys::MessageBatch,
    _phantom: PhantomData<T>,
}

#[cfg(feature = "queue")]
impl<T: DeserializeOwned> MessageBatch<T> {
    /// Get all messages
    pub fn messages(&self) -> Vec<Message<T>>;

    /// Queue name
    pub fn queue(&self) -> String;
}

#[cfg(feature = "queue")]
pub struct Message<T> {
    pub id: String,
    pub timestamp: js_sys::Date,
    pub body: T,
    pub attempts: u32,
}
```

## WebSocket Handling

```rust
pub struct WebSocket {
    inner: web_sys::WebSocket,
}

impl WebSocket {
    /// Accept incoming WebSocket
    pub fn accept(&self) -> Result<()>;

    /// Send message
    pub fn send(&self, message: impl Into<JsValue>) -> Result<()>;

    /// Close connection
    pub fn close(&self, code: u16, reason: &str) -> Result<()>;

    /// Serialize message
    pub fn serialize_message(msg: impl Into<JsValue>) -> Result<JsValue>;
}

// WebSocket events
pub enum WebSocketEventType {
    Message,
    Close,
    Error,
}

pub struct WebSocketEvent {
    pub ws: WebSocket,
    pub event: WebSocketEventType,
}
```

## Cache API

```rust
pub struct Cache {
    inner: CacheBinding,
}

impl Cache {
    /// Get default cache
    pub fn default() -> Result<Self>;

    /// Match request in cache
    pub async fn match_request(&self, req: impl Into<CacheKey>) -> Result<Option<Response>>;

    /// Put response in cache
    pub async fn put(&self, req: impl Into<CacheKey>, res: Response) -> Result<()>;

    /// Delete from cache
    pub async fn delete(&self, req: impl Into<CacheKey>) -> Result<CacheDeletionOutcome>;
}

pub enum CacheKey {
    String(String),
    Request(Request),
}
```

## Send Helpers for Async

### Problem

JavaScript types are not `Send`, but frameworks like axum require `Send` handlers:

```rust
// JsFuture is !Send
let fut = JsFuture::from(promise);
fut.await  // Makes entire function !Send
```

### Solution: #[worker::send] Macro

```rust
#[worker::send]
async fn handler(Extension(env): Extension<Env>) -> Response<String> {
    let kv = env.kv("FOO").unwrap()?;  // KvStore is !Send
    let value = kv.get("foo").text().await?;  // OK: function is Send
    Ok(format!("Got value: {:?}", value))
}
```

### send::SendFuture Wrapper

```rust
use worker::send::SendFuture;

// Wrap !Send future to make it Send
let fut = SendFuture::new(async move {
    JsFuture::from(promise).await  // JsFuture is !Send
});

// fut is now Send
```

### send::SendWrapper

```rust
use worker::send::SendWrapper;

// Wrap !Send type to make it Send
let store = env.kv("FOO")?;  // KvStore is !Send
let state = SendWrapper::new(store);  // Now Send

// Use with axum state
let router = axum::Router::new()
    .layer(Extension(state));
```

## RPC Support (Experimental)

### Server-Side

```rust
// Export methods via wasm-bindgen
#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen]
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    #[wasm_bindgen]
    pub fn multiply(&self, a: i32, b: i32) -> i32 {
        a * b
    }
}
```

### Client-Side

```rust
// Manual wasm-bindgen bindings
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Calculator")]
    pub type Calculator;

    #[wasm_bindgen(method)]
    pub fn add(this: &Calculator, a: i32, b: i32) -> i32;
}

// Usage
let calc: Calculator = /* get binding */;
let result = calc.add(2, 3);
```

### WIT Code Generation (Experimental)

```wit
// wit/calculator.wit
interface calculator {
    add: func(a: s32, b: s32) -> s32;
    multiply: func(a: s32, b: s32) -> s32;
}
```

```rust
// build.rs generates bindings from WIT
fn main() {
    workers_rs_build::generate_wit_bindings("wit/");
}
```

## WASM Optimization

### Release Profile Configuration

```toml
[profile.release]
opt-level = "z"           # Optimize for size
codegen-units = 1         # Single codegen unit for better optimization
lto = true                # Link-time optimization

[profile.release.package."*"]
codegen-units = 1
opt-level = "z"
```

**Effects:**
- `opt-level = "z"`: Produces smaller binaries than `opt-level = "s"`
- `codegen-units = 1`: Slower compilation but better inlining
- `lto = true`: Cross-crate optimization at link time

## Error Handling

### Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Rust Error: {0}")]
    RustError(String),

    #[error("Error parsing JSON: {0}")]
    JsonParsing(#[from] serde_json::Error),

    #[error("Error parsing headers: {0}")]
    HeaderParsing(#[from] http::Error),

    #[error("Internal error: {0}")]
    InternalError(String),
}
```

### Result Type Alias

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

## Logging

```rust
use worker::{console_log, console_error, console_warn, console_debug};

console_log!("Info message");
console_debug!("Debug message");
console_warn!("Warning message");
console_error!("Error message");
```

## CORS Helper

```rust
pub struct Cors {
    origins: Vec<String>,
    methods: Vec<Method>,
    headers: Vec<String>,
    max_age: Option<u32>,
}

impl Cors {
    pub fn new() -> Self;
    pub fn with_origins(mut self, origins: &[&str]) -> Self;
    pub fn with_methods(mut self, methods: &[Method]) -> Self;
    pub fn with_headers(mut self, headers: &[&str]) -> Self;
    pub fn with_max_age(mut self, max_age: u32) -> Self;

    pub fn preflight(self, req: &Request) -> Result<Response>;
    pub fn with_headers(self, res: Response) -> Response;
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    async fn test_handler() {
        let req = Request::new("http://test.com/", Method::Get).unwrap();
        let env = /* mock env */;
        let ctx = Context::new();

        let res = main(req, env, ctx).await.unwrap();
        assert_eq!(res.status_code(), 200);
    }
}
```

## References

- [workers-rs GitHub](https://github.com/cloudflare/workers-rs)
- [Cloudflare Workers Documentation](https://developers.cloudflare.com/workers/)
- [Rust and WebAssembly Book](https://rustwasm.github.io/docs/book/)
