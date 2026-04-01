---
title: "ArrowAndDBs Columnar Storage Deep Dive"
subtitle: "Arrow memory layout, compression, and encoding"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.ArrowAndDBs
related: 00-zero-to-analytics-engineer.md
---

# 01 - Columnar Storage Deep Dive: ArrowAndDBs

## Overview

This document covers columnar storage internals - Arrow memory layout, compression techniques, and encoding schemes used by analytical databases.

## Part 1: Arrow Memory Layout

### Array Structure

```
Arrow Array Memory Layout:

Every Arrow array has two buffers:
1. Value buffer: Actual data
2. Validity bitmap: Null tracking

Int32 Array Example:
┌─────────────────────────────────────────────────────────┐
│ [1, null, 3, 4, null, 6]                                 │
│                                                          │
│ Validity Bitmap (1 byte per 8 values):                   │
│ [1, 0, 1, 1, 0, 1, 0, 0] = 0b00101101 = 0x2D            │
│  ^LSB                                                    │
│  Bit 0 = 1 (value 0 is valid)                            │
│  Bit 1 = 0 (value 1 is null)                             │
│  Bit 2 = 1 (value 2 is valid)                            │
│  ...                                                     │
│                                                          │
│ Value Buffer (contiguous):                               │
│ [1, ?, 3, 4, ?, 6, ?, ?]  (4 bytes each)                │
│ ? = undefined (don't care, null values ignored)          │
│                                                          │
│ Total size: 1 byte bitmap + 24 bytes values = 25 bytes   │
└───────────────────────────────────────────────────────────┘

Variable-Length Data (Strings):
┌─────────────────────────────────────────────────────────┐
│ ["hello", "world", null, "foo"]                          │
│                                                          │
│ Validity Bitmap: [1, 1, 0, 1] = 0x05                    │
│                                                          │
│ Offset Buffer (i32, length+1):                          │
│ [0, 5, 10, 10, 13]                                       │
│  ^    ^    ^    ^    ^                                   │
│  |    |    |    |    └─ End of last string              │
│  |    |    |    └────── Null has same offset as prev    │
│  |    |    └─────────── "world" ends at 10              │
│  |    └──────────────── "hello" ends at 5               │
│  └───────────────────── Start at 0                      │
│                                                          │
│ Value Buffer (UTF8 bytes):                              │
│ [h e l l o w o r l d f o o]                             │
│                                                          │
│ String reconstruction:                                   │
│ str[i] = value_buffer[offsets[i]..offsets[i+1]]         │
└───────────────────────────────────────────────────────────┘

Struct Type (nested):
┌─────────────────────────────────────────────────────────┐
│ {x: Int32, y: Float64}[]                                 │
│                                                          │
│ Child Arrays (stored separately):                        │
│ x: Int32Array [1, 2, 3, ...]                            │
│ y: Float64Array [1.0, 2.0, 3.0, ...]                    │
│                                                          │
│ Each child has its own validity bitmap                   │
└───────────────────────────────────────────────────────────┘

List Type (arrays):
┌─────────────────────────────────────────────────────────┐
│ [[1, 2], [3, 4, 5], null, [6]]                          │
│                                                          │
│ Offset Buffer: [0, 2, 5, 5, 6]                          │
│  (same as string offsets)                               │
│                                                          │
│ Child Values: [1, 2, 3, 4, 5, 6]                        │
│                                                          │
│ Validity: [1, 1, 0, 1]                                   │
└───────────────────────────────────────────────────────────┘
```

### Buffer Alignment

```
Memory Alignment for SIMD:

Arrow buffers are aligned to 64 bytes (cache line):
- Enables efficient SIMD loads
- Avoids split cache lines
- Optimal for GPU transfer

Alignment guarantee:
```rust
const ALIGNMENT: usize = 64;

fn allocate_aligned(size: usize) -> Vec<u8> {
    // Allocate with extra space for alignment
    let mut buf = vec![0u8; size + ALIGNMENT];
    let ptr = buf.as_mut_ptr();

    // Align pointer
    let aligned = (ptr as usize + ALIGNMENT - 1) & !(ALIGNMENT - 1);
    let offset = aligned - ptr as usize;

    // Return aligned slice
    &mut buf[offset..offset + size]
}
```
```

## Part 2: Compression Techniques

### Dictionary Encoding

```
Dictionary Encoding for Low-Cardinality Columns:

Problem: String columns have repeated values
Solution: Store unique values once, use indices

Original:
["red", "blue", "red", "green", "blue", "red", "blue"]

Dictionary:
[0: "red", 1: "blue", 2: "green"]

Indices:
[0, 1, 0, 2, 1, 0, 1]

Space savings:
- Original: 7 × 4 bytes (UTF8 avg) = 28 bytes
- Encoded: 3 × 4 bytes (dict) + 7 × 1 byte (indices) = 19 bytes
- Savings: ~32%

Benefits:
- Reduced storage
- Faster equality checks (compare indices)
- Efficient grouping (hash indices, not strings)

Used in:
- Parquet RLE_DICTIONARY encoding
- Arrow DictionaryArray
```

### Run-Length Encoding (RLE)

```
RLE for Repeated Values:

Problem: Many consecutive identical values
Solution: Store (value, count) pairs

Original:
[0, 0, 0, 0, 0, 1, 1, 2, 2, 2, 2, 0, 0]

RLE:
[(0, 5), (1, 2), (2, 4), (0, 2)]
(value, run_length)

Bit-packed RLE (Parquet):
- Header: (run_length << 1) | is_value_bit
- For short runs: pack values densely

Example with 3-bit values:
Original: [0, 0, 0, 1, 1, 7, 7, 7]
Encoded:  [header:10, value:0, header:6, value:1, header:10, value:7]
          (header = run_length << 1, indicates RLE when LSB=0)

Excellent for:
- Boolean columns (high repetition)
- Sorted columns
- Partition columns
```

### Delta Encoding

```
Delta Encoding for Monotonic Data:

Problem: Sorted integers waste bits
Solution: Store differences (deltas)

Original (sorted IDs):
[1000, 1005, 1008, 1015, 1020, 1025]

Delta:
[1000, 5, 3, 7, 5, 5]
(first value, then deltas)

Delta-of-deltas (for linear sequences):
Original: [100, 105, 110, 115, 120]
Delta:    [100, 5, 5, 5, 5]
DoD:      [100, 5, 0, 0, 0]  (even better compression!)

Used in:
- Timestamp columns
- Auto-increment IDs
- Sorted numeric columns
```

### Bloom Filters

```
Bloom Filters for Predicate Pushdown:

Problem: Must read file to check if value exists
Solution: Bloom filter for quick existence check

Bloom filter structure:
- Bit array of m bits
- k hash functions

Algorithm:
```
Insert(x):
  for h in hashes:
    bits[h(x)] = 1

Query(x):
  for h in hashes:
    if bits[h(x)] == 0:
      return FALSE  # Definitely not present
  return MAYBE  # Probably present (false positive possible)
```

Configuration:
- m = 10 bits per value
- k = 7 hash functions
- False positive rate: ~1%

Usage in Parquet/Delta:
- One bloom filter per column per row group
- Stored in file metadata
- Checked before reading row group
```

---

*This document is part of the ArrowAndDBs exploration series. See [exploration.md](./exploration.md) for the complete index.*
