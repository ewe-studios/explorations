---
title: Architecture — FUSE Protocol, VM Integration, Component Relationships
---

# Architecture — FUSE Protocol, VM Integration, Component Relationships

**iii-filesystem sits between the virtio-fs protocol and the host filesystem, translating guest FUSE operations into host syscalls.** This document covers the full architecture, from the guest kernel's FUSE requests through the VMM to the PassthroughFs backend.

## FUSE Protocol Flow

```mermaid
sequenceDiagram
    participant App as Guest Application
    participant GKernel as Guest Kernel
    participant VMM as VMM (libkrun)
    participant Passthrough as PassthroughFs
    participant Inodes as Inode Table
    participant HostFS as Host Filesystem

    App->>GKernel: open("/file.txt")
    GKernel->>VMM: FUSE_LOOKUP(parent=1, name="file.txt")
    VMM->>Passthrough: lookup(ctx, parent=1, name="file.txt")

    Passthrough->>Passthrough: validate_name("file.txt")
    Passthrough->>Passthrough: get_inode_fd(parent=1)
    Passthrough->>Passthrough: open_beneath(parent_fd, "file.txt", O_PATH)
    Passthrough->>Passthrough: statx(fd, AT_EMPTY_PATH)
    Passthrough->>Inodes: check host identity (ino+dev+mnt_id)

    alt Inode already tracked
        Inodes-->>Passthrough: bump refcount, return entry
    else New inode
        Passthrough->>Inodes: allocate FUSE inode, insert
    end

    Passthrough-->>VMM: Entry {inode=3, attr=stat64}
    VMM-->>GKernel: FUSE_LOOKUP response
    GKernel->>App: fd for /file.txt

    App->>GKernel: read(fd, buf, size)
    GKernel->>VMM: FUSE_READ(inode=3, handle=0, offset=0, size=4096)
    VMM->>Passthrough: read(ctx, ino=3, handle=0, writer, size=4096, offset=0)
    Passthrough->>HostFS: preadv64(fd, offset=0)
    HostFS-->>Passthrough: data bytes
    Passthrough-->>VMM: write_to(writer, data)
    VMM-->>GKernel: FUSE_READ response
    GKernel-->>App: data in buf
```

## Component Architecture

```mermaid
graph TB
    subgraph LibExports["Public API (lib.rs)"]
        E1["DynFileSystem trait"]
        E2["PassthroughFs"]
        E3["PassthroughConfig"]
        E4["CachePolicy"]
    end

    subgraph Core["PassthroughFs Core"]
        C1["root_fd: File"]
        C2["inodes: RwLock<MultikeyBTreeMap>"]
        C3["handles: DashMap<u64, HandleData>"]
        C4["init_file: File (memfd)"]
        C5["next_inode: AtomicU64"]
        C6["next_handle: AtomicU64"]
    end

    subgraph Operations["FUSE Operations"]
        O1["lookup/forget"]
        O2["open/read/write/release"]
        O3["opendir/readdir/readdirplus"]
        O4["mkdir/create/symlink/link"]
        O5["unlink/rmdir/rename"]
        O6["getattr/setattr/access"]
        O7["fsync/statfs"]
    end

    subgraph Shared["Shared Infrastructure"]
        S1["inode_table.rs<br/>MultikeyBTreeMap, InodeData"]
        S2["init_binary.rs<br/>Virtual /init.krun"]
        S3["platform.rs<br/>Error translation, stat helpers"]
        S4["name_validation.rs<br/>Path traversal protection"]
    end

    E1 --> Core
    Core --> Operations
    Operations --> Shared
```

## How It Integrates with iii-worker

The `iii-worker` crate (42,998 LOC) manages krun-based VMs for sandboxed workers. Each VM needs a filesystem backend to access host files. `iii-filesystem` provides that backend:

```mermaid
flowchart LR
    subgraph IIIWorker["iii-worker (42,998 LOC)"]
        W1["VM Manager"]
        W2["krun VM Instance"]
    end

    subgraph IIIFilesystem["iii-filesystem (4,421 LOC)"]
        F1["PassthroughFs"]
        F2["PassthroughFsBuilder"]
    end

    subgraph Krun["libkrun"]
        K1["virtio-fs server"]
        K2["DynFileSystem trait"]
    end

    W1 -->|creates VM with| W2
    W2 -->|uses| F1
    F1 -->|implements| K2
    K2 -->|served by| K1
    K1 -->|FUSE protocol| W2
```

## Inode Numbering

The FUSE protocol uses synthetic inode numbers that iii-filesystem manages independently from the host:

| Inode | Purpose | Notes |
|-------|---------|-------|
| 1 | Root directory | Always the configured `root_dir` |
| 2 | `/init.krun` | Virtual file with embedded init binary (only when `embed-init` feature enabled) |
| 3+ | Real files/dirs | Monotonically allocated via `next_inode` |

**Aha:** When init is NOT embedded (no `embed-init` feature or binary unavailable), inode 2 and handle 0 are available for real files. The code uses a conditional `start_inode`/`start_handle` to avoid wasting synthetic inode numbers:

Source: `backends/passthroughfs/mod.rs:182-186`
```rust
let (start_inode, start_handle) = if init_binary::has_init() {
    (3u64, 1u64)
} else {
    (2u64, 0u64)
};
```

Since `has_init()` is a const fn checking `INIT_BYTES.len()`, the compiler optimizes away the dead branch entirely.

## What's Next

- [02 — PassthroughFs](02-passthrough-fs.md) — The core struct, configuration, builder, and lifecycle
- [03 — Inode Management](03-inode-management.md) — Dual-key lookup, lookup collapse, reference counting
