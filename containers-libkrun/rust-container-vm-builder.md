---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.containers/
revised_at: 2026-03-19
---

# Rust Guide: Building Container VMs with libkrun

This comprehensive guide covers using libkrun from Rust to create container images, root filesystems, and Linux boot images for container workloads.

## Table of Contents

1. [Overview](#overview)
2. [Project Setup](#project-setup)
3. [Core Data Structures](#core-data-structures)
4. [Root Filesystem Builder](#root-filesystem-builder)
5. [Disk Image Creator](#disk-image-creator)
6. [Kernel Boot Image Builder](#kernel-boot-image-builder)
7. [libkrun VM Manager](#libkrun-vm-manager)
8. [Complete Example: Container VM CLI](#complete-example-container-vm-cli)
9. [Advanced Topics](#advanced-topics)

## Overview

libkrun provides a C API (with Rust bindings) that enables:

1. **Building container root filesystems** from OCI images
2. **Creating bootable disk images** for VM-based containers
3. **Launching container workloads** in lightweight VMs
4. **Managing VM lifecycle** with near-container startup times

```
┌─────────────────────────────────────────────────────────────┐
│              Your Rust Application                          │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  libkrun-sys / libkrun-bindings                     │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    libkrun (C API)                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   VM        │  │   Disk      │  │   Network           │  │
│  │   Manager   │  │   Builder   │  │   Configuration     │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
       ┌────────────┐ ┌────────────┐  ┌────────────┐
       │  libkrunfw │ │  buildah   │  │  libkrun   │
       │  (Kernel)  │ │  (Images)  │  │   (VMM)    │
       └────────────┘ └────────────┘  └────────────┘
```

## Project Setup

### Cargo.toml

```toml
[package]
name = "container-vm-builder"
version = "0.1.0"
edition = "2021"

[dependencies]
# libkrun bindings
libkrun-sys = { path = "../libkrun/krun-sys" }

# OCI image handling
oci-spec = { version = "0.6", features = ["image", "distribution"] }
dockerfile-parser = "0.8"

# Disk image creation
loopdev = "0.4"
ext4-std = "0.1"

# Tar handling for rootfs
tar = "0.4"
flate2 = "1.0"
xz2 = "0.1"

# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP client for registry access
reqwest = { version = "0.11", features = ["json", "stream"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# CLI
clap = { version = "4.0", features = ["derive"] }

# Utilities
uuid = { version = "1.0", features = ["v4"] }
tempfile = "3.0"
```

### Build.rs (for libkrun linkage)

```rust
// build.rs
use std::path::PathBuf;

fn main() {
    // Link against libkrun
    println!("cargo:rustc-link-lib=dylib=krun");

    // Set library search path (adjust for your system)
    println!("cargo:rustc-link-search=native=/usr/local/lib");

    // Rebuild if libkrun headers change
    println!("cargo:rerun-if-changed=/usr/local/include/libkrun.h");

    // Tell cargo to rerun if pkg-config findings change
    pkg_config::Config::new()
        .atleast_version("1.0")
        .probe("libkrun")
        .expect("libkrun not found");
}
```

### src/lib.rs - Module Structure

```rust
pub mod rootfs;
pub mod disk_image;
pub mod kernel;
pub mod vm;
pub mod oci;

pub use rootfs::RootFsBuilder;
pub use disk_image::DiskImageCreator;
pub use kernel::KernelBuilder;
pub use vm::{VmContext, VmConfig, ContainerVmRunner};
```

## Core Data Structures

### VM Configuration

```rust
// src/vm/config.rs
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContainerVmError {
    #[error("libkrun error: {0}")]
    Libkrun(i32),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("OCI error: {0}")]
    Oci(#[from] oci_spec::error::Error),

    #[error("Disk image error: {0}")]
    DiskImage(String),

    #[error("Kernel error: {0}")]
    Kernel(String),
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    /// Number of virtual CPUs
    pub cpus: u8,
    /// Memory in MiB
    pub memory_mib: u32,
    /// Path to root filesystem
    pub rootfs: PathBuf,
    /// Optional custom kernel path
    pub kernel: Option<PathBuf>,
    /// Optional initramfs path
    pub initramfs: Option<PathBuf>,
    /// Kernel command line
    pub kernel_cmdline: Option<String>,
    /// Optional firmware/EFI path
    pub firmware: Option<PathBuf>,
    /// Command to execute in guest
    pub cmd: Vec<String>,
    /// Environment variables
    pub env: Vec<String>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Enable GPU acceleration
    pub gpu_enabled: bool,
    /// Enable networking
    pub network_enabled: bool,
    /// Network backend (passt, gvproxy, tap)
    pub network_backend: NetworkBackend,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub host_path: PathBuf,
    pub guest_path: PathBuf,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub struct PortMapping {
    pub host_port: u16,
    pub guest_port: u16,
}

#[derive(Debug, Clone, Default)]
pub enum NetworkBackend {
    #[default]
    Passt,
    Gvproxy,
    Tap,
    Tsi,  // Transparent Socket Impersonation
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            cpus: 2,
            memory_mib: 1024,
            rootfs: PathBuf::from("/"),
            kernel: None,
            initramfs: None,
            kernel_cmdline: None,
            firmware: None,
            cmd: vec!["/sbin/init".to_string()],
            env: vec![],
            volumes: vec![],
            ports: vec![],
            gpu_enabled: false,
            network_enabled: true,
            network_backend: NetworkBackend::default(),
        }
    }
}
```

## Root Filesystem Builder

### Creating RootFS from OCI Image

```rust
// src/rootfs.rs
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use tar::{Archive, Entry, EntryType};
use flate2::read::GzDecoder;
use xz2::read::XzDecoder;
use oci_spec::image::{ImageManifest, Config as OciConfig, Descriptor};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RootFsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("OCI error: {0}")]
    Oci(#[from] oci_spec::error::Error),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Layer extraction error: {0}")]
    LayerExtract(String),
}

pub struct RootFsBuilder {
    workdir: PathBuf,
    layers_dir: PathBuf,
    rootfs_dir: PathBuf,
}

impl RootFsBuilder {
    pub fn new(workdir: impl AsRef<Path>) -> Result<Self, RootFsError> {
        let workdir = workdir.as_ref().to_path_buf();
        let layers_dir = workdir.join("layers");
        let rootfs_dir = workdir.join("rootfs");

        fs::create_dir_all(&layers_dir)?;
        fs::create_dir_all(&rootfs_dir)?;

        Ok(Self {
            workdir,
            layers_dir,
            rootfs_dir,
        })
    }

    /// Extract OCI image layers into rootfs
    pub async fn from_oci_image(
        &self,
        image_name: &str,
    ) -> Result<PathBuf, RootFsError> {
        tracing::info!("Pulling image: {}", image_name);

        // Parse image reference
        let (registry, repository, tag) = self.parse_image_name(image_name)?;

        // Pull manifest
        let manifest = self.pull_manifest(&registry, &repository, &tag).await?;

        // Pull and extract layers
        self.extract_layers(&manifest, &registry, &repository).await?;

        Ok(self.rootfs_dir.clone())
    }

    /// Parse image name into components
    fn parse_image_name(&self, name: &str) -> Result<(String, String, String), RootFsError> {
        // Simple parser - in production use oci-spec distribution types
        let parts: Vec<&str> = name.split('/').collect();

        let (registry, repository, tag) = match parts.len() {
            1 => ("docker.io".to_string(), "library".to_string(), parts[0].to_string()),
            2 => ("docker.io".to_string(), parts[0].to_string(), parts[1].to_string()),
            3 => (parts[0].to_string(), parts[1].to_string(), parts[2].to_string()),
            _ => return Err(RootFsError::Registry("Invalid image name".into())),
        };

        // Handle tag with digest
        let (repository, tag) = if let Some(idx) = tag.find('@') {
            (tag[..idx].to_string(), tag[idx+1..].to_string())
        } else {
            (repository, if tag.contains(':') { tag } else { format!("{}:latest", tag) })
        };

        Ok((registry, repository, tag))
    }

    /// Pull manifest from registry
    async fn pull_manifest(
        &self,
        registry: &str,
        repository: &str,
        tag: &str,
    ) -> Result<ImageManifest, RootFsError> {
        let url = format!("https://{}/v2/{}/manifests/{}", registry, repository, tag);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Accept", "application/vnd.oci.image.manifest.v1+json")
            .send()
            .await
            .map_err(|e| RootFsError::Registry(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RootFsError::Registry(format!(
                "Registry returned {}",
                response.status()
            )));
        }

        let manifest: ImageManifest = response
            .json()
            .await
            .map_err(|e| RootFsError::Registry(e.to_string()))?;

        Ok(manifest)
    }

    /// Download and extract all layers
    async fn extract_layers(
        &self,
        manifest: &ImageManifest,
        registry: &str,
        repository: &str,
    ) -> Result<(), RootFsError> {
        let repository = repository.replace("library/", "");

        for (i, layer) in manifest.layers().iter().enumerate() {
            tracing::info!("Extracting layer {}/{}", i + 1, manifest.layers().len());

            // Download layer
            let layer_data = self.download_layer(registry, &repository, layer).await?;

            // Extract to rootfs
            self.extract_layer_data(&layer_data)?;
        }

        Ok(())
    }

    /// Download a single layer
    async fn download_layer(
        &self,
        registry: &str,
        repository: &str,
        descriptor: &Descriptor,
    ) -> Result<Vec<u8>, RootFsError> {
        let digest = descriptor.digest();
        let url = format!("https://{}/v2/{}/blobs/{}", registry, repository, digest);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| RootFsError::Registry(e.to_string()))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| RootFsError::Registry(e.to_string()))?;

        Ok(bytes.to_vec())
    }

    /// Extract layer tarball (supports gzip and xz)
    fn extract_layer_data(&self, data: &[u8]) -> Result<(), RootFsError> {
        // Detect compression from magic bytes
        let decoder: Box<dyn Read> = match data.get(0..6) {
            Some([0x1f, 0x8b, _, _, _, _]) => Box::new(GzDecoder::new(data)),
            Some([0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]) => Box::new(XzDecoder::new(data)),
            _ => Box::new(data),
        };

        let mut archive = Archive::new(decoder);

        for entry_result in archive.entries().map_err(|e| RootFsError::LayerExtract(e.to_string()))? {
            let mut entry = entry_result.map_err(|e| RootFsError::LayerExtract(e.to_string()))?;
            let path = entry.path().map_err(|e| RootFsError::LayerExtract(e.to_string()))?;
            let entry_type = entry.header().entry_type();

            // Handle different entry types
            match entry_type {
                EntryType::Regular => {
                    let target_path = self.rootfs_dir.join(path);

                    // Create parent directories
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    // Write file content
                    let mut file = File::create(&target_path)?;
                    std::io::copy(&mut entry, &mut file)?;
                }
                EntryType::Directory => {
                    let target_path = self.rootfs_dir.join(path);
                    fs::create_dir_all(&target_path)?;
                }
                EntryType::Symlink => {
                    let target_path = self.rootfs_dir.join(path);
                    if let Some(link_name) = entry.link_name().ok() {
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        // Remove existing file if present
                        let _ = fs::remove_file(&target_path);
                        std::os::unix::fs::symlink(link_name, &target_path)?;
                    }
                }
                _ => {
                    // Skip special files (devices, fifos, etc.)
                    tracing::debug!("Skipping special file: {:?}", path);
                }
            }
        }

        Ok(())
    }

    /// Create ext4 disk image from rootfs
    pub fn create_disk_image(
        &self,
        output_path: impl AsRef<Path>,
        size_mb: u32,
    ) -> Result<PathBuf, RootFsError> {
        let output_path = output_path.as_ref().to_path_buf();

        tracing::info!("Creating {}MB disk image at {:?}", size_mb, output_path);

        // Create sparse file
        let file = File::create(&output_path)?;
        file.set_len((size_mb as u64) * 1024 * 1024)?;

        // Format as ext4
        self.format_ext4(&output_path)?;

        // Copy rootfs contents
        self.copy_rootfs_to_image(&output_path)?;

        Ok(output_path)
    }

    fn format_ext4(&self, device: &Path) -> Result<(), RootFsError> {
        // Use mkfs.ext4 - in production, consider using a Rust library
        let status = std::process::Command::new("mkfs.ext4")
            .arg("-F")           // Force overwrite
            .arg("-L rootfs")    // Label
            .arg("-E lazy_itable_init=0,lazy_journal_init=0") // Eager init
            .arg(device)
            .status()?;

        if !status.success() {
            return Err(RootFsError::LayerExtract(
                "mkfs.ext4 failed".into()
            ));
        }

        Ok(())
    }

    fn copy_rootfs_to_image(&self, device: &Path) -> Result<(), RootFsError> {
        // Use debugfs or direct ext4 writing
        // For production, consider using libext2fs bindings

        // Method 1: Using guestfs (libguestfs)
        let status = std::process::Command::new("guestfish")
            .arg("--ro")
            .arg("-a")
            .arg(device)
            .arg("run")
            .arg(":")
            .arg("tar-in")
            .arg(format!("{}/.", self.rootfs_dir.display()))
            .arg("/")
            .status()?;

        if !status.success() {
            return Err(RootFsError::LayerExtract(
                "guestfish tar-in failed".into()
            ));
        }

        Ok(())
    }
}
```

## Disk Image Creator

### Raw and QCOW2 Image Support

```rust
// src/disk_image.rs
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Read, Write, Seek, SeekFrom};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiskImageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("QCOW2 error: {0}")]
    Qcow2(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiskFormat {
    Raw,
    Qcow2,
    Vmdk,
}

pub struct DiskImageCreator {
    output_dir: PathBuf,
}

impl DiskImageCreator {
    pub fn new(output_dir: impl AsRef<Path>) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Create a raw disk image
    pub fn create_raw(
        &self,
        name: &str,
        size_mb: u32,
    ) -> Result<PathBuf, DiskImageError> {
        let path = self.output_dir.join(format!("{}.raw", name));

        tracing::info!("Creating raw disk: {} ({}MB)", path.display(), size_mb);

        let mut file = File::create(&path)?;
        file.set_len((size_mb as u64) * 1024 * 1024)?;

        Ok(path)
    }

    /// Create a QCOW2 disk image
    pub fn create_qcow2(
        &self,
        name: &str,
        size_mb: u32,
        backing_file: Option<&Path>,
    ) -> Result<PathBuf, DiskImageError> {
        let path = self.output_dir.join(format!("{}.qcow2", name));

        tracing::info!("Creating QCOW2 disk: {} ({}MB)", path.display(), size_mb);

        // QCOW2 header structure
        let mut file = File::create(&path)?;

        // Write QCOW2 magic
        file.write_all(&[b'Q', b'F', b'I', 0xfb])?;

        // Version 3
        file.write_all(&3u32.to_be_bytes())?;

        // Backing file offset (0 if none)
        let backing_offset = if backing_file.is_some() { 0x58u64 } else { 0 };
        file.write_all(&backing_offset.to_be_bytes())?;

        // Backing file size
        let backing_size = backing_file
            .map(|p| p.to_str().unwrap_or("").len() as u32)
            .unwrap_or(0);
        file.write_all(&backing_size.to_be_bytes())?;

        // Cluster bits (default 12 = 4KB clusters)
        file.write_all(&12u32.to_be_bytes())?;

        // Size in bytes
        file.write_all(&((size_mb as u64) * 1024 * 1024).to_be_bytes())?;

        // Crypt method (0 = none)
        file.write_all(&0u32.to_be_bytes())?;

        // L1 size
        let l1_size = ((size_mb as u64) * 1024 * 1024) / (1u64 << (12 + 9 + 9));
        file.write_all(&l1_size.to_be_bytes())?;

        // Padding to backing file offset
        let padding_size = 0x58 - file.metadata()?.len();
        file.write_all(&vec![0u8; padding_size as usize])?;

        // Backing file path (if specified)
        if let Some(backing) = backing_file {
            if let Some(path_str) = backing.to_str() {
                file.write_all(path_str.as_bytes())?;
            }
        }

        // Extend to final size
        file.set_len((size_mb as u64) * 1024 * 1024)?;

        Ok(path)
    }

    /// Resize a disk image
    pub fn resize(
        &self,
        image_path: &Path,
        new_size_mb: u32,
    ) -> Result<(), DiskImageError> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(image_path)?;

        file.set_len((new_size_mb as u64) * 1024 * 1024)?;

        tracing::info!("Resized {} to {}MB", image_path.display(), new_size_mb);

        Ok(())
    }

    /// Convert between disk formats
    pub fn convert(
        &self,
        source: &Path,
        dest_format: DiskFormat,
        dest_path: &Path,
    ) -> Result<PathBuf, DiskImageError> {
        match dest_format {
            DiskFormat::Raw => {
                // For raw, we can just copy (qemu-img would be better)
                let mut src = File::open(source)?;
                let mut dest = File::create(dest_path)?;
                std::io::copy(&mut src, &mut dest)?;
            }
            DiskFormat::Qcow2 => {
                // Use qemu-img for conversion
                let status = std::process::Command::new("qemu-img")
                    .arg("convert")
                    .arg("-O")
                    .arg("qcow2")
                    .arg(source)
                    .arg(dest_path)
                    .status()?;

                if !status.success() {
                    return Err(DiskImageError::Qcow2("Conversion failed".into()));
                }
            }
            DiskFormat::Vmdk => {
                let status = std::process::Command::new("qemu-img")
                    .arg("convert")
                    .arg("-O")
                    .arg("vmdk")
                    .arg(source)
                    .arg(dest_path)
                    .status()?;

                if !status.success() {
                    return Err(DiskImageError::InvalidFormat(
                        "VMDK conversion failed".into()
                    ));
                }
            }
        }

        Ok(dest_path.to_path_buf())
    }
}
```

## Kernel Boot Image Builder

### Building Custom Kernels with libkrunfw

```rust
// src/kernel.rs
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KernelError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

#[derive(Debug, Clone)]
pub struct KernelConfig {
    /// Enable AMD SEV support
    pub sev_enabled: bool,
    /// Enable Intel TDX support
    pub tdx_enabled: bool,
    /// Enable EFI boot
    pub efi_enabled: bool,
    /// Extra kernel config options
    pub extra_config: Vec<String>,
    /// Kernel patches to apply
    pub patches: Vec<PathBuf>,
    /// Target architecture
    pub arch: String,
    /// Cross-compiler prefix
    pub cross_compile: Option<String>,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            sev_enabled: false,
            tdx_enabled: false,
            efi_enabled: false,
            extra_config: vec![],
            patches: vec![],
            arch: "x86_64".to_string(),
            cross_compile: None,
        }
    }
}

pub struct KernelBuilder {
    workdir: PathBuf,
    output_dir: PathBuf,
}

impl KernelBuilder {
    pub fn new(workdir: impl AsRef<Path>) -> Result<Self, KernelError> {
        let workdir = workdir.as_ref().to_path_buf();
        let output_dir = workdir.join("output");

        fs::create_dir_all(&output_dir)?;

        Ok(Self {
            workdir,
            output_dir,
        })
    }

    /// Clone libkrunfw repository
    pub fn clone_libkrunfw(&self) -> Result<(), KernelError> {
        let libkrunfw_dir = self.workdir.join("libkrunfw");

        if libkrunfw_dir.exists() {
            tracing::info!("libkrunfw already exists, pulling latest");
            Command::new("git")
                .arg("pull")
                .current_dir(&libkrunfw_dir)
                .status()?;
        } else {
            tracing::info!("Cloning libkrunfw repository");
            Command::new("git")
                .arg("clone")
                .arg("https://github.com/containers/libkrunfw")
                .arg(&libkrunfw_dir)
                .status()?;
        }

        Ok(())
    }

    /// Build kernel for libkrun
    pub fn build_kernel(&self, config: &KernelConfig) -> Result<PathBuf, KernelError> {
        let libkrunfw_dir = self.workdir.join("libkrunfw");
        let linux_dir = libkrunfw_dir.join("linux");

        // Apply patches
        self.apply_patches(&linux_dir, &config.patches)?;

        // Configure kernel
        self.configure_kernel(&linux_dir, config)?;

        // Compile kernel
        self.compile_kernel(&linux_dir, config)?;

        // Build libkrunfw library
        self.build_libkrunfw(&libkrunfw_dir, config)?;

        Ok(self.output_dir.join("libkrunfw.so"))
    }

    fn apply_patches(&self, linux_dir: &Path, patches: &[PathBuf]) -> Result<(), KernelError> {
        for patch in patches {
            tracing::info!("Applying patch: {:?}", patch);

            let status = Command::new("patch")
                .arg("-p1")
                .arg("-i")
                .arg(patch)
                .current_dir(linux_dir)
                .status()?;

            if !status.success() {
                return Err(KernelError::Build(
                    format!("Failed to apply patch: {:?}", patch)
                ));
            }
        }

        Ok(())
    }

    fn configure_kernel(&self, linux_dir: &Path, config: &KernelConfig) -> Result<(), KernelError> {
        tracing::info!("Configuring kernel");

        // Start with defconfig
        let mut cmd = Command::new("make");
        cmd.arg("defconfig").current_dir(linux_dir);

        // Add architecture-specific config
        match config.arch.as_str() {
            "x86_64" => {
                if config.sev_enabled {
                    cmd.arg("sev_defconfig");
                } else if config.tdx_enabled {
                    cmd.arg("tdx_defconfig");
                }
            }
            "aarch64" => {
                cmd.arg("defconfig");
            }
            _ => {}
        }

        cmd.status()?;

        // Apply extra config options
        self.apply_kernel_config(linux_dir, &config.extra_config)?;

        // Ensure libkrun-specific options
        let required_options = [
            "CONFIG_NR_CPUS=8",           // Memory optimization
            "CONFIG_VIRTIO=y",
            "CONFIG_VIRTIO_PCI=y",
            "CONFIG_VIRTIO_BLK=y",
            "CONFIG_VIRTIO_NET=y",
            "CONFIG_VIRTIO_FS=y",
            "CONFIG_VIRTIO_CONSOLE=y",
        ];

        self.apply_kernel_config(linux_dir, &required_options.iter().map(|s| s.to_string()).collect())?;

        Ok(())
    }

    fn apply_kernel_config(&self, linux_dir: &Path, options: &[String]) -> Result<(), KernelError> {
        for option in options {
            if let Some((key, value)) = option.split_once('=') {
                // Use scripts/config if available, otherwise sed
                let status = Command::new("scripts/config")
                    .arg("--file")
                    .arg(".config")
                    .arg("--set-str")
                    .arg(key)
                    .arg(value)
                    .current_dir(linux_dir)
                    .status();

                if status.is_err() {
                    // Fallback to sed
                    let sed_cmd = format!("s/^#?{}=.*/{}={}/", key, key, value);
                    Command::new("sed")
                        .arg("-i")
                        .arg(&sed_cmd)
                        .arg(".config")
                        .current_dir(linux_dir)
                        .status()?;
                }
            }
        }

        // Run olddefconfig to resolve any conflicts
        Command::new("make")
            .arg("olddefconfig")
            .current_dir(linux_dir)
            .status()?;

        Ok(())
    }

    fn compile_kernel(&self, linux_dir: &Path, config: &KernelConfig) -> Result<(), KernelError> {
        tracing::info!("Compiling kernel");

        let mut cmd = Command::new("make");
        cmd.arg("-j").arg(&format!("{}", num_cpus::get()));

        if let Some(ref cross) = config.cross_compile {
            cmd.arg(format!("CROSS_COMPILE={}", cross));
        }

        let status = cmd.current_dir(linux_dir).status()?;

        if !status.success() {
            return Err(KernelError::Build("Kernel compilation failed".into()));
        }

        Ok(())
    }

    fn build_libkrunfw(&self, libkrunfw_dir: &Path, config: &KernelConfig) -> Result<(), KernelError> {
        tracing::info!("Building libkrunfw");

        let mut cmd = Command::new("make");

        if config.sev_enabled {
            cmd.arg("SEV=1");
        } else if config.tdx_enabled {
            cmd.arg("TDX=1");
        }

        let status = cmd.current_dir(libkrunfw_dir).status()?;

        if !status.success() {
            return Err(KernelError::Build("libkrunfw build failed".into()));
        }

        // Copy output
        let output_lib = if config.sev_enabled {
            libkrunfw_dir.join("libkrunfw-sev.so")
        } else if config.tdx_enabled {
            libkrunfw_dir.join("libkrunfw-tdx.so")
        } else {
            libkrunfw_dir.join("libkrunfw.so")
        };

        fs::copy(&output_lib, self.output_dir.join("libkrunfw.so"))?;

        Ok(())
    }

    /// Create initramfs from rootfs
    pub fn create_initramfs(
        &self,
        rootfs_dir: &Path,
        output_path: &Path,
    ) -> Result<PathBuf, KernelError> {
        tracing::info!("Creating initramfs from {:?}", rootfs_dir);

        // Create gzip-compressed cpio archive
        let status = Command::new("find")
            .arg(".")
            .arg("-print0")
            .current_dir(rootfs_dir)
            .pipe(Command::new("cpio"))
            .arg("--null")
            .arg("-o")
            .arg("--format=newc")
            .pipe(Command::new("gzip"))
            .arg("-9")
            .arg("-c")
            .arg(">")
            .arg(output_path)
            .status()?;

        if !status.success() {
            return Err(KernelError::Build("initramfs creation failed".into()));
        }

        Ok(output_path.to_path_buf())
    }
}
```

## libkrun VM Manager

### VM Context Wrapper

```rust
// src/vm/manager.rs
use libkrun_sys::*;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use crate::vm::config::*;

pub struct VmContext {
    ctx_id: i32,
}

impl VmContext {
    pub fn new() -> Result<Self, ContainerVmError> {
        let ctx_id = unsafe { krun_create_ctx() };
        if ctx_id < 0 {
            return Err(ContainerVmError::Libkrun(ctx_id));
        }

        Ok(Self { ctx_id })
    }

    pub fn configure(&self, config: &VmConfig) -> Result<(), ContainerVmError> {
        // Set VM resources
        let ret = unsafe {
            krun_set_vm_config(
                self.ctx_id as u32,
                config.cpus,
                config.memory_mib,
            )
        };
        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }

        // Set kernel if provided
        if let Some(ref kernel_path) = config.kernel {
            self.set_kernel(kernel_path, config.initramfs.as_ref(), config.kernel_cmdline.as_deref())?;
        }

        // Set firmware for EFI boot
        if let Some(ref firmware_path) = config.firmware {
            self.set_firmware(firmware_path)?;
        }

        // Set root filesystem
        self.set_root(&config.rootfs)?;

        // Configure volumes
        for volume in &config.volumes {
            self.add_virtiofs(volume)?;
        }

        // Configure networking
        if config.network_enabled {
            self.configure_network(&config.network_backend)?;
        }

        // Configure ports
        if !config.ports.is_empty() {
            self.configure_ports(&config.ports)?;
        }

        // Configure GPU if enabled
        if config.gpu_enabled {
            self.configure_gpu()?;
        }

        Ok(())
    }

    fn set_root(&self, path: &PathBuf) -> Result<(), ContainerVmError> {
        let root_cstr = CString::new(path.as_os_str().as_bytes())?;
        let ret = unsafe { krun_set_root(self.ctx_id as u32, root_cstr.as_ptr()) };
        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }
        Ok(())
    }

    fn set_kernel(
        &self,
        kernel_path: &PathBuf,
        initramfs: Option<&PathBuf>,
        cmdline: Option<&str>,
    ) -> Result<(), ContainerVmError> {
        let kernel_cstr = CString::new(kernel_path.as_os_str().as_bytes())?;
        let initramfs_cstr = initramfs
            .map(|p| CString::new(p.as_os_str().as_bytes()))
            .transpose()?;
        let cmdline_cstr = cmdline
            .map(|s| CString::new(s.as_bytes()))
            .transpose()?;

        let ret = unsafe {
            krun_set_kernel(
                self.ctx_id as u32,
                kernel_cstr.as_ptr(),
                KRUN_KERNEL_FORMAT_ELF as u32,
                initramfs_cstr.map(|s| s.as_ptr()).unwrap_or(std::ptr::null()),
                cmdline_cstr.map(|s| s.as_ptr()).unwrap_or(std::ptr::null()),
            )
        };

        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }

        Ok(())
    }

    fn set_firmware(&self, path: &PathBuf) -> Result<(), ContainerVmError> {
        let fw_cstr = CString::new(path.as_os_str().as_bytes())?;
        let ret = unsafe { krun_set_firmware(self.ctx_id as u32, fw_cstr.as_ptr()) };
        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }
        Ok(())
    }

    fn add_virtiofs(&self, volume: &VolumeMount) -> Result<(), ContainerVmError> {
        let tag = CString::new(volume.guest_path.to_string_lossy().as_bytes())?;
        let path = CString::new(volume.host_path.as_os_str().as_bytes())?;

        let ret = unsafe {
            krun_add_virtiofs(
                self.ctx_id as u32,
                tag.as_ptr(),
                path.as_ptr(),
            )
        };

        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }

        Ok(())
    }

    fn configure_network(&self, backend: &NetworkBackend) -> Result<(), ContainerVmError> {
        match backend {
            NetworkBackend::Passt => {
                // passt networking - would need fd from passt process
                // For now, let TSI handle it
            }
            NetworkBackend::Tsi => {
                // TSI is automatic when no virtio-net is configured
            }
            NetworkBackend::Gvproxy | NetworkBackend::Tap => {
                // Would need socket/path configuration
                // Not implemented in this example
            }
        }

        Ok(())
    }

    fn configure_ports(&self, ports: &[PortMapping]) -> Result<(), ContainerVmError> {
        let port_strings: Vec<CString> = ports
            .iter()
            .map(|p| CString::new(format!("{}:{}", p.host_port, p.guest_port)))
            .collect::<Result<_, _>>()?;

        let port_ptrs: Vec<*const i8> = port_strings.iter().map(|s| s.as_ptr()).collect();
        port_ptrs.push(std::ptr::null()); // NULL terminator

        let ret = unsafe {
            krun_set_port_map(self.ctx_id as u32, port_ptrs.as_ptr())
        };

        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }

        Ok(())
    }

    fn configure_gpu(&self) -> Result<(), ContainerVmError> {
        let ret = unsafe {
            krun_set_gpu_options(self.ctx_id as u32, 0)
        };

        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }

        Ok(())
    }

    pub fn set_exec(&self, cmd: &[String], env: &[String]) -> Result<(), ContainerVmError> {
        let cmd_cstrings: Vec<CString> = cmd
            .iter()
            .map(|s| CString::new(s.as_bytes()))
            .collect::<Result<_, _>>()?;

        let cmd_ptrs: Vec<*const i8> = cmd_cstrings.iter().map(|s| s.as_ptr()).collect();
        cmd_ptrs.push(std::ptr::null());

        let env_cstrings: Vec<CString> = env
            .iter()
            .map(|s| CString::new(s.as_bytes()))
            .collect::<Result<_, _>>()?;

        let env_ptrs: Vec<*const i8> = env_cstrings.iter().map(|s| s.as_ptr()).collect();
        if !env.is_empty() {
            env_ptrs.push(std::ptr::null());
        }

        let exec_cstr = CString::new(cmd[0].as_bytes())?;

        let ret = unsafe {
            krun_set_exec(
                self.ctx_id as u32,
                exec_cstr.as_ptr(),
                cmd_ptrs.as_ptr(),
                if env.is_empty() { std::ptr::null() } else { env_ptrs.as_ptr() },
            )
        };

        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }

        Ok(())
    }

    pub fn set_workdir(&self, path: &PathBuf) -> Result<(), ContainerVmError> {
        let workdir_cstr = CString::new(path.as_os_str().as_bytes())?;
        let ret = unsafe { krun_set_workdir(self.ctx_id as u32, workdir_cstr.as_ptr()) };
        if ret < 0 {
            return Err(ContainerVmError::Libkrun(ret));
        }
        Ok(())
    }

    /// Start and enter the VM (consumes context)
    pub fn start(self) -> Result<i32, ContainerVmError> {
        tracing::info!("Starting VM with context {}", self.ctx_id);

        let ret = unsafe { krun_start_enter(self.ctx_id as u32) };

        // This typically only returns on error
        if ret < 0 {
            Err(ContainerVmError::Libkrun(ret))
        } else {
            Ok(ret)
        }
    }
}

impl Drop for VmContext {
    fn drop(&mut self) {
        if self.ctx_id >= 0 {
            unsafe { krun_free_ctx(self.ctx_id as u32) };
        }
    }
}
```

## Complete Example: Container VM CLI

```rust
// src/main.rs
use clap::{Parser, Subcommand};
use container_vm_builder::*;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vm-container")]
#[command(about = "Build and run container VMs with libkrun")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Working directory for temporary files
    #[arg(short, long, default_value = "/tmp/vm-container")]
    workdir: PathBuf,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run container in VM
    Run {
        /// OCI image name
        #[arg(required = true)]
        image: String,

        /// Number of CPUs
        #[arg(short, long, default_value = "2")]
        cpus: u8,

        /// Memory in MiB
        #[arg(short, long, default_value = "1024")]
        memory: u32,

        /// Volume mounts (host:guest[:ro])
        #[arg(short = 'v', long = "volume")]
        volumes: Vec<String>,

        /// Port mappings (host:guest)
        #[arg(short, long)]
        publish: Vec<String>,

        /// Custom kernel path
        #[arg(long)]
        kernel: Option<PathBuf>,

        /// EFI firmware path
        #[arg(long)]
        firmware: Option<PathBuf>,

        /// Command to run
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Create bootable disk image
    CreateImage {
        /// OCI image name
        #[arg(required = true)]
        image: String,

        /// Output path
        #[arg(short, long, required = true)]
        output: PathBuf,

        /// Image size in MiB
        #[arg(short, long, default_value = "4096")]
        size: u32,

        /// Output format (raw, qcow2)
        #[arg(short, long, default_value = "raw")]
        format: String,
    },

    /// Build root filesystem from OCI image
    BuildRootfs {
        /// OCI image name
        #[arg(required = true)]
        image: String,

        /// Output directory
        #[arg(short, long, required = true)]
        output: PathBuf,
    },

    /// Build custom kernel with libkrunfw
    BuildKernel {
        /// Output directory
        #[arg(short, long, required = true)]
        output: PathBuf,

        /// Enable AMD SEV support
        #[arg(long)]
        sev: bool,

        /// Enable Intel TDX support
        #[arg(long)]
        tdx: bool,

        /// Kernel patches
        #[arg(long)]
        patches: Vec<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    // Create workdir if needed
    std::fs::create_dir_all(&cli.workdir)?;

    match cli.command {
        Commands::Run {
            image,
            cpus,
            memory,
            volumes,
            publish,
            kernel,
            firmware,
            command,
        } => {
            let runner = ContainerVmRunner::new(&cli.workdir)?;

            let config = VmConfig {
                cpus,
                memory_mib: memory,
                kernel,
                firmware,
                cmd: if command.is_empty() {
                    vec!["/sbin/init".to_string()]
                } else {
                    command
                },
                volumes: parse_volumes(&volumes)?,
                ports: parse_ports(&publish)?,
                ..Default::default()
            };

            runner.run(&image, config).await?;
        }

        Commands::CreateImage { image, output, size, format } => {
            let builder = RootFsBuilder::new(&cli.workdir)?;

            // Build rootfs from image
            let _rootfs = builder.from_oci_image(&image).await?;

            // Create disk image
            let disk_path = builder.create_disk_image(&output, size)?;

            println!("Created disk image: {:?}", disk_path);
        }

        Commands::BuildRootfs { image, output } => {
            let builder = RootFsBuilder::new(&cli.workdir)?;
            let rootfs = builder.from_oci_image(&image).await?;

            // Copy to output
            std::fs::create_dir_all(&output)?;
            for entry in std::fs::read_dir(&rootfs)? {
                let entry = entry?;
                let src = entry.path();
                let dst = output.join(entry.file_name());

                if src.is_dir() {
                    copy_dir_all(&src, &dst)?;
                } else {
                    std::fs::copy(&src, &dst)?;
                }
            }

            println!("Built rootfs at: {:?}", output);
        }

        Commands::BuildKernel { output, sev, tdx, patches } => {
            let builder = KernelBuilder::new(&cli.workdir)?;
            builder.clone_libkrunfw()?;

            let config = KernelConfig {
                sev_enabled: sev,
                tdx_enabled: tdx,
                patches,
                ..Default::default()
            };

            let kernel_path = builder.build_kernel(&config)?;
            println!("Built kernel at: {:?}", kernel_path);
        }
    }

    Ok(())
}

fn parse_volumes(specs: &[String]) -> anyhow::Result<Vec<VolumeMount>> {
    specs
        .iter()
        .map(|s| {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() < 2 {
                anyhow::bail!("Invalid volume spec: {}", s);
            }
            Ok(VolumeMount {
                host_path: parts[0].into(),
                guest_path: parts[1].into(),
                read_only: parts.get(2).copied() == Some("ro"),
            })
        })
        .collect()
}

fn parse_ports(specs: &[String]) -> anyhow::Result<Vec<PortMapping>> {
    specs
        .iter()
        .map(|s| {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 2 {
                anyhow::bail!("Invalid port spec: {}", s);
            }
            Ok(PortMapping {
                host_port: parts[0].parse()?,
                guest_port: parts[1].parse()?,
            })
        })
        .collect()
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
```

## Advanced Topics

### ContainerVmRunner - High-Level API

```rust
// src/vm/runner.rs
use super::{VmContext, VmConfig};
use crate::rootfs::RootFsBuilder;
use std::path::Path;

pub struct ContainerVmRunner {
    rootfs_builder: RootFsBuilder,
}

impl ContainerVmRunner {
    pub fn new(workdir: impl AsRef<Path>) -> Result<Self, crate::rootfs::RootFsError> {
        Ok(Self {
            rootfs_builder: RootFsBuilder::new(workdir)?,
        })
    }

    /// Run container from OCI image in VM
    pub async fn run(
        &self,
        image_name: &str,
        mut config: VmConfig,
    ) -> Result<i32, ContainerVmError> {
        // Build rootfs from image
        let rootfs = self.rootfs_builder.from_oci_image(image_name).await?;
        config.rootfs = rootfs;

        // Create VM context
        let vm_ctx = VmContext::new()?;

        // Configure VM
        vm_ctx.configure(&config)?;

        // Set execution command
        vm_ctx.set_exec(&config.cmd, &config.env)?;

        // Start VM (this may not return)
        vm_ctx.start()
    }

    /// Create bootable disk image from OCI image
    pub async fn create_image(
        &self,
        image_name: &str,
        output_path: impl AsRef<Path>,
        size_mb: u32,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Build rootfs
        let _rootfs = self.rootfs_builder.from_oci_image(image_name).await?;

        // Create disk image
        let disk_path = self.rootfs_builder.create_disk_image(output_path, size_mb)?;

        Ok(disk_path)
    }
}
```

### SEV/TDX Confidential VMs

For confidential computing with AMD SEV or Intel TDX:

```rust
// Example SEV configuration
let config = VmConfig {
    cpus: 1,  // TDX limitation: max 1 vCPU
    memory_mib: 2048,  // TDX limitation: max 3072 MiB
    kernel: Some(PathBuf::from("/path/to/libkrunfw-tdx.so")),
    firmware: Some(PathBuf::from("/path/to/OVMF.fd")),
    ..Default::default()
};

// For SEV, use libkrun-sev variant
let ctx = VmContext::new_sev()?;  // Would need special constructor
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_rootfs_builder() {
        let workdir = tempdir().unwrap();
        let builder = RootFsBuilder::new(workdir.path()).unwrap();

        let rootfs = builder.from_oci_image("alpine:latest").await.unwrap();

        assert!(rootfs.exists());
        assert!(rootfs.join("bin/sh").exists());
    }

    #[test]
    fn test_vm_context_creation() {
        let ctx = VmContext::new().unwrap();
        assert!(ctx.ctx_id >= 0);
    }

    #[test]
    fn test_disk_image_creation() {
        let workdir = tempdir().unwrap();
        let builder = RootFsBuilder::new(workdir.path()).unwrap();

        let disk = builder.create_disk_image(
            workdir.path().join("test.raw"),
            64,
        ).unwrap();

        assert!(disk.exists());
        assert_eq!(disk.metadata().unwrap().len(), 64 * 1024 * 1024);
    }
}
```

## References

- [libkrun API Header](../../src.containers/libkrun/include/libkrun.h)
- [libkrunfw README](../../src.containers/libkrunfw/README.md)
- [OCI Specification](https://github.com/opencontainers/image-spec)
- [rust-vmm Crates](https://github.com/rust-vmm)
