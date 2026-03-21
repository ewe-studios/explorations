---
source: plugin-system-exploration.md
repository: N/A
revised_at: 2026-03-21T12:00:00Z
workspace: utm-plugin-system
---

# Rust Revision: Plugin System

## Overview

This document provides a comprehensive Rust implementation for a production-grade plugin system in utm-dev. The translation focuses on:

- **Plugin trait definitions** with capability-based security
- **WASM plugin loader** using wasmtime for sandboxed execution
- **Plugin registry and discovery** with manifest-based configuration
- **Hook system** for build lifecycle integration
- **Plugin marketplace** infrastructure for distribution
- **Native plugin loading** via dynamic libraries (libloading)

The implementation uses async-first design with tokio, capability-based access control for security, and supports multiple plugin formats (WASM, native, script-based).

## Workspace Structure

```
utm-plugin-system/
├── Cargo.toml                      # Workspace manifest
├── utm-plugin-core/                # Core types and traits
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── types.rs                # PluginId, PluginManifest, PluginCapability
│       ├── error.rs                # PluginError types
│       └── capability.rs           # Capability definitions
├── utm-plugin-manager/             # Plugin lifecycle management
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── manager.rs              # PluginManager
│       ├── registry.rs             # PluginRegistry
│       └── hooks.rs                # HookRegistry
├── utm-plugin-wasm/                # WASM plugin runtime
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── loader.rs               # WasmPluginLoader
│       ├── runtime.rs              # WasmRuntime wrapper
│       └── imports.rs              # Host function implementations
├── utm-plugin-native/              # Native plugin loading
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── loader.rs               # NativePluginLoader
│       └── dylib.rs                # Dynamic library handling
├── utm-plugin-hooks/               # Hook system
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── build_hooks.rs          # Build hook implementations
│       ├── post_build.rs           # Post-build processors
│       └── asset_pipeline.rs       # Asset pipeline plugins
├── utm-plugin-sdk/                 # SDK for plugin developers
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── macros.rs               # register_plugin! macro
│       └── api.rs                  # Plugin API wrappers
├── utm-plugin-marketplace/         # Marketplace client
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs               # MarketplaceClient
│       ├── registry.rs             # PluginRegistry API
│       └── download.rs             # Plugin download/install
└── utm-plugin-cli/                 # CLI binary
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── commands/
        │   ├── list.rs
        │   ├── install.rs
        │   ├── update.rs
        │   └── publish.rs
        └── args.rs
```

### Crate Breakdown

#### utm-plugin-core
- **Purpose:** Shared types, traits, and capability definitions
- **Type:** library
- **Public API:** `PluginId`, `PluginManifest`, `PluginCapability`, `PluginError`, `PluginCapability`
- **Dependencies:** serde, thiserror, semver

#### utm-plugin-manager
- **Purpose:** Plugin lifecycle and registry management
- **Type:** library
- **Public API:** `PluginManager`, `PluginRegistry`, `HookRegistry`
- **Dependencies:** tokio, serde, camino

#### utm-plugin-wasm
- **Purpose:** WASM plugin runtime and loader
- **Type:** library
- **Public API:** `WasmPluginLoader`, `WasmRuntime`, `PluginContext`
- **Dependencies:** wasmtime, tokio, anyhow

#### utm-plugin-native
- **Purpose:** Native dynamic library plugin loading
- **Type:** library
- **Public API:** `NativePluginLoader`, `DylibHandle`
- **Dependencies:** libloading, libloading-sym

#### utm-plugin-hooks
- **Purpose:** Hook system for build lifecycle
- **Type:** library
- **Public API:** `BuildHook`, `PostBuildProcessor`, `AssetPlugin`, `HookRegistry`
- **Dependencies:** tokio, async-trait

#### utm-plugin-sdk
- **Purpose:** SDK for plugin developers
- **Type:** library (proc-macro)
- **Public API:** `register_plugin!` macro, `Plugin` trait
- **Dependencies:** proc-macro2, syn, quote

#### utm-plugin-marketplace
- **Purpose:** Plugin marketplace client
- **Type:** library
- **Public API:** `MarketplaceClient`, `PluginSearch`, `PluginDownload`
- **Dependencies:** reqwest, serde, tokio

#### utm-plugin-cli
- **Purpose:** Command-line interface for plugin management
- **Type:** binary
- **Public API:** CLI commands (list, install, update, publish)
- **Dependencies:** clap, tokio, tracing

## Recommended Dependencies

| Purpose | Crate | Version | Rationale |
|---------|-------|---------|-----------|
| Async runtime | tokio | 1.0 | Full-featured async runtime |
| WASM runtime | wasmtime | 15.0 | Fast, secure WASM execution |
| Dynamic loading | libloading | 0.8 | Safe dynamic library loading |
| Serialization | serde + serde_json | 1.0 | Industry-standard serialization |
| Version parsing | semver | 1.0 | Semantic versioning |
| HTTP client | reqwest | 0.11 | For marketplace API |
| Error handling | thiserror | 1.0 | Derive-based error types |
| Async traits | async-trait | 0.1 | Trait methods with async |
| Path handling | camino | 1.0 | UTF-8 paths |
| CLI parsing | clap | 4.0 | Derive-based CLI |
| Logging | tracing | 0.1 | Structured logging |
| Proc macros | proc-macro2, syn, quote | Latest | SDK macro generation |
| SHA256 | sha2 | 0.10 | For plugin verification |
| Base64 | base64 | 0.21 | For checksums |

## Type System Design

### Core Types

```rust
// utm-plugin-core/src/types.rs

use serde::{Deserialize, Serialize};
use semver::{Version, VersionReq};
use std::path::PathBuf;
use std::collections::HashMap;

/// Unique identifier for a plugin
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginId(pub String);

impl PluginId {
    pub fn new(id: &str) -> Result<Self, PluginValidationError> {
        // Validate: lowercase, hyphens allowed, no spaces
        if !id.chars().all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit()) {
            return Err(PluginValidationError::InvalidIdFormat);
        }
        if id.is_empty() || id.len() > 64 {
            return Err(PluginValidationError::InvalidIdLength);
        }
        Ok(Self(id.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Plugin metadata from manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Plugin version (semver)
    pub version: Version,

    /// Plugin description
    pub description: Option<String>,

    /// Author information
    pub author: Option<PluginAuthor>,

    /// Plugin type/category
    pub plugin_type: PluginType,

    /// Minimum utm-dev version required
    pub min_utm_version: Option<VersionReq>,

    /// Plugin dependencies
    pub dependencies: Vec<PluginDependency>,

    /// Configuration schema (JSON Schema)
    pub config_schema: Option<serde_json::Value>,

    /// Entry point for WASM plugins
    pub entry_point: Option<String>,

    /// Required capabilities
    pub capabilities: Vec<PluginCapability>,

    /// Plugin license
    pub license: Option<String>,

    /// Homepage/documentation URL
    pub homepage: Option<String>,

    /// Repository URL
    pub repository: Option<String>,
}

/// Author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

/// Plugin type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    BuildHook,
    PostBuild,
    AssetPipeline,
    IconGenerator,
    PlatformExtension,
    Custom,
}

/// Plugin capability for access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Read-only filesystem access
    FileSystemRead,
    /// Read-write filesystem access
    FileSystemWrite,
    /// Network access (HTTP/HTTPS)
    NetworkAccess,
    /// Execute external processes
    ProcessExecution,
    /// Access environment variables
    EnvironmentVariables,
    /// Access to build configuration
    BuildConfig,
    /// Modify build output
    BuildOutput,
    /// Access to native APIs
    NativeApi,
}

/// Plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub id: String,
    pub version: VersionReq,
    pub optional: bool,
}

/// Plugin instance state
#[derive(Debug, Clone, PartialEq)]
pub enum PluginState {
    Loaded,
    Initialized,
    Running,
    Stopped,
    Error(String),
}

/// Plugin execution context
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub project_root: PathBuf,
    pub build_dir: PathBuf,
    pub output_dir: PathBuf,
    pub config: BuildConfig,
    pub environment: HashMap<String, String>,
    pub logger: PluginLogger,
}

/// Build configuration available to plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub target: String,
    pub profile: String,
    pub features: Vec<String>,
    pub incremental: bool,
}

/// Plugin logger for output
pub struct PluginLogger {
    plugin_id: String,
}

impl PluginLogger {
    pub fn new(plugin_id: &str) -> Self {
        Self { plugin_id: plugin_id.to_string() }
    }

    pub fn info(&self, message: &str) {
        println!("[{}] INFO: {}", self.plugin_id, message);
    }

    pub fn warn(&self, message: &str) {
        eprintln!("[{}] WARN: {}", self.plugin_id, message);
    }

    pub fn error(&self, message: &str) {
        eprintln!("[{}] ERROR: {}", self.plugin_id, message);
    }

    pub fn debug(&self, message: &str) {
        println!("[{}] DEBUG: {}", self.plugin_id, message);
    }
}

/// Plugin validation errors
#[derive(Debug, thiserror::Error)]
pub enum PluginValidationError {
    #[error("Invalid plugin ID format")]
    InvalidIdFormat,
    #[error("Plugin ID must be 1-64 characters")]
    InvalidIdLength,
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("Capability not permitted: {0:?}")]
    CapabilityNotPermitted(PluginCapability),
    #[error("Dependency not found: {0}")]
    DependencyNotFound(String),
    #[error("Version mismatch: required {required}, found {found}")]
    VersionMismatch { required: String, found: String },
}
```

### Error Types

```rust
// utm-plugin-core/src/error.rs

use thiserror::Error;
use std::path::PathBuf;

/// Main error type for plugin operations
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin loading failed: {0}")]
    LoadFailed(String),

    #[error("Plugin validation failed: {0}")]
    ValidationError(#[from] PluginValidationError),

    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Plugin timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Capability denied: {0:?}")]
    CapabilityDenied(PluginCapability),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("WASM error: {0}")]
    WasmError(String),

    #[error("Native library error: {0}")]
    NativeError(String),

    #[error("Hook execution failed: {plugin}: {error}")]
    HookFailed { plugin: String, error: String },

    #[error("Dependency error: {0}")]
    DependencyError(String),

    #[error("Plugin already exists: {0}")]
    AlreadyExists(String),

    #[error("Plugin is disabled: {0}")]
    Disabled(String),
}

pub type Result<T> = std::result::Result<T, PluginError>;

/// Error type for plugin marketplace operations
#[derive(Debug, Error)]
pub enum MarketplaceError {
    #[error("Plugin not found in marketplace: {0}")]
    NotFound(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type MarketplaceResult<T> = std::result::Result<T, MarketplaceError>;
```

### Traits

```rust
// utm-plugin-core/src/traits.rs

use crate::{PluginContext, PluginManifest, PluginCapability};
use std::sync::Arc;

/// Main plugin trait - all plugins must implement this
#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin manifest
    fn manifest(&self) -> &PluginManifest;

    /// Initialize the plugin
    async fn initialize(&mut self, ctx: &PluginContext) -> crate::Result<()>;

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> crate::Result<()>;

    /// Check if plugin has a capability
    fn has_capability(&self, capability: &PluginCapability) -> bool {
        self.manifest().capabilities.contains(capability)
    }

    /// Call a plugin method dynamically
    async fn call_method(
        &self,
        method: &str,
        args: serde_json::Value,
    ) -> crate::Result<serde_json::Value>;
}

/// Build hook plugin trait
#[async_trait::async_trait]
pub trait BuildHookPlugin: Plugin {
    /// Called before build starts
    async fn on_build_start(&self, ctx: &mut PluginContext) -> crate::Result<()>;

    /// Called after each file is compiled
    async fn on_file_compiled(
        &self,
        ctx: &PluginContext,
        file: &std::path::Path,
        output: &std::path::Path,
    ) -> crate::Result<()> {
        Ok(())
    }

    /// Called after build completes
    async fn on_build_complete(&self, ctx: &PluginContext) -> crate::Result<()>;

    /// Called on build failure
    async fn on_build_failure(&self, ctx: &PluginContext, error: &str) -> crate::Result<()> {
        Ok(())
    }
}

/// Post-build processor trait
#[async_trait::async_trait]
pub trait PostBuildPlugin: Plugin {
    /// Process build artifacts
    async fn process(&self, ctx: &PluginContext) -> crate::Result<PostBuildResult>;
}

#[derive(Debug, Clone, Default)]
pub struct PostBuildResult {
    pub processed_artifacts: Vec<std::path::PathBuf>,
    pub removed_artifacts: Vec<std::path::PathBuf>,
    pub generated_files: Vec<std::path::PathBuf>,
}

/// Asset pipeline plugin trait
#[async_trait::async_trait]
pub trait AssetPlugin: Plugin {
    /// Supported file extensions
    fn supported_extensions(&self) -> &[&str];

    /// Process an asset
    async fn process(&self, ctx: &AssetContext) -> crate::Result<AssetResult>;
}

pub struct AssetContext {
    pub source_path: std::path::PathBuf,
    pub output_path: std::path::PathBuf,
    pub asset_type: AssetType,
    pub config: AssetConfig,
}

pub enum AssetType {
    Image,
    Font,
    Audio,
    Video,
    Data,
    Web,
    Other,
}

pub struct AssetResult {
    pub output_path: std::path::PathBuf,
    pub optimized: bool,
    pub transformations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AssetConfig {
    pub quality: u8,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
}
```

## Key Rust-Specific Changes

### 1. Capability-Based Security

**Source Pattern:** Unrestricted plugin access

**Rust Translation:** Explicit capability enumeration with runtime enforcement

**Rationale:** Provides fine-grained access control, essential for third-party plugins.

```rust
// utm-plugin-core/src/capability.rs

pub struct CapabilityChecker {
    permitted: HashSet<PluginCapability>,
}

impl CapabilityChecker {
    pub fn new(permitted: Vec<PluginCapability>) -> Self {
        Self {
            permitted: permitted.into_iter().collect(),
        }
    }

    pub fn check(&self, required: &PluginCapability) -> Result<(), PluginError> {
        if self.permitted.contains(required) {
            Ok(())
        } else {
            Err(PluginError::CapabilityDenied(required.clone()))
        }
    }
}
```

### 2. WASM Sandboxing with Wasmtime

**Source Pattern:** Native code execution

**Rust Translation:** WASM module execution with configurable limits

**Rationale:** Provides memory safety and resource limits for untrusted plugins.

```rust
// utm-plugin-wasm/src/loader.rs

use wasmtime::{Engine, Module, Store, Instance};

pub struct WasmPluginLoader {
    engine: Engine,
    config: WasmConfig,
}

pub struct WasmConfig {
    pub max_memory_bytes: u64,
    pub max_execution_time: std::time::Duration,
    pub allowed_syscalls: Vec<String>,
}
```

### 3. Dynamic Loading with libloading

**Source Pattern:** Static linking

**Rust Translation:** Runtime dynamic library loading with symbol resolution

**Rationale:** Enables native plugins without recompiling utm-dev.

```rust
// utm-plugin-native/src/loader.rs

use libloading::{Library, Symbol};

pub struct NativePluginLoader {
    libraries: HashMap<PluginId, Library>,
}
```

### 4. Hook Registry with Priority

**Source Pattern:** Linear hook execution

**Rust Translation:** Priority-based hook execution with early exit support

**Rationale:** Allows plugins to control execution order and short-circuit.

## Ownership & Borrowing Strategy

1. **PluginManifest is Clone** - Passed to multiple components
2. **PluginContext uses Arc** - Shared across async tasks
3. **Plugin trait uses &self** - Read-only access during execution
4. **Mutable state uses tokio::sync::RwLock** - Async-safe access
5. **Plugin errors use thiserror** - Clear error propagation

```rust
// Example ownership flow

pub struct PluginManager {
    plugins: Arc<RwLock<HashMap<PluginId, Box<dyn Plugin>>>>,
}

pub async fn get_plugin(&self, id: &PluginId) -> Option<Arc<dyn Plugin>> {
    let plugins = self.plugins.read().await;
    plugins.get(id).map(|p| Arc::clone(p))
}
```

## Concurrency Model

**Approach:** Async with tokio runtime + isolated plugin execution

**Rationale:**
- Async for I/O-bound plugin operations
- Each plugin runs in isolated task
- Hook execution is sequential (deterministic order)
- Plugin loading is parallel (independent operations)

```rust
// Concurrent plugin loading

use tokio::task::JoinSet;

pub async fn load_plugins(
    &self,
    manifests: Vec<PluginManifest>,
) -> Result<Vec<PluginInstance>> {
    let mut join_set = JoinSet::new();

    for manifest in manifests {
        let loader = self.loader.clone();
        join_set.spawn(async move {
            loader.load(&manifest).await
        });
    }

    let mut plugins = Vec::new();
    while let Some(result) = join_set.join_next().await {
        plugins.push(result??);
    }

    Ok(plugins)
}
```

## Memory Considerations

1. **WASM plugins have memory limits** - Configured in WasmConfig
2. **Native plugins use separate address space** - OS-level isolation
3. **Plugin contexts are reference-counted** - Arc for shared data
4. **Large assets streamed** - Not loaded entirely into memory
5. **Hook results bounded** - Max log entries, max artifacts

## Edge Cases & Safety Guarantees

| Edge Case | Rust Handling |
|-----------|---------------|
| Plugin panic | Caught at WASM boundary, logged |
| Plugin timeout | Enforced via wasmtime epoch deadlines |
| Plugin crash | Isolated to plugin, doesn't affect host |
| Capability violation | Checked before operation, returns error |
| Circular dependencies | Detected during validation |
| Missing dependency | Plugin fails to load, error returned |
| Double initialization | State machine prevents re-init |

## Code Examples

### Example: Plugin Manager

```rust
// utm-plugin-manager/src/manager.rs

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use utm_plugin_core::{PluginId, PluginManifest, Plugin, PluginContext, PluginError, PluginState};

/// Manages plugin lifecycle
pub struct PluginManager {
    config: PluginManagerConfig,
    plugins: Arc<RwLock<HashMap<PluginId, Box<dyn Plugin>>>>,
    states: Arc<RwLock<HashMap<PluginId, PluginState>>>,
    registry: Arc<RwLock<PluginRegistry>>,
    hooks: Arc<RwLock<HookRegistry>>,
}

#[derive(Debug, Clone)]
pub struct PluginManagerConfig {
    pub plugin_dirs: Vec<std::path::PathBuf>,
    pub enabled_plugins: Vec<PluginId>,
    pub disabled_plugins: Vec<PluginId>,
    pub sandbox_enabled: bool,
    pub max_memory_mb: u32,
    pub timeout_ms: u64,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            plugin_dirs: vec![
                std::path::PathBuf::from("./plugins"),
                std::path::PathBuf::from("./.utm/plugins"),
            ],
            enabled_plugins: vec![],
            disabled_plugins: vec![],
            sandbox_enabled: true,
            max_memory_mb: 256,
            timeout_ms: 30000,
        }
    }
}

impl PluginManager {
    /// Create a new plugin manager
    pub async fn new(config: PluginManagerConfig) -> Result<Self, PluginError> {
        let manager = Self {
            config,
            plugins: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(RwLock::new(PluginRegistry::new())),
            hooks: Arc::new(RwLock::new(HookRegistry::new())),
        };

        manager.discover_plugins().await?;
        manager.load_enabled_plugins().await?;

        Ok(manager)
    }

    /// Discover all available plugins
    pub async fn discover_plugins(&self) -> Result<(), PluginError> {
        let mut registry = self.registry.write().await;

        for plugin_dir in &self.config.plugin_dirs {
            if !plugin_dir.exists() {
                continue;
            }

            // Find plugin manifests
            let manifest_paths = Self::find_manifests(plugin_dir).await?;

            for manifest_path in manifest_paths {
                let manifest = PluginManifest::load(&manifest_path).await?;
                registry.register(manifest, manifest_path.parent().unwrap().to_path_buf());
            }
        }

        Ok(())
    }

    /// Load all enabled plugins
    pub async fn load_enabled_plugins(&self) -> Result<(), PluginError> {
        let registry = self.registry.read().await;
        let mut plugins = self.plugins.write().await;
        let mut hooks = self.hooks.write().await;

        for plugin_info in registry.all() {
            if self.is_plugin_enabled(&plugin_info.manifest.id) {
                let instance = plugin_info.load().await?;

                // Register hooks
                hooks.register_hooks(&instance);

                let plugin_id = PluginId::new(&plugin_info.manifest.id)?;
                plugins.insert(plugin_id, instance);
            }
        }

        Ok(())
    }

    fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        let id = PluginId::new(plugin_id).ok();

        // Explicitly disabled takes precedence
        if let Some(id) = &id {
            if self.config.disabled_plugins.contains(id) {
                return false;
            }
        }

        // If enabled list is empty, all non-disabled are enabled
        if self.config.enabled_plugins.is_empty() {
            return true;
        }

        id.map(|i| self.config.enabled_plugins.contains(&i)).unwrap_or(false)
    }

    /// Execute build start hooks
    pub async fn on_build_start(&self, ctx: &mut PluginContext) -> Result<(), PluginError> {
        let hooks = self.hooks.read().await;
        let plugins = self.plugins.read().await;

        for hook in hooks.build_start() {
            if let Some(plugin) = plugins.get(&hook.plugin_id) {
                plugin.on_build_start(ctx).await
                    .map_err(|e| PluginError::HookFailed {
                        plugin: hook.plugin_id.as_str().to_string(),
                        error: e.to_string(),
                    })?;
            }
        }

        Ok(())
    }

    /// Execute build complete hooks
    pub async fn on_build_complete(&self, ctx: &PluginContext) -> Result<(), PluginError> {
        let hooks = self.hooks.read().await;
        let plugins = self.plugins.read().await;

        for hook in hooks.build_complete() {
            if let Some(plugin) = plugins.get(&hook.plugin_id) {
                plugin.on_build_complete(ctx).await
                    .map_err(|e| PluginError::HookFailed {
                        plugin: hook.plugin_id.as_str().to_string(),
                        error: e.to_string(),
                    })?;
            }
        }

        Ok(())
    }

    /// Get plugin by ID
    pub async fn get_plugin(&self, id: &str) -> Option<Box<dyn Plugin>> {
        let plugins = self.plugins.read().await;
        let id = PluginId::new(id).ok()?;
        plugins.get(&id).cloned()
    }

    /// List all loaded plugins
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.info()).collect()
    }

    async fn find_manifests(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, PluginError> {
        let mut manifests = Vec::new();
        let mut entries = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "toml" || e == "json").unwrap_or(false) {
                if path.file_name().map(|n| n.to_string_lossy().contains("plugin")).unwrap_or(false) {
                    manifests.push(path);
                }
            }
        }

        Ok(manifests)
    }
}
```

### Example: WASM Plugin Loader

```rust
// utm-plugin-wasm/src/loader.rs

use wasmtime::{Engine, Module, Store, Instance, Func, TypedFunc, Config};
use std::path::Path;
use std::sync::Arc;
use utm_plugin_core::{Plugin, PluginManifest, PluginContext, PluginError, PluginState};

/// WASM plugin configuration
#[derive(Debug, Clone)]
pub struct WasmConfig {
    pub max_memory_bytes: u64,
    pub max_execution_time: std::time::Duration,
    pub allowed_syscalls: Vec<String>,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 64 * 1024 * 1024, // 64MB
            max_execution_time: std::time::Duration::from_secs(30),
            allowed_syscalls: vec![], // No syscalls by default
        }
    }
}

/// Loader for WASM plugins
pub struct WasmPluginLoader {
    engine: Engine,
    config: WasmConfig,
}

impl WasmPluginLoader {
    pub fn new(config: WasmConfig) -> Result<Self, PluginError> {
        let mut wasmtime_config = Config::new();
        wasmtime_config.epoch_interruption(true);
        wasmtime_config.memory_growing(false); // No dynamic memory growth

        let engine = Engine::new(&wasmtime_config)
            .map_err(|e| PluginError::WasmError(e.to_string()))?;

        Ok(Self { engine, config })
    }

    /// Load a WASM plugin from file
    pub async fn load(&self, manifest: &PluginManifest, wasm_path: &Path) -> Result<WasmPlugin, PluginError> {
        // Validate WASM file
        let wasm_bytes = tokio::fs::read(wasm_path).await
            .map_err(|e| PluginError::IoError(e))?;

        // Validate module
        let module = Module::from_binary(&self.engine, &wasm_bytes)
            .map_err(|e| PluginError::WasmError(e.to_string()))?;

        // Create store with limits
        let mut store = Store::new(&self.engine, WasmState::new(&self.config));

        // Set up epoch deadline for timeout
        let deadline = std::time::Instant::now() + self.config.max_execution_time;
        store.set_epoch_deadline((deadline - std::time::Instant::now()).as_secs());

        // Create host functions
        let log_func = Func::wrap(&mut store, |level: i32, ptr: i32, len: i32| {
            // Host log function implementation
        });

        let read_file_func = Func::wrap(&mut store, |ptr: i32, len: i32| {
            // File read from host
        });

        let write_file_func = Func::wrap(&mut store, |ptr: i32, len: i32, data_ptr: i32, data_len: i32| {
            // File write to host
        });

        // Instantiate module with host functions
        let instance = Instance::new(&mut store, &module, &[
            log_func.into(),
            read_file_func.into(),
            write_file_func.into(),
        ]).map_err(|e| PluginError::WasmError(e.to_string()))?;

        // Get exported functions
        let on_build_start: Option<TypedFunc<(i32, i32), i32>> =
            instance.get_typed_func(&mut store, "on_build_start").ok();

        let on_build_complete: Option<TypedFunc<(i32, i32), i32>> =
            instance.get_typed_func(&mut store, "on_build_complete").ok();

        Ok(WasmPlugin {
            manifest: manifest.clone(),
            module,
            instance,
            store,
            on_build_start,
            on_build_complete,
        })
    }
}

/// WASM plugin instance
pub struct WasmPlugin {
    manifest: PluginManifest,
    module: Module,
    instance: Instance,
    store: Store<WasmState>,
    on_build_start: Option<TypedFunc<(i32, i32), i32>>,
    on_build_complete: Option<TypedFunc<(i32, i32), i32>>,
}

struct WasmState {
    memory: Vec<u8>,
    allocations: Vec<(usize, usize)>,
    config: WasmConfig,
}

impl WasmState {
    fn new(config: &WasmConfig) -> Self {
        Self {
            memory: Vec::with_capacity(1024 * 1024), // 1MB initial
            allocations: Vec::new(),
            config: config.clone(),
        }
    }

    fn allocate_memory(&mut self, data: &[u8]) -> i32 {
        let offset = self.memory.len();
        self.memory.extend_from_slice(data);
        self.allocations.push((offset, data.len()));
        offset as i32
    }

    fn read_memory(&self, ptr: i32, len: i32) -> &[u8] {
        let offset = ptr as usize;
        let len = len as usize;
        &self.memory[offset..offset + len]
    }
}

#[async_trait::async_trait]
impl Plugin for WasmPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn initialize(&mut self, ctx: &PluginContext) -> Result<(), PluginError> {
        // Initialize WASM plugin
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        // Cleanup WASM resources
        Ok(())
    }

    async fn call_method(
        &self,
        method: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        // Call WASM function dynamically
        Ok(serde_json::Value::Null)
    }
}
```

### Example: Hook Registry

```rust
// utm-plugin-manager/src/registry.rs

use std::collections::HashMap;
use utm_plugin_core::{PluginId, Plugin, PluginCapability};

/// Registry for plugin hooks
pub struct HookRegistry {
    build_start: Vec<HookRegistration>,
    build_complete: Vec<HookRegistration>,
    pre_asset_process: Vec<HookRegistration>,
    post_asset_process: Vec<HookRegistration>,
}

struct HookRegistration {
    plugin_id: PluginId,
    priority: i32, // Higher = runs first
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            build_start: Vec::new(),
            build_complete: Vec::new(),
            pre_asset_process: Vec::new(),
            post_asset_process: Vec::new(),
        }
    }

    /// Register hooks from a plugin
    pub fn register_hooks(&mut self, plugin: &dyn Plugin) {
        // Determine hook priority from manifest
        let priority = Self::extract_priority(plugin.manifest());

        // Register build_start hook
        self.build_start.push(HookRegistration {
            plugin_id: PluginId::new(&plugin.manifest().id).unwrap(),
            priority,
        });

        // Sort by priority (descending)
        self.build_start.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    fn extract_priority(manifest: &PluginManifest) -> i32 {
        // Extract from config or use default
        manifest.config_schema
            .as_ref()
            .and_then(|c| c.get("priority"))
            .and_then(|p| p.as_i64())
            .map(|p| p as i32)
            .unwrap_or(0)
    }

    /// Get all build_start hooks in execution order
    pub fn build_start(&self) -> &[HookRegistration] {
        &self.build_start
    }

    /// Get all build_complete hooks in execution order
    pub fn build_complete(&self) -> &[HookRegistration] {
        &self.build_complete
    }
}
```

### Example: Plugin SDK Macro

```rust
// utm-plugin-sdk/src/macros.rs

/// Macro for registering a plugin
///
/// Usage: register_plugin!(MyPlugin)
#[macro_export]
macro_rules! register_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn _plugin_register() -> *mut $crate::PluginRegistration {
            Box::into_raw(Box::new($crate::PluginRegistration {
                plugin_type: std::any::TypeId::of::<$plugin_type>(),
                create: |api| Box::new(<$plugin_type>::new(api)),
            }))
        }
    };
}

/// Macro for defining plugin capabilities
///
/// Usage: define_capabilities!(FileSystemRead, NetworkAccess)
#[macro_export]
macro_rules! define_capabilities {
    ($($cap:ident),*) => {
        vec![
            $(utm_plugin_core::PluginCapability::$cap),*
        ]
    };
}

/// Macro for declaring build hooks
///
/// Usage: declare_hooks!(on_build_start, on_build_complete)
#[macro_export]
macro_rules! declare_hooks {
    ($($hook:ident),*) => {
        impl $crate::BuildHookPlugin for Self {
            $(
                async fn $hook(&self, ctx: &mut PluginContext) -> Result<()> {
                    self.$hook(ctx).await
                }
            )*
        }
    };
}

/// Example plugin using the SDK
///
/// ```rust
/// use utm_plugin_sdk::*;
///
/// pub struct MyPlugin {
///     api: Box<dyn UtmDevApi>,
/// }
///
/// impl MyPlugin {
///     pub fn new(api: Box<dyn UtmDevApi>) -> Self {
///         Self { api }
///     }
/// }
///
/// register_plugin!(MyPlugin);
///
/// #[async_trait::async_trait]
/// impl Plugin for MyPlugin {
///     fn manifest(&self) -> &PluginManifest { /* ... */ }
///     async fn initialize(&mut self, ctx: &PluginContext) -> Result<()> { Ok(()) }
///     async fn shutdown(&mut self) -> Result<()> { Ok(()) }
///     async fn call_method(&self, method: &str, args: Value) -> Result<Value> { Ok(Value::Null) }
/// }
/// ```
```

### Example: Marketplace Client

```rust
// utm-plugin-marketplace/src/client.rs

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use utm_plugin_core::PluginManifest;

/// Client for the plugin marketplace API
pub struct MarketplaceClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: AuthorInfo,
    pub category: String,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: Option<f32>,
    pub versions: Vec<PluginVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersion {
    pub version: String,
    pub download_url: String,
    pub checksum: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    pub verified: bool,
}

impl MarketplaceClient {
    pub fn new(base_url: &str) -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .user_agent("utm-dev-plugin-client/1.0")
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.to_string(),
        })
    }

    /// Search for plugins
    pub async fn search(
        &self,
        query: &str,
        category: Option<&str>,
        verified_only: bool,
    ) -> Result<Vec<MarketplacePlugin>, crate::MarketplaceError> {
        let mut req = self.client.get(format!("{}/plugins", self.base_url))
            .query(&[("q", query)]);

        if let Some(cat) = category {
            req = req.query(&[("category", cat)]);
        }

        if verified_only {
            req = req.query(&[("verified", "true")]);
        }

        let response = req.send().await?;
        let plugins = response.json().await?;

        Ok(plugins)
    }

    /// Get plugin details
    pub async fn get_plugin(&self, plugin_id: &str) -> Result<MarketplacePlugin, crate::MarketplaceError> {
        let response = self.client.get(format!("{}/plugins/{}", self.base_url, plugin_id))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(crate::MarketplaceError::NotFound(plugin_id.to_string()));
        }

        Ok(response.json().await?)
    }

    /// Download plugin
    pub async fn download(
        &self,
        plugin_id: &str,
        version: Option<&str>,
    ) -> Result<DownloadedPlugin, crate::MarketplaceError> {
        let plugin = self.get_plugin(plugin_id).await?;

        // Find version
        let version_info = if let Some(v) = version {
            plugin.versions.iter().find(|ver| ver.version == v)
        } else {
            plugin.versions.first()
        }.ok_or_else(|| crate::MarketplaceError::DownloadFailed("Version not found".to_string()))?;

        // Download
        let response = self.client.get(&version_info.download_url)
            .send()
            .await?;

        let bytes = response.bytes().await?;

        // Verify checksum
        Self::verify_checksum(&bytes, &version_info.checksum)?;

        Ok(DownloadedPlugin {
            manifest: serde_json::from_str(&String::from_utf8_lossy(&bytes))?,
            bytes: bytes.to_vec(),
            checksum: version_info.checksum.clone(),
        })
    }

    fn verify_checksum(bytes: &[u8], expected: &str) -> Result<(), crate::MarketplaceError> {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(bytes);
        let actual = format!("{:x}", hash);

        if actual == expected {
            Ok(())
        } else {
            Err(crate::MarketplaceError::SignatureVerificationFailed(
                "Checksum mismatch".to_string()
            ))
        }
    }
}

pub struct DownloadedPlugin {
    pub manifest: PluginManifest,
    pub bytes: Vec<u8>,
    pub checksum: String,
}
```

## Migration Path

1. **Week 1-2: Core Infrastructure**
   - Set up workspace structure
   - Implement core types and traits
   - Define capability system

2. **Week 3-4: Plugin Manager**
   - Build plugin registry
   - Implement hook system
   - Add discovery mechanism

3. **Week 5-6: WASM Runtime**
   - Integrate wasmtime
   - Implement host functions
   - Add security sandboxing

4. **Week 7-8: Native Loading**
   - Implement libloading integration
   - Add symbol resolution
   - Security validation

5. **Week 9-10: SDK and Documentation**
   - Create plugin SDK
   - Write documentation
   - Build example plugins

6. **Week 11-12: Marketplace**
   - Build marketplace API
   - Implement download/install
   - Add signature verification

## Performance Considerations

1. **WASM startup** - Pre-warm module instances
2. **Hook execution** - Parallel for independent hooks
3. **Plugin discovery** - Cache manifest results
4. **Memory limits** - Enforced at WASM level
5. **Lazy loading** - Load plugins on first use

## Testing Strategy

1. **Unit tests** for plugin validation, capability checking
2. **Integration tests** for hook execution
3. **WASM tests** with test modules
4. **Security tests** for capability violations
5. **End-to-end tests** for marketplace flow

## Open Considerations

1. **Plugin signing** - Should plugins be cryptographically signed?
2. **Hot reload** - Support for reloading plugins without restart
3. **Version constraints** - How to handle breaking API changes
4. **Resource accounting** - Track plugin resource usage
5. **Debugging** - Tools for debugging plugin issues
