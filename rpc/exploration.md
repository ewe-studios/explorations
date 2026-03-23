# RPC Frameworks Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC
repository: Multiple (see sub-projects)
explored_at: 2026-03-23

## Overview

This exploration covers multiple RPC (Remote Procedure Call) frameworks and serialization technologies, with a focus on Cap'n Proto and related implementations across different languages. The source directory contains 8 sub-projects representing different approaches to RPC and inter-process communication.

## Sub-Projects Surveyed

| Project | Language | Description |
|---------|----------|-------------|
| capnproto | C++ | Reference implementation of Cap'n Proto |
| capnproto-rust | Rust | Rust implementation of Cap'n Proto |
| capnweb | TypeScript | JavaScript-native RPC system |
| go-capnp | Go | Go implementation of Cap'n Proto |
| ipc-channel | Rust | Inter-process communication channels |
| pycapnp | Python | Python bindings for Cap'n Proto |
| tarpc | Rust | RPC framework with schema-in-code |
| webrender | Rust | GPU rendering engine with IPC |

## Key Technologies

### Cap'n Proto

Cap'n Proto is a type system for distributed systems that provides:

1. **Zero-copy serialization**: Data is encoded in a format suitable for in-memory traversal without deserialization
2. **Capability-based RPC**: Object-capability model for secure distributed computing
3. **Promise pipelining**: Chain multiple RPC calls in a single round trip
4. **Schema-driven code generation**: Type-safe interfaces defined in `.capnp` schema files

### RPC Protocol Levels (Cap'n Proto)

The Cap'n Proto RPC specification defines multiple implementation levels:

- **Level 0**: No object references, only bootstrap interface (similar to JSON-RPC)
- **Level 1**: Object references with promise pipelining (bilateral only)
- **Level 2**: Persistent capabilities (SturdyRef)
- **Level 3**: Three-way interactions (direct connections between vats)
- **Level 4**: Full implementation including capability joins

### Wire Format Fundamentals

Cap'n Proto uses a binary wire format with these key characteristics:

1. **Fixed-width fields**: Primitive types occupy fixed byte positions
2. **Pointer-based structures**: Complex types use pointers to data segments
3. **Multi-segment messages**: Large messages span multiple memory segments
4. **No field tags**: Field positions determined at compile time from schema

## Architecture Patterns

### The Four Tables Model

Cap'n Proto RPC connections maintain four state tables per connection:

```
┌─────────────────────────────────────────────────────────────┐
│                         Vat A                                │
│  ┌─────────────┐  ┌─────────────┐                           │
│  │  Questions  │  │   Answers   │                           │
│  │  (outgoing  │  │  (incoming  │                           │
│  │   calls)    │  │   calls)    │                           │
│  └─────────────┘  └─────────────┘                           │
│  ┌─────────────┐  ┌─────────────┐                           │
│  │   Imports   │  │   Exports   │                           │
│  │  (remote    │  │  (local     │                           │
│  │   objects)  │  │   objects)  │                           │
│  └─────────────┘  └─────────────┘                           │
└─────────────────────────────────────────────────────────────┘
                           │ │
              Connection   │ │
                           │ │
┌──────────────────────────▼ ▼────────────────────────────────┐
│                         Vat B                                │
│  ┌─────────────┐  ┌─────────────┐                           │
│  │   Answers   │  │  Questions  │                           │
│  │  (incoming  │  │  (outgoing  │                           │
│  │   calls)    │  │   calls)    │                           │
│  └─────────────┘  └─────────────┘                           │
│  ┌─────────────┐  ┌─────────────┐                           │
│  │   Exports   │  │   Imports   │                           │
│  │  (local     │  │  (remote    │                           │
│  │   objects)  │  │   objects)  │                           │
│  └─────────────┘  └─────────────┘                           │
└─────────────────────────────────────────────────────────────┘
```

### Message Flow

```
Client                          Server
   │                              │
   │──── Bootstrap ──────────────>│  (Request initial capability)
   │                              │
   │<───── Return (capability) ───│  (Receive export reference)
   │                              │
   │──── Call (importId=1) ──────>│  (Invoke method)
   │                              │
   │──── Finish ─────────────────>│  (Release question ID)
   │                              │
   │<──── Return (result) ────────│  (Return results)
   │                              │
   │──── Release (exportId=1) ───>│  (Release object reference)
   │                              │
```

## Deep Dive Documents

- [Cap'n Proto (C++)](./capnproto-exploration.md) - Reference implementation
- [Cap'n Proto Rust](./capnproto-rust-exploration.md) - Rust bindings and runtime
- [Cap'n Web](./capnweb-exploration.md) - JavaScript/TypeScript RPC
- [Go Cap'n Proto](./go-capnp-exploration.md) - Go implementation
- [IPC Channel](./ipc-channel-exploration.md) - Rust IPC
- [PyCap'n Proto](./pycapnp-exploration.md) - Python bindings
- [Tarpc](./tarpc-exploration.md) - Rust RPC framework
- [WebRender](./webrender-exploration.md) - GPU renderer with IPC

## Related Documents

- [Rust Revision Guide](./rust-revision.md) - Production-level Rust RPC implementation guide

## Performance Characteristics

### Cap'n Proto vs Protocol Buffers

Cap'n Proto claims to be "infinity times faster" than Protocol Buffers because:

1. **No parsing**: Message structure is encoded in memory layout
2. **No serialization**: Data can be read directly from wire format
3. **Zero-copy**: No intermediate buffers needed
4. **Compile-time offsets**: Field positions known at compile time

### Benchmark Considerations

- Cap'n Proto excels at large, complex messages
- For simple messages, overhead may be comparable
- Network latency often dominates serialization time
- Promise pipelining reduces round trips significantly

## WASM Usage Patterns

### Cap'n Web for WebAssembly

Cap'n Web provides JavaScript-native RPC that works well with WASM:

1. **JSON-based serialization**: Browser-compatible format
2. **HTTP batch mode**: Multiple calls in single request
3. **WebSocket mode**: Persistent bidirectional connections
4. **postMessage()**: Worker and iframe communication

### Key Considerations

- Binary protocols require careful handling in WASM
- JSON remains practical for browser environments
- Cap'n Web bridges traditional RPC and web paradigms

## Schema Compilation and Code Generation

### Cap'n Proto Schema Example

```capnp
@0x986b3393db1396c9;

struct Point {
    x @0 :Float32;
    y @1 :Float32;
}

interface PointTracker {
    addPoint @0 (p :Point) -> (totalPoints :UInt64);
}
```

### Generated Code (Rust)

- `point::Reader<'a>` - Borrowed reader with `get_x()`, `get_y()`
- `point::Builder<'a>` - Mutable builder with `set_x()`, `set_y()`
- `point_tracker::Server` - Trait for server implementation
- `point_tracker::Client` - Client stub for making calls

### Build Process

1. Write `.capnp` schema files
2. Run `capnp compile` with language plugin
3. Generated code included in build
4. Implement server traits, use client stubs

## Security Considerations

### Capability-Based Security

- Objects accessed through capabilities (unforgeable references)
- No global names - all access through explicit references
- Natural implementation of principle of least authority

### Transport Security

- Cap'n Proto relies on transport layer for encryption
- TLS commonly used for network connections
- Capability references must be protected from interception
