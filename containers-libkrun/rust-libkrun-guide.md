---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.containers/
revised_at: 2026-03-19
---

# Rust Guide: Building Container VM Images with libkrun

This guide covers how to use libkrun from Rust to create and manage container images, root filesystems, and Linux boot images for container workloads.

## Overview

libkrun provides a Rust-friendly C API that can be used to:

1. **Build container root filesystems** from OCI images
2. **Create bootable disk images** for VM-based containers
3. **Launch container workloads** in lightweight VMs
4. **Manage VM lifecycle** with minimal overhead

## Architecture

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
       │  libkrunfw │ │  buildah  │  │  libkrun   │
       │  (Kernel)  │ │  (Images) │  │   (VMM)    │
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
# libkrun bindings (you may need to create these)
libkrun-sys = { path = "../libkrun/src/krun-sys" }

# OCI image handling
oci-spec = "0.6"
dockerfile-parser = "0.8"

# Disk image creation
loopdev = "0.4"
ext4-std = "0.1"

# Tar handling for rootfs
tar = "0.4"
flate2 = "1.0"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

### Build.rs (for libkrun linkage)

```rust
// build.rs
fn main() {
    // Link against libkrun
    println!("cargo:rustc-link-lib=dylib=krun");

    // Set library search path
    println!("cargo:rustc-link-search=native=/usr/local/lib");

    // Rebuild if libkrun changes
    println!("cargo:rerun-if-changed=/usr/local/include/libkrun.h");
}
```

## Core Data Structures

### VM Configuration

```rust
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
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub cpus: u8,
    pub memory_mib: u32,
    pub rootfs: PathBuf,
    pub kernel: Option<PathBuf>,
    pub cmd: Vec<String>,
    pub env: Vec<String>,
    pub volumes: Vec<VolumeMount>,
    pub ports: Vec<PortMapping>,
    pub gpu_enabled: bool,
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

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            cpus: 2,
            memory_mib: 1024,
            rootfs: PathBuf::from("/"),
            kernel: None,
            cmd: vec!["/bin/sh".to_string()],
            env: vec![],
            volumes: vec![],
            ports: vec![],
            gpu_enabled: false,
        }
    }
}
```

## Root Filesystem Builder

### Creating RootFS from OCI Image

```rust
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use tar::{Archive, Entry};
use flate2::read::GzDecoder;
use oci_spec::image::{ImageManifest, Config};

pub struct RootFsBuilder {
    workdir: PathBuf,
    layers_dir: PathBuf,
    rootfs_dir: PathBuf,
}

impl RootFsBuilder {
    pub fn new(workdir: impl AsRef<Path>) -> anyhow::Result<Self> {
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
    ) -> anyhow::Result<PathBuf> {
        // Pull image using docker/podman API or directly from registry
        let image_path = self.pull_image(image_name).await?;

        // Extract layers
        self.extract_layers(&image_path)?;

        Ok(self.rootfs_dir.clone())
    }

    /// Extract all layers from image
    fn extract_layers(&self, image_path: &Path) -> anyhow::Result<()> {
        // Read image manifest
        let manifest = self.read_manifest(image_path)?;

        // Extract each layer in order
        for layer in manifest.layers() {
            let layer_path = self.layers_dir.join(&layer.digest().to_string().replace(":", "_"));

            // Layers are typically gzipped tar
            let file = File::open(&layer_path)?;
            let reader = BufReader::new(file);
            let mut decoder = GzDecoder::new(reader);
            let mut archive = Archive::new(&mut decoder);

            // Extract to rootfs
            archive.unpack(&self.rootfs_dir)?;
        }

        Ok(())
    }

    /// Create ext4 disk image from rootfs
    pub fn create_disk_image(
        &self,
        output_path: impl AsRef<Path>,
        size_mb: u32,
    ) -> anyhow::Result<PathBuf> {
        let output_path = output_path.as_ref().to_path_buf();

        // Create sparse file
        let file = File::create(&output_path)?;
        file.set_len((size_mb as u64) * 1024 * 1024)?;

        // Format as ext4
        // Note: In practice, you'd use a library or call mkfs.ext4
        self.format_ext4(&output_path)?;

        // Copy rootfs contents
        self.copy_rootfs_to_image(&output_path)?;

        Ok(output_path)
    }

    fn format_ext4(&self, device: &Path) -> anyhow::Result<()> {
        // Use libext2fs bindings or call mkfs.ext4
        // For production, use a proper Rust library
        std::process::Command::new("mkfs.ext4")
            .arg("-F")
            .arg(device)
            .status()?;
        Ok(())
    }

    fn copy_rootfs_to_image(&self, device: &Path) -> anyhow::Result<()> {
        // Mount image and copy files
        // In production, use direct ext4 writing
        std::process::Command::new("guestfsd")
            .arg("--add")
            .arg(device)
            .arg("--mount")
            .arg(&self.rootfs_dir)
            .status()?;
        Ok(())
    }

    async fn pull_image(&self, image_name: &str) -> anyhow::Result<PathBuf> {
        // Implement image pulling from registry
        // Can use docker daemon API or direct registry calls
        todo!("Implement OCI registry pull")
    }

    fn read_manifest(&self, image_path: &Path) -> anyhow::Result<ImageManifest> {
        // Read and parse OCI manifest
        todo!("Implement manifest parsing")
    }
}
```

## libkrun VM Manager

### VM Context Wrapper

```rust
use libkrun_sys::*;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

pub struct VmContext {
    ctx_id: i32,
}

impl VmContext {
    pub fn new() -> anyhow::Result<Self> {
        let ctx_id = unsafe { krun_create_ctx() };
        if ctx_id < 0 {
            anyhow::bail!("Failed to create libkrun context: {}", ctx_id);
        }

        Ok(Self { ctx_id })
    }

    pub fn configure(&self, config: &VmConfig) -> anyhow::Result<()> {
        // Set VM resources
        let ret = unsafe {
            krun_set_vm_config(
                self.ctx_id as u32,
                config.cpus,
                config.memory_mib,
            )
        };
        if ret < 0 {
            anyhow::bail!("Failed to set VM config: {}", ret);
        }

        // Set root filesystem
        let rootfs_cstr = CString::new(config.rootfs.as_os_str().as_bytes())?;
        let ret = unsafe { krun_set_root(self.ctx_id as u32, rootfs_cstr.as_ptr()) };
        if ret < 0 {
            anyhow::bail!("Failed to set rootfs: {}", ret);
        }

        // Configure volumes
        for volume in &config.volumes {
            self.add_virtiofs(volume)?;
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

    fn add_virtiofs(&self, volume: &VolumeMount) -> anyhow::Result<()> {
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
            anyhow::bail!("Failed to add virtiofs: {}", ret);
        }

        Ok(())
    }

    fn configure_ports(&self, ports: &[PortMapping]) -> anyhow::Result<()> {
        // Convert port mappings to C strings
        let port_strings: Vec<CString> = ports
            .iter()
            .map(|p| CString::new(format!("{}:{}", p.host_port, p.guest_port)))
            .collect::<Result<_, _>>()?;

        let port_ptrs: Vec<*const i8> = port_strings.iter().map(|s| s.as_ptr()).collect();

        let ret = unsafe {
            krun_set_port_map(self.ctx_id as u32, port_ptrs.as_ptr())
        };

        if ret < 0 {
            anyhow::bail!("Failed to configure ports: {}", ret);
        }

        Ok(())
    }

    fn configure_gpu(&self) -> anyhow::Result<()> {
        // Enable virtio-gpu with virglrenderer
        let ret = unsafe {
            krun_set_gpu_options(self.ctx_id as u32, 0)
        };

        if ret < 0 {
            anyhow::bail!("Failed to configure GPU: {}", ret);
        }

        Ok(())
    }

    pub fn set_exec(&self, cmd: &[String], env: &[String]) -> anyhow::Result<()> {
        let cmd_cstrings: Vec<CString> = cmd
            .iter()
            .map(|s| CString::new(s.as_bytes()))
            .collect::<Result<_, _>>()?;

        let cmd_ptrs: Vec<*const i8> = cmd_cstrings.iter().map(|s| s.as_ptr()).collect();

        let env_cstrings: Vec<CString> = env
            .iter()
            .map(|s| CString::new(s.as_bytes()))
            .collect::<Result<_, _>>()?;

        let env_ptrs: Vec<*const i8> = env_cstrings.iter().map(|s| s.as_ptr()).collect();

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
            anyhow::bail!("Failed to set exec: {}", ret);
        }

        Ok(())
    }

    pub fn start(self) -> anyhow::Result<()> {
        // This consumes the context and enters the VM
        let ret = unsafe { krun_start_enter(self.ctx_id as u32) };

        if ret < 0 {
            anyhow::bail!("VM exited with error: {}", ret);
        }

        // Note: This typically doesn't return on success
        // as libkrun takes over the process
        Ok(())
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

## Container VM Runner

### High-Level API

```rust
pub struct ContainerVmRunner {
    rootfs_builder: RootFsBuilder,
}

impl ContainerVmRunner {
    pub fn new(workdir: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(Self {
            rootfs_builder: RootFsBuilder::new(workdir)?,
        })
    }

    /// Run container from OCI image in VM
    pub async fn run(
        &self,
        image_name: &str,
        config: VmConfig,
    ) -> anyhow::Result<i32> {
        // Build rootfs from image
        let rootfs = self.rootfs_builder.from_oci_image(image_name).await?;

        // Create VM context
        let vm_ctx = VmContext::new()?;

        // Configure VM
        let mut vm_config = config;
        vm_config.rootfs = rootfs;
        vm_ctx.configure(&vm_config)?;

        // Set execution command
        vm_ctx.set_exec(&vm_config.cmd, &vm_config.env)?;

        // Start VM (this may not return)
        vm_ctx.start()?;

        Ok(0)
    }

    /// Create bootable disk image from OCI image
    pub async fn create_image(
        &self,
        image_name: &str,
        output_path: impl AsRef<Path>,
        size_mb: u32,
    ) -> anyhow::Result<PathBuf> {
        // Build rootfs
        let _rootfs = self.rootfs_builder.from_oci_image(image_name).await?;

        // Create disk image
        let disk_path = self.rootfs_builder.create_disk_image(
            output_path,
            size_mb,
        )?;

        Ok(disk_path)
    }
}
```

## Example: Container VM Builder CLI

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vm-container")]
#[command(about = "Build and run container VMs with libkrun")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run container in VM
    Run {
        /// OCI image name
        image: String,

        /// Number of CPUs
        #[arg(short, long, default_value = "2")]
        cpus: u8,

        /// Memory in MiB
        #[arg(short, long, default_value = "1024")]
        memory: u32,

        /// Volume mounts (host:guest)
        #[arg(short, long)]
        volume: Vec<String>,

        /// Port mappings (host:guest)
        #[arg(short, long)]
        publish: Vec<String>,

        /// Command to run
        #[arg(last = true)]
        command: Vec<String>,
    },

    /// Create bootable disk image
    CreateImage {
        /// OCI image name
        image: String,

        /// Output path
        #[arg(short, long)]
        output: PathBuf,

        /// Image size in MiB
        #[arg(short, long, default_value = "4096")]
        size: u32,
    },

    /// Build root filesystem
    BuildRootfs {
        /// OCI image name
        image: String,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            image,
            cpus,
            memory,
            volume,
            publish,
            command,
        } => {
            let runner = ContainerVmRunner::new("/tmp/vm-container")?;

            let config = VmConfig {
                cpus,
                memory_mib: memory,
                cmd: if command.is_empty() {
                    vec!["/bin/sh".to_string()]
                } else {
                    command
                },
                volumes: parse_volumes(&volume)?,
                ports: parse_ports(&publish)?,
                ..Default::default()
            };

            runner.run(&image, config).await?;
        }

        Commands::CreateImage { image, output, size } => {
            let runner = ContainerVmRunner::new("/tmp/vm-container")?;
            let path = runner.create_image(&image, output, size).await?;
            println!("Created disk image: {:?}", path);
        }

        Commands::BuildRootfs { image, output } => {
            let builder = RootFsBuilder::new("/tmp/vm-container")?;
            let rootfs = builder.from_oci_image(&image).await?;
            println!("Built rootfs at: {:?}", rootfs);
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
```

## Advanced: Custom Kernel Integration

### Using libkrunfw

```rust
use std::path::Path;

pub struct KernelBuilder {
    workdir: PathBuf,
}

impl KernelBuilder {
    pub fn new(workdir: impl AsRef<Path>) -> Self {
        Self {
            workdir: workdir.as_ref().to_path_buf(),
        }
    }

    /// Build custom kernel for libkrun
    pub fn build_kernel(&self, config: &KernelConfig) -> anyhow::Result<PathBuf> {
        // Clone libkrunfw
        self.clone_libkrunfw()?;

        // Apply patches
        self.apply_patches(&config.patches)?;

        // Configure kernel
        self.configure_kernel(config)?;

        // Build
        self.compile_kernel()?;

        // Package as libkrunfw
        self.package_libkrunfw()
    }

    fn clone_libkrunfw(&self) -> anyhow::Result<()> {
        std::process::Command::new("git")
            .arg("clone")
            .arg("https://github.com/containers/libkrunfw")
            .arg(self.workdir.join("libkrunfw"))
            .status()?;
        Ok(())
    }

    fn apply_patches(&self, patches: &[PathBuf]) -> anyhow::Result<()> {
        for patch in patches {
            std::process::Command::new("patch")
                .arg("-p1")
                .arg("-i")
                .arg(patch)
                .current_dir(&self.workdir.join("libkrunfw/linux"))
                .status()?;
        }
        Ok(())
    }

    fn configure_kernel(&self, config: &KernelConfig) -> anyhow::Result<()> {
        // Generate .config for kernel
        let mut kconfig = std::process::Command::new("make");
        kconfig.arg("defconfig");

        if config.sev_enabled {
            kconfig.arg("sev_defconfig");
        } else if config.tdx_enabled {
            kconfig.arg("tdx_defconfig");
        }

        kconfig.current_dir(&self.workdir.join("libkrunfw/linux"))
            .status()?;

        Ok(())
    }

    fn compile_kernel(&self) -> anyhow::Result<()> {
        std::process::Command::new("make")
            .arg("-j")
            .arg(&format!("{}", num_cpus::get()))
            .current_dir(&self.workdir.join("libkrunfw"))
            .status()?;
        Ok(())
    }

    fn package_libkrunfw(&self) -> anyhow::Result<PathBuf> {
        // The Makefile in libkrunfw handles packaging
        Ok(self.workdir.join("libkrunfw/libkrunfw.so"))
    }
}

pub struct KernelConfig {
    pub sev_enabled: bool,
    pub tdx_enabled: bool,
    pub patches: Vec<PathBuf>,
    pub extra_config: Vec<String>,
}
```

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rootfs_builder() {
        let workdir = tempfile::tempdir().unwrap();
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
}
```

## References

- [libkrun API](../../src.containers/libkrun/include/libkrun.h)
- [OCI Specification](https://github.com/opencontainers/image-spec)
- [libkrunfw](../../src.containers/libkrunfw/README.md)
