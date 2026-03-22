---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/wasm-memory
repository: https://github.com/radu-matei/wasm-memory
explored_at: 2026-03-22
language: Rust, TypeScript, AssemblyScript, JavaScript
---

# Project Exploration: wasm-memory

## Overview

**wasm-memory** is a practical guide and reference implementation for understanding WebAssembly memory management. Created by Radu Matei, this project demonstrates how to manually manage memory when passing data between a host runtime (Node.js or Wasmtime) and a WebAssembly module.

### Key Value Proposition

- **Manual memory management** - Full control over allocation/deallocation
- **No bindgen overhead** - Direct WASM ABI without code generation tools
- **Educational reference** - Clear patterns for memory passing
- **Multi-runtime support** - Works with Node.js and Wasmtime
- **Language agnostic** - Examples in Rust and AssemblyScript

### Example Usage

```rust
// Module side (lib.rs) - Export alloc/dealloc and functions
#[no_mangle]
pub fn alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);  // Prevent destructor
    ptr  // Return pointer to host
}

#[no_mangle]
pub unsafe fn dealloc(ptr: *mut u8, size: usize) {
    let data = Vec::from_raw_parts(ptr, size, size);
    std::mem::drop(data);  // Reclaim and free
}

#[no_mangle]
pub unsafe fn array_sum(ptr: *mut u8, len: usize) -> u8 {
    let data = Vec::from_raw_parts(ptr, len, len);
    let sum = data.iter().sum();
    std::mem::forget(data);  // Host still owns this
    sum
}
```

```javascript
// Host side (test.js) - Allocate, copy, call, cleanup
const ptr = instance.exports.alloc(data.length);
const mem = new Uint8Array(instance.exports.memory.buffer, ptr, data.length);
mem.set(new Uint8Array(data));  // Copy to WASM memory

const result = instance.exports.array_sum(ptr, data.length);

instance.exports.dealloc(ptr, data.length);  // Cleanup
```

## Repository Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/wasm-memory/
├── src/
│   ├── lib.rs                     # WASM module exports (Rust)
│   └── main.rs                    # Native Rust test runner
├── index.ts                       # AssemblyScript WASM module
├── test.js                        # Node.js host runtime
├── Cargo.toml                     # Wasmtime host dependencies
├── package.json                   # AssemblyScript build config
├── rust.wasm                      # Compiled Rust WASM module
├── as.wasm                        # Compiled AssemblyScript WASM
└── readme.md                      # Build instructions
```

## Core Concepts

### 1. Linear Memory Model

WebAssembly exposes memory as a contiguous byte array:

```
┌─────────────────────────────────────────────────────────────────┐
│                    WASM Linear Memory                            │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │  Offset 0                                                  │ │
│  │  ┌─────────┬─────────┬─────────┬─────────┬─────────┐      │ │
│  │  │  buf[0] │  buf[1] │  buf[2] │  ...   │  buf[n] │      │ │
│  │  └─────────┴─────────┴─────────┴─────────┴─────────┘      │ │
│  │                                                            │ │
│  │  Guest pointer returned by alloc() ──────────────────┐    │ │
│  └──────────────────────────────────────────────────────┼────┘ │
│                                                         ▼      │
│  Host accesses via: new Uint8Array(memory.buffer, ptr, len)   │
└─────────────────────────────────────────────────────────────────┘
```

### 2. Memory Ownership Protocol

```
┌─────────────────────────────────────────────────────────────────┐
│              Data Passing Protocol                               │
│                                                                  │
│  1. Host allocates in guest memory:                              │
│     ptr = instance.exports.alloc(size)                           │
│                                                                  │
│  2. Host copies data to guest memory:                            │
│     mem = new Uint8Array(memory.buffer, ptr, size)               │
│     mem.set(data)                                                │
│                                                                  │
│  3. Host calls guest function:                                   │
│     result = instance.exports.func(ptr, size)                    │
│                                                                  │
│  4. Guest takes ownership via Vec::from_raw_parts():             │
│     let data = Vec::from_raw_parts(ptr, len, len);               │
│                                                                  │
│  5a. Guest keeps data: std::mem::forget()                        │
│  5b. Guest frees data: let _ = data; (automatic)                 │
│                                                                  │
│  6. Host deallocates if guest kept ownership:                    │
│     instance.exports.dealloc(ptr, size)                          │
└─────────────────────────────────────────────────────────────────┘
```

### 3. Allocation Pattern

**Rust Module (Guest):**

```rust
#[no_mangle]
pub fn alloc(len: usize) -> *mut u8 {
    // 1. Create Vec with exact capacity
    let mut buf = Vec::with_capacity(len);

    // 2. Get raw pointer to internal buffer
    let ptr = buf.as_mut_ptr();

    // 3. Prevent Vec destructor from running
    //    (memory would be freed immediately otherwise)
    std::mem::forget(buf);

    // 4. Return pointer for host to write data
    ptr
}
```

**Key insight:** `std::mem::forget()` prevents the `Vec` from freeing its buffer when it goes out of scope, leaving the memory available for the host to access.

### 4. Deallocation Pattern

```rust
#[no_mangle]
pub unsafe fn dealloc(ptr: *mut u8, size: usize) {
    // 1. Reconstruct Vec from raw parts
    //    This tells Rust: "we own this memory again"
    let data = Vec::from_raw_parts(ptr, size, size);

    // 2. When Vec goes out of scope, it frees the memory
    std::mem::drop(data);  // Explicit, but automatic when out of scope
}
```

### 5. Reading Data (Guest Side)

```rust
#[no_mangle]
pub unsafe fn array_sum(ptr: *mut u8, len: usize) -> u8 {
    // Reconstruct Vec - takes ownership from host
    let data = Vec::from_raw_parts(ptr, len, len);

    // Process data
    let sum = data.iter().sum();

    // Option A: We're done, let Vec free the memory
    // (just let data go out of scope)

    // Option B: Host still needs the data
    std::mem::forget(data);

    sum
}
```

### 6. Writing Data (Guest Side)

```rust
#[no_mangle]
pub unsafe fn upper(ptr: *mut u8, len: usize) -> *mut u8 {
    // Read input string (host-owned memory)
    let data = Vec::from_raw_parts(ptr, len, len);
    let input_str = String::from_utf8(data).unwrap();

    // Create output
    let mut upper = input_str.to_ascii_uppercase().as_bytes().to_owned();

    // Get pointer to return
    let ptr = upper.as_mut_ptr();

    // Forget so host can read it
    std::mem::forget(upper);

    // Return pointer for host to read
    ptr
}
```

## Host Runtime Implementations

### Node.js Host

```javascript
const fs = require("fs");

async function main() {
    // Load WASM module
    const module_bytes = fs.readFileSync("./rust.wasm");
    const mod = new WebAssembly.Module(module_bytes);
    const instance = await WebAssembly.instantiate(mod, {});

    // Call functions
    arraySum([1, 2, 3, 4, 5], instance);
    upper("hello world", instance);
}

function copyMemory(data, instance) {
    // Get allocation offset
    const ptr = instance.exports.alloc(data.length);

    // Create typed view into WASM memory
    const mem = new Uint8Array(
        instance.exports.memory.buffer,
        ptr,
        data.length
    );

    // Copy data
    mem.set(new Uint8Array(data));

    return ptr;
}

function readString(ptr, len, instance) {
    const mem = new Uint8Array(instance.exports.memory.buffer, ptr, len);
    const decoder = new TextDecoder("utf-8");
    return decoder.decode(mem);
}

function upper(input, instance) {
    const bytes = new TextEncoder("utf-8").encode(input);
    const ptr = copyMemory(bytes, instance);

    // Get result pointer
    const res_ptr = instance.exports.upper(ptr, bytes.length);

    // Read result
    const result = readString(res_ptr, bytes.length, instance);
    console.log(result);

    // Guest returned ownership to us, we must deallocate
    instance.exports.dealloc(res_ptr, bytes.length);
}
```

### Wasmtime Host (Rust)

```rust
use wasmtime::*;
use wasmtime_wasi::{Wasi, WasiCtxBuilder};

fn create_instance(filename: String) -> Result<Instance, anyhow::Error> {
    let store = Store::default();
    let mut linker = Linker::new(&store);

    // Set up WASi context
    let ctx = WasiCtxBuilder::new()
        .inherit_stdin()
        .inherit_stdout()
        .inherit_stderr()
        .build()?;

    let wasi = Wasi::new(&store, ctx);
    wasi.add_to_linker(&mut linker)?;

    let module = wasmtime::Module::from_file(store.engine(), filename)?;
    let instance = linker.instantiate(&module)?;

    Ok(instance)
}

fn copy_memory(bytes: &Vec<u8>, instance: &Instance) -> Result<isize, anyhow::Error> {
    let memory = instance
        .get_memory("memory")
        .expect("expected memory not found");

    let alloc = instance
        .get_func("alloc")
        .expect("expected alloc function not found");

    // Allocate in guest memory
    let alloc_result = alloc.call(&vec![Val::from(bytes.len() as i32)])?;

    let guest_ptr_offset = match alloc_result.get(0) {
        Val::I32(val) => *val as isize,
        _ => return Err(anyhow::Error::msg("guest_ptr must be Val::I32")),
    };

    // Copy bytes directly into guest memory
    unsafe {
        let raw = memory.data_ptr().offset(guest_ptr_offset);
        raw.copy_from(bytes.as_ptr(), bytes.len());
    }

    Ok(guest_ptr_offset)
}

fn upper(input: String) -> Result<String, anyhow::Error> {
    let instance = create_instance("rust.wasm".to_string())?;

    // Write input to guest memory
    let ptr = copy_memory(&input.as_bytes().to_vec(), &instance)?;

    // Get exported function
    let upper = instance
        .get_func("upper")
        .expect("expected upper function not found");

    // Call with (ptr, len)
    let results = upper.call(&vec![
        Val::from(ptr as i32),
        Val::from(input.as_bytes().len() as i32),
    ])?;

    // Get result pointer
    let res_ptr = match results.get(0) {
        Val::I32(val) => *val,
        _ => return Err(anyhow::Error::msg("cannot get result")),
    };

    // Read result string
    let memory = instance
        .get_memory("memory")
        .expect("expected memory not found");

    let res: String;
    unsafe {
        res = read_string(&memory, res_ptr as u32, input.as_bytes().len() as u32).unwrap();
    }

    // Deallocate result memory
    let dealloc = instance.get_func("dealloc").unwrap();
    dealloc.call(&vec![
        Val::from(res_ptr as i32),
        Val::from(input.as_bytes().len() as i32),
    ])?;

    Ok(res)
}
```

### AssemblyScript Module

```typescript
// index.ts - AssemblyScript WASM module

export function alloc(len: i32): usize {
    // Allocate array in WASM memory
    let buf = new Array<u8>(len);

    // Get pointer to array data
    let buf_ptr = memory.data(8);

    // Store array at that location
    store<Array<u8>>(buf_ptr, buf);

    return buf_ptr;
}

export function array_sum(buf_ptr: usize, len: i32): u8 {
    let result: u8 = 0;

    // Direct memory access
    for (let i = 0; i < len; i++) {
        result += load<u8>(buf_ptr + i) as u8;
    }

    return result as u8;
}

// AssemblyScript requires abort handler
export function abort(
    message: string | null,
    fileName: string | null,
    lineNumber: u32,
    columnNumber: u32
): void {}
```

## Memory Safety Considerations

### Ownership Rules

```rust
// RULE 1: When guest allocates, host must deallocate
let ptr = instance.exports.alloc(size);  // Guest allocs
// ... use ptr ...
instance.exports.dealloc(ptr, size);     // Host deallocs

// RULE 2: When host passes data, guest can take ownership
let ptr = copy_memory(&data, &instance);
instance.exports.process(ptr, size);     // Guest takes ownership via from_raw_parts
// Host must NOT access ptr after this!

// RULE 3: When guest returns data, host must deallocate
let res_ptr = instance.exports.transform(ptr, size);  // Guest forgets
// ... read result ...
instance.exports.dealloc(res_ptr, size);  // Host cleans up
```

### Common Pitfalls

**Double Free:**
```rust
// BAD: Guest frees, then host tries to free again
#[no_mangle]
pub unsafe fn bad_process(ptr: *mut u8, len: usize) {
    let data = Vec::from_raw_parts(ptr, len, len);
    std::mem::drop(data);  // Frees memory
    // ptr is now invalid!
}
// Host calling dealloc after this = double free
```

**Memory Leak:**
```rust
// BAD: Host allocates but never deallocates
let ptr = instance.exports.alloc(size);
// ... use it ...
// Forgot to call instance.exports.dealloc(ptr, size)!
```

**Use After Free:**
```rust
// BAD: Accessing memory after giving ownership
let ptr = copy_memory(&data, &instance);
instance.exports.take_ownership(ptr, size);  // Guest now owns
// BAD: Still trying to read from ptr
let result = read_from_memory(ptr, size);    // Undefined behavior!
```

## Comparison with Other Approaches

### Manual Memory (wasm-memory) vs wasm-bindgen

| Aspect | wasm-memory | wasm-bindgen |
|--------|-------------|--------------|
| Control | Full manual control | Automatic |
| Boilerplate | High (alloc/dealloc everywhere) | Low (transparent) |
| Performance | Minimal overhead | Small wrapper overhead |
| Learning | Teaches WASM memory model | Abstracts it away |
| Safety | Manual, error-prone | Type-safe guarantees |
| Bundle size | Minimal | Includes runtime |

### When to Use Manual Memory Management

1. **Learning** - Understanding WASM memory model deeply
2. **Maximum control** - Fine-tuning allocation strategies
3. **Minimal runtime** - Avoiding bindgen overhead
4. **Custom allocators** - Implementing arena allocators, pools
5. **Interop** - Working with existing C/Rust FFI code

## Build System

### Rust WASM Module

```bash
# Compile Rust to WASM
rustc --target wasm32-unknown-unknown --crate-type=cdylib \
    src/lib.rs -o rust.wasm
```

### AssemblyScript WASM Module

```bash
# Install dependencies
npm install

# Build AssemblyScript
npm run asbuild
```

### Running Tests

```bash
# Node.js runtime
node test.js

# Wasmtime runtime
cargo run

# Native Rust (for debugging without WASM)
rustc src/lib.rs -o mem
./mem

# Valgrind for memory leaks
valgrind ./mem
```

## Trade-offs

| Aspect | Benefit | Cost |
|--------|---------|------|
| Manual alloc/dealloc | Full control | Easy to leak or double-free |
| No bindgen | Minimal overhead | More boilerplate |
| Raw pointers | Direct memory access | Unsafe code required |
| Vec::from_raw_parts | Zero-copy reads | Must track ownership carefully |
| std::mem::forget | Prevent premature free | Can cause leaks if misused |

## Related Projects in Source Directory

- **articulated** - CRDT-based sequence data structures (also uses manual memory patterns)
- **fastcrdt** - Fast CRDT implementations with similar memory concerns

## References

- Original article: [A practical guide to WebAssembly memory](https://radu-matei.com/blog/practical-guide-to-wasm-memory)
- Radu Matei's GitHub: https://github.com/radu-matei
- WebAssembly Memory API: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/WebAssembly/Memory
