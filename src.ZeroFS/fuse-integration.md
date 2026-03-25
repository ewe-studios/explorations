# FUSE Integration: Filesystem in Userspace

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.ZeroFS/fuser/`

---

## Table of Contents

1. [Introduction to FUSE](#introduction-to-fuse)
2. [FUSE Architecture](#fuse-architecture)
3. [The fuser Library](#the-fuser-library)
4. [FUSE ABI and Protocol](#fuse-abi-and-protocol)
5. [Mounting and Session Management](#mounting-and-session-management)
6. [Implementing a FUSE Filesystem](#implementing-a-fuse-filesystem)
7. [Performance Considerations](#performance-considerations)
8. [Code Examples](#code-examples)

---

## Introduction to FUSE

### What is FUSE?

**FUSE (Filesystem in Userspace)** allows developers to implement filesystems entirely in userspace without kernel module development.

```
Traditional Filesystem Development:
┌─────────────────────────────────────┐
│  Kernel Space                       │
│  ┌─────────────────────────────┐    │
│  │  Filesystem Module (.ko)    │    │
│  │  - Requires kernel headers  │    │
│  │  - Risk of kernel panic     │    │
│  │  - Complex debugging        │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘

FUSE Development:
┌─────────────────────────────────────┐
│  User Space                         │
│  ┌─────────────────────────────┐    │
│  │  Your Filesystem (Rust!)    │    │
│  │  - Safe                     │    │
│  │  - Easy debugging           │    │
│  │  - No kernel dependencies   │    │
│  └─────────────────────────────┘    │
│              │                      │
│  Kernel      │ /dev/fuse            │
│  ┌───────────▼─────────────────┐    │
│  │  FUSE Kernel Module         │    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

### Why FUSE?

| Advantage | Description |
|-----------|-------------|
| **Safety** | Crash doesn't panic kernel |
| **Productivity** | Standard debugging tools |
| **Portability** | Works across kernel versions |
| **Flexibility** | Network, encrypted, virtual filesystems |
| **No Root Required** | Mount with user permissions |

### Use Cases

- **Network filesystems**: SSHFS, NFS userspace clients
- **Encrypted filesystems**: eCryptfs, gocryptfs
- **Cloud storage**: s3fs, rclone mount
- **Virtual filesystems**: /proc, /sys alternatives
- **Compression**: Compressed overlay filesystems
- **ZeroFS**: S3-backed distributed filesystem

---

## FUSE Architecture

### Three Components

```
┌─────────────────────────────────────────┐
│  1. Kernel Driver                       │
│  - Registers as filesystem              │
│  - Forwards operations to /dev/fuse     │
│  - Manages page cache                   │
└─────────────────┬───────────────────────┘
                  │ /dev/fuse
┌─────────────────▼───────────────────────┐
│  2. Userspace Library (libfuse/fuser)  │
│  - Establishes communication            │
│  - Marshals requests/responses          │
│  - Handles mount/unmount                │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  3. Filesystem Implementation           │
│  - Your code!                           │
│  - Implements Filesystem trait          │
│  - Processes operations                 │
└─────────────────────────────────────────┘
```

### Request/Response Flow

```
1. Application calls open("/mnt/fuse/file.txt")
2. Kernel VFS receives call
3. FUSE kernel module creates request
4. Request written to /dev/fuse
5. Userspace library reads request
6. Filesystem implementation processes
7. Response written back through /dev/fuse
8. Kernel receives response
9. Application gets result

Total: 2 context switches (user→kernel→user)
```

### FUSE Versions

| Version | Kernel | Features |
|---------|--------|----------|
| **FUSE 7.8** | 2.6.16 | Basic operations |
| **FUSE 7.19** | 3.0 | Big writes, ioctl |
| **FUSE 7.23** | 3.14 | Parallel dir operations |
| **FUSE 7.28** | 4.20 | Parallel reads, expiry |
| **FUSE 7.31** | 5.1 | Atomic open+create |
| **FUSE 7.40** | 6.0 | Passthrough, security |

**fuser supports FUSE 7.8 through 7.40+**

---

## The fuser Library

### Overview

**fuser** is a Rust rewrite of libfuse:

```
fuser Features:
- Full FUSE ABI support (7.8 - 7.40)
- Pure Rust implementation
- Optional libfuse dependency (Linux can work without)
- Safe Rust API
- Async-friendly design
```

### Installation

```toml
[dependencies]
fuser = "0.15"
```

**System dependencies (Linux):**
```bash
# Debian/Ubuntu
apt-get install fuse3 libfuse3-dev pkg-config

# CentOS/RHEL
yum install fuse-devel pkgconfig

# Arch
pacman -S fuse3
```

### Building Without libfuse

```toml
[dependencies.fuser]
version = "0.15"
default-features = false
features = ["fuse2", "libfuse"]  # Or just "fuse2" for pure Rust
```

---

## FUSE ABI and Protocol

### FUSE Operations

```rust
// Core filesystem operations
trait Filesystem {
    // Initialization
    fn init(&mut self, req: &Request) -> Result<(), Error>;
    fn destroy(&mut self, req: &Request);

    // Lookup and attributes
    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry);
    fn getattr(&mut self, req: &Request, ino: u64, reply: ReplyAttr);
    fn setattr(&mut self, req: &Request, ino: u64, attr: FileAttr, valid: SetattrValid, reply: ReplyAttr);

    // File operations
    fn open(&mut self, req: &Request, ino: u64, flags: i32, reply: ReplyOpen);
    fn read(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, size: u32, reply: ReplyData);
    fn write(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, data: &[u8], flags: u32, reply: ReplyWrite);
    fn flush(&mut self, req: &Request, ino: u64, fh: u64, lock_owner: u64, reply: ReplyEmpty);
    fn release(&mut self, req: &Request, ino: u64, fh: u64, flags: i32, lock_owner: u64, flush: bool, reply: ReplyEmpty);
    fn fsync(&mut self, req: &Request, ino: u64, fh: i64, datasync: bool, reply: ReplyEmpty);

    // Directory operations
    fn readdir(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, reply: ReplyDirectory);
    fn mkdir(&mut self, req: &Request, parent: u64, name: &OsStr, mode: u32, reply: ReplyEntry);
    fn rmdir(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty);
    fn unlink(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty);

    // Link operations
    fn link(&mut self, req: &Request, ino: u64, newparent: u64, newname: &OsStr, reply: ReplyEntry);
    fn symlink(&mut self, req: &Request, linkname: &OsStr, parent: u64, name: &OsStr, reply: ReplyEntry);
    fn readlink(&mut self, req: &Request, ino: u64, reply: ReplyData);

    // Extended attributes
    fn getxattr(&mut self, req: &Request, ino: u64, name: &OsStr, size: u32, reply: ReplyXattr);
    fn setxattr(&mut self, req: &Request, ino: u64, name: &OsStr, value: &[u8], flags: u32, reply: ReplyEmpty);
    fn listxattr(&mut self, req: &Request, ino: u64, size: u32, reply: ReplyXattr);
    fn removexattr(&mut self, req: &Request, ino: u64, name: &OsStr, reply: ReplyEmpty);

    // Advanced (FUSE 7.19+)
    #[cfg(feature = "abi-7-19")]
    fn ioctl(&mut self, req: &Request, ino: u64, fh: u64, flags: u32, cmd: u32, in_data: &[u8], out_size: u32, reply: ReplyIoctl);

    // Advanced (FUSE 7.21+)
    #[cfg(feature = "abi-7-21")]
    fn fallocate(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, length: i64, mode: i32, reply: ReplyEmpty);
}
```

### File Attributes

```rust
#[derive(Debug, Clone, Copy)]
pub struct FileAttr {
    pub ino: u64,         // Inode number
    pub size: u64,        // Size in bytes
    pub blocks: u64,      // Size in blocks
    pub atime: SystemTime, // Last access
    pub mtime: SystemTime, // Last modification
    pub ctime: SystemTime, // Last change
    pub crtime: SystemTime, // Creation (macOS)
    pub kind: FileType,    // File type
    pub perm: u16,         // Permissions
    pub nlink: u32,        // Hard links
    pub uid: u32,          // User ID
    pub gid: u32,          // Group ID
    pub rdev: u32,         // Device ID
    pub blksize: u32,      // Block size
    pub flags: u32,        // Flags (macOS)
}

#[derive(Debug, Clone, Copy)]
pub enum FileType {
    NamedPipe,
    CharDevice,
    BlockDevice,
    Directory,
    RegularFile,
    Symlink,
    Socket,
}
```

### Reply Types

```rust
// Each operation has a specific reply type
trait Reply {
    fn error(self, err: i32);
}

struct ReplyEntry { /* lookup result */ }
struct ReplyAttr { /* file attributes */ }
struct ReplyData { /* binary data */ }
struct ReplyDirectory { /* directory entries */ }
struct ReplyOpen { /* file handle */ }
struct ReplyWrite { /* bytes written */ }
struct ReplyEmpty { /* success/failure */ }
// ... and more
```

---

## Mounting and Session Management

### Mounting Process

```rust
use fuser::{Filesystem, Session, BackgroundSession};
use std::path::Path;

fn mount_filesystem() {
    let fs = MyFilesystem::new();
    let mountpoint = Path::new("/mnt/myfs");

    // Option 1: Blocking session
    let mut session = Session::new(fs, mountpoint, &[]).unwrap();
    session.run().unwrap();  // Blocks until unmounted

    // Option 2: Background session
    let fs = MyFilesystem::new();
    let session = BackgroundSession::new(fs, mountpoint, &[]).unwrap();
    // Session runs in background thread
    // Drop session to unmount

    // Option 3: With mount options
    let options = [
        "-o", "fsname=myfs",
        "-o", "subtype=myfs",
        "-o", "allow_other",
        "-o", "default_permissions",
    ];
    let session = BackgroundSession::new(fs, mountpoint, &options).unwrap();
}
```

### Mount Options

```rust
// Common FUSE mount options
let options = vec![
    // Filesystem identity
    "fsname=myfs",      // Name in mount table
    "subtype=myfs",     // Subtype
    "volname=MyVol",    // Volume name (macOS)

    // Permissions
    "allow_other",      // Allow other users to access
    "allow_root",       // Allow root to access
    "default_permissions",  // Kernel permission checking
    "umask=022",        // Permission mask
    "uid=1000",         // Owner UID
    "gid=1000",         // Owner GID

    // Performance
    "async_read",       // Async reads enabled
    "sync_read",        // Sync reads only
    "max_read=131072",  // Max read size
    "attr_timeout=1.0", // Attribute cache timeout
    "entry_timeout=1.0", // Entry cache timeout

    // Behavior
    "auto_unmount",     // Auto unmount on exit
    "nonempty",         // Allow mounting on non-empty dir
    "hard_remove",      // Immediate unlink (no rename)
];
```

### Unmounting

```rust
// Graceful unmount
impl Drop for BackgroundSession {
    fn drop(&mut self) {
        // Automatically unmounts
    }
}

// Manual unmount
use fuser::session_unmount;
session_unmount(mountpoint).unwrap();
```

---

## Implementing a FUSE Filesystem

### Minimal Example

```rust
use fuser::{Filesystem, FileAttr, FileType, Filesystem as FuseFS, ReplyEmpty, ReplyEntry, ReplyAttr, ReplyData, ReplyDirectory, Request};
use std::ffi::OsStr;
use std::time::{SystemTime, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1);

struct HelloFilesystem {
    root_attr: FileAttr,
}

impl HelloFilesystem {
    fn new() -> Self {
        let now = SystemTime::now();
        Self {
            root_attr: FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 2,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                blksize: 512,
                flags: 0,
            },
        }
    }
}

impl Filesystem for HelloFilesystem {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 && name == "hello.txt" {
            reply.entry(&TTL, &self.get_hello_attr(), 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, self.root_attr),
            2 => reply.attr(&TTL, self.get_hello_attr()),
            _ => reply.error(libc::ENOENT),
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino != 1 {
            reply.error(libc::ENOTDIR);
            return;
        }

        if offset == 0 {
            reply.add(1, 1, FileType::Directory, ".");
            reply.add(1, 1, FileType::Directory, "..");
            reply.add(2, 2, FileType::RegularFile, "hello.txt");
        }

        reply.ok();
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        if ino != 2 {
            reply.error(libc::ENOENT);
            return;
        }

        let data = b"Hello, FUSE!";
        let end = (offset as usize + size as usize).min(data.len());
        let start = offset as usize;

        if start < data.len() {
            reply.data(&data[start..end]);
        } else {
            reply.data(&[]);
        }
    }
}

fn main() {
    let fs = HelloFilesystem::new();
    fuser::mount2(fs, "/mnt/hello", &[
        MountOption::FSName("hello".to_string()),
        MountOption::AutoUnmount,
    ]);
}
```

### Inode Management

```rust
struct InodeManager {
    next_ino: AtomicU64,
    inodes: DashMap<u64, InodeData>,
}

struct InodeData {
    parent: u64,
    name: String,
    kind: FileType,
    attr: FileAttr,
    // ... additional data
}

impl InodeManager {
    fn new() -> Self {
        let mut inodes = DashMap::new();
        inodes.insert(1, InodeData::root());
        Self {
            next_ino: AtomicU64::new(2),
            inodes,
        }
    }

    fn allocate_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::Relaxed)
    }

    fn lookup(&self, parent: u64, name: &str) -> Option<u64> {
        for entry in self.inodes.iter() {
            if entry.parent == parent && entry.name == name {
                return Some(*entry.key());
            }
        }
        None
    }
}
```

### Directory Operations

```rust
impl Filesystem for MyFilesystem {
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, reply: ReplyEntry) {
        let parent_data = match self.inodes.get(&parent) {
            Some(data) => data.clone(),
            None => { reply.error(libc::ENOENT); return; }
        };

        if parent_data.kind != FileType::Directory {
            reply.error(libc::ENOTDIR);
            return;
        }

        let ino = self.inode_mgr.allocate_ino();
        let now = SystemTime::now();

        let attr = FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: FileType::Directory,
            perm: mode as u16,
            nlink: 2,  // . and ..
            uid: parent_data.attr.uid,
            gid: parent_data.attr.gid,
            rdev: 0,
            blksize: 512,
            flags: 0,
        };

        self.inodes.insert(ino, InodeData {
            parent,
            name: name.to_string_lossy().into_owned(),
            kind: FileType::Directory,
            attr,
        });

        reply.entry(&TTL, &attr, 0);
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let entries: Vec<_> = self.inodes.iter()
            .filter(|e| e.parent == ino)
            .collect();

        if offset == 0 {
            // Add . and ..
            reply.add(ino, 1, FileType::Directory, ".");
            let parent = self.inodes.get(&ino).map(|e| e.parent).unwrap_or(ino);
            reply.add(parent, 1, FileType::Directory, "..");

            // Add actual entries
            for (i, entry) in entries.iter().enumerate() {
                if reply.add(entry.ino(), (i + 3) as i64, entry.kind, &entry.name) {
                    break;
                }
            }
        }

        reply.ok();
    }
}
```

---

## Performance Considerations

### Caching Strategies

```rust
// TTL-based caching
const TTL: Duration = Duration::from_secs(1);

fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
    if let Some(entry) = self.cache.lookup(parent, name) {
        // Return cached entry with TTL
        reply.entry(&TTL, &entry.attr, entry.generation);
    } else {
        // Fetch from backend
        let entry = self.fetch_entry(parent, name);
        self.cache.insert(parent, name, entry.clone());
        reply.entry(&TTL, &entry.attr, entry.generation);
    }
}

// Attribute caching
fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
    if let Some(attr) = self.attr_cache.get(&ino) {
        if attr.1.elapsed() < TTL {
            reply.attr(&TTL, attr.0);
            return;
        }
    }

    let attr = self.fetch_attr(ino);
    self.attr_cache.insert(ino, (attr, Instant::now()));
    reply.attr(&TTL, attr);
}
```

### Parallel Operations

```rust
// FUSE supports parallel request handling
struct ParallelFilesystem {
    pool: ThreadPool,
}

impl Filesystem for ParallelFilesystem {
    fn read(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, size: u32, reply: ReplyData) {
        // Offload to thread pool
        let pool = self.pool.clone();
        let reply = reply;

        pool.execute(move || {
            let data = perform_io(ino, offset, size);
            reply.data(&data);
        });
    }
}
```

### Zero-Copy Reads

```rust
// Use sendfile for zero-copy reads
fn read(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, size: u32, reply: ReplyData) {
    let file = &self.open_files.get(&fh).unwrap();

    // Zero-copy: kernel handles data transfer
    let mut buf = vec![0u8; size as usize];
    file.read_at(&mut buf, offset as u64).unwrap();
    reply.data(&buf);
}
```

### Big Writes

```rust
// FUSE 7.19+ supports big writes (up to 1MB)
// Enable in mount options:
let options = [
    "-o", "big_writes",  // Enable big writes
];

// Handle in filesystem:
fn write(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {
    // data can be up to 1MB with big_writes enabled
    let file = &mut self.open_files.get_mut(&fh).unwrap();
    let written = file.write_at(data, offset as u64).unwrap();
    reply.written(written as u32);
}
```

---

## Code Examples

### Complete Simple Filesystem

```rust
use fuser::{Filesystem, FileAttr, FileType, ReplyEmpty, ReplyEntry, ReplyAttr, ReplyData, ReplyDirectory, ReplyWrite, Request};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use std::time::UNIX_EPOCH;

const TTL: Duration = Duration::from_secs(1);

#[derive(Clone)]
struct Node {
    ino: u64,
    parent: u64,
    name: String,
    kind: FileType,
    content: Vec<u8>,
    attr: FileAttr,
}

struct SimpleFS {
    nodes: Arc<Mutex<HashMap<u64, Node>>>,
    next_ino: Mutex<u64>,
}

impl SimpleFS {
    fn new() -> Self {
        let mut nodes = HashMap::new();
        let now = SystemTime::now();

        // Root directory
        nodes.insert(1, Node {
            ino: 1,
            parent: 1,
            name: "/".to_string(),
            kind: FileType::Directory,
            content: vec![],
            attr: FileAttr {
                ino: 1,
                size: 0,
                blocks: 0,
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                kind: FileType::Directory,
                perm: 0o755,
                nlink: 1,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                blksize: 512,
                flags: 0,
            },
        });

        Self {
            nodes: Arc::new(Mutex::new(nodes)),
            next_ino: Mutex::new(2),
        }
    }

    fn allocate_ino(&self) -> u64 {
        let mut next = self.next_ino.lock().unwrap();
        let ino = *next;
        *next += 1;
        ino
    }
}

impl Filesystem for SimpleFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let nodes = self.nodes.lock().unwrap();
        let name_str = name.to_string_lossy();

        for node in nodes.values() {
            if node.parent == parent && node.name == name_str {
                reply.entry(&TTL, &node.attr, 0);
                return;
            }
        }

        reply.error(libc::ENOENT);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let nodes = self.nodes.lock().unwrap();
        if let Some(node) = nodes.get(&ino) {
            reply.attr(&TTL, node.attr);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let nodes = self.nodes.lock().unwrap();

        if offset == 0 {
            // Add . and ..
            let parent = nodes.get(&ino).map(|n| n.parent).unwrap_or(ino);
            reply.add(ino, 1, FileType::Directory, ".");
            reply.add(parent, 1, FileType::Directory, "..");

            // Add children
            let mut entries: Vec<_> = nodes.values()
                .filter(|n| n.parent == ino)
                .collect();
            entries.sort_by(|a, b| a.name.cmp(&b.name));

            for (i, node) in entries.iter().enumerate() {
                if reply.add(node.ino, (i + 2) as i64, node.kind, &node.name) {
                    break;
                }
            }
        }

        reply.ok();
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        let nodes = self.nodes.lock().unwrap();

        if let Some(node) = nodes.get(&ino) {
            if node.kind != FileType::RegularFile {
                reply.error(libc::EISDIR);
                return;
            }

            let start = offset as usize;
            let end = (start + size as usize).min(node.content.len());

            if start < node.content.len() {
                reply.data(&node.content[start..end]);
            } else {
                reply.data(&[]);
            }
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, data: &[u8], _flags: u32, reply: ReplyWrite) {
        let mut nodes = self.nodes.lock().unwrap();

        if let Some(node) = nodes.get_mut(&ino) {
            if node.kind != FileType::RegularFile {
                reply.error(libc::EISDIR);
                return;
            }

            let offset = offset as usize;
            let end = offset + data.len();

            if end > node.content.len() {
                node.content.resize(end, 0);
            }

            node.content[offset..end].copy_from_slice(data);
            node.attr.size = node.content.len() as u64;
            node.attr.mtime = SystemTime::now();

            reply.written(data.len() as u32);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, _flags: i32, reply: ReplyEntry) {
        let mut nodes = self.nodes.lock().unwrap();
        let ino = self.allocate_ino();
        let now = SystemTime::now();

        let node = Node {
            ino,
            parent,
            name: name.to_string_lossy().into_owned(),
            kind: FileType::RegularFile,
            content: vec![],
            attr: FileAttr {
                ino,
                size: 0,
                blocks: 0,
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                kind: FileType::RegularFile,
                perm: mode as u16,
                nlink: 1,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                blksize: 512,
                flags: 0,
            },
        };

        nodes.insert(ino, node.clone());
        reply.entry(&TTL, &node.attr, 0);
    }

    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, reply: ReplyEntry) {
        let mut nodes = self.nodes.lock().unwrap();
        let ino = self.allocate_ino();
        let now = SystemTime::now();

        let node = Node {
            ino,
            parent,
            name: name.to_string_lossy().into_owned(),
            kind: FileType::Directory,
            content: vec![],
            attr: FileAttr {
                ino,
                size: 0,
                blocks: 0,
                atime: now,
                mtime: now,
                ctime: now,
                crtime: now,
                kind: FileType::Directory,
                perm: mode as u16,
                nlink: 1,
                uid: 1000,
                gid: 1000,
                rdev: 0,
                blksize: 512,
                flags: 0,
            },
        };

        nodes.insert(ino, node.clone());
        reply.entry(&TTL, &node.attr, 0);
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let mut nodes = self.nodes.lock().unwrap();
        let name_str = name.to_string_lossy();

        let ino = nodes.values()
            .find(|n| n.parent == parent && n.name == name_str)
            .map(|n| n.ino);

        if let Some(ino) = ino {
            nodes.remove(&ino);
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }
}

fn main() {
    let fs = SimpleFS::new();
    fuser::mount2(fs, "/mnt/simple", &[
        MountOption::FSName("simple".to_string()),
        MountOption::AutoUnmount,
    ]);
}
```

---

## Summary

### Key Takeaways

1. **FUSE** enables safe, productive userspace filesystem development
2. **fuser** is a complete Rust implementation with full FUSE ABI support
3. **Request/Response model**: Kernel forwards operations, userspace implements
4. **Caching is critical**: TTL-based caching for attributes and entries
5. **Performance optimizations**: Big writes, parallel ops, zero-copy reads

### Further Reading

- [fuser Documentation](https://docs.rs/fuser)
- [libfuse Documentation](https://libfuse.github.io/)
- [FUSE Kernel Documentation](https://www.kernel.org/doc/html/latest/filesystems/fuse.html)
