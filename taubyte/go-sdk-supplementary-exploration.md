# Taubyte Go SDK Supplementary Components - Deep-Dive Exploration

**Date:** 2026-03-22
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/go-sdk-*/`

---

## Overview

This document covers three supplementary Go SDK components that support the main Taubyte Go SDK:

1. **go-sdk-errors** - Centralized error definitions
2. **go-sdk-smartops** - Smart operations SDK
3. **go-sdk-symbols** - Host function symbol definitions and mocking

---

# 1. go-sdk-errors

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/go-sdk-errors/`

## 1.1 Purpose

The **go-sdk-errors** package provides centralized error code definitions shared across all Taubyte Go SDKs. This ensures consistent error handling and error code synchronization between the Go, Rust, and AssemblyScript SDKs.

## 1.2 Structure

```
go-sdk-errors/
├── errors.go          # Error code definitions
├── types.go           # Error types
└── vars.go            # Error variables
```

## 1.3 Error Code Categories

| Range | Category | Errors |
|-------|----------|--------|
| 0 | Success | ErrorNone |
| 1-10 | Event | EventNotFound, BufferTooSmall |
| 11-30 | HTTP | HttpWrite, ReadHeaders, ParseUrlFailed |
| 31-50 | Database | DatabaseCreateFailed, NotFound, DeleteFailed |
| 51-70 | Storage | CidNotFound, AddFileFailed, StorageNotFound |
| 71-85 | PubSub | SubscribeFailed, PublishFailed, ChannelNotFound |
| 86-120 | Ethereum | Client creation, contract, transaction errors |
| 121-135 | P2P | SendFailed, ProtocolNotFound, ListenFailed |
| 136-150 | Memory View | MemoryViewNotFound, SeekMethodNotFound |
| 151-160 | DNS | Lookup failures (A, CNAME, MX) |
| 161-170 | SmartOps | ResourceNotFound, WrongResourceInterface |

## 1.4 Error Type Definition

```go
package errno

type Errno uint32

const (
    ErrorNone Errno = iota
    ErrorEventNotFound
    ErrorBufferTooSmall
    ErrorAddressOutOfMemory
    ErrorHttpWrite
    ErrorHttpReadBody
    ErrorCloseBody
    ErrorEOF
    // ... 130+ error codes
)

// Error message lookup
var ERROR_STRINGS = map[Errno]string{
    ErrorNone: "Success",
    ErrorEventNotFound: "Event not found",
    ErrorBufferTooSmall: "Buffer too small",
    // ...
}

func (e Errno) Error() string {
    return ERROR_STRINGS[e]
}
```

## 1.5 Integration

Used by:
- `go-sdk` - Primary error handling
- `rust-sdk` - Error code synchronization
- `go-sdk-symbols` - Symbol error returns

---

# 2. go-sdk-smartops

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/go-sdk-smartops/`

## 2.1 Purpose

The **go-sdk-smartops** package provides a higher-level SDK for "Smart Operations" - business logic operations that combine multiple SDK features into cohesive workflows. It wraps the lower-level go-sdk with more expressive APIs.

## 2.2 Structure

```
go-sdk-smartops/
├── common/            # Common utilities
├── node/              # Node operations
├── resource/          # Resource management
└── symbols/           # SmartOp-specific symbols
```

## 2.3 Key Components

### Resource Interface

```go
package resource

type Resource interface {
    ID() string
    Type() string
    Config() interface{}
    Validate() error
}

type ResourceType string

const (
    ResourceTypeFunction ResourceType = "function"
    ResourceTypeDatabase ResourceType = "database"
    ResourceTypeStorage  ResourceType = "storage"
    ResourceTypeWebsite  ResourceType = "website"
)
```

### SmartOp Definition

```go
package smartops

type SmartOp struct {
    ID         string
    Name       string
    Version    string
    Resources  []Resource
    Handler    Handler
    Triggers   []Trigger
}

type Handler func(ctx Context) error

type Trigger struct {
    Type  TriggerType
    Config interface{}
}

type TriggerType string

const (
    TriggerHTTP   TriggerType = "http"
    TriggerPubSub TriggerType = "pubsub"
    TriggerCron   TriggerType = "cron"
)
```

### Context

```go
type Context struct {
    Event      interface{}
    Resources  ResourceManager
    Secrets    SecretManager
    Variables  VariableManager
}

func (c *Context) GetResource(name string) (Resource, error)
func (c *Context) GetSecret(name string) (string, error)
func (c *Context) GetVariable(name string) (string, error)
```

## 2.4 Usage Pattern

```go
package main

import (
    "github.com/taubyte/go-sdk-smartops"
)

func main() {
    op := smartops.NewSmartOp("my-operation")

    // Add resources
    op.AddResource(&DatabaseResource{Name: "users"})
    op.AddResource(&StorageResource{Name: "uploads"})

    // Set handler
    op.SetHandler(func(ctx smartops.Context) error {
        db, _ := ctx.GetResource("users")
        // Process...
        return nil
    })

    // Add triggers
    op.AddTrigger(smartops.Trigger{
        Type: smartops.TriggerHTTP,
        Config: HTTPConfig{
            Path:   "/api/users",
            Method: "POST",
        },
    })

    // Deploy
    op.Deploy()
}
```

## 2.5 Resource Types

### Database Resource

```go
type DatabaseResource struct {
    Name     string
    Schema   *Schema
    Indexes  []Index
}

func (d *DatabaseResource) Get(key string) ([]byte, error)
func (d *DatabaseResource) Put(key string, value []byte) error
func (d *DatabaseResource) Delete(key string) error
```

### Storage Resource

```go
type StorageResource struct {
    Name      string
    Bucket    string
    CDN       bool
}

func (s *StorageResource) Upload(filename string, data []byte) (string, error)
func (s *StorageResource) Download(path string) ([]byte, error)
func (s *StorageResource) Delete(path string) error
```

### Function Resource

```go
type FunctionResource struct {
    Name     string
    Runtime  RuntimeType
    Memory   uint32
    Timeout  time.Duration
}

func (f *FunctionResource) Invoke(params interface{}) (interface{}, error)
```

---

# 3. go-sdk-symbols

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.Taubyte/go-sdk-symbols/`

## 3.1 Purpose

The **go-sdk-symbols** package defines the low-level host function symbols that are exported by Taubyte nodes and imported by WASM modules. It provides:

1. **Symbol Definitions:** Function signatures for all host functions
2. **Mocking Support:** Test implementations for out-of-VM testing
3. **Bridge Layer:** Connection between SDKs and VM host functions

## 3.2 Structure

```
go-sdk-symbols/
├── crypto/              # Cryptographic symbols
│   └── rand/            # Random number generation
├── database/            # Database symbols
├── dns/                 # DNS symbols
├── ethereum/            # Ethereum symbols
├── event/               # Event handling symbols
├── http/                # HTTP symbols
│   ├── client/          # HTTP client symbols
│   └── event/           # HTTP event symbols
├── i2mv/                # Memory view symbols
│   ├── fifo/            # FIFO symbols
│   └── memview/         # Memory view symbols
├── p2p/                 # P2P symbols
├── pubsub/              # PubSub symbols
├── storage/             # Storage symbols
├── utils/               # Utility symbols
│   ├── booleans/        # Boolean conversion
│   ├── codec/           # Encoding/decoding
│   └── convert/         # Type conversion
└── mocks/               # Mock implementations
```

## 3.3 Symbol Pattern

```go
package database

// Mock interface
var mock MockInterface

// MockInterface defines all database host functions
type MockInterface interface {
    NewDatabase(name string, id *uint32) uint32
    GetDatabase(dbId uint32, key string, dataId *uint32) uint32
    PutDatabase(dbId uint32, key string, value []byte) uint32
    DeleteDatabase(dbId uint32, key string) uint32
    CloseDatabase(dbId uint32) uint32
    ListDatabase(dbId uint32, pattern string) ([]byte, error)
}

// Host function wrapper
func NewDatabase(name string, id *uint32) uint32 {
    if mock != nil {
        return mock.NewDatabase(name, id)
    }

    // Actual host function call via WASM import
    namePtr := uintptr(unsafe.Pointer(unsafe.StringData(name)))
    nameLen := uint32(len(name))
    return taubyte_db_new(namePtr, nameLen, uintptr(unsafe.Pointer(id)))
}

// WASM import
//go:wasmimport taubyte/sdk database_new
func taubyte_db_new(namePtr uintptr, nameLen uint32, idPtr uintptr) uint32
```

## 3.4 Mocking Support

```go
package mocks

// Database mock for testing
type DatabaseMock struct {
    mock.Mock
}

func (m *DatabaseMock) NewDatabase(name string, id *uint32) uint32 {
    args := m.Called(name, id)
    return args.Get(0).(uint32)
}

func (m *DatabaseMock) GetDatabase(dbId uint32, key string, dataId *uint32) uint32 {
    args := m.Called(dbId, key, dataId)
    return args.Get(0).(uint32)
}

// Usage in tests
func TestDatabase(t *testing.T) {
    mock := new(DatabaseMock)

    // Setup expectations
    mock.On("NewDatabase", "test-db", mock.Anything).Return(uint32(0))
    mock.On("GetDatabase", mock.Anything, "key", mock.Anything).Return(uint32(0))

    // Set mock
    database.SetMock(mock)

    // Test code
    db, err := database.New("test-db")
    data, err := db.Get("key")

    // Assertions
    mock.AssertExpectations(t)
}
```

## 3.5 Symbol Categories

### Database Symbols

```go
// taubyte/sdk imports
taubyte_db_new       func(namePtr, nameLen, idPtr uintptr) uint32
taubyte_db_get       func(dbId, keyPtr, keyLen, dataIdPtr uintptr) uint32
taubyte_db_put       func(dbId, keyPtr, keyLen, valuePtr, valueLen uintptr) uint32
taubyte_db_delete    func(dbId, keyPtr, keyLen uintptr) uint32
taubyte_db_close     func(dbId uintptr) uint32
taubyte_db_list      func(dbId, patternPtr, patternLen, resultPtr uintptr) uint32
```

### HTTP Client Symbols

```go
taubyte_http_client_new        func(clientIdPtr uintptr) uint32
taubyte_http_request_new       func(clientId, requestIdPtr uintptr) uint32
taubyte_http_request_set_url   func(clientId, requestId, urlPtr, urlLen uintptr) uint32
taubyte_http_request_set_method func(clientId, requestId, methodPtr, methodLen uintptr) uint32
taubyte_http_request_send      func(clientId, requestId uintptr) uint32
taubyte_http_response_status   func(clientId, requestId, statusPtr uintptr) uint32
taubyte_http_response_body     func(clientId, requestId, bodyIdPtr uintptr) uint32
```

### Memory View Symbols

```go
taubyte_memview_new      func(dataPtr, dataLen uintptr, persist bool, idPtr uintptr) uint32
taubyte_memview_open     func(id, handlePtr uintptr) uint32
taubyte_memview_read     func(handle, bufferPtr, bufferLen, bytesReadPtr uintptr) uint32
taubyte_memview_write    func(handle, dataPtr, dataLen, bytesWrittenPtr uintptr) uint32
taubyte_memview_seek     func(handle, offset int64, whence int, newPosPtr uintptr) uint32
taubyte_memview_close    func(handle uintptr) uint32
```

### Event Symbols

```go
taubyte_event_type       func(eventId, typePtr uintptr) uint32
taubyte_event_http_method func(eventId, methodPtr uintptr) uint32
taubyte_event_http_path  func(eventId, pathPtr uintptr) uint32
taubyte_event_http_query func(eventId, queryPtr, queryLen, resultPtr uintptr) uint32
taubyte_event_http_write func(eventId, dataPtr, dataLen uintptr) uint32
taubyte_event_return     func(eventId, status uint32, bodyPtr, bodyLen uintptr) uint32
```

## 3.6 Integration with TVM

```
WASM Module (Go/Rust/AssemblyScript)
    │
    ├── Imports host functions
    │   └── go-sdk-symbols wrappers
    │       ├── mock (if testing)
    │       └── taubyte_* imports (production)
    │
    └── TVM (wazero runtime)
        └── Host function implementations
            └── taubyte_* exports
```

## 3.7 Cross-Language Consistency

The symbols defined here are mirrored in:
- **Rust SDK:** `extern "C" fn taubyte_*` declarations
- **AssemblyScript SDK:** `@external("taubyte/sdk", "...")` declarations

---

# 4. Component Relationships

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
├─────────────────────────────────────────────────────────────┤
│  go-sdk-smartops (High-level business logic SDK)            │
├─────────────────────────────────────────────────────────────┤
│  go-sdk (Mid-level SDK with idiomatic Go APIs)              │
├─────────────────────────────────────────────────────────────┤
│  go-sdk-symbols (Low-level symbol wrappers + mocks)         │
├─────────────────────────────────────────────────────────────┤
│  go-sdk-errors (Shared error codes across all SDKs)         │
├─────────────────────────────────────────────────────────────┤
│                    TVM (wazero runtime)                     │
│                    Host Function Exports                    │
└─────────────────────────────────────────────────────────────┘
```

---

# 5. Related Components

| Component | Path | Description |
|-----------|------|-------------|
| go-sdk | `../go-sdk/` | Main Go SDK |
| rust-sdk | `../rust-sdk/` | Rust SDK (uses same errors) |
| assemblyscript-sdk | `../assemblyscript-sdk/` | TypeScript SDK |
| vm | `../vm/` | WebAssembly runtime |

---

# 6. Maintainers

- Sam Stoltenberg (@skelouse)
- Tafseer Khan (@tafseer-khan)
- Samy Fodil (@samyfodil)
- Aron Jalbuena (@arontaubyte)

---

*This document was generated as part of a comprehensive Taubyte codebase exploration.*
