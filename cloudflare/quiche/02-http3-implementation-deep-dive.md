---
title: "HTTP/3 Implementation Deep Dive"
subtitle: "HTTP/3 over QUIC, QPACK compression, and request/response handling"
---

# HTTP/3 Implementation Deep Dive

## Introduction

This document provides a comprehensive deep dive into HTTP/3 implementation in quiche. We'll explore how HTTP/3 maps to QUIC streams, QPACK header compression, frame types, and request/response handling.

## Table of Contents

1. [HTTP/3 Overview](#1-http3-overview)
2. [Stream Types](#2-stream-types)
3. [QPACK Compression](#3-qpack-compression)
4. [HTTP/3 Frames](#4-http3-frames)
5. [Request/Response Handling](#5-requestresponse-handling)
6. [Priorities](#6-priorities)
7. [Server Push](#7-server-push)

---

## 1. HTTP/3 Overview

### 1.1 HTTP/3 Architecture

HTTP/3 replaces TCP+TLS with QUIC as the transport:

```
Traditional HTTP Stack:          HTTP/3 Stack:

+-------------------+           +-------------------+
|    Application    |           |    Application    |
+-------------------+           +-------------------+
|       HTTP/2      |           |      HTTP/3       |
+-------------------+           +-------------------+
|       TLS         |           |       QPACK       |
+-------------------+           +-------------------+
|       TCP         |           |       QUIC        |
+-------------------+           +-------------------+
|       IP          |           |        UDP        |
+-------------------+           +-------------------+
```

### 1.2 ALPN Negotiation

HTTP/3 is negotiated via ALPN during TLS handshake:

```rust
// From quiche/src/h3/mod.rs
pub const APPLICATION_PROTOCOL: &[&[u8]] = &[b"h3"];

// Configuration
let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;
config.set_application_protos(quiche::h3::APPLICATION_PROTOCOL)?;
```

### 1.3 HTTP/3 Connection Setup

```rust
// From quiche/src/h3/mod.rs
use quiche::h3::{Connection, Config};

// First establish QUIC connection
let mut quic_conn = quiche::connect(...)?;

// Drive handshake until established
while !quic_conn.is_established() {
    // send/recv packets...
}

// Then create HTTP/3 connection
let h3_config = Config::new()?;
let mut h3_conn = Connection::with_transport(&mut quic_conn, &h3_config)?;

// HTTP/3 connection now ready for requests/responses
```

### 1.4 HTTP/3 Control Flow

```
Client                          Server
   |                              |
   |------ SETTINGS frame ------->| (on control stream)
   |                              |
   |<----- SETTINGS frame --------| (on control stream)
   |                              |
   |------ HEADERS (request) ---->| (on request stream)
   |                              |
   |<----- HEADERS (response) ----|
   |<----- DATA (body) -----------|
   |                              |
   |<----- HEADERS (trailers) ----| (optional)
```

---

## 2. Stream Types

### 2.1 Stream Type Identification

HTTP/3 defines specific stream types:

```rust
// From quiche/src/h3/stream.rs
pub const HTTP3_CONTROL_STREAM_TYPE_ID: u64 = 0x0;
pub const HTTP3_PUSH_STREAM_TYPE_ID: u64 = 0x1;
pub const QPACK_ENCODER_STREAM_TYPE_ID: u64 = 0x2;
pub const QPACK_DECODER_STREAM_TYPE_ID: u64 = 0x3;

pub enum Type {
    Control,       // SETTINGS, GOAWAY, MAX_PUSH_ID
    Request,       // HTTP request/response (bidirectional)
    Push,          // Server push
    QpackEncoder,  // QPACK encoder → decoder
    QpackDecoder,  // QPACK decoder → encoder
    Unknown,       // Unknown stream type (drained)
}
```

### 2.2 Control Stream

Exactly one control stream per direction:

```
Control Stream Frames:
+-------------------+
|    SETTINGS       | (mandatory, first frame)
+-------------------+
|    GOAWAY         | (optional, server-initiated)
+-------------------+
|  MAX_PUSH_ID      | (optional)
+-------------------+
|  PRIORITY_UPDATE  | (optional)
+-------------------+
```

**SETTINGS parameters:**
```rust
// From quiche/src/h3/frame.rs
pub enum Settings {
    QPackMaxTableCapacity = 0x00,      // QPACK dynamic table size
    MaxFieldSectionSize = 0x06,        // Max HEADERS frame size
    QPackBlockedStreams = 0x07,        // Max blocked streams
    ConnectProtocolEnabled = 0x08,     // CONNECT-UDP support
}
```

### 2.3 Request/Response Streams

Bidirectional streams carry HTTP transactions:

```
Request Stream (client-initiated):
+-------------------+
|    HEADERS        | (request headers)
+-------------------+
|      DATA         | (request body, optional)
+-------------------+
|    HEADERS        | (trailers, optional)
+-------------------+

Response Stream (server response on same stream):
+-------------------+
|    HEADERS        | (response headers)
+-------------------+
|      DATA         | (response body)
+-------------------+
|    HEADERS        | (trailers, optional)
+-------------------+
```

### 2.4 QPACK Streams

Unidirectional streams for header compression:

```
QPACK Encoder Stream (client or server):
+-------------------+
| InsertWithNameRef | (add header to dynamic table)
+-------------------+
|  InsertWithoutName| (add header without name ref)
+-------------------+
|    Duplicate      | (duplicate existing entry)
+-------------------+
|   DynamicTableSizeUpdate |
+-------------------+

QPACK Decoder Stream (acknowledgments):
+-------------------+
|  Insert Count   | (acknowledge inserts)
+-------------------+
|  Stream Cancel  | (cancel stream reference)
+-------------------+
```

### 2.5 Stream State Machine

```rust
// From quiche/src/h3/stream.rs
pub enum State {
    /// Reading the stream's type
    StreamType,

    /// Reading frame type
    FrameType,

    /// Reading frame payload length
    FramePayloadLen,

    /// Reading frame payload
    FramePayload,

    /// Reading DATA payload
    Data,

    /// Reading push ID
    PushId,

    /// Reading QPACK instruction
    QpackInstruction,

    /// Draining unknown data
    Drain,

    /// Stream complete
    Finished,
}

impl Stream {
    pub fn new(id: u64, is_local: bool) -> Stream {
        let (ty, state) = if crate::stream::is_bidi(id) {
            // Bidirectional streams are request streams
            (Some(Type::Request), State::FrameType)
        } else {
            // Unidirectional - need to read type first
            (None, State::StreamType)
        };

        Stream {
            id,
            ty,
            state,
            state_buf: vec![0; 16],
            state_len: 1,  // Start with 1 byte varint
            state_off: 0,
            // ...
        }
    }
}
```

---

## 3. QPACK Compression

### 3.1 QPACK vs HPACK

**HPACK (HTTP/2) Problem:**
```
Single dynamic table shared across all streams
Stream ordering matters for compression
HOL blocking: must wait for earlier streams to update table

Stream 1: {"x-custom": "value1"}  → adds to table
Stream 2: {"x-custom": "value2"}  → references table entry
But Stream 2 arrives first → cannot decode!
```

**QPACK (HTTP/3) Solution:**
```
Separate encoder/decoder streams
Encoder sends updates independently
Decoder acknowledges when updates received
Request streams reference table state by ID

No HOL blocking between streams!
```

### 3.2 Static Table

QPACK defines a static table of common headers:

```rust
// From quiche/src/h3/qpack/static_table.rs
pub const STATIC_TABLE: &[(&[u8], &[u8])] = &[
    (b":authority", b""),             // Index 0
    (b":path", b"/"),                  // Index 1
    (b"age", b"0"),                    // Index 2
    (b"content-disposition", b""),     // Index 3
    (b"content-length", b"0"),         // Index 4
    (b"cookie", b""),                  // Index 5
    (b"date", b""),                    // Index 6
    (b"etag", b""),                    // Index 7
    (b"if-modified-since", b""),       // Index 8
    (b"if-none-match", b""),           // Index 9
    (b"last-modified", b""),           // Index 10
    // ... 91 more entries
    (b"accept", b"*/*"),               // Index 31
    (b"accept-language", b""),         // Index 40
    (b"user-agent", b""),              // Index 54
    (b":method", b"GET"),              // Index 65
    (b":scheme", b"https"),            // Index 67
    // ...
];
```

### 3.3 QPACK Encoding

```rust
// From quiche/src/h3/qpack/encoder.rs
pub struct Encoder {
    /// Dynamic table entries
    table: Vec<(Vec<u8>, Vec<u8>)>,
    /// Current table size in bytes
    table_size: usize,
    /// Maximum table size
    max_size: usize,
    /// Insert count
    insert_count: u64,
}

impl Encoder {
    pub fn encode<T: NameValue>(
        &mut self,
        headers: &[T],
        buf: &mut [u8],
    ) -> Result<usize> {
        let mut b = octets::Octets::with_slice(buf);

        for header in headers {
            // Try to find in static table
            if let Some(idx) = self.find_in_static(header) {
                // Indexed from static: 1NNNNNNN
                b.put_varint(INDEXED | (idx as u64))?;
            }
            // Try dynamic table
            else if let Some(idx) = self.find_in_dynamic(header) {
                // Indexed from dynamic: 10NNNNNN
                b.put_varint(INDEXED_WITH_POST_BASE | (idx as u64))?;
            }
            // Literal with name reference
            else if let Some(name_idx) = self.find_name_in_static(header.name()) {
                // 01NNNNNN VVL literal_value
                b.put_varint(LITERAL_WITH_NAME_REF | (name_idx as u64))?;
                encode_str(header.value(), &mut b)?;
            }
            // Literal without name reference
            else {
                // 00NNNNNN VVL name VVL value
                b.put_varint(LITERAL)?;
                encode_str(header.name(), &mut b)?;
                encode_str(header.value(), &mut b)?;
            }
        }

        Ok(b.off())
    }
}
```

### 3.4 QPACK Decoding

```rust
// From quiche/src/h3/qpack/decoder.rs
pub struct Decoder {
    /// Dynamic table entries
    table: Vec<(Vec<u8>, Vec<u8>)>,
    /// Required insert count (blocking)
    required_insert_count: u64,
    /// Base index for post-base encoding
    base: u64,
}

impl Decoder {
    pub fn decode(&mut self, buf: &[u8]) -> Result<Vec<Header>> {
        let mut b = octets::Octets::with_slice(buf);
        let mut headers = Vec::new();

        while b.cap() > 0 {
            let first = b.peek_u8()?;

            if first & 0b1000_0000 != 0 {
                // Indexed: 1NNNNNNN
                let idx = (first & 0b0111_1111) as u64;
                let (name, value) = self.get_from_table(idx)?;
                headers.push(Header::new(name, value));
                b.skip(1)?;
            }
            else if first & 0b0100_0000 != 0 {
                // Literal with name ref: 01NNNNNN
                let name_idx = (first & 0b0011_1111) as u64;
                let name = self.get_name_from_table(name_idx)?;
                let value = decode_str(&mut b)?;
                headers.push(Header::new(name, value));
            }
            // ... more encodings
        }

        Ok(headers)
    }
}
```

### 3.5 QPACK Example

```rust
// Encoding example
let mut enc = qpack::Encoder::new();
let mut encoded = vec![0u8; 1024];

let headers = vec![
    h3::Header::new(b":method", b"GET"),      // Static index 65
    h3::Header::new(b":scheme", b"https"),    // Static index 67
    h3::Header::new(b":authority", b"example.com"),  // Static 0 + literal
    h3::Header::new(b":path", b"/index.html"), // Static 1 + literal
];

let len = enc.encode(&headers, &mut encoded)?;

// Decoding
let mut dec = qpack::Decoder::new();
let decoded = dec.decode(&encoded[..len])?;

assert_eq!(headers, decoded);
```

---

## 4. HTTP/3 Frames

### 4.1 Frame Format

```
HTTP/3 Frame Format (RFC 9114 Section 7.1)
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      Type (i)                 |      Length (i)               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                    Frame Payload (*)                        ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 4.2 Frame Types

```rust
// From quiche/src/h3/frame.rs
pub enum Frame {
    Data {
        payload: Vec<u8>,
    },
    Headers {
        header_block: Vec<u8>,
    },
    Settings {
        settings: Vec<(u64, u64)>,
    },
    CancelPush {
        push_id: u64,
    },
    Goaway {
        stream_id: u64,
    },
    MaxPushId {
        push_id: u64,
    },
    PriorityUpdateRequest {
        prioritized_element_id: u64,
        priority_field_value: Vec<u8>,
    },
    PriorityUpdatePush {
        prioritized_element_id: u64,
        priority_field_value: Vec<u8>,
    },
}

// Wire format constants
pub const DATA_FRAME_TYPE_ID: u64 = 0x00;
pub const HEADERS_FRAME_TYPE_ID: u64 = 0x01;
pub const PRIORITY_UPDATE_FRAME_TYPE_ID_REQUEST: u64 = 0xF0;
pub const PRIORITY_UPDATE_FRAME_TYPE_ID_PUSH: u64 = 0xF1;
pub const SETTINGS_FRAME_TYPE_ID: u64 = 0x04;
pub const GOAWAY_FRAME_TYPE_ID: u64 = 0x07;
```

### 4.3 Frame Parsing

```rust
// From quiche/src/h3/frame.rs
impl Frame {
    pub fn from_bytes(
        b: &mut octets::Octets,
    ) -> Result<(Frame, usize)> {
        let frame_type = b.get_varint()?;
        let frame_payload_len = b.get_varint()? as usize;

        let frame = match frame_type {
            DATA_FRAME_TYPE_ID => {
                Frame::Data {
                    payload: b.get_bytes(frame_payload_len)?.to_vec(),
                }
            }
            HEADERS_FRAME_TYPE_ID => {
                Frame::Headers {
                    header_block: b.get_bytes(frame_payload_len)?.to_vec(),
                }
            }
            SETTINGS_FRAME_TYPE_ID => {
                let mut settings = Vec::new();
                let mut remaining = frame_payload_len;

                while remaining >= 2 {
                    let id = b.get_varint()?;
                    let value = b.get_varint()?;
                    settings.push((id, value));
                    remaining -= 2;  // Simplified
                }
                Frame::Settings { settings }
            }
            GOAWAY_FRAME_TYPE_ID => {
                let stream_id = b.get_varint()?;
                Frame::Goaway { stream_id }
            }
            // ... more frame types
            _ => {
                // Unknown frame - skip payload
                b.skip(frame_payload_len)?;
                return Err(Error::FrameUnexpected);
            }
        };

        Ok((frame, frame_payload_len))
    }
}
```

### 4.4 Frame Writing

```rust
// From quiche/src/h3/frame.rs
impl Frame {
    pub fn to_bytes(&self, b: &mut octets::Octets) -> Result<usize> {
        match self {
            Frame::Data { payload } => {
                let off = b.off();
                b.put_varint(DATA_FRAME_TYPE_ID)?;
                b.put_varint(payload.len() as u64)?;
                b.put_slice(payload)?;
                Ok(b.off() - off)
            }
            Frame::Headers { header_block } => {
                let off = b.off();
                b.put_varint(HEADERS_FRAME_TYPE_ID)?;
                b.put_varint(header_block.len() as u64)?;
                b.put_slice(header_block)?;
                Ok(b.off() - off)
            }
            Frame::Settings { settings } => {
                let off = b.off();
                b.put_varint(SETTINGS_FRAME_TYPE_ID)?;

                // Calculate payload length
                let mut len = 0;
                for (id, value) in settings {
                    len += varint_len(*id) + varint_len(*value);
                }
                b.put_varint(len as u64)?;

                for (id, value) in settings {
                    b.put_varint(*id)?;
                    b.put_varint(*value)?;
                }
                Ok(b.off() - off)
            }
            // ... more frame types
        }
    }
}
```

---

## 5. Request/Response Handling

### 5.1 Sending Requests

```rust
// From quiche/src/h3/mod.rs
impl Connection {
    pub fn send_request<T: NameValue>(
        &mut self,
        conn: &mut super::Connection,
        headers: &[T],
        fin: bool,
    ) -> Result<u64> {
        // Create new bidirectional stream
        let stream_id = conn.streams.next_request_stream_id()?;

        // Encode headers with QPACK
        let mut header_block = vec![0; 4096];
        let len = self.qpack_encoder.encode(headers, &mut header_block)?;

        // Send HEADERS frame
        let frame = Frame::Headers {
            header_block: header_block[..len].to_vec(),
        };
        self.send_frame(conn, stream_id, &frame)?;

        // Mark stream as finished if no body
        if fin {
            conn.stream_shutdown(stream_id, Shutdown::Write, 0)?;
        }

        Ok(stream_id)
    }
}

// Usage example
let headers = vec![
    h3::Header::new(b":method", b"GET"),
    h3::Header::new(b":scheme", b"https"),
    h3::Header::new(b":authority", b"quic.tech"),
    h3::Header::new(b":path", b"/"),
    h3::Header::new(b"user-agent", b"quiche"),
];

let stream_id = h3_conn.send_request(&mut quic_conn, &headers, true)?;
```

### 5.2 Sending Responses

```rust
impl Connection {
    pub fn send_response<T: NameValue>(
        &mut self,
        conn: &mut super::Connection,
        stream_id: u64,
        headers: &[T],
        fin: bool,
    ) -> Result<()> {
        // Encode headers
        let mut header_block = vec![0; 4096];
        let len = self.qpack_encoder.encode(headers, &mut header_block)?;

        // Send HEADERS frame
        let frame = Frame::Headers {
            header_block: header_block[..len].to_vec(),
        };
        self.send_frame(conn, stream_id, &frame)?;

        if fin {
            conn.stream_shutdown(stream_id, Shutdown::Write, 0)?;
        }

        Ok(())
    }

    pub fn send_body(
        &mut self,
        conn: &mut super::Connection,
        stream_id: u64,
        body: &[u8],
        fin: bool,
    ) -> Result<usize> {
        // Send DATA frame
        let frame = Frame::Data {
            payload: body.to_vec(),
        };
        self.send_frame(conn, stream_id, &frame)?;

        if fin {
            conn.stream_shutdown(stream_id, Shutdown::Write, 0)?;
        }

        Ok(body.len())
    }
}

// Usage example - sending response
let response_headers = vec![
    h3::Header::new(b":status", b"200"),
    h3::Header::new(b"server", b"quiche"),
    h3::Header::new(b"content-type", b"text/html"),
];

h3_conn.send_response(&mut quic_conn, stream_id, &response_headers, false)?;
h3_conn.send_body(&mut quic_conn, stream_id, b"Hello World!", true)?;
```

### 5.3 Polling for Events

```rust
// From quiche/src/h3/mod.rs
pub enum Event {
    /// Received headers
    Headers {
        list: Vec<Header>,
        more_frames: bool,
    },
    /// Received data
    Data,
    /// Stream finished
    Finished,
    /// Stream reset
    Reset(u64),
    /// Priority update received
    PriorityUpdate,
    /// GOAWAY received
    GoAway,
}

impl Connection {
    pub fn poll(&mut self, conn: &mut super::Connection) -> Result<(u64, Event)> {
        // Check for completed streams
        if let Some(stream_id) = self.completed_streams.pop_front() {
            return Ok((stream_id, Event::Finished));
        }

        // Check readable streams
        for stream_id in conn.readable() {
            match self.process_stream_frame(conn, stream_id)? {
                Some(event) => return Ok((stream_id, event)),
                None => continue,
            }
        }

        Err(Error::Done)
    }
}

// Usage example - server loop
loop {
    match h3_conn.poll(&mut quic_conn) {
        Ok((stream_id, Event::Headers { list, .. })) => {
            // Process request headers
            let method = list.iter().find(|h| h.name() == b":method").unwrap();
            let path = list.iter().find(|h| h.name() == b":path").unwrap();

            if method.value() == b"GET" && path.value() == b"/" {
                let resp = vec![
                    h3::Header::new(b":status", b"200"),
                    h3::Header::new(b"server", b"quiche"),
                ];
                h3_conn.send_response(&mut quic_conn, stream_id, &resp, false)?;
                h3_conn.send_body(&mut quic_conn, stream_id, b"Hello!", true)?;
            }
        }
        Ok((stream_id, Event::Data)) => {
            // Read body data
            let mut buf = vec![0; 4096];
            while let Ok(read) = h3_conn.recv_body(&mut quic_conn, stream_id, &mut buf) {
                println!("Received {} bytes", read);
            }
        }
        Ok((stream_id, Event::Finished)) => {
            println!("Stream {} finished", stream_id);
        }
        Err(Error::Done) => break,
        Err(e) => {
            eprintln!("HTTP/3 error: {:?}", e);
            break;
        }
    }
}
```

### 5.4 Error Handling

```rust
// From quiche/src/h3/mod.rs
pub enum Error {
    Done,
    BufferTooShort,
    InternalError,
    ExcessiveLoad,
    IdError,
    StreamCreationError,
    ClosedCriticalStream,
    MissingSettings,
    FrameUnexpected,
    FrameError,
    QpackDecompressionFailed,
    TransportError(crate::Error),
    StreamBlocked,
    SettingsError,
    RequestRejected,
    RequestCancelled,
    RequestIncomplete,
    MessageError,
    ConnectError,
    VersionFallback,
}

impl Connection {
    /// Close connection with error
    pub fn connection_close(
        &mut self,
        conn: &mut super::Connection,
        err: Error,
    ) -> Result<()> {
        let frame = Frame::Goaway {
            stream_id: 0,
        };

        // Send on control stream
        self.send_frame(conn, self.control_stream_id?, &frame)?;

        // Close QUIC connection
        conn.close(true, err.to_wire(), b"HTTP/3 error")?;

        Ok(())
    }
}
```

---

## 6. Priorities

### 6.1 Extensible Priorities (RFC 9218)

HTTP/3 uses extensible priorities instead of HTTP/2's dependency tree:

```rust
// From quiche/src/h3/mod.rs
pub struct Priority {
    /// Urgency: 0 (highest) to 7 (lowest)
    urgency: u8,
    /// Incremental: can be deprioritized
    incremental: bool,
}

// Default values
const PRIORITY_URGENCY_DEFAULT: u8 = 3;
const PRIORITY_INCREMENTAL_DEFAULT: bool = false;
```

### 6.2 Priority Header Field

```
Priority: urgency=N, incremental
Example:
Priority: urgency=1           (high priority, not incremental)
Priority: urgency=5, incremental (low priority, incremental)
```

### 6.3 PRIORITY_UPDATE Frame

```rust
impl Connection {
    pub fn send_priority_update(
        &mut self,
        conn: &mut super::Connection,
        stream_id: u64,
        priority: &Priority,
    ) -> Result<()> {
        let priority_field_value = format!(
            "u={}, {}",
            priority.urgency,
            if priority.incremental { "i" } else { "" }
        );

        let frame = Frame::PriorityUpdateRequest {
            prioritized_element_id: stream_id,
            priority_field_value: priority_field_value.into_bytes(),
        };

        self.send_frame(conn, self.control_stream_id?, &frame)?;
        Ok(())
    }
}
```

### 6.4 Priority Scheduling

```rust
// From quiche/src/stream/mod.rs
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StreamPriorityKey {
    /// Urgency level (0-7)
    pub urgency: u8,
    /// Whether stream is incremental
    pub incremental: bool,
    /// Stream ID for tie-breaking
    pub stream_id: u64,
}

// Lower urgency = higher priority
// Incremental streams are deprioritized within same urgency
impl PartialOrd for StreamPriorityKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StreamPriorityKey {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare urgency first (lower = higher priority)
        self.urgency.cmp(&other.urgency)
            // Then incremental (non-incremental first)
            .then(self.incremental.cmp(&other.incremental).reverse())
            // Finally stream ID for determinism
            .then(self.stream_id.cmp(&other.stream_id))
    }
}
```

---

## 7. Server Push

### 7.1 Push Stream Setup

```
Server-initiated push:
1. Server sends PUSH_PROMISE on request stream
2. Server creates push stream with push ID
3. Server sends response on push stream
4. Client can accept or cancel push
```

### 7.2 Push Implementation

```rust
// From quiche/src/h3/mod.rs
impl Connection {
    pub fn send_push_promise(
        &mut self,
        conn: &mut super::Connection,
        request_stream_id: u64,
        push_id: u64,
        headers: &[Header],
    ) -> Result<()> {
        // Encode headers
        let mut header_block = vec![0; 4096];
        let len = self.qpack_encoder.encode(headers, &mut header_block)?;

        // Send on request stream (bidirectional)
        let frame = Frame::Headers {
            header_block: header_block[..len].to_vec(),
        };
        self.send_frame(conn, request_stream_id, &frame)?;

        // Create push stream
        let push_stream_id = self.next_push_stream_id(push_id)?;

        Ok(())
    }

    pub fn cancel_push(
        &mut self,
        conn: &mut super::Connection,
        push_id: u64,
    ) -> Result<()> {
        let frame = Frame::CancelPush { push_id };
        self.send_frame(conn, self.control_stream_id?, &frame)?;
        Ok(())
    }
}
```

---

## Summary

### Key Takeaways

1. **Stream types** - Control, Request, Push, QPACK encoder/decoder
2. **QPACK** - Separate encoder/decoder streams eliminate HOL blocking
3. **Frame types** - HEADERS, DATA, SETTINGS, GOAWAY, PRIORITY_UPDATE
4. **Request/Response** - send_request(), send_response(), poll()
5. **Priorities** - Extensible priorities with urgency and incremental
6. **Server Push** - PUSH_PROMISE and push streams

### Next Steps

Continue to [03-tls-integration-deep-dive.md](03-tls-integration-deep-dive.md) for:
- TLS 1.3 handshake details
- Key derivation and rotation
- 0-RTT early data
- BoringSSL integration

---

## Further Reading

- [RFC 9114 - HTTP/3](https://www.rfc-editor.org/rfc/rfc9114.html)
- [RFC 9204 - QPACK](https://www.rfc-editor.org/rfc/rfc9204.html)
- [RFC 9218 - Extensible Priorities](https://www.rfc-editor.org/rfc/rfc9218.html)
- [quiche source - h3/mod.rs](quiche/src/h3/mod.rs)
- [quiche source - h3/qpack/](quiche/src/h3/qpack/)
