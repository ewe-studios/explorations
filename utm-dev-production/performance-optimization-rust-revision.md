---
source: performance-build-optimization-exploration.md
repository: N/A
revised_at: 2026-03-21T12:00:00Z
workspace: utm-build-optimization
---

# Rust Revision: Performance & Build Optimization

## Overview

This document provides a comprehensive Rust implementation for production-grade build optimization strategies in utm-dev. The translation focuses on:

- **Incremental builds** with content-based hashing and dependency graph tracking
- **Distributed build coordination** using tokio channels for work distribution
- **Build cache** with multi-level architecture (L1 local, L2 shared, L3 remote)
- **Parallel build orchestration** with work stealing and critical path analysis
- **Build profiling** with detailed timing and resource utilization metrics
- **Artifact compression** using zstd, lz4, and other modern algorithms

The implementation uses async-first design with tokio, content-addressable storage with SHA256 hashing, and compression with zstd/lz4 for optimal performance.

## Workspace Structure

```
utm-build-optimization/
├── Cargo.toml                      # Workspace manifest
├── utm-build-core/                 # Core types and traits
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs                # BuildTask, BuildResult, CacheKey
│       ├── error.rs                # BuildError, CacheError
│       └── config.rs               # BuildConfig, CacheConfig
├── utm-build-incremental/          # Incremental build system
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── watcher.rs              # Filesystem watcher with change tracking
│       ├── manifest.rs             # BuildManifest with content hashing
│       └── dependency.rs           # DependencyGraph for affected module detection
├── utm-build-distributed/          # Distributed build coordination
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── coordinator.rs          # BuildCoordinator for task distribution
│       ├── worker.rs               # BuildWorker for executing tasks
│       └── protocol.rs             # Worker protocol messages
├── utm-build-cache/                # Multi-level cache system
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── manager.rs              # CacheManager with L1/L2/L3
│       ├── l1_cache.rs             # Local disk cache with LRU eviction
│       ├── l2_cache.rs             # Redis-backed shared cache
│       ├── l3_cache.rs             # S3-backed remote cache
│       └── analytics.rs            # Cache hit rate analytics
├── utm-build-parallel/             # Parallel build orchestration
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── orchestrator.rs         # ParallelBuildOrchestrator
│       ├── scheduler.rs            # Work-stealing scheduler
│       └── critical_path.rs        # Critical path analysis
├── utm-build-compression/          # Artifact compression
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── compressor.rs           # ArtifactCompressor
│       └── archive.rs              # Tar archive creation
├── utm-build-profiler/             # Build time profiling
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── profiler.rs             # BuildProfiler
│       ├── dashboard.rs            # BuildProfile dashboard
│       └── analyzer.rs             # Build time analyzer
└── utm-build-cli/                  # CLI binary
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── commands/
        │   ├── build.rs
        │   ├── cache.rs
        │   └── profile.rs
        └── args.rs
```

### Crate Breakdown

#### utm-build-core
- **Purpose:** Shared types, traits, and error definitions
- **Type:** library
- **Public API:** `BuildTask`, `BuildResult`, `CacheKey`, `BuildError`, `CacheConfig`
- **Dependencies:** serde, tokio, thiserror

#### utm-build-incremental
- **Purpose:** Incremental build system with dependency tracking
- **Type:** library
- **Public API:** `IncrementalBuildWatcher`, `BuildManifest`, `DependencyGraph`
- **Dependencies:** notify, sha2, serde, tokio

#### utm-build-distributed
- **Purpose:** Distributed build coordination across worker nodes
- **Type:** library
- **Public API:** `BuildCoordinator`, `BuildWorker`, `WorkerProtocol`
- **Dependencies:** tokio, serde, bincode

#### utm-build-cache
- **Purpose:** Multi-level cache system (local, shared, remote)
- **Type:** library
- **Public API:** `CacheManager`, `CacheConfig`, `CacheStats`
- **Dependencies:** tokio, redis, aws-sdk-s3, sha2, serde

#### utm-build-parallel
- **Purpose:** Parallel build orchestration with work stealing
- **Type:** library
- **Public API:** `ParallelBuildOrchestrator`, `WorkScheduler`, `CriticalPath`
- **Dependencies:** tokio, rayon, petgraph

#### utm-build-compression
- **Purpose:** Artifact compression with multiple algorithms
- **Type:** library
- **Public API:** `ArtifactCompressor`, `CompressionAlgorithm`, `CompressionStats`
- **Dependencies:** zstd, lz4, flate2, brotli, tar

#### utm-build-profiler
- **Purpose:** Build time profiling and analytics
- **Type:** library
- **Public API:** `BuildProfiler`, `BuildProfile`, `DependencyGraphProfile`
- **Dependencies:** tokio, sysinfo, serde, serde_json

#### utm-build-cli
- **Purpose:** Command-line interface for build operations
- **Type:** binary
- **Public API:** CLI commands (build, cache, profile)
- **Dependencies:** clap, tokio, tracing

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Async runtime | tokio | 1.0 | Full-featured async runtime with channels |
| Serialization | serde + serde_json | 1.0 | Industry-standard serialization |
| TOML support | toml | 0.8 | For build manifest files |
| Error handling | thiserror | 1.0 | Derive-based error types |
| Filesystem watching | notify | 6.0 | Cross-platform file watching |
| Hashing | sha2 | 0.10 | SHA256 for content addressing |
| Compression (zstd) | zstd | 0.13 | Fast compression with good ratio |
| Compression (lz4) | lz4 | 1.24 | Ultra-fast compression |
| Compression (brotli) | brotli | 3.4 | Maximum compression |
| Archives | tar | 0.4 | Tar archive creation |
| Redis client | redis | 0.24 | For shared cache layer |
| S3 client | aws-sdk-s3 | 1.0 | For remote cache layer |
| System info | sysinfo | 0.30 | CPU/memory monitoring |
| Graph algorithms | petgraph | 0.6 | Dependency graph analysis |
| Parallel iterators | rayon | 1.8 | Data parallelism |
| CLI parsing | clap | 4.0 | Derive-based CLI |
| Logging | tracing | 0.1 | Structured logging |

## Type System Design

### Core Types

```rust
// utm-build-core/src/types.rs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Unique identifier for a build task
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TaskId(pub String);

impl TaskId {
    pub fn new() -> Self {
        Self(format!("task_{}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            rand::random::<u32>()
        ))
    }
}

/// A build task to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTask {
    pub id: TaskId,
    pub module: String,
    pub source_hash: String,
    pub dependencies: Vec<String>,
    pub priority: u8,
    pub command: BuildCommand,
    pub environment: HashMap<String, String>,
}

/// Type of build command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildCommand {
    Compile {
        target: String,
        profile: String,
        features: Vec<String>,
    },
    Link {
        inputs: Vec<PathBuf>,
        output: PathBuf,
    },
    Test {
        test_suite: String,
        filter: Option<String>,
    },
    Custom {
        command: String,
        args: Vec<String>,
    },
}

/// Result of a build task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub task_id: TaskId,
    pub success: bool,
    pub duration: Duration,
    pub output: BuildOutput,
    pub artifacts: Vec<PathBuf>,
    pub cache_key: Option<CacheKey>,
    pub worker_id: Option<String>,
}

/// Build output (stdout/stderr)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// Cache key for content-addressable storage
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    pub module: String,
    pub source_hash: String,
    pub config_hash: String,
    pub target: String,
    pub toolchain_hash: String,
}

impl CacheKey {
    pub fn compute(
        module: &str,
        source_files: &[PathBuf],
        config: &BuildConfig,
        target: &str,
    ) -> Result<Self, CacheError> {
        use sha2::{Sha256, Digest};

        // Hash all source files
        let mut hasher = Sha256::new();
        for file in source_files {
            let content = std::fs::read(file)
                .map_err(|e| CacheError::IoError(file.clone(), e))?;
            hasher.update(&content);
        }
        let source_hash = format!("{:x}", hasher.finalize());

        // Hash configuration
        hasher.reset();
        hasher.update(serde_json::to_string(config)?.as_bytes());
        let config_hash = format!("{:x}", hasher.finalize());

        // Hash toolchain
        let toolchain_hash = compute_toolchain_hash()?;

        Ok(Self {
            module: module.to_string(),
            source_hash,
            config_hash,
            target: target.to_string(),
            toolchain_hash,
        })
    }

    pub fn as_string(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.module, self.source_hash, self.target, self.config_hash
        )
    }
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub incremental: bool,
    pub parallel_jobs: usize,
    pub cache_enabled: bool,
    pub profile: BuildProfile,
    pub targets: Vec<String>,
    pub features: Vec<String>,
    pub rustflags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildProfile {
    Debug,
    Release,
    Custom { optimizations: u32, debug_info: bool },
}
```

### Error Types

```rust
// utm-build-core/src/error.rs

use thiserror::Error;
use std::path::PathBuf;

/// Main error type for build operations
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),

    #[error("Dependency resolution failed: {0}")]
    DependencyError(String),

    #[error("Cache operation failed: {0}")]
    CacheError(#[from] CacheError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Worker communication failed: {0}")]
    WorkerCommunicationError(String),

    #[error("Build timeout after {0:?}")]
    Timeout(Duration),

    #[error("Build cancelled: {0}")]
    Cancelled(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Error type for cache operations
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache miss for key: {0}")]
    Miss(String),

    #[error("IO error for path {0}: {1}")]
    IoError(PathBuf, std::io::Error),

    #[error("Cache corruption detected: {0}")]
    Corruption(String),

    #[error("Cache full, eviction failed: {0}")]
    EvictionFailed(String),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Hash computation failed: {0}")]
    HashError(String),
}

pub type Result<T> = std::result::Result<T, BuildError>;
pub type CacheResult<T> = std::result::Result<T, CacheError>;
```

### Traits

```rust
// utm-build-core/src/traits.rs

use crate::{BuildTask, BuildResult, CacheKey, BuildConfig};
use std::path::PathBuf;

/// Trait for build task executors
#[async_trait::async_trait]
pub trait BuildExecutor: Send + Sync {
    /// Execute a build task
    async fn execute(&self, task: BuildTask) -> crate::Result<BuildResult>;

    /// Cancel a running task
    async fn cancel(&self, task_id: &crate::TaskId) -> crate::Result<()>;

    /// Get executor capabilities
    fn capabilities(&self) -> ExecutorCapabilities;
}

/// Executor capabilities
#[derive(Debug, Clone)]
pub struct ExecutorCapabilities {
    pub supports_incremental: bool,
    pub supports_parallelism: bool,
    pub max_parallel_jobs: usize,
    pub supported_targets: Vec<String>,
    pub cache_support: bool,
}

/// Trait for cache providers
#[async_trait::async_trait]
pub trait CacheProvider: Send + Sync {
    /// Get an entry from cache
    async fn get(&self, key: &CacheKey) -> crate::CacheResult<CacheEntry>;

    /// Put an entry in cache
    async fn put(&self, key: CacheKey, entry: CacheEntry) -> crate::CacheResult<()>;

    /// Check if key exists in cache
    async fn contains(&self, key: &CacheKey) -> crate::CacheResult<bool>;

    /// Remove an entry from cache
    async fn remove(&self, key: &CacheKey) -> crate::CacheResult<()>;

    /// Get cache statistics
    async fn stats(&self) -> CacheStats;
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub artifacts: Vec<PathBuf>,
    pub metadata: CacheMetadata,
    pub size: u64,
    pub created_at: std::time::Instant,
    pub access_count: u64,
}

#[derive(Debug, Clone)]
pub struct CacheMetadata {
    pub build_duration: std::time::Duration,
    pub compiler_version: String,
    pub rustc_hash: String,
    pub features: Vec<String>,
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_size: u64,
    pub item_count: u64,
}

/// Trait for dependency graph analysis
pub trait DependencyAnalyzer: Send + Sync {
    /// Get all dependencies for a module
    fn dependencies(&self, module: &str) -> Vec<String>;

    /// Get all dependents (reverse dependencies)
    fn dependents(&self, module: &str) -> Vec<String>;

    /// Get topological order for building
    fn topological_order(&self) -> Vec<String>;

    /// Get critical path (longest dependency chain)
    fn critical_path(&self) -> Vec<String>;

    /// Get parallelization opportunities
    fn parallel_groups(&self) -> Vec<Vec<String>>;
}
```

## Key Rust-Specific Changes

### 1. Content-Addressable Cache with SHA256

**Source Pattern:** File modification time-based caching

**Rust Translation:** Content-based hashing with SHA256 for reliable cache keys

**Rationale:** Modification times can be unreliable across systems; content hashing ensures cache correctness and enables sharing across machines.

```rust
// utm-build-cache/src/key.rs

use sha2::{Sha256, Digest};

pub fn compute_content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

pub fn compute_file_hash(path: &Path) -> std::io::Result<String> {
    let content = std::fs::read(path)?;
    Ok(compute_content_hash(&content))
}
```

### 2. Async-First Design with Tokio Channels

**Source Pattern:** Synchronous build coordination

**Rust Translation:** Async channels for task distribution and result collection

**Rationale:** Enables high-concurrency build orchestration without blocking threads, essential for distributed builds.

```rust
// utm-build-distributed/src/coordinator.rs

use tokio::sync::{mpsc, broadcast};

pub struct BuildCoordinator {
    task_queue: mpsc::Sender<BuildTask>,
    result_rx: mpsc::Receiver<BuildResult>,
    worker_tx: broadcast::Sender<WorkerMessage>,
    workers: HashMap<String, WorkerInfo>,
}
```

### 3. Multi-Level Cache with Automatic Promotion

**Source Pattern:** Single-level build cache

**Rust Translation:** L1 (local disk), L2 (Redis), L3 (S3) with automatic promotion on hit

**Rationale:** Optimizes for common case (recent builds) while providing fallback for cold caches.

### 4. Work-Stealing Parallel Scheduler

**Source Pattern:** Fixed worker assignment

**Rust Translation:** Work-stealing scheduler using rayon for dynamic load balancing

**Rationale:** Handles variable build times across modules by allowing idle workers to steal from busy ones.

## Ownership & Borrowing Strategy

The design follows these ownership patterns:

1. **BuildTask is moved** when submitted to the coordinator, then moved to worker
2. **BuildResult is returned** by value, transferring ownership to the coordinator
3. **Cache entries use Arc** for shared access across async tasks
4. **Configuration uses Clone** for passing to workers
5. **Mutable state uses tokio::sync::RwLock** for async-safe access

```rust
// Example ownership flow

pub async fn submit_build(
    &self,
    task: BuildTask,  // Takes ownership
) -> Result<TaskId> {
    let task_id = task.id.clone();
    self.task_queue.send(task).await?;  // Task moved into channel
    Ok(task_id)
}

pub async fn get_cache_stats(&self) -> CacheStats {
    // Returns by value (Clone)
    self.stats.read().await.clone()
}
```

## Concurrency Model

**Approach:** Async with tokio runtime + parallel iterators with rayon

**Rationale:**
- Async for I/O-bound operations (cache, network, filesystem)
- Rayon for CPU-bound operations (hashing, compression)
- Channels for communication between coordinator and workers

```rust
// Concurrent build execution example

use tokio::task::JoinSet;
use rayon::prelude::*;

pub async fn execute_parallel(
    &self,
    tasks: Vec<BuildTask>,
) -> Result<Vec<BuildResult>> {
    let mut join_set = JoinSet::new();

    // Spawn tasks concurrently
    for task in tasks {
        let executor = self.executor.clone();
        join_set.spawn(async move {
            executor.execute(task).await
        });
    }

    // Collect results
    let mut results = Vec::new();
    while let Some(result) = join_set.join_next().await {
        results.push(result??);
    }

    Ok(results)
}

// Parallel hashing with rayon
pub fn hash_files_parallel(files: &[PathBuf]) -> Vec<String> {
    files.par_iter()
        .map(|path| {
            let content = std::fs::read(path).unwrap();
            compute_content_hash(&content)
        })
        .collect()
}
```

## Memory Considerations

1. **Large artifacts stored on disk** - Only metadata in memory
2. **Build logs streamed** - Not accumulated in memory
3. **Cache uses LRU eviction** - Bounded memory usage
4. **Arc for shared state** - Coordinator state shared across tasks
5. **Zero-copy hashing where possible** - Using memmap for large files

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Concurrent cache writes | tokio::sync::RwLock for async-safe access |
| Worker disconnection | Timeout with automatic retry |
| Cache corruption | Checksum verification on read |
| Out of disk space | Pre-flight check, graceful degradation |
| Circular dependencies | Cycle detection in dependency graph |
| Partial build artifacts | Atomic writes with temp files |
| Signal handling (SIGINT) | tokio::signal for graceful shutdown |

## Code Examples

### Example: Incremental Build Watcher

```rust
// utm-build-incremental/src/watcher.rs

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

/// Watches filesystem for changes and tracks dirty modules
pub struct IncrementalBuildWatcher {
    watcher: RecommendedWatcher,
    state: Arc<WatcherState>,
    dependency_graph: Arc<RwLock<DependencyGraph>>,
}

struct WatcherState {
    file_hashes: Mutex<HashMap<PathBuf, String>>,
    dirty_paths: Mutex<HashSet<PathBuf>>,
    root_path: PathBuf,
}

impl IncrementalBuildWatcher {
    /// Create a new incremental build watcher
    pub fn new(root_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let state = Arc::new(WatcherState {
            file_hashes: Mutex::new(HashMap::new()),
            dirty_paths: Mutex::new(HashSet::new()),
            root_path: root_path.to_path_buf(),
        });

        let state_clone = Arc::clone(&state);

        let watcher = RecommendedWatcher::new(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                for path in event.paths {
                    Self::handle_change(&state_clone, &path);
                }
            }
        })?;

        let mut watcher = watcher;
        watcher.watch(root_path, RecursiveMode::Recursive)?;

        Ok(Self {
            watcher,
            state,
            dependency_graph: Arc::new(RwLock::new(DependencyGraph::new())),
        })
    }

    fn handle_change(state: &WatcherState, path: &Path) {
        if let Ok(content) = std::fs::read(path) {
            let hash = compute_content_hash(&content);

            let mut hashes = state.file_hashes.lock().unwrap();
            let mut dirty = state.dirty_paths.lock().unwrap();

            hashes.insert(path.to_path_buf(), hash);
            dirty.insert(path.to_path_buf());
        }
    }

    /// Get modules affected by changes
    pub async fn get_affected_modules(&self) -> HashSet<String> {
        let dirty = self.state.dirty_paths.lock().unwrap();
        let dep_graph = self.dependency_graph.read().await;

        let mut affected = HashSet::new();

        for path in dirty.iter() {
            // Find module for this path
            if let Some(module) = dep_graph.path_to_module(path) {
                affected.insert(module.clone());

                // Add all dependents
                for dependent in dep_graph.dependents(&module) {
                    affected.insert(dependent.clone());
                }
            }
        }

        affected
    }

    /// Mark paths as clean after rebuild
    pub fn mark_clean(&self, paths: &[PathBuf]) {
        let mut dirty = self.state.dirty_paths.lock().unwrap();
        for path in paths {
            dirty.remove(path);
        }
    }

    /// Check if any changes are pending
    pub fn has_changes(&self) -> bool {
        !self.state.dirty_paths.lock().unwrap().is_empty()
    }
}
```

### Example: Build Manifest with Content Hashing

```rust
// utm-build-incremental/src/manifest.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Build manifest tracking module hashes and dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildManifest {
    pub version: String,
    pub incremental: bool,
    pub cache_dir: PathBuf,
    pub modules: HashMap<String, ModuleInfo>,
    pub last_build_time: u64,
}

/// Information about a build module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub path: PathBuf,
    pub hash: String,
    pub dependencies: Vec<String>,
    pub artifacts: Vec<String>,
    pub last_built: Option<u64>,
    pub build_duration_ms: Option<u64>,
}

impl BuildManifest {
    /// Load manifest from file
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get modules that need rebuilding based on changes
    pub fn get_rebuild_order(&self, changed_modules: &[String]) -> Vec<String> {
        let mut rebuild_order = Vec::new();
        let mut visited = HashSet::new();

        for module in changed_modules {
            self.visit_module(module, &mut rebuild_order, &mut visited);
        }

        rebuild_order
    }

    fn visit_module(
        &self,
        module: &str,
        order: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(module) {
            return;
        }

        if let Some(info) = self.modules.get(module) {
            // Visit dependencies first (topological sort)
            for dep in &info.dependencies {
                self.visit_module(dep, order, visited);
            }
            order.push(module.to_string());
            visited.insert(module.to_string());
        }
    }

    /// Check if a module needs rebuilding
    pub fn needs_rebuild(&self, module: &str, current_hash: &str) -> bool {
        match self.modules.get(module) {
            Some(info) => info.hash != current_hash,
            None => true, // Never built before
        }
    }

    /// Update module after successful build
    pub fn update_module(&mut self, module: &str, hash: String, artifacts: Vec<String>) {
        if let Some(info) = self.modules.get_mut(module) {
            info.hash = hash;
            info.artifacts = artifacts;
            info.last_built = Some(current_timestamp());
        } else {
            self.modules.insert(module.to_string(), ModuleInfo {
                path: PathBuf::new(),
                hash,
                dependencies: Vec::new(),
                artifacts,
                last_built: Some(current_timestamp()),
                build_duration_ms: None,
            });
        }
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
```

### Example: Dependency Graph Analysis

```rust
// utm-build-incremental/src/dependency.rs

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Dfs;

/// Dependency graph for build modules
pub struct DependencyGraph {
    graph: DiGraph<String, ()>,
    module_to_node: HashMap<String, NodeIndex>,
    path_to_module: HashMap<PathBuf, String>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            module_to_node: HashMap::new(),
            path_to_module: HashMap::new(),
        }
    }

    /// Add a module to the graph
    pub fn add_module(&mut self, name: &str, path: PathBuf) -> NodeIndex {
        let node = self.graph.add_node(name.to_string());
        self.module_to_node.insert(name.to_string(), node);
        self.path_to_module.insert(path, name.to_string());
        node
    }

    /// Add a dependency edge
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        if let (Some(from_node), Some(to_node)) = (
            self.module_to_node.get(from),
            self.module_to_node.get(to),
        ) {
            self.graph.add_edge(*from_node, *to_node, ());
        }
    }

    /// Get all dependencies (modules this module depends on)
    pub fn dependencies(&self, module: &str) -> Vec<String> {
        let Some(node) = self.module_to_node.get(module) else {
            return Vec::new();
        };

        self.graph
            .neighbors(*node)
            .map(|n| self.graph[n].clone())
            .collect()
    }

    /// Get all dependents (modules that depend on this module)
    pub fn dependents(&self, module: &str) -> Vec<String> {
        let Some(node) = self.module_to_node.get(module) else {
            return Vec::new();
        };

        self.graph
            .neighbors_directed(*node, petgraph::Direction::Incoming)
            .map(|n| self.graph[n].clone())
            .collect()
    }

    /// Get topological order for building
    pub fn topological_order(&self) -> Vec<String> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();

        for node in self.graph.node_indices() {
            self.topo_visit(node, &mut order, &mut visited);
        }

        order
    }

    fn topo_visit(
        &self,
        node: NodeIndex,
        order: &mut Vec<String>,
        visited: &mut HashSet<NodeIndex>,
    ) {
        if !visited.insert(node) {
            return;
        }

        // Visit dependencies first
        for dep in self.graph.neighbors(node) {
            self.topo_visit(dep, order, visited);
        }

        order.push(self.graph[node].clone());
    }

    /// Detect cycles in the dependency graph
    pub fn has_cycle(&self) -> bool {
        use petgraph::algo::toposort;
        toposort(&self.graph, None).is_err()
    }

    /// Get critical path (longest chain)
    pub fn critical_path(&self) -> Vec<String> {
        // Simplified: find longest path using DFS
        let mut longest = Vec::new();

        for start in self.graph.node_indices() {
            let path = self.dfs_longest_path(start);
            if path.len() > longest.len() {
                longest = path;
            }
        }

        longest
    }

    fn dfs_longest_path(&self, start: NodeIndex) -> Vec<String> {
        let mut stack = vec![(start, vec![self.graph[start].clone()])];
        let mut longest = Vec::new();

        while let Some((node, path)) = stack.pop() {
            let neighbors: Vec<_> = self.graph.neighbors(node).collect();

            if neighbors.is_empty() {
                if path.len() > longest.len() {
                    longest = path;
                }
            } else {
                for neighbor in neighbors {
                    let mut new_path = path.clone();
                    new_path.push(self.graph[neighbor].clone());
                    stack.push((neighbor, new_path));
                }
            }
        }

        longest
    }

    /// Map a file path to a module name
    pub fn path_to_module(&self, path: &Path) -> Option<&String> {
        self.path_to_module.get(path)
    }

    /// Get parallel groups (modules that can build together)
    pub fn parallel_groups(&self) -> Vec<Vec<String>> {
        // Group by depth in dependency graph
        let mut groups: HashMap<usize, Vec<String>> = HashMap::new();

        for (node, name) in self.graph.node_indices().map(|n| (n, &self.graph[n])) {
            let depth = self.compute_depth(node);
            groups.entry(depth).or_default().push(name.clone());
        }

        groups.into_values().collect()
    }

    fn compute_depth(&self, node: NodeIndex) -> usize {
        let mut max_depth = 0;
        for dep in self.graph.neighbors(node) {
            max_depth = max_depth.max(self.compute_depth(dep) + 1);
        }
        max_depth
    }
}
```

### Example: Multi-Level Cache Manager

```rust
// utm-build-cache/src/manager.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{CacheKey, CacheEntry, CacheProvider, CacheStats, CacheResult};

/// Multi-level cache manager with L1 (local), L2 (shared), L3 (remote)
pub struct CacheManager {
    config: CacheConfig,
    l1_cache: Arc<LocalCache>,
    l2_cache: Option<Arc<SharedCache>>,
    l3_cache: Option<Arc<RemoteCache>>,
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub l1_max_size_gb: u64,
    pub l1_eviction_policy: EvictionPolicy,
    pub l2_endpoint: Option<String>,
    pub l3_bucket: Option<String>,
    pub compression: CompressionAlgorithm,
}

#[derive(Debug, Clone)]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    TimeBased(std::time::Duration),
}

#[derive(Debug, Clone)]
pub enum CompressionAlgorithm {
    None,
    Zstd { level: i32 },
    Lz4,
}

impl CacheManager {
    pub async fn new(config: CacheConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let l1_cache = Arc::new(LocalCache::new(&config).await?);

        let l2_cache = if let Some(endpoint) = &config.l2_endpoint {
            Some(Arc::new(SharedCache::new(endpoint).await?))
        } else {
            None
        };

        let l3_cache = if let Some(bucket) = &config.l3_bucket {
            Some(Arc::new(RemoteCache::new(bucket).await?))
        } else {
            None
        };

        Ok(Self {
            config,
            l1_cache,
            l2_cache,
            l3_cache,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        })
    }

    /// Get entry from cache with automatic promotion
    pub async fn get(&self, key: &CacheKey) -> CacheResult<CacheEntry> {
        // Try L1 first (fastest)
        if let Ok(entry) = self.l1_cache.get(key).await {
            self.stats.write().await.hits += 1;
            return Ok(entry);
        }

        // Try L2 (shared Redis)
        if let Some(l2) = &self.l2_cache {
            if let Ok(entry) = l2.get(key).await {
                self.stats.write().await.hits += 1;
                // Promote to L1
                let _ = self.l1_cache.put(key.clone(), entry.clone()).await;
                return Ok(entry);
            }
        }

        // Try L3 (remote S3)
        if let Some(l3) = &self.l3_cache {
            if let Ok(entry) = l3.get(key).await {
                self.stats.write().await.hits += 1;
                // Promote to L1 and L2
                let _ = self.l1_cache.put(key.clone(), entry.clone()).await;
                if let Some(l2) = &self.l2_cache {
                    let _ = l2.put(key.clone(), entry.clone()).await;
                }
                return Ok(entry);
            }
        }

        self.stats.write().await.misses += 1;
        Err(CacheError::Miss(key.as_string()))
    }

    /// Put entry in all cache levels
    pub async fn put(&self, key: CacheKey, entry: CacheEntry) -> CacheResult<()> {
        // Store in all available levels
        self.l1_cache.put(key.clone(), entry.clone()).await?;

        if let Some(l2) = &self.l2_cache {
            l2.put(key.clone(), entry.clone()).await?;
        }

        if let Some(l3) = &self.l3_cache {
            l3.put(key.clone(), entry).await?;
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Clear all caches
    pub async fn clear(&self) -> CacheResult<()> {
        self.l1_cache.clear().await?;

        if let Some(l2) = &self.l2_cache {
            l2.clear().await?;
        }

        if let Some(l3) = &self.l3_cache {
            l3.clear().await?;
        }

        Ok(())
    }
}
```

### Example: Local Cache with LRU Eviction

```rust
// utm-build-cache/src/l1_cache.rs

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use crate::{CacheKey, CacheEntry, CacheResult, CacheConfig};

/// Local disk cache with LRU eviction
pub struct LocalCache {
    cache_dir: PathBuf,
    index: RwLock<HashMap<CacheKey, CacheEntry>>,
    access_queue: RwLock<VecDeque<CacheKey>>,
    max_size_bytes: u64,
    current_size: RwLock<u64>,
}

impl LocalCache {
    pub async fn new(config: &CacheConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let cache_dir = PathBuf::from(".build-cache/l1");
        tokio::fs::create_dir_all(&cache_dir).await?;

        Ok(Self {
            cache_dir,
            index: RwLock::new(HashMap::new()),
            access_queue: RwLock::new(VecDeque::new()),
            max_size_bytes: config.l1_max_size_gb * 1024 * 1024 * 1024,
            current_size: RwLock::new(0),
        })
    }

    pub async fn get(&self, key: &CacheKey) -> CacheResult<CacheEntry> {
        let mut index = self.index.write().await;

        let entry = index.get_mut(key)
            .ok_or_else(|| crate::CacheError::Miss(key.as_string()))?;

        // Update access order for LRU
        entry.access_count += 1;

        // Move to end of access queue
        let mut queue = self.access_queue.write().await;
        if let Some(pos) = queue.iter().position(|k| k == key) {
            queue.remove(pos);
            queue.push_back(key.clone());
        }

        Ok(entry.clone())
    }

    pub async fn put(&self, key: CacheKey, entry: CacheEntry) -> CacheResult<()> {
        // Evict if necessary
        self.evict_if_needed(entry.size).await?;

        // Write to disk
        let entry_path = self.get_entry_path(&key);
        let serialized = serde_json::to_vec(&entry)?;
        tokio::fs::write(&entry_path, &serialized).await?;

        // Update index
        let mut index = self.index.write().await;
        let mut queue = self.access_queue.write().await;

        index.insert(key.clone(), entry.clone());
        queue.push_back(key);

        // Update size
        *self.current_size.write().await += entry.size;

        Ok(())
    }

    async fn evict_if_needed(&self, needed_size: u64) -> CacheResult<()> {
        let current = *self.current_size.read().await;

        if current + needed_size <= self.max_size_bytes {
            return Ok(());
        }

        let mut index = self.index.write().await;
        let mut queue = self.access_queue.write().await;
        let mut current_size = self.current_size.write().await;

        // Evict LRU entries until we have space
        while *current_size + needed_size > self.max_size_bytes {
            if let Some(evict_key) = queue.pop_front() {
                if let Some(evicted) = index.remove(&evict_key) {
                    *current_size -= evicted.size;
                    self.delete_entry(&evict_key).await;
                }
            } else {
                break; // Nothing left to evict
            }
        }

        Ok(())
    }

    fn get_entry_path(&self, key: &CacheKey) -> PathBuf {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(key.as_string().as_bytes());
        self.cache_dir.join(format!("{:x}", hash))
    }

    async fn delete_entry(&self, key: &CacheKey) {
        let path = self.get_entry_path(key);
        let _ = tokio::fs::remove_file(path).await;
    }

    pub async fn clear(&self) -> CacheResult<()> {
        let mut index = self.index.write().await;
        let mut queue = self.access_queue.write().await;
        let mut current_size = self.current_size.write().await;

        index.clear();
        queue.clear();
        *current_size = 0;

        // Delete all files in cache directory
        if self.cache_dir.exists() {
            let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let _ = tokio::fs::remove_file(entry.path()).await;
            }
        }

        Ok(())
    }
}
```

### Example: Distributed Build Coordinator

```rust
// utm-build-distributed/src/coordinator.rs

use tokio::sync::{mpsc, broadcast, RwLock};
use std::collections::HashMap;
use std::net::SocketAddr;
use crate::{BuildTask, BuildResult, TaskId, WorkerInfo};

/// Coordinates distributed build tasks across workers
pub struct BuildCoordinator {
    task_queue: mpsc::Sender<BuildTask>,
    result_rx: mpsc::Receiver<BuildResult>,
    workers: Arc<RwLock<HashMap<String, WorkerInfo>>>,
    pending_tasks: Arc<RwLock<HashMap<TaskId, BuildTask>>>,
    completed_tasks: Arc<RwLock<HashMap<TaskId, BuildResult>>>,
    shutdown_tx: broadcast::Sender<()>,
}

#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub id: String,
    pub addr: SocketAddr,
    pub capacity: u32,
    pub current_load: u32,
    pub last_heartbeat: std::time::Instant,
}

impl BuildCoordinator {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (task_tx, mut task_rx) = mpsc::channel::<BuildTask>(1000);
        let (result_tx, result_rx) = mpsc::channel::<BuildResult>(1000);
        let (shutdown_tx, _) = broadcast::channel::<()>(10);

        let workers = Arc::new(RwLock::new(HashMap::new()));
        let pending_tasks = Arc::new(RwLock::new(HashMap::new()));
        let completed_tasks = Arc::new(RwLock::new(HashMap::new()));

        // Spawn task dispatcher
        let workers_clone = Arc::clone(&workers);
        let pending_clone = Arc::clone(&pending_tasks);
        let result_tx_clone = result_tx.clone();

        tokio::spawn(async move {
            while let Some(task) = task_rx.recv().await {
                // Find least-loaded worker
                let workers = workers_clone.read().await;
                if let Some((worker_id, worker)) = workers.iter()
                    .min_by_key(|(_, w)| w.current_load)
                {
                    // Dispatch to worker
                    pending_clone.write().await.insert(task.id.clone(), task);
                    // In real implementation, send via network
                }
            }
        });

        Ok(Self {
            task_queue: task_tx,
            result_rx,
            workers,
            pending_tasks,
            completed_tasks,
            shutdown_tx,
        })
    }

    /// Submit a build task
    pub async fn submit(&self, task: BuildTask) -> Result<TaskId, Box<dyn std::error::Error>> {
        let task_id = task.id.clone();
        self.task_queue.send(task).await?;
        Ok(task_id)
    }

    /// Wait for task completion
    pub async fn wait_for(&self, task_id: &TaskId) -> Option<BuildResult> {
        // Check completed
        {
            let completed = self.completed_tasks.read().await;
            if let Some(result) = completed.get(task_id) {
                return Some(result.clone());
            }
        }

        // Wait for result
        let mut result_rx = self.result_rx.resubscribe();
        while let Some(result) = result_rx.recv().await {
            if &result.task_id == task_id {
                self.completed_tasks.write().await.insert(task_id.clone(), result.clone());
                return Some(result);
            }
        }

        None
    }

    /// Register a worker
    pub async fn register_worker(&self, info: WorkerInfo) {
        self.workers.write().await.insert(info.id.clone(), info);
    }

    /// Handle task completion from worker
    pub async fn task_completed(&self, result: BuildResult) {
        self.pending_tasks.write().await.remove(&result.task_id);
        self.completed_tasks.write().await.insert(result.task_id.clone(), result.clone());

        // Update worker load
        if let Some(worker_id) = &result.worker_id {
            let mut workers = self.workers.write().await;
            if let Some(worker) = workers.get_mut(worker_id) {
                worker.current_load = worker.current_load.saturating_sub(1);
            }
        }
    }

    /// Get queue statistics
    pub async fn stats(&self) -> CoordinatorStats {
        let workers = self.workers.read().await;
        let pending = self.pending_tasks.read().await;
        let completed = self.completed_tasks.read().await;

        let total_capacity: u32 = workers.values().map(|w| w.capacity).sum();
        let total_load: u32 = workers.values().map(|w| w.current_load).sum();

        CoordinatorStats {
            worker_count: workers.len(),
            pending_tasks: pending.len(),
            completed_tasks: completed.len(),
            total_capacity,
            current_load: total_load,
            utilization: total_load as f64 / total_capacity as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CoordinatorStats {
    pub worker_count: usize,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
    pub total_capacity: u32,
    pub current_load: u32,
    pub utilization: f64,
}
```

### Example: Build Profiler with Dashboard

```rust
// utm-build-profiler/src/profiler.rs

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Profiles build times and generates reports
pub struct BuildProfiler {
    phases: Vec<PhaseProfile>,
    current_phase: Option<(String, Instant)>,
    crate_times: HashMap<String, Duration>,
    start_time: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseProfile {
    pub name: String,
    pub duration: Duration,
    pub start_time: Instant,
    pub sub_phases: Vec<PhaseProfile>,
}

impl BuildProfiler {
    pub fn new() -> Self {
        Self {
            phases: Vec::new(),
            current_phase: None,
            crate_times: HashMap::new(),
            start_time: Instant::now(),
        }
    }

    /// Start profiling a phase
    pub fn start_phase(&mut self, name: &str) {
        self.current_phase = Some((name.to_string(), Instant::now()));
    }

    /// End current phase
    pub fn end_phase(&mut self) {
        if let Some((name, start)) = self.current_phase.take() {
            self.phases.push(PhaseProfile {
                name,
                duration: start.elapsed(),
                start_time: start,
                sub_phases: Vec::new(),
            });
        }
    }

    /// Record compilation time for a crate
    pub fn record_crate_time(&mut self, crate_name: &str, duration: Duration) {
        self.crate_times.insert(crate_name.to_string(), duration);
    }

    /// Generate profiling report
    pub fn generate_report(&self) -> BuildProfile {
        let total_duration = self.start_time.elapsed();

        // Find critical path
        let critical_path = self.find_critical_path();

        // Calculate parallel efficiency
        let sequential_time: f64 = self.crate_times.values()
            .map(|d| d.as_secs_f64())
            .sum();
        let parallel_time = total_duration.as_secs_f64();
        let parallel_efficiency = if parallel_time > 0.0 {
            sequential_time / parallel_time
        } else {
            1.0
        };

        BuildProfile {
            total_duration,
            phases: self.phases.clone(),
            crate_times: self.crate_times.clone(),
            critical_path,
            parallel_efficiency,
        }
    }

    fn find_critical_path(&self) -> Vec<String> {
        // Find longest phase chain
        self.phases.iter()
            .map(|p| p.name.clone())
            .collect()
    }

    /// Save report to JSON file
    pub fn save_report(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let report = self.generate_report();
        let json = serde_json::to_string_pretty(&report)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildProfile {
    pub total_duration: Duration,
    pub phases: Vec<PhaseProfile>,
    pub crate_times: HashMap<String, Duration>,
    pub critical_path: Vec<String>,
    pub parallel_efficiency: f64,
}

impl BuildProfile {
    /// Get slowest crates
    pub fn slowest_crates(&self, n: usize) -> Vec<(&String, &Duration)> {
        let mut sorted: Vec<_> = self.crate_times.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        sorted.into_iter().take(n).collect()
    }

    /// Get recommendations for improvement
    pub fn recommendations(&self) -> Vec<String> {
        let mut recs = Vec::new();

        if self.parallel_efficiency < 0.5 {
            recs.push("Low parallel efficiency - consider increasing parallel jobs".to_string());
        }

        if let Some((crate_name, duration)) = self.slowest_crates(1).first() {
            if duration.as_secs() > 30 {
                recs.push(format!(
                    "Crate '{}' takes {:?} - consider splitting or optimizing",
                    crate_name, duration
                ));
            }
        }

        recs
    }
}
```

### Example: Artifact Compressor

```rust
// utm-build-compression/src/compressor.rs

use std::io::{Read, Write};
use std::path::Path;

/// Compression algorithm selection
#[derive(Debug, Clone)]
pub enum CompressionAlgorithm {
    Zstd { level: i32 },
    Lz4,
    Gzip { level: u32 },
    Brotli { quality: u32 },
}

/// Statistics from compression operation
#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_size: usize,
    pub compressed_size: usize,
    pub ratio: f64,
    pub duration: std::time::Duration,
}

/// Artifact compressor with multiple algorithm support
pub struct ArtifactCompressor {
    algorithm: CompressionAlgorithm,
    chunk_size: usize,
}

impl ArtifactCompressor {
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self {
            algorithm,
            chunk_size: 64 * 1024, // 64KB chunks
        }
    }

    /// Compress a file
    pub fn compress_file(
        &self,
        input: &Path,
        output: &Path,
    ) -> Result<CompressionStats, Box<dyn std::error::Error>> {
        let input_data = std::fs::read(input)?;
        let original_size = input_data.len();

        let start = std::time::Instant::now();

        let compressed = match &self.algorithm {
            CompressionAlgorithm::Zstd { level } => {
                zstd::encode_all(input_data.as_slice(), *level)?
            }
            CompressionAlgorithm::Lz4 => {
                let mut encoder = lz4::EncoderBuilder::new()
                    .level(4)
                    .build(Vec::new())?;
                encoder.write_all(&input_data)?;
                encoder.finish().0
            }
            CompressionAlgorithm::Gzip { level } => {
                use flate2::write::GzEncoder;
                use flate2::Compression;
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(*level));
                encoder.write_all(&input_data)?;
                encoder.finish()?
            }
            CompressionAlgorithm::Brotli { quality } => {
                use brotli::CompressorReader;
                let mut reader = CompressorReader::new(&input_data[..], 0, *quality, 0);
                let mut compressed = Vec::new();
                reader.read_to_end(&mut compressed)?;
                compressed
            }
        };

        let duration = start.elapsed();

        std::fs::write(output, &compressed)?;

        Ok(CompressionStats {
            original_size,
            compressed_size: compressed.len(),
            ratio: original_size as f64 / compressed.len() as f64,
            duration,
        })
    }

    /// Decompress a file
    pub fn decompress_file(
        &self,
        input: &Path,
        output: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let compressed = std::fs::read(input)?;

        let decompressed = match &self.algorithm {
            CompressionAlgorithm::Zstd { .. } => {
                zstd::decode_all(compressed.as_slice())?
            }
            CompressionAlgorithm::Lz4 => {
                lz4::decode_block(&compressed)
                    .ok_or("LZ4 decompression failed")?
            }
            CompressionAlgorithm::Gzip { .. } => {
                use flate2::read::GzDecoder;
                let mut decoder = GzDecoder::new(&compressed[..]);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                decompressed
            }
            CompressionAlgorithm::Brotli { .. } => {
                use brotli::Decompressor;
                let mut decoder = Decompressor::new(&compressed[..], 0);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)?;
                decompressed
            }
        };

        std::fs::write(output, decompressed)?;
        Ok(())
    }
}

/// Create compressed tar archive
pub fn create_compressed_archive(
    source_dir: &Path,
    output_path: &Path,
    algorithm: CompressionAlgorithm,
) -> Result<CompressionStats, Box<dyn std::error::Error>> {
    use tar::Builder;

    // Create tar archive in memory
    let mut tar_data = Vec::new();
    {
        let mut builder = Builder::new(&mut tar_data);
        builder.append_dir_all(".", source_dir)?;
        builder.finish()?;
    }

    // Compress the tar
    let compressor = ArtifactCompressor::new(algorithm);
    let temp_tar = tempfile::NamedTempFile::new()?;
    std::fs::write(temp_tar.path(), &tar_data)?;

    let stats = compressor.compress_file(temp_tar.path(), output_path)?;

    Ok(stats)
}
```

## Migration Path

1. **Week 1-2: Core Infrastructure**
   - Set up workspace structure
   - Implement core types and error handling
   - Build basic incremental watcher

2. **Week 3-4: Cache System**
   - Implement L1 local cache with LRU eviction
   - Add L2 Redis cache for shared builds
   - Integrate with existing build system

3. **Week 5-6: Distributed Builds**
   - Build coordinator with task distribution
   - Worker implementation
   - Test with single machine first

4. **Week 7-8: Parallel Orchestration**
   - Implement work-stealing scheduler
   - Add critical path analysis
   - Integrate profiling

5. **Week 9-10: Compression & Optimization**
   - Add compression support
   - Tune cache eviction policies
   - Performance benchmarking

## Performance Considerations

1. **Hash computation** - Use parallel hashing for large codebases
2. **Cache lookup** - L1 cache should be memory-mapped for speed
3. **Worker communication** - Use binary protocol (bincode) for efficiency
4. **Compression** - Zstd level 3 for good speed/ratio tradeoff
5. **Parallelism** - Match job count to CPU cores, leave headroom

## Testing Strategy

1. **Unit tests** for individual components (hash computation, cache eviction)
2. **Integration tests** for cache hit/miss scenarios
3. **End-to-end tests** for full build pipeline
4. **Performance tests** with benchmarking suites
5. **Stress tests** for worker disconnection handling

## Open Considerations

1. **Worker discovery** - Should workers register or be discovered?
2. **Cache invalidation** - Strategy for toolchain updates
3. **Security** - Worker authentication in distributed setting
4. **Persistence** - How long to retain cache entries
5. **Cost optimization** - Spot instances for workers
