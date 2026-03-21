---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/dst-with-hissi/exploration.md
explored_at: 2026-03-22
revised_at: 2026-03-22
workspace: dst-foundation-workspace
---

# Rust Revision: DST Framework for ewe_platform foundation_core

## Overview

This document provides a complete implementation of the Deterministic Simulation Testing (DST) framework for ewe_platform's foundation_core. The implementation follows Hiisi's I/O dispatch pattern while providing ergonomic async abstractions.

## Workspace Structure

```
foundation_core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── io/
│   │   ├── mod.rs              # Feature-gated re-exports
│   │   ├── raw_conn.rs         # RawConn trait
│   │   ├── tokio_conn.rs       # Production implementation
│   │   └── simulation/
│   │       ├── mod.rs
│   │       ├── kernel.rs       # SimKernel
│   │       ├── conn.rs         # SimConn implementation
│   │       ├── virtual.rs      # VirtualStream, VirtualListener
│   │       ├── network.rs      # Network simulation
│   │       └── clock.rs        # Virtual time
│   ├── net/
│   │   ├── mod.rs
│   │   ├── tcp.rs              # TCP using RawConn
│   │   ├── udp.rs              # UDP using RawConn
│   │   └── listener.rs         # Listener abstraction
│   └── time/
│       ├── mod.rs
│       └── clock.rs            # Clock abstraction
└── tests/
    └── simulation/
        ├── basic_test.rs
        └── network_test.rs
```

## Type System Design

### Core RawConn Trait

```rust
// src/io/raw_conn.rs

use std::{
    io,
    net::SocketAddr,
    time::{Duration, Instant},
};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

/// Core I/O abstraction for deterministic simulation testing.
/// 
/// This trait abstracts over network I/O operations, allowing
/// seamless switching between real I/O (tokio) and simulated
/// I/O (deterministic kernel) for testing.
///
/// # Examples
///
/// Production code:
/// ```rust
/// async fn connect_and_read<C: RawConn>() -> io::Result<()> {
///     let mut stream = C::connect("127.0.0.1:8080".parse()?).await?;
///     // ... use stream
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait RawConn: Sized + Send + Sync + 'static {
    /// TCP stream type
    type Stream: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static;
    
    /// TCP listener type  
    type Listener: futures::stream::Stream<Item = io::Result<Self::Stream>> 
        + Send + Sync + Unpin + 'static;
    
    /// UDP socket type
    type UdpSocket: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static;
    
    /// Connect to a remote address
    async fn connect(addr: SocketAddr) -> io::Result<Self::Stream>;
    
    /// Bind and create a TCP listener
    async fn bind_listen(addr: SocketAddr) -> io::Result<Self::Listener>;
    
    /// Bind a UDP socket
    async fn bind_udp(addr: SocketAddr) -> io::Result<Self::UdpSocket>;
    
    /// Get current time (virtual in simulation)
    fn now() -> Instant;
    
    /// Sleep for duration (virtual in simulation)
    async fn sleep(duration: Duration);
    
    /// Create a deadline for timeout operations
    fn deadline(after: Duration) -> Deadline {
        Deadline::new(Self::now() + after)
    }
}

/// Deadline for timeout operations
#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    inner: Instant,
}

impl Deadline {
    pub fn new(instant: Instant) -> Self {
        Self { inner: instant }
    }
    
    pub fn has_elapsed(&self) -> bool {
        RawConnImpl::now() >= self.inner
    }
    
    pub fn time_until(&self) -> Option<Duration> {
        let now = RawConnImpl::now();
        if now >= self.inner {
            None
        } else {
            Some(self.inner - now)
        }
    }
}
```

### Production Implementation (TokioConn)

```rust
// src/io/tokio_conn.rs

use super::raw_conn::{RawConn, Deadline};
use std::{
    io,
    net::SocketAddr,
    time::{Duration, Instant},
};
use async_trait::async_trait;
use tokio::net::{TcpStream, TcpListener, UdpSocket};
use tokio::io::{AsyncRead, AsyncWrite};

/// Production I/O implementation using tokio.
///
/// This is the default implementation used in production.
/// It provides real network I/O with actual sockets.
pub struct TokioConn;

#[async_trait]
impl RawConn for TokioConn {
    type Stream = TcpStream;
    type Listener = TcpListener;
    type UdpSocket = UdpSocket;
    
    async fn connect(addr: SocketAddr) -> io::Result<Self::Stream> {
        TcpStream::connect(addr).await
    }
    
    async fn bind_listen(addr: SocketAddr) -> io::Result<Self::Listener> {
        let listener = TcpListener::bind(addr).await?;
        Ok(listener)
    }
    
    async fn bind_udp(addr: SocketAddr) -> io::Result<Self::UdpSocket> {
        UdpSocket::bind(addr).await
    }
    
    fn now() -> Instant {
        Instant::now()
    }
    
    async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

// Type alias for production use
pub type RawConnImpl = TokioConn;
```

### Simulation Kernel

```rust
// src/io/simulation/kernel.rs

use bytes::{Bytes, BytesMut};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque, BinaryHeap},
    io,
    net::SocketAddr,
    rc::Rc,
    time::{Duration, Instant},
};

use super::virtual::{VirtualStream, VirtualListener, VirtualUdpSocket};
use super::network::{NetworkConfig, PacketEvent};

/// Unique connection identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId(u64);

impl ConnId {
    pub fn generate(rng: &mut ChaCha8Rng) -> Self {
        Self(rng.next_u64())
    }
}

/// Pending timer event
#[derive(Debug, Clone)]
struct Timer {
    wake_time: Instant,
    waker_id: u64,
}

impl PartialEq for Timer {
    fn eq(&self, other: &Self) -> bool {
        self.wake_time == other.wake_time && self.waker_id == other.waker_id
    }
}

impl Eq for Timer {}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse for min-heap
        other.wake_time.cmp(&self.wake_time)
    }
}

/// Virtual connection state
struct VirtualConnection {
    id: ConnId,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    /// Data received from remote, waiting to be read
    recv_buffer: BytesMut,
    /// Data sent but not yet delivered
    send_queue: VecDeque<Bytes>,
    /// Connection is closed
    closed: bool,
    /// Error state
    error: Option<io::ErrorKind>,
}

impl VirtualConnection {
    fn new(id: ConnId, local: SocketAddr, remote: SocketAddr) -> Self {
        Self {
            id,
            local_addr: local,
            remote_addr: remote,
            recv_buffer: BytesMut::new(),
            send_queue: VecDeque::new(),
            closed: false,
            error: None,
        }
    }
    
    /// Create a pair of connected endpoints
    fn paired(id: ConnId, addr_a: SocketAddr, addr_b: SocketAddr) -> (Self, Self) {
        let a_to_b = Self::new(id, addr_a, addr_b);
        let b_to_a = Self::new(id, addr_b, addr_a);
        (a_to_b, b_to_a)
    }
}

/// Simulation kernel state
pub struct SimKernel {
    /// Virtual time
    pub virtual_time: Instant,
    
    /// PRNG for determinism
    rng: ChaCha8Rng,
    
    /// Network configuration
    config: NetworkConfig,
    
    /// Registered listeners: addr -> listeners
    listeners: HashMap<SocketAddr, VecDeque<Rc<RefCell<ListenerState>>>>,
    
    /// Active connections
    connections: HashMap<ConnId, Rc<RefCell<VirtualConnection>>>,
    
    /// Pending accepts per listener addr
    pending_accepts: HashMap<SocketAddr, VecDeque<Rc<RefCell<VirtualConnection>>>>,
    
    /// Pending timers (wake up sleeping tasks)
    timers: BinaryHeap<Timer>,
    
    /// Wakers for pending operations
    wakers: HashMap<u64, std::task::Waker>,
    
    /// Next waker ID
    next_waker_id: u64,
    
    /// Packet delivery events
    pending_packets: VecDeque<PacketEvent>,
}

struct ListenerState {
    addr: SocketAddr,
    pending_connections: VecDeque<Rc<RefCell<VirtualConnection>>>,
    closed: bool,
}

impl SimKernel {
    /// Create new kernel with seed
    pub fn with_seed(seed: u64) -> Self {
        Self {
            virtual_time: Instant::now(),
            rng: ChaCha8Rng::seed_from_u64(seed),
            config: NetworkConfig::default(),
            listeners: HashMap::new(),
            connections: HashMap::new(),
            pending_accepts: HashMap::new(),
            timers: BinaryHeap::new(),
            wakers: HashMap::new(),
            next_waker_id: 0,
            pending_packets: VecDeque::new(),
        }
    }
    
    /// Create kernel with specific config
    pub fn with_config(seed: u64, config: NetworkConfig) -> Self {
        let mut kernel = Self::with_seed(seed);
        kernel.config = config;
        kernel
    }
    
    /// Get next waker ID
    fn next_waker_id(&mut self) -> u64 {
        let id = self.next_waker_id;
        self.next_waker_id += 1;
        id
    }
    
    /// Register a waker for a pending operation
    pub fn register_waker(&mut self, waker_id: u64, waker: std::task::Waker) {
        self.wakers.insert(waker_id, waker);
    }
    
    /// Remove a waker
    pub fn remove_waker(&mut self, waker_id: u64) {
        self.wakers.remove(&waker_id);
    }
    
    /// Bind a listener
    pub fn bind_listener(&mut self, addr: SocketAddr) -> io::Result<Rc<RefCell<ListenerState>>> {
        let state = Rc::new(RefCell::new(ListenerState {
            addr,
            pending_connections: VecDeque::new(),
            closed: false,
        }));
        
        self.listeners
            .entry(addr)
            .or_default()
            .push_back(state.clone());
        
        Ok(state)
    }
    
    /// Connect to a listener
    pub fn connect(&mut self, addr: SocketAddr) -> io::Result<Rc<RefCell<VirtualConnection>>> {
        // Find a listener at this address
        let listeners = self.listeners.get(&addr)
            .ok_or_else(|| io::Error::new(io::ErrorKind::ConnectionRefused, "No listener"))?;
        
        let listener = listeners.front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::ConnectionRefused, "Listener closed"))?
            .clone();
        
        // Create virtual connection pair
        let conn_id = ConnId::generate(&mut self.rng);
        let local_addr = self.random_local_addr();
        let (local_end, remote_end) = VirtualConnection::paired(conn_id, local_addr, addr);
        
        let local_rc = Rc::new(RefCell::new(local_end));
        let remote_rc = Rc::new(RefCell::new(remote_end));
        
        // Store connection
        self.connections.insert(conn_id, local_rc.clone());
        
        // Queue for accept
        listener.borrow_mut().pending_connections.push_back(remote_rc);
        
        Ok(local_rc)
    }
    
    /// Accept a connection
    pub fn accept(&mut self, listener: &Rc<RefCell<ListenerState>>) -> io::Result<Rc<RefCell<VirtualConnection>>> {
        let mut listener_ref = listener.borrow_mut();
        
        listener_ref.pending_connections.pop_front()
            .ok_or_else(|| io::Error::new(io::ErrorKind::WouldBlock, "No pending connections"))
    }
    
    /// Send data on connection
    pub fn send(&mut self, conn: &Rc<RefCell<VirtualConnection>>, data: &[u8]) -> io::Result<usize> {
        let mut conn_ref = conn.borrow_mut();
        
        if conn_ref.closed {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Connection closed"));
        }
        
        // Apply fault injection
        if self.should_drop_packet() {
            log::debug!("Dropping packet (simulated)");
            return Ok(data.len()); // Silent drop
        }
        
        let delay = if self.should_delay_packet() {
            Some(self.random_delay())
        } else {
            None
        };
        
        let data = Bytes::copy_from_slice(data);
        
        if let Some(delay) = delay {
            // Schedule delayed delivery
            let deliver_time = self.virtual_time + delay;
            self.pending_packets.push_back(PacketEvent {
                deliver_time,
                data,
                // ... would need conn reference
            });
        } else {
            // Immediate delivery to remote end
            // Find the paired connection and deliver
        }
        
        Ok(data.len())
    }
    
    /// Receive data from connection
    pub fn recv(&mut self, conn: &Rc<RefCell<VirtualConnection>>, buf: &mut [u8]) -> io::Result<usize> {
        let mut conn_ref = conn.borrow_mut();
        
        if let Some(err) = conn_ref.error {
            return Err(io::Error::new(err, "Connection error"));
        }
        
        let available = conn_ref.recv_buffer.len();
        if available == 0 {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "No data available"));
        }
        
        let to_read = available.min(buf.len());
        buf[..to_read].copy_from_slice(&conn_ref.recv_buffer[..to_read]);
        conn_ref.recv_buffer.advance(to_read);
        
        Ok(to_read)
    }
    
    /// Schedule a timer
    pub fn schedule_timer(&mut self, delay: Duration, waker_id: u64) {
        let wake_time = self.virtual_time + delay;
        self.timers.push(Timer { wake_time, waker_id });
    }
    
    /// Cancel a timer
    pub fn cancel_timer(&mut self, waker_id: u64) {
        // Remove timer with this waker_id
        // (Inefficient but works for simulation)
        self.timers.retain(|t| t.waker_id != waker_id);
    }
    
    /// Run one simulation step
    pub fn step(&mut self) {
        // Find next event (timer or packet)
        let next_timer = self.timers.peek().map(|t| t.wake_time);
        let next_packet = self.pending_packets.front().map(|p| p.deliver_time);
        
        let next_event = match (next_timer, next_packet) {
            (Some(t), Some(p)) => t.min(p),
            (Some(t), None) => t,
            (None, Some(p)) => p,
            (None, None) => return, // Nothing to do
        };
        
        // Advance virtual time
        self.virtual_time = next_event;
        
        // Process timers
        self.process_timers();
        
        // Process packets
        self.process_packets();
        
        // Wake pending tasks
        self.wake_pending();
    }
    
    fn process_timers(&mut self) {
        while let Some(timer) = self.timers.peek() {
            if timer.wake_time <= self.virtual_time {
                let timer = self.timers.pop().unwrap();
                if let Some(waker) = self.wakers.remove(&timer.waker_id) {
                    waker.wake();
                }
            } else {
                break;
            }
        }
    }
    
    fn process_packets(&mut self) {
        while let Some(packet) = self.pending_packets.front() {
            if packet.deliver_time <= self.virtual_time {
                let packet = self.pending_packets.pop_front().unwrap();
                // Deliver packet to destination
                // ...
            } else {
                break;
            }
        }
    }
    
    fn wake_pending(&mut self) {
        // Wake any tasks that are ready
        // This is simplified - real impl would track what each waker is waiting for
    }
    
    /// Generate random local address for client connections
    fn random_local_addr(&mut self) -> SocketAddr {
        let port = 30000 + self.rng.gen_range(0..30000);
        SocketAddr::from(([127, 0, 0, 1], port))
    }
    
    /// Should drop packet (based on config)
    fn should_drop_packet(&mut self) -> bool {
        self.rng.gen_bool(self.config.packet_drop_rate)
    }
    
 /// Should delay packet
    fn should_delay_packet(&mut self) -> bool {
        self.rng.gen_bool(self.config.delay_probability)
    }
    
    /// Generate random delay
    fn random_delay(&mut self) -> Duration {
        let ms = self.rng.gen_range(self.config.latency_range.clone());
        Duration::from_millis(ms)
    }
    
    /// Run simulation to completion
    pub fn run<F, Fut, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        // Use tokio runtime for async execution
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        // Set kernel in thread-local
        KERNEL.with(|k| {
            *k.borrow_mut() = Some(Rc::new(RefCell::new(self)));
        });
        
        rt.block_on(f())
    }
}

// Thread-local kernel reference
std::thread_local! {
    static KERNEL: RefCell<Option<Rc<RefCell<SimKernel>>>> = RefCell::new(None);
}

/// Get current kernel
pub fn with_kernel<F, T>(f: F) -> T
where
    F: FnOnce(&mut SimKernel) -> T,
{
    KERNEL.with(|k| {
        let mut kernel_ref = k.borrow_mut();
        let kernel = kernel_ref.as_mut().expect("No simulation kernel");
        let mut kernel_inner = kernel.borrow_mut();
        f(&mut kernel_inner)
    })
}
```

### SimConn Implementation

```rust
// src/io/simulation/conn.rs

use super::kernel::{SimKernel, with_kernel, ConnId};
use super::virtual::{VirtualStream, VirtualListener, VirtualUdpSocket};
use super::raw_conn::{RawConn, Deadline};
use std::{
    io,
    net::SocketAddr,
    time::{Duration, Instant},
};
use async_trait::async_trait;

/// Simulation I/O implementation.
///
/// Uses a virtual network kernel to provide deterministic
/// simulation of network operations.
pub struct SimConn;

#[async_trait]
impl RawConn for SimConn {
    type Stream = VirtualStream;
    type Listener = VirtualListener;
    type UdpSocket = VirtualUdpSocket;
    
    async fn connect(addr: SocketAddr) -> io::Result<Self::Stream> {
        with_kernel(|kernel| {
            let conn = kernel.connect(addr)?;
            Ok(VirtualStream::new(conn))
        })
    }
    
    async fn bind_listen(addr: SocketAddr) -> io::Result<Self::Listener> {
        with_kernel(|kernel| {
            let listener = kernel.bind_listener(addr)?;
            Ok(VirtualListener::new(listener, addr))
        })
    }
    
    async fn bind_udp(addr: SocketAddr) -> io::Result<Self::UdpSocket> {
        with_kernel(|kernel| {
            // Similar to TCP but for UDP
            Ok(VirtualUdpSocket::new(addr))
        })
    }
    
    fn now() -> Instant {
        with_kernel(|kernel| kernel.virtual_time)
    }
    
    async fn sleep(duration: Duration) {
        with_kernel(|kernel| {
            let waker_id = kernel.next_waker_id();
            kernel.schedule_timer(duration, waker_id);
            // Will be woken when timer fires
        })
    }
}

// Type alias for simulation mode
pub type RawConnImpl = SimConn;
```

### Virtual Stream Types

```rust
// src/io/simulation/virtual.rs

use super::kernel::SimKernel;
use bytes::{Bytes, BytesMut};
use std::{
    cell::RefCell,
    io,
    net::SocketAddr,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use futures::stream::{Stream, StreamExt};
use std::collections::VecDeque;

use super::kernel::{VirtualConnection, ListenerState};

/// Virtual TCP stream
pub struct VirtualStream {
    conn: Rc<RefCell<VirtualConnection>>,
    read_waker_id: Option<u64>,
    write_waker_id: Option<u64>,
}

impl VirtualStream {
    pub(super) fn new(conn: Rc<RefCell<VirtualConnection>>) -> Self {
        Self {
            conn,
            read_waker_id: None,
            write_waker_id: None,
        }
    }
    
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.conn.borrow().local_addr)
    }
    
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.conn.borrow().remote_addr)
    }
    
    pub fn shutdown(&self) -> io::Result<()> {
        self.conn.borrow_mut().closed = true;
        Ok(())
    }
}

impl AsyncRead for VirtualStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut conn = self.conn.borrow_mut();
        
        // Check for error
        if let Some(err) = conn.error {
            return Poll::Ready(Err(io::Error::new(err, "Connection error")));
        }
        
        // Check if closed
        if conn.closed && conn.recv_buffer.is_empty() {
            return Poll::Ready(Ok(())); // EOF
        }
        
        // Try to read
        let available = conn.recv_buffer.len();
        if available == 0 {
            // No data, register waker
            let waker_id = super::kernel::with_kernel(|k| k.next_waker_id());
            self.read_waker_id = Some(waker_id);
            super::kernel::with_kernel(|k| k.register_waker(waker_id, cx.waker().clone()));
            return Poll::Pending;
        }
        
        // Read available data
        let to_read = available.min(buf.remaining());
        buf.put_slice(&conn.recv_buffer[..to_read]);
        conn.recv_buffer.advance(to_read);
        
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for VirtualStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut conn = self.conn.borrow_mut();
        
        if conn.closed {
            return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "Connection closed")));
        }
        
        // Use kernel to send (with fault injection)
        let result = super::kernel::with_kernel(|kernel| {
            kernel.send(&self.conn.clone(), buf)
        });
        
        match result {
            Ok(n) => Poll::Ready(Ok(n)),
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    // Register waker
                    let waker_id = super::kernel::with_kernel(|k| k.next_waker_id());
                    self.write_waker_id = Some(waker_id);
                    super::kernel::with_kernel(|k| k.register_waker(waker_id, cx.waker().clone()));
                    Poll::Pending
                } else {
                    Poll::Ready(Err(e))
                }
            }
        }
    }
    
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(())) // No buffering in virtual stream
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.shutdown().map(Poll::Ready)
    }
}

impl Drop for VirtualStream {
    fn drop(&mut self) {
        // Clean up wakers
        if let Some(id) = self.read_waker_id {
            super::kernel::with_kernel(|k| k.remove_waker(id));
        }
        if let Some(id) = self.write_waker_id {
            super::kernel::with_kernel(|k| k.remove_waker(id));
        }
    }
}

/// Virtual TCP listener
pub struct VirtualListener {
    listener: Rc<RefCell<ListenerState>>,
    addr: SocketAddr,
}

impl VirtualListener {
    pub(super) fn new(listener: Rc<RefCell<ListenerState>>, addr: SocketAddr) -> Self {
        Self { listener, addr }
    }
    
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.addr)
    }
}

impl Stream for VirtualListener {
    type Item = io::Result<VirtualStream>;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let listener = &self.listener;
        
        match listener.borrow_mut().pending_connections.pop_front() {
            Some(conn) => {
                Poll::Ready(Some(Ok(VirtualStream::new(conn))))
            }
            None => {
                // Register waker for when connection arrives
                let waker_id = super::kernel::with_kernel(|k| k.next_waker_id());
                super::kernel::with_kernel(|k| k.register_waker(waker_id, cx.waker().clone()));
                Poll::Pending
            }
        }
    }
}

/// Virtual UDP socket
pub struct VirtualUdpSocket {
    addr: SocketAddr,
    recv_buffer: Rc<RefCell<BytesMut>>,
}

impl VirtualUdpSocket {
    pub(super) fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            recv_buffer: Rc::new(RefCell::new(BytesMut::new())),
        }
    }
}

impl AsyncRead for VirtualUdpSocket {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // UDP implementation would go here
        Poll::Pending
    }
}

impl AsyncWrite for VirtualUdpSocket {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // UDP implementation would go here
        Poll::Pending
    }
    
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
```

### Network Configuration

```rust
// src/io/simulation/network.rs

use std::{
    ops::Range,
    time::{Duration, Instant},
};
use bytes::Bytes;

/// Network simulation configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Probability of dropping a packet (0.0 - 1.0)
    pub packet_drop_rate: f64,
    
    /// Probability of delaying a packet
    pub delay_probability: f64,
    
    /// Latency range for delayed packets (ms)
    pub latency_range: Range<u64>,
    
    /// Probability of duplicating a packet
    pub duplicate_rate: f64,
    
    /// Probability of reordering packets
    pub reorder_rate: f64,
    
    /// Bandwidth limit (bytes/second), None = unlimited
    pub bandwidth_limit: Option<u64>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            packet_drop_rate: 0.0,
            delay_probability: 0.0,
            latency_range: 1..100,
            duplicate_rate: 0.0,
            reorder_rate: 0.0,
            bandwidth_limit: None,
        }
    }
}

impl NetworkConfig {
    /// Perfect network (no faults)
    pub fn perfect() -> Self {
        Self::default()
    }
    
    /// Lossy network (10% packet loss)
    pub fn lossy() -> Self {
        Self {
            packet_drop_rate: 0.1,
            ..Self::default()
        }
    }
    
    /// High latency network (100-500ms)
    pub fn high_latency() -> Self {
        Self {
            latency_range: 100..500,
            delay_probability: 1.0,
            ..Self::default()
        }
    }
    
    /// Partitioned network (all packets dropped)
    pub fn partitioned() -> Self {
        Self {
            packet_drop_rate: 1.0,
            ..Self::default()
        }
    }
}

/// Scheduled packet delivery event
pub struct PacketEvent {
    pub deliver_time: Instant,
    pub data: Bytes,
    // Would include destination connection info
}
```

### Module Re-exports

```rust
// src/io/mod.rs

//! I/O abstraction layer with deterministic simulation support.
//!
//! This module provides the [`RawConn`] trait which abstracts over
//! network I/O operations. Depending on the `simulation` feature,
//! it re-exports either the production (tokio) or simulation implementation.
//!
//! # Features
//!
//! - `simulation`: Use the deterministic simulation kernel instead of real I/O
//!
//! # Example
//!
//! ```rust,no_run
//! use foundation_core::io::RawConn;
//!
//! async fn connect_and_read<C: RawConn>() -> io::Result<()> {
//!     let mut stream = C::connect("127.0.0.1:8080".parse()?).await?;
//!     // ... use stream
//!     Ok(())
//! }
//! ```

#[cfg(feature = "simulation")]
mod simulation;

#[cfg(feature = "simulation")]
pub use simulation::*;

#[cfg(not(feature = "simulation"))]
mod tokio_conn;

#[cfg(not(feature = "simulation"))]
pub use tokio_conn::*;

mod raw_conn;

pub use raw_conn::*;
```

### Cargo.toml with Feature Gates

```toml
[package]
name = "foundation_core"
version = "0.1.0"
edition = "2021"

[features]
default = []
simulation = []

[dependencies]
async-trait = "0.1"
bytes = "1.5"
futures = "0.3"
rand = "0.8"
rand_chacha = "0.3"
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
log = "0.4"

[dev-dependencies]
tokio-test = "0.4"
proptest = "1.0"
```

## Usage Examples

### Production Code (Unchanged)

```rust
// In your application - works in both modes!
use foundation_core::io::RawConn;
use foundation_core::net::tcp;

pub struct MyService<C: RawConn> {
    _phantom: std::marker::PhantomData<C>,
}

impl<C: RawConn> MyService<C> {
    pub async fn connect_to_server(&self, addr: SocketAddr) -> io::Result<()> {
        let mut stream = C::connect(addr).await?;
        
        // Use stream with tokio AsyncRead/AsyncWrite
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        
        stream.write_all(b"Hello, server!").await?;
        
        let mut response = vec![0u8; 1024];
        let n = stream.read(&mut response).await?;
        
        println!("Received: {}", String::from_utf8_lossy(&response[..n]));
        Ok(())
    }
}
```

### Simulation Test

```rust
#[cfg(test)]
#[cfg(feature = "simulation")]
mod tests {
    use super::*;
    use foundation_core::io::simulation::{SimKernel, SimConn};
    
    #[test]
    fn test_basic_connection() {
        let mut kernel = SimKernel::with_seed(42);
        
        kernel.run(|| async {
            let server_addr = "127.0.0.1:8080".parse().unwrap();
            
            // Spawn server task
            let server = tokio::spawn(async move {
                let listener = SimConn::bind_listen(server_addr).await.unwrap();
                
                // Accept connection (using Stream trait)
                use futures::StreamExt;
                let mut stream = listener.into_future().await.0.unwrap().unwrap();
                
                // Echo server
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = vec![0u8; 1024];
                let n = stream.read(&mut buf).await.unwrap();
                stream.write_all(&buf[..n]).await.unwrap();
            });
            
            // Give server time to start
            SimConn::sleep(std::time::Duration::from_millis(10)).await;
            
            // Client connects
            let service = MyService::<SimConn>::default();
            service.connect_to_server(server_addr).await.unwrap();
            
            server.await.unwrap();
        });
    }
    
    #[test]
    fn test_with_packet_loss() {
        let config = foundation_core::io::simulation::NetworkConfig::lossy();
        let mut kernel = SimKernel::with_config(42, config);
        
        kernel.run(|| async {
            // Test handles 10% packet loss
            // ...
        });
    }
    
    #[test]
    fn test_reproducible_failure() {
        // Use seed from failed CI run to reproduce
        let seed = 12345678u64;
        let mut kernel = SimKernel::with_seed(seed);
        
        kernel.run(|| async {
            // Same seed = same execution = reproducible bug
            // ...
        });
    }
}
```

## Integration with ewe_platform

### Migration Strategy

1. **Phase 1: Define Traits**
   - Add `RawConn` trait to `foundation_core/src/io/`
   - Add `Clock` trait for time abstraction

2. **Phase 2: Implement Backends**
   - `TokioConn` for production
   - `SimConn` for simulation

3. **Phase 3: Update foundation_core**
   - Replace direct tokio usage with `RawConn` generic
   - Use `Clock` trait for time operations

4. **Phase 4: Add Tests**
   - Simulation tests for critical paths
   - Property-based tests with fault injection

### Testing Distributed Protocols

```rust
#[cfg(test)]
#[cfg(feature = "simulation")]
mod consensus_tests {
    use foundation_core::io::simulation::{SimKernel, MultiNodeSim};
    
    #[test]
    fn test_consensus_with_partitions() {
        let mut sim = MultiNodeSim::new(5, 42); // 5 nodes
        
        // Run Raft/Paxos consensus
        sim.run(|nodes| async move {
            // Elect leader
            let leader = nodes[0].elect_leader().await;
            
            // Propose value
            leader.propose(42).await;
            
            // Verify all nodes agree
            for node in nodes {
                assert_eq!(node.get_value().await, Some(42));
            }
        });
    }
    
    #[test]
    fn test_consensus_under_partition() {
        let mut sim = MultiNodeSim::new(5, 42);
        
        sim.run(|nodes| async move {
            // Partition network
            sim.partition(NodeId(0), NodeId(1));
            sim.partition(NodeId(0), NodeId(2));
            
            // Minority partition should not commit
            // Majority should still make progress
            
            // ... verify safety property
        });
    }
}
```

