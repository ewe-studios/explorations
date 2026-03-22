# Taubyte Go SDK - Comprehensive Deep-Dive Exploration

**Date:** 2026-03-22
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/go-sdk/`

---

## 1. Purpose and Overview

The **Taubyte Go SDK** is a Go wrapper library that provides idiomatic Go bindings to the host functions (symbols) exported by Taubyte nodes through the Taubyte WebAssembly Virtual Machine (TVM). This SDK enables developers to write WebAssembly modules in Go that can interact with Taubyte's decentralized cloud infrastructure.

### Key Characteristics

- **Module Path:** `github.com/taubyte/go-sdk`
- **Go Version:** 1.21+
- **License:** BSD 3-Clause
- **Purpose:** Bridge between Go code and Taubyte VM host functions
- **Testing:** Full mocking support for out-of-VM testing

---

## 2. Architecture

### 2.1 Module Structure

```
go-sdk/
├── common/                 # Common utilities
│   └── vars.go            # Common variables
├── crypto/                 # Cryptography
│   └── rand/              # Random number generation
├── database/               # KV database operations
│   ├── types.go           # Database types
│   ├── new.go             # Database creation
│   └── methods.go         # CRUD operations
├── dns/                    # DNS resolution
├── errno/                  # Error handling
│   ├── types.go           # Error types
│   ├── error_strings.go   # Error messages
│   └── generate/          # Code generation
├── ethereum/               # Ethereum integration
│   └── client/            # Ethereum client
├── event/                  # Event handling
├── globals/                # Global variables
│   ├── f32/               # Float32 globals
│   ├── f64/               # Float64 globals
│   ├── u32/               # UInt32 globals
│   ├── u64/               # UInt64 globals
│   ├── str/               # String globals
│   └── scope/             # Scope globals
├── http/                   # HTTP operations
│   ├── client/            # HTTP client
│   └── event/             # HTTP event handling
├── i2mv/                   # Inter-Module Memory Views
│   ├── fifo/              # FIFO streams
│   └── memview/           # Memory views
├── ipfs/                   # IPFS operations
│   └── client/            # IPFS client
├── p2p/                    # P2P operations
│   ├── event/             # P2P events
│   └── node/              # P2P node operations
├── pubsub/                 # Publish/Subscribe
│   ├── event/             # PubSub events
│   └── node/              # PubSub node
├── self/                   # Self-reference operations
├── storage/                # Storage operations
├── utils/                  # Utilities
│   ├── booleans/          # Boolean utilities
│   ├── codec/             # Encoding/decoding
│   ├── convert/           # Type conversions
│   ├── ints/              # Integer utilities
│   └── slices/            # Slice utilities
└── mocks/                  # Mock implementations
```

### 2.2 Design Philosophy

The Go SDK follows these principles:

1. **Symbol Wrapping:** Clean wrappers around VM host functions
2. **Error Handling:** Idiomatic Go error returns with errno integration
3. **Mock Support:** Full mocking for testing outside the VM
4. **Type Safety:** Strong typing for VM resource IDs

---

## 3. Key Types, Interfaces, and APIs

### 3.1 Database Module

```go
package database

type Database uint32

type DatabaseData struct {
    database uint32
    data     []byte
}

// Create or open a database
func New(name string) (Database, error)

// Database methods
func (d Database) Get(key string) ([]byte, error)
func (d Database) Put(key string, value []byte) error
func (d Database) Delete(key string) error
func (d Database) List(prefix string) ([]string, error)
func (d Database) Close() error
```

**Implementation Pattern:**

```go
func New(name string) (Database, error) {
    var dbId uint32
    err := dbSym.NewDatabase(name, &dbId)
    if err != 0 {
        return 0, fmt.Errorf("Creating database failed: %s", err)
    }
    return Database(dbId), nil
}

func (d Database) Get(key string) ([]byte, error) {
    var dataId uint32
    err := dbSym.Get(uint32(d), key, &dataId)
    if err != 0 {
        return nil, fmt.Errorf("Get failed: %s", err)
    }
    // Read from memory view
    return readMemoryView(dataId)
}
```

### 3.2 HTTP Client Module

```go
package http/client

type HttpClient uint32
type HttpRequest struct {
    client HttpClient
    id     uint32
}
type HttpResponse struct {
    request HttpRequest
}

// Create HTTP client
func New() (HttpClient, error)

// Create request
func (c HttpClient) Request(url string, options ...HttpRequestOption) (HttpRequest, error)

// Request options
func WithMethod(method string) HttpRequestOption
func WithHeader(key, value string) HttpRequestOption
func WithBody(body []byte) HttpRequestOption
func WithTimeout(timeout time.Duration) HttpRequestOption

// Send request
func (r HttpRequest) Send() (HttpResponse, error)

// Response access
func (r HttpResponse) StatusCode() int
func (r HttpResponse) Header(key string) string
func (r HttpResponse) Body() ([]byte, error)
```

### 3.3 HTTP Event Module

For handling incoming HTTP requests:

```go
package http/event

type Event struct {
    event uint32
}

// Event methods
func (e Event) Method() string
func (e Event) Path() string
func (e Event) Query(key string) string
func (e Event) Header(key string) string
func (e Event) Body() ([]byte, error)
func (e Event) Write(data []byte) error
func (e Event) Return(status int, body []byte)
```

### 3.4 Event Module

```go
package event

type Event uint32

// Get event type
func (e Event) Type() EventType

// Type checking
func (e Event) IsHTTP() bool
func (e Event) IsPubSub() bool
```

### 3.5 PubSub Module

#### Node Operations

```go
package pubsub/node

type Channel uint32

// Open channel
func Open(name string) (Channel, error)

// Publish
func (c Channel) Publish(data []byte) error

// Subscribe
func (c Channel) Subscribe() (Event, error)
```

#### Event Operations

```go
package pubsub/event

type Event uint32

// Get channel name
func (e Event) Channel() string

// Get data
func (e Event) Data() ([]byte, error)
```

### 3.6 P2P Module

#### Node Operations

```go
package p2p/node

// Send P2P message
func Send(peerID string, protocol string, data []byte) error

// Listen for P2P messages
func Listen(protocol string, handler MessageHandler) error
```

### 3.7 Storage Module

```go
package storage

type Storage uint32

// Create storage instance
func New(projectId uint32) (Storage, error)

// Get capacity
func (s Storage) Capacity() (uint64, error)

// Get file
func (s Storage) Get(name string) (File, error)

// List files
func (s Storage) List() ([]string, error)
```

### 3.8 I2MV Module (Inter-Module Memory Views)

#### Memory Views

```go
package i2mv/memview

type ReadSeekCloser uint32
type Closer uint32

// Open memory view
func Open(id uint32) (*ReadSeekCloser, error)

// Create new memory view
func New(data []byte, persist bool) (*Closer, error)

// Read/Write/Seek
func (r *ReadSeekCloser) Read(p []byte) (int, error)
func (r *ReadSeekCloser) Write(p []byte) (int, error)
func (r *ReadSeekCloser) Seek(offset int64, whence int) (int64, error)
func (r *ReadSeekCloser) Close() error
```

#### FIFO Streams

```go
package i2mv/fifo

type WriteCloser uint32
type ReadCloser uint32

// Create FIFO
func NewFIFO() (*WriteCloser, *ReadCloser, error)

// Write
func (w *WriteCloser) Write(data []byte) (int, error)
func (w *WriteCloser) Close() error

// Read
func (r *ReadCloser) Read(p []byte) (int, error)
func (r *ReadCloser) Close() error
```

### 3.9 DNS Module

```go
package dns

type DNS uint32

// Create DNS resolver
func New() (DNS, error)

// Lookups
func (d DNS) LookupA(name string) ([]string, error)
func (d DNS) LookupAAAA(name string) ([]string, error)
func (d DNS) LookupCNAME(name string) (string, error)
func (d DNS) LookupMX(name string) ([]*MX, error)
```

### 3.10 Ethereum Module

```go
package ethereum/client

type Client uint32
type Contract uint32

// Create client
func NewClient(url string) (Client, error)

// Block operations
func (c Client) BlockByNumber(number *big.Int) (*Block, error)
func (c Client) BlockByHash(hash common.Hash) (*Block, error)

// Contract operations
func (c Client) Contract(address common.Address, abi string) (Contract, error)

// Transaction operations
func (c Client) SendTransaction(tx *types.Transaction) error
func (c Client) TransactionReceipt(hash common.Hash) (*Receipt, error)
```

### 3.11 Crypto/Rand Module

```go
package crypto/rand

// Random reader
var Reader io.Reader = &reader{}

// Read random bytes
func (r *reader) Read(p []byte) (int, error)

// Generate random value
func Int(max *big.Int) (*big.Int, error)
```

### 3.12 Globals Module

Global variables for cross-module communication:

```go
package globals/u32

// Set global
func Set(key string, value uint32) error

// Get global
func Get(key string) (uint32, error)

// Delete global
func Delete(key string) error
```

### 3.13 Errno Module

```go
package errno

type Errno uint32

// Error codes
const (
    ErrorNone Errno = iota
    ErrorEventNotFound
    ErrorBufferTooSmall
    ErrorDatabaseCreateFailed
    // ... 130+ error codes
)

// Error interface
func (e Errno) Error() string

// Check error
func IsErrno(err error, code Errno) bool
```

---

## 4. Symbol Integration

### 4.1 Host Function Imports

The SDK imports symbols from the VM:

```go
import dbSym "github.com/taubyte/go-sdk-symbols/database"

func New(name string) (Database, error) {
    var dbId uint32
    // Call host function through symbol
    err := dbSym.NewDatabase(name, &dbId)
    if err != 0 {
        return 0, fmt.Errorf("Creating database failed: %s", err)
    }
    return Database(dbId), nil
}
```

### 4.2 Symbol Pattern

```go
// In go-sdk-symbols
func NewDatabase(name string, id *uint32) uint32 {
    if mock != nil {
        return mock.NewDatabase(name, id)
    }
    // Actual host function call
    *id = taubyte_new_database(name_ptr, name_len)
    return 0
}
```

### 4.3 Mocking Support

```go
// Enable mocking
var mock MockInterface

// Set mock implementation
func SetMock(m MockInterface) {
    mock = m
}

// Mock interface
type MockInterface interface {
    NewDatabase(name string, id *uint32) uint32
    GetDatabase(dbId uint32, key string, dataId *uint32) uint32
    // ... etc
}
```

---

## 5. Dependencies

### 5.1 Core Dependencies

The Go SDK has minimal dependencies:

```go
require (
    github.com/taubyte/go-sdk-symbols  // Symbol definitions
    github.com/taubyte/go-sdk-errors   // Error handling
)
```

### 5.2 External Dependencies

For specific modules:
- `golang.org/x/crypto` - Cryptography
- Standard library for most operations

---

## 6. Integration with Taubyte Components

### 6.1 VM Integration

The Go SDK interfaces with TVM through host function symbols:

```
Go SDK
    ├── go-sdk-symbols (symbol wrappers)
    │   └── Host function calls
    │       └── TVM (wazero runtime)
    │           └── WebAssembly module
    │
    └── go-sdk-errors (error definitions)
```

### 6.2 Cross-SDK Compatibility

The Go SDK is the reference implementation:
- Rust SDK mirrors Go SDK structure
- AssemblyScript SDK follows same patterns
- Error codes are consistent across SDKs

### 6.3 Used By

| Component | Usage |
|-----------|-------|
| SmartOps | Business logic |
| HTTP Handlers | Request processing |
| Database Operations | Data storage |
| P2P Applications | Distributed apps |

---

## 7. Production Usage Patterns

### 7.1 HTTP Handler Example

```go
package main

import (
    "github.com/taubyte/go-sdk/event"
    "github.com/taubyte/go-sdk/http/event"
)

//export handler
func handler(eventId uint32) {
    e := event.Event{event: eventId}

    path := e.Path()
    switch path {
    case "/api/data":
        handleData(&e)
    case "/api/health":
        handleHealth(&e)
    default:
        handle404(&e)
    }
}

func handleData(e *http.Event) {
    method := e.Method()
    if method == "POST" {
        body, _ := e.Body()
        // Process body...
    }

    e.Return(200, []byte("OK"))
}
```

### 7.2 Database Example

```go
package main

import (
    "github.com/taubyte/go-sdk/database"
)

func storeData(key string, value []byte) error {
    db, err := database.New("my-store")
    if err != nil {
        return err
    }
    defer db.Close()

    return db.Put(key, value)
}

func loadData(key string) ([]byte, error) {
    db, err := database.New("my-store")
    if err != nil {
        return nil, err
    }
    defer db.Close()

    return db.Get(key)
}
```

### 7.3 HTTP Client Example

```go
package main

import (
    "github.com/taubyte/go-sdk/http/client"
)

func fetchAPI() error {
    c, err := http.New()
    if err != nil {
        return err
    }

    req, err := c.Request("https://api.example.com/data",
        http.WithMethod("GET"),
        http.WithHeader("Accept", "application/json"),
    )
    if err != nil {
        return err
    }

    resp, err := req.Send()
    if err != nil {
        return err
    }

    if resp.StatusCode() != 200 {
        return fmt.Errorf("API returned %d", resp.StatusCode())
    }

    body, err := resp.Body()
    // Process body...

    return nil
}
```

### 7.4 PubSub Example

```go
package main

import (
    "github.com/taubyte/go-sdk/pubsub/node"
)

func publishMessage(channel string, data []byte) error {
    ch, err := node.Open(channel)
    if err != nil {
        return err
    }

    return ch.Publish(data)
}

func subscribeToChannel(channel string) error {
    ch, err := node.Open(channel)
    if err != nil {
        return err
    }

    event, err := ch.Subscribe()
    if err != nil {
        return err
    }

    data, err := event.Data()
    // Process data...

    return nil
}
```

### 7.5 Storage Example

```go
package main

import (
    "github.com/taubyte/go-sdk/storage"
)

func storeFile(name string, data []byte) error {
    s, err := storage.New(projectId)
    if err != nil {
        return err
    }

    f, err := s.Get(name)
    if err != nil {
        return err
    }

    _, err = f.Write(data)
    return err
}
```

---

## 8. Testing

### 8.1 Unit Testing with Mocks

```go
package main

import (
    "testing"
    "github.com/taubyte/go-sdk/database"
    "github.com/taubyte/go-sdk/mocks"
)

func TestDatabase(t *testing.T) {
    // Set up mock
    mockDB := mocks.NewDatabaseMock()
    mockDB.On("NewDatabase", "test-store", mock.Anything).Return(uint32(0))
    mockDB.On("Get", mock.Anything, "key", mock.Anything).Return(uint32(0))

    database.SetMock(mockDB)

    // Test code
    db, err := database.New("test-store")
    if err != nil {
        t.Fatal(err)
    }

    data, err := db.Get("key")
    if err != nil {
        t.Fatal(err)
    }

    // Assertions...
}
```

### 8.2 Integration Testing

```go
// Test inside VM with tau-cli
// $ tau test ./...
```

---

## 9. Error Handling

### 9.1 Error Pattern

```go
func operation() error {
    var result uint32
    err := symbol.Call(&result)
    if err != 0 {
        return fmt.Errorf("operation failed: %s", errno.Errno(err))
    }
    return nil
}
```

### 9.2 Error Codes

Key error categories:
- **Event errors:** 1-10
- **HTTP errors:** 11-30
- **Database errors:** 31-50
- **Storage errors:** 51-70
- **PubSub errors:** 71-85
- **Ethereum errors:** 86-120
- **P2P errors:** 121-135

---

## 10. Related SDKs

| SDK | Path | Language |
|-----|------|----------|
| Rust SDK | `../rust-sdk/` | Rust |
| AssemblyScript SDK | `../assemblyscript-sdk/` | TypeScript |
| Go SDK Symbols | `../go-sdk-symbols/` | Go (symbols) |
| Go SDK Errors | `../go-sdk-errors/` | Go (errors) |
| Go SDK SmartOps | `../go-sdk-smartops/` | Go (smart ops) |

---

## 11. Maintainers

- Sam Stoltenberg (@skelouse)
- Tafseer Khan (@tafseer-khan)
- Samy Fodil (@samyfodil)
- Aron Jalbuena (@arontaubyte)

---

## 12. Documentation References

- **Official Docs:** https://tau.how
- **GoDoc:** https://pkg.go.dev/github.com/taubyte/go-sdk
- **Repository:** github.com/taubyte/go-sdk

---

*This document was generated as part of a comprehensive Taubyte codebase exploration.*
