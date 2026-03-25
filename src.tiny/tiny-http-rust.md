# tiny-http: Deep Dive into Rust HTTP Server Design

## Overview

**tiny-http** is a mature, low-level HTTP server library for Rust that prioritizes correctness and minimal abstractions. At version 0.12.0, it represents years of refinement in synchronous HTTP server design.

**Key Design Goals:**
- Minimal dependencies
- RFC-compliant HTTP/1.1 implementation
- Synchronous I/O with thread-pool concurrency
- No async runtime dependencies

---

## Architecture Deep Dive

### Server Lifecycle

```
┌─────────────────────────────────────────────────────────────────┐
│                        Server::new()                             │
│                                                                  │
│  1. Bind TCP listener                                            │
│  2. Create MessagesQueue (MPSC channel)                          │
│  3. Spawn accept thread                                          │
│  4. Create TaskPool (thread pool)                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Accept Thread                               │
│                                                                  │
│  loop {                                                          │
│    let (sock, addr) = listener.accept()?;                        │
│    task_pool.spawn(|| {                                          │
│      for request in ClientConnection::new(sock) {                │
│        messages.push(Message::NewRequest(request));              │
│      }                                                           │
│    });                                                           │
│  }                                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Application Thread                            │
│                                                                  │
│  loop {                                                          │
│    let request = server.recv()?;  // Blocks                      │
│    handle(request);                                              │
│  }                                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. Server

The main entry point that manages:
- TCP listener
- Message queue for requests
- Close signal for graceful shutdown

```rust
pub struct Server {
    close: Arc<AtomicBool>,           // Shutdown signal
    messages: Arc<MessagesQueue<Message>>,  // Request queue
    listening_addr: ListenAddr,       // Bound address
}

impl Server {
    pub fn http<A>(addr: A) -> Result<Server, Box<dyn Error + Send + Sync>>
    where A: ToSocketAddrs {
        Server::new(ServerConfig {
            addr: ConfigListenAddr::from_socket_addrs(addr)?,
            ssl: None,
        })
    }

    pub fn recv(&self) -> IoResult<Request> {
        match self.messages.pop() {
            Some(Message::Error(err)) => Err(err),
            Some(Message::NewRequest(rq)) => Ok(rq),
            None => Err(IoError::new(IoErrorKind::Other, "thread unblocked")),
        }
    }
}
```

#### 2. ClientConnection

Iterates over requests from a single TCP connection, handling:
- Request pipelining
- Connection keep-alive
- Sequential response ordering

```rust
pub struct ClientConnection {
    remote_addr: IoResult<Option<SocketAddr>>,
    source: SequentialReaderBuilder<BufReader<RefinedTcpStream>>,
    sink: SequentialWriterBuilder<BufWriter<RefinedTcpStream>>,
    next_header_source: SequentialReader<BufReader<RefinedTcpStream>>,
    no_more_requests: bool,
    secure: bool,
}

impl Iterator for ClientConnection {
    type Item = Request;

    fn next(&mut self) -> Option<Request> {
        if self.no_more_requests {
            return None;
        }

        loop {
            let rq = match self.read() {
                Err(ReadError::WrongRequestLine) => {
                    // Send 400, close connection
                    return None;
                }
                Ok(rq) => rq,
            };

            // Check Connection header for keep-alive
            match connection_header {
                Some(val) if val.contains("close") => self.no_more_requests = true,
                None if *rq.http_version() == HTTPVersion(1, 0) => self.no_more_requests = true,
                _ => (),
            }

            return Some(rq);
        }
    }
}
```

#### 3. SequentialReader/Writer

Ensures ordered I/O operations for pipelined requests:

```rust
// Simplified concept
pub struct SequentialReaderBuilder<R> {
    inner: R,
    next_read: Option<Receiver<R>>,
}

impl<R: Read> SequentialReaderBuilder<R> {
    pub fn next(&mut self) -> Option<SequentialReader<R>> {
        // Returns the next reader in sequence
        // Previous reader must be dropped first
    }
}
```

This ensures that for pipelined requests:
1. Request 1's body is read completely before Request 2
2. Response 1 is sent completely before Response 2

---

## Request Parsing

### Line-by-Line Parsing

```rust
fn read_next_line(&mut self) -> IoResult<AsciiString> {
    let mut buf = Vec::new();
    let mut prev_byte_was_cr = false;

    loop {
        let byte = self.next_header_source.by_ref().bytes().next();
        let byte = match byte {
            Some(b) => b?,
            None => return Err(IoError::new(
                ErrorKind::ConnectionAborted,
                "Unexpected EOF"
            )),
        };

        if byte == b'\n' && prev_byte_was_cr {
            buf.pop(); // Remove the '\r'
            return AsciiString::from_ascii(buf)
                .map_err(|_| IoError::new(
                    ErrorKind::InvalidInput,
                    "Header is not in ASCII"
                ));
        }

        prev_byte_was_cr = byte == b'\r';
        buf.push(byte);
    }
}
```

**Key observations:**
- Strict CRLF (`\r\n`) handling
- ASCII-only headers (per HTTP spec)
- Byte-by-byte reading for correctness

### Request Line Parsing

```rust
fn parse_request_line(line: &str) -> Result<(Method, String, HTTPVersion), ReadError> {
    let mut parts = line.split(' ');

    let method = parts.next().and_then(|w| w.parse().ok());
    let path = parts.next().map(ToOwned::to_owned);
    let version = parts.next().and_then(|w| parse_http_version(w).ok());

    method
        .and_then(|method| Some((method, path?, version?)))
        .ok_or(ReadError::WrongRequestLine)
}

fn parse_http_version(version: &str) -> Result<HTTPVersion, ReadError> {
    match version {
        "HTTP/0.9" => Ok((0, 9)),
        "HTTP/1.0" => Ok((1, 0)),
        "HTTP/1.1" => Ok((1, 1)),
        "HTTP/2.0" => Ok((2, 0)),
        "HTTP/3.0" => Ok((3, 0)),
        _ => Err(ReadError::WrongRequestLine),
    }
}
```

### Body Reading Strategy

```rust
let reader = if connection_upgrade {
    // Keep full reader for websockets
    Box::new(source_data)
} else if let Some(content_length) = content_length {
    if content_length == 0 {
        Box::new(io::empty())
    } else if content_length <= 1024 && !expects_continue {
        // Small body: read into buffer immediately
        let mut buffer = vec![0; content_length];
        // ... read loop ...
        Box::new(Cursor::new(buffer))
    } else {
        // Large body: use EqualReader to limit reads
        let (data_reader, _) = EqualReader::new(source_data, content_length);
        Box::new(FusedReader::new(data_reader))
    }
} else if transfer_encoding.is_some() {
    // Chunked encoding
    Box::new(FusedReader::new(Decoder::new(source_data)))
} else {
    // No body
    Box::new(io::empty())
};
```

**Design decisions:**
- Small bodies (< 1KB) buffered immediately
- Large bodies use streaming readers
- Chunked encoding via `chunked_transfer` crate
- `FusedReader` prevents over-reading

---

## Response Handling

### Transfer Encoding Selection

```rust
fn choose_transfer_encoding(
    status_code: StatusCode,
    request_headers: &[Header],
    http_version: &HTTPVersion,
    entity_length: &Option<usize>,
    has_additional_headers: bool,
    chunked_threshold: usize,
) -> TransferEncoding {
    // HTTP 1.0 doesn't support chunked
    if *http_version <= (1, 0) {
        return TransferEncoding::Identity;
    }

    // 1xx and 204 must not have body
    if status_code.0 < 200 || status_code.0 == 204 {
        return TransferEncoding::Identity;
    }

    // Check client's TE (Transfer-Encoding) header
    let user_request = request_headers
        .iter()
        .find(|h| h.field.equiv("TE"))
        .and_then(|h| parse_te_header(&h.value));

    if let Some(te) = user_request {
        return te;
    }

    // Additional headers require chunked
    if has_additional_headers {
        return TransferEncoding::Chunked;
    }

    // Unknown length or large content uses chunked
    if entity_length.as_ref().map_or(true, |l| *l >= chunked_threshold) {
        return TransferEncoding::Chunked;
    }

    // Default to Identity
    TransferEncoding::Identity
}
```

### Response Writing

```rust
pub fn raw_print<W: Write>(
    mut self,
    mut writer: W,
    http_version: HTTPVersion,
    request_headers: &[Header],
    do_not_send_body: bool,
    upgrade: Option<&str>,
) -> IoResult<()> {
    // Add Date header if missing
    if !self.headers.iter().any(|h| h.field.equiv("Date")) {
        self.headers.insert(0, build_date_header());
    }

    // Add Server header
    if !self.headers.iter().any(|h| h.field.equiv("Server")) {
        self.headers.insert(
            0,
            Header::from_bytes(&b"Server"[..], &b"tiny-http (Rust)"[..]).unwrap(),
        );
    }

    // Handle upgrade (WebSocket) header
    if let Some(upgrade) = upgrade {
        self.headers.insert(
            Header::from_bytes(&b"Upgrade"[..], upgrade.as_bytes()).unwrap(),
        );
        self.headers.insert(
            Header::from_bytes(&b"Connection"[..], &b"upgrade"[..]).unwrap(),
        );
    }

    // Buffer response if Content-Length unknown for HTTP/1.0
    let (mut reader, data_length) = match (self.data_length, transfer_encoding) {
        (Some(l), _) => (Box::new(self.reader), Some(l)),
        (None, Some(Identity)) => {
            // Must buffer to determine length
            let mut buf = Vec::new();
            self.reader.read_to_end(&mut buf)?;
            let l = buf.len();
            (Box::new(Cursor::new(buf)), Some(l))
        }
        _ => (Box::new(self.reader), None),
    };

    // Write headers
    write_message_header(writer.by_ref(), &http_version, &self.status_code, &self.headers)?;

    // Write body
    match transfer_encoding {
        Some(Chunked) => {
            use chunked_transfer::Encoder;
            let mut encoder = Encoder::new(writer);
            io::copy(&mut reader, &mut encoder)?;
        }
        Some(Identity) => {
            io::copy(&mut reader, &mut writer)?;
        }
        _ => (),
    }

    Ok(())
}
```

---

## Thread Pool Implementation

### TaskPool Design

```rust
pub struct TaskPool {
    sharing: Arc<Sharing>,
}

struct Sharing {
    todo: Mutex<VecDeque<Box<dyn FnMut() + Send>>>,  // Task queue
    condvar: Condvar,                                  // Notification
    active_tasks: AtomicUsize,                         // Running threads
    waiting_tasks: AtomicUsize,                        // Idle threads
}

static MIN_THREADS: usize = 4;

impl TaskPool {
    pub fn new() -> TaskPool {
        let pool = TaskPool {
            sharing: Arc::new(Sharing {
                todo: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                active_tasks: AtomicUsize::new(0),
                waiting_tasks: AtomicUsize::new(0),
            }),
        };

        // Start minimum threads
        for _ in 0..MIN_THREADS {
            pool.add_thread(None);
        }

        pool
    }

    pub fn spawn(&self, code: Box<dyn FnMut() + Send>) {
        let mut queue = self.sharing.todo.lock().unwrap();

        if self.sharing.waiting_tasks.load(Ordering::Acquire) == 0 {
            // No idle workers, spawn new thread
            self.add_thread(Some(code));
        } else {
            // Wake up idle worker
            queue.push_back(code);
            self.sharing.condvar.notify_one();
        }
    }

    fn add_thread(&self, initial_fn: Option<Box<dyn FnMut() + Send>>) {
        let sharing = self.sharing.clone();

        thread::spawn(move || {
            let _active_guard = Registration::new(&sharing.active_tasks);

            // Execute initial task if provided
            if let Some(mut f) = initial_fn {
                f();
            }

            loop {
                let mut task: Box<dyn FnMut() + Send> = {
                    let mut todo = sharing.todo.lock().unwrap();
                    loop {
                        if let Some(task) = todo.pop_front() {
                            break task;
                        }

                        let _waiting_guard = Registration::new(&sharing.waiting_tasks);

                        // Wait for work (or timeout if above MIN_THREADS)
                        let received = if sharing.active_tasks.load(Ordering::Acquire) <= MIN_THREADS {
                            todo = sharing.condvar.wait(todo).unwrap();
                            true
                        } else {
                            let (new_lock, waitres) = sharing
                                .condvar
                                .wait_timeout(todo, Duration::from_millis(5000))
                                .unwrap();
                            todo = new_lock;
                            !waitres.timed_out()
                        };

                        // Exit if no work after timeout
                        if !received && todo.is_empty() {
                            return;
                        }
                    }
                };

                task();
            }
        });
    }
}
```

**Key features:**
- Minimum 4 threads always active
- Dynamic scaling based on load
- Idle threads die after 5 seconds
- Condition variable for efficient waiting

---

## SSL/TLS Support

### Feature Flags

```toml
[features]
ssl = ["ssl-openssl"]
ssl-openssl = ["openssl", "zeroize"]
ssl-rustls = ["rustls", "rustls-pemfile", "zeroize"]
ssl-native-tls = ["native-tls", "zeroize"]
```

**Only one SSL backend can be enabled at a time** (enforced via `compile_error!`).

### SSL Accept Flow

```rust
let ssl: Option<SslContext> = match ssl_config {
    Some(config) => Some(SslContext::from_pem(
        config.certificate,
        Zeroizing::new(config.private_key),
    )?),
    None => None,
};

// In accept loop:
let new_client = match server.accept() {
    Ok((sock, _)) => {
        let (read_closable, write_closable) = match ssl {
            None => RefinedTcpStream::new(sock),
            Some(ref ssl) => {
                let sock = match ssl.accept(sock) {
                    Ok(s) => s,
                    Err(_) => continue, // Skip failed handshakes
                };
                RefinedTcpStream::new(sock)
            }
        };
        Ok(ClientConnection::new(write_closable, read_closable))
    }
    Err(e) => Err(e),
};
```

**Security notes:**
- Private keys zeroized after use (`Zeroizing`)
- Failed handshakes silently dropped
- SSL applied per-connection

---

## HTTP/1.1 Compliance

### Implemented Features

| Feature | Status | Notes |
|---------|--------|-------|
| GET, POST, PUT, DELETE | ✅ | All standard methods |
| HEAD | ✅ | No body in response |
| OPTIONS, TRACE | ✅ | |
| PATCH | ✅ | |
| CONNECT | ✅ | For proxies |
| Keep-Alive | ✅ | Default in HTTP/1.1 |
| Pipelining | ✅ | Ordered responses |
| Chunked Encoding | ✅ | Via `chunked_transfer` |
| 100-Continue | ✅ | Expect handling |
| Connection: Upgrade | ✅ | WebSocket support |
| Range Requests | ❌ | Not implemented |
| Compression | ❌ | Manual implementation |

### Header Handling

**Forbidden headers** (cannot be set by user):
- `Connection`
- `Trailer`
- `Transfer-Encoding`
- `Upgrade`

**Special headers:**
- `Content-Length`: Setting affects `data_length`, header may not appear
- `Content-Type`: Can only be set once (overwrites previous)
- `Date`: Auto-added if missing
- `Server`: Auto-added as "tiny-http (Rust)"

### Status Codes

```rust
pub struct StatusCode(pub u16);

impl StatusCode {
    pub fn default_reason_phrase(&self) -> &'static str {
        match self.0 {
            100 => "Continue",
            101 => "Switching Protocols",
            200 => "OK",
            204 => "No Content",
            301 => "Moved Permanently",
            404 => "Not Found",
            500 => "Internal Server Error",
            // ... all standard codes
            _ => "Unknown",
        }
    }
}
```

---

## Performance Considerations

### Buffer Sizes

```rust
// ClientConnection
BufReader::with_capacity(1024, read_socket)
BufWriter::with_capacity(1024, write_socket)

// MessagesQueue
MessagesQueue::with_capacity(8)
```

### Thread Pool Tuning

Default: 4 minimum threads
- Scales up under load
- Idle threads timeout after 5 seconds

### Memory Management

- Small request bodies (< 1KB) buffered immediately
- Large bodies streamed directly
- Response buffering only for HTTP/1.0 without Content-Length

---

## Error Handling

### Request Creation Errors

```rust
pub enum RequestCreationError {
    ExpectationFailed,          // 417 response needed
    CreationIoError(IoError),   // Network error
}
```

### Client Connection Errors

```rust
enum ReadError {
    WrongRequestLine,           // 400 response
    WrongHeader(HTTPVersion),   // 400 response
    ExpectationFailed(HTTPVersion),  // 417 response
    ReadIoError(IoError),       // Connection closed
}
```

### Graceful Degradation

```rust
fn ignore_client_closing_errors(result: io::Result<()>) -> io::Result<()> {
    result.or_else(|err| match err.kind() {
        ErrorKind::BrokenPipe => Ok(()),
        ErrorKind::ConnectionAborted => Ok(()),
        ErrorKind::ConnectionRefused => Ok(()),
        ErrorKind::ConnectionReset => Ok(()),
        _ => Err(err),
    })
}
```

Client disconnects during response are silently ignored.

---

## Testing Support

### TestRequest Utility

```rust
// For unit testing handlers
use tiny_http::TestRequest;

let request = TestRequest::default()
    .with_method(Method::Get)
    .with_url("/test")
    .with_header(Header::from_bytes(&b"Accept"[..], &b"text/plain"[..]).unwrap());
```

---

## Example: Multi-threaded Server

```rust
use std::sync::Arc;
use tiny_http::{Server, Response, Header};

fn main() {
    let server = Arc::new(Server::http("0.0.0.0:8080").unwrap());
    let mut handles = Vec::new();

    // Spawn worker threads
    for _ in 0..4 {
        let server = server.clone();
        handles.push(std::thread::spawn(move || {
            for request in server.incoming_requests() {
                let response = Response::from_string("Hello, World!")
                    .with_header(Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"text/plain; charset=utf-8"[..]
                    ).unwrap());
                let _ = request.respond(response);
            }
        }));
    }

    // Wait for all workers
    for h in handles {
        h.join().unwrap();
    }
}
```

---

## Limitations

### Known Constraints

1. **No HTTP/2 support** - Protocol fundamentally different
2. **No async/await** - Synchronous only
3. **No automatic compression** - Must be implemented manually
4. **Limited WebSocket support** - Basic upgrade only
5. **No graceful shutdown** - Abrupt on Server drop

### When NOT to Use

- High-concurrency (>10k connections) - Consider async runtimes
- HTTP/2 required - Use hyper or similar
- Streaming large files - Buffer management limited
- Complex routing - No built-in router

---

## Summary

tiny-http excels as:
- **Learning tool** - Clear HTTP implementation
- **Embeddable server** - Minimal dependencies
- **Testing** - Simple mock server creation
- **Low-concurrency apps** - Internal tools, admin interfaces

The synchronous design makes it easy to understand but limits scalability. For production high-traffic services, consider async alternatives like hyper or tokio-based solutions.
