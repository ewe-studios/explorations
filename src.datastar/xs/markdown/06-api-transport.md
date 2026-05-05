# xs -- API & Transport

## HTTP API Routes

**File**: `src/api.rs`

All communication with xs happens over HTTP/1.1. The same API is served regardless of transport (Unix socket, TCP, TLS, or Iroh).

| Method | Path | Purpose |
|--------|------|---------|
| GET | `/` | Stream frames (with query params for filtering) |
| GET | `/version` | Server version string |
| GET | `/last` | Most recent frame(s) across all topics |
| GET | `/last/<topic>` | Most recent frame(s) for a specific topic |
| GET | `/cas/<hash>` | Retrieve CAS content by SRI hash |
| GET | `/<scru128_id>` | Get a single frame by ID |
| POST | `/append/<topic>` | Append a new frame |
| POST | `/cas` | Store content in CAS (returns hash) |
| POST | `/import` | Import a frame preserving its original ID |
| POST | `/eval` | Evaluate a Nushell script |
| DELETE | `/<scru128_id>` | Remove a frame by ID |

## Content Negotiation

The API supports two output formats based on the `Accept` header:

### NDJSON (default)

One JSON object per line, newline-delimited:
```
{"id":"abc","topic":"user.messages","hash":"sha256-...","meta":null,"ttl":null}
{"id":"def","topic":"user.messages","hash":"sha256-...","meta":null,"ttl":null}
```

### SSE (Server-Sent Events)

Requested via `Accept: text/event-stream`:
```
id: abc
data: {"id":"abc","topic":"user.messages","hash":"sha256-..."}

id: def
data: {"id":"def","topic":"user.messages","hash":"sha256-..."}

```

SSE format is useful for browser clients using the EventSource API.

## GET / — Stream Frames

Query parameters map directly to `ReadOptions`:

| Param | Example | Effect |
|-------|---------|--------|
| `follow` | `?follow` | Stream indefinitely |
| `pulse` | `?pulse=5000` | Follow with heartbeat every 5s |
| `new` | `?new` | Skip historical, only live |
| `after` | `?after=<id>` | Start after ID (exclusive) |
| `from` | `?from=<id>` | Start from ID (inclusive) |
| `limit` | `?limit=10` | Return at most 10 frames |
| `last` | `?last=5` | Return the last 5 frames |
| `topic` | `?topic=user.*` | Filter by topic (supports wildcards) |

## POST /append/<topic>

### Request

- **Path**: Topic name (validated per ADR 0001/0002)
- **Body**: Content to store in CAS (streamed directly to cacache writer)
- **Headers**:
  - `xs-meta`: Base64-encoded JSON metadata (decoded and stored as `frame.meta`)
  - `xs-ttl`: TTL policy string (`forever`, `ephemeral`, `time:<ms>`, `last:<n>`)

### Response

JSON-serialized Frame with the assigned SCRU128 ID:
```json
{"id":"0v4fkdz7k2mfj9f2nn8yx3h71","topic":"user.messages","hash":"sha256-abc...","meta":null,"ttl":null}
```

### Empty Body

If the request body is empty (0 bytes), no CAS entry is created and `hash` is `None`. The frame may still carry metadata via `xs-meta` header.

## GET /cas/<hash>

Returns the raw content bytes for the given SRI hash. Response is `application/octet-stream`. Returns 404 if hash not found.

## POST /import

Imports a pre-formed frame (preserving its original ID). Used for replication and backup restore. The body is a JSON-serialized Frame.

## POST /eval

Evaluates a Nushell script with all xs commands available. Body is the Nushell source code. Response is the evaluation result as JSON.

## Transport Layer

**File**: `src/listener.rs`

```rust
pub enum Listener {
    Tcp(TcpListener),
    Unix(UnixListener),
    Iroh(Endpoint, String),  // Endpoint + connection ticket
}
```

### Address Detection Logic

The `--expose` flag determines transport based on address format:

```rust
fn detect_address(addr: &str) -> ConnectionKind {
    if addr.starts_with('/') || addr.starts_with('.') {
        // Unix domain socket
        ConnectionKind::Unix(PathBuf::from(addr))
    } else if addr.starts_with("iroh://") {
        // Iroh P2P
        ConnectionKind::Iroh { ticket: addr[7..].to_string() }
    } else {
        // TCP (":PORT" expands to "127.0.0.1:PORT")
        ConnectionKind::Tcp { host, port }
    }
}
```

### Unix Domain Socket (Default)

- Created at `<store_path>/sock`
- Local access only (no network exposure)
- Fastest transport (no TCP overhead)
- Automatically removed on server shutdown

### TCP

- Plain HTTP/1.1 over TCP
- Specified as `host:port` or `:port`
- No encryption — suitable for localhost or trusted networks

### TLS

- HTTPS via rustls + tokio-rustls
- Client uses webpki-roots for certificate validation
- Specified as `https://host:port` in client address

### Iroh (QUIC P2P)

**Protocol details**:
- **ALPN**: `XS/1.0`
- **Handshake**: Connecting side sends 5-byte magic `b"xs..!"`, server validates
- **Secret key**: From `IROH_SECRET` environment variable (persistent identity) or freshly generated
- **Relay mode**: Default (uses iroh's relay infrastructure for NAT traversal)
- **Transport**: QUIC with bidirectional streams, wrapped in `IrohStream`

```mermaid
sequenceDiagram
    participant Client
    participant Relay[Iroh Relay]
    participant Server

    Client->>Relay: Connect via ticket
    Relay->>Server: Forward connection
    Client->>Server: Send magic b"xs..!"
    Server->>Server: Validate magic
    Client->>Server: HTTP/1.1 requests over QUIC stream
```

The Iroh transport enables xs stores to be accessed over the internet without port forwarding, using iroh's relay infrastructure for NAT traversal.

## Client Library

**File**: `src/client/types.rs`

```rust
pub enum ConnectionKind {
    Unix(PathBuf),
    Tcp { host: String, port: u16 },
    Tls { host: String, port: u16 },
    Iroh { ticket: String },
}
```

The client uses hyper to send HTTP requests over the appropriate transport. All transports present the same API — the client code is transport-agnostic above the connection layer.

### Client Authentication

For TLS connections, basic auth can be embedded in the URL:
```
https://user:password@host:port
```

The client extracts credentials and sends them as an `Authorization` header.

## SSE Format Details

When streaming with `Accept: text/event-stream`:

```
id: <scru128_id>
data: <json_frame>

```

- Each event has the frame's SCRU128 ID as the SSE `id` field
- The `data` field contains the full JSON-serialized frame
- Events are separated by double newlines
- Browsers can use `EventSource` with `lastEventId` for automatic reconnection and resume

## Connection Lifecycle

```mermaid
sequenceDiagram
    participant Client
    participant Server
    participant Store

    Client->>Server: Connect (Unix/TCP/TLS/Iroh)
    Client->>Server: GET /?follow&topic=events.*
    Server->>Store: store.read(options)
    Store-->>Server: mpsc::Receiver<Frame>
    loop Streaming
        Server-->>Client: Frame (NDJSON or SSE)
    end
    Note over Client,Server: Client disconnects or server shuts down
    Server->>Server: Drop mpsc receiver (backpressure stops sender)
```

### Broken Pipe Detection (Unix)

**File**: `src/main.rs`

On Unix, the client detects broken pipes (server disconnect) using `AsyncFd`:
```rust
// Register stdout for poll
let fd = AsyncFd::new(std::io::stdout().as_raw_fd())?;
// Wait for EPOLLHUP or EPOLLERR
fd.readable().await?;
// → pipe is broken, exit cleanly
```

This enables clean shutdown when piped to another process (e.g., `xs cat | head -5`).
