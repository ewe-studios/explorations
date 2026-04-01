---
title: "Zero to Analytics Engineer: ArrowAndDBs"
subtitle: "OLAP fundamentals and columnar analytics"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.ArrowAndDBs
related: exploration.md
---

# 00 - Zero to Analytics Engineer: ArrowAndDBs

## Overview

This document explains OLAP (Online Analytical Processing) fundamentals - how analytical databases differ from transactional databases, and why columnar storage is key to analytics performance.

## Part 1: OLAP vs OLTP

```
OLTP (Online Transaction Processing) vs OLAP (Online Analytical Processing):

┌───────────────────────────────────────────────────────────┐
│ OLTP - Transactional Workloads                             │
│                                                            │
│ Characteristics:                                           │
│ - Many small writes (INSERT, UPDATE, DELETE)              │
│ - Point reads (SELECT by primary key)                     │
│ - High concurrency (hundreds/thousands of users)          │
│ - Low latency requirements (< 100ms)                      │
│ - ACID transactions critical                              │
│                                                            │
│ Examples:                                                  │
│ - E-commerce order processing                              │
│ - Banking transactions                                     │
│ - User session management                                  │
│                                                            │
│ Systems: PostgreSQL, MySQL, TigerBeetle                   │
└───────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────┐
│ OLAP - Analytical Workloads                                │
│                                                            │
│ Characteristics:                                           │
│ - Bulk loads (batch inserts)                              │
│ - Scans (SELECT with aggregations)                        │
│ - Low concurrency (tens of users)                         │
│ - High throughput important                               │
│ - Read-optimized (writes are batched)                     │
│                                                            │
│ Examples:                                                  │
│ - Business intelligence dashboards                        │
│ - Data science queries                                     │
│ - Log analysis                                             │
│                                                            │
│ Systems: DuckDB, ClickHouse, DataFusion, BigQuery         │
└───────────────────────────────────────────────────────────┘

Key Differences:
┌──────────────────┬──────────────────┬──────────────────┐
│ Aspect           │ OLTP             │ OLAP             │
├──────────────────┼──────────────────┼──────────────────┤
│ Data model       │ Row-oriented     │ Column-oriented  │
│ Query pattern    │ Point lookups    │ Full scans       │
│ Write pattern    │ Random writes    │ Bulk loads       │
│ Optimization     │ Index lookups    │ Vectorized ops   │
│ Compression      │ Light            │ Heavy            │
│ Concurrency      │ High             │ Low              │
└──────────────────┴──────────────────┴──────────────────┘
```

## Part 2: Columnar Storage

```
Row-Oriented vs Column-Oriented Storage:

Row-Oriented (OLTP):
┌─────────────────────────────────────────────────────────┐
│ Data layout on disk:                                     │
│                                                          │
│ [Row 1: id, name, age, score]                            │
│ [Row 2: id, name, age, score]                            │
│ [Row 3: id, name, age, score]                            │
│ [Row 4: id, name, age, score]                            │
│                                                          │
│ Memory layout:                                            │
│ [1, "Alice", 30, 95.5, 2, "Bob", 25, 87.3, ...]         │
│                                                          │
│ Best for:                                                 │
│ ✓ Reading/writing complete rows                           │
│ ✓ Point lookups by ID                                     │
│ ✗ Aggregations (must scan all columns)                   │
│ ✗ Compression (mixed data types)                          │
└───────────────────────────────────────────────────────────┘

Column-Oriented (OLAP):
┌─────────────────────────────────────────────────────────┐
│ Data layout on disk:                                     │
│                                                          │
│ Column: id    [1, 2, 3, 4, ...]                          │
│ Column: name  ["Alice", "Bob", "Charlie", "Diana", ...] │
│ Column: age   [30, 25, 35, 28, ...]                      │
│ Column: score [95.5, 87.3, 92.1, 88.9, ...]              │
│                                                          │
│ Memory layout:                                            │
│ id:    [1, 2, 3, 4, ...]                                 │
│ name:  ["Alice", "Bob", ...]                              │
│ age:   [30, 25, 35, 28, ...]                             │
│ score: [95.5, 87.3, 92.1, 88.9, ...]                     │
│                                                          │
│ Best for:                                                 │
│ ✓ Aggregations (scan only needed columns)                │
│ ✓ Compression (same type, similar values)                │
│ ✓ Vectorized operations (SIMD)                           │
│ ✗ Full row lookups (must assemble from columns)          │
└───────────────────────────────────────────────────────────┘

Why Columnar for Analytics?

Query: SELECT AVG(age), MAX(score) FROM users WHERE age > 25

Row-oriented execution:
1. Scan all rows
2. For each row: parse all columns
3. Filter by age > 25
4. Compute AVG(age), MAX(score)
5. Read: 4 columns × N rows

Column-oriented execution:
1. Scan only age column
2. Filter age > 25
3. Compute AVG(age)
4. Scan score column (only matching rows)
5. Compute MAX(score)
6. Read: 2 columns × N rows (50% less I/O!)
```

## Part 3: Apache Arrow

```
Apache Arrow - Columnar Memory Format Standard:

Purpose: Zero-copy data exchange between systems

┌───────────────────────────────────────────────────────────┐
│ Arrow Design Goals                                         │
│                                                            │
│ 1. Columnar layout                                         │
│    - Efficient analytics                                   │
│    - SIMD-friendly                                         │
│                                                            │
│ 2. Language agnostic                                       │
│    - Same format in C++, Rust, Python, Java, etc.         │
│    - No serialization needed                               │
│                                                            │
│ 3. Zero-copy                                               │
│    - Direct memory access                                  │
│    - FFI without conversion                                │
│                                                            │
│ 4. Extensible                                              │
│    - Custom types                                          │
│    - Metadata support                                      │
│                                                            │
│ 5. Hardware optimized                                      │
│    - Cache-aligned                                         │
│    - SIMD operations                                       │
│    - GPU-friendly                                          │
└───────────────────────────────────────────────────────────┘

Arrow Data Types:
```
Primitive Types:
- INT8, INT16, INT32, INT64
- UINT8, UINT16, UINT32, UINT64
- FLOAT16, FLOAT32, FLOAT64
- BOOLEAN, DATE32, DATE64, TIMESTAMP

Nested Types:
- LIST (array)
- STRUCT (record)
- UNION
- MAP

Binary/String:
- BINARY, LARGE_BINARY
- STRING, LARGE_STRING
```

Arrow Record Batch:
```rust
RecordBatch {
    schema: Schema {
        fields: [
            Field { name: "id", datatype: Int32, nullable: false },
            Field { name: "name", datatype: Utf8, nullable: true },
        ]
    },
    columns: [
        Int32Array: [1, 2, 3, 4, 5],
        StringArray: ["Alice", "Bob", "Charlie", "Diana", "Eve"],
    ],
    row_count: 5,
}
```
```

## Part 4: Vectorized Execution

```
Vectorized Query Execution:

Traditional (row-by-row) execution:
```
for each row:
    for each column:
        value = read_value(row, column)
        if filter(value):
            result.append(value)
```

Problems:
- Branch mispredictions
- No SIMD utilization
- Function call overhead

Vectorized execution:
```
for each column_vector:
    # Process 1024 values at once
    filtered = column_vector.filter(predicate)
    result.append(filtered)
```

Benefits:
- SIMD: Process 4-16 values per instruction
- Fewer branch mispredictions
- Amortized function call overhead
- Cache-friendly access patterns

Example: Vectorized Filter
```rust
// Traditional
for row in rows {
    if row.age > 25 {
        results.push(row);
    }
}

// Vectorized
let ages: Int32Vector = batch.column("age");
let mask: BooleanVector = ages.gt(25);  // SIMD
let filtered: RecordBatch = batch.filter(&mask);
```
```

---

*This document is part of the ArrowAndDBs exploration series. See [exploration.md](./exploration.md) for the complete index.*
