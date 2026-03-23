# Cap'n Proto Rust Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC/capnproto-rust
repository: https://github.com/capnproto/capnproto-rust
explored_at: 2026-03-23

## Overview

Cap'n Proto for Rust is a complete Rust implementation of the Cap'n Proto serialization and RPC system. It provides zero-copy serialization, capability-based RPC, and full integration with Rust's type system and async ecosystems.

## Workspace Structure

```
capnproto-rust/
├── capnp/                # Core runtime library
│   ├── src/
│   │   ├── lib.rs
│   │   ├── arena.rs      # Segment arena management
│   │   ├── layout.rs     # Memory layout algorithms
│   │   ├── message.rs    # Message handling
│   │   ├── serialize.rs  # Wire serialization
│   │   └── schema.rs     # Runtime reflection
│   └── Cargo.toml
├── capnpc/               # Schema compiler plugin
│   ├── src/
│   │   └── compiler_capnp.rs
│   └── Cargo.toml
├── capnp-futures/        # Async IO support
│   ├── src/
│   │   ├── read.rs
│   │   └── write.rs
│   └── Cargo.toml
├── capnp-rpc/            # RPC implementation
│   ├── src/
│   │   ├── lib.rs
│   │   ├── rpc.rs
│   │   ├── local_rpc_sys.rs
│   │   └── twoparty.rs
│   └── Cargo.toml
├── async-byte-channel/   # Async transport helper
├── benchmark/            # Performance benchmarks
├── example/              # Example applications
└── Cargo.toml            # Workspace root
```

## Core Crate: capnp

### Cargo.toml

```toml
[package]
name = "capnp"
version = "0.24.0"
edition = "2021"
rust-version.workspace = true

[features]
alloc = ["embedded-io?/alloc"]
default = ["std", "alloc"]
rpc_try = []
unaligned = []       # Relaxed alignment requirements
sync_reader = []     # AtomicUsize for thread-safe readers
std = ["embedded-io?/std"]
```

### Key Types

#### Message and Arena

```rust
/// A Cap'n Proto message
pub struct Message<A: ReaderArena + Allocator> {
    arena: A,
    // ...
}

/// Arena for allocating message segments
pub trait ReaderArena: Sized {
    fn get_segment(&self, id: SegmentId) -> Option<&[u8]>;
    fn allocate_segment(&mut self, min_size: WordCount) -> Result<&[u8]>;
}
```

#### Readers and Builders

```rust
/// Borrowed read access to a struct
pub struct Reader<'a, S> {
    segment: &'a [u8],
    offset: usize,
    _marker: PhantomData<S>,
}

/// Mutable write access to a struct
pub struct Builder<'a, S> {
    segment: &'a mut [u8],
    offset: usize,
    _marker: PhantomData<S>,
}
```

### Memory Layout

Cap'nProto Rust uses the same wire format as C++:

- **Segment-based**: Messages span one or more memory segments
- **Zero-copy**: Readers borrow from underlying buffers
- **Aligned**: 8-byte alignment by default (configurable)

### Serialization

```rust
/// Serialize message to writer
pub fn serialize<M: Message>(
    message: &M,
    write: &mut impl Write,
) -> Result<()>;

/// Deserialize message from reader
pub fn deserialize<R: Read>(
    read: &mut R,
) -> Result<HeapMessage>;
```

### Embedded IO Support

The `embedded-io` feature enables `no_std` support:

```rust
#[cfg(feature = "embedded-io")]
use embedded_io::{Read, Write};
```

## Schema Compiler: capnpc

### Build Script Integration

```rust
// build.rs
fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/example.capnp")
        .run()
        .expect("schema compiler failed");
}
```

### Generated Code Structure

For a schema like:

```capnp
@0x986b3393db1396c9;

struct Point {
    x @0 :Float32;
    y @1 :Float32;
}
```

Generates:

```rust
pub mod point {
    #[derive(Clone, Copy)]
    pub struct Reader<'a> {
        reader: capnp::struct_reader::Reader<'a>,
    }

    pub struct Builder<'a> {
        builder: capnp::struct_builder::Builder<'a>,
    }

    impl<'a> Reader<'a> {
        pub fn get_x(&self) -> f32 { /* ... */ }
        pub fn get_y(&self) -> f32 { /* ... */ }
    }

    impl<'a> Builder<'a> {
        pub fn set_x(&mut self, value: f32) { /* ... */ }
        pub fn set_y(&mut self, value: f32) { /* ... */ }
    }

    pub const CAPNP_SCHEMA: capnp::schema::Node = capnp::schema::Node {
        // Schema reflection data
    };
}
```

### Code Generation Features

- **Lifetimes**: Proper borrow checking with lifetime parameters
- **Type safety**: Compile-time type checking
- **Zero-copy**: Readers hold borrowed references
- **Builder/Reader split**: Clear separation of read/write

## Async IO: capnp-futures

### Integration with Tokio

```toml
[dependencies]
capnp-futures = "0.24.0"
futures = "0.3"
tokio = { version = "1", features = ["io-util"] }
```

### Async Serialization

```rust
use capnp_futures::{serialize, message};
use futures::io::AsyncRead + AsyncWrite;

async fn send_message<W>(
    write: &mut W,
    message: &mut message::HeapMessage,
) -> capnp::Result<()>
where
    W: AsyncWrite + Unpin,
{
    serialize::write_message(write, message).await
}
```

## RPC System: capnp-rpc

### Cargo.toml

```toml
[package]
name = "capnp-rpc"
version = "0.24.0"
edition = "2021"

[dependencies]
capnp = { version = "0.24.0", path = "../capnp" }
capnp-futures = { version = "0.24.0", path = "../capnp-futures" }
futures = "0.3"
```

### RPC Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Application                       │
├─────────────────────────────────────────────────────┤
│                   Generated Code                     │
│              (from .capnp schema)                    │
├─────────────────────────────────────────────────────┤
│                  capnp-rpc Layer                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
│  │   Client    │  │   Server    │  │   Network   │  │
│  │   Stub      │  │   Skeleton  │  │   Transport │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  │
├─────────────────────────────────────────────────────┤
│                  capnp-futures                       │
├─────────────────────────────────────────────────────┤
│                      capnp Core                      │
└─────────────────────────────────────────────────────┘
```

### Two-Party RPC

```rust
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::future::FutureExt;

async fn run_rpc<V: capnp_rpc::Server>(
    transport: capnp_rpc::twoparty::VatNetwork<capnp_rpc::tls::TlsStream>,
    server: V,
) {
    let mut network = twoparty::VatNetwork::new(
        transport,
        rpc_twoparty_capnp::Side::Client,
        None,
    );

    let rpc_system = RpcSystem::new(
        network,
        Some(client::Client::new(server)),
    );

    tokio::spawn(rpc_system.map(|_| ()));
}
```

### Capability-Based RPC

```rust
// Server trait
#[trait]
interface Calculator {
    # Create a new operation
    op @0 (operator :Operator) -> Operation;
}

# An operation that can be evaluated
interface Operation {
    evaluate @0 (x :Float64) -> (result :Float64);
}

# Union of operators
union Operator {
    add @0 :Void;
    subtract @1 :Void;
    multiply @2 :Void;
    divide @3 :Void;
}
```

### Client Usage

```rust
use capnp_rpc::pry;

async fn main() -> capnp::Result<()> {
    // Get calculator from server
    let calculator = get_calculator_client();

    // Create an "add 5" operation
    let mut request = calculator.op_request();
    request.set().set_add(());
    let operation = pry!(request.send().promise.await).get_op()?;

    // Evaluate: 10 + 5 = 15
    let mut request = operation.evaluate_request();
    request.set_x(10.0);
    let response = pry!(request.send().promise.await);
    println!("Result: {}", response.get_result());

    Ok(())
}
```

### Promise Pipelining

```rust
// Pipeline calls without waiting
let op_future = calculator.op_request().send();
let operation = op_future.get_op()?;

// Use promise before resolution
let eval_future = operation.evaluate_request().send();
// Both calls sent in same round trip!
```

### Local RPC System

For in-process RPC:

```rust
use capnp_rpc::local_rpc_sys::LocalRpcSystem;

let rpc_system = LocalRpcSystem::new(server);
let client = rpc_system.bootstrap::<MyService>();
```

## Error Handling

### Result Types

```rust
pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    kind: ErrorKind,
    description: String,
    context: Vec<String>,
}

pub enum ErrorKind {
    Failed,
    Overloaded,
    Unimplemented,
    Disconnected,
    // ...
}
```

### RPC Try Macro

With `rpc_try` feature:

```rust
fn handle_request() -> capnp::Result<()> {
    let reader = capnp::rpc_try!(read_message());
    let root = capnp::rpc_try!(reader.get_root());
    Ok(())
}
```

## Features and Capabilities

### no_std Support

```toml
[features]
default = []
no_std = []  # Disables std library
```

### Unaligned Mode

```toml
[features]
unaligned = []  # Relaxed alignment
```

Useful for:
- ARM targets without alignment hardware
- Network byte order conversions
- Special memory layouts

### Reflection

Runtime schema reflection:

```rust
use capnp::schema;

let node: schema::Node = point::CAPNP_SCHEMA;
for field in node.get_fields() {
    println!("Field: {}", field.get_proto().get_name());
}
```

## Performance Considerations

### Zero-Copy Design

```rust
// No allocation - borrows from buffer
let message = serialize::read_message(&mut buffer, Default::default())?;
let reader: point::Reader = message.get_root()?;
let x = reader.get_x();  // Direct memory access
```

### Segmented Messages

- Messages can span multiple segments
- Efficient for large messages
- Avoids single large allocation

### Builder/Reader Lifetime

```rust
// Builder owns the message
let mut message = message::HeapMessage::new();
{
    let mut builder = message.init_root::<point::Builder>();
    builder.set_x(1.0);
}  // Builder dropped, message still valid

// Reader borrows from message
let reader: point::Reader = message.get_root()?;
```

## Async Runtime Integration

### Tokio Integration

```rust
use tokio::net::TcpStream;
use capnp_futures::serialize;

async fn handle_connection(stream: TcpStream) {
    let (read, write) = stream.into_split();
    let mut message = serialize::read_message(read, Default::default())
        .await
        .unwrap();
}
```

### Stream Processing

```rust
use futures::stream::StreamExt;

async fn process_stream<S>(mut stream: S)
where
    S: Stream<Item = capnp::Result<Message>> + Unpin,
{
    while let Some(message) = stream.next().await {
        // Process message
    }
}
```

## Examples

### Address Book Example

```capnp
struct Person {
    id @0 :UInt32;
    name @1 :Text;
    email @2 :Text;
    phones @3 :List(PhoneNumber);
}

struct PhoneNumber {
    number @0 :Text;
    type @1 :Type;

    enum Type {
        mobile @0;
        home @1;
        work @2;
    }
}

struct AddressBook {
    people @0 :List(Person);
}
```

### RPC Example

```capnp
interface FileSystem {
    open @0 (path :Text) -> File;
}

interface File {
    read @0 (offset :UInt64, size :UInt32) -> (data :Data);
    write @1 (offset :UInt64, data :Data);
}
```

## Build Configuration

### Workspace Configuration

```toml
[workspace]
resolver = "2"
members = [
    "capnp",
    "capnpc",
    "capnp-futures",
    "capnp-rpc",
    "async-byte-channel",
]
```

### Rust Version

```toml
[workspace.package]
rust-version = "1.70"  # MSRV
```

## Dependencies

### Core Dependencies

- None (capnp is pure Rust)

### Optional Dependencies

- `embedded-io`: For no_std embedded systems
- `quickcheck`: Property-based testing

### Dev Dependencies

- `quickcheck`: Randomized testing

## Testing

### Unit Tests

```rust
#[test]
fn test_basic_serialization() {
    let mut message = Message::new_default();
    {
        let mut root = message.init_root::<point::Builder>();
        root.set_x(1.0);
        root.set_y(2.0);
    }
    // Serialize and deserialize
    let mut buf = Vec::new();
    serialize::write_message(&mut buf, &message).unwrap();
    let message2 = serialize::read_message(&mut &buf[..], Default::default()).unwrap();
    let root2 = message2.get_root::<point::Reader>().unwrap();
    assert_eq!(root2.get_x(), 1.0);
}
```

### Property Tests

```rust
quickcheck! {
    fn roundtrip_point(x: f32, y: f32) -> bool {
        let mut message = Message::new_default();
        {
            let mut root = message.init_root::<point::Builder>();
            root.set_x(x);
            root.set_y(y);
        }
        let mut buf = Vec::new();
        serialize::write_message(&mut buf, &message).unwrap();
        let message2 = serialize::read_message(&mut &buf[..], Default::default()).unwrap();
        let root2 = message2.get_root::<point::Reader>().unwrap();
        root2.get_x() == x && root2.get_y() == y
    }
}
```

## Security Considerations

### Read Limits

```rust
let options = ReaderOptions {
    traversal_limit_in_words: 8 * 1024 * 1024,  // 8 MB
    nesting_limit: 64,
};
let message = read_message(&mut reader, options)?;
```

### Capability Security

- Capabilities are unforgeable
- No ambient authority
- Fine-grained access control

## Comparison with Other Rust RPC

| Feature | capnp-rust | tarpc | tonic |
|---------|-----------|-------|-------|
| Wire format | Cap'n Proto | Serde | Protobuf |
| Zero-copy | Yes | No | No |
| Schema | .capnp files | Rust traits | .proto files |
| Streaming | Yes | Yes | Yes |
| Async | Yes | Yes | Yes |

## Known Limitations

- **Orphans**: Not yet implemented
- **Complexity**: Steeper learning curve than serde-based RPC
- **Schema tooling**: Less mature than protobuf ecosystem

## Resources

- [Documentation](https://docs.rs/capnp/)
- [capnproto-rust blog](https://dwrensha.github.io/capnproto-rust)
- [Examples](https://github.com/capnproto/capnproto-rust/tree/master/example)
