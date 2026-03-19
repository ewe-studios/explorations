---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev
repository: git@github.com:joeblew999/utm-dev.git
revised_at: 2026-03-19T12:30:00Z
workspace: utm-dev-rs
---

# Rust Revision: utm-dev

## Overview

This document translates the Go-based utm-dev toolchain into idiomatic Rust. The translation leverages Rust's type safety, zero-cost abstractions, and memory safety guarantees while maintaining compatibility with the original tool's features and workflows.

Key translation decisions:
- **Workspace structure**: 5 crates for separation of concerns
- **Async runtime**: Tokio for concurrent operations (downloads, VM commands)
- **Error handling**: thiserror for libraries, anyhow for applications
- **CLI**: clap with derive macros for ergonomic command definitions
- **Serialization**: serde with JSON/YAML support for configs

## Workspace Structure

```
utm-dev-rs/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── README.md
├── LICENSE
├── crates/
│   ├── utm-dev-core/             # Core types, utilities, traits
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs         # Configuration types
│   │       ├── error.rs          # Error types (thiserror)
│   │       ├── paths.rs          # Path resolution utilities
│   │       └── utils.rs          # Common utilities
│   │
│   ├── utm-dev-build/            # Build system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── gogio.rs          # gogio wrapper
│   │       ├── cache.rs          # Build cache (SHA256)
│   │       ├── icons.rs          # Icon generation
│   │       └── platforms/
│   │           ├── mod.rs
│   │           ├── macos.rs
│   │           ├── ios.rs
│   │           ├── android.rs
│   │           ├── windows.rs
│   │           └── linux.rs
│   │
│   ├── utm-dev-utm/              # UTM integration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── utmctl.rs         # utmctl wrapper
│   │       ├── applescript.rs    # AppleScript execution
│   │       ├── driver.rs         # Version-specific driver
│   │       ├── port_forward.rs   # Port forwarding
│   │       ├── vm.rs             # VM management
│   │       └── gallery.rs        # VM gallery templates
│   │
│   ├── utm-dev-sdk/              # SDK management
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── installer.rs      # SDK installer
│   │       ├── cache.rs          # Download cache
│   │       ├── download.rs       # HTTP downloads with progress
│   │       └── extract.rs        # Archive extraction
│   │
│   └── utm-dev-cli/              # CLI interface
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── build.rs
│           │   ├── run.rs
│           │   ├── utm.rs
│           │   ├── bundle.rs
│           │   └── icons.rs
│           └── completions.rs
│
└── tests/
    ├── integration_tests.rs
    └── fixtures/
```

### Crate Breakdown

#### utm-dev-core
- **Purpose:** Shared types, traits, and utilities used across all crates
- **Type:** library
- **Public API:** `Config`, `PathResolver`, `UtmDevError`, `Result<T>`
- **Dependencies:** serde, thiserror, directories

#### utm-dev-build
- **Purpose:** Cross-platform build orchestration (gogio wrapper)
- **Type:** library
- **Public API:** `BuildConfig`, `BuildCache`, `Platform`, `Builder`
- **Dependencies:** utm-dev-core, sha2, serde_json, tokio, image

#### utm-dev-utm
- **Purpose:** UTM VM management and automation
- **Type:** library
- **Public API:** `UtmDriver`, `VM`, `PortForward`, `UtmCtl`
- **Dependencies:** utm-dev-core, tokio, regex, plist (for AppleScript)

#### utm-dev-sdk
- **Purpose:** SDK download, verification, and installation
- **Type:** library
- **Public API:** `SdkInstaller`, `SdkConfig`, `DownloadCache`
- **Dependencies:** utm-dev-core, reqwest, sha2, tar, zip, flate2, indicatif

#### utm-dev-cli
- **Purpose:** Command-line interface (binary crate)
- **Type:** binary
- **Public API:** N/A (executable)
- **Dependencies:** All above crates, clap, tokio, tracing-subscriber

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| CLI framework | clap | 4.4 | Derive macros, completions, env vars |
| Async runtime | tokio | 1.35 | Full features for networking, fs, process |
| HTTP client | reqwest | 0.11 | Async downloads, streaming, JSON |
| Serialization | serde + serde_json | 1.0 | Industry standard |
| YAML support | serde_yaml | 0.9 | Config file parsing |
| Error handling | thiserror | 1.0 | Library error types |
| Error handling | anyhow | 1.0 | Application-level errors |
| Logging | tracing + tracing-subscriber | 0.1 | Structured logging |
| Progress bar | indicatif | 0.17 | Download progress, spinners |
| Checksums | sha2 | 0.10 | SHA256 for build cache |
| Hex encoding | hex | 0.4 | Checksum display |
| Image processing | image | 0.24 | Icon generation |
| Archive (tar) | tar | 0.4 | Tar extraction |
| Archive (zip) | zip | 0.6 | Zip extraction |
| Compression | flate2 | 1.0 | gzip support |
| Directories | directories | 5.0 | OS-specific paths |
|.plist parsing | plist | 1.5 | macOS Info.plist, AppleScript |
| Regex | regex | 1.10 | Pattern matching |
| Walk directory | walkdir | 2.4 | Recursive file hashing |

## Type System Design

### Core Types

```rust
// crates/utm-dev-core/src/config.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Platform target for builds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Macos,
    Android,
    Ios,
    IosSimulator,
    Windows,
    Linux,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Macos => write!(f, "macos"),
            Platform::Android => write!(f, "android"),
            Platform::Ios => write!(f, "ios"),
            Platform::IosSimulator => write!(f, "ios-simulator"),
            Platform::Windows => write!(f, "windows"),
            Platform::Linux => write!(f, "linux"),
        }
    }
}

/// Build configuration for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub platform: Platform,
    pub app_dir: PathBuf,
    pub output_dir: Option<PathBuf>,
    pub force_rebuild: bool,
    pub skip_icons: bool,
    pub schemes: Option<String>,  // Deep linking schemes
    pub queries: Option<String>,  // Android app queries
    pub sign_key: Option<String>, // Signing key/profile
}

/// SDK configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkConfig {
    pub name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
}
```

### Error Types

```rust
// crates/utm-dev-core/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum UtmDevError {
    #[error("project validation failed: {0}")]
    ProjectValidation(String),

    #[error("build failed for {platform}: {source}")]
    BuildFailed {
        platform: crate::Platform,
        #[source]
        source: anyhow::Error,
    },

    #[error("UTM operation failed: {0}")]
    UtmError(String),

    #[error("SDK installation failed: {0}")]
    SdkInstall(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
}

pub type Result<T> = std::result::Result<T, UtmDevError>;
```

### Traits

```rust
// crates/utm-dev-utm/src/driver.rs

/// Driver trait for UTM operations (version-agnostic)
pub trait UtmDriver: Send + Sync {
    /// Returns UTM version string
    fn version(&self) -> &str;

    /// Check if export is supported (UTM 4.6+)
    fn supports_export(&self) -> bool;

    /// Check if import is supported (UTM 4.6+)
    fn supports_import(&self) -> bool;

    /// Check if guest tools are available (UTM 4.6+)
    fn supports_guest_tools(&self) -> bool;

    /// Export VM to .utm file
    fn export(&self, vm_name: &str, output_path: &Path) -> Result<()>;

    /// Import VM from .utm file
    fn import(&self, utm_path: &Path) -> Result<String>;

    /// Execute command in VM
    fn exec(&self, vm_name: &str, command: &str) -> Result<()>;

    /// Push file to VM
    fn push_file(&self, vm_name: &str, local: &Path, remote: &str) -> Result<()>;

    /// Pull file from VM
    fn pull_file(&self, vm_name: &str, remote: &str, local: &Path) -> Result<()>;
}
```

## Key Rust-Specific Changes

### 1. Error Handling with thiserror

**Source Pattern (Go):**
```go
func buildMacOS(proj *GioProject) error {
    if err := validate(proj); err != nil {
        return fmt.Errorf("build failed: %w", err)
    }
    return nil
}
```

**Rust Translation:**
```rust
fn build_macos(proj: &GioProject) -> Result<()> {
    validate(proj)?;  // ? propagates errors
    Ok(())
}
```

**Rationale:** Rust's `?` operator provides cleaner error propagation. `thiserror` generates Display impl automatically.

### 2. Build Cache with Owned Types

**Source Pattern (Go):**
```go
type CacheEntry struct {
    SourceHash  string
    OutputPath  string
    Timestamp   time.Time
}
```

**Rust Translation:**
```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub source_hash: String,
    pub output_path: PathBuf,
    pub timestamp: u64,  // Unix timestamp for serde compatibility
    pub platform: Platform,
}
```

**Rationale:** `PathBuf` for owned paths, `u64` timestamp for easier serialization.

### 3. Async Downloads with Streaming

**Source Pattern (Go):**
```go
resp, _ := http.Get(url)
io.Copy(tmpFile, resp.Body)
```

**Rust Translation:**
```rust
let response = client.get(url).send().await?;
let mut stream = response.bytes_stream();

while let Some(chunk) = stream.next().await {
    let data = chunk?;
    file.write_all(&data).await?;
    progress_bar.inc(data.len() as u64);
}
```

**Rationale:** Async streaming avoids loading entire file into memory.

### 4. Platform-Specific Code with cfg_attr

**Source Pattern (Go):**
```go
// pkg/screenshot/cgwindow_darwin.go
//go:build darwin

func captureScreen() {}
```

**Rust Translation:**
```rust
// crates/utm-dev-build/src/platforms/macos.rs
#[cfg(target_os = "macos")]
pub fn capture_screen() -> Result<image::DynamicImage> {
    // macOS-specific implementation
}
```

**Rationale:** Rust's `cfg` attributes are more flexible than Go build tags.

## Ownership & Borrowing Strategy

```rust
// Build cache owns the cache file, borrows source paths for hashing
pub struct BuildCache {
    entries: HashMap<String, CacheEntry>,
    cache_path: PathBuf,
}

impl BuildCache {
    // Borrows source_dir, doesn't take ownership
    pub fn needs_rebuild(
        &self,
        app_name: &str,
        platform: Platform,
        source_dir: &Path,  // Borrowed
        output_path: &Path,
    ) -> (bool, &'static str) {
        // ...
    }

    // Takes mutable reference to update cache
    pub fn record_build(
        &mut self,  // Mutable borrow
        app_name: &str,
        platform: Platform,
        source_dir: &Path,
        output_path: &Path,
    ) -> Result<()> {
        // ...
    }
}

// VM struct owns its data
pub struct VM {
    pub uuid: String,
    pub name: String,
    pub status: VMStatus,
}
```

## Concurrency Model

**Approach:** Async with Tokio

**Rationale:**
- Network operations (SDK downloads, HTTP requests) benefit from async
- Process execution (gogio, utmctl) can run blocking in spawn_blocking
- CLI is single-threaded user-facing, no need for multi-threading complexity

```rust
// crates/utm-dev-sdk/src/installer.rs

use tokio::task::spawn_blocking;

pub async fn install_sdk(&self, sdk: &SdkConfig) -> Result<()> {
    // Async download
    let downloaded = self.download(sdk).await?;

    // Blocking extraction in spawn_blocking
    let install_path = sdk.install_path.clone();
    spawn_blocking(move || {
        extract_archive(&downloaded, &install_path)
    })
    .await??;

    Ok(())
}
```

## Memory Considerations

- **Stack vs. Heap:** Small types (Platform enum, u64) on stack; large types (String, PathBuf, Vec) on heap
- **Arc for shared state:** SDK cache could use `Arc<Mutex<Cache>>` for concurrent access
- **No unsafe code required:** All operations use safe Rust APIs

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Partial downloads | Checksum verification before install |
| Concurrent builds | `tokio::sync::Mutex` for cache access |
| Missing SDK paths | `Option<PathBuf>` forces explicit handling |
| Invalid platform | `Platform` enum is closed (no invalid values) |
| UTM version mismatch | Driver trait with version-specific impls |
| File system errors | `std::io::Error` wrapped in `UtmDevError::Io` |

## Code Examples

### Example: Build Cache Implementation

```rust
// crates/utm-dev-build/src/cache.rs

use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub source_hash: String,
    pub output_path: PathBuf,
    pub timestamp: u64,
    pub platform: String,
}

pub struct BuildCache {
    entries: HashMap<String, CacheEntry>,
    cache_path: PathBuf,
}

impl BuildCache {
    pub fn new(cache_path: PathBuf) -> Result<Self> {
        let entries = if cache_path.exists() {
            let content = fs::read_to_string(&cache_path)?;
            serde_json::from_str(&content)?
        } else {
            HashMap::new()
        };

        Ok(Self { entries, cache_path })
    }

    pub fn needs_rebuild(
        &self,
        app_name: &str,
        platform: Platform,
        source_dir: &Path,
        output_path: &Path,
    ) -> (bool, &'static str) {
        // Check output exists
        if !output_path.exists() {
            return (true, "output does not exist");
        }

        // Calculate source hash
        let current_hash = match hash_directory(source_dir) {
            Ok(hash) => hash,
            Err(_) => return (true, "failed to hash source"),
        };

        // Compare with cache
        let key = format!("{}:{}", app_name, platform);
        if let Some(entry) = self.entries.get(&key) {
            if entry.source_hash == current_hash {
                return (false, "up-to-date");
            }
        }

        (true, "source changed")
    }

    pub fn record_build(
        &mut self,
        app_name: &str,
        platform: Platform,
        source_dir: &Path,
        output_path: &Path,
    ) -> Result<()> {
        let key = format!("{}:{}", app_name, platform);
        let hash = hash_directory(source_dir)?;

        self.entries.insert(key, CacheEntry {
            source_hash: hash,
            output_path: output_path.to_path_buf(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            platform: platform.to_string(),
        });

        self.save()
    }

    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.cache_path, content)?;
        Ok(())
    }
}

fn hash_directory(dir: &Path) -> Result<String> {
    let mut hasher = Sha256::new();

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let content = fs::read(entry.path())?;
            hasher.update(&content);
        }
    }

    Ok(hex::encode(hasher.finalize()))
}
```

### Example: UTM Driver Implementation

```rust
// crates/utm-dev-utm/src/driver.rs

use std::process::Command;
use regex::Regex;

pub struct UtmCtl {
    path: PathBuf,
}

impl UtmCtl {
    pub fn new() -> Result<Self> {
        // Find utmctl in standard locations
        let path = find_utmctl()?;
        Ok(Self { path })
    }

    pub fn run(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.path)
            .args(args)
            .output()
            .map_err(|e| UtmDevError::UtmError(format!("utmctl spawn failed: {}", e)))?;

        if !output.status.success() {
            return Err(UtmDevError::UtmError(format!(
                "utmctl failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    }

    pub fn list_vms(&self) -> Result<Vec<VM>> {
        let output = self.run(&["list"])?;
        parse_vm_list(&output)
    }
}

/// Version-specific driver for UTM 4.6+
pub struct UtmDriver46 {
    version: String,
    utmctl: UtmCtl,
}

impl UtmDriver for UtmDriver46 {
    fn version(&self) -> &str {
        &self.version
    }

    fn supports_export(&self) -> bool {
        true  // 4.6+ feature
    }

    fn export(&self, vm_name: &str, output_path: &Path) -> Result<()> {
        let vm_id = get_vm_uuid(&self.utmctl, vm_name)?;
        let abs_path = std::fs::canonicalize(output_path)?;

        // Use AppleScript for export
        let script = format!(
            r#"tell application "UTM" to export virtual machine id "{}" to POSIX file "{}""#,
            vm_id,
            abs_path.display()
        );

        run_applescript(&script)?;
        Ok(())
    }

    // ... other trait methods
}
```

## Migration Path

1. **Phase 1: Core Infrastructure**
   - Implement utm-dev-core with types and errors
   - Set up workspace structure
   - Create basic CLI skeleton with clap

2. **Phase 2: Build System**
   - Implement gogio wrapper
   - Port build cache with SHA256 hashing
   - Implement icon generation with image crate

3. **Phase 3: UTM Integration**
   - Implement utmctl wrapper
   - Port AppleScript execution
   - Implement driver pattern for versions

4. **Phase 4: SDK Management**
   - Implement async downloads with reqwest
   - Port archive extraction
   - Implement checksum verification

5. **Phase 5: Testing & Polish**
   - Integration tests
   - Shell completions
   - Documentation

## Performance Considerations

- **Parallel builds:** `tokio::spawn` for building multiple platforms concurrently
- **Streaming downloads:** Avoid loading large SDKs into memory
- **Incremental hashing:** Cache individual file hashes for faster rebuild detection
- **Zero-copy parsing:** Use `&str` slices where possible to avoid allocations

## Testing Strategy

```rust
// tests/integration_tests.rs

#[cfg(test)]
mod tests {
    use utm_dev_build::{BuildCache, Platform};
    use tempfile::TempDir;

    #[test]
    fn test_build_cache_detects_changes() {
        let temp = TempDir::new().unwrap();
        let cache_path = temp.path().join("cache.json");

        let mut cache = BuildCache::new(cache_path.clone()).unwrap();

        // Create source file
        let source_dir = temp.path().join("src");
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("main.go"), "package main").unwrap();

        let output_path = temp.path().join("output.app");
        fs::write(&output_path, "binary").unwrap();

        // First check: needs rebuild (not in cache)
        let (needs_rebuild, _) = cache.needs_rebuild(
            "testapp",
            Platform::Macos,
            &source_dir,
            &output_path,
        );
        assert!(needs_rebuild);

        // Record build
        cache.record_build("testapp", Platform::Macos, &source_dir, &output_path).unwrap();

        // Second check: up-to-date
        let (needs_rebuild, _) = cache.needs_rebuild(
            "testapp",
            Platform::Macos,
            &source_dir,
            &output_path,
        );
        assert!(!needs_rebuild);

        // Modify source
        fs::write(source_dir.join("main.go"), "package main // modified").unwrap();

        // Third check: needs rebuild (source changed)
        let (needs_rebuild, _) = cache.needs_rebuild(
            "testapp",
            Platform::Macos,
            &source_dir,
            &output_path,
        );
        assert!(needs_rebuild);
    }
}
```

## Open Considerations

1. **gogio dependency:** Rust implementation still needs to invoke gogio (Go tool). Consider if pure-Rust cross-compilation is feasible.

2. **AppleScript on non-macOS:** The utm-dev-utm crate is macOS-only. Need clear feature flags to disable on Linux/Windows.

3. **Async vs. sync CLI:** User-facing CLI is synchronous, but internal operations benefit from async. Need careful tokio runtime setup.

4. **Icon generation:** The Go implementation uses external tools for some icon formats. Rust image crate support needs verification.
