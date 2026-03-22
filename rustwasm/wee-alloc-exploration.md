---
name: Wee Alloc
description: Tiny allocator for WebAssembly designed to minimize code size
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/wee_alloc/
---

# Wee Alloc - The Wee Allocator

## Overview

Wee Alloc is a **tiny memory allocator designed specifically for WebAssembly** that prioritizes code size over performance. At just a few kilobytes of compiled WASM, it's ideal for size-constrained applications where every byte counts. While not as fast as system allocators, it provides a good balance between functionality and size.

Key features:
- **Extremely small** - ~2-3KB of WASM code
- **Simple design** - Easy to audit and understand
- **WASM optimized** - Built specifically for WebAssembly
- **Drop-in replacement** - Works as #[global_allocator]
- **Feature flags** - Enable only what you need
- **No dependencies** - Pure Rust implementation

## Directory Structure

```
wee_alloc/
├── src/
│   ├── lib.rs              # Main allocator implementation
│   ├── unsafe_cell.rs      # Interior mutability
│   ├── bitset.rs           # Free list bitset
│   └── platform/           # Platform-specific code
│       ├── wasm.rs         # WebAssembly platform
│       └── unix.rs         # Unix fallback
├── tests/
│   └── alloc_tests.rs      # Allocation tests
├── Cargo.toml
├── README.md
└── LICENSE.md
```

## Installation

```toml
[dependencies]
wee_alloc = "0.4.5"

# Configure as global allocator
[profile.release]
opt-level = "s"
lto = true
```

```rust
// In lib.rs or main.rs
use wee_alloc::WeeAlloc;

#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

fn main() {
    // Now all allocations use WeeAlloc
    let v = vec![1, 2, 3];
}
```

## How It Works

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Wee Alloc Architecture                       │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  Memory Pool (Linear WASM Memory)                               │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ Header │ Block 1 │ Block 2 │ ... │ Block N │ Unused     │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│  Free List (Bitset)                                             │
│  - Tracks which blocks are free                                 │
│  - O(1) check if block is free                                  │
│  - Compact representation                                       │
└─────────────────────────────────────────────────────────────────┘
```

### Basic Allocator

```rust
use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

pub struct WeeAlloc {
    // Allocator state
    state: UnsafeCell<AllocatorState>,
}

struct AllocatorState {
    // Base of the heap
    heap_start: usize,
    // Current heap end (grows upward)
    heap_end: usize,
    // Free list bitset
    free_list: BitSet,
    // Block size in bytes
    block_size: usize,
}

unsafe impl GlobalAlloc for WeeAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let state = &mut *self.state.get();

        // Calculate size needed (aligned)
        let size = (layout.size() + state.block_size - 1)
            / state.block_size
            * state.block_size;

        // Find free block(s)
        let block_idx = state.free_list.find_contiguous(
            size / state.block_size
        );

        if let Some(idx) = block_idx {
            // Mark as allocated
            state.free_list.clear_range(idx, size / state.block_size);

            // Return pointer
            return state.block_to_ptr(idx);
        }

        // No free block, grow heap
        state.grow_heap(size)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let state = &mut *self.state.get();

        // Convert pointer to block index
        let block_idx = state.ptr_to_block(ptr);

        // Calculate block count
        let block_count = layout.size() / state.block_size;

        // Mark as free
        state.free_list.set_range(block_idx, block_count);
    }
}
```

### Free List Bitset

```rust
pub struct BitSet {
    bits: [u64; MAX_BLOCKS / 64],
}

impl BitSet {
    /// Check if block is free
    pub fn is_free(&self, idx: usize) -> bool {
        let word_idx = idx / 64;
        let bit_idx = idx % 64;
        (self.bits[word_idx] & (1 << bit_idx)) != 0
    }

    /// Mark block as allocated
    pub fn set_allocated(&mut self, idx: usize) {
        let word_idx = idx / 64;
        let bit_idx = idx % 64;
        self.bits[word_idx] &= !(1 << bit_idx);
    }

    /// Mark block as free
    pub fn set_free(&mut self, idx: usize) {
        let word_idx = idx / 64;
        let bit_idx = idx % 64;
        self.bits[word_idx] |= (1 << bit_idx);
    }

    /// Find contiguous free blocks
    pub fn find_contiguous(&self, count: usize) -> Option<usize> {
        let mut free_count = 0;
        let mut start_idx = None;

        for i in 0..MAX_BLOCKS {
            if self.is_free(i) {
                if start_idx.is_none() {
                    start_idx = Some(i);
                }
                free_count += 1;

                if free_count >= count {
                    return start_idx;
                }
            } else {
                free_count = 0;
                start_idx = None;
            }
        }

        None
    }
}
```

### Heap Growth

```rust
impl AllocatorState {
    fn grow_heap(&mut self, additional: usize) -> *mut u8 {
        // Current heap end
        let old_end = self.heap_end;

        // New heap end (aligned to page boundary)
        let new_end = align_up(old_end + additional, PAGE_SIZE);

        // Request more memory from WASM
        let pages_needed = (new_end - old_end) / PAGE_SIZE;
        let result = wasm_bindgen::memory_grow(pages_needed);

        if result.is_err() {
            // Out of memory
            return ptr::null_mut();
        }

        // Update heap end
        self.heap_end = new_end;

        // Return pointer to old end
        old_end as *mut u8
    }
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
```

## Configuration

### Feature Flags

```toml
[dependencies.wee_alloc]
version = "0.4.5"
default-features = false
features = [
    "size_classes",      # Size class allocation (faster, larger)
    "backtrace",         # Stack traces on error (larger)
    "panic_handling",    # Panic on allocation failure
]
```

### Custom Configuration

```rust
use wee_alloc::WeeAlloc;
use core::alloc::Layout;

static mut ALLOC: WeeAlloc = WeeAlloc {
    state: UnsafeCell::new(AllocatorState {
        heap_start: HEAP_START,
        heap_end: HEAP_START,
        free_list: BitSet::new(),
        block_size: 16,  // Smaller blocks = finer granularity
    }),
};

// Or use builder pattern
static ALLOC: WeeAlloc = WeeAlloc::INIT
    .configure(|config| {
        config.block_size(32);
        config.initial_heap_size(64 * 1024);  // 64KB initial
    });

#[global_allocator]
static A: &WeeAlloc = &ALLOC;
```

## Size Comparison

```
Allocator       | WASM Size | Speed    | Use Case
----------------|-----------|----------|------------------
wee_alloc       | ~2-3 KB   | Medium   | Size-critical
std::alloc      | ~10-20 KB | Fast     | General purpose
dlmalloc        | ~15 KB    | Fast     | Multi-threaded
mimalloc        | ~25 KB    | Very Fast| Performance
```

## Usage Patterns

### Minimal WASM Module

```rust
// Minimal WASM with wee_alloc
#![no_std]
#![no_main]

use wee_alloc::WeeAlloc;
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

#[wasm_bindgen]
pub fn process_data(input: &[u8]) -> Vec<u8> {
    // This allocation uses wee_alloc
    let mut output = Vec::with_capacity(input.len() * 2);

    for &byte in input {
        output.push(byte);
        output.push(byte ^ 0xFF);
    }

    output
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
```

### With Console Error Panic Hook

```rust
use wee_alloc::WeeAlloc;
use wasm_bindgen::prelude::*;
use console_error_panic_hook;

#[global_allocator]
static ALLOC: WeeAlloc = WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();

    // Application code
    let data = vec![1, 2, 3, 4, 5];
    process(&data);
}

fn process(data: &[i32]) {
    let result: Vec<i32> = data.iter().map(|x| x * 2).collect();
    console_log!("Result: {:?}", result);
}
```

### Custom Allocation Strategy

```rust
use wee_alloc::WeeAlloc;
use core::alloc::{GlobalAlloc, Layout};

struct TracingAllocator<A: GlobalAlloc> {
    inner: A,
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for TracingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        console_log!("Alloc: size={}, align={}", layout.size(), layout.align());
        self.inner.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        console_log!("Dealloc: ptr={:p}, size={}", ptr, layout.size());
        self.inner.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOC: TracingAllocator<WeeAlloc> = TracingAllocator {
    inner: WeeAlloc::INIT,
};
```

## Limitations

### Not Thread-Safe

```rust
// WeeAlloc is NOT thread-safe by default
// For multi-threaded WASM, consider:

// Option 1: Use dlmalloc
use dlmalloc::GlobalDlmalloc;

#[global_allocator]
static ALLOC: GlobalDlmalloc = GlobalDlmalloc;

// Option 2: Add mutex (larger code size)
use std::sync::Mutex;

static ALLOC: Mutex<WeeAlloc> = Mutex::new(WeeAlloc::INIT);
```

### No Coalescing

```rust
// WeeAlloc doesn't coalesce adjacent free blocks
// This can lead to fragmentation:

// Bad pattern - causes fragmentation
let mut vecs = Vec::new();
for i in 0..100 {
    vecs.push(vec![0u8; 1024]);  // Allocate
    drop(vecs.remove(0));         // Free first
}
// Now heap is fragmented

// Better pattern - allocate together
let mut vecs: Vec<Vec<u8>> = (0..100)
    .map(|_| vec![0u8; 1024])
    .collect();
```

### Size Class Limitations

```rust
// With size_classes feature:
// - Small allocations rounded up to size class
// - More efficient but potentially wasteful

// Without size_classes:
// - Exact size allocation
// - Slower but more memory efficient
```

## Performance Tips

### Pre-allocate When Possible

```rust
// Bad: Many small allocations
let mut result = Vec::new();
for item in items {
    result.push(process(item));  // Each push may allocate
}

// Good: Pre-allocate
let mut result = Vec::with_capacity(items.len());
for item in items {
    result.push(process(item));
}
```

### Reuse Allocations

```rust
// Bad: New allocation each frame
fn render_frame() -> Vec<u8> {
    let mut buffer = Vec::with_capacity(1920 * 1080 * 4);
    // ... render ...
    buffer
}

// Good: Reuse buffer
static mut BUFFER: Option<Vec<u8>> = None;

fn render_frame() -> &mut [u8] {
    unsafe {
        if BUFFER.is_none() {
            BUFFER = Some(vec![0u8; 1920 * 1080 * 4]);
        }
        BUFFER.as_mut().unwrap().as_mut_slice()
    }
}
```

## Related Documents

- [wasm-bindgen](./wasm-bindgen-exploration.md) - Rust/JS bindings
- [Twiggy](./twiggy-exploration.md) - Size analysis
- [console_error_panic_hook](./console-error-panic-hook-exploration.md) - Debug support

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.rustwasm/wee_alloc/`
- Documentation: https://docs.rs/wee_alloc/
- GitHub: https://github.com/rustwasm/wee_alloc
