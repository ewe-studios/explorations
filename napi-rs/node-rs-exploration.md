---
name: node-rs
description: Collection of production-ready Node.js bindings written in Rust, demonstrating real-world napi-rs usage patterns
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.napi-rs/node-rs/
---

# node-rs - Production Node.js Bindings

## Overview

`node-rs` is a monorepo containing production-ready Node.js bindings written in Rust using napi-rs. It serves as both a useful collection of native modules and a reference implementation for building high-quality native add-ons.

## Repository Structure

```
node-rs/
├── Cargo.toml                    # Workspace definition
├── packages/                     # Published npm packages
│   ├── art-template/             # Template engine
│   ├── bcrypt/                   # Password hashing
│   ├── bn.js/                    # Big number library
│   ├── chmod/                    # File permissions
│   ├── crock32/                  # Crockford's Base32 encoding
│   ├── deno-webidl/              # WebIDL bindings
│   ├── dotenv/                   # Environment variable loader
│   ├── esbuild/                  # Bundler (see also: esbuild itself)
│   ├── ftp/                      # FTP client
│   ├── glob/                     # Glob pattern matching
│   ├── http-client/              # HTTP client
│   ├── is/                       # Type checking utilities
│   ├── jieba/                    # Chinese word segmentation
│   ├── jsonc/                    # JSONC parser
│   ├── lru-cache/                # LRU cache implementation
│   ├── md4/                      # MD4 hash
│   ├── md5/                      # MD5 hash
│   ├── multer/                   # Multipart form parser
│   ├── nanoid/                   # Unique ID generator
│   ├── napi-rs/                  # Core bindings
│   ├── pinyin/                   # Chinese pinyin conversion
│   ├── png/                      # PNG image handling
│   ├── dotenv/                   # Environment variables
│   ├── recursive-dirs/           # Directory traversal
│   ├── rimraf/                   # rm -rf equivalent
│   ├── sha1/                     # SHA1 hash
│   ├── sha256/                   # SHA256 hash
│   ├── siphash/                  # SipHash implementation
│   ├── sm3/                      # SM3 hash (Chinese standard)
│   ├── tar/                      # Tar archive handling
│   ├── tls/                      # TLS/SSL bindings
│   ├── tokenizer/                # Text tokenization
│   ├── uuid/                     # UUID generation
│   ├── xxhash/                   # XXHash implementation
│   └── ... (many more packages)
│
├── crates/                       # Shared Rust crates
│   ├── node-buffer/              # Buffer utilities
│   ├── node-fetch/               # Fetch implementation
│   └── node-util/                # Utility functions
│
└── benchmarks/                   # Performance benchmarks
```

## Architecture

### Package Structure Convention

Each package follows a consistent structure:

```
packages/<package-name>/
├── Cargo.toml              # Rust crate config
├── package.json            # NPM package config
├── src/
│   ├── lib.rs              # Main Rust source
│   └── bindings.rs         # NAPI bindings
├── index.d.ts              # TypeScript definitions (generated)
├── index.js                # JavaScript wrapper (if needed)
└── __tests__/              # Test files
```

## Key Packages Deep Dive

### 1. @node-rs/bcrypt

Password hashing library with competitive performance.

```rust
// packages/bcrypt/src/lib.rs
use napi::bindgen_prelude::*;
use napi_derive::napi;
use bcrypt::{hash, verify, DEFAULT_COST};

#[napi]
pub fn hash_password(password: String, cost: Option<u32>) -> Result<String> {
    let cost = cost.unwrap_or(DEFAULT_COST);
    hash(&password, cost)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

#[napi]
pub fn verify_password(password: String, hash: String) -> Result<bool> {
    verify(&password, &hash)
        .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
}

// Async versions for non-blocking
#[napi]
pub async fn hash_password_async(password: String, cost: Option<u32>) -> Result<String> {
    let cost = cost.unwrap_or(DEFAULT_COST);
    tokio::task::spawn_blocking(move || {
        hash(&password, cost)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    })
    .await
    .unwrap()
}
```

### 2. @node-rs/jieba

Chinese word segmentation using the jieba algorithm.

```rust
// packages/jieba/src/lib.rs
use jieba_rs::Jieba;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use once_cell::sync::Lazy;

static JIEBA: Lazy<Jieba> = Lazy::new(Jieba::new);

#[napi]
pub fn cut(sentence: String, hidden: Option<bool>) -> Vec<String> {
    JIEBA.cut_all(&sentence)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

#[napi]
pub fn cut_for_search(sentence: String) -> Vec<String> {
    JIEBA.cut_for_search(&sentence)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

#[napi]
pub fn tag(sentence: String) -> Vec<TagWord> {
    JIEBA.tag(&sentence)
        .into_iter()
        .map(|tw| TagWord {
            word: tw.word.to_string(),
            tag: tw.tag.to_string(),
        })
        .collect()
}

#[napi(object)]
pub struct TagWord {
    pub word: String,
    pub tag: String,
}
```

### 3. @node-rs/xxhash

High-speed non-cryptographic hash function.

```rust
// packages/xxhash/src/lib.rs
use xxhash_rust::xxh3::{xxh3_64, xxh3_128};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub fn xxh3_64(input: Buffer) -> u64 {
    xxh3_64(&input)
}

#[napi]
pub fn xxh3_128_hex(input: Buffer) -> String {
    let hash = xxh3_128(&input);
    format!("{:032x}", hash)
}

// Streaming API
#[napi]
pub struct Xxh3Stream {
    state: xxhash_rust::xxh3::Xxh3,
}

#[napi]
impl Xxh3Stream {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            state: xxhash_rust::xxh3::Xxh3::new(),
        }
    }

    #[napi]
    pub fn update(&mut self, data: Buffer) {
        self.state.update(&data);
    }

    #[napi]
    pub fn digest(&mut self) -> u64 {
        self.state.digest()
    }

    #[napi]
    pub fn reset(&mut self) {
        self.state.reset();
    }
}
```

### 4. @node-rs/tar

Tar archive creation and extraction.

```rust
// packages/tar/src/lib.rs
use tar::{Archive, Builder, EntryType};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::io::Cursor;

#[napi]
pub fn create_tar(files: Vec<TarFile>) -> Result<Buffer> {
    let mut buffer = Vec::new();
    let mut builder = Builder::new(&mut buffer);

    for file in files {
        let header = tar::Header::new_gnu();
        builder.append_data(&mut header, &file.path, Cursor::new(file.content))?;
    }

    builder.finish()?;
    Ok(buffer.into())
}

#[napi]
pub fn extract_tar(archive: Buffer) -> Result<Vec<TarFile>> {
    let mut archive = Archive::new(Cursor::new(&archive[..]));
    let mut files = Vec::new();

    for entry in archive.entries()? {
        let mut entry = entry?;
        let mut content = Vec::new();
        std::io::copy(&mut entry, &mut content)?;

        files.push(TarFile {
            path: entry.path()?.to_string_lossy().to_string(),
            content: Buffer::from(content),
        });
    }

    Ok(files)
}

#[napi(object)]
pub struct TarFile {
    pub path: String,
    pub content: Buffer,
}
```

### 5. @node-rs/glob

Fast glob pattern matching.

```rust
// packages/glob/src/lib.rs
use glob::Pattern;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::fs;
use std::path::PathBuf;

#[napi]
pub fn glob_sync(pattern: String, cwd: Option<String>) -> Result<Vec<String>> {
    let pattern = Pattern::new(&pattern)
        .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))?;

    let cwd = cwd.map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap());
    let mut matches = Vec::new();

    // Walk directory tree
    for entry in walkdir::WalkDir::new(&cwd) {
        let entry = entry?;
        let path = entry.path();

        if let Some(path_str) = path.to_str() {
            if pattern.matches(path_str) {
                matches.push(path_str.to_string());
            }
        }
    }

    Ok(matches)
}

#[napi]
pub async fn glob_async(pattern: String, cwd: Option<String>) -> Result<Vec<String>> {
    tokio::task::spawn_blocking(move || glob_sync(pattern, cwd))
        .await
        .unwrap()
}
```

## Performance Optimizations

### SIMD Acceleration

```rust
// packages/md5/src/lib.rs - Using SIMD
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[napi]
pub fn md5_simd(input: Buffer) -> String {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // Use SIMD instructions for faster processing
        if is_x86_feature_detected!("sse2") {
            return md5_simd_impl(&input);
        }
    }

    // Fallback to scalar implementation
    md5_scalar(&input)
}
```

### Memory-Efficient Buffer Handling

```rust
// Zero-copy buffer passing
#[napi]
pub fn process_buffer(input: Buffer) -> Result<Buffer> {
    // Borrow the buffer data (zero-copy)
    let data: &[u8] = &input;

    // Process without allocation
    let result = process(data);

    // Return new buffer
    Ok(Buffer::from(result))
}

// Using external buffers for large data
#[napi]
pub fn create_large_buffer(size: u32) -> Result<Buffer> {
    let data = vec![0u8; size as usize];

    // Create external buffer - Rust memory is directly accessible
    Ok(Buffer::from(data))
}
```

### Multi-threading

```rust
// Parallel processing
use rayon::prelude::*;

#[napi]
pub fn parallel_hash(inputs: Vec<Buffer>) -> Vec<u64> {
    inputs
        .par_iter()
        .map(|buf| xxh3_64(buf))
        .collect()
}

// Async with thread pool
#[napi]
pub async fn heavy_computation(data: Buffer) -> Result<Buffer> {
    tokio::task::spawn_blocking(move || {
        // CPU-intensive work
        Ok(compress(&data))
    })
    .await
    .unwrap()
}
```

## TypeScript Integration

### Generated Type Definitions

The napi-rs CLI automatically generates TypeScript definitions:

```typescript
// packages/bcrypt/index.d.ts (auto-generated)

/* auto-generated by NAPI-RS */
export declare function hashPassword(password: string, cost?: number): string

export declare function verifyPassword(password: string, hash: string): boolean

export declare function hashPasswordAsync(password: string, cost?: number): Promise<string>
```

### JavaScript Wrapper Pattern

```javascript
// packages/glob/index.js
const { globSync, globAsync } = require('./glob.linux-x64-gnu.node')

module.exports = {
  sync: globSync,
  async: globAsync,
  // Convenience wrapper
  glob: globAsync,
}
```

## Build and Publishing

### package.json Configuration

```json
{
  "name": "@node-rs/bcrypt",
  "version": "1.0.0",
  "main": "./bcrypt.linux-x64-gnu.node",
  "types": "./index.d.ts",
  "napi": {
    "name": "bcrypt",
    "triples": {
      "defaults": true,
      "additional": [
        "x86_64-apple-darwin",
        "x86_64-pc-windows-msvc",
        "aarch64-unknown-linux-gnu"
      ]
    }
  },
  "scripts": {
    "build": "napi build --release",
    "prepublishOnly": "napi prepublish"
  }
}
```

### GitHub Actions for Multi-Platform Build

```yaml
# .github/workflows/ci.yml
name: Build and Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - uses: napi-rs/napi-rs@actions/build@v1
        with:
          target: ${{ matrix.target }}
```

## Design Patterns

### 1. Builder Pattern for Complex Operations

```rust
#[napi]
pub struct TarBuilder {
    builder: Builder<Vec<u8>>,
}

#[napi]
impl TarBuilder {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            builder: Builder::new(Vec::new()),
        }
    }

    #[napi]
    pub fn add_file(mut self, path: String, content: Buffer) -> Result<Self> {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);

        self.builder.append_data(&mut header, &path, &content[..])?;
        Ok(self)
    }

    #[napi]
    pub fn add_directory(mut self, path: String) -> Result<Self> {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(EntryType::Directory);
        header.set_mode(0o755);

        self.builder.append_data(&mut header, &path, &[][..])?;
        Ok(self)
    }

    #[napi]
    pub fn finish(self) -> Result<Buffer> {
        let vec = self.builder.into_inner()?.into_inner()?;
        Ok(Buffer::from(vec))
    }
}
```

### 2. Iterator Pattern

```rust
#[napi]
pub struct GlobIterator {
    entries: Vec<String>,
    index: usize,
}

#[napi]
impl GlobIterator {
    #[napi]
    pub fn next(&mut self) -> Option<String> {
        if self.index < self.entries.len() {
            let entry = self.entries[self.index].clone();
            self.index += 1;
            Some(entry)
        } else {
            None
        }
    }

    #[napi]
    pub fn has_next(&self) -> bool {
        self.index < self.entries.len()
    }
}

// JavaScript usage:
// const iter = new GlobIterator('**/*.js');
// while (iter.hasNext()) {
//   console.log(iter.next());
// }
```

### 3. Event Emitter Pattern

```rust
#[napi]
pub struct Watcher {
    tx: mpsc::UnboundedSender<WatchEvent>,
    handles: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>,
}

#[napi]
impl Watcher {
    #[napi]
    pub fn on(&self, event: String, callback: JsFunction) -> Result<Subscription> {
        // Register callback for event type
    }

    #[napi]
    pub fn emit(&self, event: String, data: serde_json::Value) -> Result<()> {
        // Emit event to all registered callbacks
    }
}
```

## Testing Strategy

### Unit Tests

```rust
// packages/xxhash/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xxh3_64() {
        let input = b"hello world";
        let hash = xxh3_64(input);
        assert_eq!(hash, 0x1234567890abcdef); // Expected value
    }
}
```

### Integration Tests

```typescript
// packages/bcrypt/__tests__/bcrypt.spec.ts
import { hashPassword, verifyPassword } from '../index'

describe('bcrypt', () => {
  it('should hash and verify password', async () => {
    const password = 'secret123'
    const hash = await hashPassword(password, 10)

    expect(await verifyPassword(password, hash)).toBe(true)
    expect(await verifyPassword('wrong', hash)).toBe(false)
  })
})
```

## Summary

node-rs demonstrates:
- **Production patterns** for napi-rs modules
- **TypeScript integration** with auto-generated types
- **Multi-platform builds** with GitHub Actions
- **Performance optimizations** using SIMD and parallelism
- **Consistent package structure** across all modules
- **Comprehensive testing** with both Rust and JavaScript tests
