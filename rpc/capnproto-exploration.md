# Cap'n Proto (C++) Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC/capnproto
repository: https://github.com/capnproto/capnproto
explored_at: 2026-03-23

## Overview

Cap'n Proto C++ is the reference implementation of the Cap'n Proto serialization and RPC system. Created by Kenton Varda (designer of Protocol Buffers at Google), Cap'n Proto was designed to be "infinity times faster" than Protocol Buffers by eliminating parsing and serialization.

## Project Structure

```
capnproto/
├── c++/                    # C++ implementation
│   ├── src/
│   │   ├── capnp/         # Core library
│   │   │   ├── rpc.capnp          # RPC protocol schema
│   │   │   ├── schema.capnp       # Schema reflection
│   │   │   └── persistent.capnp   # Persistent capabilities
│   │   ├── kj/            # KJ library (C++ utility library)
│   │   └── capnpc/        # Schema compiler plugin
│   └── samples/           # Example schemas
├── doc/                   # Documentation and blog posts
└── security-advisories/   # Security announcements
```

## Key Components

### KJ Library

The KJ library is a C++ utility library that provides:

- **Promise/Async framework**: Asynchronous programming model
- **I/O abstractions**: Streams, timers, networking
- **Memory management**: Arena allocation, blob handling
- **Error handling**: Exception system with stack traces

### Core Cap'n Proto Library

The `capnp` directory contains:

1. **Message handling**: `message.h/cpp` - Multi-segment message management
2. **Serialization**: `serialize.h/cpp` - Wire format encoding/decoding
3. **Layout**: `layout.h/cpp` - Memory layout algorithms
4. **Arena**: `arena.h/cpp` - Segment allocation

### RPC System

The RPC implementation is defined in `rpc.capnp` and includes:

#### Message Types

```capnp
struct Message {
    union {
        uninitialized @0 :Void;
        abort @1 :Exception;
        bootstrap @2 :Bootstrap;           # Level 0: Get initial interface
        accept @3 :Accept;                 # Level 3: Three-way handoff

        call @4 :Call;                     # Method call
        return @5 :Return;                 # Call response
        finish @6 :Finish;                 # Release call resources

        resolve @7 :Resolve;               # Resolve promise
        release @8 :Release;               # Release capability
        obsoleteSave @9 :Save;             # Level 2: Save capability
        obsoleteDelete @10 :Delete;        # Level 2: Delete capability

        provide @11 :Provide;              # Level 3: Provide capability
        acceptFromThirdParty @12 :AcceptFromThirdParty;

        join @13 :Join;                    # Level 4: Join check
    }
}
```

#### Call Structure

```capnp
struct Call {
    questionId @0 :QuestionId;             # Unique call identifier
    target @1 :CapDescriptor;              # Object to call
    interfaceId @2 :UInt64;                # Interface type
    methodId @3 :UInt16;                   # Method number
    allowThirdPartyTailCall @4 :Bool;      # Level 3 optimization
    params @5 :Payload;                    # Input arguments
}
```

## Wire Format

### Segment Layout

Cap'nProto messages consist of one or more segments:

```
┌─────────────────┬─────────────────┬─────────────────┬─────────────────┐
│  Segment Table  │   Segment 0     │   Segment 1     │   Segment N     │
│  (variable)     │   (variable)    │   (variable)    │   (variable)    │
└─────────────────┴─────────────────┴─────────────────┴─────────────────┘
```

### Segment Header

```
┌─────────────────┬─────────────────┐
│ Word 0:         │ Word 1+:        │
│ Segment count-1 │ Segment sizes   │
│ (or 0 for >16)  │ (in words)      │
└─────────────────┴─────────────────┘
```

For messages with >16 segments, a far pointer table is used.

### Pointer Representation

Pointers are 64-bit values with this structure:

```
┌─────────────────────────────────────────────────────────────┐
│ 0:2   | Kind (0=struct, 1=list, 2=far, 3=capability)       │
│ 3:30  | Offset (from pointer location, in words)           │
│ 31:62 | Data size (for structs) or list info               │
│ 63    | Reserved                                           │
└─────────────────────────────────────────────────────────────┘
```

### Struct Layout

```
┌─────────────────────┬─────────────────────┐
│   Data Section      │   Pointer Section   │
│   (fixed size)      │   (variable)        │
└─────────────────────┴─────────────────────┘
```

- Data section contains primitive types (integers, floats, enums)
- Pointer section contains pointers to nested structs, lists, text, data

### List Encoding

Lists are stored inline with element count followed by elements:

```
┌─────────────────┬───────────────────────────────────────────┐
│ Element count   │   Elements (padded to 64-bit boundary)   │
└─────────────────┴───────────────────────────────────────────┘
```

## RPC Protocol Details

### Question/Answer Protocol

```
Client                              Server
   │                                  │
   │─── Call (questionId=N) ─────────>│  Register in question table
   │                                  │
   │                                  │  Execute call
   │                                  │
   │<── Return (answerId=N) ──────────│  Register in answer table
   │                                  │
   │─── Finish (questionId=N) ───────>│  Release resources
   │                                  │
```

### Export/Import Reference Counting

```capnp
struct Release {
    id @0 :ExportId;                   # Capability to release
    referenceCount @1 :UInt32;         # Amount to decrement
}
```

Reference counting rules:
- Each time an ExportId is sent, increment refcount
- Send Release when done (can batch multiple releases)
- When refcount reaches 0, capability can be reclaimed

### Promise Pipelining

```capnp
struct Call {
    # ...
    target :CapDescriptor;
}

union CapDescriptor {
    none @0 :Void;
    senderHosted @1 :ExportId;         # Object sender exported
    senderPromise @2 :ExportId;        # Promise (will resolve)
    receiverHosted @3 :ImportId;       # Object receiver exported
    receiverAnswer @4 :AnswerId;       # Result of receiver's call
}
```

Pipeline example:
```rust
// Single round trip: both calls sent together
let result_promise = client.method1(args);
let final_result = result_promise.method2(more_args);  // Pipelined!
```

### Three-Way Handoff (Level 3)

When Alice (Vat A) sends Carol (Vat C) to Bob (Vat B):

1. Alice sends `Provide` message to Carol with Bob's address
2. Carol connects directly to Bob
3. Bob sends `AcceptFromThirdParty` to Carol
4. Direct connection established between Bob and Carol

## Code Generation

### Schema Compiler (capnp)

The `capnp` tool compiles `.capnp` schemas:

```bash
capnp compile -oc++ schema.capnp
```

### Generated Code Structure

For each schema file, generates:
- Header (`.h`) with type definitions
- Source (`.c++`) with schema metadata

### Usage Pattern

```cpp
// Reading
Point::Reader point = message.getRoot<Point>();
float x = point.getX();
float y = point.getY();

// Writing
Point::Builder point = message.initRoot<Point>();
point.setX(1.0f);
point.setY(2.0f);
```

## Performance Features

### Zero-Copy Design

- Messages mapped directly into memory
- No parsing required - structure encoded in layout
- Readers are lightweight views (no copying)

### Arena Allocation

- Segments allocated in large chunks
- Reduces malloc overhead
- Enables efficient message building

### Alignment

- Default: 8-byte alignment for optimal access
- Unaligned mode available for special cases

## RPC Implementation Levels

| Feature | Level 0 | Level 1 | Level 2 | Level 3 | Level 4 |
|---------|---------|---------|---------|---------|---------|
| Bootstrap | ✓ | ✓ | ✓ | ✓ | ✓ |
| Object refs | - | ✓ | ✓ | ✓ | ✓ |
| Promise pipelining | - | ✓ | ✓ | ✓ | ✓ |
| Persistent caps | - | - | ✓ | ✓ | ✓ |
| Three-way handoff | - | - | - | ✓ | ✓ |
| Join | - | - | - | - | ✓ |

## Error Handling

### Exception Structure

```capnp
struct Exception {
    reason @0 :Text;
    type @1 :Type;
    trace @2 :List(Text);
    deprecatedCallId @3 :UInt32;
}

enum Type {
    failed @0;       # General failure
    overloaded @1;   # Server overloaded
    unimplemented @2;# Feature not supported
    disconnected @3; # Connection lost
}
```

### Abort Message

```capnp
struct Abort {
    type @0 :Exception.Type;
    reason @1 :Text;
}
```

Sent when connection must be terminated.

## Security Considerations

### Capability Security

- Capabilities are unforgeable references
- No ambient authority - only held capabilities can be used
- Natural least-privilege architecture

### Transport Security

- RPC layer assumes secure transport
- TLS recommended for network connections
- Capability references are sensitive

### Advisory History

Security advisories tracked in `security-advisories/` directory.

## Build System

### CMake Configuration

```cmake
cmake_minimum_required(VERSION 3.5)
project(CapnProto)

# Build options
option(WITH_TESTS "Build tests" ON)
option(WITH_OPENSSL "Use OpenSSL for TLS" OFF)
```

### Compilation Requirements

- C++17 or later
- KJ library bundled
- Optional: OpenSSL for TLS, Zstandard for compression

## Dependencies

### Runtime Dependencies

- None (pure C++17)

### Optional Dependencies

- OpenSSL: TLS support
- libzstd: Compression
- libcap: Unix capabilities

## Testing

### Test Infrastructure

- KJ test framework included
- Property-based testing for serialization
- Integration tests for RPC

### Fuzzing

- AFL fuzzing support
- Continuous fuzzing in CI

## Related Projects

- **capnproto-rust**: Rust implementation
- **go-capnp**: Go implementation
- **pycapnp**: Python bindings
- **node-capnp**: Node.js bindings
