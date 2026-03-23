# Go Cap'n Proto (go-capnp) Exploration

location: /home/darkvoid/Boxxed/@formulas/src.rust/src.RPC/go-capnp
repository: https://github.com/capnproto/go-capnp
explored_at: 2026-03-23

## Overview

go-capnp is the official Go implementation of Cap'n Proto. It provides Go developers with zero-copy serialization and capability-based RPC, following the same protocol specification as the C++ reference implementation.

## Project Structure

```
go-capnp/
├── capnp/                 # Main package (this directory)
│   ├── arena.go           # Memory arena management
│   ├── codec.go           # Wire format encoding/decoding
│   ├── list.go            # List type support
│   ├── message.go         # Message handling
│   ├── pointer.go         # Pointer operations
│   ├── rawpointer.go      # Low-level pointer access
│   ├── segment.go         # Segment management
│   ├── struct.go          # Struct type support
│   ├── capability.go      # Capability/RPC support
│   ├── answer.go          # RPC answer handling
│   ├── request.go         # RPC request handling
│   └── doc.go             # Package documentation
├── capnpc-go/             # Schema compiler plugin
│   ├── capnpc-go.go       # Main compiler
│   ├── templates/         # Go code templates
│   └── testdata/          # Test schemas
├── rpc/                   # RPC implementation
│   ├── rpc.go             # RPC protocol
│   ├── conn.go            # Connection handling
│   └── transport.go       # Transport interfaces
├── server/                # RPC server utilities
├── flowcontrol/           # Flow control for RPC
├── pogs/                  # Plain Old Go Structs (POGOs)
├── packed/                # Packed encoding (compression)
├── encoding/text/         # Text encoding
├── example/               # Example applications
├── docs/                  # Documentation
│   ├── Getting-Started.md
│   ├── Installation.md
│   ├── Remote-Procedure-Calls-using-Interfaces.md
│   ├── Working-with-Capn-Proto-Types.md
│   └── Writing-Schemas-and-Generating-Code.md
└── go.mod
```

## Core Package: capnp

### Module Definition

```go
module capnproto.org/go/capnp/v3

go 1.22

require (
    github.com/stretchr/testify v1.8.4
    golang.org/x/exp v0.0.0-20240506185415-9bf2ced13842
)
```

### Memory Arena

```go
// Arena manages memory segments
type Arena interface {
    // Get segment by ID
    GetSegment(id SegmentID) (*Segment, error)

    // Allocate new segment
    AllocateSegment(minSize int) (*Segment, error)
}

// SingleSegment is a simple arena with one segment
type SingleSegment struct {
    data []byte
}
```

### Message Handling

```go
// Message represents a Cap'n Proto message
type Message struct {
    arena Arena
    mu    sync.Mutex
    caps  []*capTableEntry
}

// NewMessage creates a message with the given arena
func NewMessage(arena Arena) (*Message, error)

// NewRootStruct creates a new root struct
func (m *Message) NewRootStruct(s TypeStruct) (*Struct, error)
```

### Segment Management

```go
// Segment is a contiguous memory region
type Segment struct {
    msg  *Message
    data []byte
    id   SegmentID
}

// First returns the first word in the segment
func (s *Segment) First() Word

// Len returns the segment length in words
func (s *Segment) Len() int
```

### Pointer Types

```go
// Pointer represents a Cap'n Proto pointer
type Pointer interface {
    pointerType() pointerType
    segment() *Segment
    offset() int32
}

// StructPtr points to a struct
type StructPtr struct {
    seg *Segment
    off int32
}

// ListPtr points to a list
type ListPtr struct {
    seg *Segment
    off int32
}

// InterfacePtr points to a capability
type InterfacePtr struct {
    seg *Segment
    off int32
}
```

### Codec (Wire Format)

```go
// Wire format constants
const (
    StructPointer   = 0
    ListPointer     = 1
    FarPointer      = 2
    CapabilityPointer = 3
)

// Pointer encoding (64 bits)
// Bits 0-1: Type
// Bits 2-30: Offset (words)
// Bits 31-62: Data size / list info
// Bit 63: Reserved
```

### List Types

Generated code includes type-safe list wrappers:

```go
// List of structs
type StructList struct {
    List
    size ObjectSize
}

// List of primitives
type UInt32List List

func (l UInt32List) At(i int) uint32
func (l UInt32List) Set(i int, v uint32)

// List of capabilities
type InterfaceList List
```

### Text and Data

```go
// Text is a UTF-8 string
type Text []byte

func (t Text) String() string
func (t Text) MarshalText() ([]byte, error)

// Data is a byte array
type Data []byte

func (d Data) Bytes() []byte
```

## Schema Compiler: capnpc-go

### Installation

```bash
go install capnproto.org/go/capnp/v3/capnpc-go@latest
```

### Usage

```bash
capnp compile -ogo schema.capnp
```

### Build Integration

```go
//go:generate capnp compile -ogo schema.capnp
package main
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

```go
package schema

import (
    capnp "capnproto.org/go/capnp/v3"
)

// Point is the generated type
type Point capnp.Struct

// NewPoint creates a new Point
func NewPoint(s *capnp.Segment) (Point, error)

// Reader methods
func (p Point) X() float32
func (p Point) Y() float32

// Builder methods
func (p Point) SetX(v float32) error
func (p Point) SetY(v float32) error

// List type
type Point_List = capnp.StructList[Point]

// String representation
func (p Point) String() string
func (p Point) MarshalText() ([]byte, error)

// Schema for reflection
var Schema_Point = capnp.Schema{/*...*/}
```

### Template System

Uses Go templates for code generation:

```
templates/
├── structFuncs       # Struct function templates
├── structDataField   # Data field accessors
├── structPointerField # Pointer field accessors
├── structInterfaceField # Interface fields
├── structListField   # List fields
├── interfaceClient   # RPC client stubs
├── interfaceServer   # RPC server skeletons
└── schemaVar         # Schema metadata
```

## RPC System

### RPC Package

```go
package rpc

import (
    "capnproto.org/go/capnp/v3"
    "capnproto.org/go/capnp/v3/exc"
)
```

### Connection

```go
// Conn represents an RPC connection
type Conn struct {
    transport Transport
    questions *questionTable
    answers   *answerTable
    imports   *importTable
    exports   *exportTable
}

// NewConn creates a new connection
func NewConn(t Transport, opts *Options) *Conn
```

### Transport Interface

```go
// Transport sends/receives RPC messages
type Transport interface {
    NewMessage(ctx context.Context) (OutgoingMessage, error)
    RecvMessage(ctx context.Context) (IncomingMessage, error)
    Close() error
}
```

### Capability System

```go
// Client is a capability client
type Client struct {
    hook *hook
}

// Call invokes a method on the capability
func (c Client) Call(ctx context.Context, call Context) (*Answer, ReleaseFunc)

// Answer is the result of a call
type Answer struct {
    // ...
}

// ReleaseFunc releases resources
type ReleaseFunc func()
```

### Questions and Answers

```go
// Question represents an outgoing call
type question struct {
    id       QuestionID
    cap      *Client
    method   Method
    ctx      context.Context
    response chan response
}

// Answer handles the response
type answer struct {
    err error
    result capnp.Ptr
}
```

### Exports and Imports

```go
// exportState tracks exported capabilities
type exportState struct {
    cap    *Client
    refs   uint32
    ready  bool
}

// importState tracks imported capabilities
type importState struct {
    cap    *Client
    refs   uint32
}
```

## Flow Control

```go
package flowcontrol

// FlowLimiter limits outstanding bytes
type FlowLimiter struct {
    maxOutstanding int64
    current        atomic.Int64
    ready          chan struct{}
}

// Reserve bytes for sending
func (f *FlowLimiter) Reserve(ctx context.Context, bytes int64) error

// Release bytes after sending
func (f *FlowLimiter) Release(bytes int64)
```

## POGs (Plain Old Go Structs)

POGs provide a simpler API using regular Go structs:

```go
package pogs

// Insert extracts fields from a Cap'n Proto struct
func Insert(s capnp.Struct, schema *capnp.Schema, goStruct interface{}) error

// Extract fills a Cap'n Proto struct from Go struct
func Extract(s capnp.Struct, schema *capnp.Schema, goStruct interface{}) error
```

### Usage Example

```go
type Person struct {
    Id    uint32 `capnp:"id"`
    Name  string `capnp:"name"`
    Email string `capnp:"email"`
}

// Read from Cap'n Proto
var p Person
pogs.Insert(msg, schema, &p)

// Write to Cap'n Proto
pogs.Extract(msg, schema, p)
```

## Packed Encoding

Compression for storage:

```go
package packed

// Pack compresses a message
func Pack(r io.Reader, w io.Writer) error

// Unpack decompresses a message
func Unpack(r io.Reader, w io.Writer) error
```

### Encoding Format

- Run-length encoding for zeros
- Non-zero runs stored verbatim
- Effective for sparse data

## Documentation Guides

### Getting Started

1. Install capnp tool and capnpc-go
2. Write schema file
3. Generate Go code
4. Use generated types

### Working with Cap'n Proto Types

```go
// Create message
seg, _ := capnp.NewSingleSegmentMessage(nil)
point, _ := NewPoint(seg.Message())

// Set fields
point.SetX(1.0)
point.SetY(2.0)

// Get fields
x := point.X()
y := point.Y()

// Lists
list, _ := capnp.NewStructList(seg, 8, 10)
list.Set(0, point)
```

### RPC with Interfaces

```capnp
interface Calculator {
    add @0 (a :Float64, b :Float64) -> (result :Float64);
}
```

```go
// Client
client := Calculator_Client(conn.Bootstrap(ctx))
resp, release := client.Add(ctx, func(req Calculator_add) error {
    req.SetA(1.0)
    req.SetB(2.0)
    return nil
})
defer release()
result, _ := resp.Result()
fmt.Println(result.Result())
```

## Error Handling

### Exception Types

```go
package exc

type Type uint32

const (
    Failed       Type = 0
    Overloaded   Type = 1
    Unimplemented Type = 2
    Disconnected Type = 3
)

// New creates an exception
func New(t Type, msg string) Exception

// WrapError wraps Go error as exception
func WrapError(err error) Exception
```

### RPC Errors

```go
// Answer can contain error
answer, release := client.Call(ctx, call)
defer release()

if answer.Err() != nil {
    // Handle error
}
```

## Examples

### Address Book

```go
// Example from examples/addressbook
func main() {
    // Create message
    seg, cleanup, _ := capnp.NewMessage(capnp.SingleSegment(nil))
    defer cleanup()

    // Create address book
    ab, _ := NewAddressBook(seg.Message())

    // Add person
    people, _ := ab.NewPeople(1)
    person := people.At(0)
    person.SetId(1)
    person.SetName("Alice")
    person.SetEmail("alice@example.com")
}
```

### Hash Calculator

```go
// Example from examples/hashes
interface Hasher {
    md5 @0 (data :Data) -> (hash :Text);
    sha256 @1 (data :Data) -> (hash :Text);
}
```

## Testing

### Unit Tests

```go
func TestPoint(t *testing.T) {
    seg, _ := capnp.NewSingleSegmentMessage(nil)
    point, _ := NewPoint(seg.Message())

    point.SetX(1.0)
    point.SetY(2.0)

    if point.X() != 1.0 {
        t.Errorf("Expected X=1.0, got %v", point.X())
    }
}
```

### Integration Tests

```go
func TestRPC(t *testing.T) {
    // Create pipe
    p1, p2 := net.Pipe()

    // Setup server and client
    // Test RPC calls
}
```

### Benchmark Tests

```go
func BenchmarkSerialize(b *testing.B) {
    for i := 0; i < b.N; i++ {
        // Serialize benchmark
    }
}
```

## Performance Considerations

### Zero-Copy Design

```go
// No allocation for reads
func (p Point) X() float32 {
    return math.Float32frombits(p(seg).data.GetUint32(0))
}
```

### Arena Options

```go
// Single segment (fastest for small messages)
arena := capnp.SingleSegment(nil)

// Multi-segment (better for large messages)
arena := capnp.MultiSegment(nil)
```

### Buffer Reuse

```go
// Reuse buffers with sync.Pool
var pool = sync.Pool{
    New: func() interface{} {
        return make([]byte, 4096)
    },
}
```

## Security Considerations

### Read Limits

```go
// Limit message size
opts := &capnp.MessageOptions{
    TraversalLimit: 8 * 1024 * 1024,  // 8 MB
    NestingLimit: 64,
}
```

### Capability Security

- Unforgeable capability references
- No ambient authority
- Fine-grained access control

## Dependencies

### Runtime

- Go 1.22+
- No external dependencies (standard library only)

### Development

- `github.com/stretchr/testify`: Testing
- `golang.org/x/exp`: Experimental packages

## Comparison with Other Go RPC

| Feature | go-capnp | gRPC-Go | net/rpc |
|---------|----------|---------|---------|
| Wire format | Cap'n Proto | Protobuf | Gob |
| Zero-copy | Yes | No | No |
| Schema | .capnp | .proto | Go types |
| Streaming | Yes | Yes | No |
| Capabilities | Yes | No | No |

## Resources

- [GoDoc](https://pkg.go.dev/capnproto.org/go/capnp/v3)
- [Getting Started Guide](docs/Getting-Started.md)
- [Installation Guide](docs/Installation.md)
- [RPC Guide](docs/Remote-Procedure-Calls-using-Interfaces.md)
