---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/dst-with-hissi
explored_at: 2026-03-22
sources:
  - Turso Hiisi: /home/darkvoid/Boxxed/@formulas/src.rust/src.turso/hiisi
  - TigerBeetle I/O Dispatch: https://tigerbeetle.com/blog/a-friendly-abstraction-over-iouring-and-kqueue
  - libxev: https://github.com/mitchellh/libxev
---

# Deterministic Simulation Testing (DST) with RawConn Abstraction

## Overview

This exploration details how to build a production-grade Deterministic Simulation Testing (DST) framework for Rust networking code, inspired by Turso's Hiisi project and TigerBeetle's I/O dispatch architecture. The goal is to create abstractions that allow seamless swapping between production I/O (real sockets) and simulation I/O (deterministic mock networking) for testing distributed systems.

## What is Deterministic Simulation Testing?

DST is a testing methodology where you run your production code inside a controlled, deterministic environment that simulates external dependencies (network, disk, time). Key properties:

```
┌─────────────────────────────────────────────────────────────────┐
│              Deterministic Simulation Testing                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Production Code (unchanged)                                    │
│       │                                                          │
│       │ depends on                                               │
│       ▼                                                          │
│  ┌─────────────────┐                                            │
│  │  Abstraction    │ ◄── Trait-based I/O interface              │
│  │  (RawConn)      │                                            │
│  └────────┬────────┘                                            │
│           │                                                      │
│     ┌─────┴─────┐                                               │
│     │           │                                                │
│     ▼           ▼                                                │
│  ┌──────┐   ┌──────────┐                                        │
│  │Real  │   │Simulation│                                        │
│  │IO    │   │Kernel    │                                        │
│  │      │   │          │                                        │
│  │ - TCP│   │ - Virtual│                                        │
│  │ - UDP│   │   Nets   │                                        │
│  │ - UDP│   │ - Determin│                                        │
│  │ Listen│  │   PRNG   │                                        │
│  └──────┘   └──────────┘                                        │
│                                                                  │
│  Same code runs in both modes                                    │
│  Simulation is reproducible with seed                            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Why DST Matters

| Problem | Traditional Testing | DST |
|---------|-------------------|-----|
| Network partitions | Hard to reproduce | Simulated deterministically |
| Race conditions | Flaky tests | Reproducible with seed |
| Timing issues | Depends on system load | Controlled virtual time |
| Distributed bugs | Non-deterministic | Replay with seed |

## Hiisi Architecture Analysis

Hiisi (Turso's libSQL server) implements DST following TigerBeetle's architecture. Let's break down how it works.

### Core Insight: I/O as Event Dispatch

Instead of using blocking or async I/O directly, Hiisi uses a **completion-based I/O dispatcher**:

```rust
// Traditional async (tokio):
async fn handle_connection(stream: TcpStream) { ... }

// Hiisi approach:
fn on_accept(io: &mut IO, server_sock: Rc<Socket>, ..., client_sock: Rc<Socket>, ...) {
    io.accept(server_sock, addr, on_accept);  // Re-arm accept
    io.recv(client_sock, on_recv);            // Queue recv
}

fn on_recv(io: &mut IO, sock: Rc<Socket>, buf: &[u8], n: usize) {
    io.send(sock, response, n, on_send);      // Queue send
}

fn on_send(io: &mut IO, sock: Rc<Socket>, n: usize) {
    io.recv(sock, on_recv);                   // Re-arm recv
}

// Main loop:
loop {
    io.run_once();  // Process completions
}
```

### Key Design Patterns

#### 1. Callback-Based I/O Completion

```rust
// From hiisi-server/src/io/generic.rs (production I/O with polling)
pub fn accept(
    &mut self,
    server_sock: Rc<socket2::Socket>,
    server_addr: socket2::SockAddr,
    cb: AcceptCallback<C>,  // Callback when accept completes
) {
    // Register with poller for readability
    let key = self.get_key();
    unsafe {
        self.poller.add(server_sock, Event::readable(key)).unwrap();
    }
    // Store completion with callback
    self.submissions.insert(key, Completion::Accept {
        server_sock, server_addr, cb
    });
}

// When poller detects event:
fn complete(self, io: &mut IO<C>) {
    match self {
        Completion::Accept { server_sock, server_addr, cb } => {
            let (sock, sock_addr) = server_sock.accept().unwrap();
            // Invoke callback with result
            cb(io, server_sock, server_addr, Rc::new(sock), sock_addr);
        }
        // ...
    }
}
```

#### 2. Feature-Gated I/O Implementations

```rust
// hiisi-server/src/io/mod.rs
#[cfg(not(feature = "simulation"))]
mod generic;  // Production I/O with polling

#[cfg(not(feature = "simulation"))]
pub use generic::IO;

#[cfg(feature = "simulation")]
mod simulation;  // Deterministic simulation

#[cfg(feature = "simulation")]
pub use simulation::IO;
```

Same code, different I/O backends based on feature flag.

#### 3. Simulation I/O with Virtual Connections

```rust
// From hiisi-server/src/io/simulation.rs
pub struct IO<C> {
    context: C,
    completions: RefCell<VecDeque<Completion<C>>>,
    listener_sockets: HashMap<i32, Rc<socket2::Socket>>,
    conn_sockets: HashMap<i32, Socket>,  // Virtual connections
    accept_listeners: HashMap<sockAddr, (Rc<Socket>, AcceptCallback<C>)>,
    recv_listeners: HashMap<i32, (Rc<Socket>, RecvCallback<C>)>,
}

// Virtual connection pairs local and remote sockets
struct Socket {
    local_sock: Rc<socket2::Socket>,
    remote_sock: Rc<socket2::Socket>,  // The "other end"
    xmit_queue: RefCell<VecDeque<Bytes>>,  // Pending sends
}

// Connect synthesizes both ends of a connection
pub fn connect(
    &mut self,
    local_sock: Rc<socket2::Socket>,
    remote_addr: socket2::SockAddr,
    cb: ConnectCallback<C>,
) {
    // Find the listener at remote_addr
    let (accept_sock, accept_cb) = self.accept_listeners.remove(&remote_addr).unwrap();
    
    // Create remote socket (simulated kernel creates peer)
    let remote_sock = Rc::new(socket2::Socket::new(Domain::IPV4, Type::STREAM, None).unwrap());
    
    // Register virtual connection
    self.register_socket(remote_sock.clone(), local_sock.clone());
    
    // Fire accept callback on server side
    let c = Completion::Accept {
        server_sock: accept_sock.clone(),
        server_addr: remote_addr.clone(),
        client_sock: remote_sock.clone(),
        client_addr: local_addr.clone(),
        cb: accept_cb,
    };
    self.enqueue(c);
    
    // Fire connect callback on client side
    let c = Completion::Connect {
        sock: local_sock.clone(),
        addr: local_addr,
        cb,
    };
    self.enqueue(c);
}
```

#### 4. Send/Receive via Transmission Queues

```rust
// Send queues data for delivery
pub fn send(&mut self, sock: Rc<socket2::Socket>, buf: Bytes, n: usize, cb: SendCallback<C>) {
    let socket = self.conn_sockets.get(&sockfd).unwrap();
    socket.xmit_queue.borrow_mut().push_back(buf.clone());
    
    let c = Completion::Send { sock, buf, n, cb };
    self.enqueue(c);
}

// Flush transmit queues to receive buffers
fn flush_xmit_queues(&mut self) {
    for (sockfd, socket) in self.conn_sockets.iter_mut() {
        let mut xmit_queue = socket.xmit_queue.borrow_mut();
        let remote_sockfd = socket.remote_sock.as_raw_fd();
        
        // Find recv listener for remote end
        let (recv_socket, cb) = self.recv_listeners.remove(&remote_sockfd).unwrap();
        
        // Deliver each buffered message
        while let Some(buf) = xmit_queue.pop_front() {
            let c = Completion::Recv {
                sock: recv_socket.clone(),
                buf,
                cb,
            };
            completions.push(c);
        }
    }
}
```

### How Simulation Achieves Determinism

```rust
// From hiisi-simulator/src/main.rs
fn main() {
    // Get seed from environment or generate randomly
    let seed = std::env::var("SEED")
        .map(|s| s.parse::<u64>().unwrap())
        .unwrap_or_else(|| rand::thread_rng().next_u64());

    log::info!("Starting simulation with seed {}", seed);

    // Seed the PRNG
    let rng = ChaCha8Rng::seed_from_u64(seed);
    let user_data = UserData { rng: RefCell::new(rng) };
    
    // ... setup ...
    
    // Main simulation loop
    loop {
        io.run_once();  // Process all completions
    }
}

// Use RNG for probabilistic fault injection
fn gen_perform_client_req_fault(ctx: &Context) -> PerformClientReqFault {
    let mut rng = ctx.user_data.rng.borrow_mut();
    if rng.gen_bool(0.9) {
        PerformClientReqFault::Normal  // 90% normal
    } else {
        PerformClientReqFault::Fuzz    // 10% fuzzed
    }
}
```

**Key insight**: The PRNG is seeded once, so every "random" decision is deterministic given the seed. Same seed = same execution.

## RawConn Abstraction Design

To integrate this approach into ewe_platform's foundation_core, we need a trait-based abstraction that:

1. Works with standard tokio/std networking in production
2. Can swap in a simulation kernel for testing
3. Supports TCP, UDP, Unix sockets, and listeners
4. Maintains type safety and ergonomics

### Trait Design

```rust
/// Core I/O operations for deterministic simulation
#[async_trait]
pub trait RawConn: Sized + Send + Sync {
    type Stream: AsyncRead + AsyncWrite + Send + Sync + Unpin;
    type Listener: Stream<Item = std::io::Result<Self::Stream>> + Send + Sync;
    type UdpSocket: AsyncRead + AsyncWrite + Send + Sync + Unpin;
    
    /// Connect to a remote address
    async fn connect(addr: SocketAddr) -> io::Result<Self::Stream>;
    
    /// Bind and listen on an address
    async fn bind_listen(addr: SocketAddr) -> io::Result<Self::Listener>;
    
    /// Bind a UDP socket
    async fn bind_udp(addr: SocketAddr) -> io::Result<Self::UdpSocket>;
    
    /// Get current time (for virtual time in simulation)
    fn now() -> Instant;
    
    /// Sleep for a duration (virtual in simulation)
    async fn sleep(duration: Duration);
}
```

### Production Implementation (tokio)

```rust
pub struct TokioConn;

impl RawConn for TokioConn {
    type Stream = tokio::net::TcpStream;
    type Listener = tokio::net::TcpListener;
    type UdpSocket = tokio::net::UdpSocket;
    
    async fn connect(addr: SocketAddr) -> io::Result<Self::Stream> {
        tokio::net::TcpStream::connect(addr).await
    }
    
    async fn bind_listen(addr: SocketAddr) -> io::Result<Self::Listener> {
        tokio::net::TcpListener::bind(addr).await
    }
    
    async fn bind_udp(addr: SocketAddr) -> io::Result<Self::UdpSocket> {
        tokio::net::UdpSocket::bind(addr).await
    }
    
    fn now() -> Instant {
        Instant::now()
    }
    
    async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
```

### Simulation Implementation (deterministic kernel)

```rust
pub struct SimConn {
    kernel: Rc<RefCell<SimKernel>>,
}

pub struct SimKernel {
    virtual_time: Instant,
    rng: ChaCha8Rng,
    // Virtual network state
    listeners: HashMap<SocketAddr, ListenerHandle>,
    connections: HashMap<ConnId, VirtualConn>,
    pending_accepts: HashMap<SocketAddr, VecDeque<VirtualStream>>,
    pending_recvs: HashMap<ConnId, VecDeque<Bytes>>,
    // ...
}

impl RawConn for SimConn {
    type Stream = VirtualStream;
    type Listener = VirtualListener;
    type UdpSocket = VirtualUdpSocket;
    
    async fn connect(addr: SocketAddr) -> io::Result<Self::Stream> {
        // In simulation, connect synthesizes both ends
        let kernel = self.kernel.borrow();
        kernel.connect(addr)
    }
    
    async fn bind_listen(addr: SocketAddr) -> io::Result<Self::Listener> {
        let mut kernel = self.kernel.borrow_mut();
        kernel.bind_listener(addr)
    }
    
    fn now() -> Instant {
        // Return virtual time, not real time
        KERNEL.with(|k| k.borrow().virtual_time)
    }
    
    async fn sleep(duration: Duration) {
        // Register wake-up event in virtual time
        KERNEL.with(|k| {
            let wake_time = k.borrow().virtual_time + duration;
            // ... register timer
        })
    }
}
```

### Virtual Network Kernel

```rust
pub struct SimKernel {
    /// Virtual time (advances deterministically)
    pub virtual_time: Instant,
    
    /// PRNG for non-determinism simulation
    pub rng: ChaCha8Rng,
    
    /// Registered listeners
    listeners: HashMap<SocketAddr, ListenerConfig>,
    
    /// Active connections (bidirectional)
    connections: HashMap<ConnId, VirtualConnection>,
    
    /// Pending timers
    timers: BinaryHeap<Timer>,
    
    /// Network configuration
    config: NetworkConfig,
}

impl SimKernel {
    /// Create a virtual connection between two endpoints
    pub fn connect(&mut self, addr: SocketAddr) -> io::Result<VirtualStream> {
        let listener = self.listeners.get(&addr)
            .ok_or(io::ErrorKind::ConnectionRefused)?;
        
        // Create bidirectional virtual connection
        let conn_id = ConnId::generate(&mut self.rng);
        let (local_end, remote_end) = VirtualConnection::paired(conn_id);
        
        // Queue connection for accept
        self.pending_accepts.entry(addr)
            .or_default()
            .push_back(remote_end);
        
        Ok(local_end)
    }
    
    /// Accept a pending connection
    pub fn accept(&mut self, addr: SocketAddr) -> io::Result<VirtualStream> {
        self.pending_accepts.get_mut(&addr)
            .and_then(|q| q.pop_front())
            .ok_or(io::ErrorKind::WouldBlock)
    }
    
    /// Send data on virtual connection
    pub fn send(&mut self, conn_id: ConnId, data: &[u8]) -> io::Result<usize> {
        let conn = self.connections.get_mut(&conn_id)
            .ok_or(io::ErrorKind::BrokenPipe)?;
        
        // Simulate network conditions
        if self.should_drop_packet() {
            return Ok(data.len());  // Silent drop
        }
        
        if self.should_delay_packet() {
            let delay = self.random_delay();
            self.schedule_delivery(conn_id, data, delay);
        } else {
            // Immediate delivery
            conn.remote_recv_buffer.extend_from_slice(data);
        }
        
        Ok(data.len())
    }
    
    /// Receive data from virtual connection
    pub fn recv(&mut self, conn_id: ConnId, buf: &mut [u8]) -> io::Result<usize> {
        let conn = self.connections.get_mut(&conn_id)
            .ok_or(io::ErrorKind::NotConnected)?;
        
        let available = conn.local_recv_buffer.len();
        if available == 0 {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        
        let to_read = available.min(buf.len());
        buf[..to_read].copy_from_slice(&conn.local_recv_buffer[..to_read]);
        conn.local_recv_buffer.drain(..to_read);
        
        Ok(to_read)
    }
    
    /// Deterministic packet drop decision
    fn should_drop_packet(&mut self) -> bool {
        self.rng.gen_bool(self.config.packet_drop_rate)
    }
    
    /// Deterministic delay calculation
    fn random_delay(&mut self) -> Duration {
        let ms = self.rng.gen_range(self.config.latency_range.clone());
        Duration::from_millis(ms)
    }
    
    /// Run simulation step
    pub fn step(&mut self) {
        // Advance virtual time
        let next_event = self.next_event_time();
        self.virtual_time = next_event;
        
        // Process timers
        self.process_timers();
        
        // Deliver delayed packets
        self.deliver_pending();
        
        // Wake sleeping tasks
        self.wake_sleepers();
    }
}
```

## Integration with ewe_platform foundation_core

### Current Structure

Looking at foundation_core, we need to:

1. Add `RawConn` trait to `foundation_core/src/io/`
2. Implement `TokioConn` for production
3. Implement `SimConn` with `SimKernel` for testing
4. Create feature gate (`simulation`) to swap implementations
5. Build simulation test harness

### Proposed Directory Structure

```
foundation_core/
├── src/
│   ├── io/
│   │   ├── mod.rs           # Re-export based on feature
│   │   ├── raw_conn.rs      # RawConn trait definition
│   │   ├── tokio_conn.rs    # Production tokio impl
│   │   └── simulation/
│   │       ├── mod.rs       # Simulation I/O module
│   │       ├── kernel.rs    # SimKernel
│   │       ├── conn.rs      # SimConn implementation
│   │       ├── virtual.rs   # VirtualStream, VirtualListener
│   │       └── network.rs   # Network config, fault injection
│   ├── net/
│   │   ├── tcp.rs           # TCP abstractions using RawConn
│   │   ├── udp.rs           # UDP abstractions using RawConn
│   │   └── listener.rs      # Listener abstractions
│   └── time/
│       ├── mod.rs           # Time abstractions
│       └── clock.rs         # RealClock vs SimClock
├── Cargo.toml               # Feature: "simulation"
└── tests/
    └── simulation/
        ├── basic_test.rs    # Basic simulation tests
        └── network_test.rs  # Network partition tests
```

### Usage Pattern

```rust
// In your application code (unchanged between modes):
use foundation_core::io::RawConn;
use foundation_core::net::tcp::TcpStream;

async fn handle_client<C: RawConn>() {
    let mut stream = C::connect(server_addr).await?;
    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await?;
    // ...
}

// In production (default):
// cargo run -> uses TokioConn

// In simulation tests:
// cargo test --features simulation -> uses SimConn
```

### Test Harness

```rust
#[cfg(test)]
#[cfg(feature = "simulation")]
mod simulation_tests {
    use foundation_core::io::simulation::{SimKernel, SimConn};
    use foundation_core::net::tcp;
    
    #[test]
    fn test_basic_connection() {
        let kernel = SimKernel::with_seed(42);
        
        kernel.run(|conn| async move {
            // Spawn server
            let server_handle = tokio::spawn(async move {
                let listener = SimConn::bind_listen(addr).await?;
                let stream = listener.accept().await?;
                handle_server(stream).await
            });
            
            // Spawn client
            let client_handle = tokio::spawn(async move {
                let stream = SimConn::connect(addr).await?;
                handle_client(stream).await
            });
            
            // Run to completion
            let (server_result, client_result) = 
                tokio::join!(server_handle, client_handle);
            
            assert!(server_result.is_ok());
            assert!(client_result.is_ok());
        });
    }
    
    #[test]
    fn test_network_partition() {
        let mut kernel = SimKernel::with_seed(42);
        kernel.config.packet_drop_rate = 1.0;  // Drop ALL packets
        
        kernel.run(|conn| async move {
            // Test should handle partition gracefully
            // ...
        });
    }
    
    #[test]
    fn test_reproducible_failure() {
        // Find a bug, get the seed, reproduce exactly
        let seed = 12345678;  // Seed from failed CI run
        let kernel = SimKernel::with_seed(seed);
        
        // Same seed = same execution = reproducible bug
        kernel.run(|conn| async move {
            // Trigger bug
        });
    }
}
```

## DST for Distributed Systems

### Simulating Multiple Nodes

```rust
pub struct MultiNodeSim {
    nodes: HashMap<NodeId, NodeSim>,
    network: NetworkSim,
    virtual_time: Instant,
}

pub struct NodeSim {
    id: NodeId,
    kernel: SimKernel,
    handle: Option<NodeHandle>,
}

pub struct NetworkSim {
    links: HashMap<(NodeId, NodeId), LinkConfig>,
    partitions: HashSet<(NodeId, NodeId)>,  // Currently partitioned pairs
}

impl MultiNodeSim {
    /// Create simulation with N nodes
    pub fn new(node_count: usize, seed: u64) -> Self {
        let mut nodes = HashMap::new();
        for i in 0..node_count {
            let node_kernel = SimKernel::with_seed(seed ^ i as u64);
            nodes.insert(NodeId(i), NodeSim::new(node_kernel));
        }
        
        Self {
            nodes,
            network: NetworkSim::default(),
            virtual_time: Instant::now(),
        }
    }
    
    /// Inject network partition
    pub fn partition(&mut self, a: NodeId, b: NodeId) {
        self.network.partitions.insert((a, b));
        self.network.partitions.insert((b, a));
    }
    
    /// Heal network partition
    pub fn heal(&mut self, a: NodeId, b: NodeId) {
        self.network.partitions.remove(&(a, b));
        self.network.partitions.remove(&(b, a));
    }
    
    /// Run simulation step
    pub fn step(&mut self) {
        // Process each node
        for (id, node) in &mut self.nodes {
            // Deliver pending messages (if not partitioned)
            for (from_id, msg) in node.incoming.drain(..) {
                if !self.network.is_partitioned(*id, from_id) {
                    node.kernel.deliver(msg);
                }
            }
            
            // Run node's I/O loop
            node.kernel.step();
        }
        
        // Advance virtual time
        self.virtual_time += STEP_DURATION;
    }
}
```

### Fault Injection

```rust
pub struct FaultConfig {
    /// Packet drop probability (0.0 - 1.0)
    pub packet_drop_rate: f64,
    
    /// Latency range in milliseconds
    pub latency_range: Range<u64>,
    
    /// Bandwidth limit in bytes/second
    pub bandwidth_limit: Option<u64>,
    
    /// Duplicate packet probability
    pub duplicate_rate: f64,
    
    /// Reorder probability
    pub reorder_rate: f64,
    
    /// Corrupt packet probability
    pub corruption_rate: f64,
}

pub enum FaultEvent {
    /// Drop packet silently
    Drop,
    
    /// Delay packet
    Delay(Duration),
    
    /// Duplicate packet
    Duplicate,
    
    /// Reorder (deliver later packet first)
    Reorder,
    
    /// Corrupt packet contents
    Corrupt,
    
    /// Kill connection
    KillConn,
    
    /// Partition network
    Partition(NodeId, NodeId),
    
    /// Node crash
    Crash(NodeId),
    
    /// Node restart
    Restart(NodeId),
}

impl SimKernel {
    /// Inject fault based on configuration
    pub fn inject_fault(&mut self, fault: FaultEvent) {
        match fault {
            FaultEvent::Drop => {
                // Already handled in should_drop_packet()
            }
            FaultEvent::Delay(d) => {
                // Schedule delayed delivery
            }
            FaultEvent::Partition(a, b) => {
                // Update partition state
            }
            FaultEvent::Crash(node) => {
                // Stop node processing
            }
            // ...
        }
    }
    
    /// Generate deterministic fault schedule
    pub fn generate_fault_schedule(&mut self, duration: Duration) -> Vec<(Instant, FaultEvent)> {
        let mut schedule = Vec::new();
        let mut current = self.virtual_time;
        
        while current < self.virtual_time + duration {
            // Use PRNG to determine next fault
            match self.rng.gen_range(0..100) {
                0..=5 => {
                    // 5% chance of partition
                    let a = self.random_node();
                    let b = self.random_other_node(a);
                    schedule.push((current, FaultEvent::Partition(a, b)));
                }
                6..=20 => {
                    // 15% chance of latency spike
                    schedule.push((current, FaultEvent::Delay(Duration::from_secs(5))));
                }
                // ...
                _ => {}
            }
            
            current += Duration::from_secs(self.rng.gen_range(1..10));
        }
        
        schedule
    }
}
```

### Property-Based Testing with DST

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_consistency_under_partitions(
        seed in any::<u64>(),
        partition_schedule in prop::collection::vec(any::<(u64, u64)>(), 0..10),
    ) {
        let mut sim = MultiNodeSim::new(3, seed);
        
        // Apply partition schedule
        for (time_ms, partition_pair) in partition_schedule {
            let (a, b) = partition_pair;
            sim.partition(NodeId(a as usize), NodeId(b as usize));
        }
        
        // Run consensus protocol
        sim.run_until_stable();
        
        // Verify consistency property
        let values: Vec<_> = sim.nodes.values()
            .map(|n| n.state.get_value())
            .collect();
        
        // All nodes should agree
        assert!(values.iter().all(|v| v == &values[0]));
    }
}
```

## Related Work and Inspiration

| Project | Contribution |
|---------|--------------|
| TigerBeetle | I/O dispatch abstraction, determinism |
| Hiisi (Turso) | Rust implementation, polling-based I/O |
| libxev (Mitchell Hashimoto) | Event loop design |
| FoundationDB | Deterministic simulation testing |
| Murmur (Volta) | Distributed system simulation |
| Elara | Virtual time for distributed tests |
| tokio-test | Async testing utilities (different approach) |

## Implementation Recommendations

### Phase 1: Core Abstractions

1. Define `RawConn` trait
2. Implement `TokioConn` (production)
3. Implement basic `SimConn` + `SimKernel`
4. Create feature gate infrastructure

### Phase 2: Virtual Network

1. Implement `VirtualStream`, `VirtualListener`
2. Build network simulation (buffers, queues)
3. Add basic fault injection (drop, delay)
4. Create test harness

### Phase 3: Multi-Node Simulation

1. Build `MultiNodeSim` framework
2. Implement network partitions
3. Add fault scheduling
4. Integrate with property-based testing

### Phase 4: Production Integration

1. Migrate foundation_core networking to use `RawConn`
2. Add simulation tests to CI
3. Document patterns and best practices
4. Build debugging tools (execution traces, visualization)

## Challenges and Considerations

### Async/Sync Mismatch

Hiisi uses callback-based I/O, but Rust async is prevalent. Options:

1. **Callback-based (Hiisi style)**: More control, less ergonomic
2. **Async trait (our RawConn)**: More ergonomic, needs careful implementation
3. **Hybrid**: Async trait, simulation uses `futures::executor`

### Performance Overhead

Simulation adds overhead. Mitigate with:

- Feature gates (zero cost in production)
- Compile-time selection
- Optional instrumentation

### Real Time Dependencies

Code that uses `std::time::Instant::now()` directly breaks simulation. Solution:

```rust
// Use abstraction:
pub trait Clock {
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration) -> impl Future;
}

// Inject via context:
pub struct App<C: Clock> {
    clock: C,
}
```

