# Tiny HTTP Servers Exploration: tiny-http vs tinyhttp

## Executive Summary

This exploration covers two Rust-based HTTP server implementations:

1. **tiny-http** - A mature, low-level synchronous HTTP server library (v0.12.0)
2. **tinyhttp** - A modern, macro-driven HTTP server with sync/async support (v0.5.0)

Both projects demonstrate different approaches to building HTTP servers in Rust, from raw socket handling to procedural macro abstractions.

---

## Project Overview

### tiny-http (Rust)

**Repository:** https://github.com/tiny-http/tiny-http

**Philosophy:** Low-level, synchronous HTTP server with minimal abstractions.

**Key Characteristics:**
- Pure synchronous I/O using std::net
- Thread-pool based concurrency
- Request pipelining support
- HTTP/1.0 and HTTP/1.1 compliant
- Optional SSL support (OpenSSL, rustls, native-tls)
- Unix socket support

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│                     Application                          │
├─────────────────────────────────────────────────────────┤
│                    Server::recv()                        │
├─────────────────────────────────────────────────────────┤
│              MessagesQueue (MPSC Channel)                │
├─────────────────────────────────────────────────────────┤
│                 TaskPool (Thread Pool)                   │
├─────────────────────────────────────────────────────────┤
│              ClientConnection Iterator                   │
├─────────────────────────────────────────────────────────┤
│         SequentialReader/Writer (Ordered I/O)            │
├─────────────────────────────────────────────────────────┤
│              TcpStream / UnixStream                      │
└─────────────────────────────────────────────────────────┘
```

### tinyhttp (Rust)

**Repository:** https://github.com/mateocabanal/tinyhttp

**Philosophy:** Developer-friendly HTTP server with procedural macro routing.

**Key Characteristics:**
- Procedural macro-based route definitions
- Both sync and async (tokio) modes
- Built-in GZIP compression
- SSL/TLS support via OpenSSL
- MIME type inference
- SPA (Single Page Application) support

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│              #[get("/")] #[post("/")]                    │
│                   Procedural Macros                      │
├─────────────────────────────────────────────────────────┤
│                    Routes HashMap                        │
├─────────────────────────────────────────────────────────┤
│              Config + Middleware Chain                   │
├─────────────────────────────────────────────────────────┤
│         ThreadPool (rusty_pool) or Tokio Runtime         │
├─────────────────────────────────────────────────────────┤
│              Request Parser (Buffer-based)               │
├─────────────────────────────────────────────────────────┤
│              TcpStream (sync or async)                   │
└─────────────────────────────────────────────────────────┘
```

---

## Detailed Project Structure

### tiny-http Directory Structure

```
tiny-http/
├── src/
│   ├── lib.rs              # Main server implementation
│   ├── client.rs           # Client connection handling
│   ├── common.rs           # HTTP types (Status, Headers, Methods)
│   ├── connection.rs       # TCP/Unix connection abstraction
│   ├── request.rs          # Request parsing and structure
│   ├── response.rs         # Response building and sending
│   ├── ssl.rs              # SSL/TLS implementation
│   ├── log.rs              # Logging utilities
│   ├── test.rs             # Test utilities
│   └── util/
│       ├── mod.rs
│       ├── task_pool.rs    # Thread pool implementation
│       ├── messages_queue.rs  # MPSC queue for requests
│       ├── sequential.rs   # Ordered read/write operations
│       ├── refined_tcp_stream.rs  # Split TCP stream
│       ├── fused_reader.rs # Reader adapters
│       └── custom_stream.rs  # Custom stream types
├── examples/
│   ├── hello-world.rs      # Basic server example
│   ├── ssl.rs              # HTTPS server
│   ├── websockets.rs       # WebSocket upgrade example
│   └── php-cgi.rs          # CGI integration
└── benches/
    └── bench.rs            # Performance benchmarks
```

### tinyhttp Directory Structure

```
tinyhttp/
├── tinyhttp/                    # Main crate (re-exports)
│   └── src/lib.rs
├── tinyhttp-internal/           # Core implementation
│   └── src/
│       ├── lib.rs
│       ├── http.rs              # Sync HTTP handling
│       ├── async_http.rs        # Async HTTP handling
│       ├── request.rs           # Request types
│       ├── response.rs          # Response types
│       ├── config.rs            # Server configuration
│       └── codegen/
│           ├── mod.rs
│           └── route.rs         # Route trait definitions
├── tinyhttp-codegen/            # Procedural macros
│   └── src/lib.rs               # #[get], #[post] macros
└── benches/
    └── create_req.rs            # Request parsing benchmarks
```

---

## Key Design Patterns

### 1. Connection Handling

**tiny-http:**
- Uses `ClientConnection` as an iterator over requests
- Implements request pipelining with `SequentialReader`/`SequentialWriter`
- Ensures response ordering for pipelined requests

```rust
pub struct ClientConnection {
    remote_addr: IoResult<Option<SocketAddr>>,
    source: SequentialReaderBuilder<BufReader<RefinedTcpStream>>,
    sink: SequentialWriterBuilder<BufWriter<RefinedTcpStream>>,
    next_header_source: SequentialReader<BufReader<RefinedTcpStream>>,
    no_more_requests: bool,
    secure: bool,
}
```

**tinyhttp:**
- Direct stream parsing per connection
- Optional thread pool for concurrent handling
- Async mode uses tokio's cooperative multitasking

```rust
pub struct HttpListener {
    pub(crate) socket: TcpListener,
    pub config: Config,
    pub pool: ThreadPool,
    pub use_pool: bool,
    #[cfg(feature = "ssl")]
    pub ssl_acpt: Option<Arc<SslAcceptor>>,
}
```

### 2. Request Parsing

**tiny-http:**
- Line-by-line parsing using `read_next_line()`
- Proper CRLF handling
- Header validation with strict ASCII requirements

```rust
fn read_next_line(&mut self) -> IoResult<AsciiString> {
    let mut buf = Vec::new();
    let mut prev_byte_was_cr = false;
    loop {
        let byte = self.next_header_source.by_ref().bytes().next();
        let byte = match byte {
            Some(b) => b?,
            None => return Err(IoError::new(...)),
        };
        if byte == b'\n' && prev_byte_was_cr {
            buf.pop();
            return AsciiString::from_ascii(buf)...;
        }
        prev_byte_was_cr = byte == b'\r';
        buf.push(byte);
    }
}
```

**tinyhttp:**
- Buffer-based parsing with window searches
- Finds `\r\n\r\n` boundary for header/body split
- Content-Length based body reading

```rust
fn build_and_parse_req<P: Read>(conn: &mut P) -> Result<Request, RequestError> {
    let mut buf_reader = BufReader::new(conn);
    let mut status_line_str = String::new();
    buf_reader.read_line(&mut status_line_str).unwrap();

    let iter = buf_reader.fill_buf().unwrap();
    let header_end_idx = iter.windows(4)
        .position(|w| matches!(w, b"\r\n\r\n")).unwrap();

    // Parse headers...
    // Read body based on content-length
}
```

### 3. Response Handling

**tiny-http:**
- Transfer-Encoding detection (Chunked vs Identity)
- Automatic Date and Server headers
- Content-Length management

```rust
fn choose_transfer_encoding(...) -> TransferEncoding {
    if *http_version <= (1, 0) { return Identity; }
    if status_code < 200 || status_code == 204 { return Identity; }
    if has_additional_headers { return Chunked; }
    if content_length >= threshold { return Chunked; }
    Identity
}
```

**tinyhttp:**
- Builder pattern for response construction
- MIME type inference with `infer` crate
- Optional GZIP compression

```rust
Response::new()
    .status_line("HTTP/1.1 200 OK\r\n")
    .body(body)
    .mime("text/html")
```

---

## HTTP Protocol Implementation

### HTTP/1.1 Compliance

Both implementations follow RFC 7230-7235:

| Feature | tiny-http | tinyhttp |
|---------|-----------|----------|
| HTTP/1.0 | ✅ | ✅ |
| HTTP/1.1 | ✅ | ✅ |
| HTTP/2 | ❌ | Experimental |
| Keep-Alive | ✅ | ✅ |
| Pipelining | ✅ | ❌ |
| Chunked Encoding | ✅ | ❌ |
| 100-Continue | ✅ | ❌ |
| Connection Upgrade | ✅ | ❌ |

### Request Format Handling

```
GET /path HTTP/1.1\r\n
Host: example.com\r\n
Content-Length: 13\r\n
\r\n
Hello, World!
```

Both parsers handle this format, but with different approaches:
- **tiny-http:** Incremental line-by-line reading
- **tinyhttp:** Buffer fill + window search

### Response Format Generation

```
HTTP/1.1 200 OK\r\n
Date: Wed, 26 Mar 2026 12:00:00 GMT\r\n
Server: tiny-http (Rust)\r\n
Content-Type: text/plain\r\n
Content-Length: 13\r\n
\r\n
Hello, World!
```

---

## Performance Characteristics

### tiny-http Benchmarks

From `benches/bench.rs`:

1. **Sequential Requests:** Tests single-connection throughput
2. **Parallel Requests:** Tests 1000 concurrent connections

Key performance factors:
- Thread pool size (default: 4 minimum)
- Message queue capacity (8 requests)
- Buffer sizes (1024 bytes default)

### tinyhttp Benchmarks

From `benches/create_req.rs`:

- Uses Criterion for statistical analysis
- Reports ~15,000 req/s on Raspberry Pi 4

Key performance factors:
- Thread pool via `rusty_pool`
- Optional async mode with tokio
- GZIP compression overhead

---

## Comparison Matrix

| Aspect | tiny-http | tinyhttp |
|--------|-----------|----------|
| **Maturity** | v0.12.0, years of development | v0.5.0, active development |
| **API Style** | Low-level, explicit | High-level, macro-based |
| **Async Support** | No (sync only) | Yes (tokio feature) |
| **Routing** | Manual URL matching | Procedural macro attributes |
| **SSL/TLS** | Multiple backends | OpenSSL only |
| **Compression** | Manual | Built-in GZIP |
| **Middleware** | Manual implementation | Built-in support |
| **Documentation** | Extensive rustdocs | README-focused |
| **Tests** | Comprehensive | Basic |
| **Dependencies** | Minimal | Moderate |

---

## Use Cases

### Choose tiny-http When:
- You need fine-grained control over HTTP behavior
- Request pipelining is required
- You want minimal dependencies
- You prefer explicit over implicit
- Building libraries that need HTTP server functionality

### Choose tinyhttp When:
- You want rapid API development
- Macro-based routing appeals to you
- You need async/await support
- Built-in compression is desired
- Coming from Express.js-like frameworks

---

## Learning Outcomes

### From tiny-http:
1. **Raw HTTP parsing** - Understanding wire protocol
2. **Thread pool design** - Managing concurrent connections
3. **Sequential I/O** - Handling pipelined requests
4. **Transfer encoding** - Chunked vs Identity decisions
5. **SSL integration** - Multiple backend support

### From tinyhttp:
1. **Procedural macros** - DSL creation for routing
2. **Feature flags** - Sync vs async compilation
3. **MIME inference** - Content-type detection
4. **Compression** - GZIP encoder integration
5. **Builder patterns** - Fluent API design

---

## Related Projects in Source Tree

The `src.tinyhttp` directory contains additional HTTP-related crates:

- **httpdate** - HTTP date formatting (used by tiny-http)
- **chunked_transfer** - Chunked encoding implementation
- **rust-ascii** - ASCII string handling
- **fdlimit** - File descriptor limit management
- **fantoccini** - WebDriver client for browser automation

---

## Next Steps

See the following documents for deeper dives:

1. **tiny-http-rust.md** - Detailed tiny-http analysis
2. **tinyhttp-rust.md** - Detailed tinyhttp analysis
3. **http-protocol.md** - HTTP protocol implementation details
4. **comparison.md** - Feature-by-feature comparison
5. **rust-revision.md** - How to build HTTP servers in Rust

---

## Sources

- `/home/darkvoid/Boxxed/@formulas/src.rust/src.tiny/tiny-http/` - tiny-http source
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.tiny/src.tinyhttp/tinyhttp/` - tinyhttp source
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.tiny/src.tinyhttp/` - Supporting crates
