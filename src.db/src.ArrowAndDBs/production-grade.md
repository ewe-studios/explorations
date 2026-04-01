---
title: "ArrowAndDBs Production Deployment"
subtitle: "Deployment patterns for Arrow, DuckDB, DataFusion, and Polars"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.ArrowAndDBs
related: exploration.md
---

# Production-Grade ArrowAndDBs

## Overview

This document covers production deployment patterns for Arrow-based analytics systems.

## Part 1: DuckDB Deployment

### Embedded Analytics

```rust
use duckdb::{Connection, params};

// In-memory database
let conn = Connection::open_in_memory()?;

// Create table
conn.execute(
    "CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)",
    [],
)?;

// Insert data
conn.execute(
    "INSERT INTO users VALUES (?, ?, ?)",
    params![1, "Alice", 30],
)?;

// Query
let mut stmt = conn.prepare("SELECT * FROM users WHERE age > ?")?;
let rows = stmt.query_map(params![25], |row| {
    Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
})?;

// Read Parquet directly
let mut stmt = conn.prepare("SELECT * FROM 'data.parquet' WHERE age > 25")?;
```

### DuckDB Configuration

```rust
// Optimize DuckDB for production
let conn = Connection::open_in_memory()?;

// Set memory limit
conn.execute("SET memory_limit='4GB'", [])?;

// Set number of threads
conn.execute("SET threads=4", [])?;

// Enable persistence
conn.execute("PRAGMA enable_persistence", [])?;

// Configure temp directory
conn.execute("PRAGMA temp_directory='/tmp/duckdb'", [])?;
```

## Part 2: DataFusion Deployment

### Query Service

```rust
use datafusion::prelude::*;
use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct QueryRequest {
    sql: String,
}

#[derive(Serialize)]
struct QueryResponse {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[tokio::main]
async fn main() {
    let ctx = SessionContext::new();

    // Register tables
    ctx.register_csv("users", "users.csv", CsvReadOptions::new())
        .await
        .unwrap();

    let app = Router::new()
        .layer(AddExtensionLayer::new(ctx))
        .route("/query", post(handle_query));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handle_query(
    Extension(ctx): Extension<SessionContext>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    let df = ctx.sql(&req.sql).await.unwrap();
    let results = df.collect().await.unwrap();

    // Convert to JSON
    let columns = results[0].schema().fields().iter()
        .map(|f| f.name().clone())
        .collect();

    let rows = results.iter()
        .flat_map(|batch| {
            (0..batch.num_rows()).map(move |i| {
                (0..batch.num_columns())
                    .map(|j| format!("{:?}", batch.column(j).data_type()))
                    .collect()
            })
        })
        .collect();

    Json(QueryResponse { columns, rows })
}
```

## Part 3: Performance Tuning

### Memory Management

```rust
// Arrow memory configuration
use arrow::util::memory;

// Set memory pool limit
let pool = FairSpillPool::new(4 * 1024 * 1024 * 1024);  // 4GB

// DataFusion configuration
let config = SessionConfig::new()
    .with_batch_size(8192)  // Rows per batch
    .with_target_partitions(16)  // Parallelism
    .with_memory_limit(4 * 1024 * 1024 * 1024, 0.9);  // 4GB at 90%

let ctx = SessionContext::with_config(config);
```

### Parallel Execution

```rust
// Configure parallelism
let config = SessionConfig::new()
    .with_target_partitions(num_cpus::get())
    .with_repartition_joins(true)
    .with_repartition_aggregations(true)
    .with_repartition_windows(true);

// Enable hash join
let config = config.with_hash_join_single_partition_threshold(1024 * 1024);

// Enable sort preservation
let config = config.with_coalesce_batches(true);
```

---

*This document is part of the ArrowAndDBs exploration series. See [exploration.md](./exploration.md) for the complete index.*
