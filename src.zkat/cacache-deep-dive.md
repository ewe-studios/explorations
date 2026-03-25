# cacache-rs Deep Dive: Content-Addressable Caching in Rust

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/cacache-rs/`

**Version:** 13.1.0

---

## Table of Contents

1. [Introduction to Content-Addressable Storage](#introduction-to-content-addressable-storage)
2. [cacache Architecture](#cacache-architecture)
3. [Hash-Based Lookups](#hash-based-lookups)
4. [Cache Internals](#cache-internals)
5. [LRU Eviction and Garbage Collection](#lru-eviction-and-garbage-collection)
6. [Async API Design](#async-api-design)
7. [Use Cases](#use-cases)
8. [Code Examples](#code-examples)

---

## Introduction to Content-Addressable Storage

### What is Content-Addressable Storage?

**Content-addressable storage (CAS)** is a storage paradigm where data is identified and retrieved by its *content* rather than its *location*. Instead of asking "where is this stored?", you ask "what is this?"

```
Traditional Storage (Location-Addressable):
┌─────────────┬──────────────┐
│  Location   │     Data     │
├─────────────┼──────────────┤
│  /path/to/1 │  "hello"     │
│  /path/to/2 │  "world"     │
└─────────────┴──────────────┘
         ↑
    "Get data at /path/to/1"

Content-Addressable Storage:
┌──────────────────┬──────────────┐
│  Content Hash    │     Data     │
├──────────────────┼──────────────┤
│  sha256-abc123   │  "hello"     │
│  sha256-def456   │  "world"     │
└──────────────────┴──────────────┘
         ↑
    "Get data with hash sha256-abc123"
```

### Why Content-Addressable?

1. **Automatic Deduplication:** Identical content always produces identical hashes
2. **Data Integrity:** Hash verification ensures data hasn't been corrupted
3. **Cache Coherency:** No stale data - same content = same hash
4. **Efficient Storage:** Store once, reference many times
5. **Concurrency:** No locks needed for reads - data is immutable

### How cacache Implements CAS

```
┌─────────────────────────────────────────────────────────┐
│                    cacache Cache                         │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐  │
│  │    Index    │    │    Index    │    │    Index    │  │
│  │  (key→hash) │    │  (key→hash) │    │  (key→hash) │  │
│  │  "user:1"   │    │  "user:2"   │    │  "config"   │  │
│  │    ↓        │    │    ↓        │    │    ↓        │  │
│  │  sha256-a   │    │  sha256-b   │    │  sha256-c   │  │
│  └─────────────┘    └─────────────┘    └─────────────┘  │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │                  Content Store                       ││
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐        ││
│  │  │  sha256-a │  │  sha256-b │  │  sha256-c │        ││
│  │  │  [data]   │  │  [data]   │  │  [data]   │        ││
│  │  └───────────┘  └───────────┘  └───────────┘        ││
│  └─────────────────────────────────────────────────────┘│
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## cacache Architecture

### Directory Structure

```
my-cache/
├── index/          # Key-to-hash mappings (metadata)
│   ├── v5/         # Index version
│   │   ├── a/      # First char of hash for distribution
│   │   ├── b/
│   │   └── ...
└── content-v2/     # Content-addressable data
    ├── sha512/
    │   ├── ab/
    │   │   └── [full hash as filename]
    │   └── cd/
    └── sha256/
        └── ...
```

### Core Components

```rust
// Simplified architecture from cacache source

pub mod content {
    // Content-addressable storage backend
    // Handles writing/reading by hash
    // Implements deduplication automatically
}

pub mod index {
    // Key-to-hash metadata store
    // Tracks which keys point to which content
    // Includes timestamps for LRU
}

// Operations
pub mod get;   // Read operations (by key or hash)
pub mod put;   // Write operations
pub mod rm;    // Removal operations
pub mod ls;    // Listing/enumeration
#[cfg(feature = "link_to")]
pub mod linkto; // Symlink existing files into cache
```

### File Organization

cacache uses a two-level directory structure to avoid filesystem limitations:

```
Content Path Generation:
1. Hash: sha512-ABCDEF123456...
2. Extract algorithm: sha512
3. Extract first 2 chars of hash: AB
4. Path: content-v2/sha512/AB/CDEF123456...

This prevents:
- Single directory with millions of files
- Filesystem performance degradation
```

---

## Hash-Based Lookups

### Subresource Integrity (SRI) Hashes

cacache uses the W3C Subresource Integrity format for hashes, via the `ssri` crate:

```rust
use cacache::{Integrity, Algorithm};

// SRI format: algorithm-base64hash
let sri: Integrity = "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=".parse().unwrap();

// Multiple hashes for the same content (different algorithms)
let multi_hash: Integrity = "sha256-abc123... sha512-def456...".parse().unwrap();
```

### Hash Algorithm Selection

```rust
// cacache supports multiple hash algorithms
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Algorithm {
    Sha512,   // Most secure, larger hash
    Sha384,   // Middle ground
    Sha256,   // Default, good balance
    Sha1,     // Legacy, not cryptographically secure
    Xxh3,     // Non-cryptographic, very fast
}
```

### Lookup Flow

```
Read by Key ("user:123"):
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌──────────┐
│  read(cache,│───▶│  Lookup key │───▶│  Get hash   │───▶│  Return  │
│   "key")    │    │  in index   │    │  sha256-abc │    │  data    │
└─────────────┘    └─────────────┘    └─────────────┘    └──────────┘

Read by Hash (content-addressable):
┌─────────────┐    ┌─────────────┐    ┌──────────┐
│  read_hash( │───▶│  Direct     │───▶│  Return  │
│  cache, sri)│    │  content    │    │  data    │
└─────────────┘    └─────────────┘    └──────────┘
     (faster - skips index lookup!)
```

### Integrity Verification

Every read operation verifies data integrity:

```rust
// From cacache/src/get.rs (simplified)
pub async fn read(cache: &str, key: &str) -> Result<Vec<u8>> {
    // 1. Look up key in index to get hash
    let metadata = index::find(cache, key).await?;

    // 2. Read content by hash
    let data = content::read(cache, &metadata.integrity).await?;

    // 3. Verify integrity - ensures data wasn't corrupted
    metadata.integrity.check(&data)?;

    Ok(data)
}
```

---

## Cache Internals

### Write Path

```rust
// Simplified from cacache/src/put.rs
pub async fn write(cache: &str, key: &str, data: &[u8]) -> Result<Integrity> {
    // 1. Calculate hash of data
    let integrity = Integrity::from(data);

    // 2. Write to content store (deduplicated!)
    // If hash already exists, this is a no-op
    content::write(cache, &integrity, data).await?;

    // 3. Update index to map key to hash
    index::insert(cache, key, integrity.clone()).await?;

    // 4. Return integrity hash for future lookups
    Ok(integrity)
}
```

### Atomic Writes

cacache ensures atomic writes even for large files:

```
1. Write data to temporary file
2. Verify hash matches expected
3. Atomically move temp file to final location
4. Update index (also atomic)

If any step fails, the cache remains unchanged.
```

### Memory-Mapped I/O

```rust
// cacache supports mmap for zero-copy reads
#[cfg(feature = "mmap")]
use memmap2::Mmap;

// Memory-map a file for reading
let file = File::open(&path)?;
let mmap = unsafe { Mmap::map(&file)? };

// Access data without copying to userspace
let data: &[u8] = &mmap[..];
```

### Large File Support

For large files, cacache provides streaming APIs:

```rust
use cacache::Writer;
use async_std::io::WriteExt;

// Stream large data to cache
let mut fd = cacache::Writer::create("./my-cache", "key").await?;
for chunk in data_chunks {
    fd.write_all(chunk).await?;
}
// Commit atomically after all writes complete
let sri = fd.commit().await?;
```

---

## LRU Eviction and Garbage Collection

### Index Metadata

Each index entry contains metadata for cache management:

```rust
// From cacache/src/index.rs
pub struct Metadata {
    pub key: String,           // The lookup key
    pub integrity: Integrity,  // Content hash
    pub time: u128,            // Insertion timestamp (ms since epoch)
    pub size: u64,             // Content size in bytes
}
```

### Manual Garbage Collection

cacache provides primitives for implementing custom eviction policies:

```rust
use cacache::{self, index};

// List all entries
for entry in cacache::ls("./my-cache").await? {
    println!("Key: {}, Size: {}, Time: {}", entry.key, entry.size, entry.time);
}

// Remove by key
cacache::rm("./my-cache", "old-key").await?;

// Remove by hash (content)
cacache::rm_hash("./my-cache", &integrity).await?;

// Clear entire cache
cacache::rm::all("./my-cache").await?;
```

### LRU Implementation Pattern

```rust
// Example LRU eviction implementation
use std::collections::HashMap;

pub async fn evict_lru(cache: &str, max_entries: usize) -> cacache::Result<()> {
    let mut entries: Vec<_> = cacache::ls(cache).await?.collect();

    // Sort by timestamp (oldest first)
    entries.sort_by_key(|e| e.time);

    // Remove oldest entries until under limit
    while entries.len() > max_entries {
        if let Some(oldest) = entries.first() {
            cacache::rm(cache, &oldest.key).await?;
            entries.remove(0);
        }
    }
    Ok(())
}
```

### Content Garbage Collection

When an index entry is removed, the content may still be referenced by other keys:

```rust
// Content is only removed when no keys reference it
pub async fn rm(cache: &str, key: &str) -> Result<bool> {
    // 1. Get metadata for this key
    let metadata = index::find(cache, key)?;

    // 2. Remove index entry
    index::remove(cache, key)?;

    // 3. Check if any other keys reference this content
    let references = index::count_references(cache, &metadata.integrity)?;

    // 4. Only remove content if no references remain
    if references == 0 {
        content::remove(cache, &metadata.integrity)?;
    }

    Ok(true)
}
```

---

## Async API Design

### Runtime Agnostic

cacache supports multiple async runtimes:

```toml
# Cargo.toml - default uses async-std
[dependencies]
cacache = "13.0"

# Or use tokio
[dependencies]
cacache = { version = "13.0", default-features = false, features = ["tokio-runtime", "mmap"] }

# Or sync-only (no async runtime dependency)
[dependencies]
cacache = { version = "13.0", default-features = false, features = ["mmap"] }
```

### Async vs Sync APIs

```rust
// Async API (default)
async fn example_async() -> cacache::Result<()> {
    cacache::write("./cache", "key", b"data").await?;
    let data = cacache::read("./cache", "key").await?;
    Ok(())
}

// Sync API (_sync suffix)
fn example_sync() -> cacache::Result<()> {
    cacache::write_sync("./cache", "key", b"data")?;
    let data = cacache::read_sync("./cache", "key")?;
    Ok(())
}
```

### Performance Characteristics

| Operation | Async | Sync |
|-----------|-------|------|
| Single read/write | Similar | Similar |
| Concurrent operations | Better | Worse |
| Large file streaming | Better | Good |
| Runtime overhead | ~5-10% | None |

---

## Use Cases

### 1. Build System Caching

```rust
// Cache compilation artifacts
async fn compile_with_cache(source: &str) -> Result<Vec<u8>> {
    let cache_key = format!("compile:{}", hash_source(source));

    // Try cache first
    if let Ok(cached) = cacache::read("./build-cache", &cache_key).await {
        return Ok(cached);
    }

    // Compile and cache result
    let output = compile(source).await?;
    cacache::write("./build-cache", &cache_key, &output).await?;
    Ok(output)
}
```

### 2. HTTP Response Caching

```rust
// Cache HTTP responses by URL hash
async fn fetch_with_cache(url: &str) -> Result<reqwest::Response> {
    let cache_key = format!("http:{}", url);

    if let Ok(data) = cacache::read("./http-cache", &cache_key).await {
        return Ok(Response::from_bytes(data)?);
    }

    let response = reqwest::get(url).await?;
    let data = response.bytes().await?;
    cacache::write("./http-cache", &cache_key, &data).await?;
    Ok(Response::from_bytes(data.to_vec())?)
}
```

### 3. Asset Pipeline Caching

```rust
// Cache transformed assets (e.g., SASS → CSS)
async fn transform_asset(input: &[u8]) -> Result<Vec<u8>> {
    let input_hash = Integrity::from(input);
    let cache_key = format!("asset:{}", input_hash);

    // Check for cached transformation
    if let Ok(output) = cacache::read("./asset-cache", &cache_key).await {
        return Ok(output);
    }

    // Transform and cache
    let output = sass::compile(input)?;
    cacache::write("./asset-cache", &cache_key, &output).await?;
    Ok(output)
}
```

### 4. Dependency Caching (like orogene)

```rust
// Cache npm packages by integrity hash
async fn fetch_package(spec: &str) -> Result<Vec<u8>> {
    // Resolve package to get integrity
    let meta = resolve_package(spec).await?;

    // Try content-addressable lookup (fastest!)
    if let Ok(data) = cacache::read_hash("./pkg-cache", &meta.integrity).await {
        return Ok(data);
    }

    // Fetch from registry
    let data = download_package(&meta.integrity).await?;

    // Store with both key and hash
    cacache::write("./pkg-cache", &meta.name, &data).await?;
    Ok(data)
}
```

### 5. Compute Result Memoization

```rust
// Memoize expensive computations
async fn expensive_compute(input: &[u8]) -> Result<Vec<u8>> {
    let cache_key = format!("compute:{}", Integrity::from(input));

    if let Ok(result) = cacache::read("./compute-cache", &cache_key).await {
        return Ok(result);
    }

    let result = do_expensive_work(input).await?;
    cacache::write("./compute-cache", &cache_key, &result).await?;
    Ok(result)
}
```

---

## Code Examples

### Basic Usage

```rust
use cacache;

#[async_std::main]
async fn main() -> Result<(), cacache::Error> {
    let cache_dir = "./my-cache";

    // Write data
    cacache::write(cache_dir, "my-key", b"Hello, world!").await?;

    // Read data back
    let data = cacache::read(cache_dir, "my-key").await?;
    assert_eq!(data, b"Hello, world!");

    // Read by hash (content-addressable)
    let sri = cacache::write(cache_dir, "another-key", b"Content").await?;
    let data = cacache::read_hash(cache_dir, &sri).await?;

    // Clean up
    cacache::rm(cache_dir, "my-key").await?;
    cacache::rm::all(cache_dir).await?;

    Ok(())
}
```

### Streaming Large Files

```rust
use cacache::Writer;
use async_std::io::WriteExt;
use async_std::fs::File;
use async_std::io::ReadExt;

async fn cache_large_file(path: &str) -> cacache::Result<()> {
    let mut file = File::open(path).await?;
    let mut writer = Writer::create("./cache", path).await?;

    let mut buffer = vec![0u8; 8192];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 { break; }
        writer.write_all(&buffer[..n]).await?;
    }

    let integrity = writer.commit().await?;
    println!("Cached with integrity: {}", integrity);
    Ok(())
}
```

### Custom Metadata

```rust
use cacache::{self, index, Value};
use serde_json::json;

async fn write_with_metadata() -> cacache::Result<()> {
    let data = b"Important data";

    // Write data
    cacache::write("./cache", "key", data).await?;

    // Get and update metadata
    let mut meta = index::find("./cache", "key")?;
    meta.info = Some(json!({
        "content-type": "text/plain",
        "etag": "abc123",
        "cached-at": chrono::Utc::now().to_rfc3339(),
    }));
    index::insert("./cache", "key", meta)?;

    Ok(())
}
```

### Error Handling with miette

```rust
use cacache;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
enum CacheError {
    #[error("Cache miss for key: {0}")]
    Miss(String),

    #[error("Cache corrupted: {0}")]
    Corrupted(String),

    #[error(transparent)]
    Cacache(#[from] cacache::Error),
}

async fn robust_cache_lookup(key: &str) -> Result<Vec<u8>, CacheError> {
    match cacache::read("./cache", key).await {
        Ok(data) => Ok(data),
        Err(cacache::Error::NotFound(_)) => Err(CacheError::Miss(key.into())),
        Err(cacache::Error::IntegrityError(e)) => Err(CacheError::Corrupted(e)),
        Err(e) => Err(e.into()),
    }
}
```

---

## Summary

cacache-rs is a production-ready content-addressable cache library that provides:

1. **Content-Addressable Storage:** Data identified by hash, not location
2. **Automatic Deduplication:** Identical content stored once
3. **Integrity Verification:** Every read verifies data integrity
4. **High Concurrency:** Lockless reads, atomic writes
5. **Async-First:** async-std or tokio runtime support
6. **Large File Support:** Streaming APIs for big data
7. **Cross-Platform:** Windows, macOS, Linux support
8. **Excellent Errors:** miette integration for helpful diagnostics

The library is used in production by orogene (package manager) and is suitable for any application requiring high-performance, reliable caching.
