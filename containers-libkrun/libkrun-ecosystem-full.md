---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/
explored_at: 2026-03-19
---

# libkrun Ecosystem Exploration

A comprehensive exploration of the libkrun ecosystem and related virtualization technologies.

## Ecosystem Overview

```mermaid
graph TB
    subgraph "libkrun Core"
        libkrun[libkrun - Core VMM Library]
        libkrunfw[libkrunfw - Bundled Kernel]
        libkrun_sev[libkrun-sev - AMD SEV]
        libkrun_tdx[libkrun-tdx - Intel TDX]
        libkrun_efi[libkrun-efi - macOS EFI]
    end

    subgraph "libkrun Tools"
        krunvm[krunvm - OCI MicroVM CLI]
        krunkit[krunkit - macOS VM Launcher]
        muvm[muvm - MicroVM Manager]
    end

    subgraph "Container Runtimes"
        crun[crun - OCI Runtime]
        podman[Podman]
        containerd[containerd]
    end

    subgraph "Networking"
        passt[passt - User Networking]
        gvproxy[gvisor-tap-vsock]
        netavark[Netavark]
    end

    subgraph "GPU/Acceleration"
        virgl[virglrenderer]
        venus[Venus (Mesa3D)]
    end

    subgraph "Related VMMs"
        firecracker[Firecracker - AWS]
        cloudhv[Cloud Hypervisor]
        qemu[QEMU]
    end

    subgraph "Rust VMM Ecosystem"
        rustvmm[rust-vmm crates]
        kvmbindings[kvm-bindings]
        vmm_sys[vmm-sys-utils]
    end

    libkrun --> libkrunfw
    libkrun --> libkrun_sev
    libkrun --> libkrun_tdx
    libkrun --> libkrun_efi

    krunvm --> libkrun
    krunkit --> libkrun_efi
    muvm --> libkrun

    crun --> libkrun
    podman --> crun

    libkrun --> passt
    libkrun --> gvproxy

    libkrun --> virgl
    virgl --> venus

    crun --> containerd
    crun --> podman

    firecracker --> rustvmm
    cloudhv --> rustvmm
```

## Component Deep Dives

### 1. libkrun (Core VMM Library)

**Repository**: https://github.com/containers/libkrun

**Purpose**: A self-sufficient VMM library with a simple C API for running processes in partially isolated KVM/HVF environments.

**Key Characteristics**:
- Written in Rust with C API for broad language interoperability
- Minimal footprint (~20MB VMM overhead)
- Fast boot times (~200ms to userspace)
- Self-sufficient (no external VMM dependency)

**Variants**:
| Variant | Platform | Purpose |
|---------|----------|---------|
| libkrun | Linux/macOS | Generic VMM |
| libkrun-sev | Linux | AMD SEV encrypted VMs |
| libkrun-tdx | Linux | Intel TDX encrypted VMs |
| libkrun-efi | macOS | EFI boot with OVMF |

**Architecture**:
```
┌─────────────────────────────────────────────────────────────┐
│                    libkrun Architecture                      │
├─────────────────────────────────────────────────────────────┤
│  C API Layer                                                 │
│  ────────────                                                │
│  krun_create_ctx(), krun_set_vm_config(), krun_start_enter() │
│                                                              │
│  Context Manager                                             │
│  ───────────────                                             │
│  VM state, device configuration, execution context           │
│                                                              │
│  VMM Core                                                    │
│  ─────────                                                   │
│  KVM (Linux) / HVF (macOS) abstraction                       │
│                                                              │
│  Virtio Device Layer                                         │
│  ───────────────────                                         │
│  virtio-block, virtio-net, virtio-fs, virtio-vsock, etc.     │
│                                                              │
│  Platform Layer                                              │
│  ──────────────                                              │
│  libkrunfw (kernel), virglrenderer, passt/gvproxy            │
└─────────────────────────────────────────────────────────────┘
```

**See Also**: [libkrun Primary Deep Dive](libkrun-primary-deep-dive.md)

---

### 2. libkrunfw (Firmware Library)

**Repository**: https://github.com/containers/libkrunfw

**Purpose**: Bundles a Linux kernel as a dynamic library that libkrun can directly map into guest memory.

**Key Features**:
- Zero-copy kernel loading (direct mmap into guest memory)
- Pre-configured kernel (CONFIG_NR_CPUS=8, virtio built-in)
- Multiple variants (generic, SEV, TDX, EFI)
- GPL-2.0 (kernel), LGPL-2.1 (library code)

**Build Process**:
```
Linux Kernel Source (submodule)
         │
         ▼
Apply libkrun Patches (TSI, virtio optimizations)
         │
         ▼
Configure (defconfig + libkrun-specific options)
         │
         ▼
Compile Kernel
         │
         ▼
Bundle as Shared Library (.so)
         │
         ▼
libkrunfw.so / libkrunfw-sev.so / libkrunfw-tdx.so
```

**See Also**: [RootFS and Kernel Creation](libkrun-rootfs-kernel-creation.md)

---

### 3. krunvm (OCI MicroVM CLI)

**Repository**: https://github.com/containers/krunvm

**Purpose**: CLI utility for creating microVMs from OCI container images.

**Key Features**:
- Zero disk image maintenance (runs directly from extracted layers)
- Zero network configuration (automatic TSI or passt)
- Fast boot times
- Volume mapping support
- Port exposure support

**Usage Example**:
```bash
# Run a container as a VM
krunvm run alpine:latest --cmd "/bin/sh -c 'echo hello'"

# With volume mapping
krunvm run alpine:latest --volume /host/path:/guest/path

# With port forwarding
krunvm run nginx:latest --port 8080:80
```

**Architecture**:
```
┌─────────────────────────────────────────────────────────────┐
│                    krunvm Architecture                       │
├─────────────────────────────────────────────────────────────┤
│  CLI Interface                                               │
│  ─────────────                                               │
│  clap (Rust) - argument parsing                              │
│                                                              │
│  OCI Handler                                                 │
│  ───────────                                                 │
│  Pull manifest, download layers, extract rootfs              │
│                                                              │
│  VM Builder                                                  │
│  ──────────                                                  │
│  Configure libkrun context based on image and flags          │
│                                                              │
│  libkrun Integration                                         │
│  ───────────────────                                         │
│  Create context, set config, enter VM                        │
└─────────────────────────────────────────────────────────────┘
```

**See Also**: [krunvm Deep Dive](krunvm-deep-dive.md)

---

### 4. krunkit (macOS VM Launcher)

**Repository**: https://github.com/containers/krunkit

**Purpose**: Launch configurable VMs on macOS using libkrun-efi.

**Key Features**:
- GPU acceleration via Venus (Mesa3D)
- Native context for 4k page games
- EFI boot with OVMF
- Lightweight development environments

**Usage Example**:
```bash
# Launch a VM with GPU acceleration
krunkit run \
  --cpus 4 \
  --memory 4096 \
  --disk /path/to/disk.raw \
  --gpu

# Mount host directory
krunkit run \
  --disk /path/to/disk.raw \
  --volume /Users:/mnt/host
```

**Architecture**:
```
┌─────────────────────────────────────────────────────────────┐
│                    krunkit Architecture                      │
├─────────────────────────────────────────────────────────────┤
│  macOS HVF                                                   │
│  ───────────                                                 │
│  Apple Hypervisor.framework                                  │
│                                                              │
│  libkrun-efi                                                 │
│  ───────────                                                 │
│  EFI boot with OVMF                                          │
│                                                              │
│  virglrenderer                                               │
│  ─────────────                                               │
│  GPU command translation                                     │
│                                                              │
│  Venus (Mesa3D)                                              │
│  ───────                                                     │
│  Android GPU driver for macOS                                │
└─────────────────────────────────────────────────────────────┘
```

**See Also**: [krunkit Deep Dive](krunkit-deep-dive.md)

---

### 5. crun (OCI Runtime)

**Repository**: https://github.com/containers/crun

**Purpose**: Lightweight OCI container runtime written in C with libkrun integration.

**Key Features**:
- ~50% faster than runc (1.69s vs 3.34s for 100 containers)
- Lower memory footprint (512k minimum vs runc's 4M)
- libkrun integration for krun mode
- Written in C for minimal dependencies

**Performance Comparison**:
```
┌─────────────────────────────────────────────────────────────┐
│                    Runtime Performance                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Starting 100 containers:                                    │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━    │
│  runc:   ████████████████████████████████  3.34s            │
│  crun:   ████████████████                   1.69s           │
│                                                              │
│  Minimum memory footprint:                                   │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━    │
│  runc:   ████████████████████████████      4MB              │
│  crun:   ████                               512KB           │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**libkrun Integration**:
```c
// crun's libkrun integration
static int libkrun_create_container(libcrun_container_t *container,
                                    libcrun_error_t *err)
{
    int32_t ctx = krun_create_ctx();

    krun_set_vm_config(ctx, num_vcpus, ram_mib);
    krun_set_root(ctx, container->container_def->root->path);
    krun_set_exec(ctx, argv[0], argv, envp);

    return krun_start_enter(ctx);
}
```

**See Also**: [crun Deep Dive](crun-deep-dive.md)

---

### 6. muvm (MicroVM Manager)

**Repository**: https://github.com/containers/muvm

**Purpose**: High-level microVM manager built on libkrun.

**Key Features**:
- Simple CLI interface
- Automatic resource management
- Integration with Podman
- Targeted at development environments

**Usage Example**:
```bash
# Run a microVM
muvm run alpine:latest

# With custom resources
muvm run --cpus 4 --memory 4G alpine:latest

# With GUI application
muvm run --gui firefox alpine:latest
```

---

### 7. Networking Components

#### passt (User-mode Networking)

**Repository**: https://github.com/containers/passt

**Purpose**: User-mode networking backend for libkrun.

**Key Features**:
- No root privileges required
- NAT with port forwarding
- DHCP and DNS support
- IPv4 and IPv6

**How it Works**:
```
┌─────────────────────────────────────────────────────────────┐
│                    passt Architecture                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Guest (VM)                                                  │
│  ─────────                                                   │
│  virtio-net device                                           │
│       │                                                      │
│       ▼                                                      │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  libkrun (VMM)                                        │  │
│  │       │                                                │  │
│  │       ▼                                                │  │
│  │  Unix socket (virtio-net backend)                      │  │
│  └───────────────────────────────────────────────────────┘  │
│       │                                                      │
│       ▼                                                      │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  passt process                                        │  │
│  │       │                                                │  │
│  │       ├──→ TCP proxy (port forwarding)                 │  │
│  │       ├──→ UDP proxy                                  │  │
│  │       ├──→ DHCP server                                │  │
│  │       └──→ DNS proxy                                  │  │
│  └───────────────────────────────────────────────────────┘  │
│       │                                                      │
│       ▼                                                      │
│  Host Network                                                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### gvisor-tap-vsock

**Repository**: https://github.com/containers/gvisor-tap-vsock

**Purpose**: Go-based userspace networking with advanced features.

**Key Features**:
- HTTP/HTTPS proxy
- Dynamic DNS
- VPN-like functionality
- Used by krunkit on macOS

---

### 8. GPU/Acceleration

#### virglrenderer

**Repository**: https://gitlab.freedesktop.org/virgl/virglrenderer

**Purpose**: 3D acceleration for virtio-gpu devices.

**Key Features**:
- OpenGL command translation
- Works with QEMU, crosvm, libkrun
- Supports both Linux and macOS

#### Venus (Mesa3D)

**Repository**: https://docs.mesa3d.org/drivers/venus.html

**Purpose**: Vulkan driver for virtio-gpu.

**Key Features**:
- Vulkan support for VMs
- Used by krunkit for macOS GPU acceleration
- Based on ANGLE/Vulkan translation

---

## Related VMM Projects

### Firecracker

**Repository**: https://github.com/firecracker-microvm/firecracker

**Purpose**: AWS's microVMM for serverless workloads.

**Comparison with libkrun**:
| Feature | Firecracker | libkrun |
|---------|-------------|---------|
| Language | Rust | Rust |
| API | JSON over Unix socket | C API |
| Boot time | ~125ms | ~200ms |
| Guest memory | 5MB overhead | ~20MB overhead |
| Device model | Minimal (virtio only) | More devices |
| Networking | tap device | TSI/passt |
| GPU support | No | Yes (via virgl) |
| Confidential | No | Yes (SEV/TDX) |

**See Also**: [Firecracker Deep Dive](../../src.firecracker/firecracker-deep-dive.md)

### Cloud Hypervisor

**Repository**: https://github.com/cloud-hypervisor/cloud-hypervisor

**Purpose**: Rust-based VMM for cloud workloads.

**Key Features**:
- KVM and MSHV (Windows) support
- x86-64, AArch64, RISC-V 64
- Part of rust-vmm ecosystem
- More full-featured than Firecracker

**Comparison with libkrun**:
| Feature | Cloud Hypervisor | libkrun |
|---------|------------------|---------|
| Target | Cloud VMs | Container isolation |
| Complexity | Higher | Lower |
| API | REST/HTTP | C API |
| Boot time | ~500ms | ~200ms |
| Device model | Full virtio | Selective virtio |

---

## Rust VMM Ecosystem

### rust-vmm

**Repository**: https://github.com/rust-vmm

**Purpose**: Foundational Rust VMM crates shared across projects.

**Key Crates**:
| Crate | Purpose | Used By |
|-------|---------|---------|
| vm-memory | Guest memory management | Firecracker, Cloud Hypervisor |
| kvm-bindings | KVM ioctl bindings | Firecracker, Cloud Hypervisor, libkrun |
| kvm-ioctls | KVM ioctl wrappers | Firecracker, Cloud Hypervisor |
| virtio-queue | Virtio queue management | Firecracker, Cloud Hypervisor |
| vhost | vhost protocol implementation | Firecracker, Cloud Hypervisor |
| seccompiler | seccomp filter compiler | Firecracker, Cloud Hypervisor |
| vmm-sys-util | VMM system utilities | Firecracker, Cloud Hypervisor |
| versionize | Serialization with versioning | Firecracker, Cloud Hypervisor |

**See Also**: [rust-vmm Directory](../../src.rust-vmm/)

### kvm-bindings

**Repository**: https://github.com/rust-vmm/kvm-bindings

**Purpose**: Rust FFI bindings for KVM ioctls.

**Generated from**: Linux kernel KVM headers

**Usage Example**:
```rust
use kvm_bindings::{kvm_userspace_memory_region, KVM_MEM_LOG_DIRTY_PAGES};
use kvm_ioctls::{Kvm, VmFd};

// Create KVM context
let kvm = Kvm::new()?;

// Create VM
let vm_fd = kvm.create_vm()?;

// Set up guest memory
let mem_region = kvm_userspace_memory_region {
    slot: 0,
    guest_phys_addr: 0x100000,
    memory_size: 0x10000000,
    userspace_addr: userspace_addr as u64,
    flags: KVM_MEM_LOG_DIRTY_PAGES,
};

unsafe {
    vm_fd.set_user_memory_region(mem_region)?;
}
```

---

## Directory Structure Reference

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/
│
├── src.containers/
│   ├── libkrun/           # Core VMM library
│   ├── libkrunfw/         # Bundled kernel library
│   ├── krunvm/            # OCI microVM CLI
│   ├── krunkit/           # macOS VM launcher
│   ├── crun/              # OCI runtime with libkrun
│   ├── muvm/              # MicroVM manager
│   ├── passt/             # User-mode networking
│   ├── gvisor-tap-vsock/  # Go userspace networking
│   │
│   ├── firecracker/       # AWS microVMM
│   ├── cloud-hypervisor/  # Cloud VMM
│   ├── rust-vmm/          # Rust VMM foundational crates
│   │
│   └── [80+ other container/VM projects]
│
└── src.cloud-hypervisor/  # Cloud Hypervisor specific
    src.firecracker/       # Firecracker specific
    src.weave-ignite/      # Ignite (Firecracker manager)
```

---

## Quick Comparison

### VMM Comparison

| VMM | Best For | Boot Time | Memory | Language |
|-----|----------|-----------|--------|----------|
| libkrun | Container isolation | ~200ms | ~20MB | Rust |
| Firecracker | Serverless | ~125ms | ~5MB | Rust |
| Cloud Hypervisor | Cloud VMs | ~500ms | ~30MB | Rust |
| QEMU | General purpose | ~2s | ~50MB | C |
| Weave Ignite | Developer VMs | ~1s | ~20MB | Go |

### Networking Comparison

| Method | Pros | Cons | Used By |
|--------|------|------|---------|
| TSI (virtio-vsock) | No virtual NIC, simple | Requires custom kernel | libkrun |
| passt | User-mode, no root | NAT only | libkrun, crun |
| gvproxy | Advanced features | Go dependency | krunkit |
| tap device | Full networking | Requires root | Firecracker, Cloud HV |

---

## References

- [libkrun GitHub](https://github.com/containers/libkrun)
- [libkrun Matrix Channel](https://matrix.to/#/#libkrun:matrix.org)
- [crun Documentation](https://github.com/containers/crun/blob/main/README.md)
- [Firecracker Specification](../../src.firecracker/firecracker/SPECIFICATION.md)
- [rust-vmm Crates](https://crates.io/organizations/rust-vmm)
- [OCI Runtime Specification](https://github.com/opencontainers/runtime-spec)
