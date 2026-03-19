---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/firecracker-deep-dive
repository: https://github.com/firecracker-microvm/firecracker
created_at: 2026-03-19
language: Rust
---

# Building and Running Firecracker MicroVMs with Rust

A comprehensive guide to building Firecracker, creating kernel images and root filesystems, and controlling microVMs programmatically using Rust.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Building Firecracker from Source](#building-firecracker-from-source)
3. [Building Guest Kernel Images](#building-guest-kernel-images)
4. [Creating Root Filesystems](#creating-root-filesystems)
5. [Rust SDK for Firecracker Control](#rust-sdk-for-firecracker-control)
6. [Complete Working Examples](#complete-working-examples)
7. [Production Deployment with Jailer](#production-deployment-with-jailer)

## Prerequisites

### System Requirements

- Linux host with KVM support (`/dev/kvm` must exist)
- Rust 1.70+ (check with `rustc --version`)
- Root or KVM group membership for `/dev/kvm` access
- Sufficient disk space (~5GB for full build + artifacts)

### Verify KVM Access

```bash
# Check if KVM module is loaded
ls -la /dev/kvm

# Add user to kvm group if needed
sudo usermod -aG kvm $USER

# Verify group membership
groups $USER
```

### Install Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
```

## Building Firecracker from Source

### Clone the Repository

```bash
git clone https://github.com/firecracker-microvm/firecracker.git
cd firecracker
```

### Build Using devtool (Recommended)

Firecracker provides a `devtool` script that automates the build process:

```bash
# Build the release version
./tools/devtool build --release

# Build with all features
./tools/devtool build --release --all-features

# Output will be in:
# - build/cargo_target/release/firecracker
# - build/cargo_target/release/jailer
```

### Manual Build with Cargo

```bash
# Navigate to firecracker directory
cd firecracker

# Build release version
cargo build --release

# Binaries will be in:
# - target/release/firecracker
# - target/release/jailer
```

### Install Binaries

```bash
# Copy binaries to system path
sudo cp build/cargo_target/release/firecracker /usr/local/bin/
sudo cp build/cargo_target/release/jailer /usr/local/bin/

# Or use cargo install (alternative)
cargo install --path firecracker
cargo install --path jailer
```

### Verify Installation

```bash
firecracker --version
jailer --version
```

## Building Guest Kernel Images

### Download Kernel Source

```bash
git clone https://github.com/torvalds/linux.git linux.git
cd linux.git
git checkout v6.1  # Use a supported LTS version
```

### Configure Kernel for Firecracker

#### Option 1: Use Firecracker's CI Config

```bash
# Copy the recommended config from Firecracker repo
cp /path/to/firecracker/resources/guest_configs/microvm-kernel-ci-x86_64-6.1.config .config

# Or for aarch64
cp /path/to/firecracker/resources/guest_configs/microvm-kernel-ci-arm64-6.1.config .config
```

#### Option 2: Manual Configuration

```bash
make menuconfig
```

**Essential kernel config options:**

```bash
# Minimal boot support
CONFIG_BLK_DEV_INITRD=y
CONFIG_INITRAMFS_SOURCE=""
CONFIG_RD_LZMA=y

# For x86_64 guests
CONFIG_KVM_GUEST=y
CONFIG_PARAVIRT=y
CONFIG_SERIAL_8250_CONSOLE=y
CONFIG_SERIAL_8250=y
CONFIG_SERIAL_8250_NR_UARTS=1
CONFIG_SERIAL_8250_RUNTIME_UARTS=1
CONFIG_SERIAL_8250_EXTENDED=y
CONFIG_SERIAL_8250_SHARE_IRQ=y
CONFIG_SERIAL_8250_DETECT_IF=y
CONFIG_SERIAL_8250_RSA=y
CONFIG_SERIAL_8250_DW=y
CONFIG_SERIAL_8250_RT288X=y

# For aarch64 guests
CONFIG_VIRTIO_MMIO=y
CONFIG_SERIAL_AMBA_PL011_CONSOLE=y
CONFIG_SERIAL_AMBA_PL011=y

# Root filesystem support
CONFIG_BLK_DEV=y
CONFIG_BLK_DEV_SD=y
CONFIG_ATA=y
CONFIG_SATA_AHCI=y
CONFIG_PATA_OLDPIIX=y
CONFIG_PATA_AMD=y

# VirtIO devices (essential for Firecracker)
CONFIG_VIRTIO=y
CONFIG_VIRTIO_PCI=y
CONFIG_VIRTIO_MMIO=y
CONFIG_VIRTIO_BALLOON=y
CONFIG_VIRTIO_NET=y
CONFIG_VIRTIO_BLK=y
CONFIG_VIRTIO_CONSOLE=y
CONFIG_VIRTIO_VSOCKETS=y
CONFIG_HW_RANDOM_VIRTIO=y

# Network support
CONFIG_NET=y
CONFIG_PACKET=y
CONFIG_UNIX=y
CONFIG_INET=y
CONFIG_IP_MULTICAST=y
CONFIG_IP_ADVANCED_ROUTER=y
CONFIG_IP_MULTIPLE_TABLES=y
CONFIG_INET_ESP=y
CONFIG_INET_XFRM_MODE_TRANSPORT=y
CONFIG_INET_XFRM_MODE_TUNNEL=y
CONFIG_INET_XFRM_MODE_BEET=y
CONFIG_INET_DIAG=y
CONFIG_INET_TCP_DIAG=y
CONFIG_IPV6=y

# Filesystem support
CONFIG_EXT4_FS=y
CONFIG_EXT4_FS_POSIX_ACL=y
CONFIG_EXT4_FS_SECURITY=y
CONFIG_BTRFS_FS=y
CONFIG_BTRFS_FS_POSIX_ACL=y
CONFIG_FUSE_FS=y
CONFIG_OVERLAY_FS=y

# Cgroups and namespaces for containers
CONFIG_CGROUPS=y
CONFIG_CGROUP_FREEZER=y
CONFIG_CGROUP_DEVICE=y
CONFIG_CGROUP_CPUACCT=y
CONFIG_CGROUP_PERF=y
CONFIG_NAMESPACES=y
CONFIG_UTS_NS=y
CONFIG_IPC_NS=y
CONFIG_USER_NS=y
CONFIG_PID_NS=y
CONFIG_NET_NS=y

# Kernel command line
CONFIG_CMDLINE="console=ttyS0 reboot=k panic=1 pci=off"
CONFIG_CMDLINE_BOOL=y
```

### Build the Kernel

```bash
# For x86_64
make -j$(nproc) vmlinux

# For aarch64
make -j$(nproc) Image

# The kernel binary will be:
# - vmlinux (x86_64) - uncompressed ELF kernel
# - Image (aarch64)
```

### Optional: Build Initramfs

```bash
# Create a simple initramfs
mkdir -p initramfs/{bin,sbin,etc,proc,sys,dev,root}
cd initramfs

# Create basic init script
cat > init << 'EOF'
#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
exec /sbin/init
EOF
chmod +x init

# Create the cpio archive
find . | cpio -o -H newc | gzip > ../initramfs.cpio.gz
```

## Creating Root Filesystems

### Method 1: Using Docker (Recommended)

```bash
# Create empty ext4 filesystem image
dd if=/dev/zero of=rootfs.ext4 bs=1M count=500
mkfs.ext4 rootfs.ext4

# Mount the filesystem
mkdir -p /mnt/rootfs
sudo mount -o loop rootfs.ext4 /mnt/rootfs
```

#### Alpine Linux Base (Minimal)

```bash
# Use Docker to populate the filesystem
docker run --rm -it -v /mnt/rootfs:/rootfs alpine:latest /bin/sh

# Inside the container, install base packages
apk add --no-cache \
    busybox \
    openrc \
    util-linux \
    e2fsprogs \
    eudev \
    alpine-baselayout

# Configure system
echo "ttyS0" > /rootfs/etc/securetty
ln -s agetty /rootfs/etc/init.d/agetty.ttyS0
rc-update add agetty.ttyS0 default
rc-update add devfs boot
rc-update add procfs boot
rc-update add sysfs boot
rc-update add networking boot

# Create basic fstab
cat > /rootfs/etc/fstab << 'EOF'
/dev/vda1 / ext4 defaults 0 0
none /proc proc defaults 0 0
none /sys sysfs defaults 0 0
EOF

# Exit Docker
exit
```

#### Debian/Ubuntu Base

```bash
# Use debootstrap (Debian/Ubuntu only)
sudo debootstrap stable /mnt/rootfs http://deb.debian.org/debian

# Or with docker
docker run --rm -it -v /mnt/rootfs:/rootfs debian:stable /bin/bash

# Inside container
apt-get update
apt-get install -y systemd
apt-get clean

# Create init script for systemd
mkdir -p /rootfs/etc/systemd/system/serial-getty@ttyS0.service.d/
cat > /rootfs/etc/systemd/system/serial-getty@ttyS0.service.d/override.conf << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty -o '-p -- \\u' --keep-baud 115200 %I $TERM
EOF

exit
```

### Method 2: Using firecracker-containerd Image Builder

```bash
cd /path/to/firecracker-containerd/tools/image-builder

# Build default rootfs (Debian-based)
make image

# Copy to standard location
sudo cp rootfs.img /var/lib/firecracker-containerd/runtime/default-rootfs.img
```

### Method 3: Using Nix (Reproducible Builds)

```bash
# Create a flake.nix for reproducible rootfs
cat > flake.nix << 'EOF'
{
  description = "Firecracker rootfs";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";

  outputs = { self, nixpkgs }: {
    packages.x86_64-linux.rootfs =
      let
        pkgs = import nixpkgs { system = "x86_64-linux"; };
      in
      pkgs.buildFHSEnv {
        name = "firecracker-rootfs";
        targetPkgs = pkgs: with pkgs; [
          busybox
          systemd
        ];
        runScript = "sh";
      };
  };
}
EOF

nix build .#rootfs
```

### Unmount and Verify

```bash
# Unmount the filesystem
sudo umount /mnt/rootfs

# Verify filesystem
e2fsck -f rootfs.ext4

# Optionally resize to save space
resize2fs -M rootfs.ext4
```

## Rust SDK for Firecracker Control

Firecracker exposes a RESTful API over a Unix domain socket. Here's how to interact with it using Rust.

### Project Setup

Create a new Rust project:

```bash
cargo new firecracker-vm
cd firecracker-vm
```

Add dependencies to `Cargo.toml`:

```toml
[package]
name = "firecracker-vm"
version = "0.1.0"
edition = "2021"

[dependencies]
# HTTP client for Unix domain socket
reqwest = { version = "0.11", features = ["json"] }
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Unix socket support
tokio-unix = "0.1"

# Error handling
anyhow = "1"
thiserror = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
```

### API Models

Create `src/models.rs` with Firecracker API types:

```rust
use serde::{Deserialize, Serialize};

/// Machine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfiguration {
    pub vcpu_count: u8,
    pub mem_size_mib: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_template: Option<CpuTemplate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_dirty_pages: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CpuTemplate {
    T2,
    T2S,
    T2CL,
    T2A,
    C3,
    #[serde(rename = "None")]
    NoTemplate,
}

/// Boot source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootSource {
    pub kernel_image_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_args: Option<String>,
}

/// Block device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drive {
    pub drive_id: String,
    pub path_on_host: String,
    pub is_root_device: bool,
    pub is_read_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partuuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limiter: Option<RateLimiter>,
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub iface_id: String,
    pub host_dev_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guest_mac: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_rate_limiter: Option<RateLimiter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_rate_limiter: Option<RateLimiter>,
}

/// Rate limiter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bandwidth: Option<TokenBucket>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ops: Option<TokenBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBucket {
    pub size: u64,
    pub one_time_burst: u64,
    pub refill_time: u64,
}

/// Vsock configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vsock {
    pub guest_cid: u64,
    pub uds_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vsock_id: Option<String>,
}

/// VM State
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum VMState {
    Halted,
    Paused,
    Resumed,
}

/// VM configuration for state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VM {
    pub state: VMState,
}

/// Instance action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceActionInfo {
    #[serde(rename = "action_type")]
    pub action_type: String,
}

impl InstanceActionInfo {
    pub fn start() -> Self {
        Self {
            action_type: "InstanceStart".to_string(),
        }
    }

    pub fn send_ctrl_alt_del() -> Self {
        Self {
            action_type: "SendCtrlAltDel".to_string(),
        }
    }
}

/// Snapshot create parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotCreateParams {
    pub mem_file_path: String,
    pub snapshot_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Snapshot load parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotLoadParams {
    pub mem_backend: MemoryBackend,
    pub snapshot_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_diff_snapshots: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_vm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBackend {
    pub backend_path: String,
    pub backend_type: String,
}

/// Logger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Logger {
    pub log_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    pub show_level: bool,
    pub show_log_origin: bool,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub metrics_path: String,
}
```

### Firecracker Client

Create `src/client.rs`:

```rust
use crate::models::*;
use anyhow::{Context, Result};
use reqwest::{Client, Response};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;

/// Firecracker API client
pub struct FirecrackerClient {
    client: Client,
    socket_path: String,
    base_url: String,
}

impl FirecrackerClient {
    /// Create a new Firecracker client
    pub fn new(socket_path: impl AsRef<Path>) -> Result<Self> {
        let socket_path = socket_path.as_ref().to_string_lossy().to_string();

        // Create Unix socket connector
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            socket_path: socket_path.clone(),
            base_url: "http://localhost".to_string(),
        })
    }

    /// Wait for the Firecracker socket to become available
    pub async fn wait_for_socket(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if Path::new(&self.socket_path).exists() {
                // Try a test request
                if self.get_machine_config().await.is_ok() {
                    return Ok(());
                }
            }
            sleep(Duration::from_millis(100)).await;
        }

        anyhow::bail!("Firecracker socket did not become available within {:?}", timeout)
    }

    /// PUT request helper
    async fn put<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<Response> {
        let url = format!("{}{}", self.base_url, endpoint);

        let response = self.client
            .put(&url)
            .json(body)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send()
            .await
            .context(format!("PUT {} failed", endpoint))?;

        Ok(response)
    }

    /// PATCH request helper
    async fn patch<T: Serialize>(&self, endpoint: &str, body: &T) -> Result<Response> {
        let url = format!("{}{}", self.base_url, endpoint);

        let response = self.client
            .patch(&url)
            .json(body)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .send()
            .await
            .context(format!("PATCH {} failed", endpoint))?;

        Ok(response)
    }

    /// GET request helper
    async fn get(&self, endpoint: &str) -> Result<Response> {
        let url = format!("{}{}", self.base_url, endpoint);

        let response = self.client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .context(format!("GET {} failed", endpoint))?;

        Ok(response)
    }

    // ==================== Machine Configuration ====================

    /// Configure the machine (vCPUs, memory)
    pub async fn configure_machine(&self, config: &MachineConfiguration) -> Result<()> {
        let response = self.put("/machine-config", config).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await?;
            anyhow::bail!("Configure machine failed ({}): {}", status, body)
        }
    }

    /// Get machine configuration
    pub async fn get_machine_config(&self) -> Result<MachineConfiguration> {
        let response = self.get("/machine-config").await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            anyhow::bail!("Get machine config failed: {}", response.status())
        }
    }

    // ==================== Boot Source ====================

    /// Configure boot source (kernel)
    pub async fn configure_boot_source(&self, boot_source: &BootSource) -> Result<()> {
        let response = self.put("/boot-source", boot_source).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Configure boot source failed: {}", body)
        }
    }

    // ==================== Block Devices ====================

    /// Attach a block device
    pub async fn attach_drive(&self, drive: &Drive) -> Result<()> {
        let endpoint = format!("/drives/{}", drive.drive_id);
        let response = self.put(&endpoint, drive).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Attach drive {} failed: {}", drive.drive_id, body)
        }
    }

    /// Update a block device (e.g., for hot-plugging)
    pub async fn update_drive(&self, drive_id: &str, path_on_host: &str, is_read_only: Option<bool>) -> Result<()> {
        #[derive(Serialize)]
        struct DriveUpdate {
            path_on_host: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_read_only: Option<bool>,
        }

        let update = DriveUpdate {
            path_on_host: path_on_host.to_string(),
            is_read_only,
        };

        let endpoint = format!("/drives/{}", drive_id);
        let response = self.patch(&endpoint, &update).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Update drive {} failed: {}", drive_id, body)
        }
    }

    // ==================== Network Interfaces ====================

    /// Attach a network interface
    pub async fn attach_network_interface(&self, iface: &NetworkInterface) -> Result<()> {
        let endpoint = format!("/network-interfaces/{}", iface.iface_id);
        let response = self.put(&endpoint, iface).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Attach network interface {} failed: {}", iface.iface_id, body)
        }
    }

    /// Update network interface rate limits
    pub async fn update_network_interface(&self, iface_id: &str, rx_limiter: Option<RateLimiter>, tx_limiter: Option<RateLimiter>) -> Result<()> {
        #[derive(Serialize)]
        struct NetworkUpdate {
            iface_id: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            rx_rate_limiter: Option<RateLimiter>,
            #[serde(skip_serializing_if = "Option::is_none")]
            tx_rate_limiter: Option<RateLimiter>,
        }

        let update = NetworkUpdate {
            iface_id: iface_id.to_string(),
            rx_rate_limiter: rx_limiter,
            tx_rate_limiter: tx_limiter,
        };

        let endpoint = format!("/network-interfaces/{}", iface_id);
        let response = self.patch(&endpoint, &update).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Update network interface {} failed: {}", iface_id, body)
        }
    }

    // ==================== Vsock ====================

    /// Configure vsock device
    pub async fn configure_vsock(&self, vsock: &Vsock) -> Result<()> {
        let response = self.put("/vsock", vsock).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Configure vsock failed: {}", body)
        }
    }

    // ==================== VM Lifecycle ====================

    /// Start the VM
    pub async fn start_vm(&self) -> Result<()> {
        #[derive(Serialize)]
        struct Action {
            action_type: String,
        }

        let action = Action {
            action_type: "InstanceStart".to_string(),
        };

        let response = self.put("/actions", &action).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Start VM failed: {}", body)
        }
    }

    /// Pause the VM
    pub async fn pause_vm(&self) -> Result<()> {
        let vm_state = VM { state: VMState::Paused };
        let response = self.patch("/vm", &vm_state).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Pause VM failed: {}", body)
        }
    }

    /// Resume the VM
    pub async fn resume_vm(&self) -> Result<()> {
        let vm_state = VM { state: VMState::Resumed };
        let response = self.patch("/vm", &vm_state).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Resume VM failed: {}", body)
        }
    }

    /// Stop the VM (graceful shutdown via Ctrl+Alt+Del)
    pub async fn stop_vm(&self) -> Result<()> {
        #[derive(Serialize)]
        struct Action {
            action_type: String,
        }

        let action = Action {
            action_type: "SendCtrlAltDel".to_string(),
        };

        let response = self.put("/actions", &action).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Stop VM failed: {}", body)
        }
    }

    // ==================== Snapshots ====================

    /// Create a VM snapshot
    pub async fn create_snapshot(&self, params: &SnapshotCreateParams) -> Result<()> {
        let response = self.put("/snapshot/create", params).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Create snapshot failed: {}", body)
        }
    }

    /// Load a VM snapshot
    pub async fn load_snapshot(&self, params: &SnapshotLoadParams) -> Result<()> {
        let response = self.put("/snapshot/load", params).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Load snapshot failed: {}", body)
        }
    }

    // ==================== Logging & Metrics ====================

    /// Configure logging
    pub async fn configure_logger(&self, logger: &Logger) -> Result<()> {
        let response = self.put("/logger", logger).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Configure logger failed: {}", body)
        }
    }

    /// Configure metrics
    pub async fn configure_metrics(&self, metrics: &Metrics) -> Result<()> {
        let response = self.put("/metrics", metrics).await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body = response.text().await?;
            anyhow::bail!("Configure metrics failed: {}", body)
        }
    }
}
```

### VM Manager

Create `src/vm.rs` for high-level VM management:

```rust
use crate::client::FirecrackerClient;
use crate::models::*;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// High-level VM manager
pub struct VMManager {
    client: FirecrackerClient,
    firecracker_path: PathBuf,
    socket_path: PathBuf,
    firecracker_process: Option<Child>,
}

impl VMManager {
    /// Create a new VM manager
    pub fn new(socket_path: impl AsRef<Path>, firecracker_path: impl AsRef<Path>) -> Result<Self> {
        let socket_path = socket_path.as_ref().to_path_buf();
        let firecracker_path = firecracker_path.as_ref().to_path_buf();

        // Remove existing socket if present
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)
                .context("Failed to remove existing socket")?;
        }

        let client = FirecrackerClient::new(&socket_path)?;

        Ok(Self {
            client,
            firecracker_path,
            socket_path,
            firecracker_process: None,
        })
    }

    /// Start the Firecracker process
    pub fn start_firecracker(&mut self) -> Result<()> {
        info!("Starting Firecracker process");

        let cmd = Command::new(&self.firecracker_path)
            .arg("--api-sock")
            .arg(&self.socket_path)
            .spawn()
            .context("Failed to start Firecracker")?;

        self.firecracker_process = Some(cmd);

        // Wait for socket to become available
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.client.wait_for_socket(Duration::from_secs(10)).await
            })
        })?;

        info!("Firecracker started successfully");
        Ok(())
    }

    /// Stop the Firecracker process
    pub fn stop_firecracker(&mut self) -> Result<()> {
        if let Some(mut process) = self.firecracker_process.take() {
            info!("Stopping Firecracker process");
            process.kill().context("Failed to kill Firecracker process")?;
            process.wait().context("Failed to wait for Firecracker process")?;
        }

        // Clean up socket
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .context("Failed to remove socket")?;
        }

        Ok(())
    }

    /// Configure the VM
    pub async fn configure(
        &self,
        vcpu_count: u8,
        memory_mib: u64,
        kernel_path: impl AsRef<Path>,
        rootfs_path: impl AsRef<Path>,
    ) -> Result<()> {
        info!("Configuring VM with {} vCPUs and {}MB memory", vcpu_count, memory_mib);

        // Machine configuration
        let machine_config = MachineConfiguration {
            vcpu_count,
            mem_size_mib: memory_mib,
            smt: None,
            cpu_template: None,
            track_dirty_pages: Some(true), // Enable for snapshot support
        };
        self.client.configure_machine(&machine_config).await?;

        // Boot source
        let boot_source = BootSource {
            kernel_image_path: kernel_path.as_ref().to_string_lossy().to_string(),
            initrd_path: None,
            boot_args: Some("console=ttyS0 reboot=k panic=1 pci=off".to_string()),
        };
        self.client.configure_boot_source(&boot_source).await?;

        // Root filesystem
        let drive = Drive {
            drive_id: "rootfs".to_string(),
            path_on_host: rootfs_path.as_ref().to_string_lossy().to_string(),
            is_root_device: true,
            is_read_only: false,
            cache_type: Some("Unsafe".to_string()),
            io_engine: None,
            partuuid: None,
            rate_limiter: None,
        };
        self.client.attach_drive(&drive).await?;

        Ok(())
    }

    /// Configure networking
    pub async fn configure_network(&self, tap_device: &str, iface_id: Option<&str>) -> Result<()> {
        let iface_id = iface_id.unwrap_or("eth0").to_string();
        info!("Configuring network interface {} on TAP {}", iface_id, tap_device);

        let iface = NetworkInterface {
            iface_id,
            host_dev_name: tap_device.to_string(),
            guest_mac: None,
            rx_rate_limiter: None,
            tx_rate_limiter: None,
        };
        self.client.attach_network_interface(&iface).await?;

        Ok(())
    }

    /// Start the VM
    pub async fn start(&self) -> Result<()> {
        info!("Starting VM");
        self.client.start_vm().await?;
        Ok(())
    }

    /// Pause the VM
    pub async fn pause(&self) -> Result<()> {
        info!("Pausing VM");
        self.client.pause_vm().await?;
        Ok(())
    }

    /// Resume the VM
    pub async fn resume(&self) -> Result<()> {
        info!("Resuming VM");
        self.client.resume_vm().await?;
        Ok(())
    }

    /// Stop the VM
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping VM");
        self.client.stop_vm().await?;
        Ok(())
    }

    /// Create a snapshot
    pub async fn create_snapshot(
        &self,
        mem_file_path: impl AsRef<Path>,
        snapshot_path: impl AsRef<Path>,
        snapshot_type: Option<&str>,
    ) -> Result<()> {
        info!("Creating snapshot");

        // Pause VM first
        self.pause().await?;

        let params = SnapshotCreateParams {
            mem_file_path: mem_file_path.as_ref().to_string_lossy().to_string(),
            snapshot_path: snapshot_path.as_ref().to_string_lossy().to_string(),
            snapshot_type: snapshot_type.map(String::from),
            version: None,
        };

        self.client.create_snapshot(&params).await?;

        // Resume VM
        self.resume().await?;

        Ok(())
    }

    /// Load a snapshot
    pub async fn load_snapshot(
        &self,
        mem_file_path: impl AsRef<Path>,
        snapshot_path: impl AsRef<Path>,
        enable_diff: bool,
    ) -> Result<()> {
        info!("Loading snapshot");

        let params = SnapshotLoadParams {
            mem_backend: MemoryBackend {
                backend_path: mem_file_path.as_ref().to_string_lossy().to_string(),
                backend_type: "File".to_string(),
            },
            snapshot_path: snapshot_path.as_ref().to_string_lossy().to_string(),
            enable_diff_snapshots: Some(enable_diff),
            resume_vm: Some(true),
        };

        self.client.load_snapshot(&params).await?;

        Ok(())
    }
}

impl Drop for VMManager {
    fn drop(&mut self) {
        let _ = self.stop_firecracker();
    }
}
```

### Main Entry Point

Create `src/main.rs`:

```rust
mod client;
mod models;
mod vm;

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use vm::VMManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .init();

    info!("Firecracker Rust SDK Demo");

    // Paths
    let socket_path = PathBuf::from("/tmp/firecracker.socket");
    let firecracker_path = PathBuf::from("/usr/local/bin/firecracker");
    let kernel_path = PathBuf::from("/path/to/vmlinux");
    let rootfs_path = PathBuf::from("/path/to/rootfs.ext4");

    // Create VM manager
    let mut vm_manager = VMManager::new(&socket_path, &firecracker_path)?;

    // Start Firecracker
    vm_manager.start_firecracker()?;

    // Configure VM
    vm_manager
        .configure(2, 512, &kernel_path, &rootfs_path)
        .await?;

    // Start VM
    vm_manager.start().await?;

    info!("VM started successfully");

    // Example: Pause and resume
    // tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    // vm_manager.pause().await?;
    // tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    // vm_manager.resume().await?;

    // Example: Create snapshot
    // vm_manager
    //     .create_snapshot("/tmp/mem.bin", "/tmp/snapshot.bin", Some("Full"))
    //     .await?;

    // Wait for user input or handle cleanup
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");

    // Stop VM gracefully
    vm_manager.stop().await?;

    Ok(())
}
```

## Complete Working Examples

### Example 1: Minimal VM Setup

This example creates a minimal VM with kernel and rootfs:

```rust
use firecracker_sdk::{FirecrackerClient, models::*};
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = FirecrackerClient::new("/tmp/firecracker.socket")?;

    // Wait for Firecracker to be ready
    client.wait_for_socket(std::time::Duration::from_secs(5)).await?;

    // 1. Configure machine
    client.configure_machine(&MachineConfiguration {
        vcpu_count: 2,
        mem_size_mib: 256,
        smt: None,
        cpu_template: None,
        track_dirty_pages: None,
    }).await?;

    // 2. Set boot source
    client.configure_boot_source(&BootSource {
        kernel_image_path: "/images/vmlinux".to_string(),
        initrd_path: None,
        boot_args: Some("console=ttyS0 reboot=k panic=1".to_string()),
    }).await?;

    // 3. Attach rootfs
    client.attach_drive(&Drive {
        drive_id: "root".to_string(),
        path_on_host: "/images/rootfs.ext4".to_string(),
        is_root_device: true,
        is_read_only: false,
        cache_type: None,
        io_engine: None,
        partuuid: None,
        rate_limiter: None,
    }).await?;

    // 4. Start VM
    client.start_vm().await?;

    println!("VM started!");

    // Keep running
    tokio::signal::ctrl_c().await?;

    Ok(())
}
```

### Example 2: VM with Networking

```rust
use firecracker_sdk::{FirecrackerClient, models::*};

async fn setup_networking(client: &FirecrackerClient, tap_name: &str) -> anyhow::Result<()> {
    // Create TAP device (requires root or CAP_NET_ADMIN)
    std::process::Command::new("ip")
        .args(["tuntap", "add", "dev", tap_name, "mode", "tap"])
        .status()?;

    std::process::Command::new("ip")
        .args(["link", "set", tap_name, "up"])
        .status()?;

    // Attach to Firecracker
    client.attach_network_interface(&NetworkInterface {
        iface_id: "eth0".to_string(),
        host_dev_name: tap_name.to_string(),
        guest_mac: Some("06:00:AC:10:00:01".to_string()),
        rx_rate_limiter: None,
        tx_rate_limiter: None,
    }).await?;

    Ok(())
}
```

### Example 3: VM with Vsock for Host-Guest Communication

```rust
async fn setup_vsock(client: &FirecrackerClient) -> anyhow::Result<()> {
    let vsock_path = "/tmp/vsock.sock";

    client.configure_vsock(&Vsock {
        guest_cid: 3,
        uds_path: vsock_path.to_string(),
        vsock_id: None,
    }).await?;

    // On host: connect to vsock_path and send "CONNECT 52\n"
    // to connect to guest port 52

    Ok(())
}
```

### Example 4: Snapshot and Restore

```rust
async fn snapshot_demo(client: &FirecrackerClient) -> anyhow::Result<()> {
    // Create full snapshot
    client.create_snapshot(&SnapshotCreateParams {
        mem_file_path: "/tmp/vm_memory.bin".to_string(),
        snapshot_path: "/tmp/vm_snapshot.bin".to_string(),
        snapshot_type: Some("Full".to_string()),
        version: None,
    }).await?;

    println!("Snapshot created!");

    // Later, load snapshot in new Firecracker instance
    client.load_snapshot(&SnapshotLoadParams {
        mem_backend: MemoryBackend {
            backend_path: "/tmp/vm_memory.bin".to_string(),
            backend_type: "File".to_string(),
        },
        snapshot_path: "/tmp/vm_snapshot.bin".to_string(),
        enable_diff_snapshots: false,
        resume_vm: Some(true),
    }).await?;

    println!("Snapshot loaded!");

    Ok(())
}
```

## Network Setup Scripts

### Creating TAP Devices

```bash
#!/bin/bash
# setup-tap.sh

TAP_NAME="${1:-tap0}"

# Create TAP device
sudo ip tuntap add dev $TAP_NAME mode tap user $(whoami)

# Bring it up
sudo ip link set $TAP_NAME up

# Optional: Add to bridge
# sudo brctl addif br0 $TAP_NAME

# Enable NAT for internet access
sudo iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE
sudo iptables -A FORWARD -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
sudo iptables -A FORWARD -i $TAP_NAME -j ACCEPT

echo "TAP device $TAP_NAME created"
```

### Network Namespace Isolation

```bash
#!/bin/bash
# setup-netns.sh

NS_NAME="${1:-fc-ns}"
VETH_HOST="veth0"
VETH_GUEST="veth1"

# Create network namespace
sudo ip netns add $NS_NAME

# Create veth pair
sudo ip link add $VETH_HOST type veth peer name $VETH_GUEST

# Move guest end to namespace
sudo ip link set $VETH_GUEST netns $NS_NAME

# Configure host side
sudo ip link set $VETH_HOST up
sudo ip addr add 10.0.0.1/24 dev $VETH_HOST

# Configure guest side
sudo ip netns exec $NS_NAME ip link set $VETH_GUEST up
sudo ip netns exec $NS_NAME ip addr add 10.0.0.2/24 dev $VETH_GUEST
sudo ip netns exec $NS_NAME ip link set lo up
sudo ip netns exec $NS_NAME ip route add default via 10.0.0.1

# Enable forwarding
sudo sysctl -w net.ipv4.ip_forward=1
sudo iptables -t nat -A POSTROUTING -s 10.0.0.0/24 -j MASQUERADE

echo "Network namespace $NS_NAME configured"
```

## Production Deployment with Jailer

### Starting Firecracker via Jailer

```bash
#!/bin/bash
# start-jailed.sh

VM_ID="my-secure-vm"
FIRECRACKER_BIN="/usr/local/bin/firecracker"
JAILER_BIN="/usr/local/bin/jailer"
CHROOT_BASE="/srv/jailer"
UID=10001
GID=10001

# Create required directories
sudo mkdir -p $CHROOT_BASE/firecracker/$VM_ID/root

# Copy kernel and rootfs into jail
sudo cp /images/vmlinux $CHROOT_BASE/firecracker/$VM_ID/root/
sudo cp /images/rootfs.ext4 $CHROOT_BASE/firecracker/$VM_ID/root/

# Start via jailer
sudo $JAILER_BIN \
    --id $VM_ID \
    --exec-file $FIRECRACKER_BIN \
    --uid $UID \
    --gid $GID \
    --chroot-base-dir $CHROOT_BASE \
    --cgroup cpu.shares=100 \
    --cgroup memory.limit_in_bytes=512M \
    --netns /var/run/netns/fc-ns \
    --resource-limit no-file=1024 \
    -- \
    --api-sock /firecracker.socket
```

### Rust Code for Jailer Integration

```rust
use std::process::{Command, Stdio};

fn start_jailed_firecracker(
    vm_id: &str,
    firecracker_path: &str,
    jailer_path: &str,
    chroot_base: &str,
    uid: u32,
    gid: u32,
    cgroups: &[(&str, &str)],
    netns: Option<&str>,
) -> anyhow::Result<()> {
    let mut cmd = Command::new(jailer_path);

    cmd.args([
        "--id", vm_id,
        "--exec-file", firecracker_path,
        "--uid", &uid.to_string(),
        "--gid", &gid.to_string(),
        "--chroot-base-dir", chroot_base,
    ]);

    // Add cgroups
    for (cg_file, cg_value) in cgroups {
        cmd.args(["--cgroup", &format!("{}={}", cg_file, cg_value)]);
    }

    // Add netns
    if let Some(ns) = netns {
        cmd.args(["--netns", ns]);
    }

    // Add resource limits
    cmd.args(["--resource-limit", "no-file=1024"]);
    cmd.args(["--resource-limit", "fsize=104857600"]);

    // Separator for Firecracker args
    cmd.arg("--");

    // Firecracker-specific args
    cmd.args(["--api-sock", "/firecracker.socket"]);

    // Run detached
    cmd.stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}
```

## Troubleshooting

### Common Issues

1. **KVM Permission Denied**
   ```bash
   sudo usermod -aG kvm $USER
   # Then log out and back in
   ```

2. **Socket Not Created**
   - Check Firecracker logs
   - Verify socket path is not too long (Unix socket limit ~108 chars)
   - Ensure parent directory exists and is writable

3. **VM Won't Boot**
   - Verify kernel is uncompressed ELF (use `file vmlinux`)
   - Check boot_args are correct for your kernel
   - Try adding `console=ttyS0` to boot_args for serial output

4. **Network Not Working**
   - Ensure TAP device is up: `ip link show tap0`
   - Check NAT rules: `iptables -t nat -L`
   - Verify guest has correct kernel config for VirtIO net

### Debug Mode

Run Firecracker with logging enabled:

```rust
client.configure_logger(&Logger {
    log_path: "/tmp/fc.log".to_string(),
    level: Some("Debug".to_string()),
    show_level: true,
    show_log_origin: true,
}).await?;
```

## Resources

- [Firecracker Documentation](https://github.com/firecracker-microvm/firecracker/tree/main/docs)
- [Firecracker Go SDK](https://github.com/firecracker-microvm/firecracker-go-sdk)
- [Firecracker Containerd](https://github.com/firecracker-microvm/firecracker-containerd)
- [KVM Documentation](https://www.kernel.org/doc/html/latest/virt/kvm/)
- [VirtIO Specification](https://docs.oasis-open.org/virtio/virtio/v1.1/virtio-v1.1.html)
