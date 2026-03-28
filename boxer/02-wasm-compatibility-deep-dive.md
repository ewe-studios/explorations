---
title: "WASM Compatibility Deep Dive"
subtitle: "WASM target architecture, import/export patterns, memory management, and wasm-vfs integration"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/boxer/02-wasm-compatibility-deep-dive.md
related_to: ./exploration.md
created: 2026-03-27
status: complete
---

# WASM Compatibility Deep Dive

## Executive Summary

This deep dive covers WebAssembly compatibility patterns and how Boxer implements them:

1. **WASM Target Architecture** - wasm32-unknown-unknown, wasm32-wasi
2. **Import/Export Patterns** - Host-Wasm communication
3. **Memory Management** - Linear memory, allocations, lifetimes
4. **wasm-vfs Integration** - Virtual filesystem mounting
5. **Browser and Edge Deployment** - Runtime compatibility

---

## 1. WASM Target Architecture

### Rust WASM Targets

Rust supports multiple WASM targets:

```bash
# List available WASM targets
rustup target list | grep wasm

# Output:
# wasm32-unknown-unknown
# wasm32-wasi
# wasm32-unknown-emscripten
# asmjs-unknown-emscripten
```

### Target Comparison

| Target | Description | Use Case |
|--------|-------------|----------|
| `wasm32-unknown-unknown` | Bare WASM, no OS | Browser, custom runtimes |
| `wasm32-wasi` | WASI syscalls | Server-side, CLI tools |
| `wasm32-unknown-emscripten` | Emscripten compat | Porting C/C++ code |

### Boxer's Target Choice

Boxer uses **wasm32-unknown-unknown** for maximum portability:

```toml
# wasm-vfs/Cargo.toml
[package]
name = "wasm-vfs"
version = "0.1.1"
edition = "2021"

[lib]
path = "src/lib.rs"
crate-type = ["staticlib"]  # Static library for linking

[dependencies]
serde = { version = "1.0", features = ["derive"] }
```

**Why `staticlib`?**
- Produces `.a` archive for linking
- No dynamic linking required
- Compatible with C toolchains

### Build Configuration

```bash
# Build wasm-vfs for WASM
cargo build --target wasm32-unknown-unknown --release

# Output:
# target/wasm32-unknown-unknown/release/libwasm_vfs.a
```

### Custom WASM Target Features

```rust
// Enable specific WASM features
#![feature(wasm_import_module)]
#![feature(wasm_target_feature)]

// Enable bulk memory operations
#[target_feature(enable = "bulk-memory")]
unsafe fn fast_copy(src: *mut u8, dst: *mut u8, len: usize) {
    core::ptr::copy_nonoverlapping(src, dst, len);
}
```

---

## 2. Import/Export Patterns

### WASM Module Interface

```
┌─────────────────────────────────────────┐
│ Host (Rust/Node.js/Browser)             │
│                                         │
│ ┌─────────────────────────────────┐     │
│ │ Imports (provided to WASM)      │     │
│ │ - wasm_vfs_open()               │     │
│ │ - wasm_vfs_read()               │     │
│ │ - wasm_vfs_write()              │     │
│ └─────────────────────────────────┘     │
│                                         │
│ ┌─────────────────────────────────┐     │
│ │ Exports (called from WASM)      │     │
│ │ - _start()                      │     │
│ │ - wasm_vfs_mount_in_memory()    │     │
│ └─────────────────────────────────┘     │
└─────────────────────────────────────────┘
           ↑           ↓
      Exports     Imports
           ↓           ↑
┌─────────────────────────────────────────┐
│ WASM Module (wasm-vfs + application)    │
│                                         │
│ ┌─────────────────────────────────┐     │
│ │ Exports (callable by host)      │     │
│ │ - _start()                      │     │
│ │ - wasm_vfs_mount_in_memory()    │     │
│ └─────────────────────────────────┘     │
│                                         │
│ ┌─────────────────────────────────┐     │
│ │ Imports (calls to host)         │     │
│ │ - wasm_vfs_open()               │     │
│ │ - wasm_vfs_read()               │     │
│ └─────────────────────────────────┘     │
└─────────────────────────────────────────┘
```

### Exporting Functions from WASM

```rust
// wasm-vfs/src/lib.rs

/// Mount files from host memory into WASM VFS
///
/// # Arguments
/// * `count` - Number of files to mount
/// * `files_ptr` - Pointer to FileDef array in WASM memory
///
/// # Returns
/// * `0` on success, error code otherwise
#[no_mangle]
pub extern "C" fn wasm_vfs_mount_in_memory(count: u32, files_ptr: u32) -> i32 {
    unsafe {
        // Read FileDef array from WASM memory
        let files = core::slice::from_raw_parts(files_ptr as *const FileDef, count as usize);

        // Mount each file
        for file_def in files {
            let path = read_cstr(file_def.path_off);
            let data = core::slice::from_raw_parts(
                file_def.data_off as *const u8,
                file_def.data_len as usize,
            );

            // Insert into VFS
            FILESYSTEM.mount_file(&path, data);
        }

        0  // Success
    }
}
```

### Importing Host Functions

```rust
// wasm-vfs/src/ffi/mod.rs

extern "C" {
    /// Host-provided function to allocate memory
    fn wasm_vfs_alloc(size: u32) -> u32;

    /// Host-provided function to free memory
    fn wasm_vfs_free(ptr: u32);

    /// Host-provided function to write to console
    fn wasm_vfs_log(msg_ptr: u32, msg_len: u32);
}

/// Safe wrapper for host allocation
pub fn alloc(size: usize) -> *mut u8 {
    unsafe {
        let ptr = wasm_vfs_alloc(size as u32);
        ptr as *mut u8
    }
}
```

### Host-Side Implementation (Wasmtime)

```rust
// boxer/box/src/builder/builder.rs

use wasmtime::*;

fn mount_files(module: Module, files: Vec<(String, Vec<u8>)>) -> Result<()> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());

    // Instantiate module
    let instance = linker.instantiate(&mut store, &module)?;

    // Get exported memory
    let memory = instance.get_memory(&mut store, "memory")
        .ok_or("No exported memory")?;

    // Build FileDef array
    let filedefs = build_filedefs(&files);

    // Copy to guest memory
    let filedef_ptr = allocate_guest_memory(memory, &mut store, filedefs.len());
    copy_to_guest(memory, &mut store, filedef_ptr, &filedefs);

    // Call wasm_vfs_mount_in_memory
    let mount_func = instance.get_func(&mut store, "wasm_vfs_mount_in_memory")
        .ok_or("Function not found")?;

    let typed_mount = mount_func.typed::<(u32, u32), i32>(&store)?;
    let result = typed_mount.call(&mut store, (filedefs.len() as u32, filedef_ptr))?;

    println!("Mount result: {}", result);
    Ok(())
}
```

---

## 3. Memory Management in WASM

### Linear Memory Model

Wasm uses a **linear memory** model - a contiguous array of bytes:

```
WASM Memory Layout:
0x000000 ┌────────────────────────────┐
         │ Stack (grows downward)     │
         │ - Local variables          │
         │ - Function call frames     │
         ├────────────────────────────┤
         │ Free space                 │
         ├────────────────────────────┤
         │ Heap (grows upward)        │
         │ - Allocated objects        │
         │ - File data                │
         │ - Path strings             │
         ├────────────────────────────┤
         │ Static data                │
         │ - FileDef array            │
         │ - Constants                │
         └────────────────────────────┘ 0xFFFFF
```

### Memory Operations

```rust
// Host writes to WASM memory
memory.write(&mut store, offset, data)?;

// Host reads from WASM memory
memory.read(&mut store, offset, &mut buffer)?;

// WASM module requests more pages
let old_size = memory.grow(&mut store, additional_pages)?;
```

### FileDef Structure

```rust
// boxer/box/src/builder/builder.rs

/// File definition structure for mounting files
/// This must match the struct on the WASM side
#[repr(C)]
pub struct FileDef {
    pub path_off: u32,   // Offset to null-terminated path string
    pub data_off: u32,   // Offset to file data
    pub data_len: u32,   // Length of file data in bytes
}

// Size: 12 bytes (3 * u32)
assert_eq!(std::mem::size_of::<FileDef>(), 12);
```

### Memory Copy Pattern

```rust
// Builder copies files to WASM memory
let mut guest_cursor = 0x10000;  // Start at 64KB (avoid stack)
let filedef_array_base = guest_cursor;

// Helper for copying data
let mut copy_to_guest = |host_data: &[u8], offset: usize| {
    memory.write(&mut store, offset, host_data)
        .expect("Failed to write to guest memory");
};

for (dest_path, data) in &self.copied_files {
    // 1. Copy path (null-terminated)
    let mut path_bytes = dest_path.as_bytes().to_vec();
    path_bytes.push(0);
    let path_offset = guest_cursor;
    copy_to_guest(&path_bytes, path_offset);
    guest_cursor += path_bytes.len();

    // 2. Copy file data
    let data_offset = guest_cursor;
    copy_to_guest(data, data_offset);
    guest_cursor += data.len();

    // 3. Update FileDef struct
    let base = idx * std::mem::size_of::<FileDef>();
    filedef_structs[base..base+4]
        .copy_from_slice(&(path_offset as u32).to_le_bytes());
    filedef_structs[base+4..base+8]
        .copy_from_slice(&(data_offset as u32).to_le_bytes());
    filedef_structs[base+8..base+12]
        .copy_from_slice(&(data.len() as u32).to_le_bytes());
}

// 4. Copy FileDef array itself
copy_to_guest(&filedef_structs, filedef_array_base);
```

---

## 4. wasm-vfs Integration

### FileSystem Structure

```rust
// wasm-vfs/src/filesystem.rs

pub struct FileSystem {
    /// Inodes indexed by inode number
    pub inodes: Vec<Inode>,
    pub next_inode_number: u64,

    /// Current working directory
    pub current_directory: PathBuf,

    /// Root inode
    pub root_inode: Inode,

    /// File data storage (inode_number -> bytes)
    pub files: HashMap<u64, Vec<u8>, FILES_CAP>,

    /// Path to inode mapping
    pub path_map: HashMap<PathBuf, u64, PATH_MAP_CAP>,
}

impl FileSystem {
    pub fn new() -> Self {
        let root_inode = Inode::new(
            0, 0, Permissions::default(),
            0, 0, 0, 0, 0, InodeKind::Directory
        );

        let mut fs = Self {
            inodes: vec![root_inode.clone()],
            next_inode_number: 1,
            current_directory: PathBuf::from("/"),
            root_inode,
            files: HashMap::init(),
            path_map: HashMap::init(),
        };

        // Insert root directory
        fs.path_map.insert(PathBuf::from("/"), 0);
        fs
    }
}
```

### Mounting Files

```rust
// wasm-vfs/src/filesystem.rs

impl FileSystem {
    /// Mount a file from host memory
    pub fn mount_file(&mut self, path: &PathBuf, data: &[u8]) -> u64 {
        // Create new inode
        let inode_number = self.next_inode_number;
        self.next_inode_number += 1;

        let inode = Inode::new(
            inode_number,
            data.len() as u64,
            Permissions::from(0o644),
            0, 0, 0, 0, 0,
            InodeKind::File
        );

        // Register inode
        self.inodes.push(inode);

        // Store file data
        self.files.insert(inode_number, data.to_vec());

        // Map path to inode
        self.path_map.insert(path.clone(), inode_number);

        inode_number
    }

    /// Lookup inode by path
    pub fn lookup_inode_by_path(&self, path: &PathBuf) -> Option<u64> {
        self.path_map.get(path).copied()
    }

    /// Create a new file
    pub fn create_file(&mut self, path: &PathBuf, mode: u32) -> u64 {
        let inode_number = self.next_inode_number;
        self.next_inode_number += 1;

        let inode = Inode::new(
            inode_number,
            0,
            Permissions::from(mode as u16),
            0, 0, 0, 0, 0,
            InodeKind::File
        );

        self.inodes.push(inode);
        self.path_map.insert(path.clone(), inode_number);
        self.files.insert(inode_number, Vec::new());

        inode_number
    }
}
```

### C FFI Layer

```rust
// wasm-vfs/src/ffi/mod.rs

/// C-compatible string structure
pub struct CStr {
    bytes: *const i8,
}

impl CStr {
    /// Create Rust string from C string pointer
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

/// C-compatible owned string
pub struct CString {
    pub bytes: *mut i8,
}

impl CString {
    /// Create C string from Rust string
    pub fn new(s: &str) -> Self {
        let mut v = Vec::with_capacity(s.len() + 1);
        v.extend_from_slice(s.as_bytes());
        v.push(0);  // Null terminator

        let ptr = v.as_mut_ptr() as *mut i8;
        core::mem::forget(v);  // Transfer ownership
        Self { bytes: ptr }
    }
}
```

### POSIX Syscall Interface

wasm-vfs implements POSIX-like syscalls:

```rust
// wasm-vfs/src/lib.rs

/// Open a file and return file descriptor
#[no_mangle]
pub extern "C" fn wasm_vfs_open(path_ptr: *const u8, path_len: u32, flags: i32) -> i32 {
    unsafe {
        let path_bytes = core::slice::from_raw_parts(path_ptr, path_len as usize);
        let path = PathBuf::from(String::from_utf8_lossy(path_bytes).as_ref());

        let fs = get_filesystem();
        match fs.lookup_inode_by_path(&path) {
            Some(inode_num) => allocate_fd(inode_num),
            None => -1,  // ENOENT
        }
    }
}

/// Read from file descriptor
#[no_mangle]
pub extern "C" fn wasm_vfs_read(fd: i32, buf_ptr: *mut u8, buf_len: u32) -> i32 {
    unsafe {
        let fs = get_filesystem();
        let inode_num = get_inode_from_fd(fd);

        if let Some(data) = fs.files.get(&inode_num) {
            let read_len = core::cmp::min(buf_len as usize, data.len());
            core::ptr::copy_nonoverlapping(data.as_ptr(), buf_ptr, read_len);
            read_len as i32
        } else {
            -1  // EBADF
        }
    }
}

/// Write to file descriptor
#[no_mangle]
pub extern "C" fn wasm_vfs_write(fd: i32, buf_ptr: *const u8, buf_len: u32) -> i32 {
    unsafe {
        let fs = get_filesystem_mut();
        let inode_num = get_inode_from_fd(fd);

        let data = core::slice::from_raw_parts(buf_ptr, buf_len as usize);

        if let Some(existing) = fs.files.get_mut(&inode_num) {
            *existing = data.to_vec();
            buf_len as i32
        } else {
            -1  // EBADF
        }
    }
}

/// Get file status
#[no_mangle]
pub extern "C" fn wasm_vfs_stat(path_ptr: *const u8, path_len: u32, stat_buf: *mut Stat) -> i32 {
    unsafe {
        let path_bytes = core::slice::from_raw_parts(path_ptr, path_len as usize);
        let path = PathBuf::from(String::from_utf8_lossy(path_bytes).as_ref());

        let fs = get_filesystem();
        match fs.lookup_inode_by_path(&path) {
            Some(inode_num) => {
                let inode = &fs.inodes[inode_num as usize];
                write_stat(stat_buf, inode);
                0  // Success
            }
            None => -1,  // ENOENT
        }
    }
}
```

---

## 5. Browser and Edge Deployment

### Browser Integration

```html
<!DOCTYPE html>
<html>
<head>
    <title>WASM Box in Browser</title>
</head>
<body>
    <pre id="output"></pre>
    <script type="module">
        // Load WASM module
        const wasm = await WebAssembly.instantiateStreaming(
            fetch('box.wasm'),
            {
                env: {
                    // Provide imports
                    wasm_vfs_open: (pathPtr, pathLen) => {
                        // Browser implementation
                        return -1;  // Not implemented
                    },
                    wasm_vfs_read: (fd, bufPtr, bufLen) => {
                        // Browser implementation
                        return 0;
                    },
                    wasm_vfs_write: (fd, bufPtr, bufLen) => {
                        // Write to console
                        const data = new Uint8Array(memory.buffer, bufPtr, bufLen);
                        const text = new TextDecoder().decode(data);
                        document.getElementById('output').textContent += text;
                        return bufLen;
                    }
                }
            }
        );

        // Call _start
        wasm.instance.exports._start();
    </script>
</body>
</html>
```

### Edge Runtime (Cloudflare Workers)

```typescript
// worker.ts
export default {
    async fetch(request: Request): Promise<Response> {
        // Load WASM module
        const wasmModule = await WebAssembly.compile(WASM_BINARY);

        // Create instance with imports
        const instance = await WebAssembly.instantiate(wasmModule, {
            env: {
                memory: new WebAssembly.Memory({ initial: 256 }),
                wasm_vfs_open: (pathPtr: number, pathLen: number) => {
                    // Edge filesystem implementation
                    return -1;
                },
                wasm_vfs_read: (fd: number, bufPtr: number, bufLen: number) => {
                    return 0;
                },
                wasm_vfs_write: (fd: number, bufPtr: number, bufLen: number) => {
                    // Log to worker logs
                    const data = new Uint8Array(
                        (instance.exports.memory as WebAssembly.Memory).buffer,
                        bufPtr, bufLen
                    );
                    console.log(new TextDecoder().decode(data));
                    return bufLen;
                }
            }
        });

        // Execute
        (instance.exports._start as () => void)();

        return new Response('Box executed successfully');
    }
};
```

### WASI Compatibility Layer

For WASI targets, provide WASI shims:

```rust
// wasm-vfs/src/wasi_shim.rs

/// WASI fd_write implementation
#[no_mangle]
pub extern "C" fn fd_write(fd: i32, iovs_ptr: *const Iovec, iovs_len: i32, nwritten: *mut i32) -> i32 {
    unsafe {
        let iovs = core::slice::from_raw_parts(iovs_ptr, iovs_len as usize);
        let mut total_written = 0;

        for iov in iovs {
            let data = core::slice::from_raw_parts(iov.buf, iov.buf_len);

            if fd == 1 {  // STDOUT
                wasm_vfs_write(fd, data.as_ptr(), data.len() as u32);
            }

            total_written += iov.buf_len;
        }

        *nwritten = total_written;
        0  // Success
    }
}

#[repr(C)]
struct Iovec {
    buf: *const u8,
    buf_len: usize,
}
```

---

## 6. Complete Integration Example

### Building a Complete Box

```rust
// Complete boxer build flow
fn build_box(dockerfile_path: &Path) -> Result<Vec<u8>> {
    // 1. Parse Dockerfile
    let dockerfile_content = fs::read_to_string(dockerfile_path)?;
    let dockerfile = Dockerfile::parse(&dockerfile_content)?;

    // 2. Initialize builder
    let mut builder = Builder::new();

    // 3. Process instructions
    for stage in dockerfile.iter_stages() {
        for instruction in stage.instructions {
            match instruction {
                Instruction::From(instr) => {
                    // Set base WASM (Marcotte for libc)
                    builder.config_base(&instr.image.content);
                }
                Instruction::Copy(instr) => {
                    // Bundle files into VFS
                    let mut buffer = HashMap::new();
                    for src in &instr.sources {
                        let content = fs::read(src)?;
                        buffer.insert(instr.destination.clone(), content);
                    }
                    builder.bundle_fs_from_buffer(buffer);
                }
                Instruction::Cmd(instr) => {
                    // Set entrypoint
                    builder.set_entrypoint(&instr.command);
                }
                _ => {}  // Handle other instructions
            }
        }
    }

    // 4. Build and snapshot
    builder.build(true);

    // 5. Return WASM binary
    Ok(builder.finalize())
}
```

### Running the Box

```rust
// Run a built box
fn run_box(wasm_bytes: &[u8]) -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm_bytes)?;

    let mut linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());

    // Provide WASI-like imports
    linker.func_wrap("env", "wasm_vfs_open", |ptr: i32, len: i32| {
        // Host implementation
        Ok(0i32)
    })?;

    linker.func_wrap("env", "wasm_vfs_read", |fd: i32, ptr: i32, len: i32| {
        Ok(0i32)
    })?;

    linker.func_wrap("env", "wasm_vfs_write", |fd: i32, ptr: i32, len: i32| {
        Ok(len)
    })?;

    let instance = linker.instantiate(&mut store, &module)?;

    // Call entrypoint
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
    start.call(&mut store, ())?;

    Ok(())
}
```

---

## 7. Summary

### WASM Compatibility Checklist

| Requirement | Status | Notes |
|-------------|--------|-------|
| `wasm32-unknown-unknown` target | ✅ | Primary target |
| `#[no_mangle]` exports | ✅ | C ABI compatibility |
| `extern "C"` imports | ✅ | Host function calls |
| Linear memory management | ✅ | Offsets, not pointers |
| No std dependency | ✅ | `core` and `alloc` only |
| C FFI compatibility | ✅ | CStr, CString helpers |
| POSIX syscall interface | ✅ | open, read, write, stat |

### wasm-vfs Components

| Component | Purpose | Lines |
|-----------|---------|-------|
| `filesystem.rs` | Core FileSystem, Inode | 200 |
| `path/mod.rs` | PathBuf implementation | 90 |
| `collections/mod.rs` | Custom HashMap | 100 |
| `ffi/mod.rs` | C string helpers | 40 |
| `sync/mod.rs` | Spinlock Mutex | 75 |
| `lazy_static/mod.rs` | Lazy initialization | 25 |
| `cmp/mod.rs` | Utility functions | 10 |

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial deep dive creation |

---

*Continue to [Performance Deep Dive](03-performance-deep-dive.md) for optimization strategies.*
