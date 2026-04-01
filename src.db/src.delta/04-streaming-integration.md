---
title: "Delta Lake Streaming Integration"
subtitle: "Structured Streaming, CDC, and real-time pipelines"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.delta
related: exploration.md
---

# 04 - Streaming Integration: Delta Lake

## Overview

This document covers Delta Lake's streaming capabilities - Structured Streaming integration, change data capture, and real-time data pipelines.

## Part 1: Structured Streaming

### Streaming Write

```python
from pyspark.sql.functions import current_timestamp, col

# Streaming write to Delta
df = spark \
    .readStream \
    .format("kafka") \
    .option("kafka.bootstrap.servers", "kafka:9092") \
    .option("subscribe", "events") \
    .load()

# Process and write to Delta
query = df \
    .selectExpr("CAST(key AS STRING)", "CAST(value AS STRING)", "current_timestamp() as ts") \
    .writeStream \
    .format("delta") \
    .outputMode("append") \
    .option("checkpointLocation", "/tmp/checkpoint") \
    .trigger(processingTime="10 seconds") \
    .start("/tmp/delta_table")

query.awaitTermination()
```

### Streaming Read

```python
# Streaming read from Delta
stream_df = spark \
    .readStream \
    .format("delta") \
    .option("ignoreChanges", "true") \
    .load("/tmp/delta_table")

# Process updates
result = stream_df.groupBy("user_id").count()

query = result \
    .writeStream \
    .format("parquet") \
    .outputMode("complete") \
    .option("checkpointLocation", "/tmp/checkpoint2") \
    .start("/tmp/output")
```

## Part 2: Change Data Feed

```python
# Enable change data feed (Delta Lake 2.0+)
spark.conf.set("spark.databricks.delta.changeDataFeed.enabled", "true")

# Read changes between versions
cdf_df = spark \
    .readStream \
    .format("delta") \
    .option("readChangeFeed", "true") \
    .option("startingVersion", 10) \
    .option("endingVersion", 20) \
    .load("/tmp/delta_table")

# CDF schema includes metadata
# _change_type: insert, update_preimage, update_postimage, delete
# _commit_version: version number
# _commit_timestamp: commit timestamp

# Process CDC
cdf_df.filter(col("_change_type") == "insert") \
    .writeStream \
    .foreachBatch(lambda df, id: process_inserts(df)) \
    .start()
```

## Part 3: Real-Time Pipelines

### Medallion Architecture

```
Medallion Architecture Pattern:

┌─────────────────────────────────────────────────────────┐
│ Bronze Layer (Raw)                                       │
│ - Raw streaming data                                     │
│ - Append-only                                            │
│ - Schema-on-read                                         │
│                                                          │
│ spark.readStream.kafka(...).writeStream.delta("bronze") │
└───────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│ Silver Layer (Cleaned)                                   │
│ - Filtered, validated                                    │
│ - Deduplicated                                           │
│ - Schema-enforced                                        │
│                                                          │
│ deduped = spark.readStream("bronze").dropDuplicates()    │
│ validated = apply_schema(deduped)                         │
│ validated.writeStream.format("delta").start("silver")    │
└───────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│ Gold Layer (Aggregated)                                  │
│ - Business-level aggregates                              │
│ - Dimensional modeling                                   │
│ - Query-optimized                                        │
│                                                          │
│ aggregated = spark.readStream("silver")                  │
│   .groupBy("category", window("ts", "1h")).count()       │
│ aggregated.writeStream.format("delta").start("gold")     │
└───────────────────────────────────────────────────────────┘
```

---

*This document is part of the Delta Lake exploration series. See [exploration.md](./exploration.md) for the complete index.*
