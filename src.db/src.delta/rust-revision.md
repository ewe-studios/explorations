---
title: "Delta Lake Rust Revision"
subtitle: "delta-rs implementation and usage patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.delta
related: exploration.md
---

# Rust Revision: Delta Lake

## Overview

This document covers delta-rs - the native Rust implementation of Delta Lake, providing read/write access to Delta tables without Spark.

## Part 1: delta-rs Architecture

### Core Crates

```
delta-rs Ecosystem:

┌──────────────────────────────────────────────────────────┐
│ Crate           │ Purpose                                │
├──────────────────────────────────────────────────────────┤
│ deltalake       │ Main crate - table operations          │
│ deltalake-core  │ Core types and traits                  │
│ deltalake-aws   │ AWS S3 storage backend                 │
│ deltalake-gcp   │ GCP GCS storage backend                │
│ deltalake-azure │ Azure ADLS storage backend             │
└──────────────────────────────────────────────────────────┘

Dependencies:
- arrow: Columnar data format
- parquet: Parquet read/write
- object_store: Cloud storage abstraction
- tokio: Async runtime
```

### Table Operations

```rust
use deltalake::{DeltaTable, DeltaTableBuilder, DeltaOps};
use arrow::record_batch::RecordBatch;

/// Open existing Delta table
pub async fn open_table(table_uri: &str) -> Result<DeltaTable> {
    let mut table = DeltaTableBuilder::from_uri(table_uri).build()?;
    table.load().await?;
    Ok(table)
}

/// Read table data
pub async fn read_table(table: &DeltaTable) -> Result<Vec<RecordBatch>> {
    use deltalake::datafusion::prelude::*;

    // Create DataFusion context
    let ctx = SessionContext::new();

    // Register Delta table
    ctx.register_table("delta_table", Arc::new(table.clone()))?;

    // Query
    let df = ctx.sql("SELECT * FROM delta_table").await?;
    df.collect().await
}

/// Write data to Delta table
pub async fn write_table(
    table_uri: &str,
    batches: Vec<RecordBatch>,
) -> Result<DeltaTable> {
    let mut table = DeltaTableBuilder::from_uri(table_uri).build()?;

    // Write operation
    DeltaOps(table)
        .write(batches)
        .with_save_mode(SaveMode::Append)
        .await?;

    Ok(table)
}

/// Update with predicate
pub async fn update_table(
    table: &mut DeltaTable,
    predicate: &str,
    updates: HashMap<&str, &str>,
) -> Result<()> {
    DeltaOps(table)
        .update()
        .with_predicate(predicate)
        .with_updates(updates)
        .await?;

    Ok(())
}

/// Merge (upsert)
pub async fn merge_table(
    table: &mut DeltaTable,
    source: DataFrame,
    predicate: &str,
) -> Result<()> {
    DeltaOps(table)
        .merge(source, predicate)
        .with_update(
            "target.age = source.age",
            "target.id = source.id",
        )
        .with_insert("source.id NOT IN (SELECT id FROM target)")
        .await?;

    Ok(())
}
```

## Part 2: Transaction Implementation

```rust
/// Delta transaction log writer
pub struct DeltaLogWriter {
    storage: Box<dyn StorageBackend>,
    table_uri: String,
}

impl DeltaLogWriter {
    /// Write new commit atomically
    pub async fn write_commit(
        &self,
        actions: Vec<Action>,
        version: i64,
    ) -> Result<i64> {
        // Create commit JSON
        let commit_json = serde_json::to_string(&actions)?;

        // Write to temp file
        let temp_path = format!(
            "{}/_delta_log/{:020}.json.tmp",
            self.table_uri, version
        );
        let commit_path = format!(
            "{}/_delta_log/{:020}.json",
            self.table_uri, version
        );

        // Atomic write
        self.storage.put(&temp_path, commit_json.as_bytes()).await?;

        // Atomic rename
        match self.storage.rename_if_not_exists(&temp_path, &commit_path).await {
            Ok(()) => Ok(version),
            Err(StorageError::AlreadyExists) => {
                // Conflict - another writer committed first
                Err(Error::CommitConflict(version))
            }
            Err(e) => Err(e.into()),
        }
    }
}

/// Optimistic transaction
pub struct DeltaTransaction {
    table: DeltaTable,
    read_version: i64,
    actions: Vec<Action>,
}

impl DeltaTransaction {
    pub fn new(table: DeltaTable) -> Self {
        let read_version = table.version();
        Self {
            table,
            read_version,
            actions: Vec::new(),
        }
    }

    pub fn add_action(&mut self, action: Action) {
        self.actions.push(action);
    }

    pub async fn commit(mut self) -> Result<i64> {
        let writer = DeltaLogWriter {
            storage: self.table.storage.clone(),
            table_uri: self.table.table_uri.clone(),
        };

        // Try to commit
        match writer.write_commit(self.actions, self.read_version + 1).await {
            Ok(version) => Ok(version),
            Err(Error::CommitConflict(_)) => {
                // Conflict - reload table and retry
                self.table.update().await?;
                Err(Error::CommitConflict(self.read_version))
            }
            Err(e) => Err(e),
        }
    }
}
```

---

*This document is part of the Delta Lake exploration series. See [exploration.md](./exploration.md) for the complete index.*
