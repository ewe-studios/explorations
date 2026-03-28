---
title: "Rust Revision: Native Rust Implementation Guide"
subtitle: "Boxer is already Rust - ownership patterns, FFI considerations, and WASM-specific patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/boxer/rust-revision.md
related_to: ./exploration.md
created: 2026-03-27
status: complete
---

# Rust Revision: Native Rust Implementation

## Executive Summary

**Boxer is already implemented in Rust** - no translation needed. This document covers:

1. **Current Rust Implementation** - Architecture overview
2. **Ownership Patterns** - How Boxer uses Rust's type system
3. **FFI Considerations** - C ABI for WASM
4. **WASM-specific Patterns** - `no_std`, custom allocators
5. **Best Practices** - Lessons from the implementation

---

## 1. Current Rust Implementation

### Project Structure

```
src.boxer/
├── boxer/              # Main CLI and builder (Rust)
│   └── box/
│       ├── src/
│       │   ├── main.rs          # Entry point
│       │   ├── builder/
│       │   │   ├── builder.rs   # Build orchestration
│       │   │   └── packer.rs    # WASM packing
│       │   └── host/
│       │       └── server/
│       │           └── box_host.rs
│       ├── Cargo.toml
│       └── README.md
│
├── wasm-vfs/           # Virtual filesystem (Rust, no_std)
│   ├── src/
│   │   ├── lib.rs              # Library entry
│   │   ├── filesystem.rs       # Core FS implementation
│   │   ├── system.rs           # Process state
│   │   ├── path/               # PathBuf (no std)
│   │   ├── collections/        # HashMap (no std)
│   │   ├── ffi/                # C string helpers
│   │   ├── sync/               # Mutex (no std)
│   │   └── lazy_static/        # Lazy init (no std)
│   ├── Cargo.toml
│   └── README.md
│
└── wacker/             # Dependency manager (planned)
```

### Rust Version and Features

```rust
// wasm-vfs/src/lib.rs
#![no_std]  // No std library dependency
#![allow(dead_code)]  // Allow unused code (library)

// Core library only
extern crate alloc;
extern crate core;

// Dependencies
use serde::{Serialize, Deserialize};
```

### Cargo Configuration

```toml
# wasm-vfs/Cargo.toml
[package]
name = "wasm-vfs"
version = "0.1.1"
edition = "2021"
description = "a Virtual Filesystem layer for Wasm Applications"
license = "Apache-2.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }

[lib]
path = "src/lib.rs"
crate-type = ["staticlib"]  # Static library for linking
```

```toml
# boxer/box/Cargo.toml
[package]
name = "box"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1.0.40"
marcotte = { path = "../../marcotte-wasm" }
dockerfile-parser = "0.8.0"
wasm-encoder = "0.38.1"
wasmparser = "0.106.0"
wizer = "4.0.0"
wasmtime = "17.0.0"
wasmtime-wasi = "16.0.0"
structopt = "0.3.26"
clap = "2.33"
walkdir = "2.3"
```

---

## 2. Ownership Patterns

### FileSystem Ownership

```rust
// wasm-vfs/src/filesystem.rs

pub struct FileSystem {
    pub inodes: Vec<Inode>,           // Owned vector
    pub next_inode_number: u64,
    pub current_directory: PathBuf,   // Owned path
    pub root_inode: Inode,            // Owned inode
    pub files: HashMap<u64, Vec<u8>, FILES_CAP>,  // Owned file data
    pub path_map: HashMap<PathBuf, u64, PATH_MAP_CAP>,  // Owned mapping
}

impl FileSystem {
    // Mutable borrow for modifications
    pub fn mount_file(&mut self, path: &PathBuf, data: &[u8]) -> u64 {
        // Borrow path_map mutably
        let inode_number = self.next_inode_number;
        self.next_inode_number += 1;

        // Insert into owned collections
        self.path_map.insert(path.clone(), inode_number);
        self.files.insert(inode_number, data.to_vec());
    }

    // Immutable borrow for lookups
    pub fn lookup_inode_by_path(&self, path: &PathBuf) -> Option<u64> {
        self.path_map.get(path).copied()
    }
}
```

### Builder Pattern with Ownership

```rust
// boxer/box/src/builder/builder.rs

pub struct Builder {
    pub base_build: Vec<u8>,                    // Owned WASM bytes
    pub working_directory: PathBuf,             // Owned path
    pub copied_files: Vec<(String, Vec<u8>)>,  // Owned file list
}

impl Builder {
    pub fn new() -> Self {
        Self {
            base_build: Vec::new(),
            working_directory: PathBuf::from("/"),
            copied_files: Vec::new(),
        }
    }

    // Mutable borrow for configuration
    pub fn config_base(&mut self, base: &str) {
        match base {
            "scratch" => {
                self.base_build = fs::read("output_wizered.wasm")
                    .unwrap_or_default();
            }
            _ => {}
        }
    }

    // Transfer ownership of files
    pub fn bundle_fs_from_buffer(&mut self, buffer: HashMap<String, Vec<u8>>) {
        for (path, content) in buffer {
            self.copied_files.push((path, content));  // Move ownership
        }
    }
}
```

### Reference Counting (Not Used in wasm-vfs)

```rust
// wasm-vfs uses single ownership - no Rc/Arc needed
// because:
// 1. Single-threaded WASM environment
// 2. All data owned by FileSystem struct
// 3. No shared ownership requirements

// If multi-threading were needed:
use std::sync::Arc;
use std::sync::Mutex;

pub struct SharedFileSystem {
    files: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
}
```

---

## 3. FFI Considerations

### C-compatible Structures

```rust
// wasm-vfs/src/filesystem.rs

/// POSIX stat structure - C ABI compatible
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,      // Device ID
    pub st_ino: u64,      // Inode number
    pub st_mode: u32,     // File mode
    pub st_nlink: u32,    // Link count
    pub st_uid: u32,      // User ID
    pub st_gid: u32,      // Group ID
    pub st_rdev: u64,     // Device type
    pub st_size: i64,     // File size
    pub st_blksize: i64,  // Block size
    pub st_blocks: i64,   // Blocks allocated
    pub st_atime: i64,    // Access time
    pub st_mtime: i64,    // Modification time
    pub st_ctime: i64,    // Change time
}

/// Directory entry - C ABI compatible
#[repr(C)]
pub struct Dirent {
    pub d_ino: u64,       // Inode number
    pub d_off: i64,       // Offset
    pub d_reclen: u16,    // Record length
    pub d_type: u8,       // File type
    pub d_name: [u8; 256], // Name (fixed size for C compatibility)
}
```

### C String Handling

```rust
// wasm-vfs/src/ffi/mod.rs

/// Borrowed C string (non-owned)
pub struct CStr {
    bytes: *const i8,
}

impl CStr {
    /// Create from raw pointer (unsafe)
    pub unsafe fn from_ptr<'a>(ptr: *const i8) -> &'a Self {
        &*(ptr as *const CStr)
    }

    /// Convert to Rust String
    pub unsafe fn to_string_lossy(&self) -> String {
        let mut len = 0;
        while *self.bytes.add(len) != 0 {
            len += 1;
        }

        let slice = core::slice::from_raw_parts(
            self.bytes as *const u8,
            len
        );
        String::from_utf8_lossy(slice).into_owned()
    }
}

/// Owned C string
pub struct CString {
    pub bytes: *mut i8,
}

impl CString {
    /// Create from Rust string
    pub fn new(s: &str) -> Self {
        let mut v = Vec::with_capacity(s.len() + 1);
        v.extend_from_slice(s.as_bytes());
        v.push(0);  // Null terminator

        let ptr = v.as_mut_ptr() as *mut i8;
        core::mem::forget(v);  // Prevent free, transfer ownership
        Self { bytes: ptr }
    }
}
```

### WASM Import/Export

```rust
// wasm-vfs/src/lib.rs

/// WASM export - visible to host
#[no_mangle]
pub extern "C" fn wasm_vfs_mount_in_memory(count: u32, files_ptr: u32) -> i32 {
    unsafe {
        let files = core::slice::from_raw_parts(
            files_ptr as *const FileDef,
            count as usize
        );

        let fs = get_filesystem_mut();
        for file_def in files {
            let path = read_cstr(file_def.path_off);
            let data = core::slice::from_raw_parts(
                file_def.data_off as *const u8,
                file_def.data_len as usize,
            );
            fs.mount_file(&path, data);
        }

        0  // Success
    }
}

/// WASM import - provided by host
extern "C" {
    fn wasm_vfs_alloc(size: u32) -> u32;
    fn wasm_vfs_free(ptr: u32);
}
```

### FileDef for FFI

```rust
// boxer/box/src/builder/builder.rs

/// File definition - matches WASM side exactly
/// Must be #[repr(C)] and same size on both sides
#[repr(C)]
pub struct FileDef {
    pub path_off: u32,   // Offset to path string
    pub data_off: u32,   // Offset to file data
    pub data_len: u32,   // Length of file data
}

// Verify size at compile time
const _: () = assert!(std::mem::size_of::<FileDef>() == 12);
```

---

## 4. WASM-specific Patterns

### no_std Implementation

```rust
// wasm-vfs/src/lib.rs
#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;
extern crate core;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::vec;

// Custom panic handler
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Custom alloc error handler
#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    loop {}
}
```

### Custom HashMap (No std::collections)

```rust
// wasm-vfs/src/collections/mod.rs

/// Simple HashMap with compile-time capacity
/// No heap allocation for the map structure
pub struct HashMap<K, V, const CAP: usize> {
    entries: Vec<Option<(K, V)>>,
}

impl<K: PartialEq, V, const CAP: usize> HashMap<K, V, CAP> {
    /// Initialize with capacity
    pub fn init() -> Self {
        let mut v = Vec::new();
        v.resize_with(CAP, || None);
        Self { entries: v }
    }

    /// Linear search insertion
    pub fn insert(&mut self, key: K, value: V) {
        for slot in self.entries.iter_mut() {
            if let Some((ref stored_k, _)) = slot {
                if *stored_k == key {
                    *slot = Some((key, value));
                    return;
                }
            } else {
                *slot = Some((key, value));
                return;
            }
        }
        // No expansion - capacity is fixed
    }

    /// Linear search lookup
    pub fn get(&self, key: &K) -> Option<&V> {
        for slot in &self.entries {
            if let Some((ref stored_k, ref stored_v)) = slot {
                if *stored_k == *key {
                    return Some(stored_v);
                }
            }
        }
        None
    }
}
```

### Custom PathBuf (No std::path)

```rust
// wasm-vfs/src/path/mod.rs

use core::fmt;
use core::hash::{Hash, Hasher};
use serde::Serialize;

#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct PathBuf {
    inner: String,
}

impl Hash for PathBuf {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl fmt::Debug for PathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathBuf")
            .field("inner", &self.inner)
            .finish()
    }
}

impl PathBuf {
    pub fn from<S: Into<String>>(s: S) -> Self {
        Self { inner: s.into() }
    }

    pub fn is_absolute(&self) -> bool {
        self.inner.starts_with('/')
    }

    pub fn join(&self, other: &PathBuf) -> PathBuf {
        if other.is_absolute() {
            other.clone()
        } else {
            let mut joined = self.inner.clone();
            if !joined.ends_with('/') && !joined.is_empty() {
                joined.push('/');
            }
            joined.push_str(&other.inner);
            PathBuf { inner: joined }
        }
    }
}
```

### Spinlock Mutex (No OS Threads)

```rust
// wasm-vfs/src/sync/mod.rs

use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;

pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    /// Spinlock - no OS blocking
    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self.locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            != Ok(false)
        {
            // Spin - wait for lock
            core::hint::spin_loop();
        }
        MutexGuard { mutex: self }
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}
```

---

## 5. Best Practices

### Error Handling

```rust
// Use anyhow for simple error handling
use anyhow::{Result, Context};

pub fn build_box(dockerfile_path: &Path) -> Result<Vec<u8>> {
    let dockerfile_content = fs::read_to_string(dockerfile_path)
        .with_context(|| format!("Failed to read {:?}", dockerfile_path))?;

    let dockerfile = Dockerfile::parse(&dockerfile_content)
        .context("Failed to parse Dockerfile")?;

    // ... process instructions

    Ok(builder.finalize())
}
```

### Unsafe Code Guidelines

```rust
// wasm-vfs uses unsafe only when necessary
// Each unsafe block has a safety comment

impl FileSystem {
    /// Mount files from host memory
    ///
    /// # Safety
    /// - `files_ptr` must point to valid memory
    /// - `count` must match actual array length
    /// - Memory must not be modified during this call
    pub unsafe fn mount_from_ptr(&mut self, files_ptr: *const FileDef, count: u32) {
        let files = core::slice::from_raw_parts(files_ptr, count as usize);
        // ... process files
    }
}
```

### Testing

```rust
// wasm-vfs/tests/filesystem_tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_file() {
        let mut fs = FileSystem::new();
        let path = PathBuf::from("/test/file.txt");

        let inode = fs.create_file(&path, 0o644);

        assert_eq!(inode, 1);
        assert!(fs.lookup_inode_by_path(&path).is_some());
    }

    #[test]
    fn test_mount_files() {
        let mut fs = FileSystem::new();
        let path = PathBuf::from("/app/data.txt");
        let data = b"Hello, Wasm!";

        fs.mount_file(&path, data);

        let inode = fs.lookup_inode_by_path(&path).unwrap();
        assert_eq!(fs.files.get(&inode).unwrap(), data);
    }
}
```

---

## 6. Summary

### Rust Features Used

| Feature | Location | Purpose |
|---------|----------|---------|
| `no_std` | wasm-vfs | WASM compatibility |
| `extern "C"` | wasm-vfs | FFI with host |
| `#[no_mangle]` | wasm-vfs | Export functions |
| `#[repr(C)]` | filesystem.rs | C ABI compatibility |
| Unsafe blocks | ffi/mod.rs | Pointer operations |
| Custom allocators | collections/mod.rs | No std::collections |

### Ownership Summary

| Component | Ownership Model |
|-----------|-----------------|
| FileSystem | Single owner |
| Inodes | Owned by FileSystem |
| File data | Owned by FileSystem |
| Builder | Single owner |
| WASM module | Owned by runtime |

### FFI Summary

| Type | C ABI | Usage |
|------|-------|-------|
| FileDef | ✅ `#[repr(C)]` | File mounting |
| Stat | ✅ `#[repr(C)]` | File status |
| Dirent | ✅ `#[repr(C)]` | Directory listing |
| CStr/CString | ✅ Manual | String conversion |

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial Rust revision notes |

---

*Continue to [Production-Grade](production-grade.md) for deployment considerations.*
