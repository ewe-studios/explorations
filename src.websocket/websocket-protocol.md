# WebSocket Protocol Deep Dive (RFC 6455)

## Overview

WebSocket is a bidirectional, full-duplex communication protocol designed for real-time web applications. Unlike HTTP's request-response model, WebSocket provides persistent connections for efficient server-to-client and client-to-server communication.

## Protocol Specification (RFC 6455)

### Key Characteristics

- **Full-duplex communication**: Both parties can send data independently
- **Low overhead**: Minimal framing overhead (2-14 bytes per message)
- **Persistent connection**: Single TCP connection for the session lifetime
- **Origin-based security**: Browser enforces same-origin policy
- **Subprotocol support**: Application-level protocol negotiation

## WebSocket Handshake

The handshake is an HTTP Upgrade request that establishes the WebSocket connection.

### Client Request

```http
GET /chat HTTP/1.1
Host: example.com:8000
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
Sec-WebSocket-Protocol: chat, superchat
Origin: http://example.com:8000
```

**Required Headers:**

| Header | Purpose |
|--------|---------|
| `Upgrade: websocket` | Indicates protocol upgrade |
| `Connection: Upgrade` | Requests connection upgrade |
| `Sec-WebSocket-Key` | Random 16-byte base64 value |
| `Sec-WebSocket-Version: 13` | Protocol version |

### Server Response

```http
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
Sec-WebSocket-Protocol: chat
```

**Key Validation:**

The server computes `Sec-WebSocket-Accept` by:
1. Concatenating the client's key with the GUID `258EAFA5-E914-47DA-95CA-C5AB0DC85B11`
2. Computing SHA-1 hash
3. Base64 encoding the result

```rust
// From tungstenite-rs implementation
pub fn derive_accept_key(request_key: &[u8]) -> String {
    const WS_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut sha1 = Sha1::default();
    sha1.update(request_key);
    sha1.update(WS_GUID);
    data_encoding::BASE64.encode(&sha1.finalize())
}
```

## WebSocket Frame Format

The frame format is defined in RFC 6455 Section 5.2:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-------+-+-------------+-------------------------------+
|F|R|R|R| opcode|M| Payload len |    Extended payload length    |
|I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
|N|V|V|V|       |S|             |   (if payload len==126/127)   |
| |1|2|3|       |K|             |                               |
+-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
|     Extended payload length continued, if payload len == 127  |
+ - - - - - - - - - - - - - - - +-------------------------------+
|                               |Masking-key, if MASK set to 1  |
+-------------------------------+-------------------------------+
| Masking-key (continued)       |          Payload Data         |
+-------------------------------- - - - - - - - - - - - - - - - +
:                     Payload Data continued ...                :
+ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
|                     Payload Data continued ...                |
+---------------------------------------------------------------+
```

### Frame Header Fields

| Field | Size | Description |
|-------|------|-------------|
| FIN | 1 bit | Final fragment indicator |
| RSV1-3 | 3 bits | Reserved for extensions |
| Opcode | 4 bits | Frame type |
| MASK | 1 bit | Payload masking indicator |
| Payload Length | 7/15/71 bits | Data length |
| Masking Key | 32 bits | Mask (if MASK=1) |

### Opcodes

**Data Frames:**
- `0x0` - Continuation frame
- `0x1` - Text frame (UTF-8 encoded)
- `0x2` - Binary frame

**Control Frames:**
- `0x8` - Connection close
- `0x9` - Ping
- `0xA` - Pong
- `0xB-F` - Reserved

### Payload Length Encoding

```rust
// From tungstenite-rs: protocol/frame/frame.rs
enum LengthFormat {
    U8(u8),   // 0-125 bytes
    U16,      // 126-65535 bytes (2 extension bytes)
    U64,      // 65536+ bytes (8 extension bytes)
}

impl LengthFormat {
    fn for_length(length: u64) -> Self {
        if length < 126 {
            LengthFormat::U8(length as u8)
        } else if length < 65536 {
            LengthFormat::U16
        } else {
            LengthFormat::U64
        }
    }
}
```

### Masking

**Client-to-Server frames MUST be masked.** This prevents intermediaries from interpreting WebSocket frames as HTTP.

```rust
// Fast 32-bit masking from tungstenite-rs
pub fn apply_mask_fast32(buf: &mut [u8], mask: [u8; 4]) {
    let mask_u32 = u32::from_ne_bytes(mask);
    let (prefix, words, suffix) = unsafe { buf.align_to_mut::<u32>() };
    apply_mask_fallback(prefix, mask);

    // Rotate mask for alignment
    let head = prefix.len() & 3;
    let mask_u32 = if head > 0 {
        if cfg!(target_endian = "big") {
            mask_u32.rotate_left(8 * head as u32)
        } else {
            mask_u32.rotate_right(8 * head as u32)
        }
    } else {
        mask_u32
    };

    for word in words.iter_mut() {
        *word ^= mask_u32;
    }
    apply_mask_fallback(suffix, mask_u32.to_ne_bytes());
}
```

## Control Frames

### Ping/Pong (Heartbeat)

Ping frames test connection liveness. The receiver MUST respond with a Pong frame containing the same payload.

```rust
// Creating Ping/Pong frames
pub fn ping(data: Vec<u8>) -> Frame {
    Frame {
        header: FrameHeader {
            opcode: OpCode::Control(Control::Ping),
            ..FrameHeader::default()
        },
        payload: data,
    }
}

pub fn pong(data: Vec<u8>) -> Frame {
    Frame {
        header: FrameHeader {
            opcode: OpCode::Control(Control::Pong),
            ..FrameHeader::default()
        },
        payload: data,
    }
}
```

### Close Frame

Close frames gracefully terminate connections with an optional status code and reason.

```rust
pub fn close(msg: Option<CloseFrame>) -> Frame {
    let payload = if let Some(CloseFrame { code, reason }) = msg {
        let mut p = Vec::with_capacity(reason.as_bytes().len() + 2);
        p.extend(u16::from(code).to_be_bytes()); // Status code (2 bytes)
        p.extend_from_slice(reason.as_bytes());  // Reason text
        p
    } else {
        Vec::new() // No status code = normal closure
    };
    Frame { header: FrameHeader::default(), payload }
}
```

### Status Codes

| Code | Name | Meaning |
|------|------|---------|
| 1000 | Normal | Normal closure |
| 1001 | Away | Endpoint going away |
| 1002 | Protocol | Protocol error |
| 1003 | Unsupported | Unsupported data type |
| 1005 | Status | No status received |
| 1006 | Abnormal | Abnormal closure |
| 1007 | Invalid | Invalid frame payload |
| 1008 | Policy | Policy violation |
| 1009 | Size | Message too big |
| 1010 | Extension | Missing extension |
| 1011 | Error | Server error |
| 1012 | Restart | Server restarting |
| 1013 | Again | Try again later |
| 1015 | TLS | TLS handshake failure |

## Message Fragmentation

Large messages can be split across multiple frames:

1. **Initial frame**: FIN=0, Opcode=Text/Binary
2. **Continuation frames**: FIN=0, Opcode=Continue
3. **Final frame**: FIN=1, Opcode=Continue

```rust
// Message reassembly in tungstenite-rs
match data {
    OpData::Continue => {
        if let Some(ref mut msg) = self.incomplete {
            msg.extend(frame.into_data(), self.config.max_message_size)?;
        } else {
            return Err(Error::Protocol(ProtocolError::UnexpectedContinueFrame));
        }
        if fin {
            Ok(Some(self.incomplete.take().unwrap().complete()?))
        } else {
            Ok(None) // Still waiting for more fragments
        }
    }
    // ...
}
```

## Protocol State Machine

```
┌─────────────┐
│   Active    │ ◄── Initial state
└──────┬──────┘
       │
       │ Send/Receive Close
       ▼
┌─────────────┐
│ ClosedByUs  │ ◄── We sent close
└──────┬──────┘
       │
       │ Receive Close
       ▼
┌─────────────┐
│    Close    │ ◄── Close acknowledged
│Acknowledged │
└──────┬──────┘
       │
       │ Server: flush remaining data
       ▼
┌─────────────┐
│ Terminated  │ ◄── Connection closed
└─────────────┘
```

## Security Considerations

### Origin Checking

Servers MUST validate the `Origin` header to prevent cross-origin attacks.

### Masking Requirement

Client-to-server frames MUST be masked to prevent:
- Cache poisoning by malicious intermediaries
- Interpreting WebSocket frames as HTTP requests

### Rate Limiting

Implementations should:
- Limit connection rates
- Set maximum message sizes
- Implement connection timeouts

## Error Handling

Tungstenite's comprehensive error types:

```rust
pub enum Error {
    ConnectionClosed,           // Normal closure
    AlreadyClosed,              // Using closed connection
    Io(io::Error),              // I/O errors
    Tls(TlsError),              // TLS errors
    Capacity(CapacityError),    // Buffer/size limits
    Protocol(ProtocolError),    // Protocol violations
    WriteBufferFull(Message),   // Backpressure
    Utf8,                       // UTF-8 encoding errors
    AttackAttempt,              // Security violation
    Url(UrlError),              // URL errors
    Http(Response),             // HTTP errors
}
```

## Best Practices

1. **Always handle ping/pong** - Implement heartbeat detection
2. **Validate message sizes** - Prevent memory exhaustion
3. **Handle fragmented messages** - Reassemble properly
4. **Implement backpressure** - Use write buffers
5. **Graceful shutdown** - Send/receive close frames
6. **Validate UTF-8** - Text frames must be valid UTF-8
7. **Mask client frames** - Required by the protocol

## References

- [RFC 6455 - The WebSocket Protocol](https://tools.ietf.org/html/rfc6455)
- [RFC 7692 - Compression Extensions](https://tools.ietf.org/html/rfc7692)
- [MDN WebSocket API](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
- [tungstenite-rs GitHub](https://github.com/snapview/tungstenite-rs)
