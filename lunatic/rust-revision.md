---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/lunatic/
repository: https://github.com/lunatic-solutions/lunatic
explored_at: 2026-03-23
revised_at: 2026-03-23
workspace: lunatic-rust-workspace
---

# Rust Revision: Lunatic Actor-Based Wasm Runtime (Full Ecosystem)

## Overview

This document provides guidance for reproducing lunatic's actor-based WebAssembly runtime architecture in Rust at production level. Lunatic combines WebAssembly's memory isolation with Erlang-inspired actor-model concurrency to deliver fault-tolerant, massively concurrent server-side applications. The system includes: a Wasm runtime with per-process isolation, an actor system with mailboxes and supervisors, distributed clustering via QUIC/mTLS, a client SDK, web frameworks, HTTP clients, database drivers, and logging infrastructure.

## Workspace Structure

```
lunatic-rust-workspace/
  Cargo.toml                              # Workspace definition
  crates/
    runtime/                              # Core Wasm actor runtime
    runtime-api/                          # Host API traits and types
    process-api/                          # Process management host functions
    messaging-api/                        # Message passing host functions
    networking-api/                       # TCP/UDP/TLS/DNS host functions
    timer-api/                            # Timer and sleep host functions
    registry-api/                         # Named process registry host functions
    distributed-api/                      # Cross-node process management
    sqlite-api/                           # SQLite host functions
    wasi-api/                             # WASI integration
    error-api/                            # Error management host functions
    metrics-api/                          # Prometheus metrics host functions
    trap-api/                             # Wasm trap handling
    version-api/                          # Version query API
    common/                               # Shared types (signals, config, process state)
    distributed/                          # QUIC networking, node management, mTLS
    control/                              # Control plane (HTTP/Axum for node discovery)
    sdk/                                  # Client SDK (lunatic-rs equivalent)
    sdk-macros/                           # Proc macros (#[lunatic::main], derive)
    stack-switching/                      # Async wormhole (switcheroo + async yielder)
    web-framework/                        # HTTP framework (submillisecond-like)
    web-framework-macros/                 # Router proc macro
    liveview/                             # LiveView implementation
    http-client/                          # HTTP client (nightfly-like)
    db-mysql/                             # MySQL driver
    db-redis/                             # Redis driver
    logging/                              # Process-based logging
    html-templates/                       # Compile-time HTML (maud-like)
    html-templates-macros/                # html! proc macro
  examples/
    hello-world/                          # Basic example
    chat-server/                          # Telnet chat (process-per-connection)
    web-app/                              # Web application with LiveView
    distributed-cluster/                  # Multi-node example
  tools/
    installer/                            # Install scripts
    control-server/                       # Standalone control plane binary
```

## Crate 1: common (Shared Types)

### Purpose
Shared types, signals, and configuration used across all runtime crates.

### Key Types

```rust
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Unique process identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(u64);

/// Unique node identifier in a distributed cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(u64);

/// Unique environment identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnvironmentId(u64);

/// Signal types for inter-process communication.
/// Inspired by Erlang's signal system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Signal {
    /// Deliver a message to the process's mailbox.
    Message(Message),
    /// Immediately terminate the process.
    Kill,
    /// Establish a bidirectional link.
    Link {
        tag: Option<i64>,
    },
    /// Remove a bidirectional link.
    UnLink,
    /// Notification that a linked process died.
    LinkDied {
        tag: Option<i64>,
        process_id: ProcessId,
        reason: DeathReason,
    },
    /// Toggle trap_exit behavior.
    DieWhenLinkDies(bool),
    /// Start monitoring a process (unidirectional).
    Monitor {
        process_id: ProcessId,
    },
    /// Stop monitoring.
    StopMonitoring {
        process_id: ProcessId,
    },
    /// Notification that a monitored process died.
    ProcessDied {
        process_id: ProcessId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeathReason {
    /// Process completed normally.
    Normal,
    /// Process panicked or trapped.
    Failure(String),
    /// Process was killed.
    Killed,
    /// No process with this ID exists.
    NoProcess,
}

/// Opaque message data for inter-process communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Tag for selective receive.
    pub tag: Option<i64>,
    /// Serialized message data.
    pub data: Vec<u8>,
    /// Resources transferred with the message (TCP streams, etc.).
    pub resources: Vec<Resource>,
}

/// A transferable resource (file descriptor, TCP stream, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Resource {
    TcpStream(/* platform handle */),
    TlsStream(/* platform handle */),
    UdpSocket(/* platform handle */),
    Process(ProcessId),
}

/// Process configuration controlling sandboxing and permissions.
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// Maximum Wasm linear memory in bytes (default: 4GB).
    pub max_memory: u64,
    /// Compute budget in fuel units (None = unlimited).
    pub max_fuel: Option<u64>,
    /// Permission to compile new Wasm modules.
    pub can_compile_modules: bool,
    /// Permission to create child process configurations.
    pub can_create_configs: bool,
    /// Permission to spawn sub-processes.
    pub can_spawn_processes: bool,
    /// WASI preopened directories.
    pub preopened_dirs: Vec<PreopenedDir>,
    /// Command-line arguments visible to WASI.
    pub args: Vec<String>,
    /// Environment variables visible to WASI.
    pub env_vars: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct PreopenedDir {
    pub host_path: std::path::PathBuf,
    pub guest_path: String,
}
```

### Dependencies
- `serde` (serialization for distributed messaging)
- No runtime dependencies -- this is a pure types crate

## Crate 2: stack-switching (Async Wormhole)

### Purpose
Enable `.await` calls inside synchronous Wasm code by creating separate stacks and switching between them.

### Key Design

The fundamental challenge: Wasm code is compiled ahead of time (or JIT-compiled) and cannot be transformed into Rust async state machines. But lunatic host functions need to perform async I/O. The solution is stack switching.

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A future that runs a closure on a separate stack, allowing it to
/// yield back to the async executor at any point.
pub struct AsyncWormhole<Output> {
    /// The separate stack allocated for the closure.
    stack: Stack,
    /// Current execution state.
    state: WormholeState<Output>,
}

enum WormholeState<Output> {
    Running,
    Yielded,
    Finished(Output),
}

/// Passed to the closure, allowing it to yield control back to the
/// async executor (equivalent to .await).
pub struct AsyncYielder<Output> {
    /// Internal handle to the stack switcher.
    _phantom: std::marker::PhantomData<Output>,
}

impl<Output> AsyncYielder<Output> {
    /// Suspend the current stack, yielding a future to the async executor.
    /// When the future resolves, execution resumes here.
    pub fn async_suspend<F, R>(&self, future: F) -> R
    where
        F: Future<Output = R>,
    {
        // 1. Save current stack pointer
        // 2. Switch back to the async executor's stack
        // 3. The executor polls the inner future
        // 4. When the future completes, switch back to this stack
        // 5. Return the result
        todo!("Platform-specific stack switching via switcheroo")
    }
}

impl<Output> Future for AsyncWormhole<Output> {
    type Output = Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Output> {
        // Switch to the separate stack, run until it yields or finishes
        todo!()
    }
}

/// Platform-specific stack allocation and switching.
mod switcheroo {
    pub struct Stack {
        /// mmap-allocated stack memory.
        base: *mut u8,
        size: usize,
    }

    impl Stack {
        pub fn new(size: usize) -> std::io::Result<Self> {
            // mmap with MAP_STACK flag, guard page at bottom
            todo!()
        }
    }

    impl Drop for Stack {
        fn drop(&mut self) {
            // munmap
        }
    }

    /// Switch from current stack to target stack.
    /// SAFETY: target must be a valid stack with a suspended context.
    #[cfg(target_arch = "x86_64")]
    pub unsafe fn switch(from: &mut StackContext, to: &StackContext) {
        // Assembly: save registers, swap RSP, restore registers
        todo!()
    }
}
```

### Platform Support
- x86_64 Linux/macOS: Assembly-based stack switching
- aarch64: ARM64 register save/restore
- Windows: Fiber-based or SEH-compatible switching

### Dependencies
- `libc` (mmap for stack allocation)
- No other dependencies -- this is a low-level primitive

### Safety Considerations
- Stack overflow protection via guard pages (mmap with PROT_NONE)
- Stack size must be large enough for Wasm execution
- The `unsafe` boundary is well-contained in the `switcheroo` module
- Platform-specific assembly for register save/restore

## Crate 3: runtime (Core Wasm Actor Runtime)

### Purpose
The main runtime binary. Compiles Wasm modules, spawns processes, manages environments, runs the scheduler, and hosts all API crates.

### Key Architecture

```rust
use wasmtime::{Engine, Module, Store, Linker, Config as WasmConfig};
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::{ProcessId, Signal, ProcessConfig, EnvironmentId};

/// The Wasm execution engine (shared across all processes).
pub struct RuntimeEngine {
    engine: Engine,
}

impl RuntimeEngine {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = WasmConfig::new();
        config.async_support(true);
        config.consume_fuel(true);
        config.epoch_interruption(true);
        // Cranelift is the default compiler
        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }
}

/// An environment is a collection of processes with shared configuration.
/// Analogous to an Erlang application.
pub struct Environment {
    id: EnvironmentId,
    /// Compiled Wasm modules available in this environment.
    modules: HashMap<String, Module>,
    /// All live processes in this environment.
    processes: HashMap<ProcessId, ProcessHandle>,
    /// Default configuration for new processes.
    default_config: ProcessConfig,
}

/// Handle to communicate with a running process.
#[derive(Clone)]
pub struct ProcessHandle {
    id: ProcessId,
    /// Channel for sending signals to this process.
    signal_tx: mpsc::UnboundedSender<Signal>,
    /// Channel for sending messages to this process's mailbox.
    message_tx: mpsc::UnboundedSender<common::Message>,
}

impl ProcessHandle {
    pub fn send_signal(&self, signal: Signal) {
        let _ = self.signal_tx.send(signal);
    }

    pub fn send_message(&self, message: common::Message) {
        let _ = self.message_tx.send(message);
    }
}

/// Per-process state. Each process gets its own Wasm instance.
pub struct ProcessState {
    pub id: ProcessId,
    pub config: ProcessConfig,
    /// Signal receiver (Kill, Link, etc.).
    pub signal_rx: mpsc::UnboundedReceiver<Signal>,
    /// Message mailbox.
    pub message_rx: mpsc::UnboundedReceiver<common::Message>,
    /// Buffered messages for selective receive.
    pub mailbox_buffer: Vec<common::Message>,
    /// Linked processes.
    pub links: HashMap<ProcessId, Option<i64>>,
    /// Monitored processes.
    pub monitors: HashMap<ProcessId, ()>,
    /// Whether to die when a linked process dies.
    pub die_when_link_dies: bool,
    /// WASI state.
    pub wasi_ctx: Option<wasmtime_wasi::WasiCtx>,
    /// Resource table (TCP streams, etc.).
    pub resources: ResourceTable,
}

/// Resource table for host-managed objects.
pub struct ResourceTable {
    next_id: u32,
    tcp_streams: HashMap<u32, lunatic_networking::TcpStream>,
    tls_streams: HashMap<u32, lunatic_networking::TlsStream>,
    udp_sockets: HashMap<u32, lunatic_networking::UdpSocket>,
    dns_iterators: HashMap<u32, lunatic_networking::DnsIterator>,
    configs: HashMap<u32, ProcessConfig>,
    modules: HashMap<u32, Module>,
}

impl ResourceTable {
    pub fn insert_tcp_stream(&mut self, stream: lunatic_networking::TcpStream) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.tcp_streams.insert(id, stream);
        id
    }

    pub fn get_tcp_stream(&self, id: u32) -> Option<&lunatic_networking::TcpStream> {
        self.tcp_streams.get(&id)
    }

    pub fn remove_tcp_stream(&mut self, id: u32) -> Option<lunatic_networking::TcpStream> {
        self.tcp_streams.remove(&id)
    }
}
```

### Process Lifecycle

```rust
/// Spawn a new process from a compiled Wasm module.
pub async fn spawn_process(
    engine: &RuntimeEngine,
    module: &Module,
    config: ProcessConfig,
    entry_function: &str,
    env: Arc<Environment>,
) -> anyhow::Result<ProcessHandle> {
    let (signal_tx, signal_rx) = mpsc::unbounded_channel();
    let (message_tx, message_rx) = mpsc::unbounded_channel();

    let id = ProcessId::next();
    let handle = ProcessHandle {
        id,
        signal_tx,
        message_tx,
    };

    let state = ProcessState {
        id,
        config: config.clone(),
        signal_rx,
        message_rx,
        mailbox_buffer: Vec::new(),
        links: HashMap::new(),
        monitors: HashMap::new(),
        die_when_link_dies: true,
        wasi_ctx: None,
        resources: ResourceTable::default(),
    };

    // Spawn on the Tokio runtime
    tokio::spawn(async move {
        run_process(engine, module, state, entry_function, env).await
    });

    Ok(handle)
}

/// The main process execution loop.
async fn run_process(
    engine: &RuntimeEngine,
    module: &Module,
    mut state: ProcessState,
    entry: &str,
    env: Arc<Environment>,
) {
    // Create a new Wasm store with the process state
    let mut store = Store::new(&engine.engine, state);

    // Set fuel limit if configured
    if let Some(fuel) = store.data().config.max_fuel {
        store.set_fuel(fuel).unwrap();
    }

    // Create linker and register all host APIs
    let mut linker = Linker::new(&engine.engine);
    register_process_api(&mut linker);
    register_messaging_api(&mut linker);
    register_networking_api(&mut linker);
    register_timer_api(&mut linker);
    register_registry_api(&mut linker);
    register_wasi_api(&mut linker);
    // ... other APIs

    // Instantiate and run
    let instance = linker.instantiate_async(&mut store, module).await.unwrap();
    let entry_fn = instance.get_typed_func::<(), ()>(&mut store, entry).unwrap();

    // Run inside an AsyncWormhole so host functions can .await
    let result = entry_fn.call_async(&mut store, ()).await;

    // Process finished -- notify linked processes
    let reason = match result {
        Ok(()) => DeathReason::Normal,
        Err(e) => DeathReason::Failure(e.to_string()),
    };

    // Send LinkDied to all linked processes
    for (linked_id, tag) in &store.data().links {
        if let Some(handle) = env.processes.get(linked_id) {
            handle.send_signal(Signal::LinkDied {
                tag: *tag,
                process_id: store.data().id,
                reason: reason.clone(),
            });
        }
    }
}
```

### Scheduler Integration

The runtime uses Tokio as the async scheduler. Each lunatic process is a Tokio task. The process's main loop uses `tokio::select!` with bias to prioritize signal handling:

```rust
async fn process_main_loop(state: &mut ProcessState) {
    loop {
        tokio::select! {
            biased;

            // Priority 1: Handle signals (Kill, Link, etc.)
            Some(signal) = state.signal_rx.recv() => {
                match signal {
                    Signal::Kill => return,
                    Signal::Link { tag } => { /* add link */ },
                    Signal::LinkDied { tag, process_id, reason } => {
                        if state.die_when_link_dies {
                            return; // Propagate failure
                        }
                        // Otherwise deliver as a message
                    },
                    _ => { /* handle other signals */ }
                }
            }

            // Priority 2: Receive messages
            Some(message) = state.message_rx.recv() => {
                state.mailbox_buffer.push(message);
            }

            // Priority 3: Resume Wasm execution
            // (handled by the AsyncWormhole)
        }
    }
}
```

### Dependencies
- `wasmtime` (v8+) -- Wasm execution engine
- `wasmtime-wasi` -- WASI implementation
- `tokio` (full features) -- async runtime
- `anyhow` -- error handling
- `tracing` -- structured logging for the runtime itself

## Crate 4: distributed (Cluster Networking)

### Purpose
QUIC-based node-to-node communication with mutual TLS for distributed lunatic clusters.

### Key Design

```rust
use quinn::{Endpoint, ServerConfig, ClientConfig};
use rustls::{Certificate, PrivateKey};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::common::{NodeId, ProcessId, Signal, Message};

/// Represents a node in the distributed cluster.
pub struct DistributedNode {
    pub id: NodeId,
    /// QUIC endpoint for accepting and initiating connections.
    endpoint: Endpoint,
    /// Known peer nodes.
    peers: Arc<RwLock<HashMap<NodeId, PeerConnection>>>,
    /// TLS certificate for this node.
    certificate: Certificate,
    /// TLS private key.
    private_key: PrivateKey,
}

/// Connection to a peer node.
struct PeerConnection {
    node_id: NodeId,
    /// QUIC connection (multiplexed streams).
    connection: quinn::Connection,
}

impl DistributedNode {
    /// Connect to a control server, receive certificate, and join the cluster.
    pub async fn join_cluster(control_url: &str) -> anyhow::Result<Self> {
        // 1. Register with control server via HTTP
        // 2. Receive TLS certificate and list of peer nodes
        // 3. Create QUIC endpoint with mTLS configuration
        // 4. Establish connections to known peers
        todo!()
    }

    /// Spawn a process on a remote node.
    pub async fn remote_spawn(
        &self,
        target_node: NodeId,
        module_bytes: &[u8],
        entry: &str,
        config: common::ProcessConfig,
    ) -> anyhow::Result<ProcessId> {
        let peer = self.peers.read().await;
        let conn = peer.get(&target_node)
            .ok_or_else(|| anyhow::anyhow!("Unknown node: {:?}", target_node))?;

        // Open a new QUIC stream for this RPC
        let (mut send, mut recv) = conn.connection.open_bi().await?;

        // Send spawn request
        let request = RemoteSpawnRequest {
            module: module_bytes.to_vec(),
            entry: entry.to_string(),
            config,
        };
        send_message(&mut send, &request).await?;

        // Receive process ID
        let response: RemoteSpawnResponse = recv_message(&mut recv).await?;
        Ok(response.process_id)
    }

    /// Send a message to a process on a remote node.
    pub async fn remote_send(
        &self,
        target_node: NodeId,
        target_process: ProcessId,
        message: Message,
    ) -> anyhow::Result<()> {
        let peer = self.peers.read().await;
        let conn = peer.get(&target_node)
            .ok_or_else(|| anyhow::anyhow!("Unknown node"))?;

        let (mut send, _) = conn.connection.open_bi().await?;
        let request = RemoteSendRequest {
            target: target_process,
            message,
        };
        send_message(&mut send, &request).await?;
        Ok(())
    }
}
```

### Dependencies
- `quinn` -- QUIC implementation
- `rustls` -- TLS implementation
- `rcgen` -- Certificate generation
- `serde` + `bincode` -- Message serialization

## Crate 5: control (Control Plane)

### Purpose
HTTP-based control server for node discovery, certificate issuance, and cluster management.

```rust
use axum::{Router, Json, extract::State};
use std::sync::Arc;
use tokio::sync::RwLock;

struct ControlState {
    nodes: RwLock<Vec<RegisteredNode>>,
    ca_cert: rcgen::Certificate,
}

struct RegisteredNode {
    id: NodeId,
    address: std::net::SocketAddr,
    last_heartbeat: std::time::Instant,
}

async fn register_node(
    State(state): State<Arc<ControlState>>,
    Json(request): Json<RegisterRequest>,
) -> Json<RegisterResponse> {
    // 1. Generate a node-specific TLS certificate signed by the CA
    // 2. Add node to the registry
    // 3. Return certificate + list of known nodes
    todo!()
}

pub fn control_router(state: Arc<ControlState>) -> Router {
    Router::new()
        .route("/register", axum::routing::post(register_node))
        .route("/heartbeat", axum::routing::post(heartbeat))
        .route("/nodes", axum::routing::get(list_nodes))
        .with_state(state)
}
```

### Dependencies
- `axum` -- HTTP framework
- `rcgen` -- Certificate authority
- `rustls` -- TLS
- `serde` -- Serialization

## Crate 6: sdk (Client SDK -- lunatic-rs equivalent)

### Purpose
Idiomatic Rust API for building lunatic applications. This compiles to `wasm32-wasi` and runs inside the runtime.

### Key Types and Patterns

```rust
use serde::{Serialize, Deserialize, de::DeserializeOwned};

/// A handle to a running process.
#[derive(Clone, Serialize, Deserialize)]
pub struct Process<M: Serialize + DeserializeOwned> {
    id: u64,
    node_id: u64,
    _phantom: std::marker::PhantomData<M>,
}

impl<M: Serialize + DeserializeOwned> Process<M> {
    /// Send a message to this process.
    pub fn send(&self, message: &M) {
        let data = bincode::serialize(message).unwrap();
        // Call host function: lunatic::messaging::send(self.id, data)
        unsafe { host::send_message(self.id, data.as_ptr(), data.len()) };
    }

    /// Look up a process by name in the registry.
    pub fn lookup<N: ProcessName>(name: &N) -> Option<Self> {
        // Call host function: lunatic::registry::lookup(name)
        todo!()
    }

    /// Register this process under a name.
    pub fn register<N: ProcessName>(&self, name: &N) {
        // Call host function: lunatic::registry::register(name, self.id)
        todo!()
    }
}

/// A typed mailbox for receiving messages.
pub struct Mailbox<M: Serialize + DeserializeOwned> {
    _phantom: std::marker::PhantomData<M>,
}

impl<M: Serialize + DeserializeOwned> Mailbox<M> {
    /// Block until a message is received.
    pub fn receive(&self) -> M {
        // Call host function: lunatic::messaging::receive()
        // This blocks the Wasm execution (but not the host thread,
        // thanks to async wormhole stack switching)
        let data = unsafe { host::receive_message() };
        bincode::deserialize(&data).unwrap()
    }

    /// Receive with a timeout.
    pub fn receive_timeout(&self, timeout: std::time::Duration) -> Option<M> {
        let millis = timeout.as_millis() as u64;
        let data = unsafe { host::receive_message_timeout(millis) };
        data.map(|d| bincode::deserialize(&d).unwrap())
    }

    /// Selective receive: receive the first message matching a tag.
    pub fn tag_receive(&self, tags: &[i64]) -> M {
        todo!()
    }
}

/// Trait for processes with structured message handling.
/// Equivalent to Erlang's gen_server.
pub trait AbstractProcess: Serialize + DeserializeOwned {
    type Arg: Serialize + DeserializeOwned;
    type State: Serialize + DeserializeOwned;

    /// Initialize process state from arguments.
    fn init(config: &ProcessConfig, arg: Self::Arg) -> Self::State;

    /// Handle a message, returning updated state.
    fn handle_message(state: &mut Self::State, message: Self)
    where
        Self: Sized;

    /// Start the process.
    fn start(arg: Self::Arg) -> Result<Process<Self>, SpawnError>
    where
        Self: Sized,
    {
        // Spawn a new process running the AbstractProcess loop
        todo!()
    }

    /// Start with a link to the current process.
    fn start_link(arg: Self::Arg) -> Result<Process<Self>, SpawnError>
    where
        Self: Sized,
    {
        todo!()
    }
}

/// A supervisor that monitors and restarts child processes.
pub trait Supervisor {
    type Arg: Serialize + DeserializeOwned;
    type Children: SupervisorChildren;

    /// Define the children to supervise.
    fn init(config: &ProcessConfig, arg: Self::Arg) -> Self::Children;
}

/// Trait for supervisor child specifications.
pub trait SupervisorChildren {
    fn start_all(&self) -> Vec<Process<()>>;
}

/// Spawn a new process with a closure.
/// The closure runs in a new Wasm instance.
pub fn spawn<M, F>(f: F) -> Result<Process<M>, SpawnError>
where
    M: Serialize + DeserializeOwned,
    F: FnOnce(Mailbox<M>) + Serialize + DeserializeOwned,
{
    // Serialize the closure
    // Call host function to spawn a new process
    // The new process deserializes the closure and executes it
    todo!()
}

/// Spawn and link.
pub fn spawn_link<M, F>(f: F) -> Result<Process<M>, SpawnError>
where
    M: Serialize + DeserializeOwned,
    F: FnOnce(Mailbox<M>) + Serialize + DeserializeOwned,
{
    todo!()
}
```

### The `spawn_link!` Macro

```rust
/// Macro for spawning linked processes with captured variables.
/// Variables are automatically serialized and sent to the new process.
#[macro_export]
macro_rules! spawn_link {
    (|$($captures:ident),* , mailbox: Mailbox<$msg_ty:ty>| $body:block) => {
        {
            // Each captured variable must be Serialize + DeserializeOwned
            $(
                let $captures = $captures;
            )*
            $crate::spawn_link(move |mailbox: Mailbox<$msg_ty>| {
                $body
            })
        }
    };
}
```

### Process-Local Storage

```rust
/// Macro for declaring process-local storage.
/// Similar to thread_local! but for lunatic processes.
#[macro_export]
macro_rules! process_local {
    (static $name:ident : $ty:ty = $init:expr;) => {
        // In Wasm, each process has its own linear memory,
        // so regular statics are already process-local.
        // This macro just provides the familiar API.
        static $name: std::cell::LazyCell<$ty> = std::cell::LazyCell::new(|| $init);
    };
}
```

### Dependencies (SDK crate -- compiles to wasm32-wasi)
- `serde` -- serialization
- `bincode` -- binary serialization format
- No `tokio` -- the SDK is synchronous; async is handled by the host runtime

## Crate 7: networking-api (Host Networking Functions)

### Purpose
Host functions for TCP/UDP/TLS/DNS operations exposed to Wasm guests.

```rust
use wasmtime::Caller;

/// Register networking host functions with the Wasmtime linker.
pub fn register(linker: &mut wasmtime::Linker<ProcessState>) {
    // TCP
    linker.func_wrap_async("lunatic::networking", "tcp_bind_v1",
        |mut caller: Caller<'_, ProcessState>, addr_ptr: u32, addr_len: u32, port: u32|
    {
        Box::new(async move {
            let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
            let addr_bytes = &memory.data(&caller)[addr_ptr as usize..(addr_ptr + addr_len) as usize];
            let addr = std::str::from_utf8(addr_bytes).unwrap();
            let socket_addr = format!("{}:{}", addr, port);

            match tokio::net::TcpListener::bind(&socket_addr).await {
                Ok(listener) => {
                    let id = caller.data_mut().resources.insert_tcp_listener(listener);
                    Ok(id as i64)
                }
                Err(e) => {
                    let err_id = caller.data_mut().resources.insert_error(e.to_string());
                    Err(err_id as i64)
                }
            }
        })
    }).unwrap();

    // TCP accept
    linker.func_wrap_async("lunatic::networking", "tcp_accept_v1",
        |mut caller: Caller<'_, ProcessState>, listener_id: u32|
    {
        Box::new(async move {
            let listener = caller.data().resources.get_tcp_listener(listener_id).unwrap();
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let stream_id = caller.data_mut().resources.insert_tcp_stream(stream);
                    Ok(stream_id as i64)
                }
                Err(e) => {
                    let err_id = caller.data_mut().resources.insert_error(e.to_string());
                    Err(err_id as i64)
                }
            }
        })
    }).unwrap();

    // TCP read, write, connect, DNS resolution, TLS, UDP...
}
```

### Key Insight: Async Transparency

When a Wasm guest calls `tcp_accept_v1`, the host function is async. Thanks to the `stack-switching` crate, the Wasm execution is transparently suspended, the Tokio runtime polls the accept future, and when a connection arrives, execution resumes at the exact point in the Wasm code where it called the host function. The guest code looks synchronous.

## Crate 8: http-client (nightfly-like)

### Purpose
HTTP client library for lunatic applications (compiles to wasm32-wasi).

```rust
use serde::{Serialize, Deserialize};

/// An HTTP client with connection management and configuration.
#[derive(Clone, Serialize, Deserialize)]
pub struct Client {
    default_headers: Vec<(String, String)>,
    redirect_policy: RedirectPolicy,
    max_redirects: usize,
    timeout: Option<std::time::Duration>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            default_headers: Vec::new(),
            redirect_policy: RedirectPolicy::default(),
            max_redirects: 10,
            timeout: None,
        }
    }

    pub fn get(&self, url: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), Method::GET, url.as_ref())
    }

    pub fn post(&self, url: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(self.clone(), Method::POST, url.as_ref())
    }
}

pub struct RequestBuilder {
    client: Client,
    method: Method,
    url: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl RequestBuilder {
    pub fn json<T: Serialize>(mut self, value: &T) -> Self {
        self.body = Some(serde_json::to_vec(value).unwrap());
        self.headers.push(("Content-Type".into(), "application/json".into()));
        self
    }

    pub fn send(self) -> Result<Response, Error> {
        // 1. Parse URL, resolve DNS
        // 2. Open TCP connection (or TLS for HTTPS)
        // 3. Write HTTP/1.1 request
        // 4. Read and parse response (httparse)
        // 5. Handle redirects, decompression, cookies
        // All I/O uses lunatic host functions (synchronous from guest perspective)
        todo!()
    }
}
```

### Dependencies (wasm32-wasi target)
- `http` -- HTTP types
- `httparse` -- HTTP/1.1 parsing
- `url` -- URL parsing
- `serde` / `serde_json` -- Serialization
- `flate2` -- gzip/deflate
- `brotli` -- Brotli decompression

## Crate 9: logging (Process-Based Logging)

### Purpose
Logging library using a dedicated process for log aggregation.

```rust
use serde::{Serialize, Deserialize};

/// Log event sent from any process to the logging process.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEvent {
    pub level: Level,
    pub message: String,
    pub module: String,
    pub file: String,
    pub line: u32,
    pub timestamp: String,
}

/// Subscriber trait -- implement to customize log output.
pub trait Subscriber: Serialize + for<'de> Deserialize<'de> + Send + 'static {
    fn enabled(&self, level: &Level) -> bool;
    fn event(&self, event: &LogEvent);
}

/// Initialize logging. Spawns a subscriber process and registers it.
pub fn init(subscriber: impl Subscriber) {
    let process = sdk::spawn_link!(|subscriber, mailbox: Mailbox<LogEvent>| {
        loop {
            let event = mailbox.receive();
            if subscriber.enabled(&event.level) {
                subscriber.event(&event);
            }
        }
    });
    process.register(&LoggingProcessName);
}

/// Log macros look up the logging process and send events.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        if let Some(logger) = $crate::lookup_logger() {
            logger.send(&$crate::LogEvent {
                level: $crate::Level::Info,
                message: format!($($arg)*),
                module: module_path!().to_string(),
                file: file!().to_string(),
                line: line!(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };
}
```

### Key Pattern
This demonstrates the fundamental lunatic pattern for cross-cutting concerns: run a dedicated process, register it by name, and have other processes communicate with it via message passing. This replaces global statics which do not work across Wasm instances.

## Crate 10: web-framework (submillisecond-like)

### Purpose
HTTP web framework that spawns one lunatic process per request.

```rust
/// Start a web server on the given address.
pub fn serve(addr: &str, router: Router) -> ! {
    let listener = sdk::net::TcpListener::bind(addr).unwrap();
    loop {
        let (stream, _addr) = listener.accept().unwrap();
        let router = router.clone();
        // Each request gets its own process
        sdk::spawn(move |_: Mailbox<()>| {
            handle_connection(stream, &router);
        }).unwrap();
    }
}

/// Router built with a proc macro.
pub struct Router {
    routes: Vec<Route>,
    middleware: Vec<Box<dyn Middleware>>,
}

struct Route {
    method: Method,
    pattern: RoutePattern,
    handler: Box<dyn Handler>,
}

pub trait Handler: Send + Sync + Clone {
    fn handle(&self, request: Request) -> Response;
}

/// Middleware can wrap handlers.
pub trait Middleware: Send + Sync {
    fn call(&self, request: Request, next: &dyn Handler) -> Response;
}
```

### Process-Per-Request Model

```
                          +--> [Process: GET /users]
TCP Listener Process ---->+--> [Process: POST /api/data]
                          +--> [Process: GET /static/file.js]
                          +--> [Process: WebSocket /ws]
```

Each request runs in isolation. If a request handler panics, only that process dies. The listener process is unaffected.

## Error Handling Strategy

### Runtime Errors (Host Side)
```rust
// Use anyhow for internal runtime errors
use anyhow::{Result, Context};

fn compile_module(bytes: &[u8]) -> Result<Module> {
    let module = Module::new(&engine, bytes)
        .context("Failed to compile Wasm module")?;
    Ok(module)
}
```

### Guest-Visible Errors
```rust
// Errors exposed to Wasm guests use an ID-based system
// Each error is stored in a table and referenced by ID
pub struct ErrorTable {
    errors: HashMap<u32, String>,
    next_id: u32,
}

impl ErrorTable {
    pub fn insert(&mut self, error: String) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.errors.insert(id, error);
        id
    }

    pub fn get(&self, id: u32) -> Option<&str> {
        self.errors.get(&id).map(|s| s.as_str())
    }

    pub fn remove(&mut self, id: u32) -> Option<String> {
        self.errors.remove(&id)
    }
}
```

### SDK Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("Permission denied: process cannot spawn")]
    PermissionDenied,
    #[error("Module compilation failed: {0}")]
    CompilationFailed(String),
    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,
}

#[derive(Debug, thiserror::Error)]
pub enum MailboxError {
    #[error("Receive timed out")]
    Timeout,
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
}
```

## Concurrency Considerations

### Process Isolation via Wasm
Each process runs in its own Wasm instance with separate linear memory. There is zero shared mutable state between processes. This eliminates data races at the architecture level.

### Host-Side Concurrency
The host runtime is a standard Tokio application. Shared state (process registry, environment) uses:
- `Arc<RwLock<T>>` for read-heavy shared state
- `mpsc::UnboundedSender` for signal/message channels (lock-free)
- `DashMap` for concurrent hash maps where appropriate

### Fuel-Based Preemption
Wasmtime's fuel mechanism provides cooperative preemption:
```rust
// Set fuel limit (each Wasm instruction consumes fuel)
store.set_fuel(100_000)?;

// Check remaining fuel
let remaining = store.get_fuel()?;
if remaining == 0 {
    // Process exceeded compute budget
    // Send Kill signal
}
```

### Message Passing Semantics
Messages are always copied (serialized), never shared. This is inherent to the Wasm isolation model -- there is no shared memory between instances. Serialization formats:
- **Within a node**: `bincode` for speed
- **Across nodes**: `bincode` over QUIC (already has framing)

## Performance Considerations

### Module Caching
Wasm modules should be compiled once and instantiated many times:
```rust
// Compile once
let module = Module::new(&engine, wasm_bytes)?;

// Instantiate per-process (fast -- no recompilation)
let instance = linker.instantiate(&mut store, &module)?;
```

### Stack Allocation
The `stack-switching` crate should use a stack pool to avoid repeated mmap/munmap:
```rust
struct StackPool {
    free_stacks: Vec<Stack>,
    stack_size: usize,
}

impl StackPool {
    fn acquire(&mut self) -> Stack {
        self.free_stacks.pop().unwrap_or_else(|| Stack::new(self.stack_size))
    }

    fn release(&mut self, stack: Stack) {
        self.free_stacks.push(stack);
    }
}
```

### Resource Cleanup
When a process dies, all its resources must be cleaned up:
- Close TCP/TLS/UDP sockets
- Remove from link sets of other processes
- Remove from monitor sets
- Remove from process registry
- Free the Wasm store (and thus linear memory)

### Benchmarking Targets
Based on the original lunatic's design goals:
- Process spawn: < 10 microseconds
- Message send (small): < 1 microsecond
- Context switch: < 1 microsecond
- Memory per idle process: ~64 KB (minimum Wasm page)

## Key Crate Dependencies Summary

| Crate | Dependency | Purpose |
|-------|-----------|---------|
| runtime | wasmtime 8+ | Wasm execution |
| runtime | tokio (full) | Async scheduler |
| distributed | quinn | QUIC transport |
| distributed | rustls | mTLS |
| control | axum | HTTP control plane |
| control | rcgen | Certificate generation |
| sdk | serde + bincode | Message serialization |
| http-client | httparse | HTTP parsing |
| http-client | flate2, brotli | Decompression |
| logging | chrono | Timestamps |
| web-framework | (sdk only) | No extra deps |
| stack-switching | libc | mmap for stacks |
| common | serde | Shared type serialization |

## Testing Strategy

### Unit Tests
- Each API crate: test host function registration and behavior
- SDK: test serialization/deserialization of messages
- Router: test URL pattern matching
- Distributed: test message framing and serialization

### Integration Tests
- Spawn processes and verify message delivery
- Test supervisor restart behavior
- Test link/monitor propagation
- Test distributed spawn and remote messaging

### Conformance Tests
- Run the WASI test suite via `wasmtime-wasi`
- WebSocket: Autobahn test suite (for the web framework)
- HTTP: Test against httpbin.org patterns

### Fuzz Testing
- DOM diffing (LiveView)
- HTTP parsing
- RESP protocol parsing (Redis driver)
- Message deserialization

## Migration Path from Existing Lunatic Code

Applications written for `lunatic-rs` can be adapted to this workspace by:
1. Replacing `lunatic` imports with `sdk` imports
2. Ensuring all message types implement `Serialize + DeserializeOwned`
3. Replacing `nightfly` with `http-client`
4. Replacing `lunatic-log` with `logging`
5. The `#[lunatic::main]` attribute becomes `#[sdk::main]`
6. `AbstractProcess` trait remains the same
7. `Supervisor` trait remains the same

The Wasm compilation target (`wasm32-wasi`) and execution model (`lunatic run app.wasm`) remain identical.
