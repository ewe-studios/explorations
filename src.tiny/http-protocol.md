# HTTP Protocol Implementation Deep Dive

## HTTP/1.1 From First Principles

This document explains how HTTP works at the wire level, using examples from both tiny-http and tinyhttp implementations.

---

## What is HTTP?

**HTTP (HyperText Transfer Protocol)** is an application-layer protocol for distributed, collaborative, hypermedia information systems. It is the foundation of data communication on the World Wide Web.

### Key Characteristics

- **Request-Response Protocol**: Client sends request, server sends response
- **Stateless**: Each request is independent (cookies/sessions add state)
- **Text-based**: Human-readable protocol (unlike binary HTTP/2)
- **TCP-based**: Relies on TCP for reliable delivery

---

## The HTTP Request

### Structure

```
┌─────────────────────────────────────────┐
│           Request Line                   │
│           (Method + URL + Version)       │
├─────────────────────────────────────────┤
│           Headers                        │
│           (Key: Value pairs)             │
├─────────────────────────────────────────┤
│           Empty Line                     │
│           (\r\n\r\n)                     │
├─────────────────────────────────────────┤
│           Body (optional)                │
│           (Data payload)                 │
└─────────────────────────────────────────┘
```

### Example Request

```
GET /api/users/123 HTTP/1.1\r\n
Host: example.com\r\n
User-Agent: Mozilla/5.0\r\n
Accept: application/json\r\n
Authorization: Bearer abc123\r\n
Content-Length: 0\r\n
\r\n
```

### Request Line

```
METHOD SP Request-URI SP HTTP-Version CRLF
```

**Components:**
- **Method**: Action to perform (GET, POST, PUT, DELETE, etc.)
- **Request-URI**: Path and query string
- **HTTP-Version**: Protocol version (HTTP/1.0, HTTP/1.1)

**Parsing in tiny-http:**

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
```

**Parsing in tinyhttp:**

```rust
let status_line_index = buf.windows(2)
    .enumerate()
    .find(|(_, w)| matches!(*w, b"\r\n"))
    .map(|(i, _)| i)
    .unwrap();

let status_line = std::str::from_utf8(&buf[..status_line_index]).unwrap();
let str_status_line = Vec::from_iter(status_line.split_whitespace());
```

### HTTP Methods

| Method | Description | Idempotent | Safe |
|--------|-------------|------------|------|
| GET | Retrieve resource | ✅ | ✅ |
| HEAD | Like GET, no body | ✅ | ✅ |
| POST | Create/submit | ❌ | ❌ |
| PUT | Replace resource | ✅ | ❌ |
| DELETE | Remove resource | ✅ | ❌ |
| PATCH | Partial update | ❌* | ❌ |
| OPTIONS | Get capabilities | ✅ | ✅ |
| TRACE | Echo request | ✅ | ✅ |
| CONNECT | Tunnel connection | ❌ | ❌ |

*PATCH idempotency depends on implementation

**Method implementation in tiny-http:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connect,
    Options,
    Trace,
    Patch,
    NonStandard(AsciiString),
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> Result<Method, ()> {
        Ok(match s {
            "GET" => Method::Get,
            "HEAD" => Method::Head,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            "CONNECT" => Method::Connect,
            "OPTIONS" => Method::Options,
            "TRACE" => Method::Trace,
            "PATCH" => Method::Patch,
            s => Method::NonStandard(AsciiString::from_ascii(s).map_err(|_| ())?),
        })
    }
}
```

### HTTP Headers

Format: `Header-Name: Header-Value\r\n`

**Common Headers:**

| Header | Purpose | Example |
|--------|---------|---------|
| Host | Target server | `Host: example.com:8080` |
| User-Agent | Client info | `User-Agent: Mozilla/5.0` |
| Accept | Desired content type | `Accept: application/json` |
| Content-Type | Body format | `Content-Type: application/json` |
| Content-Length | Body size | `Content-Length: 256` |
| Authorization | Credentials | `Authorization: Bearer xyz` |
| Cookie | Session data | `Cookie: session=abc123` |
| Connection | Keep-alive | `Connection: keep-alive` |

**Header Parsing in tiny-http:**

```rust
fn read_next_line(&mut self) -> IoResult<AsciiString> {
    let mut buf = Vec::new();
    let mut prev_byte_was_cr = false;

    loop {
        let byte = self.next_header_source.by_ref().bytes().next();
        let byte = match byte {
            Some(b) => b?,
            None => return Err(IoError::new(ConnectionAborted, "Unexpected EOF")),
        };

        if byte == b'\n' && prev_byte_was_cr {
            buf.pop(); // Remove the '\r'
            return AsciiString::from_ascii(buf)...;
        }

        prev_byte_was_cr = byte == b'\r';
        buf.push(byte);
    }
}

// Parse header line
let header: Header = line.parse().unwrap();
// Header: "Content-Type: text/html"
```

**Header Structure:**

```rust
#[derive(Debug, Clone)]
pub struct Header {
    pub field: HeaderField,
    pub value: AsciiString,
}

pub struct HeaderField(AsciiString);  // Case-insensitive comparison

impl HeaderField {
    pub fn equiv(&self, other: &'static str) -> bool {
        other.eq_ignore_ascii_case(self.as_str().as_str())
    }
}
```

**Important:** Header names are case-insensitive per RFC 7230.

### Request Body

The body contains data sent to the server (POST, PUT, PATCH).

**Body determination:**

1. **Content-Length specified**: Read exactly N bytes
2. **Transfer-Encoding: chunked**: Read until zero-length chunk
3. **Neither**: No body (or connection close terminates)

**Body reading in tiny-http:**

```rust
let reader = if let Some(content_length) = content_length {
    if content_length <= 1024 && !expects_continue {
        // Small body: buffer immediately
        let mut buffer = vec![0; content_length];
        let mut offset = 0;
        while offset != content_length {
            let read = source_data.read(&mut buffer[offset..])?;
            if read == 0 {
                return Err(IoError::new(ConnectionAborted, "EOF before expected length"));
            }
            offset += read;
        }
        Box::new(Cursor::new(buffer))
    } else {
        // Large body: stream with length limiter
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

---

## The HTTP Response

### Structure

```
┌─────────────────────────────────────────┐
│           Status Line                   │
│           (Version + Code + Reason)     │
├─────────────────────────────────────────┤
│           Headers                        │
│           (Key: Value pairs)             │
├─────────────────────────────────────────┤
│           Empty Line                     │
│           (\r\n\r\n)                     │
├─────────────────────────────────────────┤
│           Body (optional)                │
│           (Response data)                │
└─────────────────────────────────────────┘
```

### Example Response

```
HTTP/1.1 200 OK\r\n
Date: Wed, 26 Mar 2026 12:00:00 GMT\r\n
Server: tiny-http (Rust)\r\n
Content-Type: application/json\r\n
Content-Length: 42\r\n
Connection: keep-alive\r\n
\r\n
{"id": 123, "name": "John Doe"}
```

### Status Line

```
HTTP-Version SP Status-Code SP Reason-Phrase CRLF
```

**Status Code Categories:**

| Range | Category | Description |
|-------|----------|-------------|
| 1xx | Informational | Request received, continuing |
| 2xx | Success | Action completed |
| 3xx | Redirection | Further action needed |
| 4xx | Client Error | Request was wrong |
| 5xx | Server Error | Server failed |

**Common Status Codes:**

| Code | Meaning | Description |
|------|---------|-------------|
| 200 | OK | Success |
| 201 | Created | Resource created |
| 204 | No Content | Success, no body |
| 301 | Moved Permanently | Redirect |
| 304 | Not Modified | Use cached version |
| 400 | Bad Request | Malformed request |
| 401 | Unauthorized | Authentication needed |
| 403 | Forbidden | No permission |
| 404 | Not Found | Resource doesn't exist |
| 429 | Too Many Requests | Rate limited |
| 500 | Internal Server Error | Server crash |
| 503 | Service Unavailable | Temporarily down |

**Status code implementation:**

```rust
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct StatusCode(pub u16);

impl StatusCode {
    pub fn default_reason_phrase(&self) -> &'static str {
        match self.0 {
            100 => "Continue",
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
    }
}
```

### Response Headers

**Required Headers:**

| Header | When Required |
|--------|---------------|
| Date | Always (auto-added) |
| Content-Type | When body present |
| Content-Length | Unless chunked |
| Server | Recommended |

**Response header writing:**

```rust
fn write_message_header<W>(
    mut writer: W,
    http_version: &HTTPVersion,
    status_code: &StatusCode,
    headers: &[Header],
) -> IoResult<()>
where
    W: Write,
{
    // Status line
    write!(
        &mut writer,
        "HTTP/{}.{} {} {}\r\n",
        http_version.0,
        http_version.1,
        status_code.0,
        status_code.default_reason_phrase()
    )?;

    // Headers
    for header in headers.iter() {
        writer.write_all(header.field.as_str().as_ref())?;
        write!(&mut writer, ": ")?;
        writer.write_all(header.value.as_str().as_ref())?;
        write!(&mut writer, "\r\n")?;
    }

    // Empty line (header/body separator)
    write!(&mut writer, "\r\n")?;

    Ok(())
}
```

---

## Connection Management

### Keep-Alive

HTTP/1.1 uses persistent connections by default:

```
# Request 1
GET /page1 HTTP/1.1\r\n
Host: example.com\r\n
\r\n

# Response 1
HTTP/1.1 200 OK\r\n
\r\n
...body...

# Request 2 (same connection!)
GET /page2 HTTP/1.1\r\n
Host: example.com\r\n
\r\n

# Response 2
HTTP/1.1 200 OK\r\n
\r\n
...body...
```

**Connection header values:**

| Value | Behavior |
|-------|----------|
| `keep-alive` | Reuse connection |
| `close` | Close after response |
| (none in 1.1) | Keep alive (default) |
| (none in 1.0) | Close (default) |

**Keep-alive detection:**

```rust
let connection_header = rq
    .headers()
    .iter()
    .find(|h| h.field.equiv("Connection"))
    .map(|h| h.value.as_str());

match connection_header {
    Some(val) if val.contains("close") => self.no_more_requests = true,
    Some(val) if val.contains("upgrade") => self.no_more_requests = true,
    None if *rq.http_version() == HTTPVersion(1, 0) => self.no_more_requests = true,
    _ => (),  // Keep connection alive
}
```

### Request Pipelining

Multiple requests can be sent before receiving responses:

```
Client                              Server
  │                                   │
  │────GET /1 ───────────────────────>│
  │────GET /2 ───────────────────────>│
  │────GET /3 ───────────────────────>│
  │                                   │
  │<────Response 1────────────────────│
  │<────Response 2────────────────────│
  │<────Response 3────────────────────│
  │                                   │
```

**Key constraint:** Responses MUST be sent in order.

tiny-http handles this with `SequentialWriter`:

```rust
pub struct SequentialWriterBuilder<W> {
    inner: W,
    next_write: Option<Receiver<W>>,
}

impl<W: Write> SequentialWriterBuilder<W> {
    pub fn next(&mut self) -> Option<SequentialWriter<W>> {
        // Returns next writer in sequence
        // Previous writer must be dropped first
    }
}
```

---

## Transfer Encodings

### Identity (Default)

Body sent as-is with Content-Length:

```
HTTP/1.1 200 OK\r\n
Content-Length: 13\r\n
\r\n
Hello, World!
```

### Chunked Encoding

Body sent in chunks without knowing total length:

```
HTTP/1.1 200 OK\r\n
Transfer-Encoding: chunked\r\n
\r\n
7\r\n
Mozilla\r\n
9\r\n
Developer\r\n
7\r\n
Network\r\n
0\r\n
\r\n
```

**Chunk format:**
```
<chunk-size-in-hex>\r\n
<chunk-data>\r\n
```

**Final chunk:** `0\r\n\r\n` (zero-length chunk)

**Chunked encoding in tiny-http:**

```rust
use chunked_transfer::Encoder;

match transfer_encoding {
    Some(TransferEncoding::Chunked) => {
        let mut encoder = Encoder::new(writer);
        io::copy(&mut reader, &mut encoder)?;
        // Encoder automatically writes final 0\r\n\r\n
    }
    Some(TransferEncoding::Identity) => {
        io::copy(&mut reader, &mut writer)?;
    }
    _ => (),
}
```

**When chunked is required:**

```rust
fn choose_transfer_encoding(...) -> TransferEncoding {
    // HTTP/1.0 doesn't support chunked
    if *http_version <= (1, 0) {
        return TransferEncoding::Identity;
    }

    // 1xx and 204 must not have body
    if status_code < 200 || status_code == 204 {
        return TransferEncoding::Identity;
    }

    // Unknown length requires chunked (HTTP/1.1)
    if entity_length.is_none() {
        return TransferEncoding::Chunked;
    }

    // Large content benefits from chunked
    if entity_length.unwrap() >= chunked_threshold {
        return TransferEncoding::Chunked;
    }

    TransferEncoding::Identity
}
```

---

## Special HTTP Features

### 100-Continue

Client can ask permission before sending body:

```
# Client sends headers only
POST /upload HTTP/1.1\r\n
Content-Length: 1000000\r\n
Expect: 100-continue\r\n
\r\n

# Server responds
HTTP/1.1 100 Continue\r\n
\r\n

# Client sends body
[...body data...]

# Server sends final response
HTTP/1.1 200 OK\r\n
\r\n
```

**Handling in tiny-http:**

```rust
let expects_continue = match headers
    .iter()
    .find(|h| h.field.equiv("Expect"))
    .map(|h| h.value.as_str())
{
    Some("100-continue") => true,
    Some(_) => return Err(RequestCreationError::ExpectationFailed),  // 417
    None => false,
};

// Later, when body is read:
if self.must_send_continue {
    let msg = Response::new_empty(StatusCode(100));
    msg.raw_print(self.response_writer.as_mut().unwrap().by_ref(), ...)?;
    self.response_writer.as_mut().unwrap().flush()?;
    self.must_send_continue = false;
}
```

### Connection Upgrade (WebSocket)

Upgrades HTTP connection to different protocol:

```
# Client request
GET /chat HTTP/1.1\r\n
Upgrade: websocket\r\n
Connection: Upgrade\r\n
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n
Sec-WebSocket-Version: 13\r\n
\r\n

# Server response
HTTP/1.1 101 Switching Protocols\r\n
Upgrade: websocket\r\n
Connection: Upgrade\r\n
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n
\r\n

# Now both sides speak WebSocket protocol
```

**Upgrade handling:**

```rust
pub fn upgrade<R: Read>(
    mut self,
    protocol: &str,
    response: Response<R>,
) -> Box<dyn ReadWrite + Send> {
    // Send upgrade response
    response.raw_print(
        self.response_writer.as_mut().unwrap().by_ref(),
        self.http_version.clone(),
        &self.headers,
        false,
        Some(protocol),  // Sets Upgrade header
    ).ok();

    self.response_writer.as_mut().unwrap().flush().ok();

    // Return raw stream for custom protocol
    let stream = CustomStream::new(
        self.extract_reader_impl(),
        self.extract_writer_impl(),
    );
    Box::new(stream)
}
```

### HEAD Requests

Like GET but without response body:

```
HEAD /resource HTTP/1.1\r\n
Host: example.com\r\n
\r\n

HTTP/1.1 200 OK\r\n
Content-Type: text/plain\r\n
Content-Length: 1234\r\n
\r\n
(no body)
```

**HEAD handling:**

```rust
let do_not_send_body = self.method == Method::Head;

response.raw_print(
    writer.by_ref(),
    self.http_version.clone(),
    &self.headers,
    do_not_send_body,  // Don't send body
    None,
)?;
```

---

## Error Handling

### Client Errors (4xx)

| Code | Cause | Example |
|------|-------|---------|
| 400 | Malformed request | Invalid headers |
| 401 | No authentication | Missing token |
| 403 | Forbidden | No permission |
| 404 | Not found | Unknown URL |
| 405 | Method not allowed | POST to GET endpoint |
| 408 | Request timeout | Slow client |
| 413 | Payload too large | Big upload |
| 414 | URI too long | Long URL |
| 429 | Too many requests | Rate limited |

**Error response generation:**

```rust
match self.read() {
    Err(ReadError::WrongRequestLine) => {
        let response = Response::new_empty(StatusCode(400));
        response.raw_print(writer, HTTPVersion(1, 1), &[], false, None).ok();
        return None;  // Close connection
    }
    Err(ReadError::WrongHeader(ver)) => {
        let response = Response::new_empty(StatusCode(400));
        response.raw_print(writer, ver, &[], false, None).ok();
        return None;
    }
    Err(ReadError::ExpectationFailed(ver)) => {
        let response = Response::new_empty(StatusCode(417));
        response.raw_print(writer, ver, &[], true, None).ok();
        return None;
    }
    _ => { /* Normal processing */ }
}
```

### Server Errors (5xx)

| Code | Cause | Example |
|------|-------|---------|
| 500 | Internal error | Panic in handler |
| 501 | Not implemented | Unknown method |
| 502 | Bad gateway | Upstream error |
| 503 | Unavailable | Maintenance |
| 505 | HTTP version unsupported | HTTP/2.0 request |

**Automatic 500 on panic:**

```rust
impl Drop for Request {
    fn drop(&mut self) {
        // If Request is dropped without response, send 500
        if self.response_writer.is_some() {
            let response = Response::empty(500);
            let _ = self.respond_impl(response);
        }
    }
}
```

---

## HTTP over TLS (HTTPS)

### TLS Handshake

Before HTTP communication:

```
Client                          Server
  │                               │
  │────ClientHello──────────────>│
  │<────ServerHello──────────────│
  │<────Certificate──────────────│
  │<────ServerHelloDone──────────│
  │                               │
  │────KeyExchange──────────────>│
  │────ChangeCipherSpec─────────>│
  │────Finished─────────────────>│
  │<────ChangeCipherSpec─────────│
  │<────Finished─────────────────│
  │                               │
  │◄──── Encrypted HTTP ────────►│
```

### HTTPS in tiny-http

```rust
#[cfg(feature = "ssl-rustls")]
type SslContext = rustls::ServerConfig;

let ssl: Option<SslContext> = match ssl_config {
    Some(config) => Some(SslContext::from_pem(
        config.certificate,
        Zeroizing::new(config.private_key),
    )?),
    None => None,
};

// In accept loop:
match server.accept() {
    Ok((sock, _)) => {
        match ssl {
            None => RefinedTcpStream::new(sock),
            Some(ref ssl) => {
                // TLS handshake
                let sock = match ssl.accept(sock) {
                    Ok(s) => s,  // TLS established
                    Err(_) => continue,  // Handshake failed
                };
                RefinedTcpStream::new(sock)
            }
        }
    }
    Err(e) => Err(e),
}
```

---

## Summary

### Request Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENT                                   │
│                                                                  │
│  1. Build request line: "GET /path HTTP/1.1"                    │
│  2. Add headers: "Host: example.com"                            │
│  3. Add empty line: "\r\n"                                      │
│  4. Add body (if any)                                           │
│  5. Send via TCP socket                                         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ TCP Stream
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         SERVER                                   │
│                                                                  │
│  1. Read bytes until "\r\n\r\n" (headers)                       │
│  2. Parse request line                                          │
│  3. Parse headers                                               │
│  4. Read body (Content-Length or chunked)                       │
│  5. Process request                                             │
│  6. Build response                                              │
│  7. Send response                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Takeaways

1. **CRLF line endings** - All lines end with `\r\n`
2. **Headers case-insensitive** - `Content-Type` = `content-type`
3. **Empty line separator** - `\r\n\r\n` divides headers from body
4. **Length determines body** - Content-Length or chunked encoding
5. **Connection reuse** - Keep-alive is default in HTTP/1.1
6. **Response ordering** - Responses match request order (pipelining)

---

## References

- RFC 7230 - HTTP/1.1 Message Syntax and Routing
- RFC 7231 - HTTP/1.1 Semantics and Content
- RFC 7235 - HTTP/1.1 Authentication
- RFC 2818 - HTTP Over TLS (HTTPS)
