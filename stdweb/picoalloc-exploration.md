---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.stdweb/picoalloc
repository: https://github.com/koute/picoalloc
explored_at: 2026-03-23
language: Rust
---

# Sub-Project Exploration: picoalloc

## Overview

**picoalloc** is a simple, small, and fast memory allocator written in pure Rust. It is designed for constrained environments (WASM, embedded, PolkaVM) where a tiny footprint is critical. The allocator provides constant-time allocation and deallocation, low fragmentation, is panic-free, has no external dependencies, and compiles down to approximately 2.5KB of machine code.

The allocator is authored by Jan Bujak (Koute, same author as stdweb and cargo-web) and uses a two-level bitmap approach inspired by Sebastian Aaltonen's OffsetAllocator for bin management.

## Architecture

```mermaid
graph TD
    subgraph "Allocator Core"
        Allocator[Allocator struct]
        BitMask[BitMask - Two-level bitmask]
        FreeLists[Free Lists Array]
        ChunkHeaders[Chunk Headers in Memory]
    end

    subgraph "Memory Layout"
        BaseAddr[Base Address - mmap/sbrk]
        Chunk1[Chunk: Header + Data]
        Chunk2[Free Chunk: Header + Prev/Next Pointers]
        Chunk3[Chunk: Header + Data]
    end

    subgraph "Environment Backends"
        Linux[env/linux.rs - mmap/munmap]
        WASM[env/wasm.rs - memory.grow]
        PolkaVM[env/polkavm.rs - sbrk]
        CoreVM[env/corevm.rs]
    end

    subgraph "Global Allocator"
        GlobalRust[global_allocator_rust.rs - #[global_allocator]]
        GlobalLibc[global_allocator_libc.rs - malloc/free replacement]
    end

    Allocator --> BitMask
    Allocator --> FreeLists
    Allocator --> ChunkHeaders
    Allocator --> BaseAddr

    BaseAddr --> Linux
    BaseAddr --> WASM
    BaseAddr --> PolkaVM
    BaseAddr --> CoreVM

    GlobalRust --> Allocator
    GlobalLibc --> Allocator
```

## Directory Structure

```
picoalloc/
├── Cargo.toml                     # Workspace + package config
├── src/
│   ├── lib.rs                     # Crate root, exports Allocator and Size
│   ├── allocator.rs               # Core allocator (917 lines)
│   ├── env.rs                     # Environment abstraction (memory backend dispatch)
│   ├── env/
│   │   ├── linux.rs               # Linux mmap backend
│   │   ├── polkavm.rs             # PolkaVM sbrk backend
│   │   └── corevm.rs              # CoreVM backend
│   ├── global_allocator.rs        # Shared global allocator infrastructure
│   ├── global_allocator_rust.rs   # Rust GlobalAlloc implementation
│   └── global_allocator_libc.rs   # C-compatible malloc/free/realloc
├── native/                        # Native test helper crate
│   ├── Cargo.toml
│   └── src/lib.rs
├── fuzz/                          # Fuzz testing
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   └── fuzz_targets/
│       └── allocator.rs           # Fuzz target for allocator
├── ci/                            # CI scripts
├── .github/                       # GitHub workflows
├── .vscode/                       # Editor settings
├── README.md
├── rust-toolchain.toml
└── rustfmt.toml
```

## Key Components

### Allocator Design

The allocator uses a **segregated free list** approach with a **two-level bitmap** for O(1) bin lookup:

**Constants:**
- `MAX_ALLOCATION_SIZE`: 1 GB
- `MAX_BINS`: 4096
- `ALLOCATION_GRANULARITY`: 32 bytes (minimum allocation unit)

**Chunk Structure:**
- `ChunkHeader { prev_chunk_size: Size, size: ChunkSize }` - 8 bytes per allocation
- `FreeChunkHeader` - extends `ChunkHeader` with `next_in_list` and `prev_in_list` pointers for the free list
- The `ChunkSize` uses the lowest bit as an allocated/free flag

**Bin Index Calculation:**
- Based on Sebastian Aaltonen's OffsetAllocator algorithm
- Uses a mantissa/exponent scheme for logarithmic binning
- The optimal configuration is calculated at compile time via const functions
- `to_bin_index_generic<MANTISSA_BITS, ROUND_UP>()` maps sizes to bin indices

**Two-Level BitMask:**
- Primary mask: indicates which groups of bins have free chunks
- Secondary masks: individual bin availability within each group
- `find_first()` uses `trailing_zeros()` for fast bit scanning
- Architecture-aware: uses `u32` masks on 32-bit targets, `u64` on 64-bit

### Allocation Algorithm

1. Calculate minimum size including header and alignment padding
2. Find the smallest bin with available free space using the two-level bitmask
3. If the rounded-up bin has no space, try the rounded-down bin (oversized regions)
4. Split the found free chunk into: left padding, allocation, right remainder
5. Register padding/remainder as free space, mark allocation as allocated
6. Return data pointer (after header)

### Deallocation Algorithm

1. Locate the chunk header by subtracting `HEADER_SIZE` from the pointer
2. Try to merge with the previous chunk if it is free
3. Try to merge with the next chunk if it is free
4. Register the merged region as free space

### Pointer Abstraction

The `Pointer<T>` type abstracts over raw pointers:
- Supports both strict provenance (`with_addr()`) and legacy provenance (`expose_provenance()`)
- Architecture-specific `Address` type: `u32` on 32-bit, `u64` on 64-bit
- `#[repr(transparent)]` for zero-cost abstraction

### Environment Backends

The `env` module provides platform-specific memory operations:
- `allocate_address_space(size)` - Reserve virtual address space
- `expand_memory_until(ptr)` - Ensure memory is committed up to a pointer
- `free_address_space(ptr, size)` - Release address space
- `abort()` - Abort the process

Supported backends: Linux (mmap), WASM (memory.grow), PolkaVM (sbrk), CoreVM

### Global Allocator Integration

Two modes:
- `global_allocator_rust` - Implements `GlobalAlloc` trait for `#[global_allocator]`
- `global_allocator_libc` - Provides C-compatible `malloc`, `free`, `realloc`, `calloc` symbols

## Features

| Feature | Purpose |
|---------|---------|
| `global_allocator_libc` | Export C malloc/free symbols |
| `global_allocator_rust` | Implement Rust GlobalAlloc trait |
| `paranoid` | Enable extensive runtime assertions |
| `strict_provenance` | Use strict pointer provenance APIs |
| `corevm` | PolkaVM/CoreVM backend support |

## Key Insights

- The allocator is `#![no_std]` and panic-free, making it suitable for bare-metal and WASM targets
- The two-level bitmap approach (inspired by OffsetAllocator) provides true O(1) allocation by using bit-scanning instructions (`trailing_zeros`)
- All bin configuration is computed at compile time via const functions, meaning zero runtime overhead for setup
- The 32-byte allocation granularity means there is a minimum overhead per allocation, but this keeps the free list management simple
- Paranoid assertions can be enabled for debugging without affecting release performance
- The strict provenance support shows awareness of Rust's evolving memory model
- Fuzz testing is included, which is critical for allocator correctness
- The ~2.5KB machine code footprint makes this one of the smallest general-purpose allocators available in Rust
- Adjacent free chunk coalescing prevents fragmentation over time
