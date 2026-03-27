---
title: "Colima Volume Mounting Deep Dive"
subtitle: "Volume drivers, 9p, virtiofs, sshfs, inotify propagation, and performance optimization"
based_on: "Colima - Lima-based Container Runtime"
level: "Intermediate to Advanced"
prerequisites: "[VM Management Deep Dive](01-vm-management-deep-dive.md)"
---

# Volume Mounting Deep Dive

## Table of Contents

1. [Volume Mount Fundamentals](#1-volume-mount-fundamentals)
2. [Mount Types: sshfs, 9p, virtiofs](#2-mount-types-sshfs-9p-virtiofs)
3. [Mount Configuration](#3-mount-configuration)
4. [Inotify Propagation](#4-inotify-propagation)
5. [Volume Drivers Architecture](#5-volume-drivers-architecture)
6. [Performance Considerations](#6-performance-comparisons)
7. [Troubleshooting Mounts](#7-troubleshooting-mounts)

---

## 1. Volume Mount Fundamentals

### 1.1 Why Volume Mounts Matter

Containers need access to host files for:
- **Development** - Edit code on host, run in container
- **Data persistence** - Survive container restarts
- **Sharing** - Multiple containers access same files
- **Performance** - Access large datasets without copying

```
┌─────────────────────────────────────────────────────────┐
│                    Host (macOS)                         │
│  /Users/dev/project/                                    │
│  ├── src/                                               │
│  ├── package.json                                       │
│  └── node_modules/                                      │
│         │                                               │
│         │ MOUNT (read/write)                            │
│         v                                               │
│  ┌──────────────────────────────────────────────────┐   │
│  │              Lima VM (Linux)                     │   │
│  │  /Users/dev/project/  <- Same path               │   │
│  │  ┌────────────────────────────────────────────┐  │   │
│  │  │            Container                       │  │   │
│  │  │  /app/  <- Mount point inside container    │  │   │
│  │  └────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Mount Challenges on macOS

| Challenge | Description | Solution |
|-----------|-------------|----------|
| **Different filesystems** | APFS (macOS) vs ext4 (Linux VM) | Network filesystem (9p, virtiofs) |
| **Performance** | Network overhead | Caching, virtiofs (shared memory) |
| **File events** | inotify not crossing VM boundary | inotify daemon proxy |
| **Permissions** | Different UID/GID mapping | Mount options, ID mapping |
| **Case sensitivity** | APFS case-insensitive, Linux case-sensitive | Consistent naming, warnings |

### 1.3 Mount Flow

```
1. User specifies mount in config
         │
         v
2. Colima adds mount to Lima config
         │
         v
3. Lima configures VM with mount
         │
         v
4. VM mounts host directory via network FS
         │
         v
5. Container mounts VM directory as volume
```

---

## 2. Mount Types: sshfs, 9p, virtiofs

### 2.1 Comparison Table

| Feature | sshfs | 9p | virtiofs |
|---------|-------|----|----------|
| **Protocol** | SSH/SFTP | 9P2000.L | Virtio-fs |
| **VM Type** | QEMU | QEMU | vz, krunkit |
| **Performance** | Slow | Moderate | Fast |
| **CPU Overhead** | High | Moderate | Low |
| **Features** | Basic | Symlinks, hardlinks | Full POSIX |
| **Setup** | Automatic | Requires QEMU config | Requires vz |
| **Use Case** | Fallback | QEMU with features | Apple Silicon macOS 13+ |

### 2.2 sshfs (SSH Filesystem)

**How it works:**
- Uses SSH protocol for file access
- Runs over standard SSH connection
- No special VM configuration needed

```go
// Default for QEMU VMs
if conf.VMType != "vz" && conf.VMType != "krunkit" {
    conf.MountType = "sshfs"  // Default for QEMU
}
```

**Advantages:**
- Works everywhere SSH works
- No additional dependencies
- Secure (encrypted)

**Disadvantages:**
- Slowest option (network round-trips)
- High CPU overhead
- Limited POSIX compliance
- No file event support

**Performance characteristics:**
```
Small files:  ~10-50 MB/s
Large files:  ~50-100 MB/s
Latency:      ~5-10ms per operation
```

### 2.3 9p (Plan 9 Filesystem)

**How it works:**
- Plan 9 filesystem protocol
- Shared memory between QEMU and host
- Direct VM-host communication

```go
// 9p configuration in Lima
type Mount struct {
    Location   string
    MountPoint string
    Writable   bool
    NineP      NineP  // 9p-specific options
}

type NineP struct {
    SecurityModel   string  // passthrough, mapped, etc.
    ProtocolVersion string  // 9p2000.L
    Msize           string  // Message size
    Cache           string  // cache mode
}
```

**Advantages:**
- Better performance than sshfs
- Supports symlinks and hardlinks
- Good POSIX compliance

**Disadvantages:**
- QEMU only (not available in vz)
- Requires QEMU configuration
- Moderate CPU overhead

**Performance characteristics:**
```
Small files:  ~100-200 MB/s
Large files:  ~200-400 MB/s
Latency:      ~1-2ms per operation
```

### 2.4 virtiofs (Virtual Filesystem)

**How it works:**
- Virtio-based shared filesystem
- DAX (Direct Access) for zero-copy
- Shared memory region between host and VM

```go
// Default for vz VMs
if conf.VMType == "vz" || conf.VMType == "krunkit" {
    conf.MountType = "virtiofs"  // Default for vz
}
```

**Advantages:**
- Best performance
- Lowest CPU overhead
- Full POSIX compliance
- Native to Apple Virtualization Framework

**Disadvantages:**
- macOS 13+ only (vz backend)
- Apple Silicon only

**Performance characteristics:**
```
Small files:  ~500 MB/s - 1 GB/s
Large files:  ~1-2 GB/s
Latency:      ~0.1-0.5ms per operation
```

### 2.5 Mount Type Selection Logic

```go
// From start.go
func setFlagDefaults(cmd *cobra.Command) {
    defaultMountTypeQEMU := "sshfs"
    defaultMountTypeVZ := "virtiofs"

    // Auto-select based on VM type
    if cmd.Flag("vm-type").Changed && startCmdArgs.VMType == "vz" {
        if !cmd.Flag("mount-type").Changed {
            startCmdArgs.MountType = defaultMountTypeVZ
        }
    }

    // Convert incompatible mount types
    if startCmdArgs.VMType != "vz" && startCmdArgs.VMType != "krunkit" {
        if startCmdArgs.MountType == "virtiofs" {
            startCmdArgs.MountType = defaultMountTypeQEMU
            log.Warnf("virtiofs only available for 'vz' vmType, using %s", defaultMountTypeQEMU)
        }
    }

    if startCmdArgs.VMType == "vz" && startCmdArgs.MountType == "9p" {
        startCmdArgs.MountType = "virtiofs"
        log.Warnf("9p only available for 'qemu' vmType, using %s", defaultMountTypeVZ)
    }
}
```

---

## 3. Mount Configuration

### 3.1 Config Format

```yaml
# ~/.colima/default/colima.yaml
mounts:
  # Single mount (writable)
  - location: ~/projects
    writable: true

  # Single mount (read-only)
  - location: ~/data
    writable: false

  # Custom mount point in VM
  - location: ~/projects
    mountPoint: /workspace
    writable: true

  # Disable all mounts
  - location: none
```

```go
// From config/config.go
type Mount struct {
    Location   string `yaml:"location"`     // Host path
    MountPoint string `yaml:"mountPoint,omitempty"`  // VM path (optional)
    Writable   bool   `yaml:"writable"`     // Read/write permission
}

func (c Config) MountsOrDefault() []Mount {
    // Empty explicit list means mount home directory
    if c.Mounts != nil && len(c.Mounts) == 0 {
        return []Mount{
            {Location: util.HomeDir(), Writable: true},
        }
    }
    // nil means no mounts, non-nil means user-specified
    return c.Mounts
}
```

### 3.2 CLI Mount Specification

```bash
# Single writable mount
colima start --mount ~/projects:w

# Multiple mounts
colima start --mount ~/projects:w --mount ~/data

# Disable mounting
colima start --mount none

# Custom mount point
colima start --mount ~/projects:/workspace:w
```

```go
// From start.go - mountsFromFlag
func mountsFromFlag(mounts []string) []config.Mount {
    mnts := make([]config.Mount, len(mounts))
    for i, mount := range mounts {
        // Handle "none" special case
        if strings.ToLower(mount) == "none" {
            return nil
        }

        str := strings.SplitN(mount, ":", 3)
        mnt := config.Mount{Location: str[0]}

        // Parse writable flag
        if len(str) > 1 {
            if filepath.IsAbs(str[1]) {
                mnt.MountPoint = str[1]
            } else if str[1] == "w" {
                mnt.Writable = true
            }
        }
        if len(str) > 2 && str[2] == "w" {
            mnt.Writable = true
        }

        mnts[i] = mnt
    }
    return mnts
}
```

### 3.3 Lima Mount Translation

```go
// From lima/yaml.go
func newConf(ctx context.Context, conf config.Config) (l limaconfig.Config, error) {
    // Translate mounts to Lima format
    for _, m := range conf.MountsOrDefault() {
        l.Mounts = append(l.Mounts, limaconfig.Mount{
            Location:   m.Location,
            MountPoint: m.MountPoint,
            Writable:   m.Writable,
            NineP: limaconfig.NineP{
                SecurityModel:   "none",
                ProtocolVersion: "9p2000.L",
                Msize:           "128KiB",
                Cache:           "fscache",
            },
        })
    }
    return l, nil
}
```

### 3.4 Container Volume Mounts

Once the VM mount is established, containers can mount VM directories:

```bash
# Run container with volume mount
docker run -v /Users/dev/project:/app myimage

# The path /Users/dev/project is:
# 1. Mounted from host to VM by Lima
# 2. Mounted from VM to container by Docker
```

---

## 4. Inotify Propagation

### 4.1 The Inotify Problem

**Inotify** is a Linux kernel feature that notifies applications about file changes. It's essential for:
- Hot reload in development
- File watchers (webpack, nodemon, etc.)
- IDE file synchronization

**Problem:** File events don't cross VM boundaries by default.

```
┌─────────────────────────────────────────────────────────┐
│  Host (macOS)              VM (Linux)                   │
│                                                         │
│  File change  ──X──>  inotify event NOT received       │
│                                                         │
│  Reason: File events are kernel-level, VM has separate │
│          kernel instance                                │
└─────────────────────────────────────────────────────────┘
```

### 4.2 Inotify Daemon Solution

Colima runs a daemon that watches host files and forwards events to the VM:

```go
// From daemon/daemon.go
func (l processManager) Start(ctx context.Context, conf config.Config) error {
    args := []string{osutil.Executable(), "daemon", "start", config.CurrentProfile().ShortName}

    if conf.MountINotify {
        args = append(args, "--inotify")
        args = append(args, "--inotify-runtime", conf.Runtime)
        for _, mount := range conf.MountsOrDefault() {
            p, _ := util.CleanPath(mount.Location)
            args = append(args, "--inotify-dir", p)
        }
    }

    return host.RunQuiet(args...)
}
```

### 4.3 Inotify Process Architecture

```go
// From inotify/inotify.go
type inotifyProcess struct {
    vmVols  []string                    // VM volumes to watch
    guest   environment.GuestActions    // Guest access for event injection
    runtime string                      // Container runtime (docker, containerd)
    log     *logrus.Entry
}

func (f *inotifyProcess) Start(ctx context.Context) error {
    args, ok := ctx.Value(CtxKeyArgs()).(Args)
    if !ok {
        return fmt.Errorf("args missing in context")
    }

    f.vmVols = omitChildrenDirectories(args.Dirs)
    f.guest = args.GuestActions
    f.runtime = args.Runtime

    // Wait for VM to start
    f.waitForLima(ctx)

    // Start watching
    watcher := &defaultWatcher{log: f.log}
    return f.handleEvents(ctx, watcher)
}
```

### 4.4 Event Flow

```
1. Host file changes
         │
         v
2. Host inotify detects change
         │
         v
3. inotify daemon captures event
         │
         v
4. Event sent to VM via socket
         │
         v
5. VM injects event into container runtime
         │
         v
6. Container receives inotify event
```

### 4.5 Inotify Configuration

```yaml
# ~/.colima/default/colima.yaml
mountInotify: true  # Enable inotify propagation (default: true)
```

```bash
# Disable inotify
colima start --mount-inotify=false

# Enable inotify (default)
colima start --mount-inotify=true
```

### 4.6 Volume Watching

```go
// From inotify/volumes.go
func (f *inotifyProcess) handleEvents(ctx context.Context, watcher watcher) error {
    log := f.log

    // Watch for Docker/containerd volumes
    go func() {
        for {
            select {
            case <-ctx.Done():
                return
            case <-time.After(volumesInterval):
                volumes, err := f.runtimeVolumes(ctx)
                if err != nil {
                    continue
                }
                // Update watch list with new volumes
                watcher.Update(volumes)
            }
        }
    }()

    // Process file events
    for {
        select {
        case <-ctx.Done():
            return nil
        case event := <-watcher.Events():
            f.forwardEvent(event)
        }
    }
}
```

---

## 5. Volume Drivers Architecture

### 5.1 Driver Interface

```go
// Volume driver abstraction
type VolumeDriver interface {
    // Mount the volume
    Mount(source, target string, options map[string]string) error

    // Unmount the volume
    Unmount(target string) error

    // Check if mounted
    IsMounted(target string) bool

    // Get driver stats
    Stats() DriverStats
}
```

### 5.2 Driver Registration

```go
// From environment/volume.go
var drivers = map[string]VolumeDriver{}

func RegisterDriver(name string, driver VolumeDriver) {
    drivers[name] = driver
}

func GetDriver(name string) (VolumeDriver, error) {
    driver, ok := drivers[name]
    if !ok {
        return nil, fmt.Errorf("unknown volume driver: %s", name)
    }
    return driver, nil
}
```

### 5.3 Built-in Drivers

| Driver | Backend | Use Case |
|--------|---------|----------|
| **sshfs** | FUSE over SSH | QEMU fallback |
| **9p** | Plan 9 filesystem | QEMU with 9p support |
| **virtiofs** | Virtio-fs | Apple vz/krunkit |
| **nfs** | Network File System | Network shares |
| **local** | Bind mount | Local paths |

---

## 6. Performance Considerations

### 6.1 Performance Comparison

```
┌─────────────────────────────────────────────────────────┐
│           Mount Type Performance (MB/s)                 │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  sshfs     ████████████████░░░░░░░░░░░░░░░░░░  ~50     │
│  9p        ████████████████████████████░░░░░░  ~200    │
│  virtiofs  ████████████████████████████████████  ~1000 │
│                                                         │
│  Higher is better                                       │
└─────────────────────────────────────────────────────────┘
```

### 6.2 Latency Comparison

| Operation | sshfs | 9p | virtiofs |
|-----------|-------|----|----------|
| Open file | 5-10ms | 1-2ms | 0.1-0.5ms |
| Read 1KB | 1-2ms | 0.5-1ms | 0.1ms |
| Write 1KB | 2-5ms | 1-2ms | 0.2ms |
| Stat file | 2-3ms | 0.5ms | 0.1ms |

### 6.3 Optimization Strategies

**For Development (many small files):**
```yaml
# Use virtiofs with vz
vmType: vz
mountType: virtiofs

# Enable caching
mounts:
  - location: ~/projects
    writable: true
```

**For Data Processing (large files):**
```yaml
# Use 9p with larger msize
# Or virtiofs for best performance
vmType: qemu  # or vz
mountType: 9p  # or virtiofs

# Consider copying large datasets into VM
# instead of mounting for repeated access
```

**For Production:**
```yaml
# Copy data into container image or volumes
# Don't rely on host mounts for production
mounts: []  # No host mounts

# Use Docker volumes instead
# docker volume create mydata
```

### 6.4 Cache Configuration

```go
// 9p cache options
type NineP struct {
    Cache string  // "fscache", "loose", "fscache+loose", ""
}

// Cache modes:
// - "fscache": Use FS-Cache for read caching
// - "loose": Loose cache consistency (faster, may see stale data)
// - "fscache+loose": Both optimizations
// - "": No caching (safest, slowest)
```

---

## 7. Troubleshooting Mounts

### 7.1 Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Mount not visible | VM not started | `colima restart` |
| Permission denied | UID/GID mismatch | Use `--mount-type=virtiofs` |
| Slow performance | Using sshfs | Switch to `virtiofs` or `9p` |
| File events not working | inotify disabled | `--mount-inotify=true` |
| Mount point missing | Path doesn't exist | Create host directory first |

### 7.2 Debug Commands

```bash
# Check mount status
colima status

# SSH into VM and check mounts
colima ssh
ls -la /Users/  # Should show host files
mount | grep Users  # Show mount details

# Check inotify daemon
ps aux | grep colima  # Look for daemon process

# Test file events
# Terminal 1 (in container):
watch -n 0.1 'ls -la /app'

# Terminal 2 (on host):
touch /Users/dev/project/test.txt
```

### 7.3 Manual Mount Verification

```bash
# Verify Lima mounts
limactl shell colima-default mount

# Check VM mount points
colima ssh -- ls -la /Users/

# Test read/write
colima ssh -- touch /Users/dev/project/test-write
ls -la ~/project/test-write  # Should appear on host
```

---

## Summary

| Topic | Key Points |
|-------|------------|
| **Mount Types** | sshfs (slow, universal), 9p (moderate, QEMU), virtiofs (fast, vz) |
| **Configuration** | YAML config or CLI flags, default home mount |
| **Inotify** | Daemon forwards file events from host to VM |
| **Performance** | virtiofs > 9p > sshfs, use appropriate type for workload |
| **Troubleshooting** | Check VM status, verify paths, test permissions |

---

*Next: [Networking Deep Dive](04-networking-deep-dive.md)*
