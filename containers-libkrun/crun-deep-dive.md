---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.Containers/src.containers/crun
explored_at: 2026-03-19
---

# crun Deep Dive

## Purpose

crun is a fast and low-memory footprint OCI Container Runtime fully written in C. It conforms to the OCI Container Runtime specification and can use libkrun for VM-based isolation.

## Performance

### Speed Comparison

Running 100 containers sequentially with `/bin/true`:

| Runtime | Time | Relative |
|---------|------|----------|
| crun | 1.69s | 100% |
| runc | 3.34s | 198% |

**crun is ~50% faster than runc**

### Memory Footprint

| Runtime | Minimum Memory |
|---------|---------------|
| crun | 512 KB |
| runc | 4 MB |

crun can run in 8x less memory than runc.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Container Orchestration                   │
│              (Kubernetes, Podman, Docker, etc.)              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                         crun                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   OCI       │  │   libkrun   │  │   Standard          │  │
│  │   Runtime   │  │   Mode      │  │   Mode              │  │
│  │   Handler   │  │  (Optional) │  │  (runc-like)        │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
       ┌────────────┐ ┌────────────┐  ┌────────────┐
       │  Namespaces│ │   KVM      │  │   cgroups  │
       │  (Standard)│ │  (krun)    │  │            │
       └────────────┘ └────────────┘  └────────────┘
```

## Features

### Standard OCI Runtime

- Full OCI specification compliance
- Namespace isolation
- cgroups support
- seccomp profiles
- capabilities handling
- hooks support

### krun Mode

When using libkrun integration:

- VM-based isolation
- Separate kernel instance
- Enhanced security boundary
- Confidential computing support (SEV/TDX)

## Installation

### Fedora/RHEL

```bash
dnf install crun
```

### Ubuntu/Debian

```bash
apt install crun
```

### Alpine

```bash
apk add crun
```

### Building from Source

```bash
# Dependencies (Fedora/RHEL)
dnf install -y \
    autoconf automake gcc git-core glibc-static go-md2man \
    libcap-devel libseccomp-devel libtool make pkg-config \
    python python-libmount systemd-devel yajl-devel

# Build
./autogen.sh
./configure
make
sudo make install
```

### Static Build (Nix)

```bash
# Install Nix
curl -L https://nixos.org/nix/install | sh

# Build static binary
git clone --recursive https://github.com/containers/crun.git
cd crun
nix build -f nix/

# Run
./result/bin/crun --version
```

## Configuration

### Runtime Configuration

```json
// /etc/crun/crun.json
{
    "container_name_template": "{uuid}",
    "hooks": {
        "prestart": ["/path/to/hook.sh"]
    },
    "runtimes": {
        "krun": {
            "path": "/usr/bin/crun",
            "args": ["--runtime=krun"]
        }
    }
}
```

### Podman Integration

```bash
# Configure crun as default runtime
cat > /etc/containers/libpod.conf << EOF
runtime = "crun"
EOF

# Or use krun mode
podman run --runtime /usr/bin/crun --security-opt seccomp=unconfined image
```

## krun Mode Usage

### Enable krun Mode

```bash
# Run container with krun isolation
crun --runtime=krun run my-container

# With Podman
podman run --runtime /usr/bin/crun:krun my-image
```

### krun-Specific Options

```bash
# Set VM resources
--annotation=io.containers.krun.cpus=4
--annotation=io.containers.krun.memory=4096

# Enable GPU
--annotation=io.containers.krun.gpu=true
```

## Build Options

### Configure Flags

```bash
# Enable shared library
./configure --enable-shared

# Enable embedded yajl (RHEL/CentOS 10)
./configure --enable-embedded-yajl

# Enable systemd support
./configure --enable-systemd

# Enable seccomp
./configure --with-seccomp

# Custom prefix
./configure --prefix=/usr
```

### Feature Matrix

| Feature | Flag | Default |
|---------|------|---------|
| seccomp | `--with-seccomp` | Yes |
| systemd | `--enable-systemd` | Yes |
| yajl | `--enable-embedded-yajl` | System |
| crun-wasm | `--enable-crun-wasm` | No |
| go-md2man | `--with-go-md2man` | Yes |

## Internal Structure

```
crun/
├── configure.ac          # Autoconf configuration
├── src/
│   ├── libcrun/
│   │   ├── crun.c        # Main crun logic
│   │   ├── container.c   # Container management
│   │   ├── linux.c       # Linux-specific code
│   │   ├── krun.c        # libkrun integration
│   │   ├── seccomp.c     # Seccomp handling
│   │   ├── cgroups.c     # cgroups management
│   │   └── ...
│   └── crun.c            # CLI entry point
├── tests/
│   ├── tests_libcrun/    # Unit tests
│   └── integration/      # Integration tests
├── lua/
│   └── crun.lua          # Lua bindings
└── docs/
    └── crun.1.md         # Manual page
```

## libkrun Integration

### Architecture

```c
// Simplified krun integration in crun

int libcrun_container_enter_krun(libcrun_container_t *container) {
    // Create libkrun context
    int32_t ctx_id = krun_create_ctx();

    // Configure VM
    krun_set_vm_config(ctx_id, cpus, memory_mib);

    // Set root filesystem
    krun_set_root(ctx_id, container->rootfs);

    // Configure executable
    krun_set_exec(ctx_id,
                  container->command,
                  container->args,
                  container->env);

    // Enter VM
    return krun_start_enter(ctx_id);
}
```

### Krun-Specific Code Paths

When `--runtime=krun` is specified:

1. **Container Setup**:
   - Standard OCI bundle preparation
   - Root filesystem mounted

2. **VM Configuration**:
   - libkrun context created
   - VM resources configured (CPUs, memory)
   - virtio devices configured

3. **VM Entry**:
   - `krun_start_enter()` called
   - Guest boots with bundled kernel (libkrunfw)
   - Init process runs in VM

4. **Exit Handling**:
   - VM exit code propagated
   - Cleanup performed

## Security Features

### Standard Mode

- seccomp profiles
- capabilities dropping
- namespace isolation
- AppArmor/SELinux support
- rootless containers

### krun Mode

- VM-level isolation
- Separate kernel instance
- Hardware virtualization
- SEV/TDX support (with appropriate libkrun variant)
- Confidential computing

## Comparison with runc

| Feature | crun | runc |
|---------|------|------|
| Language | C | Go |
| Binary Size | ~200 KB | ~15 MB |
| Memory | 512 KB min | 4 MB min |
| Startup Time | ~50ms | ~100ms |
| libkrun Support | Yes | No |
| Wasm Support | Yes (optional) | No |

## Lua Bindings

crun provides Lua bindings for extensibility:

```lua
-- crun.lua example
crun = require("crun")

function prestart(container)
    print("Container starting: " .. container.id)
end

function poststop(container)
    print("Container stopped: " .. container.id)
end
```

## Troubleshooting

### Debug Mode

```bash
# Enable debug logging
crun --debug run my-container

# Specify log file
crun --log=/var/log/crun.log run my-container
```

### Common Issues

**Permission denied**:
```bash
# Check capabilities
getcap /usr/bin/crun

# Should have: /usr/bin/crun = cap_sys_admin+ep
```

**seccomp errors**:
```bash
# Check seccomp support
grep Seccomp /proc/self/status

# Disable seccomp for testing
crun run --security-opt seccomp=unconfined my-container
```

## References

- [crun README](../../src.containers/crun/README.md)
- [crun Manual](crun.1.md)
- [OCI Runtime Specification](https://github.com/opencontainers/runtime-spec)
- [libkrun README](../../src.containers/libkrun/README.md)
