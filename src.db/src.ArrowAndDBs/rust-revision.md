---
title: "ArrowAndDBs Rust Revision"
subtitle: "Arrow, DataFusion, and Polars in Rust"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.ArrowAndDBs
related: exploration.md
---

# Rust Revision: ArrowAndDBs

## Overview

This document covers the Rust implementations in the Arrow ecosystem - Arrow itself, DataFusion query engine, and Polars DataFrame library.

## Part 1: Apache Arrow Rust

### Creating Arrays

```rust
use arrow::array::{Int32Array, StringArray, Float64Array, BooleanArray};
use arrow::record_batch::RecordBatch;
use arrow::datatypes::{Schema, Field, DataType};
use std::sync::Arc;

// Create primitive array
let int_array = Int32Array::from(vec![1, 2, 3, 4, 5]);

// Create array with nulls
let int_with_nulls = Int32Array::from(vec![Some(1), None, Some(3), None, Some(5)]);

// Create string array
let str_array = StringArray::from(vec!["Alice", "Bob", "Charlie", "Diana", "Eve"]);

// Create boolean array (for filters)
let bool_array = BooleanArray::from(vec![true, false, true, false, true]);

// Create float array
let float_array = Float64Array::from(vec![1.5, 2.5, 3.5, 4.5, 5.5]);
```

### Creating RecordBatches

```rust
// Define schema
let schema = Schema::new(vec![
    Field::new("id", DataType::Int32, false),
    Field::new("name", DataType::Utf8, false),
    Field::new("age", DataType::Int32, true),  // nullable
    Field::new("score", DataType::Float64, true),
]);

// Create RecordBatch
let batch = RecordBatch::try_new(
    Arc::new(schema),
    vec![
        Arc::new(Int32Array::from(vec![1, 2, 3, 4, 5])),
        Arc::new(StringArray::from(vec!["Alice", "Bob", "Charlie", "Diana", "Eve"])),
        Arc::new(Int32Array::from(vec![Some(30), Some(25), None, Some(35), Some(28)])),
        Arc::new(Float64Array::from(vec![95.5, 87.3, 92.1, 88.9, 91.0])),
    ],
).unwrap();

// Access columns
let id_col = batch.column(0).as_any().downcast_ref::<Int32Array>().unwrap();
let name_col = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();

// Get row count
assert_eq!(batch.num_rows(), 5);

// Get schema
let schema = batch.schema();
```

### Array Operations

```rust
use arrow::compute::*;

// Filter
let mask = BooleanArray::from(vec![true, false, true, false, true]);
let filtered = filter(&int_array, &mask).unwrap();

// Take (gather by indices)
let indices = Int32Array::from(vec![0, 2, 4]);
let taken = take(&int_array, &indices, None).unwrap();

// Sort
let sorted = sort(&int_array, None).unwrap();

// Comparison
let other = Int32Array::from(vec![1, 3, 3, 4, 5]);
let eq = equal(&int_array, &other).unwrap();  // [true, false, true, true, true]
let gt = greater(&int_array, &other).unwrap(); // [false, false, false, false, false]

// Aggregation
let sum = arrow::compute::sum(&int_array).unwrap();  // 15
let min = arrow::compute::min(&int_array).unwrap();  // 1
let max = arrow::compute::max(&int_array).unwrap();  // 5
```

## Part 2: DataFusion

### Query Execution

```rust
use datafusion::prelude::*;
use datafusion::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Create execution context
    let ctx = SessionContext::new();

    // Register table
    ctx.register_csv("users", "users.csv", CsvReadOptions::new()).await?;

    // SQL query
    let df = ctx.sql("SELECT name, age FROM users WHERE age > 25").await?;
    let results = df.collect().await?;

    // DataFrame API
    let df = ctx.table("users").await?;
    let df = df.filter(col("age").gt(lit(25)))?;
    let df = df.select(vec![col("name"), col("age")])?;
    let results = df.collect().await?;

    Ok(())
}
```

### Custom Functions

```rust
use datafusion::logical_expr::{ScalarUDF, Volatility};
use datafusion::physical_expr::ColumnarValue;
use arrow::array::{ArrayRef, Int32Array};

// Define custom scalar function
fn my_function(args: &[ColumnarValue]) -> Result<ColumnarValue> {
    let array = args[0].clone().into_array(1);
    let int_array = array.as_any().downcast_ref::<Int32Array>().unwrap();

    // Apply transformation
    let result: Int32Array = int_array.iter()
        .map(|v| v.map(|x| x * 2))
        .collect();

    Ok(ColumnarValue::Array(Arc::new(result)))
}

// Register function
ctx.register_udf(ScalarUDF::new(
    "my_function",
    &Signature::exact(vec![DataType::Int32], Volatility::Immutable),
    &ReturnType::new(Arc::new(DataType::Int32)),
    &my_function,
));
```

## Part 3: Polars

### DataFrame Operations

```rust
use polars::prelude::*;

// Create DataFrame
let df = df! {
    "name" => ["Alice", "Bob", "Charlie", "Diana", "Eve"],
    "age" => [30, 25, 35, 28, 32],
    "score" => [95.5, 87.3, 92.1, 88.9, 91.0],
}.unwrap();

// Filter
let filtered = df.filter(&df.column("age").unwrap().gt(25)).unwrap();

// Select columns
let selected = df.select(["name", "age"]).unwrap();

// Group by aggregation
let grouped = df.groupby(["age"])
    .unwrap()
    .agg(vec![("score", ["mean", "max"])])
    .unwrap();

// Sort
let sorted = df.sort(["age"], false).unwrap();

// Join
let joined = df1.join(&df2, ["id"], ["id"], JoinType::Inner).unwrap();

// Lazy evaluation (query optimization)
let lazy_result = df.lazy()
    .filter(col("age").gt(lit(25)))
    .groupby([col("age")])
    .agg(vec![col("score").mean()])
    .collect()
    .unwrap();
```

---

*This document is part of the ArrowAndDBs exploration series. See [exploration.md](./exploration.md) for the complete index.*
