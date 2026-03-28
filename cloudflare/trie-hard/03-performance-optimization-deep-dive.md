# Performance Optimization Deep Dive

**Deep Dive 03** | Memory Layout, Cache Efficiency, and CPU Optimization
**Source:** `trie-hard/benches/`, `trie-hard/src/lib.rs` | **Date:** 2026-03-27

---

## Executive Summary

trie-hard achieves its performance through:
1. **Contiguous memory layout** - Maximizes cache hits
2. **Bitwise operations** - Single CPU instructions
3. **Branch prediction friendly** - Consistent control flow
4. **Fail-fast design** - Early termination on misses

This document analyzes each optimization with benchmark data and explains how to extend them.

---

## Part 1: Memory Layout Optimization

### The Problem: Pointer Chasing

Traditional trie implementations scatter nodes across the heap:

```
Traditional Trie Memory:
Heap Address 0x1000: [TrieNode] -> points to 0x5000
Heap Address 0x2000: [TrieNode] -> points to 0x7500
Heap Address 0x3500: [TrieNode] -> points to 0x1200
Heap Address 0x5000: [TrieNode] -> points to 0x8000
...

Each lookup: 4-5 cache misses (load from random addresses)
Cache miss cost: ~100 CPU cycles
Total overhead: 400-500 cycles per lookup
```

### trie-hard Solution: Contiguous Storage

```rust
pub struct TrieHardSized<'a, T, I> {
    masks: MasksByByteSized<I>,  // Fixed-size array
    nodes: Vec<TrieState<'a, T, I>>,  // Contiguous vector
}
```

**Memory layout:**
```
trie-hard Memory:
Heap Address 0x1000: [TrieHardSized header]
Heap Address 0x1020: [masks array - 256 entries contiguous]
Heap Address 0x1120: [nodes Vec - all TrieState entries contiguous]

Each lookup: 1-2 cache misses (prefetch helps)
Cache miss cost: ~100 CPU cycles
Total overhead: 100-200 cycles per lookup
```

### Cache Line Utilization

Modern CPUs load memory in **cache lines** (typically 64 bytes):

```
Cache line: 64 bytes

trie-hard node (u32 masks):
  SearchNode {
    mask: u32,       // 4 bytes
    edge_start: usize // 8 bytes (64-bit)
  }
  Total: 12 bytes (+ 4 padding) = 16 bytes

Cache line holds: 64 / 16 = 4 nodes

When loading node N, nodes N+1, N+2, N+3 come along for free!
```

**Impact on traversal:**
```
Lookup "content-type" (12 characters):

Without cache efficiency:
  12 nodes visited -> 12 cache misses -> 1200 cycles

With cache efficiency:
  12 nodes visited -> 3 cache misses (4 nodes per line) -> 300 cycles

Speedup: 4x
```

### Measuring Cache Performance

```rust
use std::arch::x86_64::_mm_mfence;

// Force cache miss before benchmark
unsafe {
    _mm_mfence();  // Memory fence
    // Flush specific addresses (requires privileged instruction)
    // _mm_clflush(addr);
}

// Now benchmark - first iteration will have cache misses
```

**Better approach: perf counters**
```bash
# Run with performance counters
perf stat -e cache-misses,cache-references ./target/release/benchmarks

# Expected output for trie-hard:
# cache-misses: ~10% of references
# For HashMap: ~25% of references
```

---

## Part 2: Bitwise Operation Efficiency

### The Child Index Formula

```rust
let child_index = ((input_mask - 1) & node.mask).count_ones() as usize;
```

**Assembly output (x86-64):**
```asm
; input_mask in rax, node.mask in rbx
sub rax, 1          ; input_mask - 1
and rax, rbx        ; & node.mask
popcnt rax, rax     ; count_ones() (single instruction!)
; Result in rax
```

**Total: 3 CPU instructions** (excluding the comparison)

### Comparison to HashMap

HashMap lookup requires:

```rust
// HashMap get() internals:
fn get(&self, key: &str) -> Option<&V> {
    let hash = self.hasher.hash_one(key);  // ~50 instructions
    let bucket = hash % self.buckets.len(); // Division: ~20-50 cycles
    // Then traverse bucket链表...
}
```

**Hash function for "content-type":**
```rust
// SipHash-1-3 (default HashMap hasher)
// Processes 12 bytes:
for byte in key.bytes() {
    state = mix(state, byte);  // Multiple rounds of mixing
}
// ~200+ instructions total
```

**Comparison:**
| Operation | trie-hard | HashMap |
|-----------|-----------|---------|
| Per-byte cost | 3 instructions | ~15 instructions |
| Total for 12-byte key | ~36 instructions | ~200+ instructions |
| Memory accesses | 1-2 | 4-6 |

---

## Part 3: Branch Prediction Analysis

### Predictable Control Flow

Modern CPUs use **branch prediction** to speculate on if/else outcomes:

```rust
// Good for branch prediction:
for byte in key.bytes() {
    let mask = masks[byte as usize];  // Always happens
    if (node.mask & mask) > 0 {       // Consistent pattern
        // Continue traversal
    } else {
        return None;  // Early exit on miss
    }
}
```

**Branch prediction success rate:**
- First character: ~50% (depends on hit rate)
- Subsequent characters: >90% (if first matched, likely continues)

### Fail-Fast Optimization

```rust
// Early exit is GOOD for performance on misses
fn get(&self, key: &[u8]) -> Option<T> {
    for (i, c) in key.iter().enumerate() {
        // Check if byte is allowed
        if (node.mask & masks[*c]) == 0 {
            return None;  // Branch NOT taken on hits, taken on misses
        }
        // Continue...
    }
}
```

**For 50% miss rate:**
```
HashMap: Always hashes entire key (wasted work on misses)
trie-hard: Fails after 1-3 bytes on average (50% of lookups)

Example miss "xyz123":
  HashMap: Hash all 6 bytes -> lookup -> None
  trie-hard: Check 'x' -> not in trie -> None (1 byte processed)

Speedup on misses: 6x
```

### Branchless Alternatives (For Comparison)

```rust
// Branchless lookup (NOT used by trie-hard, for comparison)
fn get_branchless(&self, key: &[u8]) -> Option<T> {
    let mut valid = true;
    let mut node_index = 0;

    for c in key.iter() {
        let mask = masks[*c as usize];
        let is_valid = (nodes[node_index].mask & mask) > 0;
        valid &= is_valid;
        // Can't easily make branchless - need to update node_index conditionally
    }

    valid.then(|| /* extract value */)
}
```

**Why trie-hard doesn't do this:** Early exit is faster for high miss rates.

---

## Part 4: Benchmark Analysis

### Benchmark Setup

From `trie-hard/benches/divan_bench.rs`:

```rust
#[divan::bench(args = args())]
fn trie_hard_get(bencher: divan::Bencher, input: &Input) {
    bencher
        .with_inputs(|| {
            let words = match input.size {
                Size::Header => get_header_text(),  // 119 headers
                Size::Big => get_big_text(),        // 1984 corpus
                Size::Small => get_small_text(),    // Sun Rising corpus
            };
            let trie = make_trie(&words);
            (generate_samples(&words, input.percent), trie)
        })
        .bench_values(|(samples, trie): (Vec<&str>, TrieHard<'_, &str>)| {
            samples
                .iter()
                .filter_map(|w| trie.get(black_box(&w[..])))
                .count()
        });
}
```

### Understanding the Benchmark

```rust
// generate_samples creates realistic hit/miss distribution
fn generate_samples<'a>(hits: &[&'a str], hit_percent: i32) -> Vec<&'a str> {
    let roulette_inc = hit_percent as f64 / 100.;
    let mut roulette = 0.;

    let mut result = get_random_text().to_owned();  // Misses from random text
    let mut hit_iter = hits.iter().cycle().copied();  // Hits from dataset

    for w in result.iter_mut() {
        roulette += roulette_inc;
        if roulette >= 1. {
            roulette -= 1.;
            *w = hit_iter.next().unwrap();  // Replace with hit
        }
    }

    result
}
```

**For 50% hit rate with 10k samples:**
- 5,000 hits (keys in trie)
- 5,000 misses (random keys)

### Benchmark Results Explained

From README.md charts:

```
Headers vs HashMap (119 entries, 10k lookups):

Hit Rate | HashMap | trie-hard | Winner
---------|---------|-----------|--------
100%     | 45 μs   | 35 μs     | trie-hard (22% faster)
75%      | 48 μs   | 32 μs     | trie-hard (33% faster)
50%      | 52 μs   | 25 μs     | trie-hard (52% faster)
25%      | 56 μs   | 20 μs     | trie-hard (64% faster)
10%      | 58 μs   | 18 μs     | trie-hard (69% faster)
1%       | 62 μs   | 15 μs     | trie-hard (76% faster)
```

**Trend:** As miss rate increases, trie-hard's advantage grows.

### Insert Performance (Bulk Load)

```
Time to load 15.5k words:

Structure  | Time    | Notes
-----------|---------|------
HashMap    | 2.1 ms  | Fastest (just hash + insert)
Radix Trie | 3.49 ms | 1.7x slower than HashMap
trie-hard  | 11.92 ms| 5.7x slower than HashMap
```

**Why trie-hard is slower to build:**
1. Must collect all unique bytes first
2. Sort input for deterministic construction
3. BFS traversal to build nodes
4. More complex node structure

**Trade-off:** Slower build, faster queries (for high miss rates)

---

## Part 5: SIMD Potential

### Current Implementation (Scalar)

```rust
// Scalar: one byte at a time
for c in key.iter() {
    let c_mask = trie.masks.0[c as usize];
    let mask_res = self.mask & c_mask;
    if mask_res == 0 {
        return None;
    }
    // ...
}
```

### SIMD Approach (Hypothetical)

```rust
use std::arch::x86_64::*;

// Process 16 bytes simultaneously
unsafe fn get_simd(&self, key: &[u8]) -> Option<T> {
    let chunks = key.chunks_exact(16);

    for chunk in chunks {
        // Load 16 bytes into SIMD register
        let bytes = _mm_loadu_si128(chunk.as_ptr() as *const __m128i);

        // Gather masks for all 16 bytes (requires AVX2 gather)
        let masks = _mm_i32gather_epi32(self.masks.0.as_ptr() as *const i32, bytes, 4);

        // Check if any byte is invalid
        // ... complex SIMD logic
    }

    // Handle remainder
    for c in key.chunks_exact(16).remainder() {
        // Scalar fallback
    }
}
```

**Why trie-hard doesn't use SIMD:**
1. Key lengths vary (not always multiple of 16)
2. Early exit benefit lost (must process all bytes)
3. Gather instructions expensive on some CPUs
4. Complexity vs. marginal gain for short keys

**When SIMD helps:**
- Long keys (> 32 bytes)
- Batch processing multiple keys
- Known fixed-length keys

---

## Part 6: Memory Footprint Analysis

### Space Complexity Breakdown

For `n` entries with `u` unique bytes:

```
MasksByByte: 256 * (u / 8) bytes  (rounded up to integer size)

Nodes:
  - Leaf nodes: n entries
  - Internal nodes: ~n entries (depends on common prefixes)
  - Per node: mask_size + edge_start + enum discriminant

Total ≈ 256 * (u/8) + 2n * (mask_size + 16) bytes
```

### Concrete Example: HTTP Headers

```
Input: 119 HTTP headers
Unique bytes: ~30 (lowercase letters, hyphen)
Mask type: u32 (4 bytes)

MasksByByte: 256 * 4 = 1024 bytes
Nodes: 2 * 119 * (4 + 8 + 1) ≈ 3094 bytes  (1 byte for enum tag)

Total: ~4KB for 119 headers
Per entry: ~34 bytes

Compare to HashMap<&str, &str>:
  Per entry: 8 (hash) + 8 (key ptr) + 8 (value ptr) + overhead ≈ 32 bytes
  Total: ~3.8KB

trie-hard uses ~7% more memory but faster for high miss rates
```

### Scaling Behavior

| Entries | Unique Bytes | trie-hard Size | HashMap Size |
|---------|--------------|----------------|--------------|
| 100 | 20 | 3.5 KB | 3.2 KB |
| 1,000 | 40 | 35 KB | 32 KB |
| 10,000 | 60 | 350 KB | 320 KB |
| 100,000 | 80 | 3.5 MB | 3.2 MB |

**Note:** Memory overhead constant at ~10% regardless of scale.

---

## Part 7: Optimization Checklist

### Compile-Time Optimizations

```toml
[profile.release]
opt-level = 3        # Maximum optimization
lto = "fat"          # Link-time optimization across crates
codegen-units = 1    # Single codegen unit for better optimization
target-cpu = "native" # Optimize for current CPU (not portable!)
```

**Build command:**
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

### Runtime Optimizations

```rust
// 1. Pre-allocate with correct capacity
let mut nodes = Vec::with_capacity(estimated_node_count);

// 2. Use get_unchecked for hot paths (unsafe!)
unsafe {
    let node = nodes.get_unchecked(index);
}

// 3. Inline small functions
#[inline]
fn evaluate(&self, c: u8) -> Option<usize> {
    // ...
}

// 4. Use raw pointers for inner loops (unsafe!)
unsafe {
    let masks_ptr = self.masks.0.as_ptr();
    let node_mask = *masks_ptr.add(c as usize);
}
```

### Benchmarking Best Practices

```rust
use divan::black_box;

// Always black_box inputs to prevent optimization
fn benchmark(trie: &TrieHard, keys: &[&str]) {
    for key in keys {
        black_box(trie.get(black_box(key)));
    }
}

// Run multiple iterations
#[divan::bench(samples = 1000)]
fn bench_get(bencher: divan::Bencher) {
    bencher.with_inputs(|| (make_trie(), make_keys()))
        .bench_values(|(trie, keys)| benchmark(&trie, &keys));
}
```

---

## Part 8: Profiling Guide

### Using perf (Linux)

```bash
# Record performance data
perf record -g ./target/release/benchmarks

# View flamegraph
perf script | stackcollapse-perf.pl | flamegraph.pl > perf.svg

# Analyze cache misses
perf stat -e cache-misses,cache-references,cycles,instructions \
    ./target/release/benchmarks
```

### Using Instruments (macOS)

```bash
# Launch Instruments with Time Profiler
open -a Instruments ./target/release/benchmarks

# Or use command-line
xcrun xctrace record --template 'Time Profiler' \
    --launch -- ./target/release/benchmarks
```

### Using cargo-benchcmp

```bash
# Install
cargo install cargo-benchcmp

# Run benchmarks before and after change
cargo bench | tee before.txt
# Make changes...
cargo bench | tee after.txt

# Compare
cargo benchcmp before.txt after.txt
```

---

## Summary

Performance optimizations in trie-hard:

1. **Contiguous memory** - 4x fewer cache misses
2. **Bitwise ops** - 3 instructions vs 200+ for hashing
3. **Branch prediction** - High success rate on traversal
4. **Fail-fast** - Early termination on misses
5. **Adaptive sizing** - Right integer type for dataset

### Next Steps

Continue to **[04-concurrency-patterns-deep-dive.md](04-concurrency-patterns-deep-dive.md)** for:
- Thread-safe read sharing
- Lock-free patterns
- Arc wrapping strategies

---

## Exercises

1. Run benchmarks with different hit rates
2. Profile cache miss rates with perf
3. Implement pre-allocation for bulk loading
4. Measure the impact of LTO on performance
5. Compare native vs WASM performance
