---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.containers/krunkit
explored_at: 2026-03-19
---

# krunkit Deep Dive

## Purpose

krunkit is a tool to launch configurable virtual machines using the libkrun platform, specifically designed for macOS with EFI boot support.

## Key Features

- **EFI Boot**: Uses OVMF/EDK2 firmware for UEFI boot
- **GPU Acceleration**: Venus (Mesa3D) and native context support
- **Configurable VMs**: Full control over VM parameters
- **macOS Native**: Leverages Hypervisor.framework on ARM64

## Installation

### Homebrew (Recommended)

```bash
brew tap slp/krunkit
brew install krunkit
```

This installs:
- krunkit binary
- libkrun-efi.dylib
- OVMF firmware files
- Dependencies (lld, xz)

## Building from Source

```bash
# Dependencies
# - Rust toolchain
# - libkrun-efi installed
# - macOS 14+

# If libkrun-efi is in non-standard location:
make LIBKRUN_EFI=/custom/path/lib/libkrun-efi.dylib

# Default (Homebrew location):
make

sudo make install
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      krunkit CLI                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   libkrun-efi.dylib                         │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  OVMF/EDK2 Firmware                                  │    │
│  │  - UEFI Boot                                         │    │
│  │  - Secure Boot support                               │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  VMM Core                                            │    │
│  │  - HVF (Hypervisor.framework)                        │    │
│  │  - virtio devices                                    │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                  macOS Hypervisor.framework                 │
└─────────────────────────────────────────────────────────────┘
```

## Usage

### Basic VM Launch

```bash
# Boot with firmware and disk
krunkit run \
    --firmware /path/to/OVMF.fd \
    --disk /path/to/disk.img
```

### GPU-Accelerated VM

```bash
# Enable virtio-gpu with Venus
krunkit run \
    --firmware /path/to/OVMF.fd \
    --disk /path/to/disk.img \
    --gpu \
    --gpu-shm-size 268435456
```

### Volume Mapping

```bash
# Map host directory (macOS virtio-fs)
krunkit run \
    --firmware /path/to/OVMF.fd \
    --disk /path/to/disk.img \
    --volume /host/path:/guest/path
```

### Port Forwarding

```bash
# Forward host port to guest
krunkit run \
    --firmware /path/to/OVMF.fd \
    --disk /path/to/disk.img \
    --port 8080:80
```

### Network Configuration

```bash
# Using vmnet (macOS)
krunkit run \
    --firmware /path/to/OVMF.fd \
    --disk /path/to/disk.img \
    --net vmnet

# Using gvproxy
krunkit run \
    --firmware /path/to/OVMF.fd \
    --disk /path/to/disk.img \
    --net gvproxy
```

## GPU Support

### Venus (Mesa3D)

Venus provides Vulkan support in guests via virtio-gpu:

```bash
# Enable Venus
krunkit run --gpu --gpu-flags venus
```

**Requirements**:
- Host: Mesa3D with Venus support
- Guest: virtio-gpu driver with Venus

### Native Context

Native context provides Metal-based GPU acceleration:

```bash
# Enable native context
krunkit run --gpu --gpu-flags native-context
```

**Use Cases**:
- Games requiring 4k pages
- macOS-native GPU workloads
- Low-latency graphics

### GPU Flags

| Flag | Description |
|------|-------------|
| `venus` | Mesa3D Venus Vulkan |
| `native-context` | macOS Metal context |
| `virgl` | VirGL renderer |

## Firmware Options

### OVMF (Open Virtual Machine Firmware)

krunkit uses OVMF for UEFI boot:

```bash
# Standard OVMF
krunkit run --firmware OVMF.fd ...

# OVMF with secure boot
krunkit run --firmware OVMF.secboot.fd ...
```

### Firmware Variables

```bash
# Separate variables store
krunkit run \
    --firmware /path/to/OVMF.fd \
    --variables /path/to/vars.fd \
    ...
```

## Disk Support

### Raw Images

```bash
krunkit run --disk /path/to/disk.raw ...
```

### Multiple Disks

```bash
krunkit run \
    --disk /path/to/root.raw \
    --disk /path/to/data.raw ...
```

### Read-Only Disks

```bash
krunkit run --disk /path/to/readonly.raw:ro ...
```

## Comparison with Alternatives

| Feature | krunkit | QEMU | UTM |
|---------|---------|------|-----|
| Boot Time | ~500ms | ~2s | ~1s |
| GPU (Venus) | Yes | Limited | No |
| GPU (Metal) | Yes | No | No |
| HVF Native | Yes | Yes | Yes |
| CLI Focus | Yes | Yes | No (GUI) |
| libkrun Based | Yes | No | No |

## Use Cases

### Gaming on macOS

krunkit enables running Windows/Linux games on macOS:

```bash
# Game VM with GPU
krunkit run \
    --firmware OVMF.fd \
    --disk windows.img \
    --gpu \
    --gpu-flags native-context \
    --cpus 8 \
    --memory 16384
```

### Development VMs

```bash
# Linux development environment
krunkit run \
    --firmware OVMF.fd \
    --disk ubuntu.img \
    --volume ~/projects:/projects \
    --cpus 4 \
    --memory 8192
```

### Testing Environments

```bash
# Ephemeral test VM
krunkit run \
    --firmware OVMF.fd \
    --disk test.img \
    --memory 2048 \
    --snapshot
```

## Internal Structure

```
krunkit/
├── Makefile
├── src/
│   ├── main.rs           # Entry point
│   ├── config.rs         # Configuration parsing
│   ├── vm/
│   │   ├── create.rs     # VM creation
│   │   ├── run.rs        # VM execution
│   │   └── gpu.rs        # GPU configuration
│   └── devices/
│       ├── disk.rs       # Block devices
│       ├── net.rs        # Network devices
│       └── fs.rs         # Filesystem devices
├── docs/
│   └── usage.md
└── LICENSE
```

## Configuration File

krunkit supports YAML configuration files:

```yaml
# vm-config.yaml
firmware: /path/to/OVMF.fd
cpus: 4
memory: 8192
disks:
  - path: /path/to/root.img
    read_only: false
volumes:
  - host: /Users/dev/projects
    guest: /projects
gpu:
  enabled: true
  flags:
    - native-context
  shm_size: 268435456
```

```bash
# Run with config
krunkit run --config vm-config.yaml
```

## Troubleshooting

### Common Issues

**GPU not working**:
```bash
# Check Venus support
glxinfo | grep Venus

# Check native context
system_profiler SPDisplaysDataType
```

**Firmware not found**:
```bash
# Homebrew location
ls /opt/homebrew/opt/ovmf/share/ovmf/
```

**Network issues**:
```bash
# Check vmnet status
sudo /usr/libexec/vmnet --check
```

## References

- [krunkit README](../../src.containers/krunkit/README.md)
- [krunkit Usage Docs](../../src.containers/krunkit/docs/usage.md)
- [libkrun README](../../src.containers/libkrun/README.md)
- [Venus Documentation](https://docs.mesa3d.org/drivers/venus.html)
