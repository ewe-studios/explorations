---
name: wasm-memory
description: Practical WebAssembly memory management guide demonstrating manual allocation, data passing, and ownership patterns between host and guest
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/wasm-memory/
---

# wasm-memory - WebAssembly Memory Management Deep Dive

## Overview

**wasm-memory** is a practical educational project by Radu Matei that demonstrates manual WebAssembly memory management. It shows how to pass data between a host runtime (Node.js or Wasmtime) and a WebAssembly module without using bindgen or other code generation tools.

### Key Value Proposition

- **Manual memory management** - Full control over allocation and deallocation
- **No bindgen overhead** - Direct WASM ABI without code generation
- **Educational reference** - Clear patterns for memory passing
- **Multi-runtime support** - Works with Node.js and Wasmtime
- **Multi-language** - Examples in Rust and AssemblyScript
- **Zero-copy reads** - Direct memory access from host

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
console.log(`Result: ${result}`);
```

## Directory Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/wasm-memory/
├── src/
│   ├── lib.rs                      # WASM module exports (Rust)
│   └── main.rs                     # Native Rust test runner
├── index.ts                        # AssemblyScript WASM module
├── test.js                         # Node.js host runtime
├── Cargo.toml                      # Wasmtime host dependencies
├── package.json                    # AssemblyScript build config
├── rust.wasm                       # Compiled Rust WASM module
├── as.wasm                         # Compiled AssemblyScript WASM
└── readme.md                       # Build instructions
```

## Core Concepts

### 1. Linear Memory Model

WebAssembly exposes memory as a contiguous byte array (Linear Memory):

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
│                    Memory Ownership Flow                         │
│                                                                 │
│  Host Runtime                    WASM Module                    │
│       │                               │                         │
│       │  1. alloc(len) ─────────────► │  Returns pointer       │
│       │◄───────────────────────────── │  (e.g., 1048576)       │
│       │                               │                         │
│       │  [Write data to memory]       │                         │
│       │  memory.buffer[ptr...ptr+len] │                         │
│       │                               │                         │
│       │  2. function(ptr, len) ──────►│  Reads from memory     │
│       │◄───────────────────────────── │  Returns result        │
│       │                               │                         │
│       │  [Read result from memory]    │                         │
│       │                               │                         │
│       │  3. dealloc(ptr, len) ───────►│  Frees memory          │
│       │                               │                         │
└─────────────────────────────────────────────────────────────────┘
```

### 3. Allocation Pattern

**Rust Module (`lib.rs`):**

```rust
/// Allocate memory into the module's linear memory
/// and return the offset to the start of the block.
#[no_mangle]
pub fn alloc(len: usize) -> *mut u8 {
    // Create a new mutable buffer with capacity `len`
    let mut buf = Vec::with_capacity(len);

    // Take a mutable pointer to the buffer
    let ptr = buf.as_mut_ptr();

    // Take ownership of the memory block and
    // ensure its destructor is not called when
    // the object goes out of scope at the end of the function
    std::mem::forget(buf);

    // Return the pointer so the runtime can write data at this offset
    ptr
}

#[no_mangle]
pub unsafe fn dealloc(ptr: *mut u8, size: usize) {
    // Reconstruct the Vec from raw parts
    let data = Vec::from_raw_parts(ptr, size, size);
    // Drop it to free the memory
    std::mem::drop(data);
}
```

**Key Safety Considerations:**

```rust
// SAFE: We forget the Vec, but the host will write data
// and then call a function that takes ownership
#[no_mangle]
pub unsafe fn array_sum(ptr: *mut u8, len: usize) -> u8 {
    // Reconstruct Vec to read the data
    let data = Vec::from_raw_parts(ptr, len, len);

    // Compute the sum
    let sum = data.iter().sum();

    // FORGET the Vec - host still owns this memory!
    // If we drop it here, the host's pointer becomes invalid
    std::mem::forget(data);

    sum
}

// UNSAFE if host uses pointer after this!
#[no_mangle]
pub unsafe fn array_sum_owned(ptr: *mut u8, len: usize) -> u8 {
    let data = Vec::from_raw_parts(ptr, len, len);
    let sum = data.iter().sum();
    // Vec drops here - memory freed
    sum  // Host pointer is now dangling!
}
```

### 4. Host-Side Memory Access (Node.js)

```javascript
// test.js - Complete host runtime

const fs = require("fs");
const module_bytes = fs.readFileSync("./rust.wasm");

(async () => {
  // Instantiate the module
  const mod = new WebAssembly.Module(module_bytes);
  const instance = await WebAssembly.instantiate(mod, {});

  // Test array sum
  arraySum([1, 2, 3, 4, 5], instance);

  // Test string transformation
  upper("this should be uppercase", instance);
})();

// Copy data into module memory and call array_sum
function arraySum(array, instance) {
  // 1. Allocate memory in WASM
  var ptr = instance.exports.alloc(array.length);

  // 2. Create typed view into WASM memory
  var mem = new Uint8Array(
    instance.exports.memory.buffer,
    ptr,
    array.length
  );

  // 3. Copy data from JS to WASM
  mem.set(new Uint8Array(array));

  // 4. Call the function
  var res = instance.exports.array_sum(ptr, array.length);
  console.log(`Result: ${res}`);  // 15

  // No dealloc needed - array_sum doesn't consume the memory
}

// String transformation with return value
function upper(input, instance) {
  // 1. Encode string to UTF-8 bytes
  var bytes = new TextEncoder("utf-8").encode(input);

  // 2. Copy to WASM memory
  var ptr = copyMemory(bytes, instance);

  // 3. Call function that returns NEW pointer
  var res_ptr = instance.exports.upper(ptr, bytes.length);

  // 4. Read result from WASM memory
  var result = readString(res_ptr, bytes.length, instance);
  console.log(result);  // "THIS SHOULD BE UPPERCASE"

  // 5. WE own this memory - must deallocate
  deallocGuestMemory(res_ptr, bytes.length, instance);
}

function copyMemory(data, instance) {
  var ptr = instance.exports.alloc(data.length);
  var mem = new Uint8Array(instance.exports.memory.buffer, ptr, data.length);
  mem.set(new Uint8Array(data));
  return ptr;
}

function readString(ptr, len, instance) {
  var m = new Uint8Array(instance.exports.memory.buffer, ptr, len);
  var decoder = new TextDecoder("utf-8");
  return decoder.decode(m.slice(0, len));
}

function deallocGuestMemory(ptr, len, instance) {
  instance.exports.dealloc(ptr, len);
}
```

### 5. AssemblyScript Implementation

```typescript
// index.ts - AssemblyScript version

// Import memory management from AssemblyScript runtime
export function alloc(len: i32): usize {
    // Allocate array and return pointer to memory
    let buf = new Array<u8>(len);
    let buf_ptr = memory.data(8);  // Get pointer to array metadata
    store<Array<u8>>(buf_ptr, buf);
    return buf_ptr;  // Return pointer for host to write
}

export function array_sum(buf_ptr: usize, len: i32): u8 {
    let result: u8 = 0;
    // Direct memory access using load intrinsic
    for(let i = 0; i < len; i++) {
        result += load<u8>(buf_ptr + i) as u8;
    }
    return result as u8;
}

// Required by AssemblyScript runtime
export function abort(
    message: string | null,
    fileName: string | null,
    lineNumber: u32,
    columnNumber: u32
): void {}
```

### 6. Wasmtime Host Runtime (Rust)

```rust
// Cargo.toml
[dependencies]
wasmtime = "0.22"
wasmtime-wasi = "0.22"
wasi-common = "0.22"
tokio = { version = "0.2", features = ["full"] }
anyhow = "1.0"

// main.rs - Wasmtime host
use anyhow::Result;
use std::slice;
use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let store = Store::new(&engine);

    // Load module
    let module = Module::from_file(&engine, "rust.wasm")?;

    // Create instance
    let instance = Instance::new(&store, &module, &[])?;

    // Get exports
    let alloc = instance.get_func("alloc").unwrap();
    let dealloc = instance.get_func("dealloc").unwrap();
    let array_sum = instance.get_func("array_sum").unwrap();
    let memory = instance.get_memory("memory").unwrap();

    // Test array sum
    let input = vec![1u8, 2, 3, 4, 5];
    let ptr = alloc.call(&[Value::I32(input.len() as i32)])?[0].i32().unwrap();

    // Write data to WASM memory
    unsafe {
        let data = memory.data_ptr().add(ptr as usize);
        ptr::copy(input.as_ptr(), data, input.len());
    }

    // Call function
    let result = array_sum.call(&[
        Value::I32(ptr),
        Value::I32(input.len() as i32)
    ])?;

    println!("Sum: {}", result.i32().unwrap());  // 15

    Ok(())
}
```

### 7. String Handling Pattern

Strings require special handling due to UTF-8 encoding:

```rust
// Rust module - string transformation
#[no_mangle]
pub unsafe fn upper(ptr: *mut u8, len: usize) -> *mut u8 {
    // Read input string from memory
    let data = Vec::from_raw_parts(ptr, len, len);
    let input_str = String::from_utf8(data).unwrap();

    // Transform to uppercase
    let mut upper = input_str.to_ascii_uppercase().as_bytes().to_owned();

    // Get pointer to result
    let result_ptr = upper.as_mut_ptr();

    // Forget the Vec - host must read and then dealloc
    std::mem::forget(upper);

    result_ptr  // Return pointer to NEW allocation
}
```

```javascript
// Host reads string returned by module
function readString(ptr, len, instance) {
    // Create view into memory at returned pointer
    var m = new Uint8Array(instance.exports.memory.buffer, ptr, len);
    var decoder = new TextDecoder("utf-8");
    return decoder.decode(m.slice(0, len));
}

// Host must deallocate memory that module allocated
function deallocGuestMemory(ptr, len, instance) {
    instance.exports.dealloc(ptr, len);
}
```

## Memory Layout Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│              WebAssembly Linear Memory Layout                    │
│                                                                 │
│  Memory grows downward (from high addresses to low)            │
│                                                                 │
│  HIGH ADDRESSES                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Stack Area                              │  │
│  │  (Local variables, function calls, temporary data)         │  │
│  └───────────────────────────────────────────────────────────┘  │
│                            ▼                                    │
│                    Grows downward                               │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Heap Area                               │  │
│  │  (Vec allocations, String data, Arrays)                    │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │  │
│  │  │  Vec<u8>    │  │  String     │  │  Result     │       │  │
│  │  │  [1,2,3]    │  │  "hello"    │  │  "HELLO"    │       │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │  │
│  └───────────────────────────────────────────────────────────┘  │
│                            ▼                                    │
│                    Grows upward                                 │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Static Data                             │  │
│  │  (Constants, function pointers, vtable)                    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  LOW ADDRESSES                                                  │
│                                                                 │
│  Host accesses via: memory.buffer[ptr...ptr+len]               │
└─────────────────────────────────────────────────────────────────┘
```

## Build Commands

```bash
# Build Rust WASM module
rustc --target wasm32-unknown-unknown --crate-type=cdylib src/lib.rs -o rust.wasm

# Build AssemblyScript WASM module
npm install
npm run asbuild

# Run Node.js host
node test.js

# Run Wasmtime host
cargo run

# Build native Rust binary for testing
rustc src/lib.rs -o mem
./mem

# Check for memory leaks with Valgrind
valgrind ./mem
```

## Key Insights

1. **`#[no_mangle]` exports functions** - Makes Rust functions visible to WASM host
2. **`std::mem::forget` prevents deallocation** - Critical for transferring ownership
3. **Pointers are offsets** - WASM pointers are offsets into linear memory
4. **Host owns memory after alloc** - Module reads via `Vec::from_raw_parts`
5. **Module owns memory after return** - Host must call `dealloc` for cleanup
6. **UTF-8 is explicit** - Strings must be encoded/decoded, not assumed ASCII

## Common Pitfalls

```rust
// WRONG: Double-free if host calls dealloc after this
#[no_mangle]
pub unsafe fn bad_function(ptr: *mut u8, len: usize) {
    let data = Vec::from_raw_parts(ptr, len, len);
    let result = data.iter().sum();
    // data drops here - memory freed!
    // If host calls dealloc(ptr, len), double-free!
}

// CORRECT: Forget the Vec if host should deallocate
#[no_mangle]
pub unsafe fn good_function(ptr: *mut u8, len: usize) -> u8 {
    let data = Vec::from_raw_parts(ptr, len, len);
    let result = data.iter().sum();
    std::mem::forget(data);  // Host will dealloc
    result
}

// CORRECT: Take ownership if module allocates result
#[no_mangle]
pub unsafe fn upper(ptr: *mut u8, len: usize) -> *mut u8 {
    let data = Vec::from_raw_parts(ptr, len, len);
    let input = String::from_utf8(data).unwrap();
    let mut result = input.into_bytes();
    result.make_ascii_uppercase();
    let result_ptr = result.as_mut_ptr();
    std::mem::forget(result);  // Host must dealloc this NEW allocation
    result_ptr
}
```

## Performance Characteristics

| Operation | Cost | Notes |
|-----------|------|-------|
| alloc() | O(1) | Vec allocation |
| Memory copy | O(n) | Host → Guest |
| Function call | O(1) | Direct WASM call |
| Memory read | O(1) | Direct pointer access |
| dealloc() | O(1) | Vec drop |

## Use Cases

1. **High-performance compute** - Offload number crunching to WASM
2. **Sandboxed execution** - Run untrusted code safely
3. **Polyglot applications** - Mix Rust, TypeScript, C in one app
4. **Edge computing** - Portable binaries for edge runtimes
5. **Education** - Learn WASM memory model hands-on

## Open Questions

- How does this scale to large data (GB+)?
- What about shared memory for concurrent access?
- How do bulk operations compare to individual calls?
- What are the implications for garbage-collected languages?
