---
title: "ArrowAndDBs Vectorized Execution Deep Dive"
subtitle: "SIMD operations, query pipelines, and optimization"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.ArrowAndDBs
related: 00-zero-to-analytics-engineer.md, 01-columnar-storage-deep-dive.md
---

# 02 - Vectorized Execution Deep Dive: ArrowAndDBs

## Overview

This document covers vectorized query execution - how analytical databases process data in batches using SIMD operations for maximum throughput.

## Part 1: SIMD Fundamentals

### SIMD Instructions

```
SIMD (Single Instruction, Multiple Data):

CPU SIMD Registers:
┌─────────────────────────────────────────────────────────┐
│ SSE (128-bit): 4 × float32 or 2 × float64                │
│ AVX2 (256-bit): 8 × float32 or 4 × float64               │
│ AVX-512 (512-bit): 16 × float32 or 8 × float64           │
│ NEON (ARM, 128-bit): 4 × float32 or 2 × float64          │
└───────────────────────────────────────────────────────────┘

Vectorized Addition Example:

Scalar (one at a time):
```
a[0] + b[0] = c[0]
a[1] + b[1] = c[1]
a[2] + b[2] = c[2]
a[3] + b[3] = c[3]
# 4 instructions
```

SIMD (4 at a time with AVX):
```
[ a[0], a[1], a[2], a[3] ]
+ [ b[0], b[1], b[2], b[3] ]
= [ c[0], c[1], c[2], c[3] ]
# 1 instruction (vaddps)
```

Speedup: 4x for float32, 8x for float64 (AVX2)

SIMD Operations Available:
- Arithmetic: add, sub, mul, div
- Comparison: gt, lt, eq, neq
- Logical: and, or, xor
- Shuffle: permute, blend, gather
- Reduction: sum, min, max
```

### Vectorized Filter

```rust
/// Vectorized filter with SIMD
use std::arch::x86_64::*;

/// Filter Int32 array: keep values > threshold
unsafe fn filter_gt_simd(values: &[i32], threshold: i32) -> Vec<i32> {
    let mut result = Vec::new();

    // Broadcast threshold to SIMD register
    let thresh_vec = _mm256_set1_epi32(threshold);

    // Process 8 values at a time (AVX2)
    let chunks = values.chunks_exact(8);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let vals = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);

        // Compare: vals > threshold
        let mask = _mm256_cmpgt_epi32(vals, thresh_vec);

        // Extract matching values
        let mask_bits = _mm256_movemask_epi8(mask) as u32;
        for i in 0..8 {
            if (mask_bits >> i) & 1 != 0 {
                result.push(chunk[i]);
            }
        }
    }

    // Handle remainder
    for &val in remainder {
        if val > threshold {
            result.push(val);
        }
    }

    result
}

/// Boolean vector operations (for filter masks)
struct BooleanVector {
    bits: Vec<u64>,
    len: usize,
}

impl BooleanVector {
    /// AND two boolean vectors (for combining filters)
    fn and(&self, other: &Self) -> Self {
        let bits = self.bits.iter()
            .zip(other.bits.iter())
            .map(|(&a, &b)| a & b)
            .collect();

        Self { bits, len: self.len }
    }

    /// Count set bits (for cardinality)
    fn count_ones(&self) -> usize {
        self.bits.iter().map(|&b| b.count_ones() as usize).sum()
    }
}
```

## Part 2: Query Execution Pipeline

### Vectorized Query Plan

```
Query: SELECT SUM(age) FROM users WHERE age > 25 AND city = 'NYC'

Vectorized Execution Plan:
┌─────────────────────────────────────────────────────────┐
│ Step 1: Scan 'age' column                                │
│   - Load 1024 values into SIMD register                  │
│   - Output: Int32Vector[1024]                            │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 2: Filter age > 25                                  │
│   - Compare: vcmpgt_epi32(age, 25)                       │
│   - Output: BooleanVector[1024] (mask)                   │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 3: Scan 'city' column                               │
│   - Load dictionary indices                              │
│   - Output: Int32Vector[1024]                            │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 4: Filter city = 'NYC'                              │
│   - Compare: vcmpeq_epi32(city, nyc_id)                  │
│   - Output: BooleanVector[1024] (mask)                   │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 5: Combine masks (AND)                              │
│   - mask = (age > 25) AND (city = 'NYC')                 │
│   - Output: BooleanVector[1024]                          │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 6: Apply mask to age values                         │
│   - Compress: keep only matching values                  │
│   - Output: Int32Vector[matched]                         │
└───────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 7: SUM aggregation                                  │
│   - SIMD horizontal add                                  │
│   - Output: i64 (scalar sum)                             │
└───────────────────────────────────────────────────────────┘
```

### Batch Processing

```rust
/// Vectorized query executor
pub struct VectorizedExecutor {
    batch_size: usize,  // Typically 1024 or 2048
}

impl VectorizedExecutor {
    /// Execute query in batches
    pub fn execute(&self, batches: &[RecordBatch]) -> i64 {
        let mut total_sum = 0i64;

        for batch in batches {
            // Process one batch
            total_sum += self.process_batch(batch);
        }

        total_sum
    }

    fn process_batch(&self, batch: &RecordBatch) -> i64 {
        let age_col = batch.column("age").as_int32();
        let city_col = batch.column("city").as_int32();

        // Create filter masks
        let age_mask = age_col.gt(25);
        let city_mask = city_col.eq(CITY_NYC_ID);

        // Combine masks
        let combined_mask = age_mask.and(&city_mask);

        // Apply mask and sum
        let filtered = age_col.filter(&combined_mask);
        filtered.sum()
    }
}

/// Column trait for vectorized operations
trait Column {
    fn gt(&self, value: i32) -> BooleanVector;
    fn eq(&self, value: i32) -> BooleanVector;
    fn filter(&self, mask: &BooleanVector) -> Self;
    fn sum(&self) -> i64;
}
```

## Part 3: Aggregation

### Vectorized SUM

```rust
/// SIMD SUM aggregation
unsafe fn sum_simd(values: &[i32]) -> i64 {
    let mut sum = 0i64;

    // Process 8 values at a time (AVX2)
    let chunks = values.chunks_exact(8);
    let remainder = chunks.remainder();

    // Accumulator vector
    let mut acc = _mm256_setzero_si256();

    for chunk in chunks {
        let vals = _mm256_loadu_si256(chunk.as_ptr() as *const __m256i);
        acc = _mm256_add_epi32(acc, vals);
    }

    // Horizontal sum of accumulator
    let acc_vec = _mm256_hadd_epi32(acc, acc);
    let acc_vec = _mm256_hadd_epi32(acc_vec, acc_vec);
    sum += _mm256_extract_epi32(acc_vec, 0) as i64;
    sum += _mm256_extract_epi32(acc_vec, 1) as i64;

    // Remainder
    for &val in remainder {
        sum += val as i64;
    }

    sum
}
```

### Hash Aggregation (GROUP BY)

```rust
/// Vectorized GROUP BY aggregation
use std::collections::HashMap;

pub fn group_by_sum(keys: &[i32], values: &[i32]) -> HashMap<i32, i64> {
    let mut result: HashMap<i32, i64> = HashMap::new();

    // Process in batches
    for (key, value) in keys.iter().zip(values.iter()) {
        *result.entry(*key).or_insert(0) += *value as i64;
    }

    result
}

/// SIMD-optimized hash table (for large datasets)
pub struct VectorizedHashTable {
    keys: Vec<i32>,
    values: Vec<i64>,
    hashes: Vec<u32>,
}

impl VectorizedHashTable {
    /// Insert batch of key-value pairs
    pub fn insert_batch(&mut self, keys: &[i32], values: &[i32]) {
        // Compute all hashes first (can SIMD)
        let hashes: Vec<u32> = keys.iter()
            .map(|&k| MurmurHash3::hash(k))
            .collect();

        // Insert with probing
        for (i, (&key, &value)) in keys.iter().zip(values.iter()).enumerate() {
            let mut pos = hashes[i] as usize % self.keys.len();

            loop {
                if self.keys[pos] == 0 {
                    // Empty slot
                    self.keys[pos] = key;
                    self.values[pos] += value as i64;
                    self.hashes[pos] = hashes[i];
                    break;
                } else if self.keys[pos] == key {
                    // Found existing key
                    self.values[pos] += value as i64;
                    break;
                }
                // Probe to next slot
                pos = (pos + 1) % self.keys.len();
            }
        }
    }
}
```

---

*This document is part of the ArrowAndDBs exploration series. See [exploration.md](./exploration.md) for the complete index.*
