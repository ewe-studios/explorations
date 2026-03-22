# Taubyte Rust Implementation Revision

## Overview

This document provides a comprehensive guide for implementing Taubyte functionality in Rust. It covers the Rust SDK architecture, best practices, patterns, and production considerations for building serverless functions on Taubyte.

---

## Rust SDK Architecture

### SDK Structure

```
rust-sdk/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Main library exports
│   ├── errno/              # Error handling
│   │   ├── mod.rs          # Error codes
│   │   └── error_strings.rs
│   ├── database/           # KV Database operations
│   │   ├── mod.rs          # Database struct
│   │   ├── new.rs          # Create/open database
│   │   ├── get.rs          # Get value
│   │   ├── put.rs          # Put value
│   │   ├── delete.rs       # Delete key
│   │   ├── list.rs         # List keys
│   │   └── close.rs        # Close database
│   ├── storage/            # Object storage
│   │   ├── mod.rs          # Storage struct
│   │   ├── new.rs          # Create/open bucket
│   │   ├── get.rs          # Get file
│   │   ├── file/           # File operations
│   │   │   ├── mod.rs
│   │   │   ├── new.rs
│   │   │   ├── open.rs
│   │   │   ├── read.rs
│   │   │   ├── get.rs
│   │   │   ├── put.rs
│   │   │   ├── delete.rs
│   │   │   └── close.rs
│   │   └── content/        # Content operations
│   │       ├── mod.rs
│   │       ├── new.rs
│   │       ├── open.rs
│   │       ├── read.rs
│   │       └── write.rs
│   ├── http/               # HTTP operations
│   │   ├── mod.rs
│   │   ├── event/          # HTTP event handling
│   │   │   ├── mod.rs
│   │   │   ├── body.rs
│   │   │   ├── headers.rs
│   │   │   ├── host.rs
│   │   │   ├── method.rs
│   │   │   ├── path.rs
│   │   │   ├── query.rs
│   │   │   ├── return.rs
│   │   │   ├── write.rs
│   │   │   └── imports.rs
│   │   └── client/         # HTTP client
│   │       ├── mod.rs
│   │       ├── new.rs
│   │       ├── request.rs
│   │       ├── response.rs
│   │       ├── send.rs
│   │       └── imports.rs
│   ├── pubsub/             # Pub/Sub operations
│   │   ├── mod.rs
│   │   ├── event/          # Event-based
│   │   │   ├── channel.rs
│   │   │   ├── data.rs
│   │   │   └── imports.rs
│   │   └── node/           # Node-based
│   │       ├── channel.rs
│   │       ├── publish.rs
│   │       ├── subscribe.rs
│   │       ├── socket.rs
│   │       └── imports.rs
│   ├── i2mv/               # Inter-VM memory views
│   │   ├── mod.rs
│   │   ├── fifo/           # FIFO queues
│   │   │   ├── mod.rs
│   │   │   ├── imports.rs
│   │   │   ├── read_closer.rs
│   │   │   └── write_closer.rs
│   │   └── memview/        # Memory views
│   │       ├── mod.rs
│   │       ├── imports.rs
│   │       ├── closer.rs
│   │       └── read_seek_closer.rs
│   └── utils/              # Utilities
│       ├── mod.rs
│       ├── codec/          # Encoding/decoding
│       │   ├── bytes_slice.rs
│       │   ├── cid.rs
│       │   └── string_slice.rs
│       ├── convert/        # Type conversions
│       │   └── method.rs
│       └── test/           # Test utilities
│           ├── mod.rs
│           ├── read.rs
│           └── write.rs
```

---

## Core SDK Usage

### Database Operations

```rust
// Example: Using the KV Database
use taubyte_sdk::database::Database;

fn main() {
    // Open/create a database
    let db = Database::new("my-database");

    // Put a value
    db.put("user:123", b"John Doe");

    // Get a value
    match db.get("user:123") {
        Some(value) => println!("User: {:?}", value),
        None => println!("User not found"),
    }

    // List keys with prefix
    let keys = db.list("user:");
    for key in keys {
        println!("Key: {}", key);
    }

    // Delete a key
    db.delete("user:123");

    // Close the database (optional, auto-closed on drop)
    db.close();
}
```

### Storage Operations

```rust
// Example: Using Object Storage
use taubyte_sdk::storage::{Storage, File};

fn main() {
    // Open/create a bucket
    let storage = Storage::new("my-bucket");

    // Put a file
    storage.put("documents/readme.txt", b"Hello, Taubyte!");

    // Open a file for reading
    let file = storage.open("documents/readme.txt");
    let mut buffer = Vec::new();
    file.read(&mut buffer);
    println!("File content: {:?}", buffer);

    // Get file info
    if let Some(info) = storage.get("documents/readme.txt") {
        println!("Size: {}, Versions: {}", info.size, info.versions);
    }

    // List files with prefix
    let files = storage.list("documents/");
    for file in files {
        println!("File: {}", file);
    }

    // Delete a file
    storage.delete("documents/readme.txt");
}
```

### HTTP Event Handling

```rust
// Example: HTTP Function Handler
use taubyte_sdk::http::event::Event;

#[no_mangle]
pub fn handle(event: Event) {
    // Get request method
    let method = event.method();

    // Get request path
    let path = event.path();

    // Get query parameters
    let query = event.query("name").unwrap_or_default();

    // Get headers
    let content_type = event.header("Content-Type").unwrap_or_default();

    // Get request body
    let body = event.body();

    // Set response headers
    event.set_header("Content-Type", "application/json");

    // Set response status code
    event.set_status(200);

    // Write response
    let response = format!(r#"{{"method":"{}","path":"{}"}}"#, method, path);
    event.write(response.as_bytes());
}
```

### HTTP Client

```rust
// Example: HTTP Client
use taubyte_sdk::http::client::Client;

fn main() {
    // Create HTTP client
    let client = Client::new();

    // GET request
    let response = client
        .request("GET", "https://api.example.com/users")
        .header("Accept", "application/json")
        .send();

    println!("Status: {}", response.status());
    println!("Body: {:?}", response.body());

    // POST request
    let response = client
        .request("POST", "https://api.example.com/users")
        .header("Content-Type", "application/json")
        .body(b r#"{"name":"John"}"# )
        .send();

    // Response handling
    if response.status() == 201 {
        println!("User created: {:?}", response.body());
    }
}
```

### Pub/Sub Operations

```rust
// Example: Pub/Sub
use taubyte_sdk::pubsub::{Channel, Event};

fn main() {
    // Publish to a channel
    let channel = Channel::new("notifications");
    channel.publish(b"New notification!");

    // Subscribe to a channel (in event-driven context)
    // Note: Subscription is typically handled by the platform
}

// Event-driven function with pubsub
#[no_mangle]
pub fn handle(event: Event) {
    // Get channel from event
    let channel = event.channel();

    // Get published data
    let data = event.data();

    // Process data
    println!("Received: {:?}", data);
}
```

---

## Memory Views (I2MV)

### Using Memory Views

```rust
// Example: Memory Views for efficient data transfer
use taubyte_sdk::i2mv::memview::{Closer, ReadSeekCloser};
use taubyte_sdk::i2mv::fifo::{FifoReadCloser, FifoWriteCloser};

fn main() {
    // Create a memory view (write-only)
    let data = b"Hello, Taubyte!";
    let mv = Closer::new(data, true).unwrap();
    println!("Memory view ID: {}", mv.id);

    // Read from a memory view
    let mut reader = ReadSeekCloser::open(mv.id).unwrap();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer);
    println!("Read: {:?}", buffer);

    // Using FIFO for streaming
    let fifo_write = FifoWriteCloser::new(true).unwrap();
    let fifo_read = FifoReadCloser::open(fifo_write.id).unwrap();
}
```

### I2MV with Storage

```rust
// Example: Using I2MV with storage operations
use taubyte_sdk::storage::Storage;
use taubyte_sdk::i2mv::memview::Closer;

fn main() {
    let storage = Storage::new("my-bucket");

    // Large file transfer using memory views
    let data = generate_large_data();
    let mv = Closer::new(&data, true).unwrap();

    // Storage operations can use memory view ID
    // This avoids copying data
    storage.put_with_memview("large-file.bin", mv.id);
}
```

---

## Error Handling

### SDK Error Types

```rust
// errno/mod.rs
#[repr(u32)]
pub enum TaubyteError {
    Success = 0,
    InvalidHandle = 1,
    NotFound = 2,
    AlreadyExists = 3,
    PermissionDenied = 4,
    OutOfMemory = 5,
    InvalidArgument = 6,
    IoError = 7,
    Timeout = 8,
    Unknown = 255,
}

impl TaubyteError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::InvalidHandle => "Invalid handle",
            Self::NotFound => "Not found",
            Self::AlreadyExists => "Already exists",
            Self::PermissionDenied => "Permission denied",
            Self::OutOfMemory => "Out of memory",
            Self::InvalidArgument => "Invalid argument",
            Self::IoError => "I/O error",
            Self::Timeout => "Timeout",
            Self::Unknown => "Unknown error",
        }
    }
}

pub type Result<T> = core::result::Result<T, TaubyteError>;
```

### Error Handling Pattern

```rust
use taubyte_sdk::errno::{TaubyteError, Result};

fn process_data(key: &str) -> Result<Vec<u8>> {
    let db = taubyte_sdk::database::Database::new("my-db");

    // Handle potential errors
    match db.get(key) {
        Some(value) => Ok(value),
        None => Err(TaubyteError::NotFound),
    }
}

// Using Result combinators
fn get_user_name(user_id: u32) -> Result<String> {
    let db = taubyte_sdk::database::Database::new("users");
    let key = format!("user:{}", user_id);

    db.get(&key)
        .map(|bytes| String::from_utf8(bytes).unwrap())
        .ok_or(TaubyteError::NotFound)
}
```

---

## Building Rust Functions for Taubyte

### Cargo Configuration

```toml
# Cargo.toml
[package]
name = "taubyte-function"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
crate-name = "my_function"

[dependencies]
taubyte-sdk = "0.1.6"

# Optimize for size (important for WASM)
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

### Build Script

```bash
#!/bin/bash
# build.sh

# Install WASM target if not already installed
rustup target add wasm32-unknown-unknown

# Build for WASM
cargo build --target wasm32-unknown-unknown --release

# Optimize WASM binary (optional but recommended)
wasm-opt -Oz target/wasm32-unknown-unknown/release/my_function.wasm \
    -o my_function.optimized.wasm

# Deploy to Taubyte
tau deploy my_function.optimized.wasm
```

### Example Function

```rust
// lib.rs - A complete Taubyte function
use taubyte_sdk::http::event::Event;
use taubyte_sdk::database::Database;

/// HTTP handler function
///
/// This function:
/// 1. Reads a key from the query parameter
/// 2. Looks up the value in the database
/// 3. Returns JSON response
#[no_mangle]
pub fn handle(event: Event) {
    // Get the key from query params
    let key = event.query("key").unwrap_or("default");

    // Look up in database
    let db = Database::new("my-data");

    match db.get(key) {
        Some(value) => {
            // Success - return value
            event.set_header("Content-Type", "application/json");
            let response = format!(r#"{{"key":"{}","value":"{:?}"}}"#, key, value);
            event.write(response.as_bytes());
        }
        None => {
            // Not found
            event.set_status(404);
            event.set_header("Content-Type", "application/json");
            event.write(br#"{"error":"Key not found"}"#);
        }
    }
}
```

---

## Advanced Patterns

### Stateful Functions

```rust
// Using database for function state
use taubyte_sdk::{
    http::event::Event,
    database::Database,
};

#[no_mangle]
pub fn handle(event: Event) {
    let db = Database::new("function-state");

    // Get invocation count
    let count_key = "invocation_count";
    let count: u32 = db.get(count_key)
        .map(|b| std::str::from_utf8(&b).unwrap().parse().unwrap_or(0))
        .unwrap_or(0);

    // Increment counter
    db.put(count_key, (count + 1).to_string().as_bytes());

    // Return response with count
    event.set_header("Content-Type", "application/json");
    let response = format!(r#"{{"invocation":{}}}"#, count + 1);
    event.write(response.as_bytes());
}
```

### Async-like Patterns

```rust
// Using I2MV for streaming responses
use taubyte_sdk::{
    http::event::Event,
    i2mv::memview::Closer,
};

#[no_mangle]
pub fn handle(event: Event) {
    event.set_header("Content-Type", "application/json");
    event.set_header("Transfer-Encoding", "chunked");

    // Stream data in chunks
    for i in 0..10 {
        let chunk = format!(r#"{{"item":{}}}"#, i);

        // Create memory view for chunk
        let mv = Closer::new(chunk.as_bytes(), true).unwrap();

        // Write chunk (platform-specific)
        event.write_chunk(mv.id);
    }
}
```

### Multi-Service Integration

```rust
// Function that uses multiple Taubyte services
use taubyte_sdk::{
    http::event::Event,
    database::Database,
    storage::Storage,
    http::client::Client,
    pubsub::Channel,
};

#[no_mangle]
pub fn handle(event: Event) {
    // Parse request body
    let body = event.body();

    // Store in database
    let db = Database::new("main-db");
    db.put("latest-request", &body);

    // Store attachment in object storage
    let storage = Storage::new("attachments");
    storage.put("request.bin", &body);

    // Call external API
    let client = Client::new();
    let response = client
        .request("POST", "https://api.example.com/webhook")
        .body(&body)
        .send();

    // Publish notification
    let channel = Channel::new("notifications");
    channel.publish(b"New request processed");

    // Return response
    event.set_status(response.status());
    event.write(&response.body());
}
```

---

## Testing Rust Functions

### Unit Testing

```rust
// tests/mod.rs
#[cfg(test)]
mod tests {
    use taubyte_sdk::utils::test::{setup_test_context, cleanup_test_context};

    #[test]
    fn test_database_operations() {
        setup_test_context();

        let db = taubyte_sdk::database::Database::new("test-db");
        db.put("test-key", b"test-value");

        let value = db.get("test-key").unwrap();
        assert_eq!(value, b"test-value");

        cleanup_test_context();
    }

    #[test]
    fn test_storage_operations() {
        setup_test_context();

        let storage = taubyte_sdk::storage::Storage::new("test-bucket");
        storage.put("test.txt", b"hello");

        let file = storage.open("test.txt");
        let mut buffer = Vec::new();
        file.read(&mut buffer);
        assert_eq!(buffer, b"hello");

        cleanup_test_context();
    }
}
```

### Integration Testing

```rust
// tests/integration.rs
use taubyte_sdk::http::event::Event;

// Mock HTTP event for testing
struct MockEvent {
    method: String,
    path: String,
    headers: std::collections::HashMap<String, String>,
    body: Vec<u8>,
}

impl MockEvent {
    fn new(method: &str, path: &str, body: &[u8]) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: std::collections::HashMap::new(),
            body: body.to_vec(),
        }
    }
}

#[test]
fn test_http_handler() {
    let event = MockEvent::new("GET", "/api/test", b"");

    // Call the handler (would need adaptation for testing)
    // handle(event);
}
```

---

## Security Considerations

### Secrets Management

```rust
// Never hardcode secrets!
// BAD:
// const API_KEY: &str = "hardcoded-secret";

// GOOD: Use Taubyte secrets
use taubyte_sdk::database::Database;

fn get_api_key() -> String {
    let db = Database::new("_secrets");
    match db.get("api_key") {
        Some(key) => String::from_utf8(key).unwrap(),
        None => panic!("API key not found in secrets"),
    }
}
```

### Input Validation

```rust
// Always validate input
use taubyte_sdk::http::event::Event;

#[no_mangle]
pub fn handle(event: Event) {
    // Validate method
    if event.method() != "POST" {
        event.set_status(405);
        event.write(b"Method not allowed");
        return;
    }

    // Validate content type
    let content_type = event.header("Content-Type").unwrap_or_default();
    if !content_type.contains("application/json") {
        event.set_status(415);
        event.write(b"Unsupported media type");
        return;
    }

    // Validate body size
    let body = event.body();
    if body.len() > 1024 * 1024 {  // 1MB limit
        event.set_status(413);
        event.write(b"Payload too large");
        return;
    }

    // Process validated request...
}
```

---

## Performance Best Practices

### Memory Management

```rust
// Efficient memory usage
use taubyte_sdk::i2mv::memview::Closer;

// GOOD: Use memory views for large data
fn process_large_data(data: &[u8]) {
    // Create memory view - zero copy
    let mv = Closer::new(data, true).unwrap();
    // Pass mv.id to other functions
}

// AVOID: Unnecessary allocations
fn bad_example() {
    let mut result = String::new();
    for i in 0..1000 {
        result.push_str(&format!("{}", i));  // Many allocations
    }
}

// GOOD: Pre-allocate when size is known
fn good_example() {
    let mut result = String::with_capacity(1000 * 4);  // Pre-allocate
    for i in 0..1000 {
        result.push_str(&format!("{}", i));
    }
}
```

### WASM Optimization

```toml
# Cargo.toml optimizations
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
strip = true         # Strip debug symbols
panic = "abort"      # Smaller binary

[dependencies]
# Use no_std compatible crates when possible
```

---

## Production Checklist

Before deploying Rust functions to production:

- [ ] WASM binary optimized with `wasm-opt`
- [ ] All error paths handled
- [ ] Input validation implemented
- [ ] Secrets stored in Taubyte secrets (not hardcoded)
- [ ] Database connections properly closed
- [ ] Memory views used for large data transfers
- [ ] Function timeout configured appropriately
- [ ] Memory limit configured appropriately
- [ ] Logging/debug statements removed
- [ ] Unit tests pass
- [ ] Integration tests pass

---

## Related Documents

- `exploration.md` - Main exploration
- `subsystems/monkey.md` - Function execution service
- `production-grade.md` - Production considerations
- `../rust-sdk/Cargo.toml` - SDK dependencies
