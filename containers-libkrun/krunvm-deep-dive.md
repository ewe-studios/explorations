---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.containers/krunvm
explored_at: 2026-03-19
---

# krunvm Deep Dive

## Purpose

krunvm is a CLI-based utility for creating microVMs from OCI images, using libkrun and buildah. It provides a container-like experience for launching VMs.

## Features

- **Minimal footprint**: Only essential resources allocated
- **Fast boot time**: VMs start in sub-second time
- **Zero disk image maintenance**: No qcow2/raw image management needed
- **Zero network configuration**: Automatic networking setup
- **Volume mapping**: Map host directories into guest
- **Port exposure**: Expose guest ports to host

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         krunvm CLI                          │
│                    (Rust + libkrun-sys)                     │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
       ┌────────────┐  ┌────────────┐  ┌────────────┐
       │  libkrun   │  │  buildah   │  │   podman   │
       │   (VMM)    │  │  (Images)  │  │  (Runtime) │
       └────────────┘  └────────────┘  └────────────┘
              │               │
              │               ▼
              │        ┌────────────┐
              │        │   OCI      │
              │        │  Registry  │
              │        └────────────┘
              ▼
       ┌────────────┐
       │  Guest VM  │
       │  (microVM) │
       └────────────┘
```

## Supported Platforms

| Platform | Architecture | Hypervisor |
|----------|-------------|------------|
| Linux | x86_64 | KVM |
| Linux | AArch64 | KVM |
| macOS | ARM64 | Hypervisor.framework |

## Installation

### macOS (Homebrew)

```bash
brew tap slp/krun
brew install krunvm
```

### Fedora (COPR)

```bash
dnf copr enable -y slp/libkrunfw
dnf copr enable -y slp/libkrun
dnf copr enable -y slp/krunvm
dnf install -y krunvm
```

### Building from Source

```bash
# Dependencies
# - Rust toolchain
# - libkrun (installed)
# - buildah
# - asciidoctor (for docs)

cargo build --release
```

## Usage

### Basic VM Launch

```bash
# Run VM from OCI image
krunvm run ubuntu:22.04

# With resource limits
krunvm run ubuntu:22.04 --cpus 4 --memory 4096

# With custom command
krunvm run ubuntu:22.04 -- /bin/bash -c "echo hello"
```

### Volume Mapping

```bash
# Map host directory to guest
krunvm run ubuntu:22.04 -v /host/path:/guest/path

# Read-only mapping
krunvm run ubuntu:22.04 -v /host/path:/guest/path:ro
```

### Port Exposure

```bash
# Expose guest port to host
krunvm run ubuntu:22.04 -p 8080:80

# Multiple ports
krunvm run ubuntu:22.04 -p 8080:80 -p 443:443
```

### Networking

krunvm supports multiple networking modes:

```bash
# Using passt (default on Linux)
krunvm run ubuntu:22.04 --net=passt

# Using TSI (Transparent Socket Impersonation)
krunvm run ubuntu:22.04 --net=tsi

# Using tap device
krunvm run ubuntu:22.04 --net=tap0
```

## OCI Image Handling

### Image Sources

krunvm uses buildah/podman for image management:

```bash
# Pull image
buildah pull ubuntu:22.04

# Build custom image
buildah build -t my-vm-image .

# List images
buildah images
```

### Image Requirements

For an OCI image to work as a VM rootfs:

1. **Init system**: Must have `/sbin/init` or similar
2. **Kernel modules**: May need specific modules for virtio devices
3. **Filesystem**: Must be compatible with Linux kernel in libkrunfw

### Building VM-Ready Images

```dockerfile
# Dockerfile for VM image
FROM ubuntu:22.04

# Install init system
RUN apt-get update && apt-get install -y \
    systemd \
    openssh-server \
    sudo

# Configure systemd
ENV container=podman

# Expose SSH
EXPOSE 22

# Default command
CMD ["/sbin/init"]
```

## Internal Structure

### Workspace

```
krunvm/
├── Cargo.toml
├── src/
│   ├── main.rs           # CLI entry point
│   ├── vm/
│   │   ├── config.rs     # VM configuration
│   │   ├── create.rs     # VM creation logic
│   │   └── run.rs        # VM execution
│   ├── image/
│   │   ├── oci.rs        # OCI image handling
│   │   └── storage.rs    # Image storage management
│   └── network/
│       ├── passt.rs      # passt integration
│       └── tsi.rs        # TSI networking
├── docs/
│   └── usage.md
└── Makefile
```

### Key Data Structures

```rust
// VM Configuration
struct VmConfig {
    cpus: u8,
    memory: u32,  // MiB
    image: String,
    volumes: Vec<VolumeMap>,
    ports: Vec<PortMap>,
    network: NetworkMode,
    kernel: Option<String>,
}

// Volume Mapping
struct VolumeMap {
    host_path: PathBuf,
    guest_path: PathBuf,
    read_only: bool,
}

// Port Mapping
struct PortMap {
    host_port: u16,
    guest_port: u16,
}
```

## Comparison with Alternatives

| Feature | krunvm | Ignite | Docker |
|---------|--------|--------|--------|
| VM Isolation | Yes | Yes | No |
| Boot Time | ~125ms | ~1s | ~50ms |
| OCI Native | Yes | Yes | Yes |
| GPU Support | Via krunkit | No | Limited |
| macOS Support | Yes | No | Yes |
| SEV/TDX | Via libkrun | No | No |

## Use Cases

### Development Environments

```bash
# Create isolated dev environment
krunvm run my-dev-image -v ~/code:/code -p 3000:3000
```

### CI/CD Runners

```bash
# Ephemeral build environment
krunvm run ci-image --rm -- /build.sh
```

### Testing

```bash
# Test in isolated VM
krunvm run test-image -- /run-tests.sh
```

### Container-to-VM Migration

```bash
# Run existing container as VM
krunvm run my-container-image
```

## Security Considerations

### Isolation Level

krunvm provides VM-level isolation via:
- KVM hardware virtualization
- Separate kernel instance
- virtio device boundaries

### Limitations

1. **virtio-fs**: Guest can attempt to access host filesystem paths
   - Mitigation: Use mount namespaces

2. **Networking**: TSI mode proxies all network traffic
   - Mitigation: Apply network policies to krunvm process

3. **Shared Volumes**: Host directories are accessible
   - Mitigation: Use read-only mounts where possible

## References

- [krunvm README](../../src.containers/krunvm/README.md)
- [libkrun README](../../src.containers/libkrun/README.md)
- [buildah Documentation](https://github.com/containers/buildah)
