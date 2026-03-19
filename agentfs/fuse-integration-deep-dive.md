# FUSE Integration in Rust: Beginner to Advanced

A comprehensive guide to becoming a skilled FUSE developer on Linux, using the AgentFS codebase as a reference implementation.

## Table of Contents

1. [Introduction](#introduction)
2. [Part 1: FUSE Fundamentals](#part-1-fuse-fundamentals)
3. [Part 2: Building Your First FUSE Filesystem](#part-2-building-your-first-fuse-filesystem)
4. [Part 3: Deep Dive into AgentFS](#part-3-deep-dive-into-agentfs)
5. [Part 4: Overlay Filesystems](#part-4-overlay-filesystems)
6. [Part 5: Advanced Topics](#part-5-advanced-topics)
7. [Part 6: Production Patterns](#part-6-production-patterns)
8. [Appendix: Reference Materials](#appendix-reference-materials)

---

## Introduction

### What is FUSE?

**FUSE (Filesystem in Userspace)** is a kernel mechanism that allows userspace programs to implement filesystem functionality. Instead of writing kernel modules (complex, dangerous, requires reboot on crashes), FUSE lets you write filesystems as regular userspace processes.

### Why FUSE in Rust?

- **Memory Safety**: Rust's ownership model prevents common filesystem bugs (use-after-free, buffer overflows)
- **Performance**: Zero-cost abstractions, no GC pauses
- **Async Support**: Tokio integration for non-blocking I/O
- **Strong Typing**: Catch errors at compile time

### What You'll Build

By the end of this guide, you'll understand:
- How to mount a FUSE filesystem on Linux
- How to implement all filesystem operations
- How copy-on-write overlays work
- How to intercept and redirect syscalls
- Production patterns for caching, performance, and debugging

---

## Part 1: FUSE Fundamentals

### Chapter 1.1: Linux Filesystem Basics

Before writing a FUSE filesystem, understand what you're implementing.

#### The VFS Layer

Linux uses a **Virtual Filesystem (VFS)** layer that abstracts filesystem operations:

```
+------------------+
|    Application   |
+------------------+
|  libc (open, read) |
+------------------+
|   Syscall Interface |
+------------------+
|      VFS Layer      |
+------------------+
|  Ext4 | XFS | FUSE |
+------------------+
```

#### Inodes and Dentries

- **Inode**: File metadata (permissions, size, timestamps, data blocks)
- **Dentry**: Directory entry (name → inode mapping)
- **Path Resolution**: Walking dentries from root to target

#### File Operations

Every filesystem implements these operations:

| Operation | Purpose | Returns |
|-----------|---------|---------|
| `lookup` | Resolve name in directory | inode |
| `getattr` | Get file attributes | stat struct |
| `readdir` | List directory contents | list of names |
| `open` | Open a file | file handle |
| `read` | Read file data | bytes |
| `write` | Write file data | bytes written |
| `create` | Create a new file | inode + handle |
| `unlink` | Delete a file | success/error |
| `mkdir` | Create directory | inode |
| `rmdir` | Remove directory | success/error |
| `rename` | Move/rename file | success/error |
| `chmod` | Change permissions | success/error |
| `setattr` | Change attributes | updated stat |

### Chapter 1.2: How FUSE Works

FUSE communication flow:

```
+-------------+     FUSE Device      +-------------+
|   Kernel    | <-----------------> |   Userspace |
|   VFS       |    (/dev/fuse)      |   FUSE Lib  |
+-------------+                      +-------------+
     |                                     |
     v                                     v
  File Operation                       Your Handler
  Request                              Implementation
```

1. Application calls `open("/mnt/file.txt")`
2. Kernel VFS sends request to `/dev/fuse`
3. FUSE library receives request
4. Your code handles the request
5. Response sent back through `/dev/fuse`
6. Kernel returns result to application

### Chapter 1.3: Setting Up Your Environment

#### Install FUSE on Linux

```bash
# Arch Linux
sudo pacman -S fuse2 fuse3

# Ubuntu/Debian
sudo apt install fuse libfuse-dev

# Fedora
sudo dnf install fuse fuse-devel

# Verify installation
fusermount3 --version
ls -la /dev/fuse  # Should show character device
```

#### Create a New Project

```bash
cargo new my-fuse-fs
cd my-fuse-fs

# Add dependencies
cargo add fuser tokio parking_lot anyhow libc
cargo add --dev tempfile

# Your Cargo.toml should have:
# [dependencies]
# fuser = "0.15"
# tokio = { version = "1", features = ["full"] }
# parking_lot = "0.12"
# anyhow = "1.0"
# libc = "0.2"
```

#### Create a Minimal FUSE Filesystem

Create `src/main.rs`:

```rust
use fuser::{Filesystem, Request, ReplyEmpty, ReplyEntry, ReplyAttr, ReplyData,
            ReplyDirectory, FileAttr, FileType};
use std::time::{Duration, UNIX_EPOCH};
use std::ffi::OsStr;

const TTL: Duration = Duration::from_secs(1);

struct HelloFs;

impl Filesystem for HelloFs {
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        match ino {
            1 => {
                // Root directory
                let attr = FileAttr {
                    ino: 1,
                    size: 0,
                    blocks: 0,
                    atime: UNIX_EPOCH,
                    mtime: UNIX_EPOCH,
                    ctime: UNIX_EPOCH,
                    crtime: UNIX_EPOCH,
                    kind: FileType::Directory,
                    perm: 0o755,
                    nlink: 2,
                    uid: unsafe { libc::getuid() },
                    gid: unsafe { libc::getgid() },
                    rdev: 0,
                    flags: 0,
                    blksize: 512,
                };
                reply.attr(&TTL, &attr);
            }
            _ => reply.error(libc::ENOENT),
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64,
               mut reply: ReplyDirectory) {
        if ino != 1 {
            reply.error(libc::ENOENT);
            return;
        }

        // Add "." and ".." entries
        reply.add(1, 0, FileType::Directory, ".");
        reply.add(1, 0, FileType::Directory, "..");

        reply.ok();
    }
}

fn main() {
    let mountpoint = std::env::args().nth(1).expect("Usage: my-fuse-fs <mountpoint>");
    let fs = HelloFs;

    println!("Mounting FUSE filesystem at {}", mountpoint);
    fuser::mount2(fs, &mountpoint, &[]);
}
```

#### Build and Mount

```bash
cargo build --release

# Create mount point
mkdir /tmp/hello-mount

# Mount (in foreground)
./target/release/my-fuse-fs /tmp/hello-mount

# In another terminal:
ls -la /tmp/hello-mount
df -h /tmp/hello-mount

# Unmount
fusermount3 -u /tmp/hello-mount
```

---

## Part 2: Building Your First FUSE Filesystem

### Chapter 2.1: In-Memory Filesystem

Let's build a functional in-memory filesystem with files and directories.

```rust
use fuser::{Filesystem, Request, ReplyEmpty, ReplyEntry, ReplyAttr, ReplyData,
            ReplyDirectory, ReplyCreate, FileAttr, FileType, FUSE_WRITE_LOCK};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::{Arc, Mutex};

const TTL: Duration = Duration::from_secs(1);

#[derive(Clone)]
struct Node {
    ino: u64,
    kind: FileType,
    perm: u16,
    uid: u32,
    gid: u32,
    atime: SystemTime,
    mtime: SystemTime,
    ctime: SystemTime,
    content: Vec<u8>,  // Only for files
    children: HashMap<String, u64>,  // Only for directories
}

struct MemFs {
    nodes: Arc<Mutex<HashMap<u64, Node>>>,
    next_ino: Mutex<u64>,
}

impl MemFs {
    fn new() -> Self {
        let mut nodes = HashMap::new();
        let now = SystemTime::now();

        // Root directory (ino = 1)
        nodes.insert(1, Node {
            ino: 1,
            kind: FileType::Directory,
            perm: 0o755,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            atime: now,
            mtime: now,
            ctime: now,
            content: Vec::new(),
            children: HashMap::new(),
        });

        Self {
            nodes: Arc::new(Mutex::new(nodes)),
            next_ino: Mutex::new(2),
        }
    }

    fn alloc_ino(&self) -> u64 {
        let mut next = self.next_ino.lock().unwrap();
        let ino = *next;
        *next += 1;
        ino
    }
}

impl Filesystem for MemFs {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let nodes = self.nodes.lock().unwrap();

        let parent_node = match nodes.get(&parent) {
            Some(n) if n.kind == FileType::Directory => n,
            _ => { reply.error(libc::ENOENT); return; }
        };

        let name_str = match name.to_str() {
            Some(s) => s,
            None => { reply.error(libc::EINVAL); return; }
        };

        match parent_node.children.get(name_str) {
            Some(&ino) => {
                if let Some(node) = nodes.get(&ino) {
                    let attr = node_to_attr(node);
                    reply.entry(&TTL, &attr, 0);
                } else {
                    reply.error(libc::ENOENT);
                }
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let nodes = self.nodes.lock().unwrap();
        match nodes.get(&ino) {
            Some(node) => reply.attr(&TTL, &node_to_attr(node)),
            None => reply.error(libc::ENOENT),
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64,
               mut reply: ReplyDirectory) {
        let nodes = self.nodes.lock().unwrap();

        let node = match nodes.get(&ino) {
            Some(n) if n.kind == FileType::Directory => n,
            _ => { reply.error(libc::ENOENT); return; }
        };

        // Build entry list
        let mut entries = vec![
            (ino, FileType::Directory, "."),
            (if ino == 1 { 1 } else {
                // Find parent inode (simplified - in production, store parent pointers)
                1
            }, FileType::Directory, ".."),
        ];

        for (name, &ino) in &node.children {
            let kind = nodes.get(&ino).map(|n| n.kind).unwrap_or(FileType::RegularFile);
            entries.push((ino, kind, name.as_str()));
        }

        // Add entries starting from offset
        for (i, (ino, kind, name)) in entries.iter().enumerate().skip(offset as usize) {
            if reply.add(*ino, (i + 1) as i64, *kind, name) {
                break;
            }
        }

        reply.ok();
    }

    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32,
              _umask: u32, _flags: i32, reply: ReplyCreate) {
        let mut nodes = self.nodes.lock().unwrap();

        // Get parent directory
        let parent_node = match nodes.get_mut(&parent) {
            Some(n) if n.kind == FileType::Directory => n,
            _ => { reply.error(libc::ENOENT); return; }
        };

        let name_str = match name.to_str() {
            Some(s) => s,
            None => { reply.error(libc::EINVAL); return; }
        };

        // Check if already exists
        if parent_node.children.contains_key(name_str) {
            reply.error(libc::EEXIST);
            return;
        }

        // Create new file
        let ino = self.alloc_ino();
        let now = SystemTime::now();

        let file = Node {
            ino,
            kind: FileType::RegularFile,
            perm: (mode & 0o777) as u16,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            atime: now,
            mtime: now,
            ctime: now,
            content: Vec::new(),
            children: HashMap::new(),
        };

        parent_node.children.insert(name_str.to_string(), ino);
        nodes.insert(ino, file);

        // Update parent mtime
        parent_node.mtime = SystemTime::now();

        if let Some(node) = nodes.get(&ino) {
            let attr = node_to_attr(node);
            reply.created(&TTL, &attr, 0, 0, 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn write(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64,
             data: &[u8], _write_flags: u32, reply: ReplyWrite) {
        let mut nodes = self.nodes.lock().unwrap();

        let node = match nodes.get_mut(&ino) {
            Some(n) if n.kind == FileType::RegularFile => n,
            _ => { reply.error(libc::EBADF); return; }
        };

        let offset = offset as usize;
        let new_len = offset + data.len();

        // Extend file if needed
        if node.content.len() < new_len {
            node.content.resize(new_len, 0);
        }

        // Write data
        node.content[offset..offset + data.len()].copy_from_slice(data);
        node.mtime = SystemTime::now();

        reply.written(data.len() as u32);
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64,
            size: u32, reply: ReplyData) {
        let nodes = self.nodes.lock().unwrap();

        let node = match nodes.get(&ino) {
            Some(n) if n.kind == FileType::RegularFile => n,
            _ => { reply.error(libc::EBADF); return; }
        };

        let offset = offset as usize;
        let size = size as usize;

        if offset >= node.content.len() {
            reply.data(&[]);
        } else {
            let end = (offset + size).min(node.content.len());
            reply.data(&node.content[offset..end]);
        }
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let mut nodes = self.nodes.lock().unwrap();

        let parent_node = match nodes.get_mut(&parent) {
            Some(n) if n.kind == FileType::Directory => n,
            _ => { reply.error(libc::ENOENT); return; }
        };

        let name_str = match name.to_str() {
            Some(s) => s,
            None => { reply.error(libc::EINVAL); return; }
        };

        // Get inode to remove
        let ino = match parent_node.children.remove(name_str) {
            Some(ino) => ino,
            None => { reply.error(libc::ENOENT); return; }
        };

        // Remove the node
        nodes.remove(&ino);
        parent_node.mtime = SystemTime::now();

        reply.ok();
    }

    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32,
             _umask: u32, reply: ReplyEntry) {
        let mut nodes = self.nodes.lock().unwrap();

        let parent_node = match nodes.get_mut(&parent) {
            Some(n) if n.kind == FileType::Directory => n,
            _ => { reply.error(libc::ENOENT); return; }
        };

        let name_str = match name.to_str() {
            Some(s) => s,
            None => { reply.error(libc::EINVAL); return; }
        };

        if parent_node.children.contains_key(name_str) {
            reply.error(libc::EEXIST);
            return;
        }

        let ino = self.alloc_ino();
        let now = SystemTime::now();

        let dir = Node {
            ino,
            kind: FileType::Directory,
            perm: (mode & 0o777) as u16,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            atime: now,
            mtime: now,
            ctime: now,
            content: Vec::new(),
            children: HashMap::new(),
        };

        parent_node.children.insert(name_str.to_string(), ino);
        parent_node.mtime = SystemTime::now();
        nodes.insert(ino, dir);

        if let Some(node) = nodes.get(&ino) {
            let attr = node_to_attr(node);
            reply.entry(&TTL, &attr, 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        let mut nodes = self.nodes.lock().unwrap();

        let parent_node = match nodes.get_mut(&parent) {
            Some(n) if n.kind == FileType::Directory => n,
            _ => { reply.error(libc::ENOENT); return; }
        };

        let name_str = match name.to_str() {
            Some(s) => s,
            None => { reply.error(libc::EINVAL); return; }
        };

        let ino = match parent_node.children.get(name_str) {
            Some(&ino) => ino,
            None => { reply.error(libc::ENOENT); return; }
        };

        // Check if directory is empty
        let is_empty = nodes.get(&ino)
            .map(|n| n.children.is_empty())
            .unwrap_or(false);

        if !is_empty {
            reply.error(libc::ENOTEMPTY);
            return;
        }

        parent_node.children.remove(name_str);
        nodes.remove(&ino);
        parent_node.mtime = SystemTime::now();

        reply.ok();
    }

    fn rename(&mut self, _req: &Request, parent: u64, name: &OsStr,
              newparent: u64, newname: &OsStr, _flags: u32, reply: ReplyEmpty) {
        let mut nodes = self.nodes.lock().unwrap();

        // Get source
        let parent_node = match nodes.get_mut(&parent) {
            Some(n) if n.kind == FileType::Directory => n,
            _ => { reply.error(libc::ENOENT); return; }
        };

        let name_str = match name.to_str() {
            Some(s) => s,
            None => { reply.error(libc::EINVAL); return; }
        };

        let ino = match parent_node.children.remove(name_str) {
            Some(ino) => ino,
            None => { reply.error(libc::ENOENT); return; }
        };

        // Get destination parent
        let newparent_node = match nodes.get_mut(&newparent) {
            Some(n) if n.kind == FileType::Directory => n,
            _ => {
                // Restore source
                parent_node.children.insert(name_str.to_string(), ino);
                reply.error(libc::ENOENT);
                return;
            }
        };

        let newname_str = match newname.to_str() {
            Some(s) => s,
            None => {
                parent_node.children.insert(name_str.to_string(), ino);
                reply.error(libc::EINVAL);
                return;
            }
        };

        // Remove destination if exists
        newparent_node.children.remove(newname_str);

        // Add to new parent
        newparent_node.children.insert(newname_str.to_string(), ino);
        newparent_node.mtime = SystemTime::now();
        parent_node.mtime = SystemTime::now();

        reply.ok();
    }
}

fn node_to_attr(node: &Node) -> FileAttr {
    FileAttr {
        ino: node.ino,
        size: node.content.len() as u64,
        blocks: ((node.content.len() as u64 + 511) / 512) as u64,
        atime: node.atime,
        mtime: node.mtime,
        ctime: node.ctime,
        crtime: node.ctime,
        kind: node.kind,
        perm: node.perm,
        nlink: if node.kind == FileType::Directory { 2 } else { 1 },
        uid: node.uid,
        gid: node.gid,
        rdev: 0,
        flags: 0,
        blksize: 512,
    }
}

fn main() {
    let mountpoint = std::env::args().nth(1).expect("Usage: memfs <mountpoint>");
    let fs = MemFs::new();

    println!("Mounting in-memory FUSE filesystem at {}", mountpoint);
    fuser::mount2(fs, &mountpoint, &[]);
}
```

### Chapter 2.2: Testing Your Filesystem

Create `tests/integration_test.rs`:

```rust
use std::fs;
use std::io::{Write, Read};
use std::process::{Command, Child};
use std::thread;
use std::time::Duration;

fn start_fuse_fs(mountpoint: &str) -> Child {
    Command::new("cargo")
        .args(["run", "--", mountpoint])
        .spawn()
        .expect("Failed to start FUSE filesystem")
}

#[test]
fn test_basic_operations() {
    let mountpoint = "/tmp/memfs-test";

    // Clean up from previous runs
    let _ = fs::remove_dir_all(mountpoint);
    fs::create_dir_all(mountpoint).unwrap();

    // Start FUSE filesystem
    let _fuse_process = start_fuse_fs(mountpoint);
    thread::sleep(Duration::from_secs(1)); // Wait for mount

    // Test file creation
    let file_path = format!("{}/test.txt", mountpoint);
    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(b"Hello, FUSE!").unwrap();
    drop(file);

    // Test file read
    let mut file = fs::File::open(&file_path).unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).unwrap();
    assert_eq!(content, "Hello, FUSE!");

    // Test directory creation
    let dir_path = format!("{}/subdir", mountpoint);
    fs::create_dir(&dir_path).unwrap();
    assert!(fs::metadata(&dir_path).unwrap().is_dir());

    // Test listing
    let entries: Vec<_> = fs::read_dir(mountpoint)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert!(entries.contains(&"test.txt".to_string()));
    assert!(entries.contains(&"subdir".to_string()));

    // Test deletion
    fs::remove_file(&file_path).unwrap();
    assert!(!fs::exists(&file_path).unwrap());

    // Cleanup
    let _ = Command::new("fusermount3")
        .args(["-u", mountpoint])
        .output();
    let _ = fs::remove_dir_all(mountpoint);
}
```

---

## Part 3: Deep Dive into AgentFS

Now let's study how AgentFS implements a production-ready FUSE filesystem.

### Chapter 3.1: AgentFS Architecture Recap

AgentFS has a layered architecture:

```
┌─────────────────────────────────────────┐
│         FUSE Layer (fuse.rs)            │
│  - Path cache (inode → path)            │
│  - File handle tracking                 │
│  - fuser::Filesystem implementation     │
└─────────────────────────────────────────┘
              ↓
┌─────────────────────────────────────────┐
│      OverlayFS (overlayfs.rs)           │
│  - Copy-on-write semantics              │
│  - Whiteout tracking                    │
│  - Base + Delta merging                 │
└─────────────────────────────────────────┘
         ↓              ↓
┌──────────────┐  ┌──────────────┐
│   HostFS     │  │   AgentFS    │
│  (host dir)  │  │  (SQLite)    │
└──────────────┘  └──────────────┘
```

### Chapter 3.2: Path Cache Design

AgentFS maintains an inode-to-path cache because SQLite stores paths, not inodes:

```rust
struct AgentFSFuse {
    fs: Arc<dyn FileSystem>,
    runtime: Runtime,
    path_cache: Arc<Mutex<HashMap<u64, String>>>,  // inode → path
    open_files: Arc<Mutex<HashMap<u64, OpenFile>>>,
    next_fh: AtomicU64,
    uid: u32,
    gid: u32,
    mountpoint_path: String,  // For deadlock prevention
}
```

**Key Methods:**

```rust
// Resolve path from parent inode + name (like kernel's d_lookup)
fn lookup_path(&self, parent_ino: u64, name: &OsStr) -> Option<String> {
    let path_cache = self.path_cache.lock();
    let parent_path = path_cache.get(&parent_ino)?;
    let name_str = name.to_str()?;

    let path = if parent_path == "/" {
        format!("/{}", name_str)
    } else {
        format!("{}/{}", parent_path, name_str)
    };

    // Prevent looking up our own mountpoint (deadlock prevention)
    if path.starts_with(&self.mountpoint_path) {
        None
    } else {
        Some(path)
    }
}

// Add inode → path mapping (like kernel's d_add)
fn add_path(&self, ino: u64, path: String) {
    let mut path_cache = self.path_cache.lock();
    path_cache.insert(ino, path);
}

// Remove inode from cache (like kernel's d_drop)
fn drop_path(&self, ino: u64) {
    let mut path_cache = self.path_cache.lock();
    path_cache.remove(&ino);
}
```

**Why This Matters:**

The kernel expects stable inode numbers. SQLite doesn't provide native inodes, so AgentFS:
1. Uses the path to uniquely identify files
2. Caches inode → path mappings for reverse lookups
3. Generates synthetic inodes from hashes (in HostFS) or uses 1 for root

### Chapter 3.3: File Handle Management

AgentFS tracks open files separately from path lookups:

```rust
struct OpenFile {
    file: BoxedFile,  // Arc<dyn File>
}

fn open(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
    let Some(path) = self.get_path(ino) else {
        reply.error(libc::ENOENT);
        return;
    };

    let file = self.runtime.block_on(async {
        self.fs.open(&path).await
    })?;

    let fh = self.alloc_fh();
    self.open_files.lock().insert(fh, OpenFile { file });
    reply.opened(fh, 0);
}

fn read(&mut self, _req: &Request, _ino: u64, fh: u64, offset: i64,
        size: u32, reply: ReplyData) {
    let file = {
        let open_files = self.open_files.lock();
        open_files.get(&fh).map(|f| f.file.clone())?
    };

    let data = self.runtime.block_on(async {
        file.pread(offset as u64, size as u64).await
    })?;

    reply.data(&data);
}
```

**Key Insights:**

1. **File handles are independent of inodes** - Multiple opens of the same file get different handles
2. **Async bridging** - FUSE is synchronous, but the SDK is async. `block_on` bridges the gap
3. **Handle allocation** - Uses atomic counter for unique handle numbers

### Chapter 3.4: Lookup and Attribute Operations

```rust
fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
    // Resolve path from parent + name
    let Some(path) = self.lookup_path(parent, name) else {
        reply.error(libc::ENOENT);
        return;
    };

    // Stat the path
    let fs = self.fs.clone();
    let result = self.runtime.block_on(async move {
        fs.lstat(&path).await
    });

    match result {
        Ok(Some(stats)) => {
            let attr = fillattr(&stats, self.uid, self.gid);
            self.add_path(attr.ino, path);  // Cache the mapping
            reply.entry(&TTL, &attr, 0);
        }
        Ok(None) => reply.error(libc::ENOENT),
        Err(e) => reply.error(error_to_errno(&e)),
    }
}

fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
    // Reverse lookup: get path from inode
    let Some(path) = self.get_path(ino) else {
        reply.error(libc::ENOENT);
        return;
    };

    let fs = self.fs.clone();
    let result = self.runtime.block_on(async move {
        fs.lstat(&path).await
    });

    match result {
        Ok(Some(stats)) => reply.attr(&TTL, &fillattr(&stats, self.uid, self.gid)),
        Ok(None) => reply.error(libc::ENOENT),
        Err(e) => reply.error(error_to_errno(&e)),
    }
}
```

### Chapter 3.5: Directory Listing

AgentFS implements both `readdir` and `readdirplus` (optimized with attributes):

```rust
fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64,
           mut reply: ReplyDirectory) {
    let Some(path) = self.get_path(ino) else {
        reply.error(libc::ENOENT);
        return;
    };

    // readdir_plus fetches entries WITH stats (avoids N+1 queries)
    let entries = self.runtime.block_on(async {
        self.fs.readdir_plus(&path).await
    }).ok().flatten().unwrap_or_default();

    // Determine parent inode
    let parent_ino = if ino == 1 { 1 } else {
        // Compute from path...
        1
    };

    // Build entries: ".", "..", then children
    let mut all_entries = vec![
        (ino, FileType::Directory, "."),
        (parent_ino, FileType::Directory, ".."),
    ];

    for entry in &entries {
        let kind = if entry.stats.is_directory() {
            FileType::Directory
        } else if entry.stats.is_symlink() {
            FileType::Symlink
        } else {
            FileType::RegularFile
        };

        // Cache inode → path
        let entry_path = format!("{}/{}", path, entry.name);
        self.add_path(entry.stats.ino as u64, entry_path);
        all_entries.push((entry.stats.ino as u64, kind, entry.name.as_str()));
    }

    // Add entries starting from offset
    for (i, (ino, kind, name)) in all_entries.iter().enumerate().skip(offset as usize) {
        if reply.add(*ino, (i + 1) as i64, *kind, name) {
            break;
        }
    }
    reply.ok();
}
```

**Key Optimization:** `readdir_plus` returns entries with stats, avoiding N+1 `getattr` calls.

---

## Part 4: Overlay Filesystems

### Chapter 4.1: Copy-on-Write Fundamentals

An overlay filesystem combines two layers:
- **Base layer**: Read-only (e.g., host directory)
- **Delta layer**: Writable (e.g., SQLite database)

**Lookup Algorithm:**
```
function lookup(path):
    if path in whiteouts:
        return NOT_FOUND

    if path in delta:
        return delta[path]

    if path in base:
        return base[path]

    return NOT_FOUND
```

**Write Algorithm:**
```
function write(path, data):
    remove_whiteout(path)
    ensure_parent_dirs(path)
    delta.write(path, data)
```

**Delete Algorithm:**
```
function delete(path):
    if path in base:
        create_whiteout(path)

    if path in delta:
        delta.delete(path)
```

### Chapter 4.2: Whiteout Implementation

AgentFS uses a SQLite table for whiteouts:

```sql
CREATE TABLE fs_whiteout (
    path TEXT PRIMARY KEY,
    parent_path TEXT NOT NULL,  -- For O(1) child lookups
    created_at INTEGER NOT NULL
);

CREATE INDEX idx_fs_whiteout_parent ON fs_whiteout(parent_path);
```

**Check for whiteout:**
```rust
async fn is_whiteout(&self, path: &str) -> Result<bool> {
    let normalized = self.normalize_path(path);
    let conn = self.delta.get_connection();

    // Check path and all parent paths
    let mut check_path = normalized.clone();
    loop {
        let result = conn.query(
            "SELECT 1 FROM fs_whiteout WHERE path = ?",
            (check_path.as_str(),)
        ).await;

        if let Ok(mut rows) = result {
            if rows.next().await?.is_some() {
                return Ok(true);
            }
        }

        // Check parent
        if let Some(parent_end) = check_path.rfind('/') {
            if parent_end == 0 { break; }
            check_path = check_path[..parent_end].to_string();
        } else {
            break;
        }
    }
    Ok(false)
}
```

### Chapter 4.3: Directory Entry Merging

When listing a directory, merge entries from both layers:

```rust
async fn readdir(&self, path: &str) -> Result<Option<Vec<String>>> {
    let normalized = self.normalize_path(path);

    // Get whiteouts for children
    let child_whiteouts = self.get_child_whiteouts(&normalized).await?;

    let mut entries = HashSet::new();

    // Get entries from delta (take precedence)
    if let Some(delta_entries) = self.delta.readdir(&normalized).await? {
        entries.extend(delta_entries);
    }

    // Get entries from base (if not whiteout)
    if let Some(base_entries) = self.base.readdir(&normalized).await? {
        for entry in base_entries {
            if !child_whiteouts.contains(&entry) {
                entries.insert(entry);
            }
        }
    }

    let mut result: Vec<_> = entries.into_iter().collect();
    result.sort();
    Ok(Some(result))
}
```

### Chapter 4.4: Copy-on-Write File Handles

AgentFS implements CoW at the file handle level:

```rust
pub struct OverlayFile {
    delta_file: Option<BoxedFile>,
    base_file: Option<BoxedFile>,
    delta: AgentFS,
    path: String,
    copied_to_delta: AtomicBool,
}

#[async_trait]
impl File for OverlayFile {
    async fn pwrite(&self, offset: u64, data: &[u8]) -> Result<()> {
        // If already have delta file, write directly
        if let Some(ref delta_file) = self.delta_file {
            return delta_file.pwrite(offset, data).await;
        }

        // Copy-on-write: read from base, write to delta
        if !self.copied_to_delta.load(Ordering::Acquire) {
            if let Some(ref base_file) = self.base_file {
                let stats = base_file.fstat().await?;
                let base_data = base_file.pread(0, stats.size as u64).await?;
                self.delta.write_file(&self.path, &base_data).await?;
            }
            self.copied_to_delta.store(true, Ordering::Release);
        }

        // Now write to delta
        let delta_file = self.delta.open(&self.path).await?;
        delta_file.pwrite(offset, data).await
    }
}
```

### Chapter 4.5: Ensuring Parent Directories

When writing to the delta layer, ensure parent directories exist:

```rust
async fn ensure_parent_dirs_in_delta(&self) -> Result<()> {
    let components: Vec<&str> = self.path.split('/').filter(|s| !s.is_empty()).collect();

    let mut current = String::new();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        current = format!("{}/{}", current, component);

        // Create if doesn't exist in delta
        if self.delta.stat(&current).await?.is_none() {
            self.delta.mkdir(&current).await?;
        }
    }
    Ok(())
}
```

---

## Part 5: Advanced Topics

### Chapter 5.1: FUSE Performance Optimizations

AgentFS enables several FUSE capabilities in `init()`:

```rust
fn init(&mut self, _req: &Request, config: &mut KernelConfig) -> Result<(), libc::c_int> {
    let _ = config.add_capabilities(
        FUSE_ASYNC_READ           // Parallel read requests
        | FUSE_WRITEBACK_CACHE    // Kernel buffers writes
        | FUSE_PARALLEL_DIROPS    // Concurrent readdir
        | FUSE_CACHE_SYMLINKS     // Cache symlink targets
        | FUSE_NO_OPENDIR_SUPPORT // Skip opendir/releasedir
    );
    Ok(())
}
```

**What each does:**

| Capability | Effect |
|------------|--------|
| `FUSE_ASYNC_READ` | Kernel can issue multiple read requests in parallel |
| `FUSE_WRITEBACK_CACHE` | Kernel buffers small writes, flushes later |
| `FUSE_PARALLEL_DIROPS` | Multiple threads can readdir same directory |
| `FUSE_CACHE_SYMLINKS` | Symlink targets cached, no repeated lookups |
| `FUSE_NO_OPENDIR_SUPPORT` | Skip opendir/releasedir calls |

### Chapter 5.2: Attribute Caching

AgentFS uses `TTL = Duration::MAX` (never expire) because it's the only writer:

```rust
const TTL: Duration = Duration::MAX;

fn lookup(&mut self, ...) {
    // ...
    reply.entry(&TTL, &attr, 0);  // Kernel caches forever
}

fn getattr(&mut self, ...) {
    // ...
    reply.attr(&TTL, &attr);  // Kernel caches forever
}
```

**When to use shorter TTL:**
- Multiple processes might modify the filesystem
- External changes need to be detected
- Trade-off: consistency vs. performance

### Chapter 5.3: Deadlock Prevention

AgentFS prevents deadlocks when the FUSE mountpoint is under the base directory:

```rust
struct AgentFSFuse {
    mountpoint_path: String,  // "/mnt/agentfs"
}

fn lookup_path(&self, parent_ino: u64, name: &OsStr) -> Option<String> {
    // ... build path ...

    // Prevent looking up our own mountpoint
    if path.starts_with(&self.mountpoint_path) {
        None  // Return ENOENT, don't recurse
    } else {
        Some(path)
    }
}
```

**Why this matters:**

If you mount `/` at `/mnt`, then `ls /mnt` would:
1. Call `readdir("/")` in FUSE
2. See entry `mnt`
3. Call `getattr("/mnt")` → enters FUSE again
4. FUSE tries to lookup `/mnt/mnt` → infinite recursion

The fix: detect and block self-referential lookups.

### Chapter 5.4: Error Code Translation

Translate Rust errors to errno codes:

```rust
fn error_to_errno(e: &anyhow::Error) -> i32 {
    e.downcast_ref::<FsError>()
        .map(|fs_err| fs_err.to_errno())
        .unwrap_or(libc::EIO)
}

enum FsError {
    NotFound,      // → ENOENT
    AlreadyExists, // → EEXIST
    NotADirectory, // → ENOTDIR
    IsADirectory,  // → EISDIR
    NotEmpty,      // → ENOTEMPTY
    InvalidPath,   // → EINVAL
}

impl FsError {
    fn to_errno(&self) -> i32 {
        match self {
            FsError::NotFound => libc::ENOENT,
            FsError::AlreadyExists => libc::EEXIST,
            FsError::NotADirectory => libc::ENOTDIR,
            FsError::IsADirectory => libc::EISDIR,
            FsError::NotEmpty => libc::ENOTEMPTY,
            FsError::InvalidPath => libc::EINVAL,
        }
    }
}
```

### Chapter 5.5: Signal Handling

Reset SIGPIPE to prevent panics when piping output:

```rust
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}
```

**Why:** Without this, `agentfs | head -n1` causes SIGPIPE when the pipe closes.

---

## Part 6: Production Patterns

### Chapter 6.1: Debugging FUSE

**Enable tracing:**
```bash
# Build with debug symbols
cargo build

# Run with tracing
RUST_LOG=agentfs=debug,trace ./target/debug/agentfs mount test /mnt -f
```

**Use strace:**
```bash
strace -f -e trace=fuse ./target/release/agentfs mount test /mnt
```

**Debug FUSE requests:**
```bash
# Mount with FUSE debug options
./target/release/agentfs mount test /mnt -o debug
```

### Chapter 6.2: Testing Strategies

**Integration tests:**
```rust
#[test]
fn test_mount_operations() {
    let mountpoint = tempfile::tempdir().unwrap();

    // Start FUSE in background
    let fuse_process = Command::new("cargo")
        .args(["run", "--", mountpoint.path().to_str().unwrap()])
        .spawn()
        .unwrap();

    sleep(Duration::from_secs(1));

    // Test operations
    fs::write(mountpoint.path().join("test.txt"), "hello").unwrap();
    let content = fs::read_to_string(mountpoint.path().join("test.txt")).unwrap();
    assert_eq!(content, "hello");

    // Cleanup
    Command::new("fusermount3")
        .args(["-u", mountpoint.path()])
        .output()
        .unwrap();
}
```

**Property-based tests:**
```rust
proptest! {
    #[test]
    fn prop_overlay_never_modifies_host(operations in operations_strategy()) {
        // Execute random operations
        // Verify host filesystem unchanged
        // Any modification = test failure
    }
}
```

### Chapter 6.3: Filesystem Statistics

Implement `statfs` for `df` compatibility:

```rust
fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
    const BLOCK_SIZE: u64 = 4096;
    const TOTAL_INODES: u64 = 1_000_000;

    let stats = self.runtime.block_on(async {
        self.fs.statfs().await
    }).unwrap_or_default();

    let used_blocks = stats.bytes_used.div_ceil(BLOCK_SIZE);
    let free_blocks = 1_000_000_000 - used_blocks;  // Virtual capacity

    reply.statfs(
        1_000_000_000,  // Total blocks
        free_blocks,    // Free blocks
        free_blocks,    // Available blocks (for non-root)
        TOTAL_INODES,   // Total inodes
        TOTAL_INODES - stats.inodes,  // Free inodes
        BLOCK_SIZE as u32,  // Block size
        255,            // Max filename length
        BLOCK_SIZE as u32,  // Fragment size
    );
}
```

### Chapter 6.4: Ownership and Permissions

AgentFS overrides uid/gid to prevent "dubious ownership" errors:

```rust
fn mount(fs: Arc<dyn FileSystem>, opts: FuseMountOptions, runtime: Runtime) -> anyhow::Result<()> {
    let uid = opts.uid.unwrap_or_else(|| unsafe { libc::getuid() });
    let gid = opts.gid.unwrap_or_else(|| unsafe { libc::getgid() });

    let fuse_fs = AgentFSFuse::new(fs, runtime, uid, gid, opts.mountpoint.clone());

    fuser::mount2(fuse_fs, &opts.mountpoint, &mount_opts)?;
    Ok(())
}

fn fillattr(stats: &Stats, uid: u32, gid: u32) -> FileAttr {
    FileAttr {
        // ...
        uid,  // Override with mount-time uid
        gid,  // Override with mount-time gid
        // ...
    }
}
```

### Chapter 6.5: Build and Distribution

**Cargo.toml for FUSE projects:**
```toml
[package]
name = "my-fuse-fs"
version = "0.1.0"
edition = "2021"

[dependencies]
fuser = "0.15"
libc = "0.2"
tokio = { version = "1", features = ["full"] }
parking_lot = "0.12"
anyhow = "1.0"

[target.'cfg(target_os = "linux")'.dependencies]
# Linux-specific dependencies

[profile.release]
lto = true
codegen-units = 1
strip = true
```

**Build script for FUSE detection:**
```rust
// build.rs
fn main() {
    // Check for FUSE
    if !std::path::Path::new("/dev/fuse").exists() {
        println!("cargo:warning=FUSE device not found at /dev/fuse");
    }

    // Link FUSE library if needed
    println!("cargo:rustc-link-lib=fuse3");
}
```

---

## Appendix: Reference Materials

### A.1: FUSE Operation Reference

| Operation | When Called | Key Considerations |
|-----------|-------------|-------------------|
| `lookup` | Path resolution, stat | Cache inode→path mapping |
| `getattr` | ls, stat, file dialogs | Return consistent inodes |
| `readdir` | ls, file browsers | Include ".", ".." |
| `open` | Before read/write | Allocate file handle |
| `read` | File content access | Handle offset + size |
| `write` | File modification | Extend file if needed |
| `create` | open(O_CREAT) | Atomic create + open |
| `unlink` | rm, delete | Update parent mtime |
| `mkdir` | mkdir, create folder | Set nlink=2 for dirs |
| `rmdir` | rmdir, delete folder | Check empty first |
| `rename` | mv, rename | Handle cross-dir moves |
| `setattr` | chmod, chown, truncate | Support multiple attrs |
| `readlink` | ls -l, readlink | Return symlink target |
| `symlink` | ln -s | Store target path |
| `fsync` | fsync, data sync | Flush to storage |

### A.2: Common errno Codes

| errno | Value | Meaning |
|-------|-------|---------|
| `ENOENT` | 2 | No such file or directory |
| `EEXIST` | 17 | File already exists |
| `ENOTDIR` | 20 | Not a directory |
| `EISDIR` | 21 | Is a directory |
| `ENOTEMPTY` | 39 | Directory not empty |
| `EINVAL` | 22 | Invalid argument |
| `EBADF` | 9 | Bad file descriptor |
| `EACCES` | 13 | Permission denied |
| `EIO` | 5 | I/O error |

### A.3: File Type Constants

```rust
// File type bits (upper bits of mode)
const S_IFMT: u32 = 0o170000;
const S_IFREG: u32 = 0o100000;  // Regular file
const S_IFDIR: u32 = 0o040000;  // Directory
const S_IFLNK: u32 = 0o120000;  // Symlink

// Permission bits (lower 12 bits)
const S_IRWXU: u32 = 0o0700;  // User mask
const S_IRWXG: u32 = 0o0070;  // Group mask
const S_IRWXO: u32 = 0o0007;  // Other mask
```

### A.4: Learning Resources

**Documentation:**
- [libfuse documentation](https://libfuse.github.io/)
- [FUSE kernel docs](https://www.kernel.org/doc/html/latest/filesystems/fuse.html)
- [fuser crate docs](https://docs.rs/fuser/)

**Example Projects:**
- [AgentFS](https://github.com/tursodatabase/agentfs) - SQLite-backed with overlay
- [sshfs](https://github.com/libfuse/sshfs) - SSH filesystem
- [rclone mount](https://rclone.org/commands/rclone_mount/) - Cloud storage

**Books:**
- "Linux Kernel Development" by Robert Love
- "Operating Systems: Three Easy Pieces"

**Practice Exercises:**
1. In-memory filesystem with persistence
2. Encrypted overlay filesystem
3. Network-backed filesystem
4. Versioned filesystem with snapshots
5. Compressed transparent filesystem

### A.5: Troubleshooting

**Common Issues:**

| Problem | Cause | Solution |
|---------|-------|----------|
| "Transport endpoint is not connected" | FUSE crashed or unmounted cleanly | `fusermount3 -u mountpoint` or `umount -l` |
| "Permission denied" on mount | User not in fuse group | `sudo usermod -aG fuse $USER` |
| Deadlock on mount | Mountpoint under base directory | Add deadlock prevention check |
| Stale inodes | Cache inconsistency | Invalidate cache on mutations |
| High memory usage | Path cache grows unbounded | Consider LRU eviction |

---

## Conclusion

You now have a comprehensive understanding of FUSE development in Rust:

1. **Fundamentals**: How FUSE works, VFS layer, inodes and dentries
2. **Implementation**: Building a functional in-memory filesystem
3. **Production Patterns**: AgentFS architecture, caching, error handling
4. **Advanced Topics**: Overlay filesystems, copy-on-write, performance tuning
5. **Debugging**: Tracing, testing, troubleshooting

The path from beginner to skilled FUSE developer:
- **Start simple**: In-memory filesystem with basic operations
- **Study real code**: AgentFS fuse.rs, overlayfs.rs
- **Build overlays**: Understand CoW and whiteouts
- **Master debugging**: strace, tracing, property tests
- **Contribute**: Fix bugs, add features to open-source FUSE projects

Happy filesysteming!
