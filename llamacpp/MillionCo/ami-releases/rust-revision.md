---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MillionCo/ami-releases
repository: https://github.com/millionco/ami-releases
revised_at: 2026-03-28T00:00:00Z
workspace: ami-distribution
---

# Rust Revision: Ami Releases Distribution

## Overview

This document describes how to build a Rust-based software distribution system similar to ami-releases. Since the original repository is purely a distribution layer with no application code, the Rust revision focuses on:

1. **Release automation tooling** in Rust
2. **Homebrew tap management**
3. **Auto-update server** for the desktop app
4. **Distribution CLI** for managing releases

## Workspace Structure

```
ami-distribution/
├── Cargo.toml              # Workspace definition
├── crates/
│   ├── release-tool/       # CLI for creating releases
│   ├── homebrew-manager/   # Manages Homebrew tap updates
│   ├── update-server/      # Auto-update API server
│   ├── artifact-signer/    # Code signing utilities
│   └── checksum-gen/       # SHA-256 checksum generation
```

### Crate Breakdown

#### release-tool
- **Purpose:** CLI tool for creating and managing GitHub releases
- **Type:** Binary
- **Public API:** `ami-release create`, `ami-release publish`, `ami-release verify`
- **Dependencies:** octocrab (GitHub API), clap (CLI), tokio

#### homebrew-manager
- **Purpose:** Automatically updates Homebrew cask formulas
- **Type:** Library + Binary
- **Public API:** `update_cask(version, sha256, url)`
- **Dependencies:** git2 (Git operations), toml_edit

#### update-server
- **Purpose:** HTTP API for auto-update checks
- **Type:** Binary (Lambda/Cloudflare Workers compatible)
- **Public API:** `GET /api/latest`, `GET /api/release/:version`
- **Dependencies:** axum or worker (Cloudflare)

#### artifact-signer
- **Purpose:** Code signing and notarization automation
- **Type:** Library
- **Public API:** `sign_binary(path, cert)`, `notarize(path)`
- **Dependencies:** platform-specific signing libraries

#### checksum-gen
- **Purpose:** Generate and verify SHA-256 checksums
- **Type:** Library
- **Public API:** `generate_checksum(path)`, `verify(path, expected)`
- **Dependencies:** sha2

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| GitHub API | octocrab | 0.34 | Official-style GitHub client |
| CLI parsing | clap | 4.4 | Ergonomic CLI with derive macros |
| Async runtime | tokio | 1.0 | Full-featured async runtime |
| HTTP server | axum | 0.7 | Ergonomic, type-safe HTTP |
| Git operations | git2 | 0.18 | libgit2 bindings |
| Checksums | sha2 | 0.10 | SHA-2 family implementation |
| TOML editing | toml_edit | 0.22 | Edit TOML without losing comments |
| Serialization | serde + serde_json | 1.0 | Standard serialization |
| Error handling | thiserror | 1.0 | Derive-based error types |
| Logging | tracing | 0.1 | Async-aware logging |

## Type System Design

### Core Types

```rust
/// Represents a software release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub version: Version,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub assets: Vec<Asset>,
}

/// A release artifact (DMG, EXE, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub content_type: String,
    pub size: u64,
    pub download_count: u64,
    pub browser_download_url: String,
    pub checksum: String,
}

/// Semantic version
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Option<String>,
}

/// Homebrew cask definition
#[derive(Debug, Clone)]
pub struct Cask {
    pub name: String,
    pub version: Version,
    pub sha256: String,
    pub url: String,
    pub homepage: String,
    pub app_path: String,
}
```

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ReleaseError {
    #[error("GitHub API error: {0}")]
    GitHub(#[from] octocrab::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Signing failed: {0}")]
    SigningFailed(String),
}

pub type Result<T> = std::result::Result<T, ReleaseError>;
```

### Traits

```rust
/// Trait for release backends
pub trait ReleaseBackend {
    async fn create_release(&self, release: &Release) -> Result<Release>;
    async fn upload_asset(&self, release_id: u64, path: &Path) -> Result<Asset>;
    async fn publish_release(&self, version: &Version) -> Result<()>;
    async fn get_latest(&self) -> Result<Release>;
}

/// Trait for artifact signers
pub trait ArtifactSigner {
    async fn sign(&self, path: &Path) -> Result<()>;
    async fn verify(&self, path: &Path) -> Result<bool>;
}

/// Trait for checksum generators
pub trait ChecksumGenerator {
    fn generate(&self, path: &Path) -> Result<String>;
    fn verify(&self, path: &Path, expected: &str) -> Result<bool>;
}
```

## Key Rust-Specific Changes

### 1. Type-Safe Version Handling

**Source Pattern:** String-based version comparison in scripts

**Rust Translation:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Option<String>,
}

impl Version {
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim_start_matches('v');
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(ReleaseError::InvalidVersion(s.to_string()));
        }
        Ok(Version {
            major: parts[0].parse()?,
            minor: parts[1].parse()?,
            patch: parts[2].parse()?,
            pre_release: None,
        })
    }
}
```

**Rationale:** Compile-time guarantees, proper ordering, no string parsing at comparison time.

### 2. Ownership for Release Management

**Source Pattern:** Mutable global state in CI scripts

**Rust Translation:**
```rust
pub struct ReleaseManager<B: ReleaseBackend> {
    backend: B,
    signer: Box<dyn ArtifactSigner>,
}

impl<B: ReleaseBackend> ReleaseManager<B> {
    pub async fn publish(&self, version: &Version, artifact_paths: &[PathBuf]) -> Result<()> {
        // Create release
        let release = self.backend.create_release(&Release::new(version)).await?;

        // Sign and upload each artifact
        for path in artifact_paths {
            self.signer.sign(path).await?;
            self.backend.upload_asset(release.id, path).await?;
        }

        // Publish
        self.backend.publish_release(version).await
    }
}
```

**Rationale:** Clear ownership flow, testable with mock backends.

### 3. Error Handling with Context

**Source Pattern:** Exit codes and stderr in shell scripts

**Rust Translation:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum ReleaseError {
    #[error("GitHub API error: {0}")]
    GitHub(#[from] octocrab::Error),

    #[error("Checksum mismatch for {path}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}
```

**Rationale:** Type-safe error propagation with context.

## Concurrency Model

**Approach:** Async with tokio

**Rationale:**
- GitHub API calls are I/O bound
- Multiple artifacts can be uploaded in parallel
- Tokio provides good ergonomics with async/await

```rust
// Parallel artifact upload
let upload_futures: Vec<_> = artifacts
    .iter()
    .map(|artifact| uploader.upload(artifact))
    .collect();

let results = futures::future::join_all(upload_futures).await;
```

## Memory Considerations

- **Artifacts:** Stream large files instead of loading into memory
- **Checksums:** Compute incrementally with `std::io::Read` trait
- **Git operations:** Use git2's streaming APIs for large repos

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Network failure during upload | Retry with `tokio::time::sleep` backoff |
| Corrupted artifact | Checksum verification before upload |
| Version conflict | Type-safe version comparison |
| Signing certificate expiry | Validate certificate before signing |

## Code Examples

### Example: Creating a Release

```rust
use ami_release::{Release, ReleaseManager, Version};
use octocrab::Octocrab;

#[tokio::main]
async fn main() -> Result<()> {
    let github = Octocrab::builder()
        .personal_token(token.to_string())
        .build()?;

    let manager = ReleaseManager::new(github);

    let version = Version::parse("v1.0.0")?;
    let artifacts = vec!["Ami-1.0.0.dmg".into(), "Ami-1.0.0.exe".into()];

    manager.publish(&version, &artifacts).await?;

    Ok(())
}
```

### Example: Homebrew Update

```rust
use homebrew_manager::{Cask, Tap};

async fn update_homebrew_tap(release: &Release) -> Result<()> {
    let cask = Cask {
        name: "ami".to_string(),
        version: release.version.clone(),
        sha256: release.checksum_for("Ami-1.0.0.dmg"),
        url: release.asset_url("Ami-1.0.0.dmg"),
        homepage: "https://ami.dev".to_string(),
        app_path: "Ami.app".to_string(),
    };

    let tap = Tap::open("millionco/ami")?;
    tap.update_cask(&cask)?;
    tap.commit(&format!("Update ami to {}", release.version))?;

    Ok(())
}
```

## Migration Path

For migrating from a shell script-based release process:

1. Implement core types (Version, Release, Asset)
2. Build GitHub API integration with octocrab
3. Add artifact signing
4. Implement Homebrew tap automation
5. Replace shell scripts incrementally

## Performance Considerations

- **Parallel uploads:** Use `futures::future::join_all` for concurrent artifact uploads
- **Streaming:** Stream large artifacts instead of buffering
- **Caching:** Cache GitHub API responses where appropriate

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_ordering() {
        assert!(Version::parse("1.1.0").unwrap() > Version::parse("1.0.0").unwrap());
        assert!(Version::parse("1.0.1").unwrap() > Version::parse("1.0.0").unwrap());
    }

    #[tokio::test]
    async fn test_mock_release() {
        let backend = MockBackend::new();
        let manager = ReleaseManager::new(backend);
        // Test release creation without hitting GitHub API
    }
}
```

## Open Considerations

1. **Platform-specific signing:** macOS notarization requires Apple-specific tooling
2. **Homebrew authentication:** Requires GitHub token with repo permissions
3. **Release validation:** Should we add integration tests that verify releases work?
