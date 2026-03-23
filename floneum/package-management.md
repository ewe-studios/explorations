# Floneum Package Management (Floneumite) Deep Dive

## Overview

**Floneumite** is the package management system for Floneum. It handles plugin discovery, installation, and updates by searching GitHub repositories and maintaining a local package index.

**Source Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.floneum/floneum/floneum/floneumite/`

## Architecture

```
floneumite/
├── src/
│   ├── lib.rs       # Module exports and utilities
│   ├── index.rs     # Package index management
│   └── package.rs   # Package structure and metadata
└── Cargo.toml
```

## Package Index

### Index Structure

```rust
#[derive(Default, Deserialize, Serialize, Debug)]
pub struct FloneumPackageIndex {
    fetch_successful: bool,
    last_fetched: u64,
    entries: Vec<PackageIndexEntry>,
}
```

### Loading the Index

```rust
impl FloneumPackageIndex {
    pub async fn load() -> Self {
        match Self::load_from_fs().await {
            Ok(mut index) => {
                if let Err(err) = index.update().await {
                    log::error!("Error updating package index: {}", err);
                }
                index
            }
            Err(err) => {
                log::error!("Error loading package index from file system: {}", err);
                log::info!("Loading package index from github");
                match Self::fetch().await {
                    Ok(index) => index,
                    Err(err) => {
                        log::error!("Error loading package index: {}", err);
                        log::info!("Using empty package index");
                        Self::default()
                    }
                }
            }
        }
    }

    pub async fn load_from_fs() -> anyhow::Result<Self> {
        let path = packages_path()?;
        let index_path = path.join("index.toml");
        log::info!("loading index from {index_path:?}");
        Ok(toml::from_str::<Self>(
            &tokio::fs::read_to_string(index_path).await?,
        )?)
    }
}
```

### Index Update Strategy

```rust
const PACKAGE_INDEX_TIMEOUT: u64 = 60 * 60 * 24 * 3; // 3 days

pub async fn update(&mut self) -> anyhow::Result<()> {
    if SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - self.last_fetched
        > PACKAGE_INDEX_TIMEOUT
        || !self.fetch_successful
    {
        log::info!("updating index");
        *self = Self::fetch().await?;
    } else {
        // Update individual packages
        for entry in &self.entries {
            if entry.is_expired() {
                if let Err(err) = entry.update().await {
                    log::error!("Error updating package: {}", err);
                }
            }
        }
    }
    Ok(())
}
```

## Package Discovery

### GitHub Topic Search

Floneumite discovers plugins by searching GitHub repositories with the topic `floneum-v{version}`:

```rust
#[tracing::instrument]
pub async fn fetch() -> anyhow::Result<Self> {
    let path = packages_path()?;
    let instance = &*OCTOCRAB;

    // Search GitHub for repos with floneum topic
    let page = instance
        .search()
        .repositories(&format!("topic:floneum-v{}", crate::CURRENT_BINDING_VERSION))
        .sort("stars")
        .order("desc")
        .send()
        .await?;

    let mut combined_packages = Vec::new();
    let mut full_success = true;

    for item in page.items {
        match Self::fetch_repo(item, path.to_path_buf()).await {
            Ok(mut new) => combined_packages.append(&mut new),
            Err(err) => {
                log::error!("Error fetching repo: {}", err);
                full_success = false;
            }
        }
    }

    // Save index for offline use
    let index_path = path.join("index.toml");
    let config = Self {
        last_fetched: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        entries: combined_packages,
        fetch_successful: full_success,
    };

    let index = toml::to_string(&config)?;
    log::info!("saved index @{index_path:?}");
    tokio::fs::write(index_path, index).await?;

    Ok(config)
}
```

### Repository Processing

```rust
async fn fetch_repo(
    item: octocrab::models::Repository,
    path: PathBuf,
) -> anyhow::Result<Vec<PackageIndexEntry>> {
    let instance = &*OCTOCRAB;
    let mut combined_packages = Vec::new();

    if let Some(author) = &item.owner {
        let repo_handle = instance.repos(author.login.clone(), item.name.clone());
        let commits = repo_handle.list_commits().send().await?;

        if let Some(last_commit) = commits.items.first() {
            log::info!("found repo user: {} repo: {}", author.login, item.name);
            let commit_sha = last_commit.sha.clone();

            // Fetch floneum.toml from dist folder
            let file = repo_handle
                .raw_file(last_commit.sha.clone(), "dist/floneum.toml")
                .await?;

            let body = file.into_body();
            let bytes = hyper::body::to_bytes(body).await;

            if let Ok(as_str) = std::str::from_utf8(&bytes.unwrap()) {
                if let Ok(package) = toml::from_str::<Config>(as_str) {
                    log::trace!("found package: {:#?}", package);

                    for package in package.packages() {
                        // Check binding version compatibility
                        let binding_version = &package.binding_version;
                        if binding_version == "*" {
                            log::warn!("The exact version of floneum_rust is not specified in Cargo.toml");
                        } else if !VersionReq::parse(binding_version)
                            .unwrap()
                            .matches(&Version::parse(env!("CARGO_PKG_VERSION")).unwrap())
                        {
                            log::info!("skipping package: {} binding version mismatch", package.name);
                            continue;
                        }

                        match Self::fetch_package_entry(
                            path.clone(),
                            commit_sha.clone(),
                            RepoId::new(author.login.clone(), item.name.clone()),
                            package.clone(),
                            &commits,
                        ).await {
                            Ok(package) => combined_packages.push(package),
                            Err(err) => log::error!("Error fetching package: {}", err),
                        }
                    }
                }
            }
        }
    }

    Ok(combined_packages)
}
```

### Package Entry Fetching

```rust
async fn fetch_package_entry(
    path: PathBuf,
    commit_sha: String,
    repo: RepoId,
    package: PackageStructure,
    commit: &Page<RepoCommit>,
) -> anyhow::Result<PackageIndexEntry> {
    log::info!("found: {}", package.name);

    // Normalize case and URL encode package name
    let repo_path = format!(
        "dist/{}/package.wasm",
        urlencoding::encode(&package.name.to_lowercase())
    );

    // Download WASM binary
    let bytes = repo.get_file(&repo_path, commit).await?;

    // Save to local packages directory
    let package_path = path.join(&package.name).join(&package.package_version);
    tokio::fs::create_dir_all(&package_path).await?;
    let wasm_path = package_path.join("package.wasm");
    tokio::fs::write(wasm_path, bytes).await?;

    let remote = Remote::new(package.clone(), repo.clone(), commit_sha.clone());
    let package = PackageIndexEntry::new(package_path, Some(package), Some(remote));

    Ok(package)
}
```

## Package Entry Structure

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PackageIndexEntry {
    path: std::path::PathBuf,
    meta: Option<PackageStructure>,
    remote: Option<Remote>,
}

impl PackageIndexEntry {
    pub fn new(
        path: std::path::PathBuf,
        meta: Option<PackageStructure>,
        remote: Option<Remote>,
    ) -> Self {
        let mut path = path;
        // Store relative path
        if let Ok(new) = path.strip_prefix(packages_path().unwrap()) {
            path = new.to_path_buf();
        }
        log::info!("found: {}", path.display());
        Self { path, remote, meta }
    }

    pub fn is_expired(&self) -> bool {
        match &self.remote {
            Some(remote) => remote.is_expired(),
            None => false,
        }
    }

    pub async fn update(&self) -> anyhow::Result<()> {
        if let Some(remote) = &self.remote {
            remote.update().await?;
        }
        Ok(())
    }

    pub fn path(&self) -> std::path::PathBuf {
        packages_path().unwrap().join(&self.path)
    }

    pub fn wasm_path(&self) -> std::path::PathBuf {
        let path = self.path();
        if let Some("wasm") = path.extension().and_then(|ext| ext.to_str()) {
            return path;
        }
        path.join("package.wasm")
    }

    pub async fn wasm_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let wasm_path = self.wasm_path();
        log::info!("loading wasm from {wasm_path:?}");
        Ok(tokio::fs::read(wasm_path).await?)
    }
}
```

## Remote Package Tracking

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Remote {
    last_fetched: u64,
    sha: String,
    repo: RepoId,
    structure: package::PackageStructure,
}

impl Remote {
    pub fn new(structure: PackageStructure, repo: RepoId, sha: String) -> Self {
        Self {
            last_fetched: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            repo,
            sha,
            structure,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.last_fetched > PACKAGE_INDEX_TIMEOUT
    }

    pub async fn update(&self) -> anyhow::Result<()> {
        if self.is_expired() {
            self.repo
                .update(
                    &self.structure.name,
                    &self.structure.package_version,
                    &self.sha,
                )
                .await?;
        }
        Ok(())
    }
}
```

## Repository ID and File Fetching

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RepoId {
    pub owner: String,
    pub name: String,
}

impl RepoId {
    pub fn new(owner: String, name: String) -> Self {
        Self { owner, name }
    }

    pub async fn get_file(
        &self,
        path: &str,
        commits: &Page<RepoCommit>,
    ) -> anyhow::Result<Vec<u8>> {
        let instance = &*OCTOCRAB;
        let repo_handle = instance.repos(self.owner.clone(), self.name.clone());

        if let Some(last_commit) = commits.items.first() {
            let file = repo_handle.raw_file(last_commit.sha.clone(), path).await?;
            let body = file.into_body();
            let bytes = hyper::body::to_bytes(body).await?;

            // Security check: suspiciously small files
            if bytes.len() < 5000usize {
                log::error!("fetched file is suspiciously small");
                log::error!("File contents: {:?}", bytes);
            }

            Ok(bytes.to_vec())
        } else {
            Err(anyhow::anyhow!("No commits found"))
        }
    }

    pub async fn update(&self, name: &str, version: &str, old_sha: &str) -> anyhow::Result<()> {
        let instance = &*OCTOCRAB;
        let repo_handle = instance.repos(self.owner.clone(), self.name.clone());
        let commits = repo_handle.list_commits().send().await?;

        if let Some(last_commit) = commits.items.first() {
            // Skip if already up to date
            if last_commit.sha == old_sha {
                return Ok(());
            }

            let repo_path = format!("dist/{}/package.wasm", name);
            let file = repo_handle
                .raw_file(last_commit.sha.clone(), repo_path)
                .await?;

            let body = file.into_body();
            if let Ok(bytes) = hyper::body::to_bytes(body).await {
                let package_path = packages_path()?.join(name).join(version);
                tokio::fs::create_dir_all(&package_path).await?;
                let wasm_path = package_path.join("package.wasm");
                tokio::fs::write(wasm_path, bytes).await?;
            }
        }
        Ok(())
    }
}
```

## Package Structure

```rust
// From package.rs
pub struct PackageStructure {
    pub name: String,
    pub description: String,
    pub package_version: String,
    pub binding_version: String,
    // ... additional metadata
}
```

## GitHub API Client

```rust
static OCTOCRAB: Lazy<octocrab::Octocrab> = Lazy::new(|| {
    match std::env::var("GITHUB_TOKEN") {
        Ok(token) => octocrab::OctocrabBuilder::new()
            .personal_token(token)
            .build()
            .unwrap_or_else(|err| {
                tracing::error!("Failed to create octocrab instance: {}", err);
                unauthenticated_octocrab()
            }),
        Err(_) => unauthenticated_octocrab(),
    }
});

fn unauthenticated_octocrab() -> octocrab::Octocrab {
    tracing::warn!("No GITHUB_TOKEN found, using unauthenticated requests.");
    octocrab::OctocrabBuilder::new().build().unwrap()
}
```

## Packages Directory

```rust
pub const CURRENT_BINDING_VERSION: usize = 3;

pub fn packages_path() -> anyhow::Result<std::path::PathBuf> {
    let base_dirs = BaseDirs::new().ok_or_else(|| anyhow!("No home directory found"))?;
    let path = base_dirs
        .data_dir()
        .join("floneum")
        .join(format!("v{}", CURRENT_BINDING_VERSION))
        .join("packages");
    std::fs::create_dir_all(&path)?;
    Ok(path)
}
```

## Package Configuration

The `floneum.toml` file format:

```toml
[package]
name = "my-plugin-repo"
version = "0.1.0"
binding_version = "3"

[[packages]]
name = "my-first-plugin"
package_version = "0.1.0"
description = "A plugin that does something"

[[packages]]
name = "my-second-plugin"
package_version = "0.1.0"
description = "Another plugin"
```

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `octocrab` | GitHub API client |
| `toml` | Configuration parsing |
| `directories` | Platform-specific paths |
| `hyper` | HTTP body handling |
| `urlencoding` | URL-safe package names |
| `semver` | Version checking |
| `serde` | Serialization |
| `tokio` | Async runtime |
| `tracing` | Logging |

## Security Considerations

1. **Version Validation:** Binding version must match exactly (or use wildcard)
2. **Size Check:** WASM files under 5KB are flagged as suspicious
3. **SHA Verification:** Package updates tracked by commit SHA
4. **Rate Limiting:** Uses authenticated GitHub API when token available

## Index File Format

The index is stored as TOML:

```toml
fetch_successful = true
last_fetched = 1711123456

[[entries]]
path = "floneum-generate-text/0.1.0"

[entries.meta]
name = "floneum-generate-text"
description = "Generate text using LLMs"
package_version = "0.1.0"

[entries.remote]
last_fetched = 1711123456
sha = "abc123..."
repo = { owner = "floneum", name = "floneum" }
```
