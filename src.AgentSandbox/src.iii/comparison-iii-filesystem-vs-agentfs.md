# iii-filesystem vs AgentFS — Comprehensive Comparison

## Executive Summary

iii-filesystem and AgentFS are two radically different approaches to filesystem abstraction, solving different problems in different domains. iii-filesystem is a **VM guest filesystem** that exposes host directories to microVMs via virtio-fs. AgentFS is a **SQLite-backed virtual filesystem** designed for AI agent state management with auditability, reproducibility, and portability.

| Dimension | iii-filesystem | AgentFS |
|-----------|---------------|---------|
| **Purpose** | Expose host directory to guest VM | AI agent state management |
| **Storage backend** | Host filesystem (via syscalls) | SQLite database |
| **Mount mechanism** | virtio-fs (libkrun) | FUSE (Linux) / NFS (macOS) |
| **Total LOC** | 4,421 (Rust, 13 files) | 17,489 (Rust 17.4K + TS + Python, 3 crates) |
| **Primary consumer** | iii-init (PID 1 inside microVM) | AI agent SDKs (TS, Python, Rust) |
| **Threading** | Sync-only, Linux syscalls | Async (tokio), cross-platform |
| **Network** | None (local syscalls) | NFS v3 over loopback (macOS) |

## Architecture Comparison

### iii-filesystem Architecture

```
┌─────────────────────────────────────────┐
│ Guest VM (iii-init as PID 1)            │
│ ┌─────────────────────────────────────┐ │
│ │ PassthroughFs (sync, Linux-only)    │ │
│ │ ┌─────────┬───────────────────────┐ │ │
│ │ │ inodes: │ MultikeyBTreeMap      │ │ │
│ │ │         │ (FUSE inode ↔ host)   │ │ │
│ │ └─────────┴───────────────────────┘ │ │
│ │ ┌─────────┬───────────────────────┐ │ │
│ │ │ handles:│ DashMap (file fds)    │ │ │
│ │ └─────────┴───────────────────────┘ │ │
│ │                                     │ │
│ │ /bin ← bind mount → virtiofs:/bin  │ │
│ │ /etc ← bind mount → virtiofs:/etc  │ │
│ └─────────────────────────────────────┘ │
└──────────┬──────────────────────────────┘
           │ virtio-fs (shared memory)
           ▼
┌─────────────────────────────────────────┐
│ Host: libkrun NetWorker thread          │
│   rx_ring / tx_ring                     │
└─────────────────────────────────────────┘
```

### AgentFS Architecture

```
┌─────────────────────────────────────────┐
│ AI Agent (any language)                 │
│ ┌─────────────┬─────────────┬─────────┐ │
│ │ TS SDK      │ Python SDK  │Rust SDK │ │
│ │ agentfs.ts  │ agentfs.py  │ lib.rs  │ │
│ └──────┬──────┴──────┬──────┴────┬────┘ │
└───────┼──────────────┼───────────┼──────┘
        │              │           │
        ▼              ▼           ▼
┌─────────────────────────────────────────┐
│ agent.db (SQLite / Turso)               │
│ ┌─────────────────────────────────────┐ │
│ │ fs_inode  (metadata)                │ │
│ │ fs_data   (file content chunks)     │ │
│ │ fs_dentry (directory entries)       │ │
│ │ toolcalls (audit trail)             │ │
│ │ kv_store  (key-value state)         │ │
│ └─────────────────────────────────────┘ │
└──────────┬──────────────────────────────┘
           │ FUSE (Linux) / NFS (macOS)
           ▼
┌─────────────────────────────────────────┐
│ Host filesystem (mounted view)          │
│   /mnt/agentfs/...                      │
└─────────────────────────────────────────┘
```

## Design Philosophy

### iii-filesystem: Passthrough, Not Abstraction

iii-filesystem's core insight is **minimal abstraction**. It doesn't store files — it passes through host filesystem syscalls. The "filesystem" is a translation layer between FUSE operations (from the guest kernel) and Linux syscalls (on the host).

**Key principle**: The guest sees exactly what's on the host, with no virtualization of file contents or permissions. The only virtualization is:
1. Inode number mapping (synthetic FUSE inodes ↔ host identity)
2. The virtual `/init.krun` file
3. The root pivot (tmpfs root with bind mounts)

### AgentFS: SQLite as the Single Source of Truth

AgentFS's core insight is **maximal abstraction via SQLite**. Everything an agent does — every file operation, tool invocation, state change — lives in a single SQLite database. This enables:
1. **SQL queries** for agent history (`SELECT * FROM fs_dentry WHERE parent_ino = 5`)
2. **Single-file backup** (`cp agent.db snapshot.db`)
3. **Cross-machine portability** (move the .db file)
4. **ACID transactions** for file operations
5. **OverlayFS** — copy-on-write layers stacked on top of HostFS

## Detailed Feature Comparison

### Filesystem Implementation

| Feature | iii-filesystem | AgentFS |
|---------|---------------|---------|
| **Storage** | Host filesystem via syscalls | SQLite tables (fs_data, fs_inode, fs_dentry) |
| **File content** | Host disk blocks | 4KB chunks in SQLite BLOB column |
| **Inode mapping** | MultikeyBTreeMap (FUSE ↔ host ino/dev/mnt_id) | AUTOINCREMENT in SQLite |
| **Directory entries** | Host filesystem dirents | fs_dentry table (parent_ino, name, ino) |
| **Caching** | Kernel page cache (passthrough) | SQLite page cache + FUSE writeback cache |
| **Symlinks** | Host symlinks (followed via readlinkat) | Stored as symlink entries in fs_inode |
| **Permissions** | Host permissions (real fchmod/fchown) | Stored mode bits in fs_inode |

### Overlay / Layering

| Feature | iii-filesystem | AgentFS |
|---------|---------------|---------|
| **Overlay support** | No (passthrough only) | Yes (OverlayFS with base + delta) |
| **Base layer** | N/A | HostFS (real filesystem) or another AgentFS |
| **Delta layer** | N/A | AgentFS (SQLite writable) |
| **Copy-on-write** | N/A | Yes — first write copies from base to delta |
| **Stacking** | N/A | Yes — OverlayFS can nest: base=OverlayFS(base=..., delta=...) |
| **Whiteouts** | N/A | Yes — delta stores whiteout markers for deleted base files |

### Mount Mechanism

| Feature | iii-filesystem | AgentFS |
|---------|---------------|---------|
| **Primary mount** | virtio-fs (libkrun ring buffers) | FUSE (Linux) / NFS (macOS) |
| **Kernel extension** | No (virtio-fs is kernel-native) | No (FUSE is kernel-native, NFS is userspace) |
| **Cross-platform** | Linux guest only (VMs are Linux) | Linux + macOS |
| **Mount point** | Guest root `/` | Arbitrary host directory |
| **Performance** | Shared memory (zero-copy via rings) | FUSE syscalls / NFS over TCP loopback |

### Network

| Feature | iii-filesystem | AgentFS |
|---------|---------------|---------|
| **Network stack** | smoltcp (userspace TCP/IP) | NFS v3 (macOS only) |
| **DNS** | hickory-resolver hijack | N/A (host resolves) |
| **TCP proxy** | tokio tasks bridge guest ↔ host | N/A |
| **UDP relay** | Non-DNS UDP outside smoltcp | N/A |

### Threading Model

| Feature | iii-filesystem | AgentFS |
|---------|---------------|---------|
| **Runtime** | Sync-only (no tokio) | Async (tokio) |
| **Poll thread** | Dedicated OS thread (smoltcp) | N/A |
| **Proxy tasks** | tokio (TCP proxies) | N/A |
| **Cross-platform** | Linux-only (guest binary) | Linux + macOS |

### SDK / API

| Feature | iii-filesystem | AgentFS |
|---------|---------------|---------|
| **SDK languages** | None (kernel-facing only) | TypeScript, Python, Rust |
| **API style** | FUSE operations (kernel interface) | Async FileSystem trait + SDK methods |
| **File operations** | Via FUSE (open, read, write, etc.) | Via SDK (read, write, mkdir, etc.) |
| **Additional features** | N/A | KV store, toolcall audit trail |

## Key Architectural Differences

### 1. Where State Lives

**iii-filesystem**: State lives on the host filesystem. iii-filesystem doesn't own any data — it's a lens through which the guest VM sees the host. If the host directory changes, the guest sees the change immediately.

**AgentFS**: State lives in SQLite. The database IS the filesystem. Every file, every directory, every permission bit is a row in a table. The host filesystem only sees the SQLite file itself.

### 2. Mutability Model

**iii-filesystem**: Fully mutable — the guest can read and write host files directly. Changes persist on the host filesystem. No copy-on-write, no isolation.

**AgentFS**: Supports both mutable (direct writes to AgentFS) and isolated (OverlayFS with read-only base + writable delta). The overlay model enables sandboxed agents that can't modify the base project files.

### 3. Auditability

**iii-filesystem**: No audit trail. File operations go through the kernel and are not recorded.

**AgentFS**: Every file operation, tool invocation, and state change is recorded in SQLite. You can query: "What files did the agent create in the last hour?" or "Show me every tool call the agent made."

### 4. Performance Characteristics

**iii-filesystem**: High performance — shared memory rings with zero-copy I/O. The guest reads/writes host files through syscalls with minimal overhead. smoltcp adds some overhead for network traffic, but local filesystem ops are fast.

**AgentFS**: Moderate performance — SQLite introduces overhead for every file operation (SQL parsing, B-tree traversal, chunk assembly). FUSE adds syscall overhead on Linux. NFS on macOS adds TCP protocol overhead.

### 5. Use Case Fit

| Use Case | iii-filesystem | AgentFS |
|----------|---------------|---------|
| Run a worker process in a VM | ✅ Excellent | ❌ Not designed for this |
| Give AI agent persistent state | ❌ Not designed for this | ✅ Excellent |
| Snapshot agent state for replay | ❌ No mechanism | ✅ Single file copy |
| Audit agent behavior | ❌ No audit trail | ✅ Full SQL queryable history |
| Sandbox agent from host files | ⚠️ Via VM isolation | ✅ Via OverlayFS |
| High-throughput file I/O | ✅ Direct host access | ⚠️ SQLite overhead |
| Cross-platform mounting | ⚠️ Linux guest only | ✅ Linux + macOS |
| Network connectivity for VM | ✅ smoltcp TCP/IP | ❌ No network stack |

## Shared Concepts (Convergent Design)

Despite different goals, both systems converge on some patterns:

### 1. FUSE as Mount Mechanism
Both use FUSE to expose their filesystem to userspace — iii-filesystem via virtio-fs (kernel-facing) and AgentFS via `fuser` crate (host-facing).

### 2. Inode-to-Path Translation
Both need to translate between inodes and paths:
- iii-filesystem uses `MultikeyBTreeMap` (dual-key: FUSE inode ↔ host ino/dev/mnt_id)
- AgentFS uses SQLite queries on `fs_dentry` (parent_ino + name → ino)

### 3. Graceful Degradation
Both handle platform differences:
- iii-filesystem uses `#[cfg(target_os = "linux")]` guards extensively
- AgentFS uses FUSE on Linux and NFS on macOS

### 4. Writeback Caching
Both implement writeback caching:
- iii-filesystem delegates to kernel page cache (passthrough)
- AgentFS implements FUSE writeback cache with SQLite flush

## Complementarity

These systems are **complementary, not competitive**. A natural architecture would combine them:

```
┌─────────────────────────────────────────┐
│ Guest VM (iii-init as PID 1)            │
│ ┌─────────────────────────────────────┐ │
│ │ iii-filesystem (PassthroughFs)      │ │
│ │                                     │ │
│ │ /agent   ← bind mount → agentfs.db │ │
│ │ /project ← bind mount → host dir   │ │
│ └─────────────────────────────────────┘ │
└──────────┬──────────────────────────────┘
           │ virtio-fs
           ▼
┌─────────────────────────────────────────┐
│ Host: libkrun + agentfs mount           │
│   agent.db (SQLite) mounted at host dir │
└─────────────────────────────────────────┘
```

In this architecture:
- iii-filesystem provides the VM's root filesystem and network stack
- AgentFS provides the agent's state management, mounted as a directory inside the VM
- The VM gets both: isolated execution (iii-filesystem) + auditability (AgentFS)

## Summary Table

| Aspect | iii-filesystem | AgentFS |
|--------|---------------|---------|
| **Design goal** | VM guest filesystem | AI agent state management |
| **Storage** | Host filesystem | SQLite database |
| **Mount** | virtio-fs (kernel) | FUSE/NFS (userspace) |
| **Platform** | Linux guest only | Linux + macOS |
| **Threading** | Sync, dedicated poll thread | Async (tokio) |
| **Network** | smoltcp TCP/IP | None |
| **Audit** | None | Full SQL history |
| **Overlay** | No | Yes (copy-on-write) |
| **SDK** | None | TS, Python, Rust |
| **LOC** | 4,421 | 17,489+ (all languages) |
| **License** | Elastic-2.0 | MIT |
| **Maturity** | Production (iii workers) | Alpha |
