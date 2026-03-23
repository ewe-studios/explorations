# IPC Channel Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC/ipc-channel
repository: https://github.com/servo/ipc-channel
explored_at: 2026-03-23

## Overview

`ipc-channel` is a multiprocess implementation of Rust channels, inspired by CSP (Communicating Sequential Processes). It extends Rust's standard library channels to support inter-process communication (IPC) within a single operating system instance.

## Project Structure

```
ipc-channel/
├── src/
│   ├── lib.rs              # Library root
│   ├── ipc.rs              # Main IPC implementation
│   ├── asynch.rs           # Async channel support
│   ├── router.rs           # Message routing
│   ├── test.rs             # Test utilities
│   └── platform/
│       ├── mod.rs          # Platform abstraction
│       ├── inprocess/      # In-process implementation
│       ├── unix/           # Unix (Linux, BSD, etc.)
│       ├── macos/          # macOS (Mach ports)
│       └── windows/        # Windows (named pipes)
├── tests/
├── benches/
│   ├── ipc.rs              # IPC benchmarks
│   ├── ipc_receiver_set.rs
│   ├── ipc_shared_mem.rs
│   ├── platform.rs
│   └── struct_ipc.rs
├── Cargo.toml
└── LICENSE-APACHE, LICENSE-MIT
```

## Cargo Configuration

```toml
[package]
name = "ipc-channel"
version = "0.21.0"
edition = "2021"
rust-version = "1.86.0"
license = "MIT OR Apache-2.0"

[features]
default = []
force-inprocess = []      # Force in-process transport
async = ["futures-core", "futures-channel"]
win32-trace = []          # Windows debugging
enable-slow-tests = []    # Enable slow tests

[dependencies]
bincode = "1"              # Serialization
crossbeam-channel = "0.5"  # MPSC channels
serde_core = "1.0"         # Serialization traits
libc = "0.2.162"           # Platform APIs
uuid = { version = "1", features = ["v4"] }

[target.'cfg(any(target_os = "linux", target_os = "openbsd", target_os = "freebsd", target_os = "illumos"))'.dependencies]
mio = { version = "1.0", features = ["os-ext"] }
tempfile = "3.4"

[target.'cfg(target_os = "windows")'.dependencies.windows]
version = "0.61"
features = [
    "Win32_System_Pipes",
    "Win32_System_Threading",
    "Win32_System_Memory",
    "Win32_Storage_FileSystem",
]
```

## API Design

### Channel Creation

```rust
use ipc_channel::ipc;

// Create connected channels
let (tx, rx) = ipc::channel().unwrap();

// Send and receive
tx.send(data).unwrap();
let received = rx.recv().unwrap();
```

### Type Requirements

```rust
// Types must implement Serialize + Deserialize
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Message {
    id: u32,
    payload: String,
}
```

### IpcSender

```rust
pub struct IpcSender<T> {
    // Platform-specific internals
}

impl<T: Serialize> IpcSender<T> {
    pub fn send(&self, data: T) -> Result<(), Error>;

    // Send with shared memory optimization
    pub fn send_with_shared_mem(&self, data: T) -> Result<(), Error>;
}

impl<T> Clone for IpcSender<T> {
    fn clone(&self) -> Self { }
}
```

### IpcReceiver

```rust
pub struct IpcReceiver<T> {
    // Platform-specific internals
}

impl<T: Deserialize> IpcReceiver<T> {
    pub fn recv(&self) -> Result<T, Error>;

    pub fn try_recv(&self) -> Result<T, TryRecvError>;

    pub fn iter(&self) -> Iter<T>;
}

impl<T> Iterator for IpcReceiver<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> { }
}
```

### IpcReceiverSet

```rust
// Receive from multiple channels
pub struct IpcReceiverSet {
    // ...
}

impl IpcReceiverSet {
    pub fn new() -> Result<Self, Error>;

    pub fn add<T: Deserialize>(&mut self, rx: IpcReceiver<T>)
        -> Result<i32, Error>;

    pub fn select(&mut self) -> Result<Vec<IpcSelectionResult>, Error>;
}

pub enum IpcSelectionResult {
    Message(i32, OpaqueMessage),
    ChannelClosed(i32),
}
```

### One-Shot Server

```rust
// Bootstrap connection between processes
pub struct IpcOneShotServer<T> {
    // ...
}

impl<T: Serialize + Deserialize> IpcOneShotServer<T> {
    pub fn new() -> Result<(Self, String), Error>;

    pub fn accept(self) -> Result<(IpcReceiver<T>, T), Error>;
}

// Client connects
pub fn connect<T: Serialize>(name: &str) -> Result<IpcSender<T>, Error>;
```

## Platform Implementations

### Unix (Linux, BSD, Illumos)

```rust
// Platform-specific implementation
mod unix {
    use std::os::unix::io::RawFd;
    use std::os::unix::net::UnixDatagram;

    // File descriptor passing
    pub struct UnixChannel {
        socket: UnixDatagram,
        shared_mem: Option<SharedMem>,
    }

    // FD passing via ancillary data
    fn send_fd(socket: &UnixDatagram, fd: RawFd) -> io::Result<()> {
        // Use SCM_RIGHTS
    }

    // Shared memory for large messages
    struct SharedMem {
        path: PathBuf,
        size: usize,
        ptr: *mut u8,
    }
}
```

#### Shared Memory

```rust
// Create shared memory segment
let shmem = SharedMemory::create(name, size)?;

// Map into address space
let mapped = shmem.map()?;

// Zero-copy transfer for large data
```

### macOS

```rust
// Mach port-based implementation
mod macos {
    use mach2::port::mach_port_t;
    use mach2::message::mach_msg_t;

    pub struct MachChannel {
        port: mach_port_t,
    }

    // Send via Mach messages
    fn mach_send(port: mach_port_t, msg: &[u8]) -> kern_return_t;

    // Receive with timeout
    fn mach_recv(port: mach_port_t, timeout: u32) -> Result<Vec<u8>, Error>;
}
```

### Windows

```rust
// Named pipe implementation
mod windows {
    use windows::Win32::System::Pipes::*;

    pub struct WindowsChannel {
        handle: HANDLE,
    }

    // Create named pipe
    let handle = CreateNamedPipeA(
        name,
        PIPE_ACCESS_DUPLEX,
        PIPE_TYPE_MESSAGE,
        PIPE_UNLIMITED_INSTANCES,
        4096, 4096, 0, None
    )?;

    // Read/write via ReadFile/WriteFile
}
```

### In-Process (Testing)

```rust
// For testing without spawning processes
mod inprocess {
    use crossbeam_channel::{bounded, Sender, Receiver};

    pub struct InProcessChannel<T> {
        tx: Sender<T>,
        rx: Receiver<T>,
    }

    // Uses crossbeam channels internally
}
```

## Serialization

### Bincode Integration

```rust
use bincode::{serialize, deserialize, config};

// Default configuration
let config = config::standard();

// Serialize message
let bytes = serialize(&message, config)?;

// Deserialize message
let message: MyType = deserialize(&bytes, config)?;
```

### Serde Derivation

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    id: u64,
    action: String,
    payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
enum Response {
    Success(Vec<u8>),
    Error(String),
}
```

### Zero-Copy Considerations

```rust
// For zero-copy, use borrowed types
#[derive(Serialize, Deserialize)]
struct BorrowedData<'a> {
    #[serde(borrow)]
    bytes: &'a [u8],
}
```

## Async Support

### Async Channels

```rust
#[cfg(feature = "async")]
use futures_channel::mpsc;

pub async fn send_async<T: Serialize>(
    tx: &IpcSender<T>,
    data: T,
) -> Result<(), Error> {
    // ...
}

pub async fn recv_async<T: Deserialize>(
    rx: &IpcReceiver<T>,
) -> Result<T, Error> {
    // ...
}
```

### Stream Integration

```rust
use futures::stream::Stream;

impl<T: Deserialize> Stream for IpcReceiver<T> {
    type Item = Result<T, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // ...
    }
}
```

## Router

### Message Routing

```rust
pub struct Router {
    // Routes messages to multiple receivers
}

impl Router {
    pub fn new() -> Self;

    pub fn route<T: Deserialize, F: Fn(T) + Send + 'static>(
        &mut self,
        rx: IpcReceiver<T>,
        handler: F,
    ) -> Result<(), Error>;
}
```

## Testing

### Cross-Process Tests

```rust
#[test]
fn cross_process_send_recv() {
    let (tx, rx) = ipc::channel().unwrap();

    // Spawn child process
    let child = fork().unwrap();
    if child == 0 {
        // Child: send message
        tx.send(42).unwrap();
        exit(0);
    }

    // Parent: receive message
    let value = rx.recv().unwrap();
    assert_eq!(value, 42);
}
```

### One-Shot Server Test

```rust
#[test]
fn spawn_one_shot_server_client() {
    // Create server
    let (server, name) = IpcOneShotServer::new().unwrap();

    // Spawn client with server name
    let child = Command::new("./client")
        .arg(&name)
        .spawn()
        .unwrap();

    // Accept connection
    let (_rx, msg) = server.accept().unwrap();
    assert_eq!(msg, "hello");
}
```

## Benchmarks

### IPC Benchmark

```rust
#[bench]
fn bench_ipc_send(b: &mut Bencher) {
    let (tx, rx) = ipc::channel().unwrap();

    b.iter(|| {
        tx.send(42u64).unwrap();
        rx.recv().unwrap();
    });
}
```

### Shared Memory Benchmark

```rust
#[bench]
fn bench_ipc_shared_mem(b: &mut Bencher) {
    let (tx, rx) = ipc::channel().unwrap();
    let data = vec![0u8; 1024 * 1024];  // 1 MB

    b.iter(|| {
        tx.send_with_shared_mem(&data).unwrap();
        rx.recv().unwrap();
    });
}
```

## Semantic Differences from std::mpsc

| Feature | std::mpsc | ipc-channel |
|---------|-----------|-------------|
| Bounded | Yes/No | Always unbounded |
| Blocking | Yes (send/recv) | Never blocks |
| Resource usage | Minimal | OS IPC resources |
| Type safety | Compile-time | Runtime (serde) |
| Ownership | Transfer | Serialize/Deserialize |

## Use Cases

### Servo Browser Engine

```rust
// Communication between browser processes
enum ScriptMsg {
    Navigate(Url),
    LoadComplete(LoadInfo),
    ScriptEvent(Event),
}

let (script_tx, script_rx) = ipc::channel().unwrap();
```

### Process Isolation

```rust
// Isolate untrusted code
#[derive(Serialize, Deserialize)]
enum SandboxRequest {
    Evaluate(String),
    ReadFile(PathBuf),
}

let (sandbox_tx, sandbox_rx) = ipc::channel().unwrap();
```

### Plugin Systems

```rust
// Load plugins in separate processes
trait Plugin {
    fn process(&self, data: &[u8]) -> Vec<u8>;
}

struct RemotePlugin {
    tx: IpcSender<PluginRequest>,
    rx: IpcReceiver<PluginResponse>,
}
```

## Security Considerations

### File Descriptor Rights

On Unix, FD passing can be a security concern:

```rust
// Validate received FDs
fn validate_fd(fd: RawFd) -> Result<(), Error> {
    // Check FD type, permissions, etc.
}
```

### Shared Memory Security

```rust
// Use unique names with UUIDs
let name = format!("/ipc-channel-{}", uuid::Uuid::new_v4());
let shmem = SharedMemory::create(&name, size)?;
```

### Process Isolation

```rust
// Use seccomp, namespaces for isolation
use std::process::Command;

Command::new("./sandboxed_worker")
    .env("IPC_CHANNEL", name)
    .spawn()?;
```

## Platform Support

| Platform | Transport | Status |
|----------|-----------|--------|
| Linux | Unix sockets + shm | Full |
| macOS | Mach ports | Full |
| Windows | Named pipes | Full |
| FreeBSD | Unix sockets | Full |
| OpenBSD | Unix sockets | Full |
| Illumos | Unix sockets | Full |
| WASI | In-process only | Limited |

## Known Limitations

1. **Single client per server**: One-shot server accepts only one connection
2. **No authentication**: Trust-based model
3. **Platform differences**: Behavior varies by platform
4. **Serialization overhead**: Not zero-copy for all types

## Related Projects

- **crossbeam-channel**: MPMC channels (single process)
- **tokio::sync**: Async channels
- **multiprocess**: Alternative IPC library
- **capnp-rpc**: Capability-based RPC (alternative approach)

## Resources

- [GitHub Repository](https://github.com/servo/ipc-channel)
- [Documentation](https://docs.rs/ipc-channel)
- [Servo Project](https://github.com/servo/servo)
