---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/multi-engine-microvm/
revised_at: 2026-03-19
repository: Proposed new implementation
language: Rust
---

# Multi-Engine MicroVM Framework - Rust Revision

## Overview

This document provides a detailed Rust implementation plan for the unified multi-engine microvm framework combining Firecracker, libkrun, and QEMU/UTM backends.

## Workspace Structure

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/microvm-core",
    "crates/engine-firecracker",
    "crates/engine-libkrun",
    "crates/engine-qemu",
    "crates/platform-kvm",
    "crates/platform-hvf",
    "crates/devices-virtio",
    "crates/networking",
    "crates/microvm-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Apache-2.0 OR MIT"
rust-version = "1.75"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Platform-specific
nix = { version = "0.27", features = ["fs", "socket", "sched"] }
libc = "0.2"

# KVM bindings
kvm-bindings = "0.8"
kvm-ioctls = "0.16"

# CLI
clap = { version = "4.4", features = ["derive"] }

# UUID
uuid = { version = "1.6", features = ["v4"] }

# Temp directories
tempfile = "3.9"
```

## Crate 1: microvm-core

```rust
// crates/microvm-core/src/lib.rs
pub mod vm;
pub mod engine;
pub mod config;
pub mod lifecycle;
pub mod error;
pub mod capabilities;

pub use vm::*;
pub use engine::*;
pub use config::*;
pub use lifecycle::*;
pub use error::*;
pub use capabilities::*;
```

```rust
// crates/microvm-core/src/vm.rs
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use crate::{VMState, VMConfig, Result};

/// Core VM trait abstracting lifecycle and operations
#[async_trait]
pub trait VM: Send + Sync {
    /// Unique identifier for this VM
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Current VM state
    fn state(&self) -> VMState;

    /// Get VM configuration
    fn config(&self) -> &VMConfig;

    /// Start the VM
    async fn start(&mut self) -> Result<()>;

    /// Stop the VM gracefully
    async fn stop(&mut self) -> Result<()>;

    /// Force stop the VM
    async fn kill(&mut self) -> Result<()>;

    /// Pause VM execution
    async fn pause(&mut self) -> Result<()>;

    /// Resume from paused state
    async fn resume(&mut self) -> Result<()>;

    /// Check if VM is running
    fn is_running(&self) -> bool {
        matches!(self.state(), VMState::Running)
    }

    /// Get VM exit code (if exited)
    fn exit_code(&self) -> Option<i32>;

    /// Get VM PID (if running)
    fn pid(&self) -> Option<u32>;

    /// Get engine name that manages this VM
    fn engine_name(&self) -> &str;
}

/// VM information for listing
#[derive(Debug, Clone)]
pub struct VMInfo {
    pub id: String,
    pub name: String,
    pub state: VMState,
    pub engine: String,
    pub vcpus: u8,
    pub memory_mib: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

```rust
// crates/microvm-core/src/engine.rs
use async_trait::async_trait;
use std::path::Path;
use crate::{VM, VMConfig, Result, EngineCapabilities, VMInfo, Platform};

/// Engine trait for backend implementations
#[async_trait]
pub trait Engine: Send + Sync {
    /// Engine name (firecracker, libkrun, qemu)
    fn name(&self) -> &'static str;

    /// Engine version
    fn version(&self) -> &str;

    /// Check if engine is available on this platform
    fn is_available() -> Result<bool>
    where
        Self: Sized;

    /// Get engine capabilities
    fn capabilities(&self) -> EngineCapabilities;

    /// Create a new VM instance
    async fn create_vm(&self, config: VMConfig) -> Result<Box<dyn VM>>;

    /// Load existing VM from state
    async fn load_vm(&self, id: &str) -> Result<Box<dyn VM>> {
        Err(crate::Error::NotImplemented("load_vm".into()))
    }

    /// List all VMs managed by this engine
    async fn list_vms(&self) -> Result<Vec<VMInfo>> {
        Ok(Vec::new())
    }

    /// Remove a VM
    async fn remove_vm(&self, _id: &str) -> Result<()> {
        Err(crate::Error::NotImplemented("remove_vm".into()))
    }

    /// Create a snapshot (engine-specific)
    async fn snapshot(&self, _vm_id: &str, _path: &Path) -> Result<()> {
        Err(crate::Error::NotImplemented("snapshot".into()))
    }

    /// Restore from snapshot (engine-specific)
    async fn restore(&self, _snapshot_path: &Path) -> Result<Box<dyn VM>> {
        Err(crate::Error::NotImplemented("restore".into()))
    }
}

/// Engine options for VM configuration
#[derive(Debug, Clone, Default)]
pub struct EngineOptions {
    /// Require fast boot (< 500ms)
    pub require_fast_boot: bool,
    /// Require TSI networking
    pub require_tsi: bool,
    /// Require snapshot support
    pub require_snapshots: bool,
    /// Require confidential computing
    pub require_confidential: bool,
    /// Require GPU support
    pub require_gpu: bool,
    /// Prefer minimal memory footprint
    pub prefer_minimal: bool,
    /// Engine-specific options
    pub extra: std::collections::HashMap<String, String>,
}
```

```rust
// crates/microvm-core/src/config.rs
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::{Result, Error, NetworkType, DiskInterface};

/// VM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMConfig {
    pub id: String,
    pub name: String,
    pub vcpus: u8,
    pub memory_mib: u32,
    pub boot: BootConfig,
    pub disks: Vec<DiskConfig>,
    pub networks: Vec<NetworkConfig>,
    pub fs_mounts: Vec<FsMountConfig>,
    pub engine_options: EngineOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootConfig {
    pub kernel: Option<PathBuf>,
    pub initramfs: Option<PathBuf>,
    pub cmdline: String,
    pub root_disk_id: Option<String>,
    pub efi_boot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskConfig {
    pub id: String,
    pub path: PathBuf,
    pub is_root: bool,
    pub read_only: bool,
    pub interface: DiskInterface,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub id: String,
    pub net_type: NetworkType,
    pub mac_address: Option<String>,
    pub host_device: Option<String>,
    /// TSI port mappings: (guest_port, host_port)
    pub tsi_ports: Vec<(u16, u16)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsMountConfig {
    pub tag: String,
    pub path: PathBuf,
    pub read_only: bool,
}

/// Builder for VMConfig
pub struct VMConfigBuilder {
    id: String,
    name: Option<String>,
    vcpus: u8,
    memory_mib: u32,
    boot: BootConfig,
    disks: Vec<DiskConfig>,
    networks: Vec<NetworkConfig>,
    fs_mounts: Vec<FsMountConfig>,
    engine_options: EngineOptions,
}

impl VMConfigBuilder {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            vcpus: 1,
            memory_mib: 512,
            boot: BootConfig {
                kernel: None,
                initramfs: None,
                cmdline: String::new(),
                root_disk_id: None,
                efi_boot: false,
            },
            disks: Vec::new(),
            networks: Vec::new(),
            fs_mounts: Vec::new(),
            engine_options: EngineOptions::default(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn vcpus(mut self, count: u8) -> Self {
        self.vcpus = count;
        self
    }

    pub fn memory(mut self, mib: u32) -> Self {
        self.memory_mib = mib;
        self
    }

    pub fn kernel(mut self, path: impl Into<PathBuf>) -> Self {
        self.boot.kernel = Some(path.into());
        self
    }

    pub fn initramfs(mut self, path: impl Into<PathBuf>) -> Self {
        self.boot.initramfs = Some(path.into());
        self
    }

    pub fn cmdline(mut self, cmdline: impl Into<String>) -> Self {
        self.boot.cmdline = cmdline.into();
        self
    }

    pub fn rootfs(mut self, path: impl Into<PathBuf>) -> Self {
        self.boot.root_disk_id = Some(format!("rootfs-{}", self.disks.len()));
        self.disks.push(DiskConfig {
            id: format!("rootfs-{}", self.disks.len()),
            path: path.into(),
            is_root: true,
            read_only: false,
            interface: DiskInterface::Virtio,
        });
        self
    }

    pub fn disk(mut self, config: DiskConfig) -> Self {
        self.disks.push(config);
        self
    }

    pub fn network(mut self, config: NetworkConfig) -> Self {
        self.networks.push(config);
        self
    }

    pub fn tsi_network(mut self, port_mappings: Vec<(u16, u16)>) -> Self {
        self.engine_options.require_tsi = true;
        self.networks.push(NetworkConfig {
            id: format!("tsi-{}", self.networks.len()),
            net_type: NetworkType::Tsi,
            mac_address: None,
            host_device: None,
            tsi_ports: port_mappings,
        });
        self
    }

    pub fn fs_mount(mut self, tag: impl Into<String>, path: impl Into<PathBuf>, read_only: bool) -> Self {
        self.fs_mounts.push(FsMountConfig {
            tag: tag.into(),
            path: path.into(),
            read_only,
        });
        self
    }

    pub fn require_fast_boot(mut self, val: bool) -> Self {
        self.engine_options.require_fast_boot = val;
        self
    }

    pub fn require_snapshots(mut self, val: bool) -> Self {
        self.engine_options.require_snapshots = val;
        self
    }

    pub fn build(self) -> Result<VMConfig> {
        let name = self.name.unwrap_or_else(|| self.id.clone());

        // Validate configuration
        if self.boot.kernel.is_none() && !self.boot.efi_boot {
            return Err(Error::Config("kernel or efi_boot must be set".into()));
        }

        if self.disks.is_empty() {
            return Err(Error::Config("at least one disk must be configured".into()));
        }

        Ok(VMConfig {
            id: self.id,
            name,
            vcpus: self.vcpus,
            memory_mib: self.memory_mib,
            boot: self.boot,
            disks: self.disks,
            networks: self.networks,
            fs_mounts: self.fs_mounts,
            engine_options: self.engine_options,
        })
    }
}
```

```rust
// crates/microvm-core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("VM not found: {0}")]
    VmNotFound(String),

    #[error("Engine not found: {0}")]
    EngineNotFound(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process error: {0}")]
    Process(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Platform error: {0}")]
    Platform(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

```rust
// crates/microvm-core/src/lifecycle.rs
use serde::{Deserialize, Serialize};

/// VM lifecycle states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VMState {
    Created,
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Snapshotting,
    Restoring,
    Error(String),
}

/// VM lifecycle events
#[derive(Debug, Clone)]
pub enum VMEvent {
    Created,
    Starting,
    Started,
    Stopping,
    Stopped,
    Paused,
    Resumed,
    SnapshotCreated { path: String },
    Restored { path: String },
    Error(String),
}
```

```rust
// crates/microvm-core/src/capabilities.rs
use crate::Platform;

/// Engine feature capabilities
#[derive(Debug, Clone)]
pub struct EngineCapabilities {
    /// Fast boot (< 500ms)
    pub fast_boot: bool,
    /// Snapshot support
    pub snapshots: bool,
    /// Diff snapshots
    pub diff_snapshots: bool,
    /// Live migration
    pub live_migration: bool,
    /// Memory ballooning
    pub memory_ballooning: bool,
    /// GPU passthrough
    pub gpu_passthrough: bool,
    /// virtio-fs support
    pub virtio_fs: bool,
    /// TSI networking
    pub tsi_networking: bool,
    /// Confidential computing (SEV/TDX)
    pub confidential: bool,
    /// Platform support
    pub platforms: Vec<Platform>,
}

impl EngineCapabilities {
    /// Check if engine supports the current platform
    pub fn supports_current_platform(&self) -> bool {
        let current = Platform::current();
        self.platforms.contains(&current)
    }
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                Platform::MacOSAArch64
            } else {
                Platform::MacOSX86_64
            }
        } else if cfg!(target_os = "linux") {
            if cfg!(target_arch = "aarch64") {
                Platform::LinuxAArch64
            } else {
                Platform::LinuxX86_64
            }
        } else {
            panic!("Unsupported platform");
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    LinuxX86_64,
    LinuxAArch64,
    MacOSAArch64,
    MacOSX86_64,
}
```

## Crate 2: engine-libkrun

```rust
// crates/engine-libkrun/src/lib.rs
mod engine;
mod context;
mod tsi;
mod devices;

pub use engine::LibkrunEngine;
pub use context::LibkrunContext;
pub use tsi::TsiProxy;

// FFI bindings to libkrun
#[link(name = "krun")]
extern "C" {
    fn krun_create_ctx() -> i32;
    fn krun_free_ctx(ctx_id: u32) -> i32;
    fn krun_set_vm_config(ctx_id: u32, num_vcpus: u8, ram_mib: u32) -> i32;
    fn krun_set_root(ctx_id: u32, root_path: *const libc::c_char) -> i32;
    fn krun_set_exec(
        ctx_id: u32,
        exec_path: *const libc::c_char,
        argv: *const *const libc::c_char,
        envp: *const *const libc::c_char,
    ) -> i32;
    fn krun_start_enter(ctx_id: u32) -> i32;
    fn krun_add_virtiofs(
        ctx_id: u32,
        tag: *const libc::c_char,
        path: *const libc::c_char,
    ) -> i32;
    fn krun_add_tsi_port(
        ctx_id: u32,
        guest_port: u16,
        host_port: u16,
    ) -> i32;
}
```

```rust
// crates/engine-libkrun/src/engine.rs
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use microvm_core::*;
use crate::{LibkrunContext, TsiProxy};

pub struct LibkrunEngine {
    lib_path: Option<PathBuf>,
    storage_root: PathBuf,
}

impl LibkrunEngine {
    pub fn new() -> Result<Self> {
        let storage_root = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("microvm/libkrun");

        std::fs::create_dir_all(&storage_root)?;

        Ok(Self {
            lib_path: None,  // Use system libkrun or bundled
            storage_root,
        })
    }
}

#[async_trait]
impl Engine for LibkrunEngine {
    fn name(&self) -> &'static str {
        "libkrun"
    }

    fn version(&self) -> &str {
        "1.0.0"  // Would query actual version
    }

    fn is_available() -> Result<bool> {
        // Check for libkrun.so or bundled library
        // On macOS, always available (can bundle statically)
        // On Linux, check for /dev/kvm
        #[cfg(target_os = "linux")]
        {
            if !std::path::Path::new("/dev/kvm").exists() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            fast_boot: true,
            snapshots: false,
            diff_snapshots: false,
            live_migration: false,
            memory_ballooning: true,
            gpu_passthrough: true,
            virtio_fs: true,
            tsi_networking: true,
            confidential: true,
            platforms: vec![
                Platform::LinuxX86_64,
                Platform::LinuxAArch64,
                Platform::MacOSAArch64,
            ],
        }
    }

    async fn create_vm(&self, config: VMConfig) -> Result<Box<dyn VM>> {
        let ctx = LibkrunContext::create()?;

        // Configure VM
        ctx.set_vm_config(config.vcpus, config.memory_mib)?;

        // Set root filesystem
        if let Some(root_disk) = config.disks.iter().find(|d| d.is_root) {
            let root_cstr = std::ffi::CString::new(
                root_disk.path.to_string_lossy().as_ref()
            ).map_err(|_| Error::Config("invalid rootfs path".into()))?;

            unsafe {
                let ret = krun_set_root(ctx.ctx_id(), root_cstr.as_ptr());
                if ret < 0 {
                    return Err(Error::Process("failed to set rootfs".into()));
                }
            }
        }

        // Configure TSI networking
        let mut tsi_proxy = None;
        for net in &config.networks {
            if net.net_type == NetworkType::Tsi && !net.tsi_ports.is_empty() {
                let mut proxy = TsiProxy::new()?;
                for (guest_port, host_port) in &net.tsi_ports {
                    proxy.add_tcp_mapping(*guest_port, *host_port)?;
                    unsafe {
                        let ret = krun_add_tsi_port(ctx.ctx_id(), *guest_port, *host_port);
                        if ret < 0 {
                            return Err(Error::Process("failed to add TSI port".into()));
                        }
                    }
                }
                tsi_proxy = Some(proxy);
            }
        }

        // Configure virtio-fs mounts
        for mount in &config.fs_mounts {
            let tag_cstr = std::ffi::CString::new(mount.tag.as_str())
                .map_err(|_| Error::Config("invalid mount tag".into()))?;
            let path_cstr = std::ffi::CString::new(
                mount.path.to_string_lossy().as_ref()
            ).map_err(|_| Error::Config("invalid mount path".into()))?;

            unsafe {
                let ret = krun_add_virtiofs(ctx.ctx_id(), tag_cstr.as_ptr(), path_cstr.as_ptr());
                if ret < 0 {
                    tracing::warn!("failed to add virtio-fs mount: {}", mount.tag);
                }
            }
        }

        // Set executable
        if !config.boot.cmdline.is_empty() {
            // For libkrun, cmdline is used for the init process
            let cmdline_cstr = std::ffi::CString::new(config.boot.cmdline.as_str())
                .map_err(|_| Error::Config("invalid cmdline".into()))?;

            unsafe {
                let ret = krun_set_exec(ctx.ctx_id(), cmdline_cstr.as_ptr(), std::ptr::null(), std::ptr::null());
                if ret < 0 {
                    tracing::warn!("failed to set exec");
                }
            }
        }

        Ok(Box::new(LibkrunVM {
            id: config.id.clone(),
            name: config.name.clone(),
            config,
            ctx,
            tsi_proxy,
            state: VMState::Created,
            exit_code: None,
        }))
    }
}

pub struct LibkrunVM {
    id: String,
    name: String,
    config: VMConfig,
    ctx: LibkrunContext,
    tsi_proxy: Option<TsiProxy>,
    state: VMState,
    exit_code: Option<i32>,
}

#[async_trait]
impl VM for LibkrunVM {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn state(&self) -> VMState {
        self.state.clone()
    }

    fn config(&self) -> &VMConfig {
        &self.config
    }

    async fn start(&mut self) -> Result<()> {
        self.state = VMState::Starting;

        // Start TSI proxy if configured
        if let Some(ref mut proxy) = self.tsi_proxy {
            proxy.start()?;
        }

        // Enter VM (blocking call - run in spawned task)
        let ctx = self.ctx.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();

        std::thread::spawn(move || {
            let ret = ctx.enter();
            let _ = tx.send(ret);
        });

        self.state = VMState::Running;

        // Wait for exit (in real impl, would handle async differently)
        match rx.await {
            Ok(Ok(code)) => {
                self.exit_code = Some(code);
                self.state = VMState::Stopped;
            }
            Ok(Err(e)) => {
                self.state = VMState::Error(e.to_string());
            }
            Err(_) => {
                self.state = VMState::Error("task panicked".into());
            }
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        // libkrun VMs exit when the init process exits
        // No graceful shutdown mechanism available
        self.state = VMState::Stopped;
        Ok(())
    }

    async fn kill(&mut self) -> Result<()> {
        // Force kill not directly supported
        self.state = VMState::Stopped;
        Ok(())
    }

    fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    fn engine_name(&self) -> &str {
        "libkrun"
    }
}
```

```rust
// crates/engine-libkrun/src/tsi.rs
use std::collections::HashMap;
use std::net::{TcpListener, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixListener, UnixStream};
use microvm_core::{Result, Error};

/// TSI (Transparent Socket Impersonation) proxy
pub struct TsiProxy {
    port_mappings: HashMap<u16, u16>,
    tcp_listeners: HashMap<u16, TcpListener>,
    running: bool,
}

impl TsiProxy {
    pub fn new() -> Result<Self> {
        Ok(Self {
            port_mappings: HashMap::new(),
            tcp_listeners: HashMap::new(),
            running: false,
        })
    }

    pub fn add_tcp_mapping(&mut self, guest_port: u16, host_port: u16) -> Result<()> {
        let addr: SocketAddr = format!("127.0.0.1:{}", host_port).parse()
            .map_err(|e| Error::Config(format!("invalid host port: {}", e)))?;

        let listener = TcpListener::bind(addr)
            .map_err(|e| Error::Io(std::io::Error::new(e.kind(), format!("failed to bind port {}: {}", host_port, e))))?;

        listener.set_nonblocking(true)?;

        self.port_mappings.insert(guest_port, host_port);
        self.tcp_listeners.insert(host_port, listener);

        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        self.running = true;
        Ok(())
    }

    /// Run the proxy event loop
    pub async fn run(&mut self) -> Result<()> {
        while self.running {
            // Accept connections from host and forward to guest
            let mut accept_futures = Vec::new();

            for (host_port, listener) in &self.tcp_listeners {
                let port = *host_port;
                // In real impl, would use select! over all listeners
                accept_futures.push(port);
            }

            // Simplified - real impl would use tokio::select!
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Handle incoming host connection
    pub async fn handle_host_connection(
        &self,
        host_port: u16,
        host_conn: TcpStream,
        vsock_addr: &str,
    ) -> Result<()> {
        // Connect to guest via vsock
        let guest_conn = UnixStream::connect(vsock_addr).await
            .map_err(|e| Error::Io(e))?;

        // Bidirectional proxy
        tokio::spawn(async move {
            let (mut host_read, mut host_write) = host_conn.into_split();
            let (mut guest_read, mut guest_write) = guest_conn.into_split();

            let host_to_guest = async {
                let mut buf = vec![0u8; 8192];
                loop {
                    match host_read.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if guest_write.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            };

            let guest_to_host = async {
                let mut buf = vec![0u8; 8192];
                loop {
                    match guest_read.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if host_write.write_all(&buf[..n]).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            };

            tokio::select! {
                _ = host_to_guest => {},
                _ = guest_to_host => {},
            }
        });

        Ok(())
    }
}
```

## Crate 3: engine-firecracker

```rust
// crates/engine-firecracker/src/lib.rs
mod engine;
mod api_client;
mod jailer;
mod snapshot;

pub use engine::FirecrackerEngine;
pub use api_client::FirecrackerApiClient;
pub use jailer::Jailer;
```

```rust
// crates/engine-firecracker/src/engine.rs
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use microvm_core::*;
use crate::{FirecrackerApiClient, Jailer};

pub struct FirecrackerEngine {
    binary_path: PathBuf,
    jailer_path: Option<PathBuf>,
    storage_root: PathBuf,
    seccomp_level: SeccompLevel,
}

#[derive(Debug, Clone)]
pub enum SeccompLevel {
    None,
    Basic,
    Advanced,
}

impl FirecrackerEngine {
    pub fn new() -> Result<Self> {
        let storage_root = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("microvm/firecracker");

        std::fs::create_dir_all(&storage_root)?;

        // Find firecracker binary
        let binary_path = which::which("firecracker")
            .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/firecracker"));

        Ok(Self {
            binary_path,
            jailer_path: None,
            storage_root,
            seccomp_level: SeccompLevel::Advanced,
        })
    }

    pub fn with_jailer(mut self, path: impl Into<PathBuf>) -> Self {
        self.jailer_path = Some(path.into());
        self
    }
}

#[async_trait]
impl Engine for FirecrackerEngine {
    fn name(&self) -> &'static str {
        "firecracker"
    }

    fn version(&self) -> &str {
        // Query binary version
        "1.6.0"
    }

    fn is_available() -> Result<bool> {
        // Check /dev/kvm exists
        if !std::path::Path::new("/dev/kvm").exists() {
            return Ok(false);
        }

        // Check firecracker binary
        which::which("firecracker").is_ok()
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            fast_boot: true,
            snapshots: true,
            diff_snapshots: true,
            live_migration: false,
            memory_ballooning: true,
            gpu_passthrough: false,
            virtio_fs: false,
            tsi_networking: false,
            confidential: false,
            platforms: vec![
                Platform::LinuxX86_64,
                Platform::LinuxAArch64,
            ],
        }
    }

    async fn create_vm(&self, config: VMConfig) -> Result<Box<dyn VM>> {
        // Create API socket path
        let api_sock = self.storage_root
            .join(&config.id)
            .join("firecracker.sock");

        // Create VM directory
        let vm_dir = self.storage_root.join(&config.id);
        std::fs::create_dir_all(&vm_dir)?;

        // Write config JSON
        let fc_config = self.build_firecracker_config(&config)?;
        let config_path = vm_dir.join("config.json");
        std::fs::write(&config_path, serde_json::to_string_pretty(&fc_config)?)?;

        // Start firecracker (with or without jailer)
        let mut child = if let Some(ref jailer_path) = self.jailer_path {
            Jailer::spawn(
                jailer_path,
                &self.binary_path,
                &config.id,
                &api_sock,
                &config_path,
            )?
        } else {
            self.spawn_direct(&api_sock, &config_path)?
        };

        // Connect to API
        let api_client = FirecrackerApiClient::new(&api_sock)?;

        // Configure VM via API
        api_client.configure(&fc_config).await?;

        // Start VM
        api_client.start().await?;

        Ok(Box::new(FirecrackerVM {
            id: config.id.clone(),
            name: config.name.clone(),
            config,
            api_client,
            child: Some(child),
            state: VMState::Running,
            exit_code: None,
        }))
    }
}
```

## VM Manager

```rust
// crates/microvm-core/src/manager.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{Engine, VM, VMConfig, Result, Error, EngineCapabilities, VMInfo};

/// Multi-engine VM manager
pub struct MicroVMManager {
    engines: HashMap<String, Arc<dyn Engine>>,
    vms: Arc<RwLock<HashMap<String, Arc<RwLock<dyn VM>>>>>,
    default_engine: String,
}

impl MicroVMManager {
    pub fn new() -> Result<Self> {
        let mut manager = Self {
            engines: HashMap::new(),
            vms: Arc::new(RwLock::new(HashMap::new())),
            default_engine: String::new(),
        };

        manager.discover_engines()?;
        manager.default_engine = manager.select_default_engine();

        Ok(manager)
    }

    fn discover_engines(&mut self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(engine) = crate::engines::FirecrackerEngine::new() {
                if FirecrackerEngine::is_available().unwrap_or(false) {
                    self.engines.insert("firecracker".into(), Arc::new(engine));
                }
            }
        }

        if let Ok(engine) = crate::engines::LibkrunEngine::new() {
            if LibkrunEngine::is_available().unwrap_or(false) {
                self.engines.insert("libkrun".into(), Arc::new(engine));
            }
        }

        if let Ok(engine) = crate::engines::QemuEngine::new() {
            self.engines.insert("qemu".into(), Arc::new(engine));
        }

        Ok(())
    }

    fn select_default_engine(&self) -> String {
        #[cfg(target_os = "macos")]
        {
            if self.engines.contains_key("libkrun") {
                "libkrun".to_string()
            } else {
                "qemu".to_string()
            }
        }
        #[cfg(target_os = "linux")]
        {
            if self.engines.contains_key("firecracker") {
                "firecracker".to_string()
            } else if self.engines.contains_key("libkrun") {
                "libkrun".to_string()
            } else {
                "qemu".to_string()
            }
        }
    }

    pub async fn create_vm(&self, config: VMConfig) -> Result<()> {
        let engine_name = self.select_engine_for_config(&config);
        let engine = self.engines.get(&engine_name)
            .ok_or_else(|| Error::EngineNotFound(engine_name.clone()))?;

        let vm = engine.create_vm(config).await?;
        let id = vm.id().to_string();

        let mut vms = self.vms.write().await;
        vms.insert(id, Arc::new(RwLock::new(vm)));

        Ok(())
    }

    fn select_engine_for_config(&self, config: &VMConfig) -> &str {
        let opts = &config.engine_options;

        // Platform-specific defaults
        #[cfg(target_os = "macos")]
        {
            if opts.require_tsi && self.engines.contains_key("libkrun") {
                return "libkrun";
            }
            if self.engines.contains_key("libkrun") {
                return "libkrun";
            }
            return "qemu";
        }

        #[cfg(target_os = "linux")]
        {
            if opts.require_fast_boot && self.engines.contains_key("firecracker") {
                return "firecracker";
            }
            if opts.require_tsi && self.engines.contains_key("libkrun") {
                return "libkrun";
            }
            if self.engines.contains_key("firecracker") {
                return "firecracker";
            }
            if self.engines.contains_key("libkrun") {
                return "libkrun";
            }
            return "qemu";
        }
    }
}
```

## Dependencies (Cargo.toml per crate)

```toml
# crates/microvm-core/Cargo.toml
[package]
name = "microvm-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
tokio = { workspace = true }
async-trait = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
chrono = { version = "0.4", features = ["serde"] }
uuid = { workspace = true }

# crates/engine-libkrun/Cargo.toml
[package]
name = "engine-libkrun"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
microvm-core = { path = "../microvm-core" }
tokio = { workspace = true, features = ["net", "sync"] }
async-trait = { workspace = true }
libc = { workspace = true }
tracing = { workspace = true }
dirs = "5.0"

# crates/engine-firecracker/Cargo.toml
[package]
name = "engine-firecracker"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
microvm-core = { path = "../microvm-core" }
tokio = { workspace = true, features = ["full"] }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
which = "5.0"
hyper = { version = "1.0", features = ["client", "http1"] }
hyper-util = { version = "0.1", features = ["client-legacy", "tokio"] }
```

## Example Usage

```rust
// examples/basic-vm/main.rs
use microvm_core::{VMConfigBuilder, NetworkConfig, NetworkType, MicroVMManager};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create manager
    let manager = MicroVMManager::new()?;

    // Configure VM
    let config = VMConfigBuilder::new("example-vm")
        .name("Example VM")
        .vcpus(2)
        .memory(2048)
        .kernel("./vmlinux")
        .rootfs("./rootfs.ext4")
        .tsi_network(vec![(80, 8080), (443, 8443)])  // TSI port forwards
        .require_fast_boot(true)
        .build()?;

    // Create and start VM
    println!("Creating VM with engine: {}", manager.select_engine_for_config(&config));
    manager.create_vm(config).await?;

    println!("VM created and running!");

    Ok(())
}
```

## Testing Strategy

```rust
// tests/integration_test.rs
#[cfg(test)]
mod tests {
    use microvm_core::*;
    use engine_libkrun::LibkrunEngine;

    #[tokio::test]
    async fn test_libkrun_vm_lifecycle() {
        let engine = LibkrunEngine::new().unwrap();

        let config = VMConfigBuilder::new("test-vm")
            .vcpus(1)
            .memory(256)
            .kernel("./test-vmlinux")
            .rootfs("./test-rootfs.ext4")
            .build()
            .unwrap();

        let mut vm = engine.create_vm(config).await.unwrap();

        assert_eq!(vm.state(), VMState::Created);
        vm.start().await.unwrap();
        assert_eq!(vm.state(), VMState::Running);
    }
}
```
