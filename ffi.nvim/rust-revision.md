# fff.nvim Rust Revision

## Overview

**fff.nvim** is a high-performance file finder and grep engine for Neovim, implemented as a Rust workspace with 6 crates. It achieves 10-50x faster performance than traditional tools through aggressive SIMD acceleration, parallel processing, and smart caching.

**Source**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/fff.nvim/`

---

## Crate Structure

| Crate | Purpose | Key Features |
|-------|---------|--------------|
| **fff-grep** | Grep engine | SIMD substring search, line-oriented matching |
| **fff-query-parser** | Query parsing | Constraint extraction, glob detection |
| **fff-core** (fff-search) | Core engine | File indexing, fuzzy search, grep, bigram index, frecency |
| **fff-nvim** | Neovim integration | Lua cdylib, mimalloc allocator |
| **fff-mcp** | MCP server | Model Context Protocol for AI integration |
| **fff-c** | C FFI | Cross-language bindings |

---

## Key Optimizations

### 1. SIMD Acceleration

#### Packed-Pair Substring Search
- **File**: `crates/fff-core/src/case_insensitive_memmem.rs`
- Uses AVX2 (x86_64) and NEON+dotprod (aarch64) for case-insensitive search
- Selects two rare bytes from needle, scans 32/16 positions in parallel
- Achieves O(n/Σ²) selectivity vs O(n) for naive search

```rust
// AVX2: 4 cmpeq + 2 or + 1 and + 1 movemask per 32 bytes
#[target_feature(enable = "avx2")]
unsafe fn search_packed_pair_avx2(haystack, needle_lower, i1, i2) -> bool
```

### 2. Bigram Inverted Index

- **File**: `crates/fff-core/src/types.rs`
- Dense bitset columns (64 files per u64 word)
- SIMD-vectorized AND operations for filtering
- Skip-1 bigrams (stride 2) for orthogonal filtering
- Lazy column allocation to avoid 300MB upfront cost
- Overlay system for incremental updates without rebuild

```rust
// Auto-vectorized bitset AND - 8 files per CPU cycle
fn bitset_and(result: &mut [u64], bitset: &[u64]) {
    result.iter_mut().zip(bitset.iter()).for_each(|(r, b)| *r &= *b);
}
```

### 3. Memory-Mapped File Handling

- **File**: `crates/fff-core/src/types.rs` (FileContent enum)
- Platform-specific strategy:
  - Unix: mmap for files >4KB (16KB on aarch64)
  - Windows: Always heap buffer (avoids held file handles)
- Budget-limited caching to prevent resource exhaustion
- OnceLock for lock-free lazy initialization

```rust
pub enum FileContent {
    #[cfg(not(target_os = "windows"))]
    Mmap(memmap2::Mmap),
    Buffer(Vec<u8>),
}
```

### 4. Parallel Processing (Rayon)

- Parallel filesystem walking with `ignore::WalkBuilder`
- Parallel fuzzy matching with `neo_frizbee::match_list_parallel`
- Parallel grep search with bigram prefiltering
- Configurable thread count with `max_threads` option

### 5. Frecency Scoring (LMDB)

- **File**: `crates/fff-core/src/frecency.rs`
- Exponential decay: 10-day half-life (3-day for AI mode)
- Blake3 hashing for file keys
- Background GC with database compaction
- Optional NO_LOCK/NO_SYNC for single-process speed

### 6. Thread-Local Sort Buffer

- **File**: `crates/fff-core/src/sort_buffer.rs`
- Reusable buffer for glidesort operations
- Eliminates 12KB allocation per search
- Type-erased u8 buffer cast to MaybeUninit<T>

```rust
thread_local! {
    static SORT_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(1024));
}
```

### 7. Constraint Filtering

- **File**: `crates/fff-core/src/constraints.rs`
- Allocation-free path matching (no regex for simple cases)
- Extension, path segment, glob, git status filters
- OR logic for multiple extensions, AND for other constraints
- Parallel filtering with rayon for >10k items

### 8. Filesystem Watching

- **File**: `crates/fff-core/src/background_watcher.rs`
- notify-debouncer-full with 250ms debounce
- Selective directory watching (excludes gitignored dirs)
- Owner thread pattern for clean shutdown
- Bigram overlay for incremental updates

---

## Performance Benchmarks

| Operation | Time | Notes |
|-----------|------|-------|
| Index 500k files | ~2s | Parallel walk + git status |
| Fuzzy search (warm) | 5-20ms | With bigram filter |
| Grep "struct" (100 results) | 50ms | Frecency-ordered, early termination |
| Grep with constraints | 30ms | Pre-filtered candidate set |
| Grep with bigram filter | 10ms | 50x faster than ripgrep |

---

## Build Configuration

```toml
[profile.release]
opt-level = 3
lto = "fat"           # Link-time optimization
codegen-units = 1     # Single compilation unit for better optimization
strip = true          # Remove debug symbols
```

---

## Dependencies

| Dependency | Purpose |
|------------|---------|
| rayon | Data-parallel processing |
| neo_frizbee | Fuzzy matching (Smith-Waterman) |
| memchr | SIMD byte search primitives |
| heed | LMDB bindings (frecency DB) |
| memmap2 | Memory-mapped I/O |
| notify-debouncer-full | Filesystem watching |
| glidesort | Fast sorting with buffer reuse |
| smallvec | Stack-allocated small vectors |
| ahash | AES-NI accelerated hashing |
| blake3 | Fast cryptographic hashing |
| git2 | Git status caching |
| parking_lot | Fast mutex/RWLock |

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      Neovim / Lua                           │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                    fff-nvim (cdylib)                        │
│              mimalloc allocator, Lua bindings               │
└───────────────────────┬─────────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
┌───────────────┐ ┌─────────────┐ ┌─────────────┐
│  fff-core     │ │  fff-c      │ │  fff-mcp    │
│  (search SDK) │ │  (FFI)      │ │  (MCP srv)  │
└───────┬───────┘ └─────────────┘ └─────────────┘
        │
        ├─► file_picker.rs   (indexing, fuzzy search)
        ├─► grep.rs          (live grep, constraints)
        ├─► types.rs         (bigram index, FileItem)
        ├─► score.rs         (frecency scoring)
        ├─► frecency.rs      (LMDB database)
        ├─► background_watcher.rs (notify)
        └─► sort_buffer.rs   (thread-local buffer)
                │
        ┌───────┴───────┐
        ▼               ▼
┌───────────────┐ ┌─────────────┐
│ fff-grep      │ │fff-query-   │
│ (grep engine) │ │  parser     │
│ SIMD matcher  │ │ constraints │
└───────────────┘ └─────────────┘
```

---

## Key Design Decisions

1. **Sorted Vec<FileItem>** over HashMap: Stable indices for bigram bitsets, binary search support
2. **Tombstone flags** for deletions: Preserves bigram index stability
3. **Lazy content loading**: OnceLock avoids initialization until first access
4. **Budget-limited caching**: Prevents monorepo resource exhaustion
5. **Frecency-ordered search**: Most relevant files first, enables early termination
6. **Platform-specific mmap**: Unix gets zero-copy, Windows avoids held handles
7. **Selective directory watching**: Prevents FSEvents buffer overflow on macOS

---

## Future Optimization Opportunities

1. **AVX-512 support**: For newer Intel CPUs (Ice Lake+)
2. **GPU-accelerated bigram AND**: CUDA/Metal for massive repos
3. **Compressed bitsets**: Roaring bitmaps for sparse columns
4. **Incremental bigram rebuild**: Update columns without full rebuild
5. **Prefetching hints**: Software prefetch for sequential bitset access
6. **Huge page allocation**: For bigram index (reduces TLB misses)

---

## Related Deep Dive

See `deep-dives/ssa-optimizations-and-performance.md` for comprehensive coverage of:
- SIMD implementation details
- Bigram index algorithms
- Memory management strategies
- Parallel processing patterns
- Benchmark comparisons
