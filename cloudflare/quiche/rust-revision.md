---
title: "Rust Revision: quiche Translation Guide"
subtitle: "Rust patterns, zero-copy design, and replication for ewe_platform"
---

# Rust Revision: quiche Translation Guide

## Introduction

This document provides a comprehensive guide to translating quiche's Rust patterns for replication in ewe_platform. We cover zero-copy design, intrusive collections, enum_dispatch for polymorphism, and FFI boundaries.

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Zero-Copy Buffer Design](#2-zero-copy-buffer-design)
3. [Intrusive Collections](#3-intrusive-collections)
4. [Enum Dispatch Pattern](#4-enum-dispatch-pattern)
5. [Error Handling](#5-error-handling)
6. [FFI Boundaries](#6-ffi-boundaries)
7. [Replication for ewe_platform](#7-replication-for-ewe_platform)

---

## 1. Architecture Overview

### 1.1 quiche Crate Structure

```
quiche/
├── lib.rs           # Connection struct, Config, connect()/accept()
├── h3/              # HTTP/3 module
├── recovery/        # Loss detection, congestion control
├── stream/          # Stream multiplexing, flow control
├── tls/             # TLS abstraction (BoringSSL/OpenSSL)
├── crypto/          # Packet protection, key derivation
├── packet.rs        # Packet parsing
├── frame.rs         # Frame encode/decode
└── ffi.rs           # C FFI (optional feature)
```

### 1.2 Key Design Patterns

| Pattern | Purpose | Location |
|---------|---------|----------|
| BufFactory | Zero-copy buffer creation | range_buf.rs |
| enum_dispatch | Zero-cost CC polymorphism | recovery/mod.rs |
| Intrusive RB-Tree | Priority stream scheduling | stream/mod.rs |
| RangeSet | ACK range tracking | ranges.rs |
| Octets wrapper | Safe buffer parsing | octets crate |

---

## 2. Zero-Copy Buffer Design

### 2.1 BufFactory Trait

```rust
// From quiche/src/range_buf.rs
pub trait BufFactory: Clone + Default + Send + Sync {
    /// Create a new buffer of the given size
    fn new_box(&self, size: usize) -> Box<[u8]>;
}

/// Default implementation using Vec
#[derive(Default, Clone)]
pub struct DefaultBufFactory;

impl BufFactory for DefaultBufFactory {
    fn new_box(&self, size: usize) -> Box<[u8]> {
        vec![0u8; size].into_boxed_slice()
    }
}
```

### 2.2 BufSplit Trait

```rust
// Zero-copy buffer splitting
pub trait BufSplit: AsRef<[u8]> + AsMut<[u8]> {
    /// Split buffer at offset, returning two views
    fn split_at(self, mid: usize) -> (Self, Self)
    where
        Self: Sized;
}

impl BufSplit for Box<[u8]> {
    fn split_at(self, mid: usize) -> (Self, Self) {
        // Note: This is simplified - actual impl uses Arc for sharing
        let (left, right) = self.split_at(mid);
        (left.to_vec().into_boxed_slice(), right.to_vec().into_boxed_slice())
    }
}
```

### 2.3 RangeBuf for Zero-Copy

```rust
// From quiche/src/range_buf.rs
pub struct RangeBuf {
    /// Underlying buffer (can be shared)
    data: Arc<Box<[u8]>>,
    /// Offset into buffer
    offset: usize,
    /// Length of view
    len: usize,
}

impl RangeBuf {
    pub fn from_slice(slice: &[u8]) -> Self {
        Self {
            data: Arc::new(slice.to_vec().into_boxed_slice()),
            offset: 0,
            len: slice.len(),
        }
    }

    /// Split without copying
    pub fn split_off(&mut self, at: usize) -> Self {
        assert!(at <= self.len);

        let new = Self {
            data: Arc::clone(&self.data),
            offset: self.offset + at,
            len: self.len - at,
        };

        self.len = at;
        new
    }
}
```

### 2.4 Connection Generic Over BufFactory

```rust
// From quiche/src/lib.rs
pub struct Connection<F: BufFactory = DefaultBufFactory> {
    /// Receive buffer
    recv_buf: F,
    /// Send buffer
    send_buf: F,
    /// Stream map (also generic)
    streams: stream::StreamMap<F>,
    /// Crypto spaces
    crypto: [CryptoSpace<F>; 3],
    // ...
}

impl<F: BufFactory> Connection<F> {
    pub fn recv(
        &mut self,
        buf: &mut [u8],
        info: RecvInfo,
    ) -> Result<usize> {
        // Zero-copy: use buf directly without intermediate allocation
        let hdr = packet::parse_header(&mut octets::Octets::with_slice(buf))?;

        // Process packet...
    }
}
```

---

## 3. Intrusive Collections

### 3.1 Why Intrusive Collections?

Standard Rust collections allocate nodes on the heap. Intrusive collections store links within the data structure itself:

```
Standard RB-Tree:
┌─────────────┐     ┌─────────────┐
│ TreeNode    │     │ TreeNode    │
│ ┌─────────┐ │     │ ┌─────────┐ │
│ │ link    │─┼────►│ │ link    │ │
│ └─────────┘ │     │ └─────────┘ │
│   data...   │     │   data...   │
└─────────────┘     └─────────────┘

Intrusive RB-Tree:
┌─────────────┐     ┌─────────────┐
│ Stream      │     │ Stream      │
│ ┌─────────┐ │     │ ┌─────────┐ │
│ │ link    │─┼────►│ │ link    │ │
│ └─────────┘ │     │ └─────────┘ │
│   data...   │     │   data...   │
└─────────────┘     └─────────────┘

No separate node allocation!
```

### 3.2 Intrusive Adapter Definition

```rust
// From quiche/src/stream/mod.rs
use intrusive_collections::{intrusive_adapter, RBTreeAtomicLink};

intrusive_collections::intrusive_adapter! {
    /// Adapter for flushable streams RB-tree
    StreamFlushablePriorityAdapter = Arc<Stream>:
    Stream.link_flushable { RBTreeAtomicLink }
}

intrusive_collections::intrusive_adapter! {
    /// Adapter for readable streams RB-tree
    StreamReadablePriorityAdapter = Arc<Stream>:
    Stream.link_readable { RBTreeAtomicLink }
}

intrusive_collections::intrusive_adapter! {
    /// Adapter for writable streams RB-tree
    StreamWritablePriorityAdapter = Arc<Stream>:
    Stream.link_writable { RBTreeAtomicLink }
}
```

### 3.3 Stream with Links

```rust
// From quiche/src/stream/mod.rs
use intrusive_collections::RBTreeAtomicLink;

pub struct Stream<F: BufFactory = DefaultBufFactory> {
    /// Stream ID
    id: u64,

    /// Receive buffer
    recv: RecvBuf<F>,

    /// Send buffer
    send: SendBuf<F>,

    /// Priority for scheduling
    priority: StreamPriority,

    // Intrusive links for RB-trees
    link_flushable: RBTreeAtomicLink,
    link_readable: RBTreeAtomicLink,
    link_writable: RBTreeAtomicLink,

    // ... other fields
}

impl<F: BufFactory> Stream<F> {
    /// Get priority key for RB-tree ordering
    pub fn priority_key(&self) -> StreamPriorityKey {
        StreamPriorityKey {
            urgency: self.priority.urgency,
            incremental: self.priority.incremental,
            stream_id: self.id,
        }
    }
}
```

### 3.4 RB-Tree Operations

```rust
use intrusive_collections::{RBTree, KeyAdapter};

pub struct StreamMap<F: BufFactory = DefaultBufFactory> {
    /// Streams that have data to send
    flushable: RBTree<StreamFlushablePriorityAdapter>,

    /// Streams that have data to read
    readable: RBTree<StreamReadablePriorityAdapter>,

    /// Streams that can accept more data
    writable: RBTree<StreamWritablePriorityAdapter>,
}

impl<F: BufFactory> StreamMap<F> {
    /// Insert stream into flushable queue
    pub fn mark_flushable(&mut self, stream: Arc<Stream<F>>) {
        self.flushable.insert(stream);
    }

    /// Get next flushable stream (highest priority)
    pub fn next_flushable(&mut self) -> Option<Arc<Stream<F>>> {
        self.flushable.pop_front()
    }

    /// Iterate over readable streams
    pub fn readable(&self) -> impl Iterator<Item = u64> + '_ {
        self.readable.iter().map(|s| s.id)
    }
}
```

---

## 4. Enum Dispatch Pattern

### 4.1 enum_dispatch for Zero-Cost Polymorphism

```rust
// From quiche/src/recovery/mod.rs
use enum_dispatch::enum_dispatch;

/// Trait defining recovery operations
pub trait RecoveryOps {
    fn lost_count(&self) -> usize;
    fn bytes_lost(&self) -> u64;
    fn cwnd(&self) -> usize;
    fn on_packet_sent(&mut self, pkt: Sent, epoch: packet::Epoch, now: Instant);
    fn on_ack_received(
        &mut self,
        ranges: &RangeSet,
        ack_delay: u64,
        epoch: packet::Epoch,
        now: Instant,
    ) -> Result<OnAckReceivedOutcome>;
    // ... 40+ methods
}

/// Enum dispatching to different CC implementations
#[enum_dispatch::enum_dispatch(RecoveryOps)]
#[derive(Debug)]
pub(crate) enum Recovery {
    Legacy(LegacyRecovery),
    GCongestion(GRecovery),
}

// Usage - zero-cost dispatch
impl Connection {
    fn on_ack_received(&mut self, ...) {
        // Dispatches to correct implementation based on enum variant
        let outcome = self.recovery.on_ack_received(...)?;
    }
}
```

### 4.2 Comparison with Trait Objects

```rust
// Trait object approach (heap allocation, dynamic dispatch)
struct Connection {
    recovery: Box<dyn RecoveryOps>,  // Heap allocation, vtable lookup
}

// enum_dispatch approach (stack allocation, static dispatch)
struct Connection {
    recovery: Recovery,  // Stack allocated, direct call through enum
}

// enum_dispatch generates optimized match:
impl RecoveryOps for Recovery {
    fn cwnd(&self) -> usize {
        match self {
            Recovery::Legacy(r) => r.cwnd(),
            Recovery::GCongestion(r) => r.cwnd(),
        }
    }
    // ... monomorphized implementations for all methods
}
```

### 4.3 Static Vtable Pattern (congestion/)

The older congestion module uses a static vtable pattern:

```rust
// From quiche/src/recovery/congestion/mod.rs
pub struct CongestionControlOps {
    pub on_init: fn(&mut Congestion),
    pub on_packet_sent: fn(&mut Congestion, usize, usize, Instant),
    pub on_packets_acked: fn(&mut Congestion, &[Acked], usize, Instant),
    pub congestion_event: fn(&mut Congestion, usize, usize, Instant),
    pub checkpoint: fn(&mut Congestion),
    pub rollback: fn(&mut Congestion),
    #[cfg(feature = "qlog")]
    pub state_str: fn(&Congestion) -> &'static str,
    pub debug_fmt: fn(&Congestion, &mut std::fmt::DebugStruct<'_, '_>),
}

/// CUBIC implementation
pub(crate) static CUBIC: CongestionControlOps = CongestionControlOps {
    on_init,
    on_packet_sent,
    on_packets_acked,
    congestion_event,
    checkpoint,
    rollback,
    state_str,
    debug_fmt,
};

/// Reno implementation
pub(crate) static RENO: CongestionControlOps = CongestionControlOps {
    on_init: cubic::on_init,  // Reuse some implementations
    on_packet_sent: cubic::on_packet_sent,
    on_packets_acked: reno::on_packets_acked,
    congestion_event: reno::congestion_event,
    // ...
};

impl Congestion {
    pub fn new(algo: &'static CongestionControlOps) -> Self {
        Self {
            ops: algo,
            // ...
        }
    }

    pub fn on_packet_sent(&mut self, sent_bytes: usize, bif: usize, now: Instant) {
        (self.ops.on_packet_sent)(self, sent_bytes, bif, now);
    }
}
```

---

## 5. Error Handling

### 5.1 Error Type Design

```rust
// From quiche/src/error.rs
/// QUIC error type - Copy + Clone for hot-path ergonomics
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Done,
    BufferTooShort,
    UnknownVersion,
    InvalidFrame,
    InvalidPacket,
    InvalidState,
    InvalidStreamState(u64),
    InvalidTransportParam,
    CryptoFail,
    TlsFail,
    FlowControl,
    StreamLimit,
    StreamStopped(u64),
    StreamReset(u64),
    FinalSize,
    CongestionControl,
    IdLimit,
    OutOfIdentifiers,
    KeyUpdate,
    CryptoBufferExceeded,
    InvalidAckRange,
    OptimisticAckDetected,
    InvalidDcidInitialization,
}

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

// Error is Copy - can be passed by value in hot paths
impl Copy for Error {}
```

### 5.2 Error Conversion

```rust
// From quiche/src/error.rs
impl From<octets::BufferTooShortError> for Error {
    fn from(_err: octets::BufferTooShortError) -> Self {
        Error::BufferTooShort
    }
}

// HTTP/3 has separate error type
// From quiche/src/h3/mod.rs
pub enum Error {
    Done,
    BufferTooShort,
    InternalError,
    // ...
    TransportError(crate::Error),  // Wraps quiche::Error
}

impl From<crate::Error> for h3::Error {
    fn from(e: crate::Error) -> Self {
        h3::Error::TransportError(e)
    }
}
```

### 5.3 Error Handling in Hot Path

```rust
// From quiche/src/lib.rs
impl Connection {
    pub fn recv(
        &mut self,
        buf: &mut [u8],
        info: RecvInfo,
    ) -> Result<usize> {
        // Parse packet header - returns Error::BufferTooShort on failure
        let hdr = packet::parse_header(&mut octets::Octets::with_slice(buf))?;

        // Decrypt packet - returns Error::CryptoFail on failure
        let decrypted = self.decrypt_packet(&hdr, buf)?;

        // Process frames - propagate errors up
        let mut done = 0;
        while done < decrypted.len() {
            let frame = frame::from_bytes(&mut octets::Octets::with_slice(&decrypted[done..]))?;
            self.process_frame(frame)?;  // Various errors possible
            // ...
        }

        Ok(done)
    }
}
```

---

## 6. FFI Boundaries

### 6.1 FFI Feature Gate

```rust
// From quiche/Cargo.toml
[features]
# Build and expose the FFI API.
ffi = ["dep:cdylib-link-lines"]

# Exposes internal APIs for testing
internal = []

[lib]
crate-type = ["lib", "staticlib", "cdylib"]  # For C consumers
```

### 6.2 FFI Wrapper Pattern

```rust
// From quiche/src/ffi.rs
use libc::{c_int, c_void, size_t, ssize_t};

/// Opaque connection pointer for C
pub struct Connection {
    // Internal fields
}

#[no_mangle]
pub extern "C" fn quiche_connect(
    server_name: *const c_char,
    scid: *const u8,
    scid_len: size_t,
    peer: *const sockaddr,
    peer_len: socklen_t,
    config: *mut Config,
) -> *mut Connection {
    // Validate inputs
    if scid.is_null() || config.is_null() {
        return ptr::null_mut();
    }

    // Convert C strings
    let server_name = if !server_name.is_null() {
        Some(unsafe { CStr::from_ptr(server_name) }.to_str().ok()?)
    } else {
        None
    };

    // Convert SCID
    let scid_slice = unsafe { slice::from_raw_parts(scid, scid_len) };
    let scid = ConnectionId::from_ref(scid_slice);

    // Call Rust API
    let conn = Box::new(quiche::connect(server_name, &scid, /*...*/));

    Box::into_raw(conn) as *mut Connection
}

#[no_mangle]
pub extern "C" fn quiche_conn_recv(
    conn: *mut Connection,
    buf: *mut u8,
    buf_len: size_t,
    info: RecvInfo,
) -> ssize_t {
    if conn.is_null() || buf.is_null() {
        return -1;
    }

    let conn = unsafe { &mut *conn };
    let buf = unsafe { slice::from_raw_parts_mut(buf, buf_len) };

    match conn.recv(buf, info) {
        Ok(n) => n as ssize_t,
        Err(e) => e.to_c(),  // Convert to C error code
    }
}
```

### 6.3 Error Code Mapping

```rust
// From quiche/src/error.rs
impl Error {
    /// Convert to C error code
    pub(crate) fn to_c(self) -> libc::ssize_t {
        match self {
            Error::Done => -1,
            Error::BufferTooShort => -2,
            Error::UnknownVersion => -3,
            Error::InvalidFrame => -4,
            Error::InvalidPacket => -5,
            Error::InvalidState => -6,
            Error::InvalidStreamState(_) => -7,
            Error::InvalidTransportParam => -8,
            Error::CryptoFail => -9,
            Error::TlsFail => -10,
            Error::FlowControl => -11,
            Error::StreamLimit => -12,
            // ... all variants mapped
        }
    }
}
```

### 6.4 C Header Generation

```c
/* From quiche/include/quiche.h */

#ifndef QUICHE_H
#define QUICHE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

// Opaque types
typedef struct quiche_config quiche_config;
typedef struct quiche_connection quiche_connection;

// Error codes
#define QUICHE_ERR_DONE -1
#define QUICHE_ERR_BUFFER_TOO_SHORT -2
#define QUICHE_ERR_UNKNOWN_VERSION -3

// Create configuration
quiche_config *quiche_config_new(uint32_t version);

// Create connection
quiche_connection *quiche_connect(
    const char *server_name,
    const uint8_t *scid,
    size_t scid_len,
    // ...
);

// Process incoming packet
ssize_t quiche_conn_recv(
    quiche_connection *conn,
    uint8_t *buf,
    size_t buf_len,
    // ...
);

// Generate outgoing packet
ssize_t quiche_conn_send(
    quiche_connection *conn,
    uint8_t *out,
    size_t out_len,
    // ...
);

#endif
```

---

## 7. Replication for ewe_platform

### 7.1 Valtron Integration Pattern

For ewe_platform, we need to adapt quiche patterns to the valtron executor (no async/tokio):

```rust
// ewe_platform pattern - TaskIterator for QUIC events
use valtron::{TaskIterator, TaskStatus, NoSpawner};

/// QUIC connection event processing
pub struct QuicEventTask {
    conn: Connection,
    socket: UdpSocket,
    recv_buf: Vec<u8>,
    send_buf: Vec<u8>,
    state: QuicTaskState,
}

enum QuicTaskState {
    Receiving,
    Processing,
    Sending,
    Waiting(Instant),
}

impl TaskIterator for QuicEventTask {
    type Ready = QuicEventResult;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        match self.state {
            QuicTaskState::Receiving => {
                // Try to receive packet (non-blocking)
                match self.socket.recv_from(&mut self.recv_buf) {
                    Ok((len, from)) => {
                        self.state = QuicTaskState::Processing;
                        TaskStatus::Pending(())
                    }
                    Err(ref e) if e.kind() == WouldBlock => {
                        // No data, check if we need to send
                        self.state = QuicTaskState::Sending;
                        TaskStatus::Pending(())
                    }
                    Err(_) => TaskStatus::Ready(QuicEventResult::Error),
                }
            }
            QuicTaskState::Processing => {
                // Process received packet
                let recv_info = RecvInfo { from, to: self.local_addr };
                match self.conn.recv(&mut self.recv_buf, recv_info) {
                    Ok(_) => {
                        self.state = QuicTaskState::Sending;
                        TaskStatus::Pending(())
                    }
                    Err(Error::Done) => {
                        self.state = QuicTaskState::Receiving;
                        TaskStatus::Pending(())
                    }
                    Err(_) => TaskStatus::Ready(QuicEventResult::Error),
                }
            }
            QuicTaskState::Sending => {
                // Generate and send packets
                loop {
                    match self.conn.send(&mut self.send_buf) {
                        Ok((len, info)) => {
                            let _ = self.socket.send_to(&self.send_buf[..len], info.to);
                            // Continue sending more packets
                        }
                        Err(Error::Done) => {
                            // No more packets
                            if let Some(timeout) = self.conn.timeout() {
                                self.state = QuicTaskState::Waiting(Instant::now() + timeout);
                            } else {
                                self.state = QuicTaskState::Receiving;
                            }
                            return TaskStatus::Pending(());
                        }
                        Err(_) => return TaskStatus::Ready(QuicEventResult::Error),
                    }
                }
            }
            QuicTaskState::Waiting(deadline) => {
                if Instant::now() >= deadline {
                    self.conn.on_timeout();
                    self.state = QuicTaskState::Sending;
                    TaskStatus::Pending(())
                } else {
                    TaskStatus::Pending(())
                }
            }
        }
    }
}
```

### 7.2 Stream Scheduling Adaptation

```rust
// ewe_platform - Stream scheduling without async
use valtron::{StreamIterator, DrivenStreamIterator};

pub struct QuicStreamIterator {
    conn: Arc<Mutex<Connection>>,
    stream_id: u64,
    buf: Vec<u8>,
    offset: usize,
}

impl StreamIterator for QuicStreamIterator {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Result<Self::Item>> {
        let conn = self.conn.lock().unwrap();

        // Check if stream has data
        if !conn.readable().any(|id| id == self.stream_id) {
            return None;  // No data yet
        }

        // Read from stream
        match conn.stream_recv(self.stream_id, &mut self.buf) {
            Ok((0, true)) => Some(None),  // FIN received
            Ok((0, false)) => None,  // No data
            Ok((len, fin)) => {
                Some(Some(Ok(self.buf[..len].to_vec())))
            }
            Err(e) => Some(Some(Err(e))),
        }
    }
}
```

### 7.3 Memory-Efficient Buffer Pool

```rust
// ewe_platform - Reusable buffer pool for QUIC packets
pub struct QuicBufferPool {
    buffers: Vec<Vec<u8>>,
    buffer_size: usize,
    max_buffers: usize,
}

impl QuicBufferPool {
    pub fn new(buffer_size: usize, max_buffers: usize) -> Self {
        Self {
            buffers: Vec::with_capacity(max_buffers),
            buffer_size,
            max_buffers,
        }
    }

    pub fn acquire(&mut self) -> Vec<u8> {
        self.buffers.pop().unwrap_or_else(|| {
            vec![0u8; self.buffer_size]
        })
    }

    pub fn release(&mut self, mut buf: Vec<u8>) {
        if self.buffers.len() < self.max_buffers {
            buf.resize(self.buffer_size, 0);
            self.buffers.push(buf);
        }
    }
}

// BufFactory implementation using pool
#[derive(Clone)]
pub struct PooledBufFactory {
    pool: Arc<Mutex<QuicBufferPool>>,
}

impl BufFactory for PooledBufFactory {
    fn new_box(&self, size: usize) -> Box<[u8]> {
        let mut pool = self.pool.lock().unwrap();
        let buf = pool.acquire();
        buf[..size].to_vec().into_boxed_slice()
    }
}
```

---

## Summary

### Key Takeaways

1. **BufFactory** - Trait for zero-copy buffer creation, Connection generic over it
2. **Intrusive collections** - RB-tree links stored in Stream, no separate allocation
3. **enum_dispatch** - Zero-cost dispatch between CC implementations
4. **Error handling** - Copy + Clone errors for hot-path ergonomics
5. **FFI boundaries** - Clear separation, error code mapping, opaque pointers
6. **Valtron adaptation** - TaskIterator for event loop, stream iterators for data

### Replication Checklist for ewe_platform

- [ ] Implement BufFactory trait with pooled allocator
- [ ] Use intrusive collections for stream scheduling
- [ ] Implement enum_dispatch for modular CC
- [ ] Design TaskIterator for QUIC event processing
- [ ] Create buffer pool for packet I/O
- [ ] Define FFI-compatible error codes if needed

---

## Further Reading

- [intrusive-collections crate](https://docs.rs/intrusive-collections/)
- [enum_dispatch crate](https://docs.rs/enum_dispatch/)
- [quiche source - range_buf.rs](quiche/src/range_buf.rs)
- [quiche source - stream/mod.rs](quiche/src/stream/mod.rs)
- [Valtron README](/home/darkvoid/Boxxed/@dev/ewe_platform/backends/foundation_core/src/valtron/README.md)
