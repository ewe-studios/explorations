# tinyhttp: Modern Rust HTTP Server with Procedural Macros

## Overview

**tinyhttp** is a modern HTTP server library for Rust that emphasizes developer experience through procedural macro-based routing. At version 0.5.0, it offers both synchronous and asynchronous operation modes.

**Key Design Goals:**
- Express-like developer experience
- Procedural macro routing
- Optional async/await support
- Built-in compression and SSL

---

## Architecture Deep Dive

### Crate Structure

tinyhttp uses a workspace with three crates:

```
tinyhttp/                    # Main crate - public API re-exports
├── Depends on: tinyhttp-internal
└── Depends on: tinyhttp-codegen

tinyhttp-internal/           # Core implementation
├── http.rs                  # Synchronous HTTP handling
├── async_http.rs            # Asynchronous HTTP handling
├── request.rs               # Request types
├── response.rs              # Response types
├── config.rs                # Server configuration
└── codegen/                 # Route trait definitions

tinyhttp-codegen/            # Procedural macros
└── lib.rs                   # #[get], #[post] implementations
```

### Server Lifecycle

```
┌─────────────────────────────────────────────────────────────────┐
│                    HttpListener::new()                           │
│                                                                  │
│  1. Bind TcpListener                                             │
│  2. Create Config with routes                                    │
│  3. Initialize ThreadPool (rusty_pool)                           │
│  4. Setup SSL if enabled                                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Connection Loop                                │
│                                                                  │
│  for stream in socket.accept() {                                 │
│    if use_pool {                                                 │
│      pool.execute(|| parse_request(stream, config));             │
│    } else {                                                      │
│      parse_request(stream, config);                              │
│    }                                                             │
│  }                                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Request Processing                             │
│                                                                  │
│  1. Read bytes from socket                                       │
│  2. Parse headers and body                                       │
│  3. Apply request middleware                                     │
│  4. Route matching                                               │
│  5. Execute handler                                              │
│  6. Apply response middleware                                    │
│  7. Send response                                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Procedural Macro Routing

### The #[get] and #[post] Macros

The standout feature of tinyhttp is its Express-like routing:

```rust
use tinyhttp::prelude::*;

#[get("/")]
fn index() -> &'static str {
    "Hello, World!"
}

#[get("/hello/:name")]
fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[post("/echo")]
fn echo(req: Request) -> String {
    req.get_parsed_body().unwrap().to_string()
}

fn main() {
    let socket = TcpListener::bind("0.0.0.0:8080").unwrap();
    let routes = Routes::new(vec![index(), hello(), echo()]);
    let config = Config::new().routes(routes);
    let http = HttpListener::new(socket, config);
    http.start();
}
```

### Macro Expansion

The `#[get]` macro expands to:

```rust
// Input:
#[get("/hello/:name")]
fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}

// Expands to approximately:
fn hello() -> Box<dyn Route> {
    let mut get_route = GetRouteWithReqAndRes::new()
        .set_path("/hello".into());

    fn body<'b>(try_from_req: &'b mut Request, _sock: &'b mut TcpStream) -> Response {
        let name: &str = try_from_req.get_wildcard().unwrap();
        format!("Hello, {}!", name).into()
    }

    get_route = get_route.set_body(body);
    get_route = get_route.set_wildcard("name".into());

    Box::new(get_route)
}
```

### Macro Implementation

```rust
#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn: syn::ItemFn = syn::parse(item).unwrap();
    let value: syn::LitStr = syn::parse(attr).unwrap();

    let sig = item_fn.sig;
    let name = sig.ident.clone();
    let body = item_fn.block.deref();
    let return_type = sig.output;
    let fn_args = sig.inputs;

    let mut path = value.value();

    // Handle wildcard routes like /user/:id
    let new_wildcard = if path.contains("/:") {
        let path_clone = path.clone();
        let mut iter = path_clone.split(':');
        path = iter.next().unwrap().to_string();
        let id = iter.next().unwrap().to_string();
        if path.len() != 1 {
            path.pop();
        }
        quote! { get_route = get_route.set_wildcard(#id.into()); }
    } else {
        quote! {}
    };

    // Generate route registration code
    let new_get_body = if !fn_args.is_empty() {
        quote! {
            let mut get_route = GetRouteWithReqAndRes::new()
                .set_path(#path.into());

            fn body<'b>(try_from_req: &'b mut Request, _sock: &'b mut TcpStream) -> Response {
                let arg = try_from_req.into();
                #body.into()
            }

            get_route = get_route.set_body(body);
        }
    } else {
        quote! {
            let mut get_route = BasicGetRoute::new()
                .set_path(#path.into());

            fn body() -> Response {
                #body.into()
            }

            get_route = get_route.set_body(body);
        }
    };

    let output = quote! {
        fn #name() -> Box<dyn Route> {
            #new_get_body
            #new_wildcard
            Box::new(get_route)
        }
    };

    output.into()
}
```

---

## Request Handling

### Request Structure

```rust
#[derive(Clone, Debug, Default)]
pub struct Request {
    raw_headers: HashMap<String, String>,
    status_line: Vec<String>,      // [method, path, version]
    body: Vec<u8>,
    wildcard: Option<String>,      // Extracted from URL
    is_http2: bool,
}

impl Request {
    pub fn get_headers(&self) -> &HashMap<String, String> {
        &self.raw_headers
    }

    pub fn get_parsed_body(&self) -> Option<&str> {
        std::str::from_utf8(&self.body).ok()
    }

    pub fn get_raw_body(&self) -> &[u8] {
        &self.body
    }

    pub fn get_status_line(&self) -> &[String] {
        &self.status_line
    }

    pub fn get_wildcard(&self) -> Option<&String> {
        self.wildcard.as_ref()
    }
}
```

### Synchronous Request Parsing

```rust
fn build_and_parse_req<P: Read>(conn: &mut P) -> Result<Request, RequestError> {
    let mut buf_reader = BufReader::new(conn);
    let mut status_line_str = String::new();

    // Read status line
    buf_reader.read_line(&mut status_line_str).unwrap();
    status_line_str.drain(status_line_str.len() - 2..status_line_str.len());

    // Find header end (\r\n\r\n)
    let iter = buf_reader.fill_buf().unwrap();
    let header_end_idx = iter.windows(4)
        .position(|w| matches!(w, b"\r\n\r\n"))
        .unwrap();

    let headers_buf = iter[..header_end_idx + 2].to_vec();
    buf_reader.consume(header_end_idx + 4);

    // Parse headers
    let mut headers = HashMap::new();
    let mut headers_index = 0;
    let mut headers_buf_iter = headers_buf.windows(2).enumerate();

    while let Some(header_index) = headers_buf_iter
        .find(|(_, w)| matches!(*w, b"\r\n"))
        .map(|(i, _)| i)
    {
        let header = std::str::from_utf8(&headers_buf[headers_index..header_index])
            .unwrap()
            .to_lowercase();

        if header.is_empty() {
            break;
        }

        headers_index = header_index + 2;
        let mut colon_split = header.splitn(2, ':');
        headers.insert(
            colon_split.next().unwrap().to_string(),
            colon_split.next().unwrap().trim().to_string(),
        );
    }

    // Read body based on Content-Length
    let body_len = headers
        .get("content-length")
        .unwrap_or(&String::from("0"))
        .parse::<usize>()
        .unwrap();

    let mut raw_body = vec![0; body_len];
    buf_reader.read_exact(&mut raw_body).unwrap();

    Ok(Request::new(
        raw_body,
        headers,
        status_line_str.split_whitespace().map(|s| s.to_string()).collect(),
        None,
    ))
}
```

**Key observations:**
- Buffer-based parsing (not line-by-line)
- Window search for `\r\n\r\n` boundary
- Lowercase header normalization
- Content-Length based body reading

---

## Response Handling

### Response Builder Pattern

```rust
#[derive(Clone, Debug)]
pub struct Response {
    pub headers: HashMap<String, String>,
    pub status_line: String,
    pub body: Option<Vec<u8>>,
    pub mime: Option<String>,
    pub http2: bool,
    pub manual_override: bool,
}

impl Response {
    pub fn new() -> Response {
        Response {
            headers: HashMap::new(),
            status_line: String::new(),
            body: None,
            mime: Some(String::from("HTTP/1.1 200 OK")),
            http2: false,
            manual_override: false,
        }
    }

    pub fn status_line<P: Into<String>>(mut self, line: P) -> Self {
        let line_str = line.into();
        self.status_line = line_str.trim().to_string() + "\r\n";
        self
    }

    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn mime<P>(mut self, mime: P) -> Self
    where
        P: Into<String>,
    {
        self.mime = Some(mime.into());
        self
    }

    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn send<P: Read + Write>(self, sock: &mut P) {
        let line_bytes = self.status_line.as_bytes();
        let mut header_bytes: Vec<u8> = self
            .headers
            .into_iter()
            .flat_map(|(i, j)| {
                [(i + ": ").as_bytes(), (j + "\r\n").as_bytes()].concat()
            })
            .collect();

        header_bytes.extend(b"\r\n");

        let full_req: &[u8] = &[
            line_bytes,
            header_bytes.as_slice(),
            self.body.as_ref().unwrap(),
        ].concat();

        sock.write_all(full_req).unwrap();
    }
}
```

### Automatic MIME Type Inference

```rust
let inferred_mime = if let Some(mime_inferred) = infer::get(response.body.as_ref().unwrap()) {
    mime_inferred.mime_type()
} else {
    mime.as_str()
};

response.headers.extend([
    ("Content-Type".to_string(), inferred_mime.to_owned()),
    ("tinyhttp".to_string(), env!("CARGO_PKG_VERSION").to_string()),
]);
```

Uses the `infer` crate to detect content type from magic bytes.

---

## Async Mode

### Tokio-based Async

When the `async` feature is enabled:

```rust
#[cfg(feature = "async")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(feature = "async")]
pub(crate) async fn start_http(http: HttpListener) {
    loop {
        let config = http.config.clone();
        select! {
            result = http.socket.accept() => {
                let (mut conn, _) = result.unwrap();
                parse_request(&mut conn, config).await;
            }
        }
    }
}

#[cfg(feature = "async")]
pub(crate) async fn parse_request<P: AsyncReadExt + AsyncWriteExt + Unpin>(
    conn: &mut P,
    mut config: Config,
) {
    let buf = read_stream(conn).await;
    let request = build_and_parse_req(buf);
    // ... process request ...
    response.send(conn).await;
}
```

### Async Request Reading

```rust
pub(crate) async fn read_stream<P: AsyncReadExt + Unpin>(
    stream: &mut P,
) -> Vec<u8> {
    let buffer_size = 1024;
    let mut request_buffer = vec![];

    loop {
        let mut buffer = vec![0; buffer_size];
        match stream.read(&mut buffer).await {
            Ok(n) => {
                if n == 0 {
                    break;
                } else if n < buffer_size {
                    request_buffer.append(&mut buffer[..n].to_vec());
                    break;
                } else {
                    request_buffer.append(&mut buffer);
                }
            }
            Err(e) => {
                eprintln!("Error: Could not read string!: {}", e);
                std::process::exit(1);
            }
        }
    }

    request_buffer
}
```

---

## Configuration System

### Config Builder

```rust
#[derive(Clone)]
pub struct Config {
    mount_point: Option<String>,
    get_routes: Option<HashMap<String, Box<dyn Route>>>,
    post_routes: Option<HashMap<String, Box<dyn Route>>>,
    debug: bool,
    pub ssl: bool,
    ssl_chain: Option<String>,
    ssl_priv: Option<String>,
    headers: Option<HashMap<String, String>>,
    br: bool,
    gzip: bool,
    spa: bool,
    http2: bool,
    response_middleware: Option<Arc<Mutex<dyn FnMut(&mut Response) + Send + Sync>>>,
    request_middleware: Option<Arc<Mutex<dyn FnMut(&mut Request) + Send + Sync>>>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            mount_point: None,
            get_routes: None,
            post_routes: None,
            debug: false,
            ssl: false,
            ssl_chain: None,
            ssl_priv: None,
            headers: None,
            gzip: false,
            br: false,
            spa: false,
            http2: false,
            request_middleware: None,
            response_middleware: None,
        }
    }

    pub fn routes(mut self, routes: Routes) -> Self {
        let mut get_routes = HashMap::new();
        let mut post_routes = HashMap::new();

        for route in routes.get_stream() {
            match route.get_method() {
                Method::GET => {
                    get_routes.insert(route.get_path().to_string(), route);
                }
                Method::POST => {
                    post_routes.insert(route.get_path().to_string(), route);
                }
            }
        }

        self.get_routes = Some(get_routes).filter(|r| !r.is_empty());
        self.post_routes = Some(post_routes).filter(|r| !r.is_empty());
        self
    }

    pub fn ssl(mut self, ssl_chain: String, ssl_priv: String) -> Self {
        self.ssl_chain = Some(ssl_chain);
        self.ssl_priv = Some(ssl_priv);
        self.ssl = true;
        self
    }

    pub fn gzip(mut self, res: bool) -> Self {
        self.gzip = res;
        self
    }

    pub fn mount_point<P: Into<String>>(mut self, path: P) -> Self {
        self.mount_point = Some(path.into());
        self
    }

    pub fn spa(mut self, res: bool) -> Self {
        self.spa = res;
        self
    }

    pub fn request_middleware<F: FnMut(&mut Request) + Send + Sync + 'static>(
        mut self,
        middleware_fn: F,
    ) -> Self {
        self.request_middleware = Some(Arc::new(Mutex::new(middleware_fn)));
        self
    }
}
```

### Route Matching

```rust
pub fn get_routes(&self, req_path: &str) -> Option<&dyn Route> {
    // Normalize path (remove trailing slash)
    let req_path = if req_path.ends_with('/') && req_path.matches('/').count() > 1 {
        let mut chars = req_path.chars();
        chars.next_back();
        chars.as_str()
    } else {
        req_path
    };

    let routes = self.get_routes.as_ref()?;

    // Exact match first
    if let Some(route) = routes.get(req_path) {
        return Some(route.deref());
    }

    // Wildcard match
    if let Some((_, wildcard_route)) = routes
        .iter()
        .find(|(path, route)| req_path.starts_with(*path) && route.wildcard().is_some())
    {
        return Some(wildcard_route.deref());
    }

    None
}
```

---

## Compression

### GZIP Support

```rust
#[cfg(feature = "sys")]
{
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    if _comp {  // Client accepts gzip
        let mut writer = GzEncoder::new(Vec::new(), Compression::default());
        writer.write_all(response.body.as_ref().unwrap()).unwrap();
        response.body = Some(writer.finish().unwrap());
        response
            .headers
            .insert("Content-Encoding".to_string(), "gzip".to_string());
    }
}
```

**Compression detection:**
```rust
let _comp = if config.get_gzip() {
    if req_headers.contains_key("accept-encoding") {
        let accept_encoding = req_headers.get("accept-encoding").unwrap();
        let encodings: Vec<&str> = accept_encoding.split(',').map(|s| s.trim()).collect();
        encodings.contains(&"gzip")
    } else {
        false
    }
} else {
    false
};
```

---

## SSL/TLS Support

### OpenSSL Integration

```rust
#[cfg(feature = "ssl")]
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

#[cfg(feature = "ssl")]
pub fn build_https(chain: String, private: String) -> Arc<SslAcceptor> {
    let mut acceptor = SslAcceptor::mozilla_modern_v5(SslMethod::tls()).unwrap();
    acceptor.set_certificate_chain_file(chain).unwrap();
    acceptor.set_private_key_file(private, SslFiletype::PEM).unwrap();
    acceptor.check_private_key().unwrap();
    Arc::new(acceptor.build())
}
```

### Feature Flags

```toml
[features]
default = ["ssl", "sys", "log"]
async = ["tinyhttp-internal/async"]
middleware = ["tinyhttp-internal/middleware"]
openssl_vendor = ["tinyhttp-internal/openssl_vendor"]
ssl = ["tinyhttp-internal/ssl"]
sys = ["tinyhttp-internal/sys"]  # Compression
log = ["tinyhttp-internal/log"]
```

---

## Route Trait System

### Core Traits

```rust
pub trait Route: DynClone + Sync + Send + ToResponse {
    fn get_path(&self) -> &str;
    fn get_method(&self) -> Method;
    fn wildcard(&self) -> Option<String>;
    fn clone_dyn(&self) -> Box<dyn Route>;
}

pub trait ToResponse: DynClone + Sync + Send {
    fn to_res(&self, res: Request, sock: &mut TcpStream) -> Response;
}

// dyn_clone crate enables trait object cloning
pub use dyn_clone::DynClone;
```

### Route Implementations

```rust
// Basic route (no request argument)
pub struct BasicGetRoute {
    path: String,
    body: fn() -> Response,
    wildcard: Option<String>,
}

impl Route for BasicGetRoute {
    fn get_path(&self) -> &str { &self.path }
    fn get_method(&self) -> Method { Method::GET }
    fn wildcard(&self) -> Option<String> { self.wildcard.clone() }
    fn clone_dyn(&self) -> Box<dyn Route> { Box::new(self.clone()) }

    fn to_res(&self, _req: Request, sock: &mut TcpStream) -> Response {
        (self.body)()
    }
}

// Route with request argument
pub struct GetRouteWithReqAndRes {
    path: String,
    body: fn(&mut Request, &mut TcpStream) -> Response,
    wildcard: Option<String>,
}
```

---

## Middleware System

### Request Middleware

```rust
pub fn request_middleware<F: FnMut(&mut Request) + Send + Sync + 'static>(
    mut self,
    middleware_fn: F,
) -> Self {
    self.request_middleware = Some(Arc::new(Mutex::new(middleware_fn)));
    self
}

// Usage in request processing:
#[cfg(feature = "middleware")]
if let Some(req_middleware) = config.get_request_middleware() {
    req_middleware.lock().unwrap()(&mut request);
}
```

### Response Middleware

```rust
pub fn response_middleware<F: FnMut(&mut Response) + Send + Sync + 'static>(
    mut self,
    middleware_fn: F,
) -> Self {
    self.response_middleware = Some(Arc::new(Mutex::new(middleware_fn)));
    self
}
```

---

## File Serving

### Mount Point Support

```rust
pub fn mount_point<P: Into<String>>(mut self, path: P) -> Self {
    self.mount_point = Some(path.into());
    self
}

// In request handling:
None => match config.get_mount() {
    Some(old_path) => {
        let path = old_path.to_owned() + &status_line[1];

        if Path::new(&path).is_file() {
            let body = read_to_vec(&path).unwrap();
            let mime = mime_guess::from_path(&path)
                .first_raw()
                .unwrap_or("text/plain");
            Response::new()
                .status_line("HTTP/1.1 200 OK\r\n")
                .body(body)
                .mime(mime)
        } else if Path::new(&path).is_dir() {
            // Try index.html
            if Path::new(&(path.to_owned() + "/index.html")).is_file() {
                let body = read_to_vec(path + "/index.html").unwrap();
                Response::new()
                    .status_line("HTTP/1.1 200 OK\r\n")
                    .body(body)
                    .mime("text/html")
            } else {
                Response::new()
                    .status_line("HTTP/1.1 404 NOT FOUND\r\n")
                    .body(b"<h1>404 Not Found</h1>".to_vec())
                    .mime("text/html")
            }
        }
    }
    None => Response::new()
        .status_line("HTTP/1.1 404 NOT FOUND\r\n")
        .body(b"<h1>404 Not Found</h1>".to_vec())
        .mime("text/html"),
},
```

### SPA Support

```rust
if Path::new(&path).extension().is_none() && config.get_spa() {
    // Return index.html for all extensionless paths
    let body = read_to_vec(old_path.to_owned() + "/index.html").unwrap();
    Response::new()
        .status_line("HTTP/1.1 200 OK\r\n")
        .body(body)
        .mime("text/html")
}
```

Useful for React Router, Vue Router, etc.

---

## Performance

### Benchmark Results

According to the README:
- **~15,000 requests/second** on Raspberry Pi 4 with ethernet
- Tested with go-wrk benchmarking tool

### Benchmark Code

```rust
pub fn criterion_benchmark(c: &mut Criterion) {
    let http = "GET /helloworld HTTP/1.1\r\nAccept-Content: text/plain\r\n\r\n".as_bytes();

    let conf = Arc::new(Config::new().routes(Routes::new(vec![get()])));
    let buffer = Vec::with_capacity(16384);
    let mut read_write = RwWrapper::new(http, buffer);

    c.bench_function("Parse http request", move |b| {
        b.iter(|| {
            parse_request(&mut read_write, conf.clone());
        })
    });
}
```

---

## Example Applications

### Basic Hello World

```rust
use std::net::TcpListener;
use tinyhttp::prelude::*;

#[get("/")]
fn get() -> &'static str {
    "Hello, World!"
}

#[post("/")]
fn post() -> &'static str {
    "Hi, there!"
}

fn main() {
    let socket = TcpListener::bind("0.0.0.0:9001").unwrap();
    let routes = Routes::new(vec![get(), post()]);
    let config = Config::new().routes(routes);
    let http = HttpListener::new(socket, config);
    http.start();
}
```

### With Request Body

```rust
#[get("/ex2")]
fn ex2_get(req: Request) -> String {
    let accept_header = req.get_headers().get("accept").unwrap();
    format!("accept header: {}", accept_header)
}

#[post("/echo")]
fn echo(req: Request) -> String {
    req.get_parsed_body().unwrap().to_string()
}
```

### Full Response Control

```rust
#[get("/ex3")]
fn ex3_get(req: Request) -> Response {
    Response::new()
        .status_line("HTTP/1.1 200 OK\r\n")
        .mime("text/plain")
        .body(b"Hello from response!\r\n".to_vec())
}
```

### Async Example

```rust
use tinyhttp::prelude::*;
use tokio::net::TcpListener;

#[get("/")]
async fn get() -> &'static str {
    "Hello, Async World!"
}

#[tokio::main]
async fn main() {
    let socket = TcpListener::bind("0.0.0.0:9001").await.unwrap();
    let routes = Routes::new(vec![get()]);
    let config = Config::new().routes(routes);
    let http = HttpListener::new(socket, config);
    http.start().await;
}
```

---

## Feature Comparison

| Feature | Status | Notes |
|---------|--------|-------|
| GET/POST routes | ✅ | Via macros |
| Wildcard routes | ✅ | `/user/:id` syntax |
| Request body | ✅ | Via `Request` argument |
| Response builder | ✅ | Fluent API |
| Static files | ✅ | Via `mount_point` |
| SPA support | ✅ | `spa(true)` option |
| GZIP compression | ✅ | `gzip(true)` option |
| SSL/TLS | ✅ | OpenSSL backend |
| Async/await | ✅ | Tokio feature |
| Middleware | ✅ | Request/Response |
| HTTP/2 | 🟡 | Experimental |
| WebSocket | ❌ | Not implemented |

---

## Limitations

### Known Issues

1. **No connection keep-alive** - Each request handled independently
2. **No request pipelining** - Not supported in design
3. **Limited HTTP/2** - Experimental only
4. **No chunked encoding** - Body must be known upfront
5. **No graceful shutdown** - Abrupt on drop

### Dependency Concerns

- `openssl` crate requires system OpenSSL
- `rusty_pool` for thread management
- `mime_guess` for MIME types
- `infer` for content detection

---

## Summary

tinyhttp provides an Express-like developer experience in Rust:

**Strengths:**
- Intuitive macro-based routing
- Both sync and async modes
- Built-in compression and SSL
- SPA support for modern frontends
- MIME type inference

**Weaknesses:**
- Less mature than tiny-http
- Fewer tests and documentation
- OpenSSL-only SSL support
- No connection reuse

**Best for:**
- Rapid API prototyping
- Developers familiar with Express.js
- Projects needing both sync/async
- Single-page application backends
