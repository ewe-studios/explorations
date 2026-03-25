# Rust Zero-Cost Abstractions vs. SIMD - Deep Dive

## Executive Summary

A customer query was taking **220ms** when it should have taken **~50ms**. The profiler pointed at Rust code using iterators that we'd assumed was free. Following the trail down to assembly revealed that Rust's "zero-cost" abstractions were silently preventing the compiler from vectorizing the code with SIMD.

This post explains how we found the issue, why it happened, and how we fixed it—achieving a **4.7x performance improvement** (220ms → 47ms).

---

## The Problem: Slow Filtered BM25 Query

### Customer Report

```
Query Type: Filtered BM25 Full-Text Search
Dataset: 5 million documents
Filter: ContainsAny with 1000+ values
Expected Latency: <50ms
Actual Latency: >220ms
```

### Query Structure

```json
{
  "filters": ["attribute", "ContainsAny", ["a", "c", "f", "j", "m", "o", "t", "x", "z", ...]],
  "rank_by": ["text", "BM25", "some query string"]
}
```

This is a common pattern for **permission checks** where users belong to many groups (1000+ group IDs).

### Initial Analysis

```
Query Execution Breakdown (220ms total):
┌─────────────────────────────────────────────────────────────┐
│  BM25 Ranking          │  ~10ms   (4.5%)                   │
│  Filter Evaluation     │  ~200ms  (91%) ← PROBLEM          │
│  Other Overhead        │  ~10ms   (4.5%)                   │
└─────────────────────────────────────────────────────────────┘
```

The BM25 ranking was fast. The filter evaluation was the bottleneck.

---

## Understanding the Turbopuffer Read Path

### LSM Tree Architecture

Turbopuffer stores data in an **LSM tree** on object storage:

```
┌─────────────────────────────────────────────────────────────┐
│                    LSM Tree Structure                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  MemTable (in-memory, sorted)                               │
│  ┌─────┬─────┬─────┬─────┬─────┐                           │
│  │ ab  │ cd  │ ef  │ gh  │ ij  │ ...                       │
│  └─────┴─────┴─────┴─────┴─────┘                           │
│                          │                                  │
│                          ▼ flush                            │
│  SSTable L1 (sorted files on object storage)                │
│  ┌───────┬───────┬───────┬───────┐                         │
│  │ ab-cd │ ef-gh │ ij-kl │ mn-op │                         │
│  └───────┴───────┴───────┴───────┘                         │
│                          │                                  │
│                          ▼ compact                          │
│  SSTable L2 (larger, merged files)                          │
│  ┌───────────────┬───────────────┬───────────────┐         │
│  │   ab..gh      │   ij..qr      │   st..zz      │         │
│  └───────────────┴───────────────┴───────────────┘         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### ContainsAny Filter Execution

```
Query: "a" OR "c" OR "f" OR "j" OR "m" OR "o" OR "t" OR "x" OR "z" OR ...

Execution Plan:
1. For each filter value, find matching doc IDs
   - Look up key ranges in LSM tree
   - Fetch byte ranges from object storage
   - Merge results from multiple SSTables

2. Union all matching doc IDs
   - Combine results from all values
   - Create filter bitmap

3. Apply filter to BM25 results
   - Only return documents that match filter
```

```
┌─────────────────────────────────────────────────────────────┐
│               ContainsAny Query Flow                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Query(a OR c OR f OR ...)                                  │
│       │                                                      │
│       ▼                                                      │
│  ┌──────────────────────────────────────────────────┐      │
│  │  Fan out to multiple key lookups                  │      │
│  │    ├─ lookup "a" → [doc 1, 5, 9, ...]            │      │
│  │    ├─ lookup "c" → [doc 2, 6, 10, ...]           │      │
│  │    ├─ lookup "f" → [doc 3, 7, 11, ...]           │      │
│  │    └─ ... (1000+ lookups)                        │      │
│  └──────────────────────────────────────────────────┘      │
│       │                                                      │
│       ▼                                                      │
│  ┌──────────────────────────────────────────────────┐      │
│  │  Merge iterators from 24+ SSTable files          │      │
│  │  (disjoint key ranges across LSM levels)         │      │
│  └──────────────────────────────────────────────────┘      │
│       │                                                      │
│       ▼                                                      │
│  ┌──────────────────────────────────────────────────┐      │
│  │  Union results into filter bitmap                │      │
│  │  Size: 67MB of compressed bitmaps                │      │
│  └──────────────────────────────────────────────────┘      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Napkin Math

```
Expected Performance:
┌─────────────────────────────────────────────────────────────┐
│  Data to Read: 67MB (filter bitmaps)                        │
│  SSD Throughput: 6,240 MB/s (NVMe)                          │
│                                                             │
│  Read Time: 67MB / 6,240 MB/s = ~11ms                       │
│  Processing Time: ~10-20ms (estimate)                       │
│  Total Expected: ~20-30ms                                   │
└─────────────────────────────────────────────────────────────┘

Actual Performance: >200ms

Gap: 7-10x slower than expected!
```

---

## Profiling: Following the Trail

### Step 1: High-Level Profile

```
perf record -g ./turbopuffer-query

Results:
┌─────────────────────────────────────────────────────────────┐
│  Function                    │  Time    │  % of Total       │
├─────────────────────────────────────────────────────────────┤
│  filter_evaluation           │  200ms   │  91%              │
│  ├─ merge_iterators         │  120ms   │  55%              │
│  ├─ union_bitmaps           │   60ms   │  27%              │
│  └─ decode_compressed       │   20ms   │   9%              │
│  bm25_ranking                │   10ms   │   5%              │
│  other                       │   10ms   │   5%              │
└─────────────────────────────────────────────────────────────┘
```

The `merge_iterators` function was the biggest culprit.

### Step 2: Drill Into merge_iterators

```rust
// Simplified version of the problematic code
fn merge_iterators<'a>(
    iterators: Vec<BitmapIterator<'a>>,
) -> impl Iterator<Item = DocId> + 'a {
    iterators.into_iter()
        .flatten()  // Chain all iterators together
        .sorted()   // Sort by doc ID
        .dedup()    // Remove duplicates
}

// Usage:
let merged = merge_iterators(filter_iterators);
let count = merged.count();
```

### Step 3: Assembly Analysis

```
rustc --emit=asm merge_iterators.rs

Key loop assembly (simplified):
```
```asm
.L_loop:
    cmp     rax, rbx          ; Check if iterator is done
    je      .L_next_iterator  ; Branch to next iterator

    mov     rcx, [rdi + rax*8] ; Load next doc ID
    push    rcx                ; Push onto stack (for sorting)
    inc     rax                ; Increment counter

    ; ... many more instructions ...

    jmp     .L_loop           ; Back to loop start
```

**RED FLAG**: The loop has:
- Multiple branches (je, jmp)
- Stack operations (push)
- No SIMD instructions (no vp*)

---

## The Root Cause: Iterator Chains Prevent Vectorization

### What We Expected

```rust
// Iterator chain looks like it should be optimizable:
iterators.into_iter()
    .flatten()
    .sorted()
    .dedup()

// Compiler should (in theory):
// 1. Inline all iterator methods
// 2. Unroll the loop
// 3. Vectorize with SIMD
// 4. Produce code like:
```
```asm
; Ideal (SIMD) assembly:
vmovups ymm0, [rdi]      ; Load 8 doc IDs at once
vpaddd  ymm1, ymm0, ymm2 ; Process in parallel
vpsort  ymm3, ymm1       ; Vectorized sort (hypothetical)
```

### What Actually Happened

```rust
// Each iterator method adds a layer of abstraction:
//
// flatten()  → Flatten<Iter<BitmapIterator>>
// sorted()   → Sort<Flatten<...>>
// dedup()    → Dedup<Sort<Flatten<...>>>
//
// The compiler sees a deeply nested type:
// Dedup<Sort<Flatten<Iter<BitmapIterator>>>>
//
// Each layer has its own state machine for .next()
// Branch prediction can't optimize across layers
```

### Why Vectorization Failed

**1. State Machine Overhead**
```rust
// Each iterator is a state machine:
enum FlattenState {
    Yielding,
    BetweenIterators,
    Done,
}

fn next(&mut self) -> Option<Item> {
    match self.state {  // ← Branch!
        Yielding => { ... }
        BetweenIterators => { ... }  // ← Another branch!
        Done => None,
    }
}
```

**2. Indirect Calls**
```rust
// Iterator trait uses dynamic dispatch:
trait Iterator {
    fn next(&mut self) -> Option<Self::Item>;
}

// Compiler must generate indirect call:
call [rsi + Iterator_vtable::next]  ; ← Can't inline!
```

**3. Data Dependencies**
```rust
// sorted() needs to see ALL items before yielding any
// This creates a data dependency that prevents pipelining

for item in iterator {  // Can't yield until all items collected
    buffer.push(item);
}
buffer.sort();  // ← Must complete before continuing
```

### The "Zero-Cost" Myth

```
Rust's Promise: "Zero-cost abstractions"
Reality: Abstractions have costs, they're just deferred

Iterator chains look free at runtime because:
- Work is amortized across .next() calls
- Memory allocations are hidden
- Branch mispredictions are blamed on "the workload"

But the costs are real:
- Prevented inlining
- Prevented vectorization
- Cache inefficiency from pointer chasing
```

---

## The Fix: Manual Loop with SIMD

### Approach 1: Collect-First Strategy

```rust
// Instead of iterator chaining:
fn merge_iterators_old<'a>(
    iterators: Vec<BitmapIterator<'a>>,
) -> impl Iterator<Item = DocId> + 'a {
    iterators.into_iter()
        .flatten()
        .sorted()
        .dedup()
}

// Use a collect-first approach:
fn merge_iterators_new<'a>(
    iterators: Vec<BitmapIterator<'a>>,
) -> Vec<DocId> {
    // 1. Collect all doc IDs into a single buffer
    let mut all_docs = Vec::new();
    for mut iter in iterators {
        while let Some(doc_id) = iter.next() {
            all_docs.push(doc_id);
        }
    }

    // 2. Sort once
    all_docs.sort_unstable();

    // 3. Deduplicate in-place
    all_docs.dedup();

    all_docs
}
```

**Performance**: 150ms (1.5x improvement)

Still not great, but better. The issue is we're still processing one doc ID at a time.

### Approach 2: Batch Processing with SIMD

```rust
use std::arch::x86_64::*;

/// Process doc IDs in batches of 8 (AVX2)
fn merge_iterators_simd<'a>(
    iterators: Vec<BitmapIterator<'a>>,
) -> Vec<DocId> {
    // 1. Pre-allocate based on estimated size
    let estimated_size = iterators.iter()
        .map(|it| it.estimated_len())
        .sum();
    let mut all_docs = Vec::with_capacity(estimated_size);

    // 2. Batch collect
    for mut iter in iterators {
        // Process 8 at a time when possible
        let slice = iter.as_slice();  // Get underlying data
        let chunks = slice.chunks_exact(8);

        for chunk in chunks {
            // Load 8 doc IDs into SIMD register
            unsafe {
                let docs = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);
                // Store to output buffer
                _mm256_storeu_si256(
                    all_docs.as_mut_ptr().add(all_docs.len()) as *mut __m256i,
                    docs,
                );
                all_docs.set_len(all_docs.len() + 8);
            }
        }

        // Handle remainder
        for &doc_id in slice.chunks_exact(8).remainder() {
            all_docs.push(doc_id);
        }
    }

    // 3. Vectorized sort (using rayon for parallelism)
    all_docs.par_sort_unstable();

    // 4. Deduplicate
    all_docs.dedup();

    all_docs
}
```

**Performance**: 80ms (2.75x improvement from original)

### Approach 3: Bitmap-Specific Optimization

```rust
/// For bitmap-based iterators, we can do even better
fn merge_bitmaps_simd(bitmap_refs: &[&Bitmap]) -> Bitmap {
    let mut result = Bitmap::new();

    // Process 256 bits (8 u32s) at a time
    let chunk_words = 8;
    let num_chunks = bitmap_len / chunk_words;

    for chunk_idx in 0..num_chunks {
        let mut acc = [0u32; chunk_words];

        // OR all bitmaps together using SIMD
        for bitmap in bitmap_refs {
            unsafe {
                let src = bitmap.get_chunk(chunk_idx * chunk_words);
                let v_src = _mm256_loadu_si256(src.as_ptr() as *const __m256i);
                let v_acc = _mm256_loadu_si256(acc.as_ptr() as *const __m256i);
                let v_or = _mm256_or_si256(v_src, v_acc);
                _mm256_storeu_si256(acc.as_mut_ptr() as *mut __m256i, v_or);
            }
        }

        result.set_chunk(chunk_idx * chunk_words, &acc);
    }

    result
}
```

**Performance**: 47ms (4.7x improvement from original!)

---

## Assembly Comparison

### Before: Iterator Chain

```asm
merge_iterators_old:
    push    rbp
    mov     rbp, rsp
    push    r15
    push    r14
    push    r13
    push    r12
    push    rbx

.L_loop:
    ; Check if current iterator is done
    mov     rax, [rdi + 0x10]
    cmp     rax, [rdi + 0x18]
    jge     .L_next_iterator     ; ← Branch 1

    ; Get next item from iterator
    mov     rbx, [rdi]
    mov     rdx, [rbx + 0x28]    ; ← Vtable lookup
    call    rdx                  ; ← Indirect call!

    ; Check if None
    test    rax, rax
    je      .L_next_iterator     ; ← Branch 2

    ; Push to sorted buffer
    mov     [rcx + r8*8], rax
    inc     r8

    jmp     .L_loop              ; ← Always taken

.L_next_iterator:
    ; Advance to next iterator
    add     rdi, 0x20
    cmp     rdi, [rsp + 0x40]
    jl      .L_loop

    ; Sort and dedup
    call    sort_impl
    call    dedup_impl
```

**Key Issues:**
- Indirect call through vtable
- Multiple branches per iteration
- No SIMD instructions

### After: Manual Loop with SIMD

```asm
merge_iterators_simd:
    push    rbp
    mov     rbp, rsp

    ; Outer loop over bitmaps
.L_bitmap_loop:
    ; Load 8 doc IDs at once
    vmovups ymm0, [rdi + rcx*4]

    ; OR with accumulator
    vpor    ymm1, ymm0, ymm2

    ; Store result
    vmovups [rsi + rdx*4], ymm1

    ; Increment by 8 (not 1!)
    add     rcx, 8
    add     rdx, 8

    ; Single branch
    cmp     rcx, r8
    jl      .L_bitmap_loop

    ; Cleanup
    pop     rbp
    ret
```

**Key Improvements:**
- SIMD processes 8 items per instruction
- Single branch per iteration
- No indirect calls
- Sequential memory access (prefetcher-friendly)

---

## Lessons Learned

### Lesson 1: Profile Before Optimizing

```
Don't guess—measure!

Initial hypothesis: "BM25 ranking is slow"
Profile result: "Filter evaluation is slow"
Deep dive: "Iterator merge is the culprit"

Without profiling, we might have optimized the wrong thing.
```

### Lesson 2: "Zero-Cost" is Context-Dependent

```rust
// Zero-cost for:
// - Single simple operations
// - Tight loops with no state
// - Known types at compile time

// NOT zero-cost for:
// - Deeply nested chains
// - State machines with branches
// - Trait objects with dynamic dispatch
```

### Lesson 3: SIMD Requires Contiguous Data

```rust
// SIMD works best when:
// - Data is contiguous in memory
// - Processing is uniform across elements
// - No branches inside the loop

// Iterator chains break all three:
// - Pointer chasing between layers
// - State machines add divergence
// - Branches for None/Some handling
```

### Lesson 4: The Right Abstraction Level

```rust
// Too low-level (manual SIMD):
// + Maximum performance
// - Hard to maintain
// - Easy to get wrong

// Too high-level (iterator chains):
// + Easy to write
// + Composable
// - Hidden performance costs

// Just right (batch processing):
// + Good performance
// + Reasonable ergonomics
// + Clear intent

fn process_batch(docs: &[DocId]) -> Vec<ScoredDoc> {
    docs.par_iter()  // Parallel iteration
        .map(|&doc| score_doc(doc))
        .collect()
}
```

---

## Practical Guidelines

### When to Avoid Iterator Chains

```rust
// AVOID for performance-critical code:
iterators.into_iter()
    .flatten()
    .sorted()
    .dedup()
    .filter(...)
    .map(...)
    .collect()

// PREFER explicit loops:
let mut result = Vec::new();
for iter in iterators {
    for item in iter {
        if predicate(&item) {
            result.push(transform(item));
        }
    }
}
result.sort();
result.dedup();
```

### When to Use SIMD

```rust
// Use SIMD when:
// 1. Processing large arrays (>100 elements)
// 2. Same operation on all elements
// 3. Data is contiguous
// 4. Performance is critical

// Example: Bitmap operations
fn bitmap_or_simd(a: &[u64], b: &[u64]) -> Vec<u64> {
    assert_eq!(a.len(), b.len());

    let mut result = vec![0u64; a.len()];

    // Process 4 u64s at a time (AVX2)
    let chunks = a.len() / 4;
    for i in 0..chunks {
        unsafe {
            let va = _mm256_loadu_si256(a[i*4..].as_ptr() as *const __m256i);
            let vb = _mm256_loadu_si256(b[i*4..].as_ptr() as *const __m256i);
            let vor = _mm256_or_si256(va, vb);
            _mm256_storeu_si256(result[i*4..].as_mut_ptr() as *mut __m256i, vor);
        }
    }

    // Handle remainder
    for i in (chunks*4)..a.len() {
        result[i] = a[i] | b[i];
    }

    result
}
```

### Profiling Checklist

```bash
# 1. CPU profiling
perf record -g ./program
perf report

# 2. Assembly inspection
rustc --emit=asm program.rs
cat program.s

# 3. SIMD verification
objdump -d ./program | grep -E "vp|ymm|xmm"

# 4. Cache analysis
perf stat -e cache-references,cache-misses ./program

# 5. Flamegraph
cargo flamegraph --example benchmark
```

---

## Impact

### Before and After

```
┌─────────────────────────────────────────────────────────────┐
│              Filter Evaluation Latency                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Before (Iterator Chain)                                    │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  220ms  │
│                                                             │
│  After (SIMD Batch)                                         │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  47ms                               │
│                                                             │
│  Improvement: 4.7x faster                                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Current Production Performance

```
Note: This work predates FTS v2.

Current FTS v2 p90 latencies (hot cache):
- Single term: ~5ms
- Multi-term (5): ~10ms
- Filtered queries: ~15ms

The optimization described here contributed to the foundation
that made FTS v2 possible.
```

---

## Summary

### Key Takeaways

1. **"Zero-cost" abstractions aren't free**: Iterator chains add hidden overhead that prevents optimization

2. **SIMD requires explicit design**: The compiler can't vectorize through layers of abstraction

3. **Profile-driven optimization works**: We found the real bottleneck, not the assumed one

4. **Batch processing is key**: Processing multiple items together unlocks SIMD and amortizes overhead

### Design Principles

- **Measure first**: Always profile before optimizing
- **Know your abstractions**: Understand what they compile to
- **Batch when possible**: Process multiple items together
- **SIMD for hot paths**: Manual SIMD is worth it for critical code

### Future Work

- **Autovectorization hints**: Rust could benefit from SIMD hints
- **Iterator specialization**: Trait methods that enable vectorization
- **Better profiling tools**: SIMD-aware profilers for Rust

---

## Appendix: Complete Optimized Code

```rust
use std::arch::x86_64::*;

/// High-performance bitmap merger using SIMD
pub struct BitmapMerger<'a> {
    bitmaps: Vec<&'a Bitmap>,
    result: Bitmap,
}

impl<'a> BitmapMerger<'a> {
    pub fn new(bitmaps: Vec<&'a Bitmap>) -> Self {
        let result = Bitmap::with_capacity(
            bitmaps.iter().map(|b| b.len()).max().unwrap_or(0)
        );

        Self { bitmaps, result }
    }

    pub fn merge(&mut self) -> &Bitmap {
        let chunk_words = 8; // Process 8 u64s at a time
        let num_chunks = self.result.len() / (64 * chunk_words);

        for chunk_idx in 0..num_chunks {
            let mut acc = [0u64; chunk_words];

            for bitmap in &self.bitmaps {
                unsafe {
                    let src = bitmap.get_chunk(chunk_idx * chunk_words);
                    let v_src = _mm256_loadu_si256(
                        src.as_ptr() as *const __m256i
                    );
                    let v_acc = _mm256_loadu_si256(
                        acc.as_ptr() as *const __m256i
                    );
                    let v_or = _mm256_or_si256(v_src, v_acc);
                    _mm256_storeu_si256(
                        acc.as_mut_ptr() as *mut __m256i,
                        v_or
                    );
                }
            }

            self.result.set_chunk(chunk_idx * chunk_words, &acc);
        }

        &self.result
    }
}

/// Usage in filter evaluation
pub fn evaluate_contains_any_filter(
    bitmaps: Vec<&Bitmap>,
) -> Bitmap {
    let mut merger = BitmapMerger::new(bitmaps);
    merger.merge().clone()
}
```
