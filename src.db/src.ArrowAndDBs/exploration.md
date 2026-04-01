---
title: "ArrowAndDBs: Complete Exploration"
subtitle: "Columnar analytics databases"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.ArrowAndDBs
repository: https://github.com/apache/arrow
explored_at: 2026-03-28
status: COMPLETE
---

# ArrowAndDBs: Complete Exploration

## Overview

Collection of columnar analytics databases and tools built around Apache Arrow:
- **Apache Arrow** - In-memory columnar format standard
- **DuckDB** - Embedded analytical SQL database
- **DataFusion** - Modular query engine in Rust
- **Polars** - Fast DataFrame library in Rust
- **Apache Iceberg** - Open table format for data lakes

### Components

| Project | Purpose | Language | License |
|---------|---------|----------|---------|
| Arrow | Columnar memory format | C++/Rust/Python | Apache 2.0 |
| DuckDB | Embedded OLAP SQL | C++ | MIT |
| DataFusion | Query engine | Rust | Apache 2.0 |
| Polars | DataFrame library | Rust | MIT |
| Iceberg | Table format | Java/Python | Apache 2.0 |

### Documents

| Document | Description |
|----------|-------------|
| [exploration.md](./exploration.md) | Overview |
| [00-zero-to-analytics-engineer.md](./00-zero-to-analytics-engineer.md) | OLAP fundamentals |
| [01-columnar-storage-deep-dive.md](./01-columnar-storage-deep-dive.md) | Columnar storage |
| [02-vectorized-execution-deep-dive.md](./02-vectorized-execution-deep-dive.md) | Vectorized query execution |
| [rust-revision.md](./rust-revision.md) | Arrow/DataFusion/Polars in Rust |
| [production-grade.md](./production-grade.md) | Deployment patterns |

---

## Arrow Columnar Format

```
Arrow Record Batch:
┌─────────────────────────────────────────┐
│ Schema                                  │
│ - name: UTF8                            │
│ - age: INT64                            │
│ - score: FLOAT64                        │
├─────────────────────────────────────────┤
│ Column: name                            │
│ [Alice, Bob, Charlie, ...]              │
├─────────────────────────────────────────┤
│ Column: age                             │
│ [30, 25, 35, ...]                       │
├─────────────────────────────────────────┤
│ Column: score                           │
│ [95.5, 87.3, 92.1, ...]                 │
└─────────────────────────────────────────┘

Key benefits:
- Zero-copy data access
- SIMD-friendly layout
- Language-agnostic (FFI)
- Cache-efficient
```

---

## Quick Start

```rust
// Arrow basics
use arrow::array::{Int32Array, StringArray};
use arrow::record_batch::RecordBatch;
use arrow::datatypes::{Schema, Field, DataType};

// Create arrays
let names = StringArray::from(vec!["Alice", "Bob", "Charlie"]);
let ages = Int32Array::from(vec![30, 25, 35]);

// Create schema
let schema = Schema::new(vec![
    Field::new("name", DataType::Utf8, false),
    Field::new("age", DataType::Int32, false),
]);

// Create RecordBatch
let batch = RecordBatch::try_new(
    Arc::new(schema),
    vec![Arc::new(names), Arc::new(ages)],
).unwrap();
```

```python
# DuckDB embedded analytics
import duckdb

# In-memory database
con = duckdb.connect()

# Create table and insert data
con.execute("CREATE TABLE users (name VARCHAR, age INTEGER)")
con.execute("INSERT INTO users VALUES ('Alice', 30), ('Bob', 25)")

# Query
result = con.execute("SELECT * FROM users WHERE age > 25").fetchall()

# Read Parquet directly
result = con.execute("SELECT * FROM 'data.parquet'").fetchall()

# Pandas integration
import pandas as pd
df = pd.DataFrame({'name': ['Alice', 'Bob'], 'age': [30, 25]})
con.execute("INSERT INTO users SELECT * FROM df")
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-28 | Full exploration completed |
| 2026-03-28 | Added 00-zero-to-analytics-engineer.md |
| 2026-03-28 | Added 01-columnar-storage-deep-dive.md |
| 2026-03-28 | Added 02-vectorized-execution-deep-dive.md |
| 2026-03-28 | Added rust-revision.md |
| 2026-03-28 | Added production-grade.md |
