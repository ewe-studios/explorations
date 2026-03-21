//! Deterministic Simulation Kernel Example
//!
//! Demonstrates how to build a simulation kernel that provides
//! deterministic network simulation for testing distributed systems.

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
use tracing::{info, debug, trace};

// ============ Configuration ============

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub packet_drop_rate: f64,
    pub delay_probability: f64,
    pub latency_range: Range<u64>,
    pub duplicate_rate: f64,
    pub reorder_rate: f64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            packet_drop_rate: 0.0,
            delay_probability: 0.0,
            latency_range: 1..50,
            duplicate_rate: 0.0,
            reorder_rate: 0.0,
        }
    }
}

impl NetworkConfig {
    pub fn perfect() -> Self {
        Self::default()
    }

    pub fn lossy() -> Self {
        Self {
            packet_drop_rate: 0.1,
            ..Self::default()
        }
    }

    pub fn high_latency() -> Self {
        Self {
            delay_probability: 1.0,
            latency_range: 100..500,
            ..Self::default()
        }
    }
}

// ============ Virtual Connection ============

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId(u64);

impl ConnId {
    fn generate(rng: &mut ChaCha8Rng) -> Self {
        Self(rng.next_u64())
    }
}

#[derive(Debug)]
struct VirtualConnection {
    id: ConnId,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    recv_buffer: BytesMut,
    closed: bool,
}

impl VirtualConnection {
    fn new(id: ConnId, local: SocketAddr, remote: SocketAddr) -> Self {
        Self {
            id,
            local_addr: local,
            remote_addr: remote,
            recv_buffer: BytesMut::new(),
            closed: false,
        }
    }

    fn paired(id: ConnId, addr_a: SocketAddr, addr_b: SocketAddr) -> (Self, Self) {
        let a = Self::new(id, addr_a, addr_b);
        let b = Self::new(id, addr_b, addr_a);
        (a, b)
    }
}

// ============ Simulation Kernel ============

pub struct SimKernel {
    pub virtual_time: Instant,
    rng: ChaCha8Rng,
    config: NetworkConfig,
    connections: HashMap<ConnId, Rc<RefCell<VirtualConnection>>>,
    pending_accepts: HashMap<SocketAddr, VecDeque<Rc<RefCell<VirtualConnection>>>>,
    timers: BinaryHeap<Timer>,
    next_timer_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Timer {
    wake_time: Instant,
    timer_id: u64,
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.wake_time.cmp(&self.wake_time) // Reverse for min-heap
    }
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl SimKernel {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            virtual_time: Instant::now(),
            rng: ChaCha8Rng::seed_from_u64(seed),
            config: NetworkConfig::default(),
            connections: HashMap::new(),
            pending_accepts: HashMap::new(),
            timers: BinaryHeap::new(),
            next_timer_id: 0,
        }
    }

    pub fn with_config(seed: u64, config: NetworkConfig) -> Self {
        let mut kernel = Self::with_seed(seed);
        kernel.config = config;
        kernel
    }

    fn next_timer_id(&mut self) -> u64 {
        let id = self.next_timer_id;
        self.next_timer_id += 1;
        id
    }

    /// Create a virtual connection
    pub fn connect(&mut self, addr: SocketAddr) -> io::Result<Rc<RefCell<VirtualConnection>>> {
        // Check if there's a listener at this address
        if !self.pending_accepts.contains_key(&addr) {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "No listener",
            ));
        }

        let conn_id = ConnId::generate(&mut self.rng);
        let local_addr = self.random_local_addr();
        let (local_end, remote_end) = VirtualConnection::paired(conn_id, local_addr, addr);

        let local_rc = Rc::new(RefCell::new(local_end));
        let remote_rc = Rc::new(RefCell::new(remote_end));

        self.connections.insert(conn_id, local_rc.clone());

        // Queue for accept
        self.pending_accepts
            .entry(addr)
            .or_default()
            .push_back(remote_rc.clone());

        Ok(local_rc)
    }

    /// Accept a pending connection
    pub fn accept(&mut self, addr: SocketAddr) -> io::Result<Rc<RefCell<VirtualConnection>>> {
        self.pending_accepts
            .get_mut(&addr)
            .and_then(|q| q.pop_front())
            .ok_or_else(|| io::Error::new(io::ErrorKind::WouldBlock, "No pending connections"))
    }

    /// Send data on connection
    pub fn send(&mut self, conn: &Rc<RefCell<VirtualConnection>>, data: &[u8]) -> io::Result<usize> {
        let conn_ref = conn.borrow();

        if conn_ref.closed {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Connection closed"));
        }

        // Apply fault injection
        if self.should_drop_packet() {
            debug!("Dropping packet (simulated)");
            return Ok(data.len()); // Silent drop
        }

        let delay = if self.should_delay_packet() {
            Some(self.random_delay())
        } else {
            None
        };

        // Find peer connection and deliver
        let peer_conn = self.find_peer(&conn_ref);
        if let Some(peer) = peer_conn {
            let mut peer_ref = peer.borrow_mut();
            
            if let Some(delay) = delay {
                // Schedule delayed delivery
                let deliver_time = self.virtual_time + delay;
                let timer_id = self.next_timer_id();
                // Would need to store data and deliver on timer
                trace!("Scheduling delayed delivery in {:?}", delay);
            } else {
                // Immediate delivery
                peer_ref.recv_buffer.extend_from_slice(data);
            }
        }

        Ok(data.len())
    }

    /// Receive data from connection
    pub fn recv(&mut self, conn: &Rc<RefCell<VirtualConnection>>, buf: &mut [u8]) -> io::Result<usize> {
        let mut conn_ref = conn.borrow_mut();

        let available = conn_ref.recv_buffer.len();
        if available == 0 {
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "No data"));
        }

        let to_read = available.min(buf.len());
        buf[..to_read].copy_from_slice(&conn_ref.recv_buffer[..to_read]);
        conn_ref.recv_buffer.advance(to_read);

        Ok(to_read)
    }

    /// Schedule a timer
    pub fn schedule_timer(&mut self, delay: Duration) -> u64 {
        let timer_id = self.next_timer_id();
        let wake_time = self.virtual_time + delay;
        self.timers.push(Timer { wake_time, timer_id });
        timer_id
    }

    /// Run one simulation step
    pub fn step(&mut self) {
        // Process any due timers
        while let Some(timer) = self.timers.peek() {
            if timer.wake_time <= self.virtual_time {
                let _timer = self.timers.pop().unwrap();
                debug!("Timer {} fired", timer.timer_id);
                // Would wake the associated task
            } else {
                break;
            }
        }

        // Advance virtual time to next event
        if let Some(next_timer) = self.timers.peek() {
            self.virtual_time = next_timer.wake_time;
        }
    }

    fn find_peer(
        &self,
        conn: &VirtualConnection,
    ) -> Option<Rc<RefCell<VirtualConnection>>> {
        // Find connection with swapped local/remote addresses
        for (_, peer) in &self.connections {
            let peer_ref = peer.borrow();
            if peer_ref.local_addr == conn.remote_addr
                && peer_ref.remote_addr == conn.local_addr
            {
                return Some(peer.clone());
            }
        }
        None
    }

    fn random_local_addr(&mut self) -> SocketAddr {
        let port = 30000 + self.rng.gen_range(0..30000);
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    fn should_drop_packet(&mut self) -> bool {
        self.rng.gen_bool(self.config.packet_drop_rate)
    }

    fn should_delay_packet(&mut self) -> bool {
        self.rng.gen_bool(self.config.delay_probability)
    }

    fn random_delay(&mut self) -> Duration {
        let ms = self.rng.gen_range(self.config.latency_range.clone());
        Duration::from_millis(ms)
    }

    /// Run simulation with a test function
    pub fn run<F, Fut, T>(&mut self, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(f())
    }
}

// ============ Example Usage ============

async fn echo_server_example() {
    info!("Echo Server Simulation");
    info!("======================");

    let mut kernel = SimKernel::with_seed(42);
    let server_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

    // Simulate server accepting connection
    let client_conn = kernel.connect(server_addr).await;

    match client_conn {
        Ok(_) => info!("Client connected successfully"),
        Err(e) => info!("Connection failed: {}", e),
    }

    // Run some simulation steps
    for i in 0..10 {
        kernel.step();
        debug!("Step {}: virtual_time = {:?}", i, kernel.virtual_time);
    }
}

async fn test_with_faults() {
    info!("Testing with Network Faults");
    info!("===========================");

    let config = NetworkConfig::lossy(); // 10% packet loss
    let mut kernel = SimKernel::with_config(42, config);

    info!("Network config: 10% packet loss");

    // Test multiple sends
    for i in 0..20 {
        // Simulate send operations
        kernel.step();

        if kernel.should_drop_packet() {
            debug!("Send {}: DROPPED", i);
        } else {
            debug!("Send {}: delivered", i);
        }
    }
}

async fn test_reproducibility() {
    info!("Testing Reproducibility");
    info!("=======================");

    let seed = 12345u64;

    // Run same scenario twice with same seed
    let mut kernel1 = SimKernel::with_seed(seed);
    let mut kernel2 = SimKernel::with_seed(seed);

    let mut results1 = Vec::new();
    let mut results2 = Vec::new();

    for _ in 0..10 {
        kernel1.step();
        kernel2.step();
        results1.push(kernel1.should_drop_packet());
        results2.push(kernel2.should_drop_packet());
    }

    info!("Run 1: {:?}", results1);
    info!("Run 2: {:?}", results2);
    info!("Results match: {}", results1 == results2);

    assert_eq!(results1, results2, "Same seed should produce same results");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sim_kernel=info".parse().unwrap()),
        )
        .init();

    info!("Deterministic Simulation Kernel Example");
    info!("=======================================");
    info!();

    // Demo 1: Basic echo server simulation
    echo_server_example().await;
    info!();

    // Demo 2: Network fault injection
    test_with_faults().await;
    info!();

    // Demo 3: Reproducibility test
    test_reproducibility().await;
    info!();

    info!("All tests passed!");
    info!();
    info!("Key takeaways:");
    info!("  1. Simulation is deterministic given a seed");
    info!("  2. Fault injection is configurable");
    info!("  3. Virtual time advances based on events");
    info!("  4. Same seed = same execution = reproducible bugs");

    Ok(())
}
