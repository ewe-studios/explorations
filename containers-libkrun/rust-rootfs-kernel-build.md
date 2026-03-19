---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.containers/
revised_at: 2026-03-19
---

# Rust Guide: Root Filesystem and Kernel Creation for libkrun

This comprehensive guide covers creating root filesystems and custom kernels for libkrun-based microVMs using Rust.

## Table of Contents

1. [Overview](#overview)
2. [Root Filesystem Creation](#root-filesystem-creation)
3. [Kernel Building with libkrunfw](#kernel-building-with-libkrunfw)
4. [Initramfs Generation](#initramfs-generation)
5. [Complete Build Pipeline](#complete-build-pipeline)
6. [Advanced Topics](#advanced-topics)

## Overview

Building a bootable libkrun VM requires three components:

```
┌─────────────────────────────────────────────────────────────┐
│                    Boot Components                          │
├─────────────────────────────────────────────────────────────┤
│  1. Kernel (libkrunfw)                                      │
│     - Bundled Linux kernel as dynamic library               │
│     - Configured for minimal footprint                      │
│     - Patches for libkrun-specific features                 │
├─────────────────────────────────────────────────────────────┤
│  2. Root Filesystem                                         │
│     - From OCI image layers                                 │
│     - Custom-built minimal rootfs                           │
│     - ext4 disk image or virtio-fs directory                │
├─────────────────────────────────────────────────────────────┤
│  3. Initramfs (Optional)                                    │
│     - Early userspace for boot                              │
│     - Contains init process                                 │
│     - Handles rootfs switch                                 │
└─────────────────────────────────────────────────────────────┘
```

## Root Filesystem Creation

### OCI Image-Based RootFS

The most common approach is extracting rootfs from OCI images:

```rust
// src/rootfs/oci_builder.rs
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Read, Write};
use tar::Archive;
use flate2::read::GzDecoder;
use xz2::read::XzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;
use oci_spec::image::{ImageManifest, Descriptor, ImageConfiguration};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RootFsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("OCI error: {0}")]
    Oci(#[from] oci_spec::error::Error),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Layer extraction failed: {0}")]
    LayerExtract(String),
}

pub struct OciRootFsBuilder {
    workdir: PathBuf,
    layers_dir: PathBuf,
    rootfs_dir: PathBuf,
}

impl OciRootFsBuilder {
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

    /// Build rootfs from OCI image reference
    pub async fn from_image(&self, image_ref: &str) -> Result<PathBuf, RootFsError> {
        tracing::info!("Building rootfs from OCI image: {}", image_ref);

        // Parse image reference
        let (registry, repository, reference) = self.parse_image_ref(image_ref)?;

        // Pull manifest
        let manifest = self.pull_manifest(&registry, &repository, &reference).await?;

        // Get config for additional metadata
        let config = self.pull_config(&registry, &repository, &manifest).await?;

        // Download and extract layers
        self.extract_layers(&manifest, &registry, &repository).await?;

        // Apply image configuration (CMD, ENV, etc.)
        self.apply_config(&config)?;

        Ok(self.rootfs_dir.clone())
    }

    fn parse_image_ref(&self, image_ref: &str) -> Result<(String, String, String), RootFsError> {
        // Handle various image reference formats:
        // - alpine:latest
        // - library/alpine:latest
        // - docker.io/library/alpine:latest
        // - ghcr.io/user/repo:tag
        // - registry@sha256:digest

        if image_ref.contains('@') {
            // Digest reference
            let parts: Vec<&str> = image_ref.split('@').collect();
            let (name, digest) = (parts[0], parts[1]);
            let (registry, repo) = if name.contains('/') && !name.contains('.') && !name.contains(':') {
                ("docker.io".to_string(), name.to_string())
            } else if !name.contains('/') {
                ("docker.io".to_string(), format!("library/{}", name))
            } else {
                ("docker.io".to_string(), name.to_string())
            };
            return Ok((registry, repo, digest.to_string()));
        }

        let parts: Vec<&str> = image_ref.split('/').collect();

        let (registry, repository, tag) = match parts.len() {
            1 => {
                // "alpine" -> docker.io/library/alpine:latest
                ("docker.io".to_string(), "library".to_string(), parts[0].to_string())
            }
            2 => {
                // "library/alpine" or "ghcr.io/user"
                if parts[0].contains('.') || parts[0].contains(':') {
                    // Has registry
                    (parts[0].to_string(), String::new(), parts[1].to_string())
                } else {
                    // No registry
                    ("docker.io".to_string(), parts[0].to_string(), parts[1].to_string())
                }
            }
            3 => {
                // "docker.io/library/alpine"
                (parts[0].to_string(), parts[1].to_string(), parts[2].to_string())
            }
            _ => return Err(RootFsError::Registry("Invalid image reference".into())),
        };

        // Add :latest if no tag
        let tag = if tag.contains(':') { tag } else { format!("{}:latest", tag) };

        Ok((registry, repository, tag))
    }

    async fn pull_manifest(
        &self,
        registry: &str,
        repository: &str,
        reference: &str,
    ) -> Result<ImageManifest, RootFsError> {
        let url = format!("https://{}/v2/{}/manifests/{}", registry, repository, reference);

        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| RootFsError::Registry(e.to_string()))?;

        let response = client
            .get(&url)
            .header("Accept", "application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.v2+json")
            .send()
            .await
            .map_err(|e| RootFsError::Registry(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RootFsError::Registry(format!(
                "Registry returned {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let manifest: ImageManifest = response
            .json()
            .await
            .map_err(|e| RootFsError::Registry(e.to_string()))?;

        Ok(manifest)
    }

    async fn pull_config(
        &self,
        registry: &str,
        repository: &str,
        manifest: &ImageManifest,
    ) -> Result<ImageConfiguration, RootFsError> {
        let config_descriptor = manifest.config();
        let digest = config_descriptor.digest();

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

        let config: ImageConfiguration = serde_json::from_slice(&bytes)
            .map_err(|e| RootFsError::Registry(e.to_string()))?;

        Ok(config)
    }

    async fn extract_layers(
        &self,
        manifest: &ImageManifest,
        registry: &str,
        repository: &str,
    ) -> Result<(), RootFsError> {
        for (i, layer) in manifest.layers().iter().enumerate() {
            tracing::debug!("Extracting layer {}/{}", i + 1, manifest.layers().len());

            let layer_data = self.download_layer(registry, repository, layer).await?;
            self.extract_layer_data(&layer_data)?;
        }

        Ok(())
    }

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

    fn extract_layer_data(&self, data: &[u8]) -> Result<(), RootFsError> {
        // Detect compression from magic bytes
        let decoder: Box<dyn Read> = match data.get(0..6) {
            Some([0x1f, 0x8b, _, _, _, _]) => {
                // Gzip
                Box::new(GzDecoder::new(data))
            }
            Some([0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]) => {
                // XZ
                Box::new(XzDecoder::new(data))
            }
            Some([0x28, 0xb5, 0x2f, 0xfd, _, _]) => {
                // Zstd
                Box::new(ZstdDecoder::new(data).map_err(|e| RootFsError::Io(e))?)
            }
            _ => Box::new(data),
        };

        let mut archive = Archive::new(decoder);

        for entry_result in archive.entries().map_err(|e| RootFsError::LayerExtract(e.to_string()))? {
            let mut entry = entry_result.map_err(|e| RootFsError::LayerExtract(e.to_string()))?;
            let path = entry.path().map_err(|e| RootFsError::LayerExtract(e.to_string()))?.to_path_buf();
            let entry_type = entry.header().entry_type();

            let target_path = self.rootfs_dir.join(&path);

            match entry_type {
                tar::EntryType::Regular => {
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let mut file = File::create(&target_path)?;
                    std::io::copy(&mut entry, &mut file)?;
                }
                tar::EntryType::Directory => {
                    fs::create_dir_all(&target_path)?;
                }
                tar::EntryType::Symlink => {
                    if let Ok(link_name) = entry.link_name() {
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let _ = fs::remove_file(&target_path);
                        std::os::unix::fs::symlink(link_name, &target_path)?;
                    }
                }
                tar::EntryType::Link => {
                    if let Ok(link_name) = entry.link_name() {
                        let link_target = self.rootfs_dir.join(link_name);
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let _ = fs::remove_file(&target_path);
                        std::os::unix::fs::hard_link(link_target, &target_path)?;
                    }
                }
                tar::EntryType::Char | tar::EntryType::Block => {
                    // Skip device nodes - not needed for most use cases
                    tracing::debug!("Skipping device node: {:?}", path);
                }
                tar::EntryType::Fifo => {
                    // Skip FIFOs
                    tracing::debug!("Skipping FIFO: {:?}", path);
                }
                _ => {
                    tracing::debug!("Skipping unknown entry type: {:?}", path);
                }
            }
        }

        Ok(())
    }

    fn apply_config(&self, config: &ImageConfiguration) -> Result<(), RootFsError> {
        let config = config.config();

        // Set environment variables (optional - stored for VM config)
        if let Some(env) = config.env() {
            tracing::debug!("Image ENV: {:?}", env);
        }

        // Get CMD/ENTRYPOINT (optional - stored for VM config)
        let cmd = config.cmd();
        let entrypoint = config.entrypoint();
        tracing::debug!("Image ENTRYPOINT: {:?}, CMD: {:?}", entrypoint, cmd);

        Ok(())
    }
}
```

### Minimal Custom RootFS

For specialized use cases, build a minimal rootfs from scratch:

```rust
// src/rootfs/minimal_builder.rs
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::Write;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MinimalRootFsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct MinimalRootFsConfig {
    /// Include busybox
    pub busybox: bool,
    /// Include basic shell scripts
    pub shell_scripts: bool,
    /// Include /proc, /sys, /dev directories
    pub system_dirs: bool,
    /// Include init script
    pub init_script: bool,
    /// Additional files to copy
    pub extra_files: Vec<(PathBuf, PathBuf)>,
}

impl Default for MinimalRootFsConfig {
    fn default() -> Self {
        Self {
            busybox: true,
            shell_scripts: true,
            system_dirs: true,
            init_script: true,
            extra_files: vec![],
        }
    }
}

pub struct MinimalRootFsBuilder {
    workdir: PathBuf,
}

impl MinimalRootFsBuilder {
    pub fn new(workdir: impl AsRef<Path>) -> Self {
        Self {
            workdir: workdir.as_ref().to_path_buf(),
        }
    }

    pub fn build(&self, config: &MinimalRootFsConfig) -> Result<PathBuf, MinimalRootFsError> {
        let rootfs_dir = self.workdir.join("minimal_rootfs");
        fs::create_dir_all(&rootfs_dir)?;

        tracing::info!("Building minimal rootfs at {:?}", rootfs_dir);

        // Create directory structure
        self.create_directories(&rootfs_dir, config)?;

        // Install busybox if requested
        if config.busybox {
            self.install_busybox(&rootfs_dir)?;
        }

        // Create init script
        if config.init_script {
            self.create_init_script(&rootfs_dir)?;
        }

        // Create shell scripts
        if config.shell_scripts {
            self.create_shell_scripts(&rootfs_dir)?;
        }

        // Copy extra files
        for (src, dst) in &config.extra_files {
            let target = rootfs_dir.join(dst);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src, target)?;
        }

        Ok(rootfs_dir)
    }

    fn create_directories(&self, rootfs: &Path, config: &MinimalRootFsConfig) -> Result<(), MinimalRootFsError> {
        let dirs = [
            "bin", "sbin", "usr/bin", "usr/sbin",
            "lib", "lib64",
            "etc", "etc/init.d",
            "var", "var/log", "var/run", "var/tmp",
            "tmp", "home", "root",
            "proc", "sys", "dev", "run", "mnt",
            "usr/lib", "usr/local", "usr/local/bin",
        ];

        for dir in dirs {
            fs::create_dir_all(rootfs.join(dir))?;
        }

        Ok(())
    }

    fn install_busybox(&self, rootfs: &Path) -> Result<(), MinimalRootFsError> {
        // Find busybox in PATH or use provided path
        let busybox_path = std::process::Command::new("which")
            .arg("busybox")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "/bin/busybox".to_string());

        let target_bin = rootfs.join("bin/busybox");
        fs::copy(&busybox_path, &target_bin)?;

        // Create symlinks for common applets
        let applets = [
            "sh", "bash", "ls", "cat", "cp", "mv", "rm", "mkdir",
            "chmod", "chown", "echo", "pwd", "cd", "grep", "sed",
            "awk", "head", "tail", "wc", "sort", "uniq", "cut",
            "tr", "xargs", "env", "ps", "kill", "sleep", "date",
            "mount", "umount", "ip", "ifconfig", "ping",
            "init", "halt", "reboot", "poweroff",
        ];

        for applet in applets {
            let link_path = match applet {
                "sh" => rootfs.join("bin/sh"),
                "init" => rootfs.join("sbin/init"),
                _ => rootfs.join(format!("bin/{}", applet)),
            };

            let _ = fs::remove_file(&link_path);
            std::os::unix::fs::symlink("busybox", &link_path)?;
        }

        Ok(())
    }

    fn create_init_script(&self, rootfs: &Path) -> Result<(), MinimalRootFsError> {
        let init_path = rootfs.join("init");
        let mut file = File::create(&init_path)?;

        writeln!(file, "#!/bin/sh")?;
        writeln!(file)?;
        writeln!(file, "# Mount essential filesystems")?;
        writeln!(file, "mount -t proc proc /proc -o nosuid,nodev,noexec")?;
        writeln!(file, "mount -t sysfs sysfs /sys -o nosuid,nodev,noexec")?;
        writeln!(file, "mount -t devtmpfs devtmpfs /dev -o nosuid")?;
        writeln!(file, "mount -t tmpfs tmpfs /run -o nosuid,nodev,mode=755")?;
        writeln!(file)?;
        writeln!(file, "# Set hostname")?;
        writeln!(file, "hostname libkrun-vm")?;
        writeln!(file)?;
        writeln!(file, "# Start shell or execute command")?;
        writeln!(file, "exec /bin/sh")?;

        fs::set_permissions(&init_path, std::os::unix::fs::PermissionsExt::from_mode(0o755))?;

        Ok(())
    }

    fn create_shell_scripts(&self, rootfs: &Path) -> Result<(), MinimalRootFsError> {
        // Create /etc/profile
        let profile_path = rootfs.join("etc/profile");
        let mut file = File::create(&profile_path)?;

        writeln!(file, "export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin")?;
        writeln!(file, "export HOME=/root")?;
        writeln!(file, "export TERM=xterm")?;
        writeln!(file, "export PS1='\\u@\\h:\\w\\$ '")?;

        Ok(())
    }
}
```

## Kernel Building with libkrunfw

### Building libkrunfw from Rust

```rust
// src/kernel/libkrunfw_builder.rs
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KernelBuildError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Build failed: {0}")]
    Build(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

#[derive(Debug, Clone)]
pub struct LibkrunfwConfig {
    /// Target architecture
    pub arch: String,
    /// Enable AMD SEV support
    pub sev_enabled: bool,
    /// Enable Intel TDX support
    pub tdx_enabled: bool,
    /// Additional kernel config options
    pub extra_config: Vec<String>,
    /// Kernel patches to apply
    pub patches: Vec<PathBuf>,
    /// Cross-compiler prefix (e.g., "aarch64-linux-gnu-")
    pub cross_compile: Option<String>,
    /// Number of parallel jobs
    pub jobs: Option<usize>,
}

impl Default for LibkrunfwConfig {
    fn default() -> Self {
        Self {
            arch: "x86_64".to_string(),
            sev_enabled: false,
            tdx_enabled: false,
            extra_config: vec![],
            patches: vec![],
            cross_compile: None,
            jobs: None,
        }
    }
}

pub struct LibkrunfwBuilder {
    workdir: PathBuf,
    output_dir: PathBuf,
    libkrunfw_dir: PathBuf,
}

impl LibkrunfwBuilder {
    pub fn new(workdir: impl AsRef<Path>) -> Result<Self, KernelBuildError> {
        let workdir = workdir.as_ref().to_path_buf();
        let output_dir = workdir.join("output");
        let libkrunfw_dir = workdir.join("libkrunfw");

        fs::create_dir_all(&output_dir)?;

        Ok(Self {
            workdir,
            output_dir,
            libkrunfw_dir,
        })
    }

    /// Clone or update libkrunfw repository
    pub fn init_repo(&self, branch: Option<&str>) -> Result<(), KernelBuildError> {
        if self.libkrunfw_dir.exists() {
            tracing::info!("Updating libkrunfw repository");
            Command::new("git")
                .arg("pull")
                .current_dir(&self.libkrunfw_dir)
                .status()
                .map_err(|e| KernelBuildError::Build(e.to_string()))?;
        } else {
            tracing::info!("Cloning libkrunfw repository");
            let mut cmd = Command::new("git");
            cmd.arg("clone")
                .arg("https://github.com/containers/libkrunfw")
                .arg(&self.libkrunfw_dir);

            if let Some(branch) = branch {
                cmd.arg("-b").arg(branch);
            }

            cmd.status()
                .map_err(|e| KernelBuildError::Build(e.to_string()))?;
        }

        Ok(())
    }

    /// Build libkrunfw library
    pub fn build(&self, config: &LibkrunfwConfig) -> Result<PathBuf, KernelBuildError> {
        let linux_dir = self.libkrunfw_dir.join("linux");

        // Apply patches
        self.apply_patches(&linux_dir, &config.patches)?;

        // Configure kernel
        self.configure_kernel(&linux_dir, config)?;

        // Compile kernel
        self.compile_kernel(&linux_dir, config)?;

        // Build libkrunfw library
        self.build_library(config)?;

        // Copy output
        let output_lib = self.get_output_library_name(config);
        let src = self.libkrunfw_dir.join(&output_lib);
        let dst = self.output_dir.join("libkrunfw.so");

        fs::copy(&src, &dst)?;

        tracing::info!("Built libkrunfw.so at {:?}", dst);

        Ok(dst)
    }

    fn apply_patches(&self, linux_dir: &Path, patches: &[PathBuf]) -> Result<(), KernelBuildError> {
        for patch in patches {
            tracing::info!("Applying patch: {:?}", patch);

            let status = Command::new("patch")
                .arg("-p1")
                .arg("-i")
                .arg(patch)
                .current_dir(linux_dir)
                .status()
                .map_err(|e| KernelBuildError::Build(e.to_string()))?;

            if !status.success() {
                return Err(KernelBuildError::Build(
                    format!("Failed to apply patch: {:?}", patch)
                ));
            }
        }

        Ok(())
    }

    fn configure_kernel(&self, linux_dir: &Path, config: &LibkrunfwConfig) -> Result<(), KernelBuildError> {
        tracing::info!("Configuring kernel for {}", config.arch);

        // Start with defconfig
        let mut cmd = Command::new("make");
        cmd.arg(format!("ARCH={}", config.arch));

        if let Some(ref cross) = config.cross_compile {
            cmd.arg(format!("CROSS_COMPILE={}", cross));
        }

        // Base defconfig
        cmd.arg("defconfig");

        // Apply architecture-specific configs
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

        cmd.current_dir(linux_dir)
            .status()
            .map_err(|e| KernelBuildError::Build(e.to_string()))?;

        // Apply extra config options
        self.apply_kernel_config(linux_dir, &config.extra_config)?;

        // Apply libkrun-specific required options
        let required_options = vec![
            "CONFIG_NR_CPUS=8".to_string(),      // Memory optimization
            "CONFIG_VIRTIO=y".to_string(),
            "CONFIG_VIRTIO_PCI=y".to_string(),
            "CONFIG_VIRTIO_BLK=y".to_string(),
            "CONFIG_VIRTIO_NET=y".to_string(),
            "CONFIG_VIRTIO_FS=y".to_string(),
            "CONFIG_VIRTIO_CONSOLE=y".to_string(),
            "CONFIG_VIRTIO_BALLOON=y".to_string(),
            "CONFIG_VIRTIO_RNG=y".to_string(),
            "CONFIG_VIRTIO_VSOCK=y".to_string(),
            "CONFIG_9P_FS=y".to_string(),
            "CONFIG_NET_9P=y".to_string(),
            "CONFIG_NET_9P_VIRTIO=y".to_string(),
            "CONFIG_SECURITY=y".to_string(),
            "CONFIG_SECURITY_SELINUX=y".to_string(),
        ];

        self.apply_kernel_config(linux_dir, &required_options)?;

        // Run olddefconfig to resolve conflicts
        let mut cmd = Command::new("make");
        cmd.arg("olddefconfig")
            .current_dir(linux_dir);

        if let Some(ref cross) = config.cross_compile {
            cmd.arg(format!("CROSS_COMPILE={}", cross));
        }

        cmd.status()
            .map_err(|e| KernelBuildError::Build(e.to_string()))?;

        Ok(())
    }

    fn apply_kernel_config(&self, linux_dir: &Path, options: &[String]) -> Result<(), KernelBuildError> {
        for option in options {
            if let Some((key, value)) = option.split_once('=') {
                // Try scripts/config first
                let config_cmd = Command::new("scripts/config")
                    .arg("--file")
                    .arg(".config")
                    .arg("--set-str")
                    .arg(key)
                    .arg(value)
                    .current_dir(linux_dir)
                    .status();

                if config_cmd.is_err() {
                    // Fallback to sed
                    let pattern = format!("s/^#?{}=.*/{}={}/", key, key, value);
                    Command::new("sed")
                        .arg("-i")
                        .arg(&pattern)
                        .arg(".config")
                        .current_dir(linux_dir)
                        .status()
                        .map_err(|e| KernelBuildError::Build(e.to_string()))?;
                }
            } else {
                // Enable/disable option
                let enable = !option.starts_with('#');
                let key = option.trim_start_matches('#');

                Command::new("scripts/config")
                    .arg("--file")
                    .arg(".config")
                    .arg(if enable { "--enable" } else { "--disable" })
                    .arg(key)
                    .current_dir(linux_dir)
                    .status()
                    .ok();
            }
        }

        Ok(())
    }

    fn compile_kernel(&self, linux_dir: &Path, config: &LibkrunfwConfig) -> Result<(), KernelBuildError> {
        tracing::info!("Compiling kernel");

        let mut cmd = Command::new("make");
        cmd.arg(format!("ARCH={}", config.arch));

        if let Some(ref cross) = config.cross_compile {
            cmd.arg(format!("CROSS_COMPILE={}", cross));
        }

        let jobs = config.jobs.unwrap_or_else(|| num_cpus::get());
        cmd.arg(format!("-j{}", jobs));

        let status = cmd.current_dir(linux_dir)
            .status()
            .map_err(|e| KernelBuildError::Build(e.to_string()))?;

        if !status.success() {
            return Err(KernelBuildError::Build("Kernel compilation failed".into()));
        }

        Ok(())
    }

    fn build_library(&self, config: &LibkrunfwConfig) -> Result<(), KernelBuildError> {
        tracing::info!("Building libkrunfw library");

        let mut cmd = Command::new("make");

        if config.sev_enabled {
            cmd.arg("SEV=1");
        } else if config.tdx_enabled {
            cmd.arg("TDX=1");
        }

        let status = cmd.current_dir(&self.libkrunfw_dir)
            .status()
            .map_err(|e| KernelBuildError::Build(e.to_string()))?;

        if !status.success() {
            return Err(KernelBuildError::Build("libkrunfw build failed".into()));
        }

        Ok(())
    }

    fn get_output_library_name(&self, config: &LibkrunfwConfig) -> String {
        if config.sev_enabled {
            "libkrunfw-sev.so".to_string()
        } else if config.tdx_enabled {
            "libkrunfw-tdx.so".to_string()
        } else {
            "libkrunfw.so".to_string()
        }
    }

    /// Build kernel without bundling into libkrunfw (raw kernel image)
    pub fn build_raw_kernel(&self, config: &LibkrunfwConfig) -> Result<PathBuf, KernelBuildError> {
        let linux_dir = self.libkrunfw_dir.join("linux");

        self.apply_patches(&linux_dir, &config.patches)?;
        self.configure_kernel(&linux_dir, config)?;
        self.compile_kernel(&linux_dir, config)?;

        // Copy bzImage or Image
        let kernel_image = match config.arch.as_str() {
            "x86_64" => linux_dir.join("arch/x86/boot/bzImage"),
            "aarch64" => linux_dir.join("arch/arm64/boot/Image"),
            _ => linux_dir.join("vmlinux"),
        };

        let output = self.output_dir.join(format!("vmlinux-{}", config.arch));
        fs::copy(&kernel_image, &output)?;

        Ok(output)
    }
}
```

## Initramfs Generation

### Creating Initramfs from RootFS

```rust
// src/kernel/initramfs.rs
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InitramfsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cpio error: {0}")]
    Cpio(String),
}

pub struct InitramfsBuilder {
    workdir: PathBuf,
}

impl InitramfsBuilder {
    pub fn new(workdir: impl AsRef<Path>) -> Self {
        Self {
            workdir: workdir.as_ref().to_path_buf(),
        }
    }

    /// Create initramfs from directory
    pub fn from_dir(&self, rootfs_dir: &Path, output: &Path) -> Result<PathBuf, InitramfsError> {
        tracing::info!("Creating initramfs from {:?}", rootfs_dir);

        // Ensure output parent directory exists
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }

        // Method 1: Using find + cpio + gzip (most compatible)
        let status = Command::new("find")
            .arg(".")
            .arg("-print0")
            .current_dir(rootfs_dir)
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut find| {
                let find_stdout = find.stdout.take().unwrap();

                let mut cpio = Command::new("cpio")
                    .arg("--null")
                    .arg("-o")
                    .arg("--format=newc")
                    .arg("-H")
                    .arg("newc")
                    .current_dir(rootfs_dir)
                    .stdin(find_stdout)
                    .stdout(std::process::Stdio::piped())
                    .spawn()?;

                let cpio_stdout = cpio.stdout.take().unwrap();

                let mut gzip = Command::new("gzip")
                    .arg("-9")
                    .arg("-c")
                    .stdin(cpio_stdout)
                    .stdout(File::create(output)?)
                    .spawn()?;

                let output = gzip.wait()?;
                Ok(output)
            })
            .map_err(|e| InitramfsError::Cpio(e.to_string()))?;

        if !status.success() {
            return Err(InitramfsError::Cpio(
                format!("initramfs creation failed: {}", status)
            ));
        }

        tracing::info!("Created initramfs at {:?}", output);

        Ok(output.to_path_buf())
    }

    /// Create minimal initramfs with just init
    pub fn minimal(&self, output: &Path) -> Result<PathBuf, InitramfsError> {
        let temp_dir = self.workdir.join("minimal_initramfs");
        fs::create_dir_all(&temp_dir)?;

        // Create minimal directory structure
        let dirs = ["bin", "proc", "sys", "dev", "etc"];
        for dir in dirs {
            fs::create_dir_all(temp_dir.join(dir))?;
        }

        // Create init script
        let init_path = temp_dir.join("init");
        let mut file = File::create(&init_path)?;

        writeln!(file, "#!/bin/sh")?;
        writeln!(file, "mount -t proc proc /proc")?;
        writeln!(file, "mount -t sysfs sysfs /sys")?;
        writeln!(file, "mount -t devtmpfs devtmpfs /dev")?;
        writeln!(file, "exec /bin/sh")?;

        fs::set_permissions(&init_path, std::os::unix::fs::PermissionsExt::from_mode(0o755))?;

        // Copy busybox if available
        if let Ok(busybox) = std::process::Command::new("which").arg("busybox").output() {
            if let Ok(path) = String::from_utf8(busybox.stdout) {
                let target = temp_dir.join("bin/busybox");
                fs::copy(path.trim(), target).ok();

                // Create applet symlinks
                let applets = ["sh", "mount", "cat", "ls"];
                for applet in applets {
                    let _ = std::os::unix::fs::symlink(
                        "busybox",
                        temp_dir.join(format!("bin/{}", applet))
                    );
                }
            }
        }

        // Create initramfs
        self.from_dir(&temp_dir, output)
    }
}
```

## Complete Build Pipeline

### Putting It All Together

```rust
// src/pipeline/mod.rs
use crate::rootfs::{OciRootFsBuilder, MinimalRootFsBuilder, MinimalRootFsConfig};
use crate::kernel::{LibkrunfwBuilder, LibkrunfwConfig, InitramfsBuilder};
use crate::disk_image::DiskImageCreator;
use std::path::{Path, PathBuf};

pub struct VmImagePipeline {
    workdir: PathBuf,
    output_dir: PathBuf,
}

pub struct VmImagePipelineConfig {
    /// Source: OCI image or minimal
    pub source: ImageSource,
    /// Kernel configuration
    pub kernel_config: LibkrunfwConfig,
    /// Disk image size in MB
    pub disk_size_mb: u32,
    /// Output format
    pub format: DiskFormat,
}

pub enum ImageSource {
    Oci(String),
    Minimal(MinimalRootFsConfig),
}

pub enum DiskFormat {
    Raw,
    Qcow2,
    WithKernel,  // Bundles kernel into disk image
}

impl VmImagePipeline {
    pub fn new(workdir: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let workdir = workdir.as_ref().to_path_buf();
        let output_dir = workdir.join("output");
        fs::create_dir_all(&output_dir)?;

        Ok(Self {
            workdir,
            output_dir,
        })
    }

    /// Build complete VM image
    pub async fn build(&self, config: &VmImagePipelineConfig) -> Result<VmImageOutput, Box<dyn std::error::Error>> {
        // Step 1: Build rootfs
        let rootfs = self.build_rootfs(&config.source).await?;

        // Step 2: Build kernel (libkrunfw)
        let kernel = self.build_kernel(&config.kernel_config)?;

        // Step 3: Create initramfs
        let initramfs = self.create_initramfs(&rootfs)?;

        // Step 4: Create disk image
        let disk_image = self.create_disk_image(&rootfs, config)?;

        Ok(VmImageOutput {
            rootfs,
            kernel,
            initramfs,
            disk_image,
        })
    }

    async fn build_rootfs(&self, source: &ImageSource) -> Result<PathBuf, Box<dyn std::error::Error>> {
        match source {
            ImageSource::Oci(image_ref) => {
                let builder = OciRootFsBuilder::new(&self.workdir)?;
                builder.from_image(image_ref).await.map_err(|e| e.into())
            }
            ImageSource::Minimal(config) => {
                let builder = MinimalRootFsBuilder::new(&self.workdir);
                builder.build(config).map_err(|e| e.into())
            }
        }
    }

    fn build_kernel(&self, config: &LibkrunfwConfig) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let builder = LibkrunfwBuilder::new(&self.workdir)?;
        builder.init_repo(None)?;
        builder.build(config).map_err(|e| e.into())
    }

    fn create_initramfs(&self, rootfs: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let builder = InitramfsBuilder::new(&self.workdir);
        let output = self.output_dir.join("initramfs.cpio.gz");
        builder.from_dir(rootfs, &output).map_err(|e| e.into())
    }

    fn create_disk_image(&self, rootfs: &Path, config: &VmImagePipelineConfig) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let creator = DiskImageCreator::new(&self.output_dir);
        let output = self.output_dir.join(format!("disk.{}", match config.format {
            DiskFormat::Raw => "raw",
            DiskFormat::Qcow2 => "qcow2",
            DiskFormat::WithKernel => "raw",
        }));

        creator.create_from_dir(rootfs, &output, config.disk_size_mb)?;

        Ok(output)
    }
}

pub struct VmImageOutput {
    pub rootfs: PathBuf,
    pub kernel: PathBuf,
    pub initramfs: PathBuf,
    pub disk_image: PathBuf,
}
```

## Advanced Topics

### SEV/TDX Confidential Kernels

```rust
// For AMD SEV:
let sev_config = LibkrunfwConfig {
    sev_enabled: true,
    extra_config: vec![
        "CONFIG_AMD_MEM_ENCRYPT=y".to_string(),
        "CONFIG_AMD_MEM_ENCRYPT_ACTIVE_BY_DEFAULT=y".to_string(),
        "CONFIG_IOMMU_DEFAULT_PASSTHROUGH=y".to_string(),
    ],
    ..Default::default()
};

// For Intel TDX:
let tdx_config = LibkrunfwConfig {
    tdx_enabled: true,
    extra_config: vec![
        "CONFIG_INTEL_TDX_GUEST=y".to_string(),
        "CONFIG_X86_TDX=y".to_string(),
    ],
    ..Default::default()
};
```

### AArch64 Kernel Build

```rust
let aarch64_config = LibkrunfwConfig {
    arch: "aarch64".to_string(),
    cross_compile: Some("aarch64-linux-gnu-".to_string()),
    extra_config: vec![
        "CONFIG_ARM64=y".to_string(),
        "CONFIG_ARCH_VIRT=y".to_string(),
        "CONFIG_VIRTIO_MMIO=y".to_string(),
    ],
    ..Default::default()
};
```

### Custom Kernel Patches

```rust
let config = LibkrunfwConfig {
    patches: vec![
        PathBuf::from("/path/to/libkrun-patches/tsi-support.patch"),
        PathBuf::from("/path/to/custom/virtio-optimizations.patch"),
    ],
    ..Default::default()
};
```

## References

- [libkrunfw Repository](https://github.com/containers/libkrunfw)
- [libkrun API](../../src.containers/libkrun/include/libkrun.h)
- [OCI Image Specification](https://github.com/opencontainers/image-spec)
- [Linux Kernel Documentation](https://www.kernel.org/doc/html/latest/)
