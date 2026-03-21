# utm-dev Production - Hybrid App Architecture Rust Revision

## Overview

This document provides a comprehensive Rust implementation for hybrid application architecture, enabling native functionality through a plugin architecture, JavaScript bridges, native module loading, IPC mechanisms, and state synchronization between native and web layers. The implementation replaces the Go/TypeScript-based architecture with idiomatic Rust.

**Key Goals:**
- Unified plugin system supporting both native and WASM plugins
- Type-safe JavaScript bridge for native/web communication
- Secure native module loading with sandboxing
- Robust IPC mechanisms for cross-process communication
- Real-time state synchronization between layers
- Plugin marketplace infrastructure

## Workspace Structure

```
utm-hybrid/
├── Cargo.toml                 # Workspace root
├── README.md
├── utm-hybrid-core/           # Core traits and types
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── traits.rs          # Plugin trait definitions
│       ├── error.rs           # Unified error types
│       ├── config.rs          # Configuration types
│       └── capabilities.rs    # Plugin capabilities
├── utm-hybrid-plugin/         # Plugin system core
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── registry.rs        # Plugin registry
│       ├── manifest.rs        # Plugin manifest
│       ├── context.rs         # Plugin context
│       └── loader.rs          # Plugin loader
├── utm-hybrid-wasm/           # WASM plugin support
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── loader.rs          # WASM loader
│       ├── runtime.rs         # WASM runtime
│       └── sandbox.rs         # WASM sandboxing
├── utm-hybrid-bridge/         # JavaScript bridge
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── bridge.rs          # Bridge core
│       ├── methods.rs         # Built-in methods
│       └── events.rs          # Event handling
├── utm-hybrid-ipc/            # IPC mechanisms
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── channel.rs         # Message channels
│       ├── webview.rs         # WebView IPC
│       └── protocol.rs        # IPC protocol
├── utm-hybrid-state/          # State synchronization
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── sync.rs            # State sync core
│       ├── store.rs           # State store
│       └── history.rs         # State history
├── utm-hybrid-marketplace/    # Plugin marketplace
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs          # Marketplace client
│       ├── download.rs        # Plugin downloads
│       └── verify.rs          # Plugin verification
└── utm-hybrid-cli/            # CLI tool
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── commands/
            ├── plugins.rs
            ├── marketplace.rs
            └── dev.rs
```

## Crate Breakdown

| Crate | Purpose | Platforms |
|-------|---------|-----------|
| `utm-hybrid-core` | Shared traits, types, capabilities | All |
| `utm-hybrid-plugin` | Plugin registry, manifest, loader | All |
| `utm-hybrid-wasm` | WASM plugin runtime | All |
| `utm-hybrid-bridge` | JavaScript bridge | All |
| `utm-hybrid-ipc` | IPC channels and protocols | All |
| `utm-hybrid-state` | State synchronization | All |
| `utm-hybrid-marketplace` | Plugin marketplace client | All |
| `utm-hybrid-cli` | CLI for plugin management | All |

## Recommended Dependencies

### utm-hybrid-core/Cargo.toml
```toml
[package]
name = "utm-hybrid-core"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["sync"] }
async-trait = "0.1"
tracing = "0.1"
```

### utm-hybrid-plugin/Cargo.toml
```toml
[package]
name = "utm-hybrid-plugin"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-hybrid-core = { path = "../utm-hybrid-core" }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
libloading = "0.8"
semver = "1.0"
dirs = "5.0"
walkdir = "2.4"
tracing = "0.1"
```

### utm-hybrid-wasm/Cargo.toml
```toml
[package]
name = "utm-hybrid-wasm"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-hybrid-core = { path = "../utm-hybrid-core" }
utm-hybrid-plugin = { path = "../utm-hybrid-plugin" }
wasmtime = "17.0"
thiserror = "1.0"
serde_json = "1.0"
tracing = "0.1"
```

### utm-hybrid-bridge/Cargo.toml
```toml
[package]
name = "utm-hybrid-bridge"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-hybrid-core = { path = "../utm-hybrid-core" }
utm-hybrid-plugin = { path = "../utm-hybrid-plugin" }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["sync"] }
tracing = "0.1"
```

## Type System Design

### Core Plugin Trait (utm-hybrid-core)

```rust
// utm-hybrid-core/src/traits.rs
use std::any::Any;
use serde_json::Value;
use crate::error::{PluginResult, PluginCapability};

/// Unique identifier for plugin SDK version
pub const SDK_VERSION: u32 = 1;

/// Core trait that all plugins must implement
pub trait NativePlugin: Send + Sync {
    /// Get unique plugin identifier
    fn id(&self) -> &str;

    /// Get plugin version (semver format)
    fn version(&self) -> &str;

    /// Get human-readable plugin name
    fn name(&self) -> &str;

    /// Get plugin description
    fn description(&self) -> &str;

    /// Get plugin author (if any)
    fn author(&self) -> Option<&str> {
        None
    }

    /// Initialize plugin with context
    fn initialize(&mut self, ctx: &PluginContext) -> PluginResult<()>;

    /// Shutdown plugin gracefully
    fn shutdown(&mut self);

    /// Handle method call from JavaScript or other plugins
    fn call_method(&self, method: &str, args: Value) -> PluginResult<Value>;

    /// Get plugin capabilities/permissions
    fn capabilities(&self) -> &[PluginCapability];

    /// Check if plugin is currently enabled
    fn is_enabled(&self) -> bool;

    /// Enable/disable plugin
    fn set_enabled(&mut self, enabled: bool) {
        // Default implementation does nothing
    }

    /// Get plugin metadata as JSON
    fn metadata(&self) -> Value {
        serde_json::json!({
            "id": self.id(),
            "version": self.version(),
            "name": self.name(),
            "description": self.description(),
            "author": self.author(),
            "capabilities": self.capabilities().iter().map(|c| c.as_str()).collect::<Vec<_>>(),
            "enabled": self.is_enabled(),
        })
    }
}

/// Plugin context provided by the host application
pub struct PluginContext {
    /// Application data directory
    pub app_data_dir: PathBuf,

    /// Plugin-specific data directory
    pub plugin_data_dir: PathBuf,

    /// Configuration access
    pub config: Arc<PluginConfig>,

    /// Logger for this plugin
    pub logger: PluginLogger,

    /// Host API access
    pub host_api: Arc<dyn HostApi>,
}

/// Plugin configuration
#[derive(Debug, Clone)]
pub struct PluginConfig {
    /// Plugin-specific settings
    pub settings: HashMap<String, Value>,
    /// Whether plugin is enabled
    pub enabled: bool,
}

/// Logger for plugin output
pub struct PluginLogger {
    plugin_id: String,
}

impl PluginLogger {
    pub fn new(plugin_id: &str) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
        }
    }

    pub fn debug(&self, message: &str) {
        tracing::debug!("[{}] {}", self.plugin_id, message);
    }

    pub fn info(&self, message: &str) {
        tracing::info!("[{}] {}", self.plugin_id, message);
    }

    pub fn warn(&self, message: &str) {
        tracing::warn!("[{}] {}", self.plugin_id, message);
    }

    pub fn error(&self, message: &str) {
        tracing::error!("[{}] {}", self.plugin_id, message);
    }
}

/// Host API that plugins can access
pub trait HostApi: Send + Sync {
    /// File system operations
    fn fs(&self) -> &dyn FileSystemApi;

    /// Process operations
    fn process(&self) -> &dyn ProcessApi;

    /// Network operations (optional)
    fn network(&self) -> Option<&dyn NetworkApi>;

    /// UI operations
    fn ui(&self) -> &dyn UiApi;

    /// Settings access
    fn settings(&self) -> &dyn SettingsApi;
}

/// File system API for plugins
pub trait FileSystemApi: Send + Sync {
    /// Read file contents
    fn read(&self, path: &str) -> PluginResult<String>;

    /// Write file contents
    fn write(&self, path: &str, content: &str) -> PluginResult<()>;

    /// List directory contents
    fn list(&self, path: &str, recursive: bool) -> PluginResult<Vec<DirEntry>>;

    /// Check if path exists
    fn exists(&self, path: &str) -> PluginResult<bool>;

    /// Remove file or directory
    fn remove(&self, path: &str, recursive: bool) -> PluginResult<()>;

    /// Create directory
    fn create_dir(&self, path: &str) -> PluginResult<()>;
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

/// Process API for plugins
pub trait ProcessApi: Send + Sync {
    /// Run a command
    fn run(&self, command: &str, args: &[&str]) -> PluginResult<ProcessOutput>;

    /// Kill a process
    fn kill(&self, pid: u32) -> PluginResult<()>;

    /// List running processes
    fn list(&self) -> PluginResult<Vec<ProcessInfo>>;
}

/// Process output
#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Process info
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: String,
}

/// Network API for plugins
pub trait NetworkApi: Send + Sync {
    /// HTTP GET request
    fn get(&self, url: &str) -> PluginResult<HttpResponse>;

    /// HTTP POST request
    fn post(&self, url: &str, body: &str) -> PluginResult<HttpResponse>;

    /// HTTP request with full control
    fn request(&self, req: HttpRequest) -> PluginResult<HttpResponse>;
}

/// HTTP request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// UI API for plugins
pub trait UiApi: Send + Sync {
    /// Show notification
    fn notify(&self, title: &str, message: &str, level: NotificationLevel) -> PluginResult<()>;

    /// Show dialog
    fn dialog(&self, message: &str, buttons: &[&str]) -> PluginResult<Option<usize>>;

    /// Get clipboard contents
    fn clipboard_get(&self) -> PluginResult<String>;

    /// Set clipboard contents
    fn clipboard_set(&self, content: &str) -> PluginResult<()>;
}

/// Notification level
#[derive(Debug, Clone, Copy)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// Settings API for plugins
pub trait SettingsApi: Send + Sync {
    /// Get setting value
    fn get(&self, key: &str) -> PluginResult<Option<Value>>;

    /// Set setting value
    fn set(&self, key: &str, value: Value) -> PluginResult<()>;

    /// Get all settings
    fn all(&self) -> PluginResult<HashMap<String, Value>>;
}

/// Plugin capabilities/permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginCapability {
    FileSystemRead,
    FileSystemWrite,
    ProcessExecution,
    NetworkAccess,
    SystemTray,
    Notifications,
    Clipboard,
    GlobalShortcut,
    NativeMenu,
    WindowControl,
}

impl PluginCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginCapability::FileSystemRead => "fs:read",
            PluginCapability::FileSystemWrite => "fs:write",
            PluginCapability::ProcessExecution => "process:exec",
            PluginCapability::NetworkAccess => "network",
            PluginCapability::SystemTray => "ui:tray",
            PluginCapability::Notifications => "ui:notify",
            PluginCapability::Clipboard => "clipboard",
            PluginCapability::GlobalShortcut => "input:shortcut",
            PluginCapability::NativeMenu => "ui:menu",
            PluginCapability::WindowControl => "ui:window",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "fs:read" => Some(PluginCapability::FileSystemRead),
            "fs:write" => Some(PluginCapability::FileSystemWrite),
            "process:exec" => Some(PluginCapability::ProcessExecution),
            "network" => Some(PluginCapability::NetworkAccess),
            "ui:tray" => Some(PluginCapability::SystemTray),
            "ui:notify" => Some(PluginCapability::Notifications),
            "clipboard" => Some(PluginCapability::Clipboard),
            "input:shortcut" => Some(PluginCapability::GlobalShortcut),
            "ui:menu" => Some(PluginCapability::NativeMenu),
            "ui:window" => Some(PluginCapability::WindowControl),
            _ => None,
        }
    }
}
```

### Error Types (utm-hybrid-core)

```rust
// utm-hybrid-core/src/error.rs
use thiserror::Error;

/// Plugin operation result
pub type PluginResult<T> = Result<T, PluginError>;

/// Unified plugin error type
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Plugin SDK version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),

    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Method execution failed: {0}")]
    MethodFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Library loading error: {0}")]
    LibraryError(String),

    #[error("WASM error: {0}")]
    WasmError(String),

    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin timeout: {0}")]
    Timeout(String),

    #[error("Plugin crashed: {0}")]
    Crash(String),
}

/// Bridge error type
#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Method execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Callback not found: {0}")]
    CallbackNotFound(String),

    #[error("WebView error: {0}")]
    WebViewError(String),
}

pub type BridgeResult<T> = Result<T, BridgeError>;

/// IPC error type
#[derive(Error, Debug)]
pub enum IpcError {
    #[error("Send error: {0}")]
    SendFailed(String),

    #[error("Receive error: {0}")]
    ReceiveFailed(String),

    #[error("Timeout")]
    Timeout,

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),
}

pub type IpcResult<T> = Result<T, IpcError>;

/// State sync error type
#[derive(Error, Debug)]
pub enum StateError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Broadcast failed")]
    BroadcastFailed,

    #[error("No history available")]
    NoHistory,

    #[error("Merge conflict: {0}")]
    MergeConflict(String),
}

pub type StateResult<T> = Result<T, StateError>;
```

### Plugin Manifest (utm-hybrid-plugin)

```rust
// utm-hybrid-plugin/src/manifest.rs
use serde::{Deserialize, Serialize};
use utm_hybrid_core::{PluginCapability, PluginError, PluginResult};

/// Plugin manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin ID (reverse-DNS style recommended)
    pub id: String,

    /// Plugin version (semver format)
    pub version: String,

    /// Human-readable name
    pub name: String,

    /// Plugin description
    pub description: Option<String>,

    /// Author information
    pub author: Option<String>,

    /// Plugin type: "native" or "wasm"
    #[serde(rename = "type")]
    pub plugin_type: String,

    /// Entry point file
    #[serde(rename = "entry_point")]
    pub entry_point: String,

    /// Plugin is enabled by default
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Required capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Minimum host version required
    pub min_host_version: Option<String>,

    /// Plugin dependencies
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,

    /// Configuration schema (JSON Schema)
    pub config_schema: Option<serde_json::Value>,

    /// Plugin homepage URL
    pub homepage: Option<String>,

    /// Repository URL
    pub repository: Option<String>,

    /// License identifier (SPDX)
    pub license: Option<String>,
}

fn default_true() -> bool {
    true
}

impl PluginManifest {
    /// Validate the manifest
    pub fn validate(&self) -> PluginResult<()> {
        // Validate ID format (alphanumeric, dash, underscore, dot)
        if !self.id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
            return Err(PluginError::InvalidManifest(
                format!("Invalid plugin ID: {}", self.id)
            ));
        }

        // Validate version (semver)
        if let Err(e) = semver::Version::parse(&self.version) {
            return Err(PluginError::InvalidManifest(
                format!("Invalid plugin version '{}': {}", self.version, e)
            ));
        }

        // Validate type
        if self.plugin_type != "native" && self.plugin_type != "wasm" {
            return Err(PluginError::InvalidManifest(
                format!("Invalid plugin type: {}", self.plugin_type)
            ));
        }

        // Validate capabilities
        for cap in &self.capabilities {
            if PluginCapability::from_str(cap).is_none() {
                return Err(PluginError::InvalidManifest(
                    format!("Unknown capability: {}", cap)
                ));
            }
        }

        Ok(())
    }

    /// Parse capabilities from manifest
    pub fn parse_capabilities(&self) -> PluginResult<Vec<PluginCapability>> {
        self.capabilities
            .iter()
            .map(|s| {
                PluginCapability::from_str(s)
                    .ok_or_else(|| PluginError::InvalidManifest(
                        format!("Unknown capability: {}", s)
                    ))
            })
            .collect()
    }
}

/// Plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Dependency plugin ID
    pub id: String,

    /// Required version (semver range)
    pub version: String,

    /// Whether dependency is optional
    #[serde(default)]
    pub optional: bool,
}

/// Plugin info (lightweight metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub path: PathBuf,
    pub plugin_type: String,
    pub description: Option<String>,
}
```

### Plugin Registry (utm-hybrid-plugin)

```rust
// utm-hybrid-plugin/src/registry.rs
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use utm_hybrid_core::{NativePlugin, PluginContext, PluginCapability, PluginError, PluginResult};
use crate::manifest::{PluginManifest, PluginInfo};
use crate::loader::PluginLoader;

/// Plugin registry managing all loaded plugins
pub struct PluginRegistry {
    /// Loaded plugins
    plugins: HashMap<String, Box<dyn NativePlugin>>,
    /// Plugin directories to search
    plugin_dirs: Vec<PathBuf>,
    /// Registry configuration
    config: PluginRegistryConfig,
    /// Plugin loader
    loader: PluginLoader,
}

/// Plugin registry configuration
#[derive(Debug, Clone)]
pub struct PluginRegistryConfig {
    /// Enable plugin sandboxing
    pub sandbox_enabled: bool,

    /// Maximum plugin memory usage (MB)
    pub max_memory_mb: u32,

    /// Plugin call timeout (ms)
    pub timeout_ms: u64,

    /// Auto-load plugins on discovery
    pub auto_load: bool,
}

impl Default for PluginRegistryConfig {
    fn default() -> Self {
        Self {
            sandbox_enabled: true,
            max_memory_mb: 256,
            timeout_ms: 5000,
            auto_load: true,
        }
    }
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new(config: PluginRegistryConfig) -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dirs: Vec::new(),
            config,
            loader: PluginLoader::new(),
        }
    }

    /// Add a plugin directory to search
    pub fn add_plugin_dir(&mut self, path: PathBuf) {
        self.plugin_dirs.push(path);
    }

    /// Discover plugins in registered directories
    pub fn discover_plugins(&self) -> PluginResult<Vec<PluginInfo>> {
        let mut discovered = Vec::new();

        for plugin_dir in &self.plugin_dirs {
            if !plugin_dir.exists() {
                continue;
            }

            // Look for plugin manifests
            if let Ok(entries) = std::fs::read_dir(plugin_dir) {
                for entry in entries.flatten() {
                    let manifest_path = entry.path().join("plugin.json");
                    if manifest_path.exists() {
                        if let Ok(info) = self.get_plugin_info(&manifest_path) {
                            discovered.push(info);
                        }
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Get plugin info from manifest
    fn get_plugin_info(&self, manifest_path: &Path) -> PluginResult<PluginInfo> {
        let content = std::fs::read_to_string(manifest_path)?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;

        Ok(PluginInfo {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            enabled: manifest.enabled,
            path: manifest_path.parent().unwrap().to_path_buf(),
            plugin_type: manifest.plugin_type,
            description: manifest.description,
        })
    }

    /// Load a plugin from manifest path
    pub fn load_plugin(&mut self, manifest_path: &Path) -> PluginResult<PluginInfo> {
        let content = std::fs::read_to_string(manifest_path)?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;

        // Validate manifest
        manifest.validate()?;

        // Check if already loaded
        if self.plugins.contains_key(&manifest.id) {
            return Err(PluginError::AlreadyLoaded(manifest.id.clone()));
        }

        // Build plugin path
        let plugin_path = manifest_path.parent()
            .unwrap()
            .join(&manifest.entry_point);

        // Load the plugin
        let plugin = self.loader.load(&plugin_path, &manifest)?;

        let info = PluginInfo {
            id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            enabled: manifest.enabled,
            path: manifest_path.parent().unwrap().to_path_buf(),
            plugin_type: manifest.plugin_type,
            description: manifest.description.clone(),
        };

        self.plugins.insert(manifest.id, plugin);

        Ok(info)
    }

    /// Unload a plugin by ID
    pub fn unload_plugin(&mut self, plugin_id: &str) -> PluginResult<()> {
        let plugin = self.plugins.get_mut(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        plugin.shutdown();
        self.plugins.remove(plugin_id);

        Ok(())
    }

    /// Get plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&dyn NativePlugin> {
        self.plugins.get(plugin_id).map(|p| p.as_ref())
    }

    /// Get mutable plugin reference
    pub fn get_plugin_mut(&mut self, plugin_id: &str) -> Option<&mut dyn NativePlugin> {
        self.plugins.get_mut(plugin_id).map(|p| p.as_mut())
    }

    /// Call a plugin method
    pub fn call_plugin(
        &self,
        plugin_id: &str,
        method: &str,
        args: serde_json::Value,
    ) -> PluginResult<serde_json::Value> {
        let plugin = self.plugins.get(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        if !plugin.is_enabled() {
            return Err(PluginError::PermissionDenied(
                format!("Plugin {} is disabled", plugin_id)
            ));
        }

        plugin.call_method(method, args)
    }

    /// Initialize all enabled plugins
    pub fn initialize_all(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        for plugin in self.plugins.values_mut() {
            if plugin.is_enabled() {
                plugin.initialize(ctx)?;
            }
        }
        Ok(())
    }

    /// Shutdown all plugins
    pub fn shutdown_all(&mut self) {
        for plugin in self.plugins.values_mut() {
            plugin.shutdown();
        }
    }

    /// Get list of all loaded plugin IDs
    pub fn loaded_plugins(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Get all plugin info
    pub fn all_plugins(&self) -> Vec<PluginInfo> {
        // Would need to track info separately or reconstruct from plugins
        Vec::new()
    }
}

/// Thread-safe plugin registry wrapper
pub struct ThreadSafePluginRegistry {
    inner: Arc<RwLock<PluginRegistry>>,
}

impl ThreadSafePluginRegistry {
    pub fn new(config: PluginRegistryConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(PluginRegistry::new(config))),
        }
    }

    pub fn read(&self) -> std::sync::RwLockReadGuard<PluginRegistry> {
        self.inner.read().unwrap()
    }

    pub fn write(&self) -> std::sync::RwLockWriteGuard<PluginRegistry> {
        self.inner.write().unwrap()
    }
}
```

## JavaScript Bridge (utm-hybrid-bridge)

```rust
// utm-hybrid-bridge/src/bridge.rs
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;
use tokio::sync::RwLock;
use utm_hybrid_core::{BridgeError, BridgeResult, PluginCapability};
use utm_hybrid_plugin::ThreadSafePluginRegistry;

/// JavaScript bridge for native <-> web communication
pub struct JavaScriptBridge {
    /// Registered bridge methods
    methods: HashMap<String, BridgeMethod>,
    /// Event handlers
    event_handlers: HashMap<String, Vec<Arc<dyn Fn(Value) + Send + Sync>>>,
    /// Plugin registry
    plugin_registry: Arc<ThreadSafePluginRegistry>,
    /// WebView handle (trait object)
    webview: Arc<dyn WebView>,
}

/// WebView trait for bridge integration
pub trait WebView: Send + Sync {
    /// Evaluate JavaScript in the WebView
    fn eval(&self, js: &str) -> BridgeResult<()>;

    /// Get WebView URL
    fn url(&self) -> Option<String>;
}

/// Bridge method handler
struct BridgeMethod {
    name: String,
    handler: Arc<dyn Fn(Value) -> BridgeResult<Value> + Send + Sync>,
    requires_permission: Option<PluginCapability>,
}

impl JavaScriptBridge {
    /// Create a new JavaScript bridge
    pub fn new(
        webview: Arc<dyn WebView>,
        plugin_registry: Arc<ThreadSafePluginRegistry>,
    ) -> Self {
        let mut bridge = Self {
            methods: HashMap::new(),
            event_handlers: HashMap::new(),
            plugin_registry,
            webview,
        };

        // Register built-in methods
        bridge.register_builtins();

        bridge
    }

    /// Register built-in bridge methods
    fn register_builtins(&mut self) {
        // Plugin invocation
        self.register_method(
            "invoke",
            Arc::new(|args| self.handle_invoke(args)),
            None,
        );

        // Event subscription
        self.register_method(
            "subscribe",
            Arc::new(|args| self.handle_subscribe(args)),
            None,
        );

        // Event unsubscription
        self.register_method(
            "unsubscribe",
            Arc::new(|args| self.handle_unsubscribe(args)),
            None,
        );

        // Permission request
        self.register_method(
            "requestPermission",
            Arc::new(|args| self.handle_permission_request(args)),
            None,
        );

        // Get bridge info
        self.register_method(
            "getInfo",
            Arc::new(|_| self.get_bridge_info()),
            None,
        );
    }

    /// Register a bridge method
    pub fn register_method(
        &mut self,
        name: &str,
        handler: Arc<dyn Fn(Value) -> BridgeResult<Value> + Send + Sync>,
        permission: Option<PluginCapability>,
    ) {
        self.methods.insert(
            name.to_string(),
            BridgeMethod {
                name: name.to_string(),
                handler,
                requires_permission: permission,
            },
        );
    }

    /// Handle method call from JavaScript
    pub async fn handle_call(&self, method: &str, args: Value, callback_id: &str) {
        let result = if let Some(bridge_method) = self.methods.get(method) {
            // Execute handler
            match (bridge_method.handler)(args) {
                Ok(value) => Ok(value),
                Err(e) => Err(e),
            }
        } else {
            Err(BridgeError::MethodNotFound(method.to_string()))
        };

        // Send result back to JavaScript
        self.send_callback(callback_id, result).await;
    }

    /// Emit event to JavaScript
    pub async fn emit_event(&self, event_name: &str, data: Value) {
        let js_code = format!(
            "window.nativeAPI.emitEvent('{}', {})",
            event_name,
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string())
        );

        let _ = self.webview.eval(&js_code);
    }

    /// Send callback result to JavaScript
    async fn send_callback(&self, callback_id: &str, result: BridgeResult<Value>) {
        let (success, value, error) = match result {
            Ok(v) => (true, Some(v), None),
            Err(e) => (false, None, Some(e.to_string())),
        };

        let js_code = format!(
            "window.nativeAPI.handleCallback('{}', {}, {}, {})",
            callback_id,
            success,
            serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()),
            serde_json::to_string(&error).unwrap_or_else(|_| "null".to_string())
        );

        let _ = self.webview.eval(&js_code);
    }

    /// Handle plugin invoke
    fn handle_invoke(&self, args: Value) -> BridgeResult<Value> {
        #[derive(serde::Deserialize)]
        struct InvokeArgs {
            plugin: String,
            method: String,
            args: Value,
        }

        let invoke_args: InvokeArgs = serde_json::from_value(args)
            .map_err(|e| BridgeError::SerializationError(e.to_string()))?;

        let registry = self.plugin_registry.read();
        let result = registry.call_plugin(
            &invoke_args.plugin,
            &invoke_args.method,
            invoke_args.args,
        );

        match result {
            Ok(v) => Ok(v),
            Err(e) => Err(BridgeError::ExecutionFailed(e.to_string())),
        }
    }

    /// Handle event subscription
    fn handle_subscribe(&self, args: Value) -> BridgeResult<Value> {
        #[derive(serde::Deserialize)]
        struct SubscribeArgs {
            event: String,
        }

        let sub_args: SubscribeArgs = serde_json::from_value(args)
            .map_err(|e| BridgeError::SerializationError(e.to_string()))?;

        Ok(serde_json::json!({
            "subscribed": true,
            "event": sub_args.event,
        }))
    }

    /// Handle event unsubscription
    fn handle_unsubscribe(&self, args: Value) -> BridgeResult<Value> {
        Ok(serde_json::json!({ "unsubscribed": true }))
    }

    /// Handle permission request
    fn handle_permission_request(&self, args: Value) -> BridgeResult<Value> {
        #[derive(serde::Deserialize)]
        struct PermissionArgs {
            permission: String,
        }

        let perm_args: PermissionArgs = serde_json::from_value(args)
            .map_err(|e| BridgeError::SerializationError(e.to_string()))?;

        // In production, this would show a permission dialog
        Ok(serde_json::json!({
            "granted": true,
            "permission": perm_args.permission,
        }))
    }

    /// Get bridge information
    fn get_bridge_info(&self) -> BridgeResult<Value> {
        Ok(serde_json::json!({
            "version": "1.0.0",
            "methods": self.methods.keys().collect::<Vec<_>>(),
            "platform": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        }))
    }
}
```

## WASM Plugin Loading (utm-hybrid-wasm)

```rust
// utm-hybrid-wasm/src/loader.rs
use std::path::Path;
use std::sync::Arc;
use wasmtime::{Engine, Module, Store, Instance, Func, Memory, Config};
use utm_hybrid_core::{NativePlugin, PluginContext, PluginCapability, PluginError, PluginResult};
use utm_hybrid_plugin::PluginManifest;
use crate::runtime::WasmPlugin;

/// WASM plugin loader
pub struct WasmPluginLoader {
    engine: Engine,
    linker: wasmtime::Linker<WasmPluginState>,
}

/// WASM plugin state
pub struct WasmPluginState {
    memory: Option<Memory>,
    plugin_id: String,
    logger: utm_hybrid_core::PluginLogger,
    host_api: Arc<dyn utm_hybrid_core::HostApi>,
}

impl WasmPluginLoader {
    /// Create a new WASM plugin loader
    pub fn new() -> PluginResult<Self> {
        let mut config = Config::new();
        config.wasm_reference_types(true);
        config.wasm_multi_value(true);

        let engine = Engine::new(&config)
            .map_err(|e| PluginError::WasmError(e.to_string()))?;

        let mut linker = wasmtime::Linker::new(&engine);

        // Register host functions
        linker.func_wrap("env", "log", |caller: wasmtime::Caller<'_, WasmPluginState>, level: i32, ptr: i32, len: i32| {
            let state = caller.data();
            if let Some(memory) = &state.memory {
                let mut buf = vec![0u8; len as usize];
                if let Ok(_) = memory.read(&caller, ptr as usize, &mut buf) {
                    if let Ok(msg) = String::from_utf8(buf) {
                        match level {
                            0 => state.logger.debug(&msg),
                            1 => state.logger.info(&msg),
                            2 => state.logger.warn(&msg),
                            3 => state.logger.error(&msg),
                            _ => {}
                        }
                    }
                }
            }
        });

        linker.func_wrap("env", "fs_read", |caller: wasmtime::Caller<'_, WasmPluginState>, path_ptr: i32, path_len: i32, buf_ptr: i32, buf_len: i32| -> i64 {
            // Implement file read through host API
            let state = caller.data();
            let fs = state.host_api.fs();

            // Read path from WASM memory
            // ... implementation would read path, call fs.read, write result

            -1i64 // Error for now
        });

        Ok(Self { engine, linker })
    }

    /// Load a WASM plugin
    pub fn load_plugin(
        &self,
        wasm_path: &Path,
        manifest: &PluginManifest,
        host_api: Arc<dyn utm_hybrid_core::HostApi>,
    ) -> PluginResult<Box<dyn NativePlugin>> {
        // Read WASM bytes
        let wasm_bytes = std::fs::read(wasm_path)
            .map_err(|e| PluginError::LoadFailed(
                format!("Failed to read WASM file: {}", e)
            ))?;

        // Compile module
        let module = Module::from_binary(&self.engine, &wasm_bytes)
            .map_err(|e| PluginError::WasmError(e.to_string()))?;

        // Create plugin state
        let state = WasmPluginState {
            memory: None,
            plugin_id: manifest.id.clone(),
            logger: utm_hybrid_core::PluginLogger::new(&format!("wasm:{}", manifest.id)),
            host_api,
        };

        let mut store = Store::new(&self.engine, state);

        // Instantiate module
        let instance = self.linker.instantiate(&mut store, &module)
            .map_err(|e| PluginError::WasmError(e.to_string()))?;

        // Get exported memory
        let memory = instance.get_memory(&mut store, "memory")
            .cloned();

        store.data_mut().memory = memory;

        // Create WASM plugin wrapper
        let plugin = WasmPlugin {
            instance,
            store,
            manifest: manifest.clone(),
        };

        Ok(Box::new(plugin))
    }
}
```

## IPC Mechanisms (utm-hybrid-ipc)

```rust
// utm-hybrid-ipc/src/channel.rs
use tokio::sync::{mpsc, broadcast};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utm_hybrid_core::{IpcError, IpcResult};

/// Inter-process communication channel
pub struct IpcChannel {
    id: String,
    sender: mpsc::Sender<IpcMessage>,
    receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<IpcMessage>>>,
    broadcast_tx: broadcast::Sender<IpcEvent>,
}

/// IPC message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub reply_to: Option<String>,
    pub timestamp: u64,
}

/// IPC event for broadcasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEvent {
    pub event_type: String,
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

impl IpcChannel {
    /// Create a new IPC channel
    pub fn new(id: &str) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let (broadcast_tx, _) = broadcast::channel(100);

        Self {
            id: id.to_string(),
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
            broadcast_tx,
        }
    }

    /// Send message to specific target
    pub async fn send(&self, to: &str, message_type: &str, payload: serde_json::Value) -> IpcResult<()> {
        let message = IpcMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: self.id.clone(),
            to: to.to_string(),
            message_type: message_type.to_string(),
            payload,
            reply_to: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        self.sender.send(message).await
            .map_err(|e| IpcError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// Send request and wait for response
    pub async fn request(
        &self,
        to: &str,
        message_type: &str,
        payload: serde_json::Value,
        timeout_ms: u64,
    ) -> IpcResult<serde_json::Value> {
        use tokio::time::{timeout, Duration};

        let request_id = uuid::Uuid::new_v4().to_string();

        let message = IpcMessage {
            id: request_id.clone(),
            from: self.id.clone(),
            to: to.to_string(),
            message_type: message_type.to_string(),
            payload,
            reply_to: Some(request_id.clone()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        // Create one-shot receiver for this specific request
        let mut rx = self.subscribe();

        self.sender.send(message).await
            .map_err(|e| IpcError::SendFailed(e.to_string()))?;

        // Wait for response with timeout
        timeout(Duration::from_millis(timeout_ms), async {
            while let Ok(event) = rx.recv().await {
                // Check if this is our response
                if event.event_type == "response" && event.source == to {
                    if let Ok(data) = event.data {
                        return Ok(data);
                    }
                }
            }
            Err(IpcError::ChannelClosed)
        })
        .await
        .map_err(|_| IpcError::Timeout)?
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<IpcEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Broadcast event to all subscribers
    pub fn broadcast(&self, event_type: &str, source: &str, data: serde_json::Value) -> IpcResult<()> {
        let event = IpcEvent {
            event_type: event_type.to_string(),
            source: source.to_string(),
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        self.broadcast_tx.send(event)
            .map_err(|e| IpcError::SendFailed(e.to_string()))?;

        Ok(())
    }

    /// Receive next message
    pub async fn receive(&self) -> Option<IpcMessage> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    /// Get channel ID
    pub fn id(&self) -> &str {
        &self.id
    }
}
```

## State Synchronization (utm-hybrid-state)

```rust
// utm-hybrid-state/src/sync.rs
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};
use utm_hybrid_core::{StateError, StateResult};

/// State synchronization manager
pub struct StateSync {
    state: Arc<RwLock<AppState>>,
    history: Arc<RwLock<StateHistory>>,
    tx: broadcast::Sender<StateUpdate>,
}

/// Application state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    /// Build state
    pub build: BuildState,

    /// Project state
    pub project: ProjectState,

    /// Settings
    pub settings: SettingsState,

    /// UI state
    pub ui: UiState,
}

/// Build state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildState {
    pub status: String,
    pub current_build_id: Option<String>,
    pub progress: u8,
    pub logs: Vec<String>,
    pub last_duration_ms: Option<u64>,
}

/// Project state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectState {
    pub current_project: Option<String>,
    pub recent_projects: Vec<String>,
    pub unsaved_changes: bool,
}

/// Settings state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingsState {
    pub theme: String,
    pub build_target: String,
    pub build_profile: String,
    pub features: Vec<String>,
}

/// UI state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiState {
    pub sidebar_open: bool,
    pub active_panel: String,
    pub notifications: Vec<Notification>,
}

/// Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub message: String,
    pub level: String,
    pub timestamp: u64,
}

/// State update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdate {
    pub path: String,
    pub value: serde_json::Value,
    pub source: String,
    pub timestamp: u64,
}

/// State history for undo/redo
struct StateHistory {
    snapshots: Vec<(u64, AppState)>,
    max_history: usize,
}

impl StateSync {
    /// Create a new state sync manager
    pub fn new(initial_state: AppState) -> Self {
        let (tx, _rx) = broadcast::channel(100);

        Self {
            state: Arc::new(RwLock::new(initial_state)),
            history: Arc::new(RwLock::new(StateHistory {
                snapshots: Vec::new(),
                max_history: 100,
            })),
            tx,
        }
    }

    /// Get current state
    pub fn get_state(&self) -> AppState {
        self.state.read().unwrap().clone()
    }

    /// Get specific state section
    pub fn get_section<T: serde::de::DeserializeOwned>(&self, section: &str) -> StateResult<T> {
        let state = self.state.read().unwrap();
        let value = match section {
            "build" => serde_json::to_value(&state.build)?,
            "project" => serde_json::to_value(&state.project)?,
            "settings" => serde_json::to_value(&state.settings)?,
            "ui" => serde_json::to_value(&state.ui)?,
            _ => return Err(StateError::InvalidPath(section.to_string())),
        };
        serde_json::from_value(value)
            .map_err(|e| StateError::SerializationError(e))
    }

    /// Update state at path
    pub fn update(&self, path: &str, value: serde_json::Value, source: &str) -> StateResult<()> {
        let mut state = self.state.write().unwrap();

        // Update the appropriate section
        self.set_value_at_path(&mut state, path, value.clone())?;

        // Save to history
        {
            let mut history = self.history.write().unwrap();
            history.snapshots.push((
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
                state.clone(),
            ));

            // Trim history
            if history.snapshots.len() > history.max_history {
                history.snapshots.remove(0);
            }
        }

        // Broadcast update
        let update = StateUpdate {
            path: path.to_string(),
            value,
            source: source.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        };

        self.tx.send(update)
            .map_err(|_| StateError::BroadcastFailed)?;

        Ok(())
    }

    /// Subscribe to state updates
    pub fn subscribe(&self) -> broadcast::Receiver<StateUpdate> {
        self.tx.subscribe()
    }

    /// Undo last state change
    pub fn undo(&self) -> StateResult<()> {
        let mut history = self.history.write().unwrap();

        if history.snapshots.len() < 2 {
            return Err(StateError::NoHistory);
        }

        // Remove current state
        history.snapshots.pop();

        // Get previous state
        let (_, previous) = history.snapshots.last().unwrap().clone();

        let mut state = self.state.write().unwrap();
        *state = previous;

        Ok(())
    }

    /// Set value at path within state
    fn set_value_at_path(&self, state: &mut AppState, path: &str, value: serde_json::Value) -> StateResult<()> {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.is_empty() {
            return Err(StateError::InvalidPath(path.to_string()));
        }

        match parts[0] {
            "build" => self.set_nested_value(&mut state.build, &parts[1..], value),
            "project" => self.set_nested_value(&mut state.project, &parts[1..], value),
            "settings" => self.set_nested_value(&mut state.settings, &parts[1..], value),
            "ui" => self.set_nested_value(&mut state.ui, &parts[1..], value),
            _ => return Err(StateError::InvalidPath(path.to_string())),
        }

        Ok(())
    }

    /// Set nested value using reflection
    fn set_nested_value<T: Serialize + serde::de::DeserializeOwned>(
        &self,
        state: &mut T,
        path: &[&str],
        value: serde_json::Value,
    ) {
        if path.is_empty() {
            if let Ok(new_state) = serde_json::from_value(value) {
                *state = new_state;
            }
            return;
        }

        // For nested paths, would need macro or manual field handling
        // This is a simplified version
    }
}
```

## Code Examples

### Full Plugin Creation Example

```rust
// Example: Creating a custom file system plugin
use utm_hybrid_core::{
    NativePlugin, PluginContext, PluginCapability, PluginResult,
    PluginLogger, HostApi, FileSystemApi,
};
use serde_json::Value;

pub struct MyFileSystemPlugin {
    id: String,
    version: String,
    name: String,
    enabled: bool,
    logger: Option<PluginLogger>,
}

impl MyFileSystemPlugin {
    pub fn new() -> Self {
        Self {
            id: "com.example.filesystem".to_string(),
            version: "1.0.0".to_string(),
            name: "File System".to_string(),
            enabled: true,
            logger: None,
        }
    }
}

impl NativePlugin for MyFileSystemPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Advanced file system operations"
    }

    fn initialize(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        self.logger = Some(ctx.logger.clone());

        if let Some(logger) = &self.logger {
            logger.info("MyFileSystemPlugin initialized");
        }

        Ok(())
    }

    fn shutdown(&mut self) {
        if let Some(logger) = &self.logger {
            logger.info("MyFileSystemPlugin shutting down");
        }
    }

    fn call_method(&self, method: &str, args: Value) -> PluginResult<Value> {
        match method {
            "readFile" => self.read_file(args),
            "writeFile" => self.write_file(args),
            "listDir" => self.list_dir(args),
            _ => Err(utm_hybrid_core::PluginError::MethodNotFound(method.to_string())),
        }
    }

    fn capabilities(&self) -> &[PluginCapability] {
        &[
            PluginCapability::FileSystemRead,
            PluginCapability::FileSystemWrite,
        ]
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl MyFileSystemPlugin {
    fn read_file(&self, args: Value) -> PluginResult<Value> {
        #[derive(serde::Deserialize)]
        struct ReadArgs {
            path: String,
        }

        let args: ReadArgs = serde_json::from_value(args)?;
        let content = std::fs::read_to_string(&args.path)?;

        Ok(serde_json::json!({
            "content": content,
            "path": args.path,
        }))
    }

    fn write_file(&self, args: Value) -> PluginResult<Value> {
        #[derive(serde::Deserialize)]
        struct WriteArgs {
            path: String,
            content: String,
        }

        let args: WriteArgs = serde_json::from_value(args)?;
        std::fs::write(&args.path, args.content)?;

        Ok(serde_json::json!({
            "success": true,
            "path": args.path,
        }))
    }

    fn list_dir(&self, args: Value) -> PluginResult<Value> {
        #[derive(serde::Deserialize)]
        struct ListArgs {
            path: String,
        }

        let args: ListArgs = serde_json::from_value(args)?;
        let entries = std::fs::read_dir(&args.path)?;

        let result: Vec<Value> = entries
            .filter_map(|e| e.ok())
            .map(|e| {
                serde_json::json!({
                    "name": e.file_name().to_string_lossy(),
                    "path": e.path().to_string_lossy(),
                    "is_dir": e.path().is_dir(),
                })
            })
            .collect();

        Ok(serde_json::json!({
            "entries": result,
            "path": args.path,
        }))
    }
}
```

### Using the Plugin System

```rust
use utm_hybrid_plugin::{PluginRegistry, PluginRegistryConfig, PluginManifest};
use utm_hybrid_core::{PluginContext, PluginConfig, PluginLogger};
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create plugin registry
    let config = PluginRegistryConfig::default();
    let mut registry = PluginRegistry::new(config);

    // Add plugin directory
    registry.add_plugin_dir(PathBuf::from("./plugins"));

    // Discover plugins
    let discovered = registry.discover_plugins()?;
    println!("Discovered {} plugins:", discovered.len());
    for plugin in &discovered {
        println!("  - {} v{}", plugin.name, plugin.version);
    }

    // Load a specific plugin
    let manifest_path = PathBuf::from("./plugins/filesystem/plugin.json");
    let info = registry.load_plugin(&manifest_path)?;
    println!("Loaded plugin: {}", info.name);

    // Create plugin context
    let ctx = PluginContext {
        app_data_dir: PathBuf::from("./data"),
        plugin_data_dir: PathBuf::from("./data/plugins/filesystem"),
        config: Arc::new(PluginConfig {
            settings: std::collections::HashMap::new(),
            enabled: true,
        }),
        logger: PluginLogger::new("filesystem"),
        host_api: Arc::new(/* HostApi implementation */),
    };

    // Initialize all plugins
    registry.initialize_all(&ctx)?;

    // Call plugin method
    let result = registry.call_plugin(
        "com.example.filesystem",
        "listDir",
        serde_json::json!({ "path": "." }),
    )?;

    println!("Plugin result: {:?}", result);

    // Shutdown plugins
    registry.shutdown_all();

    Ok(())
}
```

### JavaScript Bridge Integration

```typescript
// TypeScript client for the JavaScript bridge
interface NativeAPI {
  invoke<T>(plugin: string, method: string, args?: any): Promise<T>;
  on(event: string, callback: (data: any) => void): () => void;
  emit(event: string, data?: any): void;
  getInfo(): Promise<BridgeInfo>;
}

interface BridgeInfo {
  version: string;
  methods: string[];
  platform: string;
  arch: string;
}

// Implementation
export function createNativeAPI(): NativeAPI {
  let callbackId = 0;
  const pendingCallbacks = new Map<number, {
    resolve: (value: any) => void;
    reject: (error: Error) => void;
  }>();

  // Set up global callback handler
  (window as any).nativeAPI = {
    handleCallback: (id: number, success: boolean, value: any, error: string | null) => {
      const callback = pendingCallbacks.get(id);
      if (!callback) return;

      pendingCallbacks.delete(id);

      if (success) {
        callback.resolve(value);
      } else {
        callback.reject(new Error(error || 'Unknown error'));
      }
    },
    emitEvent: (eventName: string, data: any) => {
      window.dispatchEvent(new CustomEvent(`native:${eventName}`, { detail: data }));
    },
  };

  async function invoke<T>(plugin: string, method: string, args?: any): Promise<T> {
    return new Promise((resolve, reject) => {
      const id = ++callbackId;
      pendingCallbacks.set(id, { resolve, reject });

      const invokeArgs = { plugin, method, args: args || {} };

      // Use platform-specific bridge
      if ((window as any).__TAURI__) {
        (window as any).__TAURI__.invoke('plugin:invoke', { args: invokeArgs, callbackId: id });
      } else if (window.external?.invoke) {
        window.external.invoke(JSON.stringify({
          type: 'invoke',
          payload: invokeArgs,
          callbackId: id,
        }));
      } else {
        reject(new Error('Native bridge not available'));
      }
    });
  }

  function on(event: string, callback: (data: any) => void): () => void {
    const handler = (e: Event) => {
      const custom = e as CustomEvent;
      callback(custom.detail);
    };

    window.addEventListener(`native:${event}`, handler);

    return () => {
      window.removeEventListener(`native:${event}`, handler);
    };
  }

  function emit(event: string, data?: any) {
    window.dispatchEvent(new CustomEvent(`emit:${event}`, { detail: data }));
  }

  async function getInfo(): Promise<BridgeInfo> {
    return invoke('bridge', 'getInfo');
  }

  return { invoke, on, emit, getInfo };
}

// Usage example
const nativeAPI = createNativeAPI();

// Call file system plugin
const files = await nativeAPI.invoke('com.example.filesystem', 'listDir', { path: '.' });
console.log('Files:', files);

// Subscribe to events
const unsubscribe = nativeAPI.on('build:progress', (data) => {
  console.log('Build progress:', data.progress);
});
```

## Migration Path

### Phase 1: Core Infrastructure (Week 1-2)
- Implement `utm-hybrid-core` crate with traits and types
- Set up workspace structure
- Define error types and capabilities

### Phase 2: Plugin System (Week 3-4)
- Implement plugin registry and manifest loading
- Create native plugin loader with libloading
- Build example plugins

### Phase 3: WASM Support (Week 5-6)
- Integrate wasmtime runtime
- Implement WASM plugin wrapper
- Set up sandboxing

### Phase 4: JavaScript Bridge (Week 7)
- Implement bridge core with method registration
- Add event handling
- Create TypeScript client library

### Phase 5: IPC & State (Week 8)
- Implement IPC channels
- Build state synchronization
- Add WebView IPC protocol

### Phase 6: Marketplace (Week 9-10)
- Create marketplace client
- Implement plugin download and verification
- Build CLI tool

## Performance Considerations

### Plugin Initialization

Lazy-load plugins to minimize startup time:

```rust
pub struct LazyPlugin {
    manifest: PluginManifest,
    instance: OnceCell<Box<dyn NativePlugin>>,
}

impl LazyPlugin {
    pub fn get_or_try_init(&self, ctx: &PluginContext) -> PluginResult<&dyn NativePlugin> {
        self.instance.get_or_try_init(|| {
            self.loader.load(&self.manifest, ctx)
        })
    }
}
```

### Method Call Caching

Cache frequently called plugin method results:

```rust
use moka::future::Cache;

pub struct CachedPlugin {
    inner: Box<dyn NativePlugin>,
    cache: Cache<String, Value>,
}

impl CachedPlugin {
    pub async fn call_cached(
        &self,
        method: &str,
        args: Value,
    ) -> PluginResult<Value> {
        let key = format!("{}:{}", method, serde_json::to_string(&args)?);

        if let Some(cached) = self.cache.get(&key).await {
            return Ok(cached);
        }

        let result = self.inner.call_method(method, args)?;
        self.cache.insert(key, result.clone()).await;
        Ok(result)
    }
}
```

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use utm_hybrid_core::{PluginContext, PluginConfig, PluginLogger};

    struct MockHostApi;
    impl HostApi for MockHostApi {
        fn fs(&self) -> &dyn FileSystemApi { &MockFsApi }
        fn process(&self) -> &dyn ProcessApi { &MockProcessApi }
        fn network(&self) -> Option<&dyn NetworkApi> { None }
        fn ui(&self) -> &dyn UiApi { &MockUiApi }
        fn settings(&self) -> &dyn SettingsApi { &MockSettingsApi }
    }

    struct MockFsApi;
    impl FileSystemApi for MockFsApi {}

    struct MockProcessApi;
    impl ProcessApi for MockProcessApi {}

    struct MockUiApi;
    impl UiApi for MockUiApi {}

    struct MockSettingsApi;
    impl SettingsApi for MockSettingsApi {}

    #[test]
    fn test_plugin_manifest_validation() {
        let manifest = PluginManifest {
            id: "com.example.test".to_string(),
            version: "1.0.0".to_string(),
            name: "Test Plugin".to_string(),
            description: None,
            author: None,
            plugin_type: "native".to_string(),
            entry_point: "libtest.dylib".to_string(),
            enabled: true,
            capabilities: vec!["fs:read".to_string()],
            min_host_version: None,
            dependencies: vec![],
            config_schema: None,
            homepage: None,
            repository: None,
            license: None,
        };

        assert!(manifest.validate().is_ok());
    }

    #[tokio::test]
    async fn test_plugin_registry_load() {
        let config = PluginRegistryConfig::default();
        let mut registry = PluginRegistry::new(config);

        // Would test actual plugin loading here
    }
}
```

## Open Considerations

1. **Plugin Hot-Reloading**: Support reloading plugins without restart for development

2. **Plugin Dependencies**: Implement dependency resolution between plugins

3. **Cross-Platform Plugin Distribution**: Standardize plugin packaging format

4. **Plugin Sandboxing**: Enhanced security with seccomp-bpf on Linux, Seatbelt on macOS

5. **Plugin Performance Monitoring**: Track plugin resource usage and execution time

6. **WebAssembly Component Model**: Future migration to WASI components for better interoperability

7. **Multi-Process Plugins**: Run untrusted plugins in separate processes for isolation

8. **Plugin Update Mechanism**: Automatic update checking and installation

9. **Plugin Configuration UI**: Standardized settings UI generation from JSON Schema

10. **Plugin Analytics**: Opt-in usage statistics for marketplace plugins
