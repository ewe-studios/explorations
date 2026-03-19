---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.containers/libkrun
explored_at: 2026-03-19
---

# libkrun Deep Dive

## Purpose

libkrun is a dynamic library that allows programs to acquire the ability to run processes in a partially isolated environment using KVM (Linux) or HVF (macOS/ARM64) virtualization.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Application (crun, krunvm, etc.)         │
│                         C API Client                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      libkrun (C API)                         │
│                    include/libkrun.h                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    libkrun Core (Rust)                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   VMM       │  │   Virtio    │  │   KVM/HVF           │  │
│  │   Core      │  │   Devices   │  │   Abstraction       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Host Kernel                               │
│              KVM (Linux) or HVF (macOS)                      │
└─────────────────────────────────────────────────────────────┘
```

## Variants

| Variant | Library Name | Use Case |
|---------|-------------|----------|
| Generic | libkrun.so | Standard KVM/HVF virtualization |
| SEV | libkrun-sev.so | AMD SEV/SEV-ES/SEV-SNP encryption |
| TDX | libkrun-tdx.so | Intel TDX encryption |
| EFI | libkrun-efi.so | macOS with OVMF/EDK2 |

## Virtio Device Support

### All Variants Support

| Device | Purpose |
|--------|---------|
| virtio-console | Serial console I/O |
| virtio-block | Block storage devices |
| virtio-fs | Filesystem sharing |
| virtio-gpu | Graphics (venus, native-context) |
| virtio-net | Network interfaces |
| virtio-vsock | VM-host socket communication + TSI |
| virtio-balloon | Memory management (free-page reporting only) |
| virtio-rng | Random number generation |
| virtio-snd | Audio |

## API Reference

### Context Management

```c
// Create a new configuration context
int32_t krun_create_ctx();

// Free a configuration context
int32_t krun_free_ctx(uint32_t ctx_id);

// Set log level (0=Off, 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace)
int32_t krun_set_log_level(uint32_t level);
```

### VM Configuration

```c
// Set VM resources
int32_t krun_set_vm_config(uint32_t ctx_id,
                           uint8_t num_vcpus,
                           uint32_t ram_mib);

// Set root filesystem (not available in SEV variant)
int32_t krun_set_root(uint32_t ctx_id, const char *root_path);

// Add disk image (raw format only)
int32_t krun_add_disk(uint32_t ctx_id,
                      const char *block_id,
                      const char *disk_path,
                      bool read_only);

// Add disk with format support (raw, qcow2, vmdk)
int32_t krun_add_disk2(uint32_t ctx_id,
                       const char *block_id,
                       const char *disk_path,
                       uint32_t disk_format,
                       bool read_only);
```

### Networking

```c
// Add network via unixstream (passt, socket_vmnet)
int32_t krun_add_net_unixstream(uint32_t ctx_id,
                                const char *c_path,
                                int fd,
                                uint8_t *c_mac,
                                uint32_t features,
                                uint32_t flags);

// Add network via unixdgram (gvproxy, vmnet-helper)
int32_t krun_add_net_unixgram(uint32_t ctx_id,
                              const char *c_path,
                              int fd,
                              uint8_t *c_mac,
                              uint32_t features,
                              uint32_t flags);

// Add tap network (Linux only)
int32_t krun_add_net_tap(uint32_t ctx_id,
                         char *c_tap_name,
                         uint8_t *c_mac,
                         uint32_t features,
                         uint32_t flags);
```

### Execution

```c
// Set working directory
int32_t krun_set_workdir(uint32_t ctx_id, const char *workdir_path);

// Set executable with arguments and environment
int32_t krun_set_exec(uint32_t ctx_id,
                      const char *exec_path,
                      const char *const argv[],
                      const char *const envp[]);

// Enter the microVM (consumes context)
int32_t krun_start_enter(uint32_t ctx_id);
```

## Example Usage

```c
#include <libkrun.h>

int main() {
    int32_t ctx_id = krun_create_ctx();
    if (ctx_id < 0) {
        // Handle error
        return 1;
    }

    // Configure VM: 2 vCPUs, 512 MiB RAM
    krun_set_vm_config(ctx_id, 2, 512);

    // Set root filesystem
    krun_set_root(ctx_id, "/path/to/rootfs");

    // Set executable
    const char *argv[] = {"/bin/sh", NULL};
    krun_set_exec(ctx_id, "/bin/sh", argv, NULL);

    // Enter VM (this call may not return)
    int32_t ret = krun_start_enter(ctx_id);

    // If we get here, an error occurred
    if (ret < 0) {
        // Handle error
        return 1;
    }

    return 0;
}
```

## Build Process

### Linux Build

```bash
# Dependencies
# - libkrunfw (provides libkrunfw.so)
# - Rust toolchain
# - glibc-static (for static init binary)
# - patchelf

# Build with optional features
make BLK=1 NET=1 SND=1 GPU=1

# Install
sudo make install
```

### Feature Flags

| Flag | Description | Dependencies |
|------|-------------|--------------|
| `BLK=1` | Enable virtio-block | - |
| `NET=1` | Enable virtio-net | - |
| `SND=1` | Enable virtio-snd | - |
| `GPU=1` | Enable virtio-gpu | virglrenderer-devel |
| `VIRGL_RESOURCE_MAP2=1` | Use virgl_resource_map2 | virglrenderer MR #1374 |

### SEV Variant

```bash
# Additional dependencies
# - libkrunfw-sev.so
# - openssl-devel

make SEV=1
sudo make SEV=1 install
```

### TDX Variant

```bash
# Additional dependencies
# - libkrunfw-tdx.so
# - openssl-devel

make TDX=1
sudo make TDX=1 install
```

**TDX Limitations**:
- Maximum 1 vCPU
- Maximum 3072 MiB memory

### macOS (EFI Variant)

```bash
# Requirements: Rust, macOS 14+, lld, xz

make EFI=1
sudo make EFI=1 install
```

## Security Considerations

### Security Model

- **Guest and VMM share the same security context**
- VMM acts as proxy for guest in many operations
- Host resources accessible to VMM can be accessed by guest

### Mitigations

1. **Run VMM in isolated context**:
   - Use namespaces on Linux
   - Apply UID/GID restrictions

2. **virtio-fs Protection**:
   - Use mount point isolation
   - Apply resource controls (inode limits, disk capacity)

3. **TSI Network Protection**:
   - Apply network restrictions to VMM process
   - Guest inherits VMM network context

### Exit Codes

| Code | Meaning |
|------|---------|
| 125 | Init cannot set up environment |
| 126 | Executable found but cannot execute |
| 127 | Executable not found |
| -EINVAL | VMM configuration error |

## Internal Structure

### Workspace Members

```
libkrun/
├── Cargo.toml              # Workspace definition
├── src/libkrun/            # Main library implementation
├── src/krun_input/         # Input device handling
├── include/libkrun.h       # Public C API
├── init/init.c             # Static init binary for guest
├── examples/
│   └── chroot_vm/          # Example chroot-like tool
└── tests/                  # Integration tests
```

### Key Rust Crates

Based on the workspace, libkrun uses:
- `krun-sys`: System bindings and FFI
- Various rust-vmm crates for virtualization primitives

## Integration Points

### With crun

crun uses libkrun to provide VM-based isolation for OCI containers:

```bash
# Run container with krun isolation
crun --runtime=krun run my-container
```

### With krunvm

krunvm uses libkrun to create VMs from OCI images:

```bash
# Create VM from OCI image
krunvm run ubuntu:22.04 --cpus 2 --memory 2048
```

### With krunkit

krunkit provides macOS-specific VM launching:

```bash
# Launch VM on macOS
krunkit run --firmware OVMF.fd --disk ubuntu.raw
```

## References

- [libkrun README](../../src.containers/libkrun/README.md)
- [libkrun.h API Header](../../src.containers/libkrun/include/libkrun.h)
- [libkrun Matrix Channel](https://matrix.to/#/#libkrun:matrix.org)
