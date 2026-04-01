---
title: "Delta Lake Query Execution Deep Dive"
subtitle: "Data skipping, statistics, and query optimization"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.delta
related: 00-zero-to-lakehouse-engineer.md, 01-transaction-log-deep-dive.md
---

# 02 - Query Execution Deep Dive: Delta Lake

## Overview

This document covers Delta Lake query execution - how data skipping works, statistics collection, and query optimization techniques.

## Part 1: Data Skipping

### Statistics Collection

```
File Statistics (stored in add action):

{
  "numRecords": 100000,
  "minValues": {
    "id": 1,
    "user_id": 100,
    "date": "2024-01-01",
    "amount": 10.50
  },
  "maxValues": {
    "id": 100000,
    "user_id": 9999,
    "date": "2024-12-31",
    "amount": 9999.99
  },
  "nullCount": {
    "id": 0,
    "user_id": 5,
    "date": 0,
    "amount": 100
  }
}

Statistics Usage:

Query: SELECT * FROM table WHERE id = 50000

File Evaluation:
┌──────────┬──────────┬──────────┬──────────────┐
│ File     │ min(id)  │ max(id)  │ Skip?        │
├──────────┼──────────┼──────────┼──────────────┤
│ file_a   │ 1        │ 25000    │ No (1 <= 50000 <= 25000) │
│ file_b   │ 25001    │ 50000    │ No (25001 <= 50000 <= 50000) │
│ file_c   │ 50001    │ 75000    │ Yes (50000 < 50001) │
│ file_d   │ 75001    │ 100000   │ Yes (50000 < 75001) │
└──────────┴──────────┴──────────┴──────────────┘

Result: Skip 50% of files!
```

### Predicate Pushdown

```rust
/// Data skipping predicate evaluation
pub struct DataSkippingIndex {
    files: Vec<FileStats>,
}

struct FileStats {
    path: String,
    min_values: HashMap<String, Value>,
    max_values: HashMap<String, Value>,
    null_counts: HashMap<String, u64>,
}

impl DataSkippingIndex {
    /// Check if file might contain matching rows
    pub fn might_match(&self, file: &FileStats, predicate: &Predicate) -> bool {
        match predicate {
            Predicate::Equals(col, value) => {
                // Check if value is within min/max range
                let min = file.min_values.get(col);
                let max = file.max_values.get(col);

                match (min, max) {
                    (Some(min), Some(max)) => {
                        value >= min && value <= max
                    }
                    _ => true, // No stats, can't skip
                }
            }

            Predicate::GreaterThan(col, value) => {
                // Skip if max <= value
                file.max_values.get(col)
                    .map_or(true, |max| max > value)
            }

            Predicate::LessThan(col, value) => {
                // Skip if min >= value
                file.min_values.get(col)
                    .map_or(true, |min| min < value)
            }

            Predicate::IsNull(col) => {
                // Skip if null_count is 0
                file.null_counts.get(col)
                    .map_or(true, |count| *count > 0)
            }

            Predicate::And(left, right) => {
                self.might_match(file, left) && self.might_match(file, right)
            }

            Predicate::Or(left, right) => {
                self.might_match(file, left) || self.might_match(file, right)
            }
        }
    }

    /// Filter files using predicate
    pub fn filter_files(&self, predicate: &Predicate) -> Vec<&str> {
        self.files.iter()
            .filter(|f| self.might_match(f, predicate))
            .map(|f| f.path.as_str())
            .collect()
    }
}
```

## Part 2: Bloom Filters

### Bloom Filter Index

```
Bloom Filter for Equality Predicates:

Problem: Min/max doesn't help with equality (id = 42)
Solution: Bloom filter for efficient existence checking

Bloom Filter Structure:
┌─────────────────────────────────────────────────────────┐
│ Bloom Filter (bit array + hash functions)                │
│                                                          │
│ For column "user_id" in file_a.parquet:                  │
│                                                          │
│ Bits: [0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0]  │
│                                                          │
│ Hash functions: h1(x), h2(x), h3(x)                      │
│                                                          │
│ To check if user_id = 123 exists:                        │
│ 1. Compute h1(123) = position 2 -> bit is 0             │
│ 2. Compute h2(123) = position 5 -> bit is 0             │
│ 3. Compute h3(123) = position 10 -> bit is 0            │
│                                                          │
│ Any bit is 0 = Definitely NOT in file (skip!)           │
│ All bits are 1 = Might be in file (must read)           │
│                                                          │
│ False positive rate: ~1% (tunable)                       │
│ False negatives: Impossible                              │
└───────────────────────────────────────────────────────────┘

Bloom Filter Storage:
- Stored in _delta_index/ directory
- One filter per column per file
- Compact: ~1KB per 100K distinct values

Usage:
```sql
-- Automatically used for equality predicates
SELECT * FROM table WHERE user_id = 123

-- Skip files where bloom filter says "definitely not"
-- Reduces I/O for high-cardinality columns
```
```

## Part 3: Z-Order Clustering

### Multi-Dimensional Clustering

```
Z-Order (Interleaved Bits):

Problem: Single-dimensional sorting doesn't help multi-column queries
Solution: Z-Order clustering for multi-dimensional pruning

Z-Order Concept:
┌─────────────────────────────────────────────────────────┐
│ 2D Space (user_id, date):                                │
│                                                          │
│ date                                                     │
│   ^                                                      │
│   │  A  B  C  D                                          │
│ 3│  0  0  1  1                                          │
│ 2│  0  0  1  1                                          │
│ 1│  1  1  0  0                                          │
│ 0│  1  1  0  0                                          │
│   +───────────────────> user_id                          │
│     0  1  2  3                                           │
│                                                          │
│ Z-Order Value (interleave bits):                         │
│ - (0,0) -> 00 = 0                                        │
│ - (1,0) -> 10 = 2                                        │
│ - (0,1) -> 01 = 1                                        │
│ - (1,1) -> 11 = 3                                        │
│                                                          │
│ Files sorted by Z-Order value:                           │
│ - Proximity in 2D space preserved in 1D ordering         │
│ - Range queries on BOTH dimensions benefit               │
└───────────────────────────────────────────────────────────┘

OPTIMIZE with Z-Order:
```sql
-- Cluster data by multiple columns
OPTIMIZE table_name ZORDER BY (user_id, date);

-- Query benefits from both columns
SELECT * FROM table
WHERE user_id = 123 AND date = '2024-01-01';

-- Data skipping uses BOTH predicates together
```

Benefits:
- 10-100x faster for multi-column filters
- Reduces data scanned by 90%+ typical
- Best for: High-cardinality columns used in WHERE
```

---

*This document is part of the Delta Lake exploration series. See [exploration.md](./exploration.md) for the complete index.*
