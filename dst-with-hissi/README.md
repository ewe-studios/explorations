# DST with Hiisi: Deterministic Simulation Testing for ewe_platform

A comprehensive exploration and implementation of Deterministic Simulation Testing (DST) for Rust networking code, inspired by Turso's Hiisi project and TigerBeetle's I/O dispatch architecture.

## Overview

This project explores how to build production-grade deterministic simulation testing infrastructure for ewe_platform's foundation_core. The key insight is to abstract I/O operations behind a trait (`RawConn`) that can be implemented by:

1. **Production backend** - Real tokio-based I/O
2. **Simulation backend** - Virtual network kernel with deterministic behavior

## Contents

```
dst-with-hissi/
├── README.md                 # This file
├── exploration.md            # Architecture, patterns, design decisions
├── rust-revision.md          # Complete Rust implementation
├── examples/
│   ├── rawconn-traits/       # RawConn trait usage example
│   ├── io-dispatcher/        # Hiisi-style I/O dispatcher
│   ├── simulation-kernel/    # Deterministic simulation kernel
│   └── ewe-integration/      # Integration with ewe_platform
└── tasks.md                  # Implementation tracking
```

## Key Concepts

### I/O Dispatcher Pattern

Instead of blocking on I/O, register callbacks and process completions:

```rust
// Traditional async
async fn handle_connection(stream: TcpStream) { ... }

// Hiisi pattern (callback-based)
fn on_accept(io: &mut IO, server_sock: Rc<Socket>, ..., client_sock: Rc<Socket>, ...) {
    io.accept(server_sock, addr, on_accept);  // Re-arm
    io.recv(client_sock, on_recv);            // Queue recv
}

fn on_recv(io: &mut IO, sock: Rc<Socket>, buf: &[u8], n: usize) {
    io.send(sock, response, n, on_send);
}

// Main loop
loop {
    io.run_once();  // Process completions
}
```

### RawConn Abstraction

```rust
#[async_trait]
pub trait RawConn {
    type Stream: AsyncRead + AsyncWrite;
    type Listener: Stream<Item = io::Result<Self::Stream>>;
    
    async fn connect(addr: SocketAddr) -> io::Result<Self::Stream>;
    async fn bind_listen(addr: SocketAddr) -> io::Result<Self::Listener>;
    fn now() -> Instant;
    async fn sleep(duration: Duration);
}
```

### Deterministic Simulation

```rust
// Same code runs in both modes:
async fn my_service<C: RawConn>() {
    let stream = C::connect(addr).await?;
    // ... use stream
}

// Production: uses TokioConn
// cargo run

// Simulation: uses SimConn  
// cargo test --features simulation
```

## Examples

### 1. RawConn Traits

Demonstrates writing generic code that works with both real and simulated I/O.

```bash
cd examples/rawconn-traits
cargo run  # Production mode (tokio)
```

### 2. I/O Dispatcher

Implements the Hiisi-style callback-based I/O dispatcher with polling.

```bash
cd examples/io-dispatcher
cargo run  # Echo server using I/O dispatcher
```

### 3. Simulation Kernel

Demonstrates deterministic simulation with configurable fault injection.

```bash
cd examples/simulation-kernel
cargo run  # Simulation examples
```

## Why DST Matters

| Problem | Traditional Testing | DST |
|---------|-------------------|-----|
| Network partitions | Hard to reproduce | Simulated with seed |
| Race conditions | Flaky | Reproducible |
| Timing issues | System-dependent | Virtual time |
| Distributed bugs | Non-deterministic | Replay with seed |

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│                   Application Code                          │
│                    (unchanged)                              │
└────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────────────────────────┐
│                   RawConn Trait                             │
│              (I/O abstraction layer)                        │
└────────────────────────────────────────────────────────────┘
            │                               │
            ▼                               ▼
    ┌───────────────┐              ┌───────────────┐
    │  TokioConn    │              │   SimConn     │
    │  (production) │              │ (simulation)  │
    │               │              │               │
    │ - Real TCP    │              │ - Virtual     │
    │ - Real UDP    │              │ - Determin.   │
    │ - Real time   │              │ - Fault inj.  │
    └───────────────┘              └───────────────┘
```

## Integration with ewe_platform

### Phase 1: Define Abstractions

```rust
// foundation_core/src/io/raw_conn.rs
pub trait RawConn { ... }

// foundation_core/src/io/tokio_conn.rs  
pub struct TokioConn;
impl RawConn for TokioConn { ... }
```

### Phase 2: Implement Simulation

```rust
// foundation_core/src/io/simulation/conn.rs
pub struct SimConn;
impl RawConn for SimConn { ... }

// foundation_core/src/io/simulation/kernel.rs
pub struct SimKernel { ... }
```

### Phase 3: Feature Gate

```rust
// foundation_core/src/io/mod.rs
#[cfg(feature = "simulation")]
pub use simulation::*;

#[cfg(not(feature = "simulation"))]
pub use tokio_conn::*;
```

### Phase 4: Write Tests

```rust
#[test]
#[cfg(feature = "simulation")]
fn test_reproducible_failure() {
    let kernel = SimKernel::with_seed(12345);
    kernel.run(|| async {
        // Trigger and verify bug
    });
}
```

## Related Work

| Project | Contribution |
|---------|--------------|
| TigerBeetle | I/O dispatch, determinism |
| Hiisi (Turso) | Rust implementation |
| libxev | Event loop design |
| FoundationDB | Simulation testing |

## Getting Started

1. Read `exploration.md` for architecture and design decisions
2. Read `rust-revision.md` for implementation details
3. Run the examples to understand the patterns
4. Integrate into foundation_core following the migration guide

## License

MIT
