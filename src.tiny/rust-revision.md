# Building HTTP Servers in Rust: A Complete Guide

## Introduction

This guide explains how to build HTTP servers in Rust, from first principles to production-ready implementations. We'll cover both synchronous and asynchronous approaches.

---

## HTTP Protocol Basics

### What You Need to Implement

1. **Request Parsing**
   - Read request line (method, path, version)
   - Parse headers (key: value pairs)
   - Read body (Content-Length or chunked)

2. **Response Generation**
   - Status line (version, code, reason)
   - Headers
   - Body (with proper encoding)

3. **Connection Management**
   - Keep-alive handling
   - Request pipelining (optional)
   - Graceful shutdown

### Wire Format

```
REQUEST:
GET /path HTTP/1.1\r\n
Host: example.com\r\n
Content-Length: 13\r\n
\r\n
Hello, World!

RESPONSE:
HTTP/1.1 200 OK\r\n
Content-Type: text/plain\r\n
Content-Length: 13\r\n
\r\n
Hello, World!
```

---

## Approach 1: Pure Standard Library

### Minimal TCP Server

```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer)?;

    // Parse request (simplified)
    let request = std::str::from_utf8(&buffer[..n])?;
    println!("{}", request);

    // Send response
    let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";
    stream.write_all(response.as_bytes())?;

    Ok(())
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            if let Err(e) = handle_client(stream) {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}
```

**Limitations:**
- One thread per connection
- No request parsing
- No error handling

### Multi-threaded Server

```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

fn handle_client(stream: TcpStream) {
    // ... handle request ...
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    let listener = Arc::new(listener);

    let mut handles = Vec::new();

    for _ in 0..4 {
        let listener = listener.clone();
        let handle = thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(stream) = stream {
                    handle_client(stream);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}
```

---

## Approach 2: Using tiny-http

### Basic Server

```rust
use tiny_http::{Server, Response, Header};

fn main() {
    let server = Server::http("0.0.0.0:8080").unwrap();

    for request in server.incoming_requests() {
        let response = Response::from_string("Hello, World!")
            .with_header(Header::from_bytes(
                &b"Content-Type"[..],
                &b"text/plain; charset=utf-8"[..]
            ).unwrap());

        let _ = request.respond(response);
    }
}
```

### With Routing

```rust
use tiny_http::{Server, Response, StatusCode, Method};

fn main() {
    let server = Server::http("0.0.0.0:8080").unwrap();

    for request in server.incoming_requests() {
        let response = match (request.method(), request.url()) {
            (&Method::Get, "/") => {
                Response::from_string("Home Page")
            }
            (&Method::Get, "/api/data") => {
                Response::from_string(r#"{"key": "value"}"#)
                    .with_header(Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"application/json"[..]
                    ).unwrap())
            }
            (&Method::Post, "/api/echo") => {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).unwrap();
                Response::from_string(body)
            }
            _ => {
                Response::from_string("Not Found")
                    .with_status_code(StatusCode(404))
            }
        };

        let _ = request.respond(response);
    }
}
```

### Multi-threaded with tiny-http

```rust
use tiny_http::{Server, Response};
use std::sync::Arc;
use std::thread;

fn main() {
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

    for handle in handles {
        handle.join().unwrap();
    }
}
```

---

## Approach 3: Using tinyhttp

### Basic Server with Macros

```rust
use tinyhttp::prelude::*;
use std::net::TcpListener;

#[get("/")]
fn index() -> &'static str {
    "Hello, World!"
}

#[get("/api/data")]
fn data() -> String {
    r#"{"key": "value"}"#.to_string()
}

#[post("/api/echo")]
fn echo(req: Request) -> String {
    req.get_parsed_body().unwrap().to_string()
}

fn main() {
    let socket = TcpListener::bind("0.0.0.0:8080").unwrap();
    let routes = Routes::new(vec![index(), data(), echo()]);
    let config = Config::new().routes(routes);
    let http = HttpListener::new(socket, config);
    http.start();
}
```

### With Static Files

```rust
use tinyhttp::prelude::*;
use std::net::TcpListener;

#[get("/")]
fn index() -> &'static str {
    "Welcome!"
}

fn main() {
    let socket = TcpListener::bind("0.0.0.0:8080").unwrap();

    let routes = Routes::new(vec![index()]);

    let config = Config::new()
        .routes(routes)
        .mount_point("./static")  // Serve static files
        .spa(true)                 // SPA fallback to index.html
        .gzip(true);               // Enable compression

    let http = HttpListener::new(socket, config);
    http.start();
}
```

### Async Server

```rust
use tinyhttp::prelude::*;
use tokio::net::TcpListener;

#[get("/")]
async fn index() -> &'static str {
    "Hello, Async!"
}

#[tokio::main]
async fn main() {
    let socket = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    let routes = Routes::new(vec![index()]);
    let config = Config::new().routes(routes);
    let http = HttpListener::new(socket, config);
    http.start().await;
}
```

---

## Approach 4: Building Your Own Parser

### Request Structure

```rust
use std::collections::HashMap;
use std::io::{Read, BufReader, BufRead};

#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn read<R: Read>(reader: R) -> Result<Self, String> {
        let mut buf_reader = BufReader::new(reader);
        let mut request_line = String::new();

        // Read request line
        buf_reader.read_line(&mut request_line)
            .map_err(|e| e.to_string())?;

        let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
        if parts.len() != 3 {
            return Err("Invalid request line".to_string());
        }

        let method = parts[0].to_string();
        let path = parts[1].to_string();
        let version = parts[2].to_string();

        // Read headers
        let mut headers = HashMap::new();
        loop {
            let mut header_line = String::new();
            buf_reader.read_line(&mut header_line)
                .map_err(|e| e.to_string())?;

            if header_line.trim().is_empty() {
                break;  // End of headers
            }

            if let Some((key, value)) = header_line.trim().split_once(':') {
                headers.insert(
                    key.trim().to_lowercase(),
                    value.trim().to_string()
                );
            }
        }

        // Read body
        let body = if let Some(content_length) = headers.get("content-length") {
            let len: usize = content_length.parse().unwrap_or(0);
            let mut body = vec![0; len];
            buf_reader.read_exact(&mut body)
                .map_err(|e| e.to_string())?;
            body
        } else {
            Vec::new()
        };

        Ok(Request {
            method,
            path,
            version,
            headers,
            body,
        })
    }
}
```

### Response Builder

```rust
use std::collections::HashMap;
use std::io::{Write, Result};

pub struct Response {
    status_code: u16,
    status_text: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Response {
    pub fn new(status_code: u16, status_text: &str) -> Self {
        Response {
            status_code,
            status_text: status_text.to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn body(&mut self, body: Vec<u8>) -> &mut Self {
        self.body = body;
        self
    }

    pub fn send<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Status line
        writeln!(
            writer,
            "HTTP/1.1 {} {}\r",
            self.status_code,
            self.status_text
        )?;

        // Headers
        for (key, value) in &self.headers {
            writeln!(writer, "{}: {}\r", key, value)?;
        }

        // Content-Length if not set
        if !self.headers.contains_key("content-length") {
            writeln!(writer, "Content-Length: {}\r", self.body.len())?;
        }

        // Empty line
        writeln!(writer, "\r")?;

        // Body
        writer.write_all(&self.body)?;
        writer.flush()?;

        Ok(())
    }
}
```

### Complete Server

```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::collections::HashMap;

fn handle_client(mut stream: TcpStream) {
    // Read request
    let request = match Request::read(&mut stream) {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            let mut response = Response::new(400, "Bad Request");
            response.body(b"Invalid request".to_vec());
            let _ = response.send(&mut stream);
            return;
        }
    };

    println!("{} {}", request.method, request.path);

    // Route handling
    let mut response = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            let mut res = Response::new(200, "OK");
            res.header("Content-Type", "text/plain");
            res.body(b"Hello, World!".to_vec());
            res
        }
        ("GET", "/api/data") => {
            let mut res = Response::new(200, "OK");
            res.header("Content-Type", "application/json");
            res.body(b r#"{"status": "ok"}"#.to_vec());
            res
        }
        _ => {
            let mut res = Response::new(404, "Not Found");
            res.body(b"404 Not Found".to_vec());
            res
        }
    };

    let _ = response.send(&mut stream);
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            handle_client(stream);
        }
    }

    Ok(())
}
```

---

## Approach 5: Async with Tokio

### Basic Async Server

```rust
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Listening on http://127.0.0.1:8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        println!("Client connected: {}", addr);

        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            match socket.read(&mut buffer).await {
                Ok(n) => {
                    let request = String::from_utf8_lossy(&buffer[..n]);
                    println!("Request:\n{}", request);

                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";
                    let _ = socket.write_all(response.as_bytes()).await;
                }
                Err(e) => eprintln!("Error reading: {}", e),
            }
        });
    }
}
```

### Async Request Parsing

```rust
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

async fn read_request<S: AsyncReadExt + Unpin>(
    mut stream: S,
) -> Result<Request, String> {
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();

    // Read request line
    reader.read_line(&mut request_line).await
        .map_err(|e| e.to_string())?;

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();

    // Read headers
    let mut headers = HashMap::new();
    loop {
        let mut header_line = String::new();
        reader.read_line(&mut header_line).await
            .map_err(|e| e.to_string())?;

        if header_line.trim().is_empty() {
            break;
        }

        if let Some((key, value)) = header_line.trim().split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    // Read body
    let body = if let Some(content_length) = headers.get("content-length") {
        let len: usize = content_length.parse().unwrap_or(0);
        let mut body = vec![0; len];
        reader.read_exact(&mut body).await
            .map_err(|e| e.to_string())?;
        body
    } else {
        Vec::new()
    };

    Ok(Request {
        method: parts.get(0).unwrap_or(&"").to_string(),
        path: parts.get(1).unwrap_or(&"").to_string(),
        version: parts.get(2).unwrap_or(&"").to_string(),
        headers,
        body,
    })
}
```

---

## Recommended Crates

### Full-Featured Frameworks

| Crate | Style | Async | Notes |
|-------|-------|-------|-------|
| **axum** | Type-driven | ✅ | Tokio ecosystem |
| **actix-web** | Builder | ✅ | Highest performance |
| **warp** | Filter-based | ✅ | Composable filters |
| **rocket** | Macro-based | ✅ | Developer friendly |
| **poem** | OpenAPI | ✅ | API-first design |

### Low-Level Libraries

| Crate | Description |
|-------|-------------|
| **hyper** | HTTP/1 + HTTP/2 core |
| **tiny-http** | Simple sync server |
| **tinyhttp** | Macro-based sync/async |
| **http** | HTTP types (used by all) |

### Supporting Crates

| Crate | Purpose |
|-------|---------|
| **tokio** | Async runtime |
| **serde** | JSON serialization |
| **serde_json** | JSON handling |
| **headers** | HTTP header types |
| **http-body** | Body traits |
| **bytes** | Efficient buffers |

---

## Production Patterns

### Error Handling

```rust
use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Route not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, ServerError>;
```

### Graceful Shutdown

```rust
use tokio::signal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static RUNNING: AtomicBool = AtomicBool::new(true);

async fn graceful_shutdown() {
    // Wait for Ctrl+C
    signal::ctrl_c().await.unwrap();
    println!("Shutting down...");
    RUNNING.store(false, Ordering::SeqCst);
}

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    let running = Arc::new(RUNNING);

    let shutdown_handle = tokio::spawn(graceful_shutdown());

    while RUNNING.load(Ordering::SeqCst) {
        tokio::select! {
            result = listener.accept() => {
                if let Ok((stream, _)) = result {
                    let running = running.clone();
                    tokio::spawn(async move {
                        // Handle with shutdown check
                    });
                }
            }
            _ = &mut shutdown_handle => break,
        }
    }
}
```

### Middleware Pattern

```rust
use std::future::Future;
use std::pin::Pin;

pub type Handler = Box<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response>>> + Send + Sync>;

pub struct MiddlewareChain {
    handlers: Vec<Handler>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        MiddlewareChain { handlers: Vec::new() }
    }

    pub fn add<F, Fut>(&mut self, handler: F)
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.handlers.push(Box::new(move |req| {
            Box::pin(handler(req))
        }));
    }

    pub async fn call(&self, request: Request) -> Response {
        let mut response = None;

        for handler in &self.handlers {
            // Chain handlers
        }

        response.unwrap()
    }
}
```

### Logging Middleware

```rust
use std::time::Instant;

async fn log_middleware(request: Request) -> Response {
    let start = Instant::now();
    let method = request.method.clone();
    let path = request.path.clone();

    println!("{} {} - Started", method, path);

    let response = /* call next handler */;

    let duration = start.elapsed();
    println!("{} {} - {} ({}ms)",
        method, path, response.status, duration.as_millis());

    response
}
```

---

## SSL/TLS Setup

### Using Rustls

```rust
use rustls::{ServerConfig, Certificate, PrivateKey};
use std::fs::File;
use std::io::BufReader;

fn load_certs(path: &str) -> Vec<Certificate> {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    rustls_pemfile::certs(&mut reader).unwrap()
        .into_iter()
        .map(Certificate)
        .collect()
}

fn load_key(path: &str) -> PrivateKey {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut reader).unwrap();
    PrivateKey(keys[0].clone())
}

fn create_tls_config() -> ServerConfig {
    ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(load_certs("cert.pem"), load_key("key.pem"))
        .unwrap()
}
```

### Using OpenSSL

```rust
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

fn create_ssl_acceptor() -> SslAcceptor {
    let mut acceptor = SslAcceptor::mozilla_modern(SslMethod::tls()).unwrap();
    acceptor.set_certificate_chain_file("cert.pem").unwrap();
    acceptor.set_private_key_file("key.pem", SslFiletype::PEM).unwrap();
    acceptor.check_private_key().unwrap();
    acceptor.build()
}
```

---

## Testing

### Unit Testing Handlers

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_handler() {
        let request = Request {
            method: "GET".to_string(),
            path: "/".to_string(),
            version: "HTTP/1.1".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        };

        let response = index_handler(request);
        assert_eq!(response.status, 200);
        assert!(String::from_utf8_lossy(&response.body).contains("Hello"));
    }
}
```

### Integration Testing

```rust
#[cfg(test)]
mod integration {
    use std::net::TcpStream;
    use std::io::{Read, Write};
    use std::thread;
    use std::time::Duration;

    fn start_test_server() {
        thread::spawn(|| {
            let server = Server::http("127.0.0.1:0").unwrap();
            for request in server.incoming_requests() {
                let response = Response::from_string("OK");
                let _ = request.respond(response);
            }
        });
        thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_server_responds() {
        start_test_server();

        let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
        stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();

        assert!(response.contains("200 OK"));
        assert!(response.contains("OK"));
    }
}
```

---

## Performance Optimization

### Buffer Management

```rust
// Bad: allocate per request
let buffer = vec![0; 1024];

// Good: reuse buffer
thread_local! {
    static BUFFER: RefCell<Vec<u8>> = RefCell::new(vec![0; 8192]);
}

BUFFER.with(|buf| {
    let mut buf = buf.borrow_mut();
    // Use buffer
});
```

### Connection Pooling

```rust
use deadpool::managed::{Manager, Pool};

struct ConnectionManager {
    // Connection details
}

impl Manager for ConnectionManager {
    type Type = TcpStream;
    type Error = io::Error;

    async fn create(&self) -> Result<TcpStream, io::Error> {
        TcpStream::connect("127.0.0.1:8080").await
    }
}

// Usage
let pool = Pool::builder(ConnectionManager { ... })
    .max_size(10)
    .build()
    .unwrap();

let conn = pool.get().await?;
```

---

## Checklist for Production

- [ ] Error handling with proper status codes
- [ ] Logging (request/response, errors)
- [ ] Graceful shutdown
- [ ] SSL/TLS termination
- [ ] Rate limiting
- [ ] Request size limits
- [ ] Timeout handling
- [ ] Health check endpoint
- [ ] Metrics/monitoring
- [ ] Security headers (CORS, CSP, etc.)

---

## Summary

| Approach | Best For | Complexity |
|----------|----------|------------|
| **Standard Library** | Learning | Low |
| **tiny-http** | Simple servers | Low |
| **tinyhttp** | Rapid prototyping | Low |
| **Custom Parser** | Full control | Medium |
| **Tokio + Hyper** | Production async | High |
| **Actix/Axum** | Full frameworks | Medium |

Start with tiny-http or tinyhttp for learning, graduate to axum or actix-web for production.
