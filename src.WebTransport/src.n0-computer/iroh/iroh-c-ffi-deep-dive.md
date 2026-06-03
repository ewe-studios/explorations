# iroh-c-ffi Deep Dive

## Overview

`iroh-c-ffi` provides C-compatible FFI bindings for the iroh ecosystem, enabling integration with C, C++, and other languages that can interface with C libraries. The crate uses `safer-ffi` for generating safe C headers and bindings.

**Version:** 0.90.0
**Repository:** https://github.com/n0-computer/iroh
**License:** MIT OR Apache-2.0

---

## Architecture and Design Decisions

### safer-ffi Based Bindings

The crate uses [safer-ffi](https://github.com/GetRustDebug/safer-ffi) for C FFI generation:

1. **Header Generation**: Automatic C header file generation from Rust code
2. **Type Mapping**: Rust types mapped to C-compatible equivalents
3. **Memory Safety**: Clear ownership semantics for C interop
4. **Error Handling**: Error codes and string messages for C consumption

### Design Principles

1. **C ABI Compatibility**: Strict adherence to C ABI for maximum compatibility
2. **Explicit Ownership**: Clear documentation of memory ownership
3. **Opaque Types**: C code interacts with opaque pointers, not Rust structs
4. **Callback Support**: C functions can be called back from Rust

### Crate Structure

```
iroh-c-ffi/
├── src/
│   ├── lib.rs           # Main entry point, FFI exports
│   ├── addr.rs          # Address types
│   ├── endpoint.rs      # Endpoint bindings
│   ├── key.rs           # Key types
│   ├── stream.rs        # Stream bindings
│   ├── util.rs          # Utility functions
│   └── bin/
│       └── generate_headers.rs  # Header generation binary
├── Cargo.toml
└── include/
    └── irohnet.h        # Generated C header
```

### Memory Model

```
┌─────────────────────────────────────────┐
│           C Application                  │
│  - Allocates via iroh_*_new()           │
│  - Frees via iroh_*_free()              │
│  - Calls methods via iroh_*_*()         │
└─────────────────────────────────────────┘
                    │
                    │ C ABI (extern "C")
                    ▼
┌─────────────────────────────────────────┐
│         iroh-c-ffi Library               │
│  - Rust implementation                  │
│  - Memory managed by library            │
│  - Returns opaque pointers              │
└─────────────────────────────────────────┘
```

---

## Key APIs and Data Structures

### Header Generation

```rust
// lib.rs
use safer_ffi::headers;

/// Generate C header file
#[cfg(feature = "headers")]
pub fn generate_headers() -> std::io::Result<()> {
    headers::builder()
        .with_language(headers::Language::C)
        .with_naming_convention(headers::NamingConvention::Prefix("iroh".into()))
        .to_file("irohnet.h")?
        .generate()
}
```

### Error Handling

```rust
use safer_ffi::prelude::*;

/// Error codes for C API
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrohErrorCode {
    Success = 0,
    IoError = 1,
    InvalidKey = 2,
    NetworkError = 3,
    NotFound = 4,
    Cancelled = 5,
    Unknown = 255,
}

/// Error representation for C
#[ffi_export]
pub struct IrohError {
    code: IrohErrorCode,
    message: char_p::Box,
}

#[ffi_export]
impl IrohError {
    /// Get error code
    pub fn code(&self) -> IrohErrorCode {
        self.code
    }

    /// Get error message
    pub fn message(&self) -> &CStr {
        self.message.as_cstr()
    }
}

/// Free error object (caller must free)
#[ffi_export]
pub fn iroh_error_free(error: repr_c::Box<IrohError>) {
    drop(error)
}
```

### Key Types

```rust
use safer_ffi::prelude::*;

/// Node public key (opaque to C)
#[ffi_export]
pub struct IrohNodeKey {
    inner: iroh::PublicKey,
}

/// Generate new random node key
#[ffi_export]
pub fn iroh_nodekey_generate() -> repr_c::Box<IrohNodeKey> {
    let inner = iroh::SecretKey::generate().public();
    Box::new(IrohNodeKey { inner }).into()
}

/// Parse node key from hex string
#[ffi_export]
pub fn iroh_nodekey_from_hex(
    hex: &CStr,
) -> Result<repr_c::Box<IrohNodeKey>, IrohErrorCode> {
    let hex_str = hex.to_str().map_err(|_| IrohErrorCode::InvalidKey)?;
    let bytes = hex::decode(hex_str).map_err(|_| IrohErrorCode::InvalidKey)?;
    let inner = iroh::PublicKey::from_bytes(&bytes)
        .map_err(|_| IrohErrorCode::InvalidKey)?;
    Ok(Box::new(IrohNodeKey { inner }).into())
}

/// Convert node key to hex string (caller must free)
#[ffi_export]
pub fn iroh_nodekey_to_hex(key: &IrohNodeKey) -> char_p::Box {
    format!("{}", key.inner.fmt_short()).try_into().unwrap()
}

/// Free node key
#[ffi_export]
pub fn iroh_nodekey_free(key: repr_c::Box<IrohNodeKey>) {
    drop(key)
}
```

### Endpoint

```rust
/// Endpoint configuration
#[ffi_export]
pub struct IrohEndpointConfig {
    /// Optional secret key (hex string, NULL for random)
    pub secret_key: Option<char_p::Box>,
    /// Use relay
    pub use_relay: bool,
    /// ALPN protocols (NULL-terminated array)
    pub alpn_protocols: Option<c_slice::Ref<'static, char_p::Box>>,
}

/// Opaque endpoint handle
#[ffi_export]
pub struct IrohEndpoint {
    inner: iroh::Endpoint,
    runtime: tokio::runtime::Runtime,
}

/// Create endpoint builder
#[ffi_export]
pub fn iroh_endpoint_builder() -> repr_c::Box<IrohEndpointBuilder> {
    Box::new(IrohEndpointBuilder::default()).into()
}

/// Build and bind endpoint
#[ffi_export]
pub fn iroh_endpoint_build(
    config: IrohEndpointConfig,
) -> Result<repr_c::Box<IrohEndpoint>, repr_c::Box<IrohError>> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| IrohError::from(e).into())?;

    let inner = runtime.block_on(async {
        let mut builder = iroh::Endpoint::builder();

        if let Some(key) = config.secret_key {
            let secret_key = iroh::SecretKey::from_hex(key.to_str())
                .map_err(|e| IrohError::from(e))?;
            builder = builder.secret_key(secret_key);
        }

        builder.bind().await.map_err(|e| IrohError::from(e))
    })?;

    Ok(Box::new(IrohEndpoint { inner, runtime }).into())
}

/// Get node ID from endpoint
#[ffi_export]
pub fn iroh_endpoint_node_id(
    endpoint: &IrohEndpoint,
) -> repr_c::Box<IrohNodeKey> {
    let inner = endpoint.inner.node_id();
    Box::new(IrohNodeKey { inner }).into()
}

/// Connect to a node (async, returns future handle)
#[ffi_export]
pub fn iroh_endpoint_connect(
    endpoint: &IrohEndpoint,
    addr: &IrohNodeAddr,
) -> Result<repr_c::Box<IrohConnection>, repr_c::Box<IrohError>> {
    // Async operation wrapped in runtime
}

/// Free endpoint
#[ffi_export]
pub fn iroh_endpoint_free(endpoint: repr_c::Box<IrohEndpoint>) {
    drop(endpoint)
}
```

### Node Address

```rust
/// Node address for connection
#[ffi_export]
pub struct IrohNodeAddr {
    /// Node ID
    pub node_id: repr_c::Box<IrohNodeKey>,
    /// Optional relay URL (NULL if direct)
    pub relay_url: Option<char_p::Box>,
    /// Direct addresses (NULL-terminated array)
    pub direct_addresses: Option<c_slice::Ref<'static, char_p::Box>>,
}

/// Create node address from node ID
#[ffi_export]
pub fn iroh_nodeaddr_new(
    node_id: repr_c::Box<IrohNodeKey>,
) -> IrohNodeAddr {
    IrohNodeAddr {
        node_id,
        relay_url: None,
        direct_addresses: None,
    }
}

/// Set relay URL
#[ffi_export]
pub fn iroh_nodeaddr_set_relay(
    addr: &mut IrohNodeAddr,
    url: char_p::Box,
) {
    addr.relay_url = Some(url);
}

/// Add direct address
#[ffi_export]
pub fn iroh_nodeaddr_add_direct(
    addr: &mut IrohNodeAddr,
    address: char_p::Box,
) {
    // Add to direct addresses list
}

/// Free node address
#[ffi_export]
pub fn iroh_nodeaddr_free(addr: IrohNodeAddr) {
    drop(addr)
}
```

### Stream Operations

```rust
/// Opaque stream handle
#[ffi_export]
pub struct IrohStream {
    inner: tokio::sync::mpsc::Sender<Vec<u8>>,
}

/// Write data to stream
#[ffi_export]
pub fn iroh_stream_write(
    stream: &IrohStream,
    data: c_slice::Ref<u8>,
) -> Result<(), repr_c::Box<IrohError>> {
    // Send data through channel
}

/// Read data from stream (blocking)
#[ffi_export]
pub fn iroh_stream_read(
    stream: &IrohStream,
    buffer: c_slice::Mut<u8>,
) -> Result<usize, repr_c::Box<IrohError>> {
    // Read data into buffer
}

/// Free stream
#[ffi_export]
pub fn iroh_stream_free(stream: repr_c::Box<IrohStream>) {
    drop(stream)
}
```

### Callbacks

```rust
/// Callback for async operations
pub type IrohAsyncCallback = unsafe extern "C" fn(
    userdata: *mut std::ffi::c_void,
    error: *const IrohError,
    result: *const std::ffi::c_void,
);

/// Callback for receiving events
pub type IrohEventCallback = unsafe extern "C" fn(
    userdata: *mut std::ffi::c_void,
    event_type: u32,
    event_data: *const std::ffi::c_void,
);

/// Register event callback
#[ffi_export]
pub fn iroh_endpoint_set_event_callback(
    endpoint: &mut IrohEndpoint,
    callback: IrohEventCallback,
    userdata: *mut std::ffi::c_void,
) {
    // Store callback for event notifications
}
```

---

## Protocol Details

### Generated C Header

The generated `irohnet.h` header looks like:

```c
#ifndef IROHNET_H
#define IROHNET_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

// Error codes
typedef enum IrohErrorCode {
    IROH_SUCCESS = 0,
    IROH_IO_ERROR = 1,
    IROH_INVALID_KEY = 2,
    IROH_NETWORK_ERROR = 3,
    IROH_NOT_FOUND = 4,
    IROH_CANCELLED = 5,
    IROH_UNKNOWN = 255,
} IrohErrorCode;

// Opaque types
typedef struct IrohNodeKey IrohNodeKey;
typedef struct IrohEndpoint IrohEndpoint;
typedef struct IrohConnection IrohConnection;
typedef struct IrohStream IrohStream;
typedef struct IrohError IrohError;
typedef struct IrohNodeAddr IrohNodeAddr;

// Node Key functions
IrohNodeKey* iroh_nodekey_generate(void);
IrohNodeKey* iroh_nodekey_from_hex(const char* hex, IrohErrorCode* err);
char* iroh_nodekey_to_hex(const IrohNodeKey* key);
void iroh_nodekey_free(IrohNodeKey* key);

// Endpoint functions
IrohEndpoint* iroh_endpoint_build(const IrohEndpointConfig* config, IrohError** err);
IrohNodeKey* iroh_endpoint_node_id(const IrohEndpoint* endpoint);
IrohConnection* iroh_endpoint_connect(
    const IrohEndpoint* endpoint,
    const IrohNodeAddr* addr,
    IrohError** err
);
void iroh_endpoint_free(IrohEndpoint* endpoint);

// Error functions
IrohErrorCode iroh_error_code(const IrohError* error);
const char* iroh_error_message(const IrohError* error);
void iroh_error_free(IrohError* error);

#endif // IROHNET_H
```

### Memory Management

```c
// C code must follow memory management rules:

// 1. Allocate using iroh_*_new or iroh_*_generate
IrohNodeKey* key = iroh_nodekey_generate();

// 2. Use the object
char* hex = iroh_nodekey_to_hex(key);
printf("Key: %s\n", hex);

// 3. Free all allocated memory
iroh_error_free(error);  // If error was returned
free(hex);               // String allocations
iroh_nodekey_free(key);  // Object itself
```

### Error Handling Pattern

```c
IrohError* error = NULL;
IrohEndpoint* endpoint = iroh_endpoint_build(&config, &error);

if (error != NULL) {
    fprintf(stderr, "Error %d: %s\n",
            iroh_error_code(error),
            iroh_error_message(error));
    iroh_error_free(error);
    return -1;
}

// Use endpoint...
iroh_endpoint_free(endpoint);
```

---

## Integration with Main Iroh Endpoint

### C Wrapper for Rust Endpoint

```rust
#[ffi_export]
pub struct IrohEndpointBuilder {
    config: Option<iroh::endpoint::Config>,
    secret_key: Option<iroh::SecretKey>,
    relay_mode: RelayMode,
}

#[ffi_export]
impl IrohEndpointBuilder {
    /// Set secret key from hex
    pub fn secret_key(&mut self, hex: &CStr) -> Result<(), IrohErrorCode> {
        let key = iroh::SecretKey::from_hex(hex.to_str()?)?;
        self.secret_key = Some(key);
        Ok(())
    }

    /// Set relay mode
    pub fn relay_mode(&mut self, mode: RelayMode) {
        self.relay_mode = mode;
    }

    /// Build the endpoint
    pub fn build(self) -> Result<repr_c::Box<IrohEndpoint>, repr_c::Box<IrohError>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(IrohError::from)?;

        let inner = runtime.block_on(async {
            let mut builder = iroh::Endpoint::builder();

            if let Some(key) = self.secret_key {
                builder = builder.secret_key(key);
            }

            builder.relay_mode(self.relay_mode);
            builder.bind().await.map_err(IrohError::from)
        })?;

        Ok(Box::new(IrohEndpoint { inner, runtime }).into())
    }
}
```

---

## Production Usage Patterns

### C Example

```c
#include <stdio.h>
#include <stdlib.h>
#include "irohnet.h"

int main() {
    // Generate key
    IrohNodeKey* key = iroh_nodekey_generate();
    char* key_hex = iroh_nodekey_to_hex(key);
    printf("Node ID: %s\n", key_hex);

    // Build endpoint config
    IrohEndpointConfig config = {
        .secret_key = NULL,  // Random key
        .use_relay = true,
        .alpn_protocols = NULL,
    };

    // Create endpoint
    IrohError* error = NULL;
    IrohEndpoint* endpoint = iroh_endpoint_build(&config, &error);

    if (error) {
        fprintf(stderr, "Failed: %s\n", iroh_error_message(error));
        iroh_error_free(error);
        iroh_nodekey_free(key);
        free(key_hex);
        return 1;
    }

    // Get endpoint node ID
    IrohNodeKey* endpoint_id = iroh_endpoint_node_id(endpoint);
    char* endpoint_hex = iroh_nodekey_to_hex(endpoint_id);
    printf("Endpoint ID: %s\n", endpoint_hex);

    // Create node address for connection
    IrohNodeKey* target_key = iroh_nodekey_from_hex("target_hex_key", NULL);
    IrohNodeAddr addr = iroh_nodeaddr_new(target_key);

    // Connect (simplified - actual API may differ)
    IrohConnection* conn = iroh_endpoint_connect(endpoint, &addr, &error);

    // Cleanup
    iroh_nodekey_free(key);
    iroh_nodekey_free(endpoint_id);
    iroh_nodekey_free(target_key);
    free(key_hex);
    free(endpoint_hex);
    iroh_nodeaddr_free(addr);
    if (conn) iroh_connection_free(conn);
    iroh_endpoint_free(endpoint);

    return 0;
}
```

### C++ Wrapper Class

```cpp
// iroh.hpp
#pragma once
#include "irohnet.h"
#include <string>
#include <memory>
#include <stdexcept>

namespace iroh {

class Error : public std::runtime_error {
public:
    Error(IrohError* err)
        : std::runtime_error(iroh_error_message(err))
        , code_(iroh_error_code(err)) {
        iroh_error_free(err);
    }

    IrohErrorCode code() const { return code_; }

private:
    IrohErrorCode code_;
};

class NodeKey {
public:
    NodeKey() : ptr_(iroh_nodekey_generate()) {}

    explicit NodeKey(const std::string& hex) {
        IrohError* err = nullptr;
        ptr_ = iroh_nodekey_from_hex(hex.c_str(), &err);
        if (err) throw Error(err);
    }

    std::string toHex() const {
        char* hex = iroh_nodekey_to_hex(ptr_.get());
        std::string result(hex);
        free(hex);
        return result;
    }

private:
    std::unique_ptr<IrohNodeKey, decltype(&iroh_nodekey_free)> ptr_{
        nullptr, iroh_nodekey_free
    };
};

class Endpoint {
public:
    Endpoint() {
        IrohEndpointConfig config{};
        IrohError* err = nullptr;
        ptr_ = iroh_endpoint_build(&config, &err);
        if (err) throw Error(err);
    }

    NodeKey nodeId() const {
        IrohNodeKey* key = iroh_endpoint_node_id(ptr_.get());
        NodeKey result;
        iroh_nodekey_free(key);
        return result;
    }

private:
    std::unique_ptr<IrohEndpoint, decltype(&iroh_endpoint_free)> ptr_{
        nullptr, iroh_endpoint_free
    };
};

} // namespace iroh
```

### Build Integration

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.15)
project(my_iroh_app C)

# Find or build iroh-c-ffi
find_package(iroh-c-ffi REQUIRED)

# Create executable
add_executable(my_app main.c)
target_link_libraries(my_app iroh::iroh-c-ffi)
target_include_directories(my_app PRIVATE ${IROH_INCLUDE_DIRS})
```

```makefile
# Makefile
CC = gcc
CFLAGS = -I./include -Wall -Wextra
LDFLAGS = -L./target/release -liroh_c_ffi -lpthread -ldl -lm

all: my_app

my_app: main.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

clean:
	rm -f my_app
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| safer-ffi | 0.1.13 | C FFI generation |
| iroh | 0.90 | Core iroh functionality |
| iroh-base | 0.90 | Base types |
| tokio | 1.45.1 | Async runtime |
| hex | 0.4 | Hex encoding |
| data-encoding | 2.9.0 | Data encoding |

### Build Configuration

```toml
[lib]
crate-type = ["staticlib", "cdylib", "lib"]

[[bin]]
name = "generate_headers"
required-features = ["headers"]

[features]
headers = ["safer-ffi/headers"]
```

### safer-ffi Patterns

```rust
use safer_ffi::prelude::*;

// Export function to C
#[ffi_export]
fn iroh_hello(name: &CStr) -> char_p::Box {
    format!("Hello, {}!", name.to_str().unwrap()).try_into().unwrap()
}

// Export struct with opaque layout
#[ffi_export]
pub struct IrohObject {
    // Fields hidden from C
    inner: RustType,
}

// Return Result to C (becomes error pointer pattern)
#[ffi_export]
fn iroh_parse_key(
    hex: &CStr,
) -> Result<repr_c::Box<IrohKey>, IrohErrorCode> {
    // ...
}

// Slice handling
#[ffi_export]
fn iroh_process_data(
    data: c_slice::Ref<u8>,
) -> c_slice::Box<u8> {
    // Process and return slice
}
```

### Potential Enhancements

1. **Async Callbacks**: Better async callback support for C
2. **Stream Iterators**: C-compatible stream iteration
3. **Error Categories**: More granular error classification
4. **Documentation**: Better generated documentation for C API

---

## Summary

`iroh-c-ffi` provides:

- **C ABI Compatibility**: Works with C, C++, and C-compatible languages
- **Automatic Headers**: Generated C header files from Rust code
- **Clear Memory Model**: Explicit allocation/deallocation patterns
- **Error Handling**: C-compatible error codes and messages
- **Callback Support**: C functions callable from Rust

The crate enables embedding iroh functionality in C/C++ applications, games, and systems programming contexts.
