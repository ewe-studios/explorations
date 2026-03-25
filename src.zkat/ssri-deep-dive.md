# ssri-rs Deep Dive: Subresource Integrity in Rust

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.zkat/ssri-rs/`

**Version:** 9.2.0

---

## Table of Contents

1. [Introduction to Subresource Integrity](#introduction-to-subresource-integrity)
2. [ssri Architecture](#ssri-architecture)
3. [Hash Algorithms](#hash-algorithms)
4. [Core Types](#core-types)
5. [Hash Verification](#hash-verification)
6. [Multi-Hash Support](#multi-hash-support)
7. [Use Cases](#use-cases)
8. [Code Examples](#code-examples)

---

## Introduction to Subresource Integrity

### What is Subresource Integrity (SRI)?

**Subresource Integrity (SRI)** is a W3C web standard that allows browsers and other systems to verify that fetched resources (like JavaScript files, stylesheets, etc.) have not been tampered with.

The standard defines a specific format for representing cryptographic hashes:

```
algorithm-base64encodedhash
```

Example from HTML:
```html
<script src="https://cdn.example.com/jquery-3.6.0.min.js"
        integrity="sha256-/xUj+3OJU5yExlq6GSYGSHk7tPXikynS7ogEvDej/m4="
        crossorigin="anonymous"></script>
```

### Why SRI Matters

1. **Supply Chain Security:** Verify downloaded content matches expected hash
2. **CDN Trust:** Use third-party CDNs without blind trust
3. **Cache Verification:** Ensure cached content hasn't been corrupted
4. **Content Integrity:** Detect any modification, accidental or malicious

### SRI Format

```
sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=
│      │
│      └── Base64-encoded hash
algorithm

Multiple hashes (space-separated):
sha256-abc123... sha384-def456... sha512-ghi789...
```

---

## ssri Architecture

### Module Structure

```
ssri/
├── lib.rs          # Public API and re-exports
├── algorithm.rs    # Hash algorithm types
├── hash.rs         # Individual hash representation
├── integrity.rs    # Integrity type (collection of hashes)
├── checker.rs      # IntegrityChecker for verification
├── opts.rs         # IntegrityOpts for building
└── errors.rs       # Error types
```

### Type Hierarchy

```
┌─────────────────────────────────────────────────────────┐
│                    Integrity                             │
│  (Collection of Hash entries, sorted by algorithm)      │
│  ┌─────────────────────────────────────────────────┐    │
│  │ hashes: Vec<Hash>                               │    │
│  └─────────────────────────────────────────────────┘    │
│                        │                                 │
│                        ▼                                 │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Hash                                             │    │
│  │  ┌───────────────┬─────────────────────────┐   │    │
│  │  │ algorithm     │ digest                  │   │    │
│  │  │ (Algorithm)   │ (String - base64)       │   │    │
│  │  └───────────────┴─────────────────────────┘   │    │
│  └─────────────────────────────────────────────────┘    │
│                        │                                 │
│                        ▼                                 │
│  ┌─────────────────────────────────────────────────┐    │
│  │ Algorithm                                        │    │
│  │  - Sha512 (most secure)                          │    │
│  │  - Sha384                                        │    │
│  │  - Sha256 (default)                              │    │
│  │  - Sha1 (legacy)                                 │    │
│  │  - Xxh3 (non-cryptographic, fast)                │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

---

## Hash Algorithms

### Supported Algorithms

```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Algorithm {
    /// SHA-512 - Most secure, 512-bit output
    Sha512,

    /// SHA-384 - Middle ground, 384-bit output
    Sha384,

    /// SHA-256 - Default, good balance, 256-bit output
    Sha256,

    /// SHA-1 - Legacy, not cryptographically secure
    /// Included for compatibility but shouldn't be used for security
    Sha1,

    /// XXH3 - Non-cryptographic, very fast, 128-bit output
    /// Good for speed when cryptographic security isn't required
    Xxh3,
}
```

### Algorithm Comparison

| Algorithm | Output Size | Security | Speed | Use Case |
|-----------|-------------|----------|-------|----------|
| SHA-512 | 64 bytes | Highest | Slow | Maximum security |
| SHA-384 | 48 bytes | High | Medium | High security |
| SHA-256 | 32 bytes | Good | Fast | Default choice |
| SHA-1 | 20 bytes | Broken | Fast | Legacy compatibility |
| XXH3 | 16 bytes | None* | Very Fast | Checksums only |

*XXH3 provides collision resistance for accidental changes but is not cryptographically secure.

### Algorithm Ordering

Algorithms are ordered by security (most secure first):

```rust
use ssri::Algorithm;

// Ordering: Sha512 > Sha384 > Sha256 > Sha1 > Xxh3
let mut algorithms = [
    Algorithm::Sha1,
    Algorithm::Sha256,
    Algorithm::Sha384,
    Algorithm::Sha512,
    Algorithm::Xxh3,
];
algorithms.sort();
// Result: [Sha512, Sha384, Sha256, Sha1, Xxh3]
```

This ordering ensures that when picking an algorithm from multiple options, the most secure one is chosen.

---

## Core Types

### Hash

The `Hash` struct represents a single hash entry:

```rust
use ssri::Hash;
use ssri::Algorithm;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Hash {
    pub algorithm: Algorithm,
    pub digest: String,  // Base64-encoded hash
}

impl Hash {
    /// Create a new Hash from raw bytes
    pub fn new(algorithm: Algorithm, digest: &[u8]) -> Self {
        use base64::Engine;
        Self {
            algorithm,
            digest: base64::prelude::BASE64_STANDARD.encode(digest),
        }
    }

    /// Decode the base64 digest back to bytes
    pub fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        base64::prelude::BASE64_STANDARD.decode(&self.digest)
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.algorithm, self.digest)
    }
}

impl std::str::FromStr for Hash {
    type Err = ssri::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse "sha256-abc123..." format
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err(Error::ParseIntegrityError(s.into()));
        }

        let algorithm = parts[0].parse()?;
        Ok(Hash {
            algorithm,
            digest: parts[1].to_string(),
        })
    }
}
```

### Integrity

The `Integrity` struct represents a collection of hashes:

```rust
use ssri::{Integrity, Hash, Algorithm};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Integrity {
    pub hashes: Vec<Hash>,
}

impl Integrity {
    /// Get the most secure algorithm available
    pub fn pick_algorithm(&self) -> Algorithm {
        self.hashes[0].algorithm  // hashes are sorted by security
    }

    /// Check data against this integrity
    pub fn check<B: AsRef<[u8]>>(&self, data: B) -> Result<Algorithm, Error> {
        let mut checker = IntegrityChecker::new(self.clone());
        checker.input(&data);
        checker.result()
    }

    /// Concatenate two Integrity values
    pub fn concat(&self, other: Integrity) -> Self {
        let mut hashes = [self.hashes.clone(), other.hashes].concat();
        hashes.sort();
        hashes.dedup();
        Integrity { hashes }
    }

    /// Check if this matches another Integrity
    pub fn matches(&self, other: &Self) -> Option<Algorithm> {
        let algo = other.pick_algorithm();
        self.hashes
            .iter()
            .filter(|h| h.algorithm == algo)
            .find(|&h| {
                other.hashes.iter().filter(|i| i.algorithm == algo).any(|i| h == i)
            })
            .map(|h| h.algorithm)
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> (Algorithm, String) {
        let hash = &self.hashes[0];
        let decoded = base64::prelude::BASE64_STANDARD.decode(&hash.digest).unwrap();
        (hash.algorithm, hex::encode(decoded))
    }

    /// Create from hex string
    pub fn from_hex<B: AsRef<[u8]>>(hex: B, algorithm: Algorithm) -> Result<Integrity, Error> {
        let decoded = hex::decode(hex).map_err(|e| Error::HexDecodeError(e.to_string()))?;
        let digest = base64::prelude::BASE64_STANDARD.encode(decoded);
        Ok(Integrity {
            hashes: vec![Hash { algorithm, digest }],
        })
    }
}
```

### IntegrityOpts

Builder pattern for creating Integrity values:

```rust
use ssri::{IntegrityOpts, Algorithm};

#[derive(Debug, Clone)]
pub struct IntegrityOpts {
    algorithms: Vec<Algorithm>,
    data: Vec<Vec<u8>>,
}

impl IntegrityOpts {
    pub fn new() -> Self {
        Self {
            algorithms: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Add an algorithm to generate
    pub fn algorithm(mut self, algo: Algorithm) -> Self {
        self.algorithms.push(algo);
        self
    }

    /// Add data incrementally (can be called multiple times)
    pub fn chain<B: AsRef<[u8]>>(mut self, data: B) -> Self {
        self.data.push(data.as_ref().to_vec());
        self
    }

    /// Generate the Integrity
    pub fn result(self) -> Integrity {
        use digest::Digest;

        let mut hashes = Vec::new();

        for algo in &self.algorithms {
            let hash = match algo {
                Algorithm::Sha256 => {
                    let mut hasher = sha2::Sha256::new();
                    for data in &self.data {
                        hasher.update(data);
                    }
                    Hash::new(*algo, &hasher.finalize())
                }
                Algorithm::Sha384 => {
                    let mut hasher = sha2::Sha384::new();
                    for data in &self.data {
                        hasher.update(data);
                    }
                    Hash::new(*algo, &hasher.finalize())
                }
                Algorithm::Sha512 => {
                    let mut hasher = sha2::Sha512::new();
                    for data in &self.data {
                        hasher.update(data);
                    }
                    Hash::new(*algo, &hasher.finalize())
                }
                Algorithm::Sha1 => {
                    let mut hasher = sha1::Sha1::new();
                    for data in &self.data {
                        hasher.update(data);
                    }
                    Hash::new(*algo, &hasher.finalize())
                }
                Algorithm::Xxh3 => {
                    use xxhash_rust::xxh3::Xxh3;
                    let mut hasher = Xxh3::new();
                    for data in &self.data {
                        hasher.update(data);
                    }
                    Hash::new(*algo, &hasher.finish().to_be_bytes())
                }
            };
            hashes.push(hash);
        }

        hashes.sort();
        Integrity { hashes }
    }
}

impl Default for IntegrityOpts {
    fn default() -> Self {
        Self::new().algorithm(Algorithm::Sha256)
    }
}
```

### IntegrityChecker

For verifying data against an Integrity:

```rust
use ssri::{Integrity, IntegrityChecker, Algorithm, Error};

pub struct IntegrityChecker {
    integrity: Integrity,
    hashes: Vec<(Algorithm, Box<dyn digest::Digest>)>,
    matched: Option<Algorithm>,
}

impl IntegrityChecker {
    pub fn new(integrity: Integrity) -> Self {
        let hashes = integrity.hashes.iter().map(|h| {
            let hasher: Box<dyn digest::Digest> = match h.algorithm {
                Algorithm::Sha256 => Box::new(sha2::Sha256::new()),
                Algorithm::Sha384 => Box::new(sha2::Sha384::new()),
                Algorithm::Sha512 => Box::new(sha2::Sha512::new()),
                Algorithm::Sha1 => Box::new(sha1::Sha1::new()),
                Algorithm::Xxh3 => Box::new(xxhash_rust::xxh3::Xxh3::new()),
            };
            (h.algorithm, hasher)
        }).collect();

        Self {
            integrity,
            hashes,
            matched: None,
        }
    }

    /// Add data to check (can be called multiple times for streaming)
    pub fn input<B: AsRef<[u8]>>(&mut self, data: B) {
        for (_, hasher) in &mut self.hashes {
            hasher.update(data.as_ref());
        }
    }

    /// Get the result
    pub fn result(mut self) -> Result<Algorithm, Error> {
        if let Some(matched) = self.matched {
            return Ok(matched);
        }

        for (algo, hasher) in self.hashes {
            let computed = hasher.finalize();

            // Find matching hash in integrity
            for hash in &self.integrity.hashes {
                if hash.algorithm == algo {
                    let expected = hash.decode()?;
                    if computed.as_slice() == expected.as_slice() {
                        self.matched = Some(algo);
                        return Ok(algo);
                    }
                }
            }
        }

        Err(Error::IntegrityError("No matching hash found".into()))
    }
}
```

---

## Hash Verification

### Basic Verification

```rust
use ssri::{Integrity, Algorithm};

// Generate integrity hash for some data
let sri = Integrity::from(b"hello world");
assert_eq!(
    sri.to_string(),
    "sha256-b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
);

// Verify data matches
assert_eq!(sri.check(b"hello world").unwrap(), Algorithm::Sha256);

// Tampered data fails verification
assert!(sri.check(b"goodbye world").is_err());
```

### Streaming Verification

For large files, verify incrementally:

```rust
use ssri::{Integrity, IntegrityChecker};
use std::fs::File;
use std::io::Read;

fn verify_file(path: &str, expected: &Integrity) -> ssri::Result<Algorithm> {
    let mut file = File::open(path)?;
    let mut checker = IntegrityChecker::new(expected.clone());
    let mut buffer = vec![0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        checker.input(&buffer[..n]);
    }

    checker.result()
}
```

### Integrity From Hex

Sometimes you need to convert from hex format (common in Git, etc.):

```rust
use ssri::{Integrity, Algorithm};

// Git-style hex hash
let hex = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
let integrity = Integrity::from_hex(hex, Algorithm::Sha256).unwrap();

assert_eq!(
    integrity.to_string(),
    "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek="
);

// Convert back to hex
let (algo, hex_out) = integrity.to_hex();
assert_eq!(algo, Algorithm::Sha256);
assert_eq!(hex_out, hex);
```

---

## Multi-Hash Support

### Why Multiple Hashes?

1. **Algorithm Migration:** Support both old and new algorithms
2. **Compatibility:** Work with systems using different algorithms
3. **Defense in Depth:** Multiple algorithms provide extra security
4. **Legacy Support:** Support older clients while upgrading

### Creating Multi-Hash Integrity

```rust
use ssri::{IntegrityOpts, Algorithm};

// Generate hashes with multiple algorithms
let sri = IntegrityOpts::new()
    .algorithm(Algorithm::Sha512)
    .algorithm(Algorithm::Sha256)
    .chain(b"hello world")
    .result();

// Result contains both hashes
println!("{}", sri);
// Output: sha256-... sha512-...

// Hashes are sorted by security (most secure first)
assert_eq!(sri.pick_algorithm(), Algorithm::Sha512);
```

### Concatenating Integrity Values

```rust
use ssri::Integrity;

let sri1: Integrity = "sha256-abc123...".parse().unwrap();
let sri2: Integrity = "sha512-def456...".parse().unwrap();

// Combine two integrity values
let combined = sri1.concat(sri2);
assert_eq!(combined.hashes.len(), 2);

// Duplicates are automatically removed
let sri3 = combined.concat(sri1.clone());
assert_eq!(sri3.hashes.len(), 2);  // Still 2, not 3
```

### Matching Integrity

```rust
use ssri::Integrity;

// Single algorithm
let sri1 = Integrity::from(b"hello");

// Multiple algorithms
let sri2 = IntegrityOpts::new()
    .algorithm(Algorithm::Sha512)
    .algorithm(Algorithm::Sha256)
    .chain(b"hello")
    .result();

// Match uses the algorithm from the "other" integrity
let m = sri1.matches(&sri2);
assert_eq!(m, Some(Algorithm::Sha256));  // sri2's preferred algorithm

// Reverse doesn't match because sri1 only has sha256
let m = sri2.matches(&sri1);
assert_eq!(m, None);  // sri1's sha256 doesn't match sri2's sha512
```

---

## Use Cases

### 1. Package Manager Integrity

```rust
use ssri::{Integrity, IntegrityOpts, Algorithm};

// NPM-style package integrity
struct Package {
    name: String,
    version: String,
    integrity: Integrity,
}

impl Package {
    /// Verify downloaded package tarball
    fn verify(&self, tarball_data: &[u8]) -> ssri::Result<()> {
        self.integrity.check(tarball_data)?;
        Ok(())
    }

    /// Generate integrity for new package
    fn generate_integrity(data: &[u8]) -> Integrity {
        IntegrityOpts::new()
            .algorithm(Algorithm::Sha512)  // NPM uses sha512
            .chain(data)
            .result()
    }
}
```

### 2. Build System Artifact Verification

```rust
use ssri::{Integrity, IntegrityOpts};
use std::collections::HashMap;

struct BuildCache {
    // Map artifact path to expected integrity
    manifest: HashMap<String, Integrity>,
}

impl BuildCache {
    fn verify_artifact(&self, path: &str, data: &[u8]) -> bool {
        if let Some(expected) = self.manifest.get(path) {
            expected.check(data).is_ok()
        } else {
            false
        }
    }

    fn cache_artifact(&mut self, path: String, data: &[u8]) -> Integrity {
        let integrity = Integrity::from(data);
        self.manifest.insert(path, integrity.clone());
        integrity
    }

    fn save_manifest(&self, path: &str) -> serde_json::Result<()> {
        // Serialize manifest to JSON
        let json = serde_json::to_string_pretty(&self.manifest)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn load_manifest(path: &str) -> serde_json::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&json)?;
        Ok(Self { manifest })
    }
}
```

### 3. Content-Addressable Storage

```rust
use ssri::{Integrity, Algorithm};

// Content-addressable key is the integrity hash
struct ContentStore {
    base_path: PathBuf,
}

impl ContentStore {
    /// Get path for content by integrity
    fn content_path(&self, integrity: &Integrity) -> PathBuf {
        // Use first (most secure) hash
        let hash = &integrity.hashes[0];
        let algo_dir = format!("{:?}", hash.algorithm).to_lowercase();
        let hash_prefix = &hash.digest[..2];

        self.base_path
            .join(&algo_dir)
            .join(hash_prefix)
            .join(&hash.digest)
    }

    /// Store content, returns integrity for future lookups
    fn store(&self, data: &[u8]) -> std::io::Result<Integrity> {
        let integrity = Integrity::from(data);
        let path = self.content_path(&integrity);

        // Only write if doesn't exist (deduplication)
        if !path.exists() {
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(&path, data)?;
        }

        Ok(integrity)
    }

    /// Retrieve content by integrity
    fn retrieve(&self, integrity: &Integrity) -> std::io::Result<Vec<u8>> {
        let path = self.content_path(integrity);
        let data = std::fs::read(&path)?;

        // Verify integrity on read
        integrity.check(&data)
            .map_err(|e| std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Integrity check failed: {}", e)
            ))?;

        Ok(data)
    }
}
```

### 4. HTML SRI Generation

```rust
use ssri::{Integrity, IntegrityOpts, Algorithm};

/// Generate SRI attribute for HTML script/link tags
fn generate_sri_attribute(url: &str, data: &[u8]) -> Integrity {
    IntegrityOpts::new()
        .algorithm(Algorithm::Sha384)  // Common for web
        .chain(data)
        .result()
}

// Usage:
// let sri = generate_sri_attribute("https://cdn.example.com/app.js", &js_data);
// println!("<script src=\"{}\" integrity=\"{}\"></script>", url, sri);
```

### 5. File Checksum Tool (like srisum)

```rust
use ssri::{IntegrityOpts, Algorithm, Integrity};
use std::path::Path;
use std::fs::File;
use std::io::Read;

fn compute_file_integrity(path: &Path, algorithms: &[Algorithm]) -> ssri::Result<Integrity> {
    let mut opts = IntegrityOpts::new();
    for &algo in algorithms {
        opts = opts.algorithm(algo);
    }

    let mut file = File::open(path)?;
    let mut buffer = vec![0u8; 65536];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        opts = opts.chain(&buffer[..n]);
    }

    Ok(opts.result())
}

fn format_checksum_line(path: &Path, integrity: &Integrity) -> String {
    format!("{}  {}", integrity, path.display())
}
```

---

## Code Examples

### Basic Usage

```rust
use ssri::{Integrity, Algorithm};

fn main() -> ssri::Result<()> {
    // Generate from bytes
    let sri = Integrity::from(b"hello world");
    println!("Integrity: {}", sri);

    // Parse from string
    let parsed: Integrity = "sha256-uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=".parse()?;
    assert_eq!(sri, parsed);

    // Verify data
    assert_eq!(sri.check(b"hello world")?, Algorithm::Sha256);

    // Wrong data fails
    assert!(sri.check(b"goodbye world").is_err());

    Ok(())
}
```

### Multiple Algorithms

```rust
use ssri::{IntegrityOpts, Algorithm};

fn main() {
    let sri = IntegrityOpts::new()
        .algorithm(Algorithm::Sha512)
        .algorithm(Algorithm::Sha384)
        .algorithm(Algorithm::Sha256)
        .chain(b"important data")
        .result();

    println!("Multi-hash SRI: {}", sri);
    // sha256-... sha384-... sha512-...

    // Most secure algorithm is picked
    assert_eq!(sri.pick_algorithm(), Algorithm::Sha512);
}
```

### Incremental/Streaming

```rust
use ssri::{IntegrityOpts, Algorithm};

fn main() {
    let sri = IntegrityOpts::new()
        .algorithm(Algorithm::Sha256)
        .chain(b"hello ")
        .chain(b"world")  // Can chain multiple times
        .result();

    // Same as hashing "hello world" all at once
    assert_eq!(sri, Integrity::from(b"hello world"));
}
```

### Hex Conversion

```rust
use ssri::{Integrity, Algorithm};

fn main() -> ssri::Result<()> {
    // Common in Git and other systems
    let git_sha = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

    let integrity = Integrity::from_hex(git_sha, Algorithm::Sha256)?;
    println!("SRI format: {}", integrity);

    // Convert back
    let (algo, hex) = integrity.to_hex();
    assert_eq!(algo, Algorithm::Sha256);
    assert_eq!(hex, git_sha);

    Ok(())
}
```

### Serialization with serde

```rust
use ssri::Integrity;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct Package {
    name: String,
    version: String,
    integrity: Integrity,
}

fn main() -> serde_json::Result<()> {
    let pkg = Package {
        name: "my-package".to_string(),
        version: "1.0.0".to_string(),
        integrity: Integrity::from(b"package data"),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&pkg)?;
    println!("{}", json);
    // {"name":"my-package","version":"1.0.0","integrity":"sha256-..."}

    // Deserialize from JSON
    let parsed: Package = serde_json::from_str(&json)?;
    assert_eq!(pkg.name, parsed.name);

    Ok(())
}
```

### Error Handling

```rust
use ssri::{Integrity, Error};
use miette::Result;

fn process_with_integrity(expected: &Integrity, data: &[u8]) -> Result<()> {
    match expected.check(data) {
        Ok(algo) => {
            println!("Verified with {}", algo);
            Ok(())
        }
        Err(Error::IntegrityError(msg)) => {
            Err(miette::miette!("Data integrity check failed: {}", msg).into())
        }
        Err(Error::ParseIntegrityError(s)) => {
            Err(miette::miette!("Invalid integrity string: {}", s).into())
        }
        Err(e) => Err(e.into()),
    }
}
```

---

## Summary

ssri-rs is a comprehensive Subresource Integrity library providing:

1. **W3C SRI Compliance:** Strict adherence to the Subresource Integrity specification
2. **Multiple Algorithms:** SHA-512, SHA-384, SHA-256, SHA-1, and XXH3
3. **Multi-Hash Support:** Multiple algorithms in a single integrity string
4. **Streaming Verification:** Incremental hash computation for large files
5. **Hex Conversion:** Easy conversion between SRI and hex formats
6. **serde Integration:** JSON serialization/deserialization
7. **Error Handling:** Comprehensive error types with miette integration
8. **Production Ready:** Used in cacache, orogene, and srisum

The library is essential for any application requiring content integrity verification, from package managers to build systems to web applications.
