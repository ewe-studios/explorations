# Tungstenite-rs Implementation Analysis

## Overview

`tungstenite-rs` is a lightweight, stream-based WebSocket implementation for Rust. It serves as the foundation for `tokio-tungstenite` (async WebSocket) and `websocat` (CLI tool).

**Key Characteristics:**
- Stream-based API (works with any `Read + Write` stream)
- Both synchronous and asynchronous usage patterns
- Complete RFC 6455 implementation
- Minimal dependencies
- TLS support via `native-tls` or `rustls`

## Architecture

```
tungstenite-rs/
├── src/
│   ├── lib.rs              # Core exports
│   ├── protocol/
│   │   ├── mod.rs          # WebSocket state machine
│   │   ├── message.rs      # Message types
│   │   └── frame/
│   │       ├── mod.rs      # Frame codec
│   │       ├── frame.rs    # Frame structure
│   │       ├── coding.rs   # Opcodes and close codes
│   │       └── mask.rs     # Masking implementation
│   ├── handshake/
│   │   ├── mod.rs          # Handshake state machine
│   │   ├── client.rs       # Client handshake
│   │   ├── server.rs       # Server handshake
│   │   ├── headers.rs      # HTTP headers
│   │   └── machine.rs      # Handshake machine
│   ├── client.rs           # Client API
│   ├── server.rs           # Server API
│   ├── error.rs            # Error types
│   ├── stream.rs           # Stream traits
│   ├── buffer.rs           # Read buffer
│   └── tls.rs              # TLS support
```

## Core Components

### WebSocket Struct

The main `WebSocket<Stream>` struct manages the protocol state:

```rust
/// WebSocket input-output stream.
pub struct WebSocket<Stream> {
    socket: Stream,                    // Underlying transport
    context: WebSocketContext,         // Protocol state
}

pub struct WebSocketContext {
    role: Role,                        // Client or Server
    frame: FrameCodec,                 // Frame encoder/decoder
    state: WebSocketState,             // Connection state
    incomplete: Option<IncompleteMessage>, // Fragmented message
    additional_send: Option<Frame>,    // Queued control frames
    unflushed_additional: bool,        // Flush state
    config: WebSocketConfig,           // Configuration
}
```

### Message Handling

Messages are the primary API for application data:

```rust
pub enum Message {
    Text(String),      // UTF-8 text
    Binary(Vec<u8>),   // Binary data
    Ping(Vec<u8>),     // Ping control frame
    Pong(Vec<u8>),     // Pong control frame
    Close(Option<CloseFrame<'static>>), // Close frame
    Frame(Frame),      // Raw frame access
}
```

### Frame Codec

The frame codec handles encoding/decoding WebSocket frames:

```rust
// Frame parsing flow
pub fn parse(cursor: &mut Cursor<impl AsRef<[u8]>>) -> Result<Option<(FrameHeader, u64)>> {
    let (first, second) = read_header_bytes()?;

    // Parse header bits
    let is_final = first & 0x80 != 0;       // FIN bit
    let rsv1 = first & 0x40 != 0;           // RSV1
    let rsv2 = first & 0x20 != 0;           // RSV2
    let rsv3 = first & 0x10 != 0;           // RSV3
    let opcode = OpCode::from(first & 0x0F); // Opcode

    let masked = second & 0x80 != 0;        // MASK bit
    let length = parse_payload_length(second & 0x7F)?;
    let mask = if masked { Some(read_mask()?) } else { None };

    Ok(Some((FrameHeader { is_final, rsv1, rsv2, rsv3, opcode, mask }, length)))
}
```

## Handshake Implementation

### Client Handshake

```rust
pub fn start(
    stream: S,
    request: Request,
    config: Option<WebSocketConfig>,
) -> Result<MidHandshake<Self>> {
    // Validate request
    if request.method() != http::Method::GET {
        return Err(Error::Protocol(ProtocolError::WrongHttpMethod));
    }

    // Extract and validate subprotocols
    let subprotocols = extract_subprotocols_from_request(&request)?;

    // Generate formatted request with key
    let (request, key) = generate_request(request)?;

    // Start handshake machine
    let machine = HandshakeMachine::start_write(stream, request);

    Ok(MidHandshake {
        role: ClientHandshake {
            verify_data: VerifyData {
                accept_key: derive_accept_key(key.as_ref()),
                subprotocols,
            },
            config,
            _marker: PhantomData,
        },
        machine,
    })
}
```

### Server Handshake

```rust
pub fn accept_hdr<S, C>(
    stream: S,
    callback: C,
) -> Result<(WebSocket<S>, Response)>
where
    C: Callback,
{
    // Parse incoming HTTP request
    let request = parse_request(&stream)?;

    // Validate WebSocket headers
    validate_websocket_request(&request)?;

    // Generate response with accept key
    let response = generate_response(&request, &callback)?;

    // Complete handshake
    Ok((WebSocket::from_partially_read(stream, tail, Role::Server, config), response))
}
```

## Error Handling

Tungstenite provides comprehensive error types:

```rust
pub enum Error {
    ConnectionClosed,           // Normal closure
    AlreadyClosed,              // Using closed connection
    Io(io::Error),              // I/O errors
    Tls(TlsError),              // TLS errors
    Capacity(CapacityError),    // Buffer/size limits
    Protocol(ProtocolError),    // Protocol violations
    WriteBufferFull(Message),   // Backpressure signal
    Utf8,                       // UTF-8 errors
    AttackAttempt,              // Security violations
    Url(UrlError),              // URL errors
    Http(Response),             // HTTP errors
}
```

### Protocol Errors

```rust
pub enum ProtocolError {
    WrongHttpMethod,
    WrongHttpVersion,
    MissingConnectionUpgradeHeader,
    MissingUpgradeWebSocketHeader,
    MissingSecWebSocketVersionHeader,
    MissingSecWebSocketKey,
    SecWebSocketAcceptKeyMismatch,
    NonZeroReservedBits,
    UnmaskedFrameFromClient,
    MaskedFrameFromServer,
    FragmentedControlFrame,
    ControlFrameTooBig,
    InvalidOpcode(u8),
    InvalidCloseSequence,
    // ... and more
}
```

## Configuration

```rust
pub struct WebSocketConfig {
    /// Target minimum write buffer size before flushing
    pub write_buffer_size: usize,        // Default: 128 KiB

    /// Maximum write buffer size for backpressure
    pub max_write_buffer_size: usize,    // Default: unlimited

    /// Maximum incoming message size
    pub max_message_size: Option<usize>, // Default: 64 MiB

    /// Maximum single frame size
    pub max_frame_size: Option<usize>,   // Default: 16 MiB

    /// Accept unmasked client frames (non-standard)
    pub accept_unmasked_frames: bool,    // Default: false
}
```

## State Machine

```rust
enum WebSocketState {
    Active,              // Normal operation
    ClosedByUs,          // We sent close frame
    ClosedByPeer,        // Peer sent close frame
    CloseAcknowledged,   // Close handshake complete
    Terminated,          // Connection dead
}
```

### State Transitions

```
Active ──[send close]──► ClosedByUs
  │                         │
  │                         │ [receive close]
  │                         ▼
  │                   CloseAcknowledged
  │                         │
  │ [receive close]         │ [flush complete]
  ▼                         ▼
ClosedByPeer ──[send close]──► Terminated
```

## Read/Write Flow

### Reading Messages

```rust
pub fn read(&mut self) -> Result<Message> {
    self.context.read(&mut self.socket)
}

// Internal read loop
fn read<Stream>(&mut self, stream: &mut Stream) -> Result<Message> {
    loop {
        // Flush any queued responses (pong/close)
        if self.additional_send.is_some() {
            self.flush(stream)?;
        }

        // Read next frame
        if let Some(message) = self.read_message_frame(stream)? {
            return Ok(message);
        }
    }
}

fn read_message_frame<Stream>(&mut self, stream: &mut Stream) -> Result<Option<Message>> {
    // Parse frame header and payload
    let frame = self.frame.read_frame(stream, self.config.max_frame_size)?;

    // Validate reserved bits
    if frame.header().rsv1 || frame.header().rsv2 || frame.header().rsv3 {
        return Err(Error::Protocol(ProtocolError::NonZeroReservedBits));
    }

    // Handle masking based on role
    match self.role {
        Role::Server => {
            if frame.is_masked() {
                frame.apply_mask();  // Unmask client frames
            }
        }
        Role::Client => {
            if frame.is_masked() {
                return Err(Error::Protocol(ProtocolError::MaskedFrameFromServer));
            }
        }
    }

    // Dispatch based on opcode
    match frame.header().opcode {
        OpCode::Control(ctl) => self.handle_control_frame(frame, ctl),
        OpCode::Data(data) => self.handle_data_frame(frame, data),
    }
}
```

### Writing Messages

```rust
pub fn send(&mut self, message: Message) -> Result<()> {
    self.write(message)?;
    self.flush()
}

pub fn write(&mut self, message: Message) -> Result<()> {
    self.context.write(&mut self.socket, message)
}

fn write<Stream>(&mut self, stream: &mut Stream, message: Message) -> Result<()> {
    let frame = match message {
        Message::Text(data) => Frame::message(data.into(), OpCode::Data(OpData::Text), true),
        Message::Binary(data) => Frame::message(data, OpCode::Data(OpData::Binary), true),
        Message::Ping(data) => Frame::ping(data),
        Message::Pong(data) => {
            self.set_additional(Frame::pong(data));
            return Ok(());
        }
        Message::Close(code) => return self.close(stream, code),
        Message::Frame(f) => f,
    };

    self._write(stream, Some(frame))?;
    Ok(())
}
```

## Example Usage

### Echo Server

```rust
use std::net::TcpListener;
use std::thread::spawn;
use tungstenite::accept;

fn main() {
    let server = TcpListener::bind("127.0.0.1:9001").unwrap();
    for stream in server.incoming() {
        spawn(move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                let msg = websocket.read().unwrap();
                if msg.is_binary() || msg.is_text() {
                    websocket.send(msg).unwrap();
                }
            }
        });
    }
}
```

### Custom Server with Headers

```rust
use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
};

let callback = |req: &Request, mut response: Response| {
    println!("Received request for path: {}", req.uri().path());

    // Add custom headers to response
    let headers = response.headers_mut();
    headers.append("MyCustomHeader", ":)".parse().unwrap());

    Ok(response)
};

let mut websocket = accept_hdr(stream, callback).unwrap();
```

### Client Connection

```rust
use tungstenite::{connect, Message};

let (mut socket, response) = connect("ws://localhost:9001/socket").unwrap();

// Send message
socket.send(Message::Text("Hello WebSocket!".into())).unwrap();

// Receive messages
loop {
    match socket.read().unwrap() {
        Message::Text(text) => println!("Received: {}", text),
        Message::Close(_) => break,
        _ => {}
    }
}
```

## Performance Considerations

### Buffer Management

- Uses chunked read buffers (4KB default)
- Write buffering for batching small writes
- Configurable buffer sizes for memory control

### Masking Optimization

```rust
// Fast 32-bit aligned masking
pub fn apply_mask_fast32(buf: &mut [u8], mask: [u8; 4]) {
    let mask_u32 = u32::from_ne_bytes(mask);
    let (prefix, words, suffix) = unsafe { buf.align_to_mut::<u32>() };

    // Handle unaligned prefix
    apply_mask_fallback(prefix, mask);

    // Process 4 bytes at a time
    for word in words.iter_mut() {
        *word ^= mask_u32;
    }

    // Handle unaligned suffix
    apply_mask_fallback(suffix, mask_u32.to_ne_bytes());
}
```

## Limitations

1. **No permessage-deflate** - Compression extension not implemented
2. **Single-threaded** - One connection per WebSocket instance
3. **No connection pooling** - Application-level responsibility
4. **Max message limits** - Default 64MB (configurable)

## Dependencies

```toml
[dependencies]
tungstenite = "0.21"

# For TLS support
tungstenite = { version = "0.21", features = ["native-tls"] }
# or
tungstenite = { version = "0.21", features = ["rustls-tls-native-roots"] }
```

## Related Crates

- **tokio-tungstenite** - Async/await support with Tokio
- **websocat** - CLI WebSocket client/server
- **warp** - Web framework with WebSocket support
- **actix-web** - Web framework with WebSocket actors
