# ZeroClaw Rust Revision

**Document Type:** Rust Implementation Reference
**Last Updated:** 2026-03-22
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/zeroclaw`

---

## Table of Contents

1. [Rust Edition & Features](#rust-edition--features)
2. [Cargo Configuration](#cargo-configuration)
3. [Dependency Management](#dependency-management)
4. [Async Runtime](#async-runtime)
5. [Error Handling Patterns](#error-handling-patterns)
6. [Trait Architecture](#trait-architecture)
7. [Memory Management](#memory-management)
8. [Concurrency Patterns](#concurrency-patterns)
9. [Serialization](#serialization)
10. [Testing Strategy](#testing-strategy)
11. [Code Organization](#code-organization)
12. [Performance Optimizations](#performance-optimizations)
13. [Security Considerations](#security-considerations)
14. [Build Profiles](#build-profiles)
15. [Clippy & Formatting](#clippy--formatting)

---

## Rust Edition & Features

**Edition:** 2021

```toml
[package]
edition = "2021"
```

**Key Features Used:**

| Feature | Usage |
|---------|-------|
| `async`/`await` | Async runtime (tokio) |
| `#[async_trait]` | Trait methods with async |
| `Result<T, E>` | Error handling |
| `Arc<T>`, `Mutex<T>` | Thread-safe shared state |
| Pattern matching | `match` expressions, enum handling |
| Closures | Functional patterns, callbacks |
| Generics | Type-safe abstractions |
| Traits | Polymorphism, extension points |
| Lifetimes | Borrow checker guarantees |
| `serde` derive | Serialization |

---

## Cargo Configuration

### Workspace Structure

```toml
# Cargo.toml (root)
[workspace]
members = [".", "crates/robot-kit"]
resolver = "2"  # Feature-aware resolver
```

### Package Metadata

```toml
[package]
name = "zeroclaw"
version = "0.1.0"
edition = "2021"
authors = ["theonlyhennygod"]
license = "Apache-2.0"
description = "Zero overhead. Zero compromise. 100% Rust. The fastest, smallest AI assistant."
repository = "https://github.com/zeroclaw-labs/zeroclaw"
readme = "README.md"
keywords = ["ai", "agent", "cli", "assistant", "chatbot"]
categories = ["command-line-utilities", "api-bindings"]
```

---

## Dependency Management

### Core Dependencies

```toml
[dependencies]
# CLI
clap = { version = "4.5", features = ["derive"] }

# Async runtime
tokio = { version = "1.42", default-features = false, features = [
    "rt-multi-thread", "macros", "time", "net", "io-util",
    "sync", "process", "io-std", "fs", "signal"
] }
tokio-util = { version = "0.7", default-features = false }

# HTTP
reqwest = { version = "0.12", default-features = false, features = [
    "json", "rustls-tls", "blocking", "multipart", "stream"
] }

# Serialization
serde = { version = "1.0", default-features = false, features = ["derive"] }
serde_json = { version = "1.0", default-features = false, features = ["std"] }

# Config
directories = "6.0"
toml = "1.0"
shellexpand = "3.1"

# Logging
tracing = { version = "0.1", default-features = false }
tracing-subscriber = { version = "0.3", default-features = false, features = [
    "fmt", "ansi", "env-filter"
] }

# Observability
prometheus = { version = "0.14", default-features = false }

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Cryptography
chacha20poly1305 = "0.10"  # AEAD for secrets
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
ring = "0.17"  # JWT, low-level crypto

# UUID
uuid = { version = "1.11", default-features = false, features = ["v4", "std"] }

# RNG
rand = "0.9"

# Concurrency
parking_lot = "0.12"  # Faster than std::sync::Mutex
async-trait = "0.1"

# Database
rusqlite = { version = "0.38", features = ["bundled"] }
postgres = { version = "0.19", features = ["with-chrono-0_4"] }

# Time
chrono = { version = "0.4", default-features = false, features = [
    "clock", "std", "serde"
] }
chrono-tz = "0.10"
cron = "0.15"

# HTTP Server
axum = { version = "0.8", default-features = false, features = [
    "http1", "json", "tokio", "query", "ws"
] }
tower = { version = "0.5", default-features = false }
tower-http = { version = "0.6", default-features = false, features = [
    "limit", "timeout"
] }

# OpenTelemetry
opentelemetry = { version = "0.31", default-features = false, features = [
    "trace", "metrics"
] }
opentelemetry_sdk = { version = "0.31", default-features = false, features = [
    "trace", "metrics"
] }
opentelemetry-otlp = { version = "0.31", default-features = false, features = [
    "trace", "metrics", "http-proto", "reqwest-client", "reqwest-rustls-webpki-roots"
] }

# Hardware
nusb = { version = "0.2", default-features = false, optional = true }
tokio-serial = { version = "5", default-features = false, optional = true }
probe-rs = { version = "0.30", optional = true }

# Linux-specific
[target.'cfg(target_os = "linux")'.dependencies]
rppal = { version = "0.22", optional = true }
landlock = { version = "0.4", optional = true }
```

### Feature Flags

```toml
[features]
default = ["hardware"]
hardware = ["nusb", "tokio-serial"]
peripheral-rpi = ["rppal"]
browser-native = ["dep:fantoccini"]
sandbox-landlock = ["dep:landlock"]
sandbox-bubblewrap = []
probe = ["dep:probe-rs"]
rag-pdf = ["dep:pdf-extract"]
```

### Dependency Best Practices

**1. Minimal Features:**
```toml
# ✅ Good: Only needed features
tokio = { version = "1.42", default-features = false, features = [
    "rt-multi-thread", "macros", "time"
] }

# ❌ Bad: All features (increases compile time, binary size)
tokio = { version = "1.42", features = ["full"] }
```

**2. Version Pinning:**
```toml
# ✅ Good: Specific minor version
serde = { version = "1.0", features = ["derive"] }

# ❌ Bad: Too loose
serde = "1"
```

**3. Optional Dependencies:**
```toml
# ✅ Good: Optional for specific use cases
nusb = { version = "0.2", optional = true }

[features]
hardware = ["nusb"]
```

---

## Async Runtime

### Tokio Configuration

```rust
// In main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // Async entry point
}
```

### Async Patterns

**1. Async Trait Implementation:**
```rust
use async_trait::async_trait;

#[async_trait]
pub trait Memory: Send + Sync {
    async fn store(&self, key: &str, content: &str) -> Result<()>;
    async fn recall(&self, query: &str) -> Result<Vec<MemoryEntry>>;
}
```

**2. Task Spawning:**
```rust
// Spawn async task
let handle = tokio::spawn(async move {
    // Async work
    do_something().await
});

// Wait for completion
let result = handle.await??;
```

**3. Channels:**
```rust
use tokio::sync::mpsc;

// Create channel
let (tx, mut rx) = mpsc::channel(32);

// Send
tx.send(message).await?;

// Receive
while let Some(msg) = rx.recv().await {
    // Process message
}
```

**4. Select (Multiple Futures):**
```rust
use tokio::select;

select! {
    Some(msg) = rx.recv() => {
        // Handle message
    }
    _ = shutdown_signal() => {
        // Handle shutdown
    }
}
```

**5. Blocking Tasks:**
```rust
// Run blocking code in async context
let result = tokio::task::spawn_blocking(|| {
    // Blocking work (e.g., file I/O, CPU-intensive)
    expensive_computation()
})
.await??;
```

### Concurrency Primitives

**1. Arc for Shared State:**
```rust
use std::sync::Arc;

pub struct Agent {
    memory: Arc<dyn Memory>,
    observer: Arc<dyn Observer>,
}
```

**2. Mutex for Interior Mutability:**
```rust
use parking_lot::Mutex;  // Faster than std::sync::Mutex

pub struct ActionTracker {
    actions: Mutex<Vec<Instant>>,
}

impl ActionTracker {
    pub fn record(&self) -> usize {
        let mut actions = self.actions.lock();
        actions.push(Instant::now());
        actions.len()
    }
}
```

**3. RwLock for Read-Heavy Data:**
```rust
use parking_lot::RwLock;

pub struct Cache {
    data: RwLock<HashMap<String, Value>>,
}

impl Cache {
    pub fn get(&self, key: &str) -> Option<Value> {
        self.data.read().get(key).cloned()
    }

    pub fn insert(&self, key: String, value: Value) {
        self.data.write().insert(key, value);
    }
}
```

---

## Error Handling Patterns

### Anyhow for Application Code

```rust
use anyhow::{Result, bail, Context};

// Function returning Result
async fn load_config() -> Result<Config> {
    let config_path = default_config_path();
    let config_toml = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config from {}", config_path.display()))?;

    let config: Config = toml::from_str(&config_toml)
        .context("Failed to parse TOML")?;

    Ok(config)
}

// Early return with error
if interactive && channels_only {
    bail!("Use either --interactive or --channels-only, not both");
}
```

### Thiserror for Library Errors

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid SSE format: {0}")]
    InvalidSse(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Result Type Aliases

```rust
// Module-level type alias
type Result<T> = anyhow::Result<T>;

// Specific error type
type StreamResult<T> = std::result::Result<T, StreamError>;
```

### Error Propagation

```rust
// Using ? operator
async fn turn(&mut self, user_message: &str) -> Result<String> {
    let system_prompt = self.build_system_prompt()?;  // Propagates error
    // ...
}

// Mapping errors
let config = Config::load()
    .map_err(|e| anyhow::anyhow!("Config error: {}", e))?;
```

---

## Trait Architecture

### Trait Definition

```rust
use async_trait::async_trait;

#[async_trait]
pub trait Memory: Send + Sync {
    /// Backend name
    fn name(&self) -> &str;

    /// Store a memory entry
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()>;

    /// Recall memories matching a query
    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>>;

    /// Default method implementation
    async fn health_check(&self) -> bool {
        true
    }
}
```

### Trait Implementation

```rust
#[async_trait]
impl Memory for SqliteMemory {
    fn name(&self) -> &str {
        "sqlite"
    }

    async fn store(&self, key: &str, content: &str, category: MemoryCategory, session_id: Option<&str>) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO memories (key, content, category, session_id, timestamp)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            (key, content, category.to_string(), session_id),
        )?;
        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize, session_id: Option<&str>) -> Result<Vec<MemoryEntry>> {
        // Hybrid search implementation
    }
}
```

### Trait Objects

```rust
// Boxed trait object (heap-allocated)
let memory: Box<dyn Memory> = Box::new(SqliteMemory::new(...)?);

// Arc trait object (thread-safe, reference-counted)
let memory: Arc<dyn Memory> = Arc::new(SqliteMemory::new(...)?);

// Factory function returning trait object
pub fn create_memory(config: &MemoryConfig) -> Result<Box<dyn Memory>> {
    match config.backend.as_str() {
        "sqlite" => Ok(Box::new(SqliteMemory::new(...)?)),
        "postgres" => Ok(Box::new(PostgresMemory::new(...)?)),
        _ => bail!("Unknown backend: {}", config.backend),
    }
}
```

---

## Memory Management

### Smart Pointers

**1. Arc (Atomic Reference Counting):**
```rust
use std::sync::Arc;

// Shared ownership across threads
let memory: Arc<dyn Memory> = Arc::new(SqliteMemory::new(...)?);

// Clone Arc (cheap, increments refcount)
let memory_clone = Arc::clone(&memory);
```

**2. Box (Heap Allocation):**
```rust
// Box large data to avoid stack overflow
let large_config: Box<Config> = Box::new(config);

// Box trait objects
let provider: Box<dyn Provider> = Box::new(OpenAiProvider::new());
```

**3. Rc (Single-threaded Reference Counting):**
```rust
// Use Rc for single-threaded contexts
use std::rc::Rc;

let data = Rc::new(vec![1, 2, 3]);
let clone = Rc::clone(&data);
```

### Borrowing Patterns

```rust
// Borrow instead of clone
fn process_data(data: &[u8]) -> Result<()> {
    // Process without taking ownership
}

// Mutable borrow for modification
fn update_config(config: &mut Config) {
    config.api_key = Some("new-key".into());
}

// Return borrowed data (with lifetimes)
fn get_name(data: &'a Data) -> &'a str {
    &data.name
}
```

### Memory-Efficient Patterns

**1. Cow (Clone on Write):**
```rust
use std::borrow::Cow;

fn normalize_input(input: &str) -> Cow<str> {
    if input.trim() == input {
        Cow::Borrowed(input)  // No allocation
    } else {
        Cow::Owned(input.trim().to_string())  // Allocate only when needed
    }
}
```

**2. String Interning:**
```rust
// Use string slices for repeated strings
let category = "core";  // &'static str, no allocation

// vs
let category = String::from("core");  // Heap allocation
```

---

## Concurrency Patterns

### Channel Communication

```rust
use tokio::sync::mpsc;

// Bounded channel (backpressure)
let (tx, rx) = mpsc::channel(100);

// Unbounded channel (use carefully)
let (tx, rx) = mpsc::unbounded_channel();

// Multiple producers, single consumer
let (tx, mut rx) = mpsc::channel(32);

// Spawn multiple producers
for i in 0..3 {
    let tx = tx.clone();
    tokio::spawn(async move {
        tx.send(format!("from {}", i)).await.unwrap();
    });
}

// Drop original tx to signal completion
drop(tx);

// Consumer
while let Some(msg) = rx.recv().await {
    println!("Received: {}", msg);
}
```

### Parallel Execution

```rust
use futures::future::join_all;

// Run tasks in parallel
let tasks = vec![
    tokio::spawn(fetch_data(1)),
    tokio::spawn(fetch_data(2)),
    tokio::spawn(fetch_data(3)),
];

let results = join_all(tasks).await;
```

### Rate Limiting

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

let semaphore = Arc::new(Semaphore::new(10));  // 10 concurrent requests

// Acquire permit
let permit = semaphore.acquire().await.unwrap();

// Do work (permit held)
do_work().await;

// Permit released when dropped
drop(permit);
```

---

## Serialization

### Serde Derives

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_key: Option<String>,
    pub default_provider: Option<String>,

    #[serde(default)]
    pub observability: ObservabilityConfig,

    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_port() -> u16 {
    3000
}
```

### Enum Serialization

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryCategory {
    Core,
    Daily,
    Conversation,
    Custom(String),
}

// Serializes to: "core", "daily", "conversation", "custom(value)"
```

### Tagged Enums

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ConversationMessage {
    Chat(ChatMessage),
    AssistantToolCalls {
        text: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    ToolResults(Vec<ToolResultMessage>),
}
```

### JSON Value Handling

```rust
use serde_json::{json, Value};

// Create JSON
let params = json!({
    "key": "value",
    "count": 42,
    "nested": {
        "array": [1, 2, 3]
    }
});

// Access JSON
if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
    println!("Name: {}", name);
}

// Serialize/deserialize
let json_str = serde_json::to_string(&config)?;
let config: Config = serde_json::from_str(&json_str)?;
```

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 3000);
    }

    #[tokio::test]
    async fn test_memory_store_recall() {
        let memory = SqliteMemory::new_temp().unwrap();

        memory.store("key", "value", MemoryCategory::Core, None)
            .await
            .unwrap();

        let results = memory.recall("value", 10, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "key");
    }
}
```

### Test Fixtures

```rust
// Test helpers
mod test_helpers {
    use super::*;

    pub fn create_test_config() -> Config {
        Config {
            api_key: Some("test-key".into()),
            ..Default::default()
        }
    }

    pub async fn setup_test_database() -> SqliteMemory {
        SqliteMemory::new_temp().unwrap()
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs
use zeroclaw::{Config, Agent};

#[tokio::test]
async fn test_full_agent_turn() {
    let config = create_test_config();
    let mut agent = Agent::from_config(&config).unwrap();

    let response = agent.turn("Hello, world!").await.unwrap();

    assert!(!response.is_empty());
}
```

### Mocking

```rust
// Mock provider for testing
struct MockProvider {
    responses: Mutex<Vec<ChatResponse>>,
}

#[async_trait]
impl Provider for MockProvider {
    async fn chat(&self, _request: ChatRequest<'_>, _model: &str, _temperature: f64) -> Result<ChatResponse> {
        let mut guard = self.responses.lock();
        Ok(guard.remove(0))
    }
}

#[tokio::test]
async fn test_agent_with_mock_provider() {
    let provider = Box::new(MockProvider {
        responses: Mutex::new(vec![ChatResponse {
            text: Some("hello".into()),
            tool_calls: vec![],
        }]),
    });

    // ... setup agent with mock
}
```

---

## Code Organization

### Module Structure

```rust
// src/lib.rs
pub mod agent;
pub mod auth;
pub mod channels;
pub mod config;
pub mod memory;
pub mod providers;
pub mod tools;
// ...

pub use config::Config;

// src/agent/mod.rs
mod agent;
mod classifier;
mod dispatcher;
mod memory_loader;
mod prompt;
mod tests;

pub use agent::{Agent, AgentBuilder};
pub use dispatcher::ToolDispatcher;
```

### Visibility

```rust
// Public API
pub struct Agent { /* ... */ }

// Internal (crate-only)
pub(crate) struct InternalHelper { /* ... */ }

// Module-private
struct PrivateStruct { /* ... */ }
```

### Re-exports

```rust
// Re-export for cleaner public API
pub use memory::{Memory, MemoryCategory, MemoryEntry};
pub use tools::{Tool, ToolResult, ToolSpec};
pub use channels::{Channel, ChannelMessage, SendMessage};
```

---

## Performance Optimizations

### Release Profile

```toml
[profile.release]
opt-level = "z"       # Optimize for size
lto = "thin"          # Link-time optimization
codegen-units = 1     # Single codegen unit (low RAM)
strip = true          # Remove debug symbols
panic = "abort"       # Smaller binary
```

### Inline Functions

```rust
#[inline]
fn small_function(x: i32) -> i32 {
    x * 2
}

#[inline(always)]
fn always_inline(x: i32) -> i32 {
    x * 2
}

#[inline(never)]
fn never_inline() {
    // Large function
}
```

### Lazy Initialization

```rust
use once_cell::sync::OnceCell;

static CONFIG: OnceCell<Config> = OnceCell::new();

fn get_config() -> &'static Config {
    CONFIG.get_or_init(|| Config::load().unwrap())
}
```

### Zero-Copy Parsing

```rust
// Use bytes instead of String when possible
fn parse_header(data: &[u8]) -> Result<Header> {
    // Parse without allocating
}
```

---

## Security Considerations

### Secret Handling

```rust
// Don't log secrets
tracing::info!("Connecting to API");  // ✅ Good
tracing::info!("API Key: {}", api_key);  // ❌ Bad

// Use zeroizing for sensitive data
use zeroize::Zeroize;

let mut secret = api_key.to_string();
// Use secret...
secret.zeroize();  // Clear from memory
```

### Constant-Time Comparison

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

fn verify_signature(message: &[u8], signature: &[u8], key: &[u8]) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
    mac.update(message);
    mac.verify_slice(signature).is_ok()  // Constant-time comparison
}
```

### Input Validation

```rust
// Validate paths
fn validate_path(path: &Path, workspace: &Path) -> Result<()> {
    let canonical = path.canonicalize()?;
    if !canonical.starts_with(workspace) {
        bail!("Path escapes workspace: {:?}", path);
    }
    Ok(())
}

// Validate commands
fn is_command_allowed(cmd: &str, allowed: &[String]) -> bool {
    let base_cmd = cmd.split_whitespace().next().unwrap_or("");
    allowed.iter().any(|a| a == base_cmd)
}
```

---

## Build Profiles

### Standard Profiles

```toml
# Development (cargo build)
[profile.dev]
opt-level = 0
debug = true

# Release (cargo build --release)
[profile.release]
opt-level = "z"
lto = "thin"
codegen-units = 1
strip = true
panic = "abort"

# Custom fast release
[profile.release-fast]
inherits = "release"
codegen-units = 8

# Distribution (maximum optimization)
[profile.dist]
inherits = "release"
opt-level = "z"
lto = "fat"
```

### Building

```bash
# Development build
cargo build

# Release build (size-optimized)
cargo build --release

# Fast release build
cargo build --profile release-fast

# Distribution build
cargo build --profile dist
```

---

## Clippy & Formatting

### Clippy Configuration

```toml
# clippy.toml
allow-unwrap-in-tests = true
```

### Allowed Lints

```rust
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // ... other allows
)]
```

### Formatting

```bash
# Check formatting
cargo fmt --all -- --check

# Format code
cargo fmt --all
```

### CI Checks

```bash
# Full validation
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

---

## Conclusion

ZeroClaw demonstrates **production-grade Rust engineering** with:

1. **Modern Rust (2021 edition)** - Latest features and patterns
2. **Async-first design** - Tokio-based concurrency
3. **Trait-driven architecture** - Polymorphism without inheritance
4. **Comprehensive error handling** - Anyhow + Thiserror
5. **Memory safety** - Arc, Mutex, borrowing patterns
6. **Performance optimization** - Size-optimized builds, LTO
7. **Security-conscious** - Secret handling, input validation
8. **Well-tested** - Unit, integration, and mock tests

This Rust implementation enables ZeroClaw to achieve its goals of **zero overhead, zero compromise** while maintaining memory safety and thread safety guarantees.
