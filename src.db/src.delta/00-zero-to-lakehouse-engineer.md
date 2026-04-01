---
title: "Zero to Lakehouse Engineer: Delta Lake"
subtitle: "Understanding data lakes, ACID transactions, and the lakehouse architecture"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.delta
related: exploration.md
---

# 00 - Zero to Lakehouse Engineer: Delta Lake

## Overview

This document explains the fundamentals of Delta Lake - why it exists, what problems it solves, and how it brings database reliability to data lakes.

## Part 1: Why Delta Lake Exists

### The Data Lake Problem

```
Traditional Data Lake Architecture:
┌─────────────────────────────────────────────────────────┐
│                    Data Lake (S3/HDFS)                   │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐           │
│  │ users/    │  │ orders/   │  │ products/ │           │
│  │ parquet   │  │ parquet   │  │ parquet   │           │
│  │           │  │           │  │           │           │
│  │ Problems: │  │           │  │           │           │
│  │ ✗ No transactions        │  │           │           │
│  │ ✗ No schema enforcement  │  │           │           │
│  │ ✗ No updates/deletes     │  │           │           │
│  │ ✗ No time travel         │  │           │           │
│  │ ✗ No data quality        │  │           │           │
│  └───────────┘  └───────────┘  └───────────┘           │
│                                                          │
│  Concurrent writes cause corruption:                      │
│  Writer 1: writes file_a.parquet                          │
│  Writer 2: writes file_b.parquet                          │
│  Reader: reads partial state (a only or b only)           │
│  Result: Inconsistent queries!                            │
└───────────────────────────────────────────────────────────┘

Data Warehouse Alternative:
┌─────────────────────────────────────────────────────────┐
│                   Data Warehouse                         │
│  (Snowflake, Redshift, BigQuery)                        │
│                                                          │
│  Benefits:                                               │
│  ✓ ACID transactions                                     │
│  ✓ Schema enforcement                                    │
│  ✓ Updates/deletes                                       │
│  ✓ Time travel                                           │
│  ✓ High performance                                      │
│                                                          │
│  Problems:                                               │
│  ✗ Expensive (10-100x data lake cost)                   │
│  ✗ Vendor lock-in                                        │
│  ✗ Data must be loaded (not direct access)              │
│  ✗ Limited scale for raw data                           │
└─────────────────────────────────────────────────────────┘

Delta Lake Solution:
┌─────────────────────────────────────────────────────────┐
│                    Lakehouse Architecture                │
│                                                          │
│  Data Lake Storage (cheap, scalable)                    │
│       +                                                   │
│  Transaction Layer (Delta Lake)                          │
│       =                                                   │
│  Warehouse-quality data at lake cost                     │
│                                                          │
│  Benefits:                                               │
│  ✓ ACID transactions (serializable isolation)            │
│  ✓ Schema enforcement (write-time validation)            │
│  ✓ Updates/deletes (MERGE, UPDATE, DELETE)               │
│  ✓ Time travel (query any version)                       │
│  ✓ Unified batch + streaming                             │
│  ✓ Open format (Parquet + JSON log)                      │
│  ✓ 1/10th warehouse cost                                 │
└─────────────────────────────────────────────────────────┘
```

### What is a Transaction Log?

```
Transaction Log Pattern:

Instead of modifying data files directly, log all changes:

┌─────────────────────────────────────────────────────────┐
│ Transaction Log (append-only)                            │
│                                                          │
│ Commit 0:                                                │
│   ADD file_a.parquet (1000 rows, id: 1-1000)             │
│   ADD file_b.parquet (1000 rows, id: 1001-2000)          │
│                                                          │
│ Commit 1:                                                │
│   UPDATE file_a.parquet SET age = age + 1                │
│   REMOVE file_a.parquet                                  │
│   ADD file_a_v2.parquet (1000 rows, updated ages)        │
│                                                          │
│ Commit 2:                                                │
│   DELETE WHERE id > 1500                                 │
│   REMOVE file_b.parquet                                  │
│   ADD file_b_v2.parquet (500 rows, id: 1001-1500)        │
│                                                          │
│ Current State (computed from log):                       │
│   - file_a_v2.parquet                                    │
│   - file_b_v2.parquet                                    │
│                                                          │
│ Key insight: Data files are IMMUTABLE                    │
│ Changes create new files, log tracks current state       │
└─────────────────────────────────────────────────────────┘

Benefits of Log-Based Architecture:
1. Atomicity - Commit is all-or-nothing
2. Consistency - Readers see consistent snapshots
3. Isolation - Concurrent writes don't conflict
4. Durability - Log is source of truth
5. Time Travel - Replay log to any point
6. Audit Trail - Complete history of changes
```

## Part 2: Delta Lake Core Concepts

### Table Structure

```
Delta Table Components:

┌─────────────────────────────────────────────────────────┐
│ Delta Table Directory                                    │
│                                                          │
│ /table_name/                                             │
│ ├── _delta_log/              <-- Metadata layer          │
│ │   ├── 00000000000000000000.json                       │
│ │   ├── 00000000000000000001.json                       │
│ │   ├── ...                                             │
│ │   └── 00000000000000000010.checkpoint.parquet         │
│ │                                                       │
│ ├── part-00000-abc123.parquet  <-- Data files           │
│ ├── part-00001-def456.parquet                           │
│ ├── part-00002-ghi789.parquet                           │
│ │                                                       │
│ └── _delta_index/              <-- Optional indices     │
│     └── bloom_filter_...                                │
└─────────────────────────────────────────────────────────┘

Delta Log Entry Structure:
```json
{
  "commitInfo": {
    "timestamp": 1679875200000,
    "userId": "user123",
    "operation": "WRITE",
    "operationParameters": {"mode": "Overwrite"},
    "notebook": {"notebookId": "abc123"},
    "clientVersion": "delta-core_2.12-2.4.0"
  }
}

{
  "add": {
    "path": "part-00000-abc123.parquet",
    "size": 1234567,
    "partitionValues": {"date": "2024-01-01"},
    "modificationTime": 1679875200000,
    "dataChange": true,
    "stats": {
      "numRecords": 10000,
      "minValues": {"id": 1, "age": 18},
      "maxValues": {"id": 10000, "age": 65},
      "nullCount": {"id": 0, "age": 0}
    },
    "tags": {"quality": "gold"}
  }
}

{
  "remove": {
    "path": "part-00001-old.parquet",
    "deletionTimestamp": 1679875300000,
    "dataChange": true,
    "extendedFileMetadata": true
  }
}
```
```

### ACID Properties in Delta Lake

```
ACID in Delta Lake:

┌─────────────────────────────────────────────────────────┐
│ Atomicity - All or Nothing                               │
│                                                          │
│ Write Operation:                                         │
│ 1. Write new Parquet files                               │
│ 2. Create log entry with all changes                     │
│ 3. Atomically commit log entry (rename)                  │
│                                                          │
│ If step 3 fails:                                         │
│ - New files exist but aren't in log                      │
│ - Readers don't see incomplete write                     │
│ - Next writer can clean up orphaned files                │
│                                                          │
│ If step 3 succeeds:                                      │
│ - All changes visible atomically                         │
│ - No partial state possible                              │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Consistency - Valid States Only                          │
│                                                          │
│ Schema Enforcement:                                      │
│ - Write rejected if columns don't match                  │
│ - Type checking (can't write string to int column)       │
│ - Nullability enforcement                                │
│                                                          │
│ Data Quality Rules:                                      │
│ - CHECK constraints (Delta Lake 2.0+)                    │
│ - Example: age >= 0, email LIKE '%@%'                    │
│ - Violations rejected at write time                      │
│                                                          │
│ Invariant: Sum of file stats = table stats               │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Isolation - Serializable Snapshot Isolation              │
│                                                          │
│ MVCC (Multi-Version Concurrency Control):                │
│                                                          │
│ Writer 1 (t=0):                                          │
│   - Reads snapshot at version 0                          │
│   - Writes files, commits at version 1                   │
│                                                          │
│ Writer 2 (t=0):                                          │
│   - Reads snapshot at version 0                          │
│   - Writes files, tries to commit at version 2           │
│   - CONFLICT: Both modified same data                    │
│   - Writer 2 must retry with new snapshot                │
│                                                          │
│ Reader (any time):                                       │
│   - Sees consistent snapshot at one version              │
│   - Never sees partial writes                            │
│   - Never blocks writers                                 │
│                                                          │
│ Serializable: Equivalent to serial execution             │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Durability - Committed = Permanent                       │
│                                                          │
│ Cloud Storage (S3, ADLS, GCS):                          │
│ - Object storage is durable (11 9s)                      │
│ - Log entries are immutable once committed               │
│ - Checkpoint provides redundancy                         │
│                                                          │
│ Consistency Considerations:                              │
│ - S3: Eventually consistent (but atomic for PUT)         │
│ - ADLS/GCS: Strongly consistent                          │
│ - Delta handles eventual consistency via log polling     │
└─────────────────────────────────────────────────────────┘
```

## Part 3: Common Operations

### Read Operations

```
Reading Delta Tables:

Time Travel - Query Historical Versions:
```python
# Read current version
df = spark.read.format("delta").load("/tmp/table")

# Read by version number
df = spark.read.format("delta").option(
    "versionAsOf", 5
).load("/tmp/table")

# Read by timestamp
df = spark.read.format("delta").option(
    "timestampAsOf", "2024-01-15T10:00:00Z"
).load("/tmp/table")

# SQL syntax
spark.sql("""
    SELECT * FROM delta.`/tmp/table` VERSION AS OF 5
""")

spark.sql("""
    SELECT * FROM delta.`/tmp/table` TIMESTAMP AS OF '2024-01-15'
""")
```

Schema Evolution:
```python
# Automatic schema evolution
df.write.format("delta") \
    .mode("append") \
    .option("mergeSchema", "true") \
    .save("/tmp/table")

# Adds new columns automatically
# Existing rows have NULL for new columns
```
```

### Write Operations

```
Writing Delta Tables:

Append:
```python
df.write.format("delta").mode("append").save("/tmp/table")
```

Overwrite (Atomic):
```python
# Atomic overwrite - readers see old or new, never partial
df.write.format("delta").mode("overwrite").save("/tmp/table")

# Partition-overwrite
df.write.format("delta").mode("overwrite") \
    .option("replaceWhere", "date = '2024-01-15'") \
    .save("/tmp/table")
```

Merge (Upsert):
```python
from delta.tables import DeltaTable

delta_tbl = DeltaTable.forPath(spark, "/tmp/table")

delta_tbl.alias("target").merge(
    source_df.alias("source"),
    "target.id = source.id"
) \
.whenMatchedUpdate(set={
    "name": "source.name",
    "age": "source.age"
}) \
.whenNotMatchedInsert(values={
    "id": "source.id",
    "name": "source.name",
    "age": "source.age"
}) \
.execute()
```

Update:
```python
delta_tbl.update(
    condition="age < 30",
    set={"age": "age + 1"}
)
```

Delete:
```python
delta_tbl.delete(condition="age > 100")
```
```

## Part 4: Performance Optimization

### Data Skipping

```
Delta Lake Data Skipping:

Problem: Scanning entire table is slow
Solution: Use file statistics to skip irrelevant files

How it works:
┌─────────────────────────────────────────────────────────┐
│ Query: SELECT * FROM table WHERE id = 42                 │
│                                                          │
│ File Statistics (in log):                                │
│ ┌──────────────┬──────────┬──────────┬──────────┐       │
│ │ File         │ min(id)  │ max(id)  │ Skipped? │       │
│ ├──────────────┼──────────┼──────────┼──────────┤       │
│ │ file_a       │ 1        │ 1000     │ No       │       │
│ │ file_b       │ 1001     │ 2000     │ Yes!     │       │
│ │ file_c       │ 2001     │ 3000     │ Yes!     │       │
│ │ file_d       │ 3001     │ 4000     │ Yes!     │       │
│ └──────────────┴──────────┴──────────┴──────────┘       │
│                                                          │
│ Only scan file_a - skip 75% of data!                     │
└─────────────────────────────────────────────────────────┘

Statistics collected automatically:
- min/max values per column
- null counts
- row counts
- bloom filters (optional, for equality predicates)

Configurations:
```python
# Enable data skipping
spark.conf.set("spark.databricks.delta.stats.required", "id,age")

# Disable for faster writes
spark.conf.set("spark.databricks.delta.stats.collect", "false")
```
```

### File Compaction

```
Small File Problem:

Cause: Many small writes create many small files
Effect: Too many files = slow listing, slow reads

Solution: Compaction (OPTIMIZE)
```python
# Compact small files
spark.sql("OPTIMIZE /tmp/table")

# Compact specific partitions
spark.sql("OPTIMIZE /tmp/table WHERE date = '2024-01-01'")

# Z-Order clustering (multi-dimensional pruning)
spark.sql("""
    OPTIMIZE /tmp/table
    ZORDER BY (user_id, date)
""")
```

Z-Order Benefits:
- Co-locates related data
- Improves data skipping for multi-column queries
- Example: ZORDER BY (user_id, date)
  - Queries filtering on user_id AND date benefit most
```

### Caching

```
Delta Lake Caching:

Metadata Cache:
```python
# Cache table metadata
spark.conf.set("spark.databricks.delta.cache.enabled", "true")
```

Data Cache:
```python
# Cache frequently accessed data
df.cache()
df.count()  # Materialize cache
```

Delta Cache (Databricks):
- Local SSD cache for remote data
- Automatic for repeated queries
```

---

*This document is part of the Delta Lake exploration series. See [exploration.md](./exploration.md) for the complete index.*
