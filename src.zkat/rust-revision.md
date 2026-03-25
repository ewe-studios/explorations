# Rust Replication Plan: Building Similar Tools in Rust

**Based on zkat Projects Exploration**

---

## Table of Contents

1. [Key Takeaways from zkat Projects](#key-takeaways-from-zkat-projects)
2. [Building Content-Addressable Storage](#building-content-addressable-storage)
3. [Building Diagnostic Error Reporting](#building-diagnostic-error-reporting)
4. [Building Integrity Verification](#building-integrity-verification)
5. [Building Terminal Feature Detection](#building-terminal-feature-detection)
6. [Recommended Crates](#recommended-crates)
7. [Best Practices](#best-practices)
8. [Project Templates](#project-templates)

---

## Key Takeaways from zkat Projects

### Common Patterns

1. **Async-First, Sync Optional**
   - Default to async APIs using async-std or tokio
   - Provide sync alternatives with `_sync` suffix
   - Allow disabling async features for minimal deps

2. **Error Excellence**
   - Use miette for user-facing errors
   - Use thiserror for library errors
   - Provide context, help text, and source snippets

3. **Content-Addressability**
   - Use ssri for integrity hashes
   - Store by hash, index by key
   - Verify on read, deduplicate on write

4. **Terminal Awareness**
   - Detect color, unicode, hyperlink support
   - Respect NO_COLOR and related env vars
   - Adapt output to environment

5. **Cross-Platform by Design**
   - Windows filesystem quirks (case-insensitive)
   - Unix permissions and symlinks
   - CI environment detection

### Architecture Principles

```
┌─────────────────────────────────────────────────────────────────┐
│              Production-Ready Rust Architecture                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Public API Layer                      │    │
│  │  - Async functions (default)                            │    │
│  │  - Sync functions (_sync suffix)                        │    │
│  │  - Builder patterns (Options struct)                    │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                   Core Logic Layer                       │    │
│  │  - Modular components                                   │    │
│  │  - Trait-based abstractions                             │    │
│  │  - Testable units                                       │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                  Storage/IO Layer                        │    │
│  │  - Content-addressable backend                          │    │
│  │  - Index/metadata management                            │    │
│  │  - Atomic operations                                    │    │
│  └─────────────────────────────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                   Error Handling                         │    │
│  │  - thiserror for library errors                         │    │
│  │  - miette for user-facing diagnostics                   │    │
│  │  - Result<T, E> throughout                              │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Building Content-Addressable Storage

### Core Concepts

**Content-addressable storage (CAS)** identifies data by content hash rather than location.

```
Key-Value Store:          Content-Addressable Store:
┌─────────┬─────────┐     ┌──────────────────┬─────────┐
│   Key   │  Data   │     │  Content Hash    │  Data   │
├─────────┼─────────┤     ├──────────────────┼─────────┤
│ "user:1"│ {name}  │     │ sha256-abc123... │ {data}  │
│ "user:2"│ {name2} │     │ sha256-def456... │ {data2} │
└─────────┴─────────┘     └──────────────────┴─────────┘
   ▲                              ▲
   │                              │
   └──── Index maps key ──────────┘
         to hash
```

### Implementation Steps

#### Step 1: Define Hash Structure

```rust
use ssri::{Integrity, Algorithm, IntegrityOpts};
use std::path::{Path, PathBuf};

pub struct ContentAddressableStore {
    base_path: PathBuf,
    algorithm: Algorithm,
}

impl ContentAddressableStore {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            algorithm: Algorithm::Sha256,
        }
    }

    /// Generate content hash
    fn hash(&self, data: &[u8]) -> Integrity {
        IntegrityOpts::new()
            .algorithm(self.algorithm)
            .chain(data)
            .result()
    }

    /// Get storage path for content hash
    fn content_path(&self, integrity: &Integrity) -> PathBuf {
        let hash = &integrity.hashes[0];
        let algo_dir = format!("{:?}", hash.algorithm).to_lowercase();
        let prefix = &hash.digest[..2];  // First 2 chars for distribution

        self.base_path
            .join("content-v2")
            .join(&algo_dir)
            .join(prefix)
            .join(&hash.digest)
    }
}
```

#### Step 2: Implement Write with Atomicity

```rust
use std::fs::{self, File};
use std::io::Write;
use tempfile::NamedTempFile;

impl ContentAddressableStore {
    pub fn store(&self, data: &[u8]) -> std::io::Result<Integrity> {
        let integrity = self.hash(data);
        let path = self.content_path(&integrity);

        // Deduplication: return early if already exists
        if path.exists() {
            return Ok(integrity);
        }

        // Atomic write: temp file then rename
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent)?;

        let mut temp = NamedTempFile::new_in(parent)?;
        temp.write_all(data)?;
        temp.persist(&path)?;  // Atomic rename

        Ok(integrity)
    }
}
```

#### Step 3: Implement Read with Verification

```rust
impl ContentAddressableStore {
    pub fn retrieve(&self, integrity: &Integrity) -> std::io::Result<Vec<u8>> {
        let path = self.content_path(integrity);
        let data = fs::read(&path)?;

        // Verify integrity - crucial for CAS!
        integrity.check(&data).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Integrity verification failed: {}", e),
            )
        })?;

        Ok(data)
    }
}
```

#### Step 4: Add Key-Value Index

```rust
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct IndexEntry {
    pub key: String,
    pub integrity: String,  // SRI string
    pub timestamp: u128,
    pub size: u64,
}

pub struct Index {
    path: PathBuf,
    entries: HashMap<String, IndexEntry>,
}

impl Index {
    pub fn insert(&mut self, key: String, integrity: Integrity, size: u64) {
        self.entries.insert(key.clone(), IndexEntry {
            key,
            integrity: integrity.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            size,
        });
        self.save();
    }

    pub fn find(&self, key: &str) -> Option<&IndexEntry> {
        self.entries.get(key)
    }

    fn save(&self) {
        let json = serde_json::to_string_pretty(&self.entries).unwrap();
        fs::write(&self.path, json).unwrap();
    }
}
```

### Crate Recommendations

| Purpose | Crate | Why |
|---------|-------|-----|
| Hashing | `ssri` | SRI standard, multi-algorithm |
| Temp files | `tempfile` | Cross-platform atomic writes |
| Serialization | `serde`, `serde_json` | Index persistence |
| Directory creation | `std::fs` | Built-in is sufficient |
| Memory mapping | `memmap2` | Zero-copy reads for large files |

---

## Building Diagnostic Error Reporting

### When to Use miette

```rust
// Use miette when:
// 1. You need source code snippets
// 2. You want beautiful error messages
// 3. You need error codes with URLs
// 4. You're building user-facing CLI tools

// Use thiserror alone when:
// 1. You're writing a library
// 2. Errors don't need source context
// 3. You want minimal dependencies
```

### Implementation Pattern

#### Step 1: Define Error Types with thiserror + miette

```rust
use miette::{Diagnostic, SourceSpan, NamedSource};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum MyAppError {
    #[error("File not found: {path}")]
    #[diagnostic(
        code(myapp::io::not_found),
        url("https://docs.myapp.dev/errors/io/not-found"),
        help("Check that the file exists and you have read permissions")
    )]
    NotFound {
        path: String,
    },

    #[error("Parse error at line {line}, column {column}")]
    #[diagnostic(code(myapp::parse::error))]
    ParseError {
        #[source_code]
        source: NamedSource<String>,

        #[label("parse error here")]
        span: SourceSpan,

        line: usize,
        column: usize,

        #[help]
        suggestion: Option<String>,
    },

    #[error(transparent)]
    #[diagnostic(transparent)]
    IoError(#[from] std::io::Error),
}
```

#### Step 2: Create Errors with Context

```rust
use miette::{IntoDiagnostic, WrapErr, Result};

fn read_config(path: &str) -> Result<String> {
    std::fs::read_to_string(path)
        .into_diagnostic()  // Convert io::Error to miette::Report
        .wrap_err_with(|| format!("Failed to read config: {}", path))
}

fn parse_number(s: &str, source: &str) -> Result<i32> {
    s.parse().map_err(|e: std::num::ParseIntError| {
        miette!(
            labels = vec![LabeledSpan::at(10, "invalid number")],
            "Failed to parse integer"
        )
        .with_source_code(NamedSource::new("input.txt", source.to_string()))
    })
}
```

#### Step 3: Set Up Main for Pretty Output

```rust
use miette::{miette, Result};

fn main() -> Result<()> {
    // Your application logic
    do_something()?;

    Ok(())
}

// miette automatically pretty-prints errors on return
```

#### Step 4: Custom Error Handler (Optional)

```rust
use miette::{MietteHandlerOpts, set_hook};

fn main() -> miette::Result<()> {
    // Customize error output
    set_hook(Box::new(|_| {
        Box::new(
            MietteHandlerOpts::new()
                .terminal_links(true)
                .color(true)
                .unicode(true)
                .context_lines(3)
                .build(),
        )
    }))?;

    run_app()
}
```

### Crate Recommendations

| Purpose | Crate | Why |
|---------|-------|-----|
| Diagnostics | `miette` | Rich error reporting |
| Error derives | `thiserror` | Standard Error derives |
| Source spans | Built into miette | SourceSpan, LabeledSpan |
| Context | miette's `WrapErr` | anyhow-style context |

---

## Building Integrity Verification

### Using ssri for Hash Verification

#### Step 1: Generate Integrity

```rust
use ssri::{Integrity, IntegrityOpts, Algorithm};

// Simple: default SHA-256
let integrity = Integrity::from(b"hello world");

// Multiple algorithms
let integrity = IntegrityOpts::new()
    .algorithm(Algorithm::Sha512)
    .algorithm(Algorithm::Sha256)
    .chain(b"hello world")
    .result();

// Incremental (for streaming)
let mut opts = IntegrityOpts::new().algorithm(Algorithm::Sha256);
for chunk in data_chunks {
    opts = opts.chain(chunk);
}
let integrity = opts.result();
```

#### Step 2: Verify Integrity

```rust
use ssri::{Integrity, IntegrityChecker};

// Simple verification
fn verify(data: &[u8], expected: &Integrity) -> ssri::Result<()> {
    expected.check(data)?;
    Ok(())
}

// Streaming verification
fn verify_stream<R: Read>(reader: &mut R, expected: &Integrity) -> ssri::Result<()> {
    let mut checker = IntegrityChecker::new(expected.clone());
    let mut buffer = vec![0u8; 8192];

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 { break; }
        checker.input(&buffer[..n]);
    }

    checker.result()?;
    Ok(())
}
```

#### Step 3: Convert Between Formats

```rust
use ssri::{Integrity, Algorithm};

// Hex to SRI (common in Git)
let hex = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
let integrity = Integrity::from_hex(hex, Algorithm::Sha256).unwrap();

// SRI to Hex
let (algo, hex) = integrity.to_hex();

// String parsing
let sri: Integrity = "sha256-abc123...".parse().unwrap();
```

### Crate Recommendations

| Purpose | Crate | Why |
|---------|-------|-----|
| SRI | `ssri` | W3C standard implementation |
| Base64 | `base64` | Included in ssri |
| Hex | `hex` | Hex encoding/decoding |
| Hashing | `sha2`, `sha1` | Included in ssri |
| Fast hash | `xxhash-rust` | XXH3 in ssri |

---

## Building Terminal Feature Detection

### Detection Pattern

```rust
use supports_color::{on as supports_color, Stream, ColorLevel};
use supports_unicode::on as supports_unicode;
use supports_hyperlinks::on as supports_hyperlinks;

struct TerminalOutput {
    color: Option<ColorLevel>,
    unicode: bool,
    hyperlinks: bool,
}

impl TerminalOutput {
    fn new() -> Self {
        Self {
            color: supports_color(Stream::Stdout),
            unicode: supports_unicode(Stream::Stdout),
            hyperlinks: supports_hyperlinks(Stream::Stdout),
        }
    }

    fn format_success(&self, message: &str) -> String {
        if let Some(level) = self.color {
            if self.unicode {
                format!("\x1b[32m✓ {}\x1b[0m", message)
            } else {
                format!("\x1b[32m[OK] {}\x1b[0m", message)
            }
        } else if self.unicode {
            format!("✓ {}", message)
        } else {
            format!("[OK] {}", message)
        }
    }

    fn format_link(&self, text: &str, url: &str) -> String {
        if self.hyperlinks {
            format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text)
        } else {
            format!("{}: {}", text, url)
        }
    }
}
```

### Environment Variable Handling

```rust
fn check_color_support() -> Option<ColorLevel> {
    // Check force variables first (highest priority)
    if let Ok(force) = std::env::var("FORCE_COLOR") {
        return match force.as_str() {
            "0" | "false" => None,
            "1" | "true" | "" => Some(ColorLevel { level: 1, has_basic: true, has_256: false, has_16m: false }),
            "2" => Some(ColorLevel { level: 2, has_basic: true, has_256: true, has_16m: false }),
            "3" => Some(ColorLevel { level: 3, has_basic: true, has_256: true, has_16m: true }),
            _ => None,
        };
    }

    // Check disable variable
    if std::env::var("NO_COLOR").is_ok() {
        return None;
    }

    // Use library detection
    supports_color::on(supports_color::Stream::Stdout)
}
```

### Crate Recommendations

| Purpose | Crate | Why |
|---------|-------|-----|
| Color detection | `supports-color` | Comprehensive detection |
| Unicode detection | `supports-unicode` | Cross-platform |
| Hyperlink detection | `supports-hyperlinks` | OSC 8 support |
| CI detection | `is_ci` | Used by supports-color |
| TTY detection | `std::io::IsTerminal` | Built-in trait |

---

## Recommended Crates

### Core Dependencies

```toml
[dependencies]
# Error handling
miette = { version = "7.0", features = ["fancy"] }
thiserror = "2.0"

# Async runtime
tokio = { version = "1.0", features = ["full"] }
# OR
async-std = { version = "1.0", features = ["attributes"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Integrity
ssri = "9.0"

# Terminal
supports-color = "3.0"
supports-unicode = "3.0"
supports-hyperlinks = "3.0"
is-terminal = "0.4"

# Filesystem
tempfile = "3.0"
memmap2 = "0.9"
walkdir = "2.0"

# Hashing (if not using ssri)
sha2 = "0.10"
blake3 = "1.0"

# CLI
clap = { version = "4.0", features = ["derive"] }
dialoguer = "0.11"
indicatif = "0.17"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Development Dependencies

```toml
[dev-dependencies]
# Testing
tokio-test = "0.4"
tempfile = "3.0"

# Benchmarking
criterion = "0.5"

# Mocking
mockall = "0.11"

# Assertions
insta = "1.0"
pretty_assertions = "1.0"
```

---

## Best Practices

### 1. Async API Design

```rust
// Provide both async and sync APIs
pub async fn read(cache: &str, key: &str) -> Result<Vec<u8>> {
    // Async implementation
}

pub fn read_sync(cache: &str, key: &str) -> Result<Vec<u8>> {
    // Sync implementation
    // Use block_on internally if needed
}

// Use _sync suffix to distinguish
cacache::read(...).await;      // Async
cacache::read_sync(...);       // Sync
```

### 2. Feature Flags

```toml
[features]
default = ["async", "mmap"]
async = ["tokio"]
mmap = ["memmap2"]
fancy = ["miette/fancy"]

# Allow users to disable features
[dependencies]
tokio = { version = "1.0", optional = true }
memmap2 = { version = "0.9", optional = true }
```

### 3. Error Propagation

```rust
// Library code: concrete error types
pub enum LibraryError {
    NotFound(String),
    InvalidInput(String),
}

// Application code: miette::Result
pub fn main() -> miette::Result<()> {
    // Use ? for propagation
    do_something()?;
    Ok(())
}
```

### 4. Atomic Operations

```rust
use tempfile::NamedTempFile;
use std::fs;

// Always use atomic operations for file writes
fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
    let temp = NamedTempFile::new_in(path.parent().unwrap())?;
    temp.as_file().write_all(data)?;
    temp.persist(path)?;  // Atomic rename
    Ok(())
}
```

### 5. Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_cache() -> (TempDir, String) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_string_lossy().to_string();
        (dir, path)
    }

    #[test]
    fn test_basic_operations() {
        let (_dir, path) = setup_test_cache();

        write_sync(&path, "key", b"value").unwrap();
        let data = read_sync(&path, "key").unwrap();
        assert_eq!(data, b"value");
    }
}
```

---

## Project Templates

### Library Template

```rust
// lib.rs
//! My library crate
//!
//! # Example
//! ```rust
//! use my_crate::MyType;
//! let item = MyType::new();
//! ```

pub use error::{Error, Result};

mod error;
mod core;

pub use core::MyType;
```

```rust
// error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

### CLI Template

```rust
// main.rs
use clap::Parser;
use miette::Result;

#[derive(Parser, Debug)]
#[command(name = "myapp")]
#[command(about = "My awesome CLI")]
struct Cli {
    /// Input file
    #[arg(short, long)]
    input: String,

    /// Output file
    #[arg(short, long)]
    output: Option<String>,

    /// Verbose output
    #[arg(short, long, default_value = "false")]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    run(args)
}

fn run(args: Cli) -> Result<()> {
    // Application logic
    Ok(())
}
```

---

## Summary

Building production-quality Rust tools following zkat patterns:

1. **Use miette** for user-facing errors with context
2. **Use ssri** for content integrity verification
3. **Use supports-*** for terminal feature detection
4. **Provide async and sync APIs** for flexibility
5. **Implement atomic operations** for data safety
6. **Verify on read, deduplicate on write** for CAS
7. **Respect environment variables** (NO_COLOR, etc.)
8. **Test thoroughly** with tempfile for isolation

The zkat projects demonstrate that excellent developer experience comes from attention to detail in error messages, documentation, and cross-platform behavior.
