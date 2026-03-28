---
title: "QUIC Protocol Deep Dive"
subtitle: "Streams, frames, connections, and flow control explained"
---

# QUIC Protocol Deep Dive

## Introduction

This document provides a comprehensive deep dive into the QUIC protocol implementation in quiche. We'll explore packet structure, frame types, connection management, stream multiplexing, and flow control.

## Table of Contents

1. [Packet Structure](#1-packet-structure)
2. [Connection IDs](#2-connection-ids)
3. [Frame Types](#3-frame-types)
4. [Stream Multiplexing](#4-stream-multiplexing)
5. [Flow Control](#5-flow-control)
6. [ACK Mechanism](#6-ack-mechanism)
7. [Connection Migration](#7-connection-migration)

---

## 1. Packet Structure

### 1.1 Long Header Packets

Used during handshake and for 0-RTT:

```
Long Header Packet (RFC 9000 Section 17.2)
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|1|  Long Type (2) | Reserved (2)| Packet Number Len (2)| Vers |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Destination CID Len (4) | Destination Connection ID (0..160)  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Source CID Len (4)| Source Connection ID (0..160) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | Length (i) | Packet Number (0..28) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | Payload (*) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Long Header Types:**
- `0xC0` - Initial (ClientHello, ServerHello)
- `0xC1` - 0-RTT (early data)
- `0xC2` - Handshake (crypto frames)
- `0xC3` - Retry (server requests address validation)

### 1.2 Short Header Packets

Used after handshake completion (1-RTT):

```
Short Header Packet (RFC 9000 Section 17.3)
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|0|1|  Reserved (2) | Spin (1)|  Packet Number Len (2)|
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Destination Connection ID (0..160) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | Packet Number (8..32) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | Payload (*) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Key differences:**
- No version field (negotiated during handshake)
- No source CID (only destination needed)
- Fixed bit = 1 indicates 1-RTT packet
- Spin bit for passive RTT measurement

### 1.3 Packet Number Encoding

QUIC uses variable-length packet numbers (1-4 bytes):

```rust
// From quiche/src/packet.rs
pub const MAX_PKT_NUM_LEN: usize = 4;

// Packet number is XORed with expected value
// This helps with middlebox compatibility
fn encode_packet_number(pn: u64, expected: u64) -> Vec<u8> {
    // Determine minimum length needed
    let len = if pn < 256 { 1 } else if pn < 65536 { 2 }
              else if pn < 16777216 { 3 } else { 4 };

    // Encode truncated value
    let truncated = (pn & ((1 << (8 * len)) - 1)) as u32;
    truncated.to_be_bytes()[4-len..].to_vec()
}
```

### 1.4 Packet Parsing in quiche

```rust
// From quiche/src/packet.rs
pub fn parse_header(buf: &mut octets::Octets) -> Result<Header> {
    let first = buf.get_u8()?;

    if first & FORM_BIT == 0 {
        // Short header (1-RTT)
        parse_short_header(first, buf)
    } else {
        // Long header
        parse_long_header(first, buf)
    }
}

// Header structure
pub struct Header {
    pub ty: Type,           // Initial, Handshake, 0-RTT, Short
    pub version: u32,
    pub dcid: ConnectionId,
    pub scid: ConnectionId,
    pub pkt_num: u64,
    pub pkt_num_len: usize,
}
```

---

## 2. Connection IDs

### 2.1 Connection ID Purpose

Connection IDs decouple connections from network 5-tuples:

```
Traditional TCP:
Connection = (src_ip, src_port, dst_ip, dst_port, protocol)
IP change → New tuple → New connection

QUIC:
Connection = Connection ID (64+ bits)
IP change → Same Connection ID → Same connection
```

### 2.2 Connection ID Structure

```rust
// From quiche/src/packet.rs
pub const MAX_CID_LEN: u8 = 20;

pub struct ConnectionId<'a>(ConnectionIdInner<'a>);

enum ConnectionIdInner<'a> {
    Vec(Vec<u8>),
    Ref(&'a [u8]),
}

impl ConnectionId<'_> {
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}
```

### 2.3 Connection ID Management

quiche uses a sophisticated CID management system:

```rust
// From quiche/src/cid.rs (38k lines)
pub struct ConnectionIdEntry {
    cid: ConnectionId<'static>,
    reset_token: Option<[u8; 16]>,
    local: bool,  // true if we issued it
    retired: bool,
}

pub struct ConnectionIds<'a> {
    // Our CIDs that we've issued to the peer
    local: VecDeque<ConnectionIdEntry>,
    // Peer's CIDs that they've issued to us
    remote: VecDeque<ConnectionIdEntry>,
    // CIDs we've retired
    retired: HashMap<u64, ConnectionIdEntry>,
}
```

**Connection ID Lifecycle:**
1. **Initial:** Client generates random SCID, server generates random SCID
2. **Handshake:** Server may send NEW_CONNECTION_ID frames
3. **Established:** Either side can issue new CIDs or retire old ones
4. **Migration:** Use alternate CID when changing paths

```rust
// Sending NEW_CONNECTION_ID
conn.send_new_connection_id(&cid, seq_num, retire_prior_to, reset_token)?;

// Retiring old CID
conn.retire_connection_id(seq_num)?;
```

### 2.4 Stateless Retry

Server can issue stateless retry to validate client address:

```
Client                          Server
   |                              |
   |-------- Initial ------------>| (ClientHello)
   |                              |
   |<------- Retry ---------------| (with Retry token)
   |                              |
   |-------- Initial ------------>| (with token in Initial)
   |         (with token)         |
   |                              |
   |<------- Handshake -----------|
```

**Retry Token:**
- Proves client received packet at claimed address
- Contains server's state (encrypted)
- Prevents amplification attacks

---

## 3. Frame Types

### 3.1 Frame Overview

QUIC packets contain one or more frames. Each frame type has a specific purpose:

```rust
// From quiche/src/frame.rs
pub enum Frame {
    Padding { len: usize },
    Ping { mtu_probe: Option<usize> },
    ACK {
        ack_delay: u64,
        ranges: RangeSet,
        ecn_counts: Option<EcnCounts>,
    },
    ResetStream { stream_id: u64, error_code: u64, final_size: u64 },
    StopSending { stream_id: u64, error_code: u64 },
    Crypto { data: RangeBuf },
    NewToken { token: Vec<u8> },
    Stream { stream_id: u64, data: RangeBuf },
    MaxData { max: u64 },
    MaxStreamData { stream_id: u64, max: u64 },
    MaxStreamsBidi { max: u64 },
    MaxStreamsUni { max: u64 },
    DataBlocked { limit: u64 },
    StreamDataBlocked { stream_id: u64, limit: u64 },
    StreamsBlockedBidi { limit: u64 },
    StreamsBlockedUni { limit: u64 },
    NewConnectionId {
        seq_num: u64,
        retire_prior_to: u64,
        conn_id: Vec<u8>,
        reset_token: [u8; 16],
    },
    RetireConnectionId { seq_num: u64 },
    PathChallenge { data: [u8; 8] },
    PathResponse { data: [u8; 8] },
    ConnectionClose { error_code: u64, frame_type: u64, reason: Vec<u8> },
    ApplicationClose { error_code: u64, reason: Vec<u8> },
    HandshakeDone,
    Datagram { data: Vec<u8> },
}
```

### 3.2 Frame Encoding

Frame type is a variable-length integer (varint):

```rust
// Varint encoding (RFC 9000 Section 16)
// 00xxxxxx -> 6 bits (0-63)
// 01xxxxxx -> 14 bits (64-16383)
// 10xxxxxx -> 30 bits (16384-1073741823)
// 11xxxxxx -> 62 bits (1073741824+)

fn encode_varint(val: u64) -> Vec<u8> {
    if val < 64 {
        vec![val as u8]
    } else if val < 16384 {
        vec![0x40 | ((val >> 8) & 0x3f) as u8, (val & 0xff) as u8]
    } else if val < 1073741824 {
        // 4 bytes
        // ...
    } else {
        // 8 bytes
        // ...
    }
}
```

### 3.3 Key Frame Types

**STREAM frames:**
```
STREAM Frame (RFC 9000 Section 19.8)
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Type (i=0x08-0x0f) | Stream ID (i) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | Offset (i) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | Length (i) | Stream Data (*) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

Flags in Type byte:
- FIN bit: stream complete
- LEN bit: Length field present
- OFF bit: Offset field present
```

**ACK frames:**
```
ACK Frame (RFC 9000 Section 19.3)
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Type (i=0x02-0x03) | Largest Acknowledged (i) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | ACK Delay (i) | ACK Range Count (i) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | First ACK Range (i) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | ACK Range (*) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**CRYPTO frames:**
```
CRYPTO Frame (RFC 9000 Section 19.6)
Used for TLS handshake data
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Type (i=0x06) | Offset (i) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
... (variable) | Length (i) | Crypto Data (*) ...
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### 3.4 Frame Parsing Example

```rust
// From quiche/src/frame.rs
impl Frame {
    pub fn from_bytes(
        b: &mut octets::Octets, pkt: packet::Type,
    ) -> Result<Frame> {
        let frame_type = b.get_varint()?;

        match frame_type {
            0x00 => {
                // PADDING - count consecutive zeros
                let mut len = 1;
                while b.peek_u8() == Ok(0x00) {
                    b.get_u8()?;
                    len += 1;
                }
                Frame::Padding { len }
            }
            0x01 => Frame::Ping { mtu_probe: None },
            0x02..=0x03 => {
                // ACK frame
                let largest = b.get_varint()?;
                let delay = b.get_varint()?;
                let range_count = b.get_varint()?;
                // ... parse ranges
                Frame::ACK { ack_delay: delay, ranges, ecn_counts: None }
            }
            0x04 => {
                // RESET_STREAM
                let stream_id = b.get_varint()?;
                let error_code = b.get_varint()?;
                let final_size = b.get_varint()?;
                Frame::ResetStream { stream_id, error_code, final_size }
            }
            // ... more frame types
            _ => return Err(Error::InvalidFrame),
        }
    }
}
```

---

## 4. Stream Multiplexing

### 4.1 Stream IDs

QUIC streams are identified by 62-bit stream IDs:

```
Stream ID structure (RFC 9000 Section 2.1)
Bits 0-1: Initiator
  00 = Client-initiated, bidirectional
  01 = Server-initiated, bidirectional
  10 = Client-initiated, unidirectional
  11 = Server-initiated, unidirectional

Bit 2: Directionality
  0 = Bidirectional
  1 = Unidirectional

Bits 3-62: Stream number

Examples:
  0x00000000 (0) = Client bidirectional stream 0
  0x00000001 (1) = Server bidirectional stream 0
  0x00000002 (2) = Client unidirectional stream 0
  0x00000003 (3) = Server unidirectional stream 0
  0x00000004 (4) = Client bidirectional stream 1
```

### 4.2 Stream States

```
Send Side:                    Receive Side:

  +--------+                  +--------+
  |  Idle  |                  |  Idle  |
  +---+----+                  +---+----+
      |                           |
      | Send STREAM              | Receive STREAM
      v                           v
  +--------+                  +--------+
  |  Open  |                  |  Recv  |
  +---+----+                  +---+----+
      |                           |
      | Send FIN                 | Receive FIN
      v                           v
  +--------+                  +--------+
  |  Half  |                  |  Half  |
  | Closed |                  | Closed |
  +--------+                  +--------+
      |                           |
      | Receive FIN              | Read all data
      v                           v
  +--------+                  +--------+
  | Closed |                  | Closed |
  +--------+                  +--------+
```

### 4.3 Stream Implementation in quiche

```rust
// From quiche/src/stream/mod.rs
pub struct Stream<F: BufFactory = DefaultBufFactory> {
    /// Receive buffer
    recv: RecvBuf<F>,
    /// Send buffer
    send: SendBuf<F>,
    /// Stream priority (for scheduling)
    priority: StreamPriority,
    /// Whether the stream is bidirectional
    bidi: bool,
    /// Whether we initiated the stream
    local: bool,
}

// StreamMap manages all streams
pub struct StreamMap<F: BufFactory = DefaultBufFactory> {
    streams: StreamIdHashMap<Stream<F>>,
    collected: StreamIdHashSet,

    // Flow control limits
    peer_max_streams_bidi: u64,
    peer_max_streams_uni: u64,
    local_max_streams_bidi: u64,
    local_max_streams_uni: u64,

    // Scheduling queues (using intrusive RB-tree)
    flushable: RBTree<StreamFlushablePriorityAdapter>,
    readable: RBTree<StreamReadablePriorityAdapter>,
    writable: RBTree<StreamWritablePriorityAdapter>,
}
```

### 4.4 Stream Scheduling

quiche uses a Red-Black tree for priority-based stream scheduling:

```rust
// From quiche/src/stream/mod.rs
intrusive_collections::intrusive_adapter! {
    StreamFlushablePriorityAdapter = Arc<Stream>:
    Stream.link_flushable { RBTreeAtomicLink }
}

impl Stream {
    // Priority key for scheduling
    pub fn priority_key(&self) -> StreamPriorityKey {
        StreamPriorityKey {
            urgency: self.priority.urgency,
            incremental: self.priority.incremental,
            stream_id: self.id,
        }
    }
}

// Lower urgency value = higher priority
// incremental streams can be deprioritized
```

---

## 5. Flow Control

### 5.1 Connection-Level Flow Control

Prevents sender from overwhelming receiver's connection buffer:

```
Sender                          Receiver
   |                              |
   | DATA (offset 0-1000) ------->|
   |                              | Increases data_received
   |                              | If > initial_max_data:
   |                              | - Block further data
   |                              | - Send MAX_DATA
   |<----- MAX_DATA (2000) -------|
   |                              |
   | DATA (offset 1000-2000) ---->|
```

```rust
// From quiche/src/flowcontrol.rs
pub struct FlowControl {
    /// Maximum offset we allow peer to send
    max_data: u64,
    /// Current consumed offset (data read by application)
    consumed: u64,
    /// Current received offset (data in buffer)
    received: u64,
    /// Whether we've sent a MAX_DATA update
    needs_update: bool,
}

impl FlowControl {
    pub fn update(&mut self, consumed: u64) -> Option<u64> {
        self.consumed = consumed;

        // Send MAX_DATA when consumed reaches threshold
        if self.consumed > self.max_data * 0.8 {
            self.max_data *= 2;  // Double the window
            self.needs_update = false;
            Some(self.max_data)
        } else {
            None
        }
    }
}
```

### 5.2 Stream-Level Flow Control

Each stream has independent flow control:

```rust
// Transport parameters
pub struct TransportParams {
    initial_max_data: u64,              // Connection-level
    initial_max_stream_data_bidi_local: u64,   // Local bidi streams
    initial_max_stream_data_bidi_remote: u64,  // Remote bidi streams
    initial_max_stream_data_uni: u64,   // Unidirectional streams
}

// Per-stream flow control
pub struct RecvBuf<F: BufFactory> {
    /// Maximum offset we allow
    max_data: u64,
    /// Current offset in stream
    offset: u64,
    /// Buffer for received data
    data: Vec<u8>,
}
```

### 5.3 Flow Control Limits

```rust
// From quiche/src/lib.rs
// Default connection window
const DEFAULT_CONNECTION_WINDOW: u64 = 48 * 1024;  // 48 KB

// Maximum connection window
const MAX_CONNECTION_WINDOW: u64 = 24 * 1024 * 1024;  // 24 MB

// Default stream window
const DEFAULT_STREAM_WINDOW: u64 = 32 * 1024;  // 32 KB

// Maximum stream window
pub const MAX_STREAM_WINDOW: u64 = 16 * 1024 * 1024;  // 16 MB

// Connection window should be larger than stream window
const CONNECTION_WINDOW_FACTOR: f64 = 1.5;
```

### 5.4 Blocking and Unblocking

```rust
// From quiche/src/stream/mod.rs
impl StreamMap {
    /// Check if stream is blocked by flow control
    fn is_stream_blocked(&self, stream_id: u64, offset: u64) -> bool {
        if let Some(stream) = self.streams.get(&stream_id) {
            offset >= stream.recv.max_data
        } else {
            false
        }
    }

    /// Mark stream as blocked
    fn mark_blocked(&mut self, stream_id: u64, limit: u64) {
        self.blocked.insert(stream_id, limit);
    }

    /// Unblock stream when MAX_STREAM_DATA received
    fn unblock_stream(&mut self, stream_id: u64, new_max: u64) {
        self.blocked.remove(&stream_id);
        if let Some(stream) = self.streams.get_mut(&stream_id) {
            stream.recv.max_data = new_max;
        }
    }
}
```

---

## 6. ACK Mechanism

### 6.1 ACK Ranges

QUIC uses gap/length encoding for ACK ranges:

```
ACK Ranges example:
Packets received: 1-5, 7-10, 13-15

Encoding:
- Largest acknowledged: 15
- First range: 13-15 (length 3)
- Gap: 2 (packets 11-12 missing)
- Second range: 7-10 (length 4)
- Gap: 1 (packet 6 missing)
- Third range: 1-5 (length 5)
```

### 6.2 Range Set Implementation

```rust
// From quiche/src/ranges.rs
pub struct RangeSet {
    /// Intervals [start, end) in ascending order
    intervals: Vec<(u64, u64)>,
}

impl RangeSet {
    /// Add a packet number to the set
    pub fn insert(&mut self, pn: u64) {
        // Find position and merge adjacent ranges
        // ...
    }

    /// Remove packets (when acknowledged)
    pub fn subtract(&mut self, other: &RangeSet) {
        // ...
    }

    /// Iterate over ranges in descending order
    pub fn ranges(&self) -> impl Iterator<Item = (u64, u64)> {
        self.intervals.iter().rev().copied()
    }
}
```

### 6.3 ACK Delay

QUIC measures and reports ACK delay:

```rust
// From quiche/src/recovery/mod.rs
const DEFAULT_ACK_DELAY_EXPONENT: u64 = 3;  // Multiply by 2^3 = 8

// ACK delay in microseconds
let ack_delay_micros = (now - recv_time).as_micros() as u64;

// Scaled delay for RTT calculation
let ack_delay_scaled = ack_delay_micros >> ack_delay_exponent;

// Include in ACK frame
Frame::ACK {
    ack_delay: ack_delay_scaled,
    ranges,
    ecn_counts: None,
}
```

### 6.4 ECN Support

QUIC supports Explicit Congestion Notification:

```rust
// From quiche/src/frame.rs
pub struct EcnCounts {
    ect0_count: u64,  // ECN-Capable Transport (0)
    ect1_count: u64,  // ECN-Capable Transport (1)
    ecn_ce_count: u64,  // ECN Congestion Experienced
}

// Included in ACK frame when ECN is used
Frame::ACK {
    ack_delay,
    ranges,
    ecn_counts: Some(ecn_counts),
}
```

---

## 7. Connection Migration

### 7.1 Path Validation

When client's IP changes, QUIC validates the new path:

```
Client moves to new network:

Client (new IP)                   Server
   |                                |
   |-------- PATH_CHALLENGE ------->| (with random data)
   |                                |
   |<------- PATH_RESPONSE ---------| (echoes challenge data)
   |                                |
   |-------- PATH_RESPONSE ------->| (echoes server's challenge)
   |                                |
   |<------- ACK -------------------|
   |                                |
   |======== DATA (migrated) =======|
```

### 7.2 Path Implementation

```rust
// From quiche/src/path.rs
pub struct Path {
    /// Peer address
    peer_addr: SocketAddr,
    /// Local address
    local_addr: SocketAddr,
    /// Whether path is validated
    validated: bool,
    /// Challenge data for validation
    challenge: Option<[u8; 8]>,
    /// Recovery state for this path (CC, RTT)
    recovery: Recovery,
}

pub struct PathManager {
    /// Active path
    active: Path,
    /// Alternate paths (for migration)
    alternate: Vec<Path>,
    /// Pending challenges
    pending_challenges: VecDeque<([u8; 8], SocketAddr)>,
}
```

### 7.3 Migration Handling

```rust
// From quiche/src/lib.rs
impl Connection {
    /// Handle packet from new address
    fn handle_new_path(
        &mut self,
        from: SocketAddr,
        to: SocketAddr,
    ) -> Result<()> {
        if from != self.paths.active().peer_addr {
            // New path detected
            let new_path_id = self.paths.add_path(from, to)?;

            // Send PATH_CHALLENGE on new path
            self.paths.send_challenge(new_path_id)?;

            // Don't migrate yet - wait for validation
            self.paths.mark_pending(new_path_id);
        }

        Ok(())
    }

    /// Migrate to validated path
    fn migrate_to_path(&mut self, path_id: usize) {
        self.paths.activate(path_id);

        // Reset congestion control for new path
        self.recovery.on_path_change();
    }
}
```

### 7.4 NAT Rebinding

QUIC handles NAT rebinding gracefully:

```rust
// Client behind NAT changes port
// Server sees packet from same IP, different port

impl Connection {
    pub fn recv(
        &mut self,
        buf: &mut [u8],
        info: RecvInfo,
    ) -> Result<usize> {
        // Check if this is a new path
        if info.from != self.peer_addr {
            // Could be NAT rebinding or migration
            if info.from.ip() == self.peer_addr.ip() {
                // Same IP, different port = likely NAT rebinding
                self.handle_nat_rebinding(info.from)?;
            } else {
                // Different IP = explicit migration
                self.handle_migration(info)?;
            }
        }

        // Process packet normally
        // ...
    }
}
```

---

## Summary

### Key Takeaways

1. **Packet structure** - Long headers for handshake, short headers for 1-RTT
2. **Connection IDs** - Enable migration, issued/retired dynamically
3. **Frame types** - STREAM, ACK, CRYPTO, and more - each with specific purpose
4. **Stream multiplexing** - Independent streams with priority scheduling
5. **Flow control** - Connection and stream level, with MAX_DATA/STREAM_DATA
6. **ACK mechanism** - Range-based, with delay measurement and ECN support
7. **Connection migration** - Path validation with PATH_CHALLENGE/RESPONSE

### Next Steps

Continue to [02-http3-implementation-deep-dive.md](02-http3-implementation-deep-dive.md) for:
- HTTP/3 over QUIC mapping
- QPACK header compression
- HTTP/3 frame types
- Request/response handling

---

## Further Reading

- [RFC 9000 - QUIC: A UDP-Based Multiplexed Transport](https://www.rfc-editor.org/rfc/rfc9000.html)
- [quiche source - packet.rs](quiche/src/packet.rs)
- [quiche source - frame.rs](quiche/src/frame.rs)
- [quiche source - stream/mod.rs](quiche/src/stream/mod.rs)
