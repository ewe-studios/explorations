---
name: WASI
description: WebAssembly System Interface - Standard system interface for WebAssembly modules
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/WASI/
---

# WASI - WebAssembly System Interface

## Overview

WASI (WebAssembly System Interface) is a **standardized system interface for WebAssembly** that provides a modular, secure, and portable way for WASM modules to interact with the host system. It defines a set of APIs that allow WASM modules to perform I/O operations, access files, use networking, and more - all while maintaining security and portability.

Key features:
- **Capability-based security** - Explicit resource access grants
- **Modular design** - Separate API proposals (wasi-common, wasi-nn, wasi-crypto, etc.)
- **Portable** - Write once, run anywhere
- **Secure by default** - No implicit access to system resources
- **Multiple versions** - wasi_snapshot_preview1, wasi_snapshot_preview2, preview3

## Directory Structure

```
WASI/
├── legacy/
│   ├── wasi_snapshot_preview1/   # Legacy snapshot preview 1
│   └── snapshot_01/              # Original snapshot
├── proposals/
│   ├── api/                      # Core API specification
│   ├── clocks/                   # Clock and time APIs
│   ├── filesystem/               # File system access
│   ├── http/                     # HTTP client/server
│   ├── io/                       # Basic I/O operations
│   ├── logging/                  # Logging interface
│   ├── misc/                     # Miscellaneous APIs
│   ├── nn/                       # Neural network inference
│   ├── crypto/                   # Cryptographic operations
│   ├── random/                   # Random number generation
│   ├── sockets/                  # Network sockets
│   └── threads/                  # Threading support
├── specs/                        # Formal specifications
├── .github/
├── README.md
└── LICENSE.md
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    WASI Architecture                            │
└─────────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│  WASM Module     │ │  WASI Interface  │ │  Host System     │
│                  │ │                  │ │                  │
│  ┌────────────┐  │ │  ┌────────────┐  │ │  ┌────────────┐  │
│  │ Application│  │ │  │ API Layer  │  │ │  │   Linux    │  │
│  │    Code    │──┼─┼─▶│  (WIT)     │──┼─┼─▶│  macOS     │  │
│  │            │  │ │  │            │  │ │  │  Windows   │  │
│  │  Imports   │  │ │  │  Exports   │  │ │  │            │  │
│  └────────────┘  │ │  └────────────┘  │ │  └────────────┘  │
│                  │ │                  │ │                  │
│  fd_read()       │ │  read(fd, buf)   │ │  read syscall    │
│  fd_write()      │ │  write(fd, buf)  │ │  write syscall   │
│  path_open()     │ │  open(path)      │ │  open syscall    │
└──────────────────┘ └──────────────────┘ └──────────────────┘
                            │
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                    WASI Proposal Stack                        │
├───────────────────────────────────────────────────────────────┤
│  Tier 3: Experimental                                         │
│  ├── wasi-nn (neural networks)                                │
│  ├── wasi-crypto (cryptography)                               │
│  └── wasi-ml (machine learning)                               │
├───────────────────────────────────────────────────────────────┤
│  Tier 2: Standardized                                         │
│  ├── wasi-sockets (TCP/UDP)                                   │
│  ├── wasi-http (HTTP client/server)                           │
│  └── wasi-threads (spawn threads)                             │
├───────────────────────────────────────────────────────────────┤
│  Tier 1: Stable (wasi_snapshot_preview1)                      │
│  ├── wasi-basic (exit, poll)                                  │
│  ├── wasi-clocks (wall time, monotonic, CPU time)             │
│  ├── wasi-filesystem (fd operations, paths)                   │
│  ├── wasi-io (read, write, seek)                              │
│  └── wasi-random (random bytes, secure random)                │
└───────────────────────────────────────────────────────────────┘
```

## Core API Categories

### File Descriptor I/O

```wit
// WIT (Wasm Interface Types) definition
interface wasi-io {
    /// Read from a file descriptor
    read: func(len: u64) -> result<tuple<u8*, u64>, error-code>;

    /// Write to a file descriptor
    write: func(buf: u8*) -> result<u64, error-code>;

    /// Seek to a position
    seek: func(offset: s64, whence: whence) -> result<u64, error-code>;

    /// Flush buffered output
    flush: func() -> result<(), error-code>;
}

enum whence {
    /// Seek from start of file
    set,
    /// Seek from current position
    cur,
    /// Seek from end of file
    end,
}
```

Rust usage with wasmtime:
```rust
use wasmtime_wasi::WasiCtx;
use wasmtime::{Store, Module, Linker};

// Create WASI context
let wasi = WasiCtxBuilder::new()
    .inherit_stdio()
    .build();

let mut store = Store::new(&engine, wasi);

// Link WASI functions
let mut linker = Linker::new(&engine);
wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

// Instantiate module
let module = Module::new(&engine, wasm_bytes)?;
let instance = linker.instantiate(&mut store, &module)?;
```

### Filesystem Access

```wit
interface wasi-filesystem {
    /// Open a file or directory
    open-at: func(
        dir-fd: fd,
        path: string,
        options: open-options
    ) -> result<fd, error-code>;

    /// Read directory entries
    read-dir: func(fd: fd) -> result<list<dir-entry>, error-code>;

    /// Get file metadata
    get-metadata: func(fd: fd) -> result<metadata, error-code>;

    /// Set file metadata
    set-metadata: func(fd: fd, metadata: metadata) -> result<(), error-code>;

    /// Create a directory
    create-directory: func(dir-fd: fd, path: string) -> result<(), error-code>;

    /// Unlink a file
    unlink-file: func(dir-fd: fd, path: string) -> result<(), error-code>;
}

record open-options {
    read: bool,
    write: bool,
    create: bool,
    truncate: bool,
    append: bool,
}

record metadata {
    type: file-type,
    size: u64,
    modified: option<timestamp>,
    accessed: option<timestamp>,
    created: option<timestamp>,
}
```

### Clocks and Time

```wit
interface wasi-clocks {
    /// Wall clock time
    wall-clock: func() -> tuple<u64, u32>;

    /// Monotonic clock
    monotonic: func() -> u64;

    /// CPU time clock
    cpu: func() -> cpu-clock-output;
}

/// Get current time as seconds and nanoseconds
fn get_wall_clock() -> (u64, u32) {
    // Returns (seconds since epoch, nanoseconds)
}

/// Subscribe to a monotonic clock
fn subscribe_monotonic(deadline: u64) -> subscription {
    // Create subscription that fires at deadline
}
```

### Random Number Generation

```wit
interface wasi-random {
    /// Get random bytes
    get-random-bytes: func(len: u64) -> list<u8>;

    /// Get secure random bytes (cryptographic quality)
    get-secure-random-bytes: func(len: u64) -> list<u8>;
}
```

## WASI Versions

### wasi_snapshot_preview1 (Legacy)

```rust
// Original WASI snapshot - still widely used
use wasi_snapshot_preview1::wasi_args_get;

#[no_mangle]
pub unsafe extern "C" fn wasi_args_get(
    argv: *mut *mut u8,
    argv_buf: *mut u8,
) -> __wasi_errno_t {
    // Get command-line arguments
}

// Functions available:
// - args_get, args_sizes_get      (command-line args)
// - environ_get, environ_sizes_get (environment vars)
// - clock_time_get, clock_res_get (clocks)
// - fd_read, fd_write, fd_seek    (I/O)
// - fd_open, fd_close, fd_stat    (file descriptors)
// - path_open, path_create_directory
// - poll_oneoff                    (async polling)
// - proc_exit, proc_raise         (process control)
// - random_get                     (random numbers)
```

### WASI Preview 2 (Component Model)

```wit
// Preview 2 uses WIT and the component model
package wasi:cli;

interface command {
    use wasi:cli/environment@0.2.0;
    use wasi:cli/exit@0.2.0;
    use wasi:cli/stdin@0.2.0;
    use wasi:cli/stdout@0.2.0;
    use wasi:cli/stderr@0.2.0;

    world command {
        export run: func() -> result<(), exit::code>;
    }
}
```

### WASI Preview 3

```wit
// Preview 3 adds more capabilities
package wasi:io@0.3.0;

interface streams {
    /// Input stream for reading
    type input-stream;

    /// Output stream for writing
    type output-stream;

    /// Read bytes from input
    read: func(input: borrow<input-stream>, len: u64)
        -> result<tuple<list<u8>, stream-error>>;

    /// Write bytes to output
    write: func(output: borrow<output-stream>, contents: list<u8>)
        -> result<u64, stream-error>;

    /// Flush output buffer
    flush: func(output: borrow<output-stream>) -> result<(), stream-error>;
}
```

## WASI Proposals

### wasi-nn (Neural Networks)

```wit
interface wasi-nn {
    /// Load a neural network model
    load: func(
        graph: list<u8>,
        encoding: graph-encoding,
        execution: execution-target
    ) -> result<graph-handle, error>;

    /// Initialize execution context
    init-execution: func(graph: graph-handle, batch: u32)
        -> result<execution-context, error>;

    /// Set input tensor
    set-input: func(
        context: execution-context,
        index: u32,
        tensor: tensor
    ) -> result<(), error>;

    /// Compute inference
    compute: func(context: execution-context) -> result<(), error>;

    /// Get output tensor
    get-output: func(
        context: execution-context,
        index: u32
    ) -> result<tensor, error>;
}

enum graph-encoding {
    openvino,
    onnx,
    tensorflow-lite,
    torch,
}

enum execution-target {
    cpu,
    gpu,
    tpu,
}
```

### wasi-crypto

```wit
interface wasi-crypto {
    /// Generate a key pair
    keypair-generate: func(algorithm: algorithm)
        -> result<keypair, error>;

    /// Import a key
    key-import: func(
        algorithm: algorithm,
        encoded: list<u8>,
        format: key-format
    ) -> result<key, error>;

    /// Sign a message
    signature-sign: func(
        key: key,
        message: list<u8>
    ) -> result<signature, error>;

    /// Verify a signature
    signature-verify: func(
        key: key,
        message: list<u8>,
        signature: signature
    ) -> result<bool, error>;

    /// Encrypt data
    encrypt: func(
        key: key,
        plaintext: list<u8>,
        options: encryption-options
    ) -> result<list<u8>, error>;

    /// Decrypt data
    decrypt: func(
        key: key,
        ciphertext: list<u8>,
        options: encryption-options
    ) -> result<list<u8>, error>;
}
```

### wasi-sockets

```wit
interface wasi-sockets {
    /// Create a network socket
    socket: func(
        address-family: address-family,
        socket-type: socket-type,
        protocol: protocol
    ) -> result<fd, error-code>;

    /// Connect to a remote address
    connect: func(fd: fd, remote: socket-address)
        -> result<(), error-code>;

    /// Bind to a local address
    bind: func(fd: fd, local: socket-address)
        -> result<(), error-code>;

    /// Listen for connections
    listen: func(fd: fd, backlog: u32)
        -> result<(), error-code>;

    /// Accept a connection
    accept: func(fd: fd)
        -> result<tuple<fd, socket-address>, error-code>;

    /// Send data
    send: func(fd: fd, data: list<u8>)
        -> result<u64, error-code>;

    /// Receive data
    recv: func(fd: fd, buf-size: u64)
        -> result<list<u8>, error-code>;
}
```

## Capability-Based Security

### Resource Handles

```rust
// WASI uses file descriptors as capabilities
// No implicit access - everything must be granted

// Example: Module can only access pre-opened directories
let wasi = WasiCtxBuilder::new()
    // Only these directories are accessible
    .preopen_dir("/host/project", "/project")?
    .preopen_dir("/host/data", "/data")?
    .build();

// Inside WASM module:
// Can open files in /project but NOT /etc or /
let f = File::open("/project/src/main.rs")?;  // OK
let f = File::open("/etc/passwd")?;           // ERROR: Not accessible
```

### Inherited vs Isolated

```rust
// Inherit host's stdio
let wasi = WasiCtxBuilder::new()
    .inherit_stdio()  // WASM can read/write host stdio
    .build();

// Or provide isolated stdio
let wasi = WasiCtxBuilder::new()
    .stdin(my_stdin_reader)   // Custom stdin
    .stdout(my_stdout_writer) // Custom stdout
    .stderr(my_stderr_writer) // Custom stdout
    .build();
```

## Integration with Wasmtime

### Basic Setup

```rust
use wasmtime::*;
use wasmtime_wasi::*;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, "module.wasm")?;

    // Create WASI context
    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .env("MY_VAR", "value")?
        .preopen_dir("./data", "./data")?
        .build();

    let mut store = Store::new(&engine, wasi);

    // Link WASI
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

    // Instantiate and run
    let instance = linker.instantiate(&mut store, &module)?;
    let run = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    run.call(&mut store, ())?;

    Ok(())
}
```

### Async WASI

```rust
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::WasiCtx;

let mut config = Config::new();
config.async_support(true);

let engine = Engine::new(&config)?;

let wasi = WasiCtxBuilder::new()
    .inherit_stdio()
    .build();

let mut store = Store::new(&engine, wasi);

// Async function call
let instance = linker.instantiate_async(&mut store, &module).await?;
let run = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
run.call_async(&mut store, ()).await?;
```

## Related Documents

- [Wasmtime Runtime](./wasmtime-runtime-exploration.md) - WASM runtime
- [wit-bindgen](./wit-bindgen-exploration.md) - Interface bindings
- [Wasm Tools](./wasm-tools-exploration.md) - Component tooling

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.wasmtime/WASI/`
- WASI Specification: https://github.com/WebAssembly/WASI
- Wasmtime WASI: https://docs.wasmtime.dev/api/wasmtime_wasi/
