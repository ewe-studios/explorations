---
title: "Zero to Container Engineer: A First-Principles Journey Through Colima"
subtitle: "Complete textbook-style guide from container fundamentals to VM-based orchestration and Rust replication"
based_on: "Colima - Lima-based Container Runtime for macOS/Linux"
level: "Beginner to Intermediate - No prior container knowledge assumed"
---

# Zero to Container Engineer: First-Principles Guide

## Table of Contents

1. [What Are Containers?](#1-what-are-containers)
2. [Containers vs Virtual Machines](#2-containers-vs-virtual-machines)
3. [Linux Kernel Primitives](#3-linux-kernel-primitives)
4. [Container Runtime Fundamentals](#4-container-runtime-fundamentals)
5. [OCI Specifications](#5-oci-specifications)
6. [Why Colima Exists](#6-why-colima-exists)
7. [Your Learning Path](#7-your-learning-path)

---

## 1. What Are Containers?

### 1.1 The Fundamental Question

**What is a container?**

A container is an isolated environment that runs applications with their own:
1. **File system** - Separate directory structure
2. **Process space** - Cannot see other processes
3. **Network stack** - Own network interfaces and ports
4. **User IDs** - Separate user/group permissions
5. **Resource limits** - CPU, memory, I/O constraints

```
┌─────────────────────────────────────────────────────────┐
│                    Container A                           │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐          │
│  │   /app   │    │  PID 1   │    │  eth0:   │          │
│  │  (root)  │    │  (bash)  │    │  10.0.0.2│          │
│  └──────────┘    └──────────┘    └──────────┘          │
│       ^                                   |             │
│       └────────── Shared Kernel ──────────┘             │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────┐
│                    Container B                           │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐          │
│  │   /app   │    │  PID 1   │    │  eth0:   │          │
│  │  (root)  │    │  (nginx) │    │  10.0.0.3│          │
│  └──────────┘    └──────────┘    └──────────┘          │
└─────────────────────────────────────────────────────────┘
```

**Real-world analogy:** Apartment buildings

| Aspect | Apartment Building | Container System |
|--------|-------------------|------------------|
| Building | Single structure | Linux kernel |
| Apartments | Individual units | Containers |
| Walls | Separate living spaces | Namespaces |
| Utilities | Shared but metered | Cgroups (resource limits) |
| Address | Unique apartment number | Unique IP/port |
| Lease | Tenant agreement | Container image |

### 1.2 Why Containers Matter

**Before containers:** Deploy applications directly on servers
- Dependency conflicts (App A needs Python 2, App B needs Python 3)
- Inconsistent environments (works on dev, fails on prod)
- Wasted resources (one app per server)
- Slow deployment (hours to provision)

**With containers:** Deploy applications in isolated packages
- No conflicts (each container has its own dependencies)
- Consistent environments (same container everywhere)
- Efficient resource usage (many containers per server)
- Fast deployment (seconds to start)

### 1.3 Container Lifecycle

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│  Build  │ -> │  Ship   │ -> │  Start  │ -> │  Stop   │
│  Image  │    │  Image  │    │Container│    │Container│
└─────────┘    └─────────┘    └─────────┘    └─────────┘
     |              |              |              |
     v              v              v              v
  Dockerfile    Registry      docker run    docker stop
  or buildah    (Docker Hub)  or nerdctl    or kill
```

---

## 2. Containers vs Virtual Machines

### 2.1 The Key Differences

| Aspect | Virtual Machine | Container |
|--------|----------------|-----------|
| **Isolation** | Hardware-level | Process-level |
| **Guest OS** | Full OS kernel | Shared host kernel |
| **Size** | GBs (complete OS) | MBs (app + deps) |
| **Boot time** | Minutes | Milliseconds |
| **Performance** | ~5-10% overhead | ~1-2% overhead |
| **Security** | Strong isolation | Good isolation |
| **Use case** | Different OS/kernel | Same kernel, different apps |

### 2.2 Visual Comparison

**Virtual Machine:**
```
┌────────────────────────────────────────┐
│              Hypervisor                │
│  (QEMU, VMware, Hyper-V, VirtualBox)   │
├──────────┬────────────┬────────────────┤
│    VM    │     VM     │      VM        │
│  ┌────┐  │  ┌────┐    │  ┌────┐        │
│  │Guest│  │  │Guest│  │  │Guest│       │
│  │ OS  │  │  │ OS  │  │  │ OS  │       │
│  ├────┤  │  ├────┤  │  ├────┤         │
│  │ App│  │  │ App │  │  │ App │        │
│  │ +  │  │  │ +   │  │  │ +   │        │
│  │Libs│  │  │Libs│  │  │Libs│         │
│  └────┘  │  └────┘  │  └────┘         │
└──────────┴──────────┴─────────────────┘
           Host Operating System
                Hardware
```

**Container:**
```
┌─────────┬──────────┬──────────┬─────────┐
│Container│ Container│ Container│Container│
│  ┌───┐  │  ┌───┐   │  ┌───┐   │  ┌───┐  │
│  │App│  │  │App│   │  │App│   │  │App│  │
│  │ + │  │  │ + │   │  │ + │   │  │ + │  │
│  │Libs│  │  │Libs│  │  │Libs│  │  │Libs│ │
│  └───┘  │  └───┘   │  └───┘   │  └───┘  │
└─────────┴──────────┴──────────┴─────────┘
        Container Runtime (Docker, containerd)
              Host Operating System Kernel
                    Hardware
```

### 2.3 Why Colima Uses Both

Colima runs containers **inside** a VM on macOS because:

1. **macOS uses XNU kernel**, not Linux kernel
2. **Containers require Linux kernel features** (namespaces, cgroups)
3. **Solution:** Run a Linux VM, then run containers inside it

```
┌─────────────────────────────────────────┐
│              macOS (Host)               │
│  ┌─────────────────────────────────┐    │
│  │         Lima VM (Linux)         │    │
│  │  ┌──────┐  ┌──────┐  ┌──────┐   │    │
│  │  │Docker│  │nerdctl│ │ k3s  │   │    │
│  │  │Container│Container│Pod    │   │    │
│  │  └──────┘  └──────┘  └──────┘   │    │
│  │       ^           ^         ^    │    │
│  │       └─── Linux Kernel ───┘    │    │
│  └─────────────────────────────────┘    │
│         QEMU or Virtualization.Framework│
└─────────────────────────────────────────┘
```

---

## 3. Linux Kernel Primitives

### 3.1 Namespaces - Process Isolation

Namespaces partition kernel resources so processes in different namespaces cannot see or affect each other.

**Types of namespaces:**

| Namespace | Flag | Isolates | Example |
|-----------|------|----------|---------|
| Mount (mnt) | CLONE_NEWNS | File system mounts | `/app` in container A, `/data` in container B |
| Process ID (pid) | CLONE_NEWPID | Process IDs | PID 1 in container != PID 1 on host |
| Network (net) | CLONE_NEWNET | Network interfaces | Separate eth0, routing tables |
| User (user) | CLONE_NEWUSER | User/group IDs | root in container != root on host |
| UTS | CLONE_NEWUTS | Hostname/domain | container-a vs host |
| IPC | CLONE_NEWIPC | Shared memory, semaphores | Separate IPC channels |
| Cgroup | CLONE_NEWCGROUP | Cgroup root directory | Different resource limits |

**Example: Creating namespaces manually**

```bash
# Run a command in new namespaces (requires root)
sudo unshare --mount --pid --net --fork /bin/bash

# Inside the new namespace:
ps aux          # Shows only processes in this namespace
mount /tmp      # Mount is isolated to this namespace
hostname mycontainer  # Hostname change is isolated
```

### 3.2 Cgroups - Resource Limits

Control groups (cgroups) limit and account for resource usage (CPU, memory, I/O).

**Cgroup v2 hierarchy:**

```
/sys/fs/cgroup/
├── user.slice/           # User sessions
│   └── user-1000.slice/
├── system.slice/         # System services
│   ├── docker.service/
│   └── ssh.service/
└── container.slice/      # Container workloads
    ├── container-abc123/
    │   ├── cpu.max       # CPU limit
    │   ├── memory.max    # Memory limit
    │   └── io.max        # I/O limit
    └── container-def456/
```

**Setting cgroup limits:**

```bash
# Limit container to 2 CPUs and 4GB memory
echo "200000 1000000" > /sys/fs/cgroup/container-abc123/cpu.max
echo "4294967296" > /sys/fs/cgroup/container-abc123/memory.max

# Read current usage
cat /sys/fs/cgroup/container-abc123/cpu.stat
cat /sys/fs/cgroup/container-abc123/memory.current
```

### 3.3 Union Filesystems - Layered Images

Container images use layered filesystems (overlay2, aufs, btrfs).

**Image layering:**

```
┌─────────────────────────┐
│    Read-Write Layer     │  <- Container changes
├─────────────────────────┤
│    Read-Only Layer 3    │  <- Application
├─────────────────────────┤
│    Read-Only Layer 2    │  <- Libraries
├─────────────────────────┤
│    Read-Only Layer 1    │  <- Base OS
├─────────────────────────┤
│         Base Image      │
└─────────────────────────┘
```

**Example: Building a layered image**

```dockerfile
# Each instruction creates a new layer
FROM ubuntu:22.04         # Layer 1: Base OS (72MB)
RUN apt-get update        # Layer 2: Package cache
RUN apt-get install -y python3  # Layer 3: Python
COPY app.py /app/         # Layer 4: Application
CMD ["python3", "/app/app.py"]  # Metadata (not a layer)
```

---

## 4. Container Runtime Fundamentals

### 4.1 Runtime Components

A container runtime consists of:

1. **Low-level runtime** (runc, crun, youki)
   - Implements OCI runtime specification
   - Creates/destroys containers using kernel primitives

2. **High-level runtime** (containerd, CRI-O, Podman)
   - Manages image transfer, storage
   - Handles container lifecycle
   - Exposes APIs to clients

3. **Client** (Docker CLI, nerdctl, kubectl)
   - User interface
   - Communicates with high-level runtime

```
┌────────────────────────────────────────┐
│  Docker CLI    nerdctl    kubectl      │  <- Clients
└───────┬───────────┬──────────┬─────────┘
        │           │          │
└───────┴───────────┴──────────┴─────────┐
│           containerd (daemon)           │  <- High-level
│  ┌──────┐  ┌────────┐  ┌──────────┐    │     runtime
│  │Image │  │Container│ │ Execution│    │
│  │Store │  │Manager │  │  (runc)  │    │
│  └──────┘  └────────┘  └──────────┘    │
└─────────────────────────────────────────┘
                  │
┌─────────────────┴──────────────────────┐
│              Linux Kernel              │  <- Low-level
│  Namespaces | Cgroups | Seccomp | SELinux│     runtime
└─────────────────────────────────────────┘
```

### 4.2 Container Creation Flow

**Step-by-step process:**

```
1. User runs: docker run nginx
                  │
                  v
2. Docker CLI sends POST /containers/create to containerd
                  │
                  v
3. containerd pulls image if not present
                  │
                  v
4. containerd creates container metadata
                  │
                  v
5. containerd calls runc create
                  │
                  v
6. runc:
   - Creates namespaces (clone/unshare)
   - Sets up cgroups
   - Configures root filesystem (overlay2)
   - Sets up network interfaces
                  │
                  v
7. runc execs the container process (becomes PID 1)
                  │
                  v
8. container returns "running" status
```

### 4.3 Colima's Runtime Architecture

Colima uses a layered approach:

```go
// environment.Container interface (from environment.go)
type Container interface {
    Provision(ctx context.Context) error    // Setup runtime
    Start(ctx context.Context) error        // Start daemon
    Stop(ctx context.Context, force bool) error  // Stop daemon
    Running(ctx context.Context) bool       // Check status
    Teardown(ctx context.Context) error     // Cleanup
    Version(ctx context.Context) string     // Runtime version
}

// Implemented by:
// - dockerRuntime (Docker Engine)
// - containerdRuntime (containerd + nerdctl)
// - incusRuntime (Incus containers/VMs)
// - kubernetesRuntime (k3s)
```

---

## 5. OCI Specifications

### 5.1 What is OCI?

Open Container Initiative (OCI) standardizes container formats and runtimes.

**Three specifications:**

1. **Image Specification** - Format for container images
2. **Runtime Specification** - How to run a container
3. **Distribution Specification** - How to push/pull images

### 5.2 OCI Image Structure

```
container-image/
├── blobs/
│   └── sha256/
│       ├── <layer-1-hash>    # Compressed tar of files
│       ├── <layer-2-hash>    # Compressed tar of files
│       └── <config-hash>     # Container config JSON
└── index.json                # Image manifest list
```

**Config JSON example:**

```json
{
  "architecture": "amd64",
  "os": "linux",
  "config": {
    "Cmd": ["/bin/bash"],
    "Env": ["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],
    "WorkingDir": "/"
  },
  "rootfs": {
    "type": "layers",
    "diff_ids": [
      "sha256:<layer-1>",
      "sha256:<layer-2>"
    ]
  }
}
```

### 5.3 OCI Runtime Bundle

The OCI runtime bundle is what runc actually executes:

```
bundle/
├── config.json    # Runtime specification
└── rootfs/        # Extracted filesystem
    ├── bin/
    ├── etc/
    └── usr/
```

**config.json excerpt:**

```json
{
  "ociVersion": "1.0.2",
  "process": {
    "terminal": true,
    "user": {"uid": 0, "gid": 0},
    "args": ["/bin/bash"],
    "env": ["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],
    "cwd": "/"
  },
  "root": {
    "path": "rootfs",
    "readonly": true
  },
  "linux": {
    "namespaces": [
      {"type": "pid"},
      {"type": "network"},
      {"type": "mount"},
      {"type": "uts"},
      {"type": "user"}
    ],
    "cgroupsPath": "/myContainer",
    "resources": {
      "memory": {"limit": 536870912},
      "cpu": {"quota": 50000, "period": 100000}
    }
  }
}
```

---

## 6. Why Colima Exists

### 6.1 The macOS Problem

macOS cannot run Linux containers natively because:

1. **Different kernel:** macOS uses XNU, containers need Linux
2. **Different syscall interface:** Linux syscalls != macOS syscalls
3. **Different filesystem:** APFS doesn't support overlay filesystems

### 6.2 Historical Solutions

| Solution | Era | Approach | Issues |
|----------|-----|----------|--------|
| Docker Toolbox (~2015) | VirtualBox VM | Slow, clunky UX | Poor performance |
| Docker Desktop (2018+) | HyperKit (Linux) | Better UX | Proprietary, resource-heavy |
| Colima (2021+) | Lima (QEMU/vz) | Open source, flexible | Requires setup |

### 6.3 Colima's Innovation

Colima provides:

1. **Open source alternative** to Docker Desktop
2. **Multiple VM backends:** QEMU, Apple Virtualization Framework, Krunkit
3. **Multiple runtimes:** Docker, containerd, Incus, Kubernetes
4. **Profiles:** Multiple isolated instances
5. **Configurable resources:** CPU, memory, disk customization

```yaml
# Colima config ( ~/.colima/default/colima.yaml )
cpu: 4
memory: 8
disk: 100
runtime: docker
kubernetes:
  enabled: true
  version: v1.28.0
mounts:
  - location: ~/projects
    writable: true
network:
  address: true
```

---

## 7. Your Learning Path

### 7.1 Recommended Progression

**Week 1-2: Container Fundamentals**
1. Complete this document
2. Read OCI specifications
3. Experiment with `unshare` and namespaces
4. Build and run simple containers

**Week 3-4: Container Runtimes**
1. Study Docker architecture
2. Explore containerd and nerdctl
3. Understand image building
4. Learn Docker Compose

**Week 5-6: Virtualization**
1. Understand QEMU basics
2. Study Lima VM management
3. Explore Apple Virtualization Framework
4. Compare VM approaches

**Week 7-8: Colima Deep Dive**
1. Read [VM Management](01-vm-management-deep-dive.md)
2. Read [Runtime Integration](02-runtime-integration-deep-dive.md)
3. Configure custom instances
4. Debug common issues

**Week 9-10: Rust Translation**
1. Read [Rust Revision](rust-revision.md)
2. Understand TaskIterator pattern
3. Implement basic container operations
4. Build minimal container runtime

### 7.2 Hands-On Exercises

**Exercise 1: Manual Container**
```bash
# Create a simple "container" using namespaces
sudo unshare --mount --pid --net --fork /bin/bash

# Inside:
mount -t proc proc /proc
ip addr add 10.0.0.2/24 dev lo
hostname mycontainer
ps aux  # Should show only bash
```

**Exercise 2: Docker without Docker**
```bash
# Pull an image manually
skopeo copy docker://alpine:latest oci:alpine

# Extract the rootfs
umoci unpack --image alpine alpine-bundle

# Run with runc
cd alpine-bundle
sudo runc run mycontainer
```

**Exercise 3: Colima Instance**
```bash
# Create a custom Colima profile
colima start mydev --cpu 4 --memory 8 --disk 50

# Verify
colima status mydev
docker context use mydev
docker run hello-world
```

### 7.3 Troubleshooting Skills

**Common issues and solutions:**

| Issue | Cause | Solution |
|-------|-------|----------|
| Container won't start | Port conflict | Check `docker ps`, use different port |
| Slow volume mounts | 9p overhead | Use virtiofs (vz VM type) |
| Network unreachable | vmnet not running | `colima stop && colima start` |
| Out of memory | Cgroup limits | Increase `--memory` on start |
| Permission denied | User namespace | Run as root or fix mount permissions |

---

## Appendix: Container Terminology

| Term | Definition |
|------|------------|
| **Image** | Read-only template used to create containers |
| **Container** | Runnable instance of an image |
| **Registry** | Service for storing and distributing images |
| **Dockerfile** | Text file with instructions to build an image |
| **Volume** | Persistent storage for containers |
| **Network** | Virtual network for container communication |
| **Compose** | Tool for defining multi-container applications |
| **Namespace** | Linux kernel feature for isolation |
| **Cgroup** | Linux kernel feature for resource limits |
| **OCI** | Open Container Initiative (standards body) |

---

*This document is a living textbook. Concepts become clearer with hands-on practice.*
