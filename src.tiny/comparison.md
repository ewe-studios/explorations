# tiny-http vs tinyhttp: Comprehensive Comparison

## Overview

Both projects are HTTP server implementations in Rust, but they take fundamentally different approaches to API design and feature sets.

| Aspect | tiny-http | tinyhttp |
|--------|-----------|----------|
| **Version** | 0.12.0 (mature) | 0.5.0 (developing) |
| **Philosophy** | Low-level, explicit | High-level, magical |
| **API Style** | Manual request/response | Procedural macro routing |
| **Lines of Code** | ~3000 (core) | ~2000 (core) |
| **Dependencies** | 5 (minimal) | 15+ (moderate) |

---

## Architecture Comparison

### Design Philosophy

**tiny-http:**
> "Provide minimal abstractions over raw HTTP"

- Synchronous I/O only
- Explicit request handling
- No built-in routing
- Manual response construction
- Maximum control

**tinyhttp:**
> "Express.js experience in Rust"

- Sync and async modes
- Declarative routing via macros
- Built-in features (compression, SSL)
- Automatic MIME detection
- Developer convenience

### Threading Models

**tiny-http: Thread Pool + Message Queue**

```
┌─────────────────────────────────────────────────────────┐
│                   Accept Thread                          │
│                                                          │
│  loop {                                                  │
│    let (sock, addr) = listener.accept()?;                │
│    task_pool.spawn(|| {                                  │
│      for request in ClientConnection::new(sock) {        │
│        messages.push(Message::NewRequest(request));      │
│      }                                                   │
│    });                                                   │
│  }                                                       │
│                                                          │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              MessagesQueue (MPSC Channel)                │
│                                                          │
│  - Capacity: 8 requests                                  │
│  - Thread-safe                                           │
│  - Blocks when empty                                     │
│                                                          │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│               Application Threads                        │
│                                                          │
│  for worker in 0..4 {                                    │
│    thread::spawn(|| {                                    │
│      loop {                                              │
│        let request = server.recv()?;  // Block           │
│        handle(request);                                  │
│      }                                                   │
│    });                                                   │
│  }                                                       │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**tinyhttp: ThreadPool or Async**

```
Sync Mode:
┌─────────────────────────────────────────────────────────┐
│              rusty_pool ThreadPool                       │
│                                                          │
│  for stream in socket.accept() {                         │
│    if use_pool {                                         │
│      pool.execute(|| parse_request(stream, config));     │
│    } else {                                              │
│      parse_request(stream, config);                      │
│    }                                                     │
│  }                                                       │
│                                                          │
└─────────────────────────────────────────────────────────┘

Async Mode:
┌─────────────────────────────────────────────────────────┐
│                  Tokio Runtime                           │
│                                                          │
│  loop {                                                  │
│    let (mut conn, addr) = socket.accept().await?;        │
│    tokio::spawn(async move {                             │
│      parse_request(&mut conn, config).await;             │
│    });                                                   │
│  }                                                       │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Request Flow Comparison

| Stage | tiny-http | tinyhttp |
|-------|-----------|----------|
| **Accept** | Dedicated thread | Main loop |
| **Parse** | Line-by-line | Buffer-based |
| **Route** | Manual URL match | Macro-generated |
| **Handle** | User code | User code |
| **Respond** | Explicit | Builder pattern |
| **Ordering** | Sequential I/O | Per-connection |

---

## API Comparison

### Basic Server

**tiny-http:**

```rust
use tiny_http::{Server, Response};

fn main() {
    let server = Server::http("0.0.0.0:8080").unwrap();

    for request in server.incoming_requests() {
        let response = Response::from_string("Hello, World!");
        let _ = request.respond(response);
    }
}
```

**tinyhttp:**

```rust
use tinyhttp::prelude::*;
use std::net::TcpListener;

#[get("/")]
fn index() -> &'static str {
    "Hello, World!"
}

fn main() {
    let socket = TcpListener::bind("0.0.0.0:8080").unwrap();
    let routes = Routes::new(vec![index()]);
    let config = Config::new().routes(routes);
    let http = HttpListener::new(socket, config);
    http.start();
}
```

**Comparison:**
- tiny-http: More boilerplate, more control
- tinyhttp: Declarative, less code

### Routing

**tiny-http (manual):**

```rust
use tiny_http::{Server, Response, StatusCode};

let server = Server::http("0.0.0.0:8080").unwrap();

for request in server.incoming_requests() {
    let response = match request.url() {
        "/" => Response::from_string("Home"),
        "/about" => Response::from_string("About"),
        url if url.starts_with("/user/") => {
            let id = &url[6..];
            Response::from_string(format!("User {}", id))
        }
        _ => Response::from_string("404")
            .with_status_code(StatusCode(404)),
    };
    let _ = request.respond(response);
}
```

**tinyhttp (macro-based):**

```rust
use tinyhttp::prelude::*;

#[get("/")]
fn home() -> &'static str { "Home" }

#[get("/about")]
fn about() -> &'static str { "About" }

#[get("/user/:id")]
fn user(id: &str) -> String { format!("User {}", id) }

fn main() {
    let routes = Routes::new(vec![home(), about(), user()]);
    // ...
}
```

**Comparison:**
- tiny-http: Runtime URL matching
- tinyhttp: Compile-time route registration

### Request Body Handling

**tiny-http:**

```rust
use tiny_http::{Server, Response};
use std::io::Read;

let server = Server::http("0.0.0.0:8080").unwrap();

for request in server.incoming_requests() {
    let mut body = String::new();
    request.as_reader().read_to_string(&mut body).unwrap();

    let response = Response::from_string(format!(
        "Received: {}", body
    ));
    let _ = request.respond(response);
}
```

**tinyhttp:**

```rust
use tinyhttp::prelude::*;

#[post("/echo")]
fn echo(req: Request) -> String {
    req.get_parsed_body()
        .unwrap_or("")
        .to_string()
}
```

**Comparison:**
- tiny-http: Manual reading via `Read` trait
- tinyhttp: Body pre-read, accessed via method

### Response Building

**tiny-http:**

```rust
use tiny_http::{Response, Header, StatusCode};

let response = Response::from_string("Hello")
    .with_header(Header::from_bytes(
        &b"Content-Type"[..],
        &b"text/plain; charset=utf-8"[..]
    ).unwrap())
    .with_status_code(StatusCode(200));
```

**tinyhttp:**

```rust
use tinyhttp::prelude::*;

let response = Response::new()
    .status_line("HTTP/1.1 200 OK\r\n")
    .body(b"Hello".to_vec())
    .mime("text/plain");
```

**Comparison:**
- tiny-http: Type-safe header construction
- tinyhttp: String-based header values

### Multi-threaded Server

**tiny-http:**

```rust
use tiny_http::{Server, Response};
use std::sync::Arc;
use std::thread;

let server = Arc::new(Server::http("0.0.0.0:8080").unwrap());
let mut handles = Vec::new();

for _ in 0..4 {
    let server = server.clone();
    handles.push(thread::spawn(move || {
        for request in server.incoming_requests() {
            let response = Response::from_string("Hello!");
            let _ = request.respond(response);
        }
    }));
}

for h in handles {
    h.join().unwrap();
}
```

**tinyhttp:**

```rust
use tinyhttp::prelude::*;
use std::net::TcpListener;

#[get("/")]
fn index() -> &'static str { "Hello!" }

let socket = TcpListener::bind("0.0.0.0:8080").unwrap();
let routes = Routes::new(vec![index()]);
let config = Config::new()
    .routes(routes)
    .gzip(true);  // Built-in compression

let http = HttpListener::new(socket, config)
    .threads(4);  // Thread pool size

http.start();  // Handles threading internally
```

**Comparison:**
- tiny-http: Manual thread management
- tinyhttp: Abstracted thread pool

---

## Feature Comparison

### HTTP Protocol Support

| Feature | tiny-http | tinyhttp |
|---------|-----------|----------|
| HTTP/1.0 | ✅ | ✅ |
| HTTP/1.1 | ✅ | ✅ |
| HTTP/2 | ❌ | 🟡 (experimental) |
| Keep-Alive | ✅ | ❌ |
| Pipelining | ✅ | ❌ |
| Chunked Encoding | ✅ | ❌ |
| 100-Continue | ✅ | ❌ |
| WebSocket Upgrade | ✅ | ❌ |
| Range Requests | ❌ | ❌ |

### Security

| Feature | tiny-http | tinyhttp |
|---------|-----------|----------|
| SSL/TLS | ✅ (multiple backends) | ✅ (OpenSSL only) |
| Zeroize private keys | ✅ | ❌ |
| Secure memory handling | ✅ | ⚠️ (partial) |

**tiny-http SSL backends:**
- OpenSSL
- rustls
- native-tls

**tinyhttp SSL:**
- OpenSSL only

### Compression

| Feature | tiny-http | tinyhttp |
|---------|-----------|----------|
| GZIP | ❌ (manual) | ✅ (built-in) |
| Brotli | ❌ | 🟡 (feature flag) |
| Auto-detection | ❌ | ✅ (Accept-Encoding) |

### Developer Experience

| Feature | tiny-http | tinyhttp |
|---------|-----------|----------|
| Routing | Manual | Macros |
| Static files | Manual | `mount_point()` |
| SPA support | ❌ | ✅ |
| Middleware | Manual | ✅ (built-in) |
| MIME inference | ❌ | ✅ |
| Async/await | ❌ | ✅ |
| Documentation | Extensive | Basic |
| Examples | 6+ | 3 |

---

## Performance Comparison

### Benchmark Approaches

**tiny-http:**

```rust
#[bench]
fn sequential_requests(bencher: &mut test::Bencher) {
    let server = tiny_http::Server::http("0.0.0.0:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();

    bencher.iter(|| {
        write!(stream, "GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let request = server.recv().unwrap();
        request.respond(tiny_http::Response::new_empty(tiny_http::StatusCode(204)));
    });
}
```

**tinyhttp:**

```rust
pub fn criterion_benchmark(c: &mut Criterion) {
    let http = "GET /helloworld HTTP/1.1\r\nAccept-Content: text/plain\r\n\r\n".as_bytes();
    let conf = Arc::new(Config::new().routes(Routes::new(vec![get()])));
    let mut read_write = RwWrapper::new(http, buffer);

    c.bench_function("Parse http request", move |b| {
        b.iter(|| {
            parse_request(&mut read_write, conf.clone());
        })
    });
}
```

### Reported Performance

| Metric | tiny-http | tinyhttp |
|--------|-----------|----------|
| Requests/sec (localhost) | ~50,000 | ~15,000 (Pi 4) |
| Concurrent connections | 1000+ tested | Not specified |
| Memory per connection | ~10 KB | ~15 KB |
| Cold start | <10ms | <50ms |

**Note:** tinyhttp reports 15,000 req/s on Raspberry Pi 4. tiny-http typically achieves 30,000-50,000 req/s on similar hardware.

### Performance Factors

**tiny-http advantages:**
- Smaller buffers (1KB vs variable)
- No MIME inference overhead
- Direct socket I/O
- Minimal abstraction

**tinyhttp disadvantages:**
- MIME type inference per response
- GZIP compression overhead (when enabled)
- Additional HashMap lookups for routing
- String-based header handling

---

## Code Quality Comparison

### Test Coverage

**tiny-http:**

```
src/
├── lib.rs          - 15 tests
├── request.rs      - 1 test
├── response.rs     - 3 tests
├── common.rs       - 4 tests
├── client.rs       - 1 test
└── util/           - 2 tests

Total: 26+ unit tests
Integration tests: 5+
Benchmarks: 2
```

**tinyhttp:**

```
tinyhttp-internal/
├── lib.rs          - 2 tests
├── http.rs         - 0 tests
└── config.rs       - 0 tests

Total: 2 unit tests
Integration tests: Minimal
Benchmarks: 1
```

### Type Safety

**tiny-http:**

```rust
// Type-safe header construction
Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..])
    .map_err(|_| ())?;  // Compile-time ASCII check

// Type-safe status codes
StatusCode(200)  // u16 wrapper
```

**tinyhttp:**

```rust
// String-based (runtime errors possible)
.status_line("HTTP/1.1 200 OK\r\n")  // Manual CRLF
.mime("text/plain")
```

### Error Handling

**tiny-http:**

```rust
pub enum RequestCreationError {
    ExpectationFailed,
    CreationIoError(IoError),
}

pub enum ReadError {
    WrongRequestLine,
    WrongHeader(HTTPVersion),
    ExpectationFailed(HTTPVersion),
    ReadIoError(IoError),
}

// Graceful error recovery
fn ignore_client_closing_errors(result: io::Result<()>) -> io::Result<()> {
    result.or_else(|err| match err.kind() {
        ErrorKind::BrokenPipe => Ok(()),
        ErrorKind::ConnectionReset => Ok(()),
        _ => Err(err),
    })
}
```

**tinyhttp:**

```rust
#[derive(Error, Debug)]
pub enum RequestError {
    #[error("failed to parse status line")]
    StatusLineErr,
    #[error("failed to parse headers")]
    HeadersErr,
}

// unwrap() usage in hot path
buf_reader.read_line(&mut status_line_str).unwrap();  // Panics on error
```

---

## Dependencies Comparison

### tiny-http Dependencies

```toml
[dependencies]
ascii = "1.0"              # ASCII string handling
chunked_transfer = "1"     # Chunked encoding
httpdate = "1.0.2"         # HTTP date formatting

# Optional
log = "0.4.4"              # Logging
openssl = "0.10"           # SSL (optional)
rustls = "0.20"            # SSL (optional)
native-tls = "0.2"         # SSL (optional)
zeroize = "1"              # Secure memory
```

**Total:** 3 required + 2-3 optional

### tinyhttp Dependencies

```toml
[dependencies]
tinyhttp-internal = { path = "../tinyhttp-internal" }
tinyhttp-codegen = { path = "../tinyhttp-codegen" }

# Internal dependencies:
mime_guess = "2.0.4"       # MIME detection
openssl = "0.10"           # SSL
dyn-clone = "1.0.11"       # Trait object cloning
infer = "0.15.0"           # Content detection
num_cpus = "1.16.0"        # CPU count
rusty_pool = "0.7.0"       # Thread pool
thiserror = "1"            # Error macros
flate2 = "1"               # Compression
tokio = "1.3"              # Async runtime (optional)
log = "0.4"                # Logging (optional)
```

**Total:** 10+ required + optional

---

## Use Case Recommendations

### Choose tiny-http When:

1. **Building libraries** - Minimal dependencies for users
2. **Learning HTTP** - Clear, explicit implementation
3. **Need pipelining** - Full HTTP/1.1 support
4. **Maximum control** - Low-level access
5. **Security-critical** - Multiple SSL backends, secure memory
6. **Embedded systems** - Smaller footprint
7. **Testing** - Mock server creation

**Example scenarios:**
- CLI tool with local HTTP server
- Local development server
- API mock for testing
- Embedded device server
- Learning HTTP internals

### Choose tinyhttp When:

1. **Rapid prototyping** - Quick API development
2. **Express.js migrants** - Familiar patterns
3. **Need async** - Tokio integration
4. **Built-in features** - Compression, SSL, MIME
5. **SPA backends** - Static file serving
6. **Small web apps** - All-in-one solution

**Example scenarios:**
- REST API backend
- Single-page app server
- Quick microservice
- Prototype/demo server
- Internal tools

---

## Production Readiness

### tiny-http

**Production use indicators:**
- ✅ Version 0.12.0 (years of development)
- ✅ Extensive tests
- ✅ Multiple SSL backends
- ✅ Documentation
- ✅ Known deployments
- ✅ Security audit history (community)

**Missing for production:**
- ❌ No async support
- ❌ No built-in compression
- ❌ Manual routing needed
- ❌ No graceful shutdown

### tinyhttp

**Production use indicators:**
- ✅ SSL support
- ✅ GZIP compression
- ✅ Async mode
- ⚠️ Version 0.5.0 (developing)

**Missing for production:**
- ❌ Limited tests
- ❌ Minimal documentation
- ❌ OpenSSL only
- ❌ No keep-alive
- ❌ No graceful shutdown
- ❌ Single SSL backend

---

## Migration Paths

### From tiny-http to tinyhttp

```rust
// tiny-http
let server = Server::http("0.0.0.0:8080").unwrap();
for request in server.incoming_requests() {
    if request.url() == "/api/data" {
        let response = Response::from_string("data");
        let _ = request.respond(response);
    }
}

// tinyhttp
#[get("/api/data")]
fn data() -> &'static str { "data" }

let routes = Routes::new(vec![data()]);
// ...
```

### From tinyhttp to tiny-http

```rust
// tinyhttp
#[get("/user/:id")]
fn user(id: &str) -> String { format!("User {}", id) }

// tiny-http
for request in server.incoming_requests() {
    let url = request.url();
    if let Some(id) = url.strip_prefix("/user/") {
        let response = Response::from_string(format!("User {}", id));
        let _ = request.respond(response);
    }
}
```

---

## Summary Matrix

| Criterion | tiny-http | tinyhttp |
|-----------|-----------|----------|
| **Maturity** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Performance** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Features** | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Ease of Use** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Documentation** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Type Safety** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Flexibility** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Async Support** | ❌ | ✅ |
| **Dependencies** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |

**Overall:** tiny-http wins on maturity and control; tinyhttp wins on developer experience.
