---
title: "Boxing Patterns Deep Dive"
subtitle: "Type boxing, value wrappers, trait objects, and ownership patterns in Rust"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/boxer/01-boxing-patterns-deep-dive.md
related_to: ./exploration.md
created: 2026-03-27
status: complete
---

# Boxing Patterns Deep Dive

## Executive Summary

This deep dive covers the complete landscape of boxing patterns in Rust and how Boxer applies them:

1. **Type Boxing Fundamentals** - From `Box<T>` to `Box<dyn Trait>`
2. **Value Wrappers** - Newtype patterns and transparent wrappers
3. **Trait Objects** - Dynamic dispatch and vtables
4. **Box vs. Rc vs. Arc** - Choosing the right pointer
5. **Type Erasure Patterns** - Hiding complexity behind interfaces

---

## 1. Type Boxing Fundamentals

### The `Box<T>` Type

`Box<T>` is Rust's **heap-allocated smart pointer**:

```rust
use std::boxed::Box;

// Stack allocation (default)
let stack_val = 42;  // Direct value on stack

// Heap allocation with Box
let heap_val: Box<i32> = Box::new(42);  // Pointer to heap

// Access value (dereference)
println!("{}", *heap_val);  // Prints: 42
```

**Memory layout:**

```
Stack:                Heap:
┌─────────────┐      ┌─────────────┐
│ heap_val    │─────>│ 42          │
│ (pointer)   │      │ (actual i32)│
└─────────────┘      └─────────────┘
```

### When to Use `Box<T>`

| Scenario | Pattern | Example |
|----------|---------|---------|
| **Recursive types** | `Box<Self>` | Linked lists, trees |
| **Trait objects** | `Box<dyn Trait>` | Polymorphism |
| **Large data** | `Box<LargeStruct>` | Reduce stack usage |
| **Transfer ownership** | `Box<T>` without cloning | Move semantics |

### Recursive Types Example

```rust
// Cannot compile without Box:
// enum List {
//     Cons(i32, List),  // Error: infinite size
//     Nil,
// }

// Correct with Box:
enum List {
    Cons(i32, Box<List>),  // Box makes size finite
    Nil,
}

// Usage:
let list = List::Cons(1,
    Box::new(List::Cons(2,
        Box::new(List::Cons(3,
            Box::new(List::Nil)
        ))
    ))
);
```

### How Boxer Uses `Box<T>`

In **wasm-vfs**, `Box` is used for the inode tree:

```rust
// filesystem.rs
pub struct Inode {
    pub number: u64,
    pub kind: InodeKind,
    // ... other fields
}

// Directory entries point to child inodes via Box
pub struct DirectoryEntry {
    name: String,
    inode: Box<Inode>,  // Heap-allocated, recursive structure
}
```

---

## 2. Value Wrappers and Newtype Patterns

### The Newtype Pattern

A **newtype** wraps a type to add semantics:

```rust
// Wrapper type with zero runtime cost
struct UserId(u64);
struct ProductId(u64);

// Prevents mixing up IDs at compile time
fn get_user(id: UserId) { ... }
fn get_product(id: ProductId) { ... }

let user_id = UserId(123);
let product_id = ProductId(456);

get_user(user_id);        // OK
get_user(product_id);     // Compile error!
```

### Transparent Wrappers with `#[repr(transparent)]`

```rust
// Guarantees same memory layout as inner type
#[repr(transparent)]
pub struct PathBuf {
    inner: String,
}

// Can be safely cast to/from String in FFI
```

### Boxer's PathBuf Wrapper

```rust
// wasm-vfs/src/path/mod.rs
#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct PathBuf {
    inner: String,
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

**Why wrap String?**
- Add domain-specific methods (`is_absolute`, `join`)
- Prevent accidental use of raw strings
- Serialization control
- FFI compatibility

---

## 3. Trait Objects and Dynamic Dispatch

### Static vs. Dynamic Dispatch

```rust
// Static dispatch (monomorphization)
fn process<T: Display>(value: T) {
    println!("{}", value);
}
// Compiler generates separate code for each T

// Dynamic dispatch (trait object)
fn process(value: &dyn Display) {
    println!("{}", value);
}
// Single code path, runtime lookup via vtable
```

### Trait Object Memory Layout

```rust
trait Animal {
    fn speak(&self) -> &str;
}

struct Dog { name: String }
struct Cat { lives: i32 }

impl Animal for Dog {
    fn speak(&self) -> &str { "Woof!" }
}

impl Animal for Cat {
    fn speak(&self) -> &str { "Meow!" }
}

// Trait object: fat pointer
let dog: Box<dyn Animal> = Box::new(Dog { name: "Rex".into() });
```

**Memory layout of `Box<dyn Animal>`:**

```
┌────────────────────────┐
│ Box<dyn Animal>        │
│ ┌─────────────────┐    │
│ │ data_ptr        │────┼──> Dog { name: "Rex" }
│ │ vtable_ptr      │────┼──> vtable [speak, drop, size]
│ └─────────────────┘    │
└────────────────────────┘
```

**Vtable contains:**
- Function pointers for trait methods
- Drop implementation
- Size/alignment info

### Boxer's Trait Object Usage

```rust
// wasm-vfs abstracts filesystem operations behind traits
pub trait FileSystemOps {
    fn open(&mut self, path: &PathBuf) -> Result<FileDescriptor>;
    fn read(&mut self, fd: FileDescriptor) -> Result<Vec<u8>>;
    fn write(&mut self, fd: FileDescriptor, data: &[u8]) -> Result<()>;
}

// In-memory implementation
pub struct InMemoryFS {
    files: HashMap<u64, Vec<u8>>,
    path_map: HashMap<PathBuf, u64>,
}

impl FileSystemOps for InMemoryFS {
    fn open(&mut self, path: &PathBuf) -> Result<FileDescriptor> {
        // Lookup in path_map
    }

    fn read(&mut self, fd: FileDescriptor) -> Result<Vec<u8>> {
        // Read from files HashMap
    }
}

// Type-erased usage in builder
pub struct Builder {
    fs: Box<dyn FileSystemOps>,  // Can swap implementations
}
```

---

## 4. Box vs. Rc vs. Arc

### Choosing the Right Pointer

| Pointer | Ownership | Thread-Safe | Use Case |
|---------|-----------|-------------|----------|
| `Box<T>` | Single | No | Heap allocation, trait objects |
| `Rc<T>` | Shared (ref-counted) | No | Shared ownership, single-threaded |
| `Arc<T>` | Shared (ref-counted) | Yes | Shared ownership, multi-threaded |

### `Box<T>` - Single Ownership

```rust
let boxed = Box::new(42);
let moved = boxed;  // Ownership transferred
// boxed is now invalid
```

### `Rc<T>` - Reference Counted (Single-Threaded)

```rust
use std::rc::Rc;

let rc1: Rc<i32> = Rc::new(42);
let rc2 = Rc::clone(&rc1);  // Increment ref count
let rc3 = rc1.clone();      // Also increments

println!("Ref count: {}", Rc::strong_count(&rc1));  // 3
```

**Memory layout:**

```
┌────────────────────────┐
│ Rc<T>                  │
│ ┌─────────────────┐    │
│ │ rc_ptr          │────┼──> ┌──────────────┐
│ └─────────────────┘    │    │ Ref count: 3 │
│                        │    │ Data: 42     │
└────────────────────────┘    └──────────────┘
```

### `Arc<T>` - Atomic Reference Counted (Multi-Threaded)

```rust
use std::sync::Arc;
use std::thread;

let arc: Arc<i32> = Arc::new(42);
let arc_clone = Arc::clone(&arc);

let handle = thread::spawn(move || {
    println!("From thread: {}", arc_clone);
});

handle.join().unwrap();
println!("From main: {}", arc);
```

### How Boxer Chooses

**wasm-vfs uses custom HashMap (no std):**

```rust
// wasm-vfs/src/collections/mod.rs
pub struct HashMap<K, V, const CAP: usize> {
    entries: Vec<Option<(K, V)>>,
}

// Single ownership is sufficient for in-memory FS
// No need for Rc/Arc in single-threaded WASM context
```

**For multi-threaded scenarios (future):**
```rust
// Would use Arc for shared state across threads
use std::sync::Arc;

pub struct SharedFS {
    files: Arc<Mutex<HashMap<u64, Vec<u8>>>>,
}
```

---

## 5. Type Erasure Patterns

### Pattern 1: Trait Objects (Runtime Erasure)

```rust
// Erase concrete type behind trait
trait Processor {
    fn process(&self, data: &[u8]) -> Vec<u8>;
}

struct Compressor;
struct Encryptor;

impl Processor for Compressor { ... }
impl Processor for Encryptor { ... }

// Type-erased collection
let pipeline: Vec<Box<dyn Processor>> = vec![
    Box::new(Compressor),
    Box::new(Encryptor),
];
```

### Pattern 2: Enums (Compile-Time Sum Types)

```rust
// Enum preserves type information at compile time
enum Processor {
    Compress(Compressor),
    Encrypt(Encryptor),
}

impl Processor {
    fn process(&self, data: &[u8]) -> Vec<u8> {
        match self {
            Processor::Compress(c) => c.compress(data),
            Processor::Encrypt(e) => e.encrypt(data),
        }
    }
}
```

**Comparison:**

| Aspect | Trait Object | Enum |
|--------|-------------|------|
| Dispatch | Runtime (vtable) | Compile-time (match) |
| Extensibility | Add new impls easily | Must modify enum |
| Performance | Virtual call overhead | Direct call |
| Size | Fat pointer (2 words) | Size of largest variant |

### Pattern 3: Opaque Types (Module Boundaries)

```rust
// In library:
mod internal {
    pub struct Handle {
        pub id: u64,
        pub state: ComplexState,
        // ... internal details
    }
}

// Public API:
pub struct Handle(internal::Handle);

impl Handle {
    pub fn new() -> Self {
        Handle(internal::Handle { ... })
    }

    pub fn use(&self) {
        // Internal details hidden
    }
}
```

### Pattern 4: Existential Types (Associated Type Erasure)

```rust
// Erase associated types
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

// Without erasure:
fn process<I: Iterator<Item = i32>>(iter: I) { ... }

// With erasure:
fn process(iter: Box<dyn Iterator<Item = i32>>) { ... }
```

---

## 6. Boxing in Boxer: Complete Examples

### Example 1: File Descriptor Boxing

```rust
// wasm-vfs/src/filesystem.rs

// File descriptor is a boxed abstraction
pub struct FileDescriptor {
    pub inode_number: u64,
    pub position: u64,
    pub mode: FileMode,
}

// Used to erase concrete file type
pub trait FileHandle {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;
}

// Regular file implementation
struct RegularFile {
    inode: Inode,
    data: Vec<u8>,
    pos: u64,
}

impl FileHandle for RegularFile { ... }

// Directory implementation (special handling)
struct Directory {
    inode: Inode,
    entries: Vec<DirectoryEntry>,
}

impl FileHandle for Directory { ... }

// Type-erased usage
pub fn open_file(fs: &mut FileSystem, path: &PathBuf) -> Box<dyn FileHandle> {
    let inode = fs.lookup(path);
    match inode.kind {
        InodeKind::File => Box::new(RegularFile { ... }),
        InodeKind::Directory => Box::new(Directory { ... }),
        InodeKind::SymbolicLink(target) => {
            // Recursively resolve
            open_file(fs, &target)
        }
    }
}
```

### Example 2: Builder Pattern with Boxing

```rust
// boxer/box/src/builder/builder.rs

pub struct Builder {
    base_build: Vec<u8>,
    working_directory: PathBuf,
    copied_files: Vec<(String, Vec<u8>)>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            base_build: Vec::new(),
            working_directory: PathBuf::from("/"),
            copied_files: Vec::new(),
        }
    }

    pub fn config_base(&mut self, base: &str) {
        match base {
            "scratch" => {
                // Load wizered WASM
                self.base_build = fs::read("output_wizered.wasm")
                    .unwrap_or_default();
            }
            _ => {}
        }
    }

    pub fn bundle_fs_from_buffer(&mut self, buffer: HashMap<String, Vec<u8>>) {
        for (path, content) in buffer {
            self.copied_files.push((path, content));
        }
    }

    pub fn build(&mut self, wasm_only: bool) {
        // 1. Compile base WASM
        let engine = Engine::default();
        let module = Module::new(&engine, &self.base_build);

        // 2. Get exported memory
        let instance = linker.instantiate(&mut store, &module);
        let memory = instance.get_memory(&mut store, "memory");

        // 3. Build FileDef array
        let filedef_structs = self.build_filedefs();

        // 4. Copy to guest memory
        self.copy_to_guest(memory, &filedef_structs);

        // 5. Call wasm_vfs_mount_in_memory
        let mount_func = instance.get_func(&mut store, "wasm_vfs_mount_in_memory");
        mount_func.call(&mut store, &[count, ptr]);
    }
}
```

### Example 3: VFS Mounting with Boxing

```rust
// Builder bundles files into WASM memory

#[repr(C)]
pub struct FileDef {
    pub path_off: u32,    // Offset to path string
    pub data_off: u32,    // Offset to file data
    pub data_len: u32,    // Length of file data
}

// In Builder::build():
for (dest_path, data) in &self.copied_files {
    // 1. Convert path to null-terminated bytes
    let mut path_bytes = dest_path.as_bytes().to_vec();
    path_bytes.push(0);

    // 2. Copy path to guest memory
    let path_offset = guest_cursor;
    memory.write(&mut store, path_offset, &path_bytes);
    guest_cursor += path_bytes.len();

    // 3. Copy data to guest memory
    let data_offset = guest_cursor;
    memory.write(&mut store, data_offset, data);
    guest_cursor += data.len();

    // 4. Build FileDef struct
    filedef_structs[base..base+4]
        .copy_from_slice(&(path_offset as u32).to_le_bytes());
    filedef_structs[base+4..base+8]
        .copy_from_slice(&(data_offset as u32).to_le_bytes());
    filedef_structs[base+8..base+12]
        .copy_from_slice(&(data.len() as u32).to_le_bytes());
}

// 5. Call wasm_vfs_mount_in_memory(count, array_ptr)
typed_mount.call(&mut store, (count_u32, ptr))?;
```

---

## 7. Advanced Boxing Patterns

### Pattern: Boxing for Recursion with State

```rust
// Recursive computation with boxed state
enum Task {
    Sequential(Vec<Box<Task>>),
    Parallel(Vec<Box<Task>>),
    Compute(Box<dyn Fn() -> i32 + Send>),
}

impl Task {
    fn execute(&self) -> i32 {
        match self {
            Task::Sequential(tasks) => {
                tasks.iter().map(|t| t.execute()).sum()
            }
            Task::Parallel(tasks) => {
                // Would spawn threads here
                tasks.iter().map(|t| t.execute()).sum()
            }
            Task::Compute(f) => f(),
        }
    }
}
```

### Pattern: Boxing for Plugin Systems

```rust
// Plugin trait
trait Plugin {
    fn name(&self) -> &str;
    fn initialize(&mut self) -> Result<()>;
    fn process(&mut self, event: &Event) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
}

// Plugin registry with type-erased plugins
struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginRegistry {
    fn register(&mut self, plugin: impl Plugin + 'static) {
        self.plugins.push(Box::new(plugin));
    }

    fn initialize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.initialize()?;
        }
        Ok(())
    }
}
```

### Pattern: Boxing for Strategy Pattern

```rust
// Strategy trait
trait CompressionStrategy {
    fn compress(&self, data: &[u8]) -> Vec<u8>;
    fn decompress(&self, data: &[u8]) -> Vec<u8>;
}

// Concrete strategies
struct GzipStrategy;
struct ZstdStrategy;
struct LzmaStrategy;

impl CompressionStrategy for GzipStrategy { ... }
impl CompressionStrategy for ZstdStrategy { ... }
impl CompressionStrategy for LzmaStrategy { ... }

// Context with boxed strategy
struct Compressor {
    strategy: Box<dyn CompressionStrategy>,
}

impl Compressor {
    fn new(strategy: impl CompressionStrategy + 'static) -> Self {
        Self {
            strategy: Box::new(strategy),
        }
    }

    fn compress(&self, data: &[u8]) -> Vec<u8> {
        self.strategy.compress(data)
    }

    fn set_strategy(&mut self, strategy: impl CompressionStrategy + 'static) {
        self.strategy = Box::new(strategy);
    }
}
```

---

## 8. Performance Considerations

### Boxing Overhead

| Operation | Stack | Box<T> | Box<dyn Trait> |
|-----------|-------|--------|----------------|
| Allocation | N/A | Heap alloc | Heap alloc |
| Access | Direct | One indirection | Two indirections |
| Call | Direct | Direct | Virtual (vtable) |
| Size | sizeof(T) | sizeof(pointer) | 2 * sizeof(pointer) |

### When to Avoid Boxing

```rust
// DON'T: Boxing when not needed
fn process(value: Box<i32>) {  // Unnecessary!
    println!("{}", *value);
}

// DO: Use reference
fn process(value: &i32) {
    println!("{}", value);
}

// DON'T: Boxing small values
let small: Box<u8> = Box::new(42);  // Overhead > value

// DO: Stack allocation for small values
let small: u8 = 42;
```

### Boxing for Performance

```rust
// DO: Boxing to reduce stack usage
struct LargeStruct {
    data: [u8; 10000],
}

fn process() {
    let large = Box::new(LargeStruct { data: [0; 10000] });
    // Stack usage: 8 bytes (pointer) instead of 10KB
}

// DO: Boxing for recursive types
enum Tree {
    Node(Box<[Tree]>),  // Heap-allocated children
    Leaf(i32),
}
```

---

## 9. Summary

### Boxing Patterns Reference

| Pattern | When to Use | Example |
|---------|-------------|---------|
| `Box<T>` | Heap allocation, trait objects | `Box<dyn FileSystem>` |
| `Rc<T>` | Shared ownership (single-threaded) | `Rc<RefCell<T>>` |
| `Arc<T>` | Shared ownership (multi-threaded) | `Arc<Mutex<T>>` |
| Newtype | Add semantics, type safety | `struct UserId(u64)` |
| Trait object | Runtime polymorphism | `Box<dyn Processor>` |
| Opaque type | Hide implementation | `pub struct Handle(internal::Handle)` |

### Boxer-Specific Patterns

| Component | Boxing Pattern | Purpose |
|-----------|---------------|---------|
| wasm-vfs | `Box<dyn FileSystemOps>` | Swappable FS backends |
| Builder | `Vec<(String, Vec<u8>)>` | File bundling |
| PathBuf | Newtype around String | Domain-specific path handling |
| Inode | Direct struct (no boxing) | Known size, no recursion |

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial deep dive creation |

---

*This document covers boxing patterns. Continue to [WASM Compatibility Deep Dive](02-wasm-compatibility-deep-dive.md) for WASM-specific patterns.*
