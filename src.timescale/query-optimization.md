# Query Optimization in TimescaleDB: Planning, Indexes, and Execution

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.timescale/timescaledb/`

---

## Table of Contents

1. [Query Planning Overview](#query-planning-overview)
2. [Hypertable Expansion](#hypertable-expansion)
3. [Partition Pruning](#partition-pruning)
4. [Index Strategies](#index-strategies)
5. [Custom Scan Nodes](#custom-scan-nodes)
6. [Query Execution](#query-execution)
7. [Performance Tuning](#performance-tuning)

---

## Query Planning Overview

### PostgreSQL Planner Integration

TimescaleDB hooks into PostgreSQL's query planner at multiple points:

```
┌─────────────────────────────────────────────────────────────────┐
│                   POSTGRESQL QUERY PLANNER                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. parse_analyze()                                              │
│     └─> Timescale: Hypertable validation                        │
│                                                                  │
│  2. planner()                                                    │
│     └─> Timescale: PlanHook for hypertable expansion            │
│                                                                  │
│  3. subquery_planning()                                          │
│     └─> Timescale: Chunk constraint extraction                  │
│                                                                  │
│  4. grouping_planner()                                           │
│     └─> Timescale: Partial aggregation optimization             │
│                                                                  │
│  5. create_plan()                                                │
│     └─> Timescale: Custom scan node injection                   │
│                                                                  │
│  6. execute_plan()                                               │
│     └─> Timescale: Custom executor hooks                        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Planner Hook Points

```c
// Key hook registrations in TimescaleDB
void _PG_init(void) {
    // Save previous hooks
    prev_planner = planner_hook;
    planner_hook = ts_planner;

    prev_set_rel_pathlist = set_rel_pathlist_hook;
    set_rel_pathlist_hook = ts_set_rel_pathlist;

    prev_get_relation_info = get_relation_info_hook;
    get_relation_info_hook = ts_get_relation_info;
}
```

---

## Hypertable Expansion

### Transformation Process

Hypertables are transformed into UNION ALL of chunks:

```sql
-- User query
SELECT avg(temperature)
FROM conditions
WHERE time >= '2024-01-15' AND time < '2024-01-16';

-- After hypertable expansion (simplified)
SELECT avg(temperature)
FROM (
    SELECT temperature FROM _hyper_1_2_chunk WHERE ...
    UNION ALL
    SELECT temperature FROM _hyper_1_3_chunk WHERE ...
    UNION ALL
    SELECT temperature FROM _hyper_1_4_chunk WHERE ...
    -- ... all chunks for the day
) subquery
WHERE time >= '2024-01-15' AND time < '2024-01-16';
```

### Expansion Code Flow

```c
// Simplified expansion logic
RelOptInfo *expand_hypertable(
    PlannerInfo *root,
    RangeTblEntry *rte,
    Index rt_index
) {
    Hypertable *ht = ts_hypertable_get_by_name(rte->relname);

    // 1. Get chunks for the hypertable
    List *chunks = chunk_get_chunks_for_time_range(
        ht,
        extract_time_constraints(rte->where_clause)
    );

    // 2. Create append relation
    RelOptInfo *append_rel = create_append_rel(chunks);

    // 3. Add constraints for partition pruning
    add_partition_constraints(append_rel, chunks);

    // 4. Return expanded relation
    return append_rel;
}
```

---

## Partition Pruning

### Constraint-Based Pruning

Chunks are excluded based on query constraints:

```c
// Constraint contradiction check
bool chunk_contradicts_constraints(
    Chunk *chunk,
    List *constraints
) {
    foreach(constraint, constraints) {
        if (constraint_is_contradictory(chunk, constraint)) {
            return true;  // Chunk can be excluded
        }
    }
    return false;
}

// Example: Time range contradiction
bool time_constraint_contradictory(
    Chunk *chunk,
    Node *constraint
) {
    // Extract chunk time range
    Datum chunk_start = chunk_get_time_start(chunk);
    Datum chunk_end = chunk_get_time_end(chunk);

    // Extract constraint range
    Datum query_start, query_end;
    extract_time_range(constraint, &query_start, &query_end);

    // Check for non-overlap
    // [chunk_start, chunk_end) ∩ [query_start, query_end] = ∅
    if (chunk_end <= query_start || chunk_start > query_end) {
        return true;  // Contradiction found
    }

    return false;
}
```

### Pruning Examples

```sql
-- Query with time filter
EXPLAIN SELECT * FROM conditions
WHERE time >= '2024-01-15 10:00' AND time < '2024-01-15 12:00';

-- Pruned plan (only relevant chunks)
Append
  -> Seq Scan on _hyper_1_15_chunk
     Filter: (time >= '10:00' AND time < '12:00')
  -> Seq Scan on _hyper_1_16_chunk
     Filter: (time >= '10:00' AND time < '12:00')
  -- Chunks for other days are pruned

-- Query with space filter
EXPLAIN SELECT * FROM conditions
WHERE location = 'office';

-- With hash partitioning, all chunks scanned but filtered
Append
  -> Seq Scan on _hyper_1_2_chunk
     Filter: (location = 'office')
  -> Seq Scan on _hyper_1_3_chunk
     Filter: (location = 'office')
  -- ... all chunks

-- Query with both filters - optimal pruning
EXPLAIN SELECT * FROM conditions
WHERE time >= '2024-01-15' AND location = 'office';

Append
  -> Seq Scan on _hyper_1_15_chunk
     Filter: (time >= '...' AND location = 'office')
  -- Only chunks for Jan 15
```

### Runtime Pruning

For prepared statements and parameters, runtime pruning occurs during execution:

```c
// Runtime pruning state
typedef struct RuntimePruneState {
    ExprContext *econtext;        // Expression evaluation context
    List *prunable_chunks;        // Chunks that can be pruned
    ExprState *pruning_expr;      // Pruning predicate
} RuntimePruneState;

// During execution
void runtime_prune_chunks(
    CustomScanState *node,
    RuntimePruneState *prune_state
) {
    // Evaluate pruning expression with current parameters
    bool should_prune = ExecEvalExpr(
        prune_state->pruning_expr,
        prune_state->econtext,
        &is_null
    );

    if (should_prune) {
        // Skip this chunk
        node->csstate.ps.state->es_ignored = true;
    }
}
```

---

## Index Strategies

### Time Index (Required)

Every chunk has an index on the time column:

```sql
-- Automatically created by TimescaleDB
CREATE INDEX ON _hyper_1_2_chunk (time DESC);
```

**Query Pattern:**
```sql
-- Efficient with time index
SELECT * FROM conditions
WHERE time >= NOW() - INTERVAL '1 hour'
ORDER BY time DESC
LIMIT 100;

-- Index scan plan
Index Scan using _hyper_1_2_chunk_time_idx on _hyper_1_2_chunk
  Index Cond: (time >= (now() - '01:00:00'::interval))
```

### Segmentby Indexes (Recommended)

For frequently filtered columns:

```sql
-- Create composite index for common query pattern
CREATE INDEX ON conditions (location, time DESC);

-- On chunks:
CREATE INDEX ON _hyper_1_2_chunk (location, time DESC);
```

**Query Pattern:**
```sql
-- Efficient with composite index
SELECT * FROM conditions
WHERE location = 'office'
  AND time >= NOW() - INTERVAL '1 hour'
ORDER BY time DESC;

-- Index scan with both conditions
Index Scan using _hyper_1_2_chunk_location_time_idx on _hyper_1_2_chunk
  Index Cond: ((location = 'office') AND (time >= ...))
```

### Covering Indexes

Include additional columns to avoid heap access:

```sql
-- Covering index for temperature queries
CREATE INDEX ON conditions (location, time DESC)
INCLUDE (temperature);

-- Index-only scan possible
SELECT location, time, temperature
FROM conditions
WHERE location = 'office'
  AND time >= NOW() - INTERVAL '1 hour';

-- Index Only Scan plan
Index Only Scan using _hyper_1_2_chunk_covering_idx
  Index Cond: ((location = 'office') AND (time >= ...))
```

### Expression Indexes

For transformed values:

```sql
-- Index on time bucket
CREATE INDEX ON conditions (time_bucket('1 hour', time));

-- Efficient bucketed queries
SELECT time_bucket('1 hour', time), avg(temperature)
FROM conditions
WHERE time_bucket('1 hour', time) >= NOW() - INTERVAL '1 day'
GROUP BY 1;
```

### BRIN Indexes for Time

Block Range indexes for very large chunks:

```sql
-- BRIN index for time (smaller than B-tree)
CREATE INDEX ON conditions USING BRIN (time);

-- Good for:
-- - Sequential time scans
-- - Minimal index overhead
-- - Less effective for point lookups
```

### Index Selection

```sql
-- Query analyzer chooses index based on:
-- 1. Selectivity estimates
-- 2. Index correlation
-- 3. Cost estimation

-- Force specific index if needed
SET enable_seqscan = off;
SET enable_indexscan = on;
```

---

## Custom Scan Nodes

### Hypertable Custom Scan

TimescaleDB injects custom scan nodes for hypertables:

```c
// Custom scan structure
typedef struct HypertableScan {
    CustomScan custom_scan;
    Hypertable *ht;
    List *chunks;
    List *pruned_chunks;
} HypertableScan;

// Custom scan methods
CustomScanMethods hypertable_scan_methods = {
    .CustomName = "HypertableScan",
    .CreateCustomScanState = create_hypertable_scan_state,
    .RecheckCustomScanData = recheck_hypertable_scan,
};
```

### GapFill Custom Scan

For time-series gap filling:

```sql
-- Gap fill query
SELECT
  time_bucket_gapfill('1 hour', time, start, end) as bucket,
  interpolate(avg(value)) as value
FROM metrics
GROUP BY 1
ORDER BY 1;
```

```
┌────────────────────────────────────────────────────────────┐
│                   GAPFILL EXECUTION PLAN                    │
├────────────────────────────────────────────────────────────┤
│                                                             │
│  GapFill                                                   │
│    │ output: bucket, interpolated_value                    │
│    │ bucket_size: 1 hour                                   │
│    │ start: 2024-01-01 00:00                               │
│    │ end: 2024-01-01 23:00                                 │
│    │                                                        │
│    └─> Sort (by bucket)                                    │
│          │                                                  │
│          └─> HashAggregate (GROUP BY bucket)               │
│                │                                            │
│                └─> HypertableScan                           │
│                      │                                      │
│                      └─> Chunks                             │
│                                                             │
└────────────────────────────────────────────────────────────┘
```

### Skip Scan for DISTINCT

Efficient DISTINCT queries on segmentby columns:

```sql
-- Skip scan efficiently finds distinct values
SELECT DISTINCT location FROM conditions;

-- Without skip scan: Full table scan + hash/dedup
-- With skip scan: Jump to each distinct value in index
```

```c
// Skip scan algorithm
void skip_scan_next(
    SkipScanState *state,
    IndexScanDesc *scan
) {
    // Get current value from index
    Datum current = index_get_current_value(scan);

    // Search for next different value
    // (skip all rows with current value)
    index_scan_key_update(scan, current + 1);

    // Position at next distinct value
    index_rescan(scan, ...);
}
```

---

## Query Execution

### Chunk-Aware Execution

```c
// Execution flow for hypertable query
void execute_hypertable_query(
    HypertableScanState *state
) {
    // 1. Open chunks (with locking)
    foreach(chunk, state->chunks) {
        chunk_open_lock(chunk->relid, AccessShareLock);
    }

    // 2. Initialize per-chunk state
    foreach(chunk, state->chunks) {
        init_chunk_scan_state(chunk);
    }

    // 3. Execute chunks in order (or parallel)
    while (!execution_complete) {
        foreach(chunk_state, state->chunk_states) {
            if (chunk_has_more_tuples(chunk_state)) {
                tuple = chunk_get_next_tuple(chunk_state);
                emit_tuple(tuple);
            }
        }
    }

    // 4. Cleanup
    foreach(chunk, state->chunks) {
        chunk_close(chunk->relid, AccessShareLock);
    }
}
```

### Parallel Chunk Execution

```sql
-- Enable parallel query
SET max_parallel_workers_per_gather = 4;

-- Parallel hypertable scan
EXPLAIN SELECT avg(temperature)
FROM conditions
WHERE time >= NOW() - INTERVAL '1 day';

-- Parallel plan
Finalize Aggregate
  -> Gather Merge
       Workers: 4
       -> Partial Aggregate
            -> HypertableScan
                 -> Chunks distributed across workers
```

### Vectorized Execution (Columnar)

For compressed chunks:

```c
// Columnar decompression
typedef struct DecompressExecState {
    ColumnDecompressor *decompressors[MAX_COLUMNS];
    VectorBatch current_batch;
    int batch_size;
} DecompressExecState;

// Vectorized processing
void decompress_and_process_batch(
    DecompressExecState *state,
    ExprState *qual
) {
    // Decompress entire batch at once
    for (int col = 0; col < num_columns; col++) {
        decompress_column(
            state->decompressors[col],
            &state->current_batch.columns[col]
        );
    }

    // Vectorized filter evaluation
    BitmapSet *matches = evaluate_qual_vectorized(
        qual,
        &state->current_batch
    );

    // Emit matching tuples
    foreach(tuple_idx, matches) {
        emit_tuple(state->current_batch.tuples[tuple_idx]);
    }
}
```

---

## Performance Tuning

### Configuration Parameters

```sql
-- Chunk management
SET timescaledb.max_open_chunks_per_txn = 100;
SET timescaledb.max_cached_chunks_per_hypertable = 50;

-- Query optimization
SET timescaledb.enable_hypertable_path = on;
SET timescaledb.enable_gapfill = on;
SET timescaledb.enable_skip_scan = on;

-- Compression
SET timescaledb.compress_enable = on;
SET timescaledb.compression_level = 3;

-- Memory
SET timescaledb.max_memory_usage_mb = 4096;
```

### Explain Analyze

```sql
-- Full execution statistics
EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
SELECT time_bucket('1 hour', time) as bucket,
       avg(temperature) as avg_temp,
       max(temperature) as max_temp
FROM conditions
WHERE time >= NOW() - INTERVAL '7 days'
  AND location = 'office'
GROUP BY 1
ORDER BY 1;

-- Key metrics to watch:
-- - Planning Time (should be < 10ms)
-- - Execution Time
-- - Buffer hits vs reads
-- - Rows removed by filter
-- - Chunks pruned
```

### Common Optimization Patterns

#### 1. Time-Range Queries

```sql
-- Good: Uses time index
SELECT * FROM conditions
WHERE time >= NOW() - INTERVAL '1 hour';

-- Bad: Function on time column
SELECT * FROM conditions
WHERE date_trunc('hour', time) >= NOW() - INTERVAL '1 hour';

-- Better: Use time_bucket
SELECT * FROM conditions
WHERE time >= date_trunc('hour', NOW() - INTERVAL '1 hour');
```

#### 2. Aggregation Queries

```sql
-- Good: Leverages continuous aggregates
SELECT * FROM conditions_summary_daily
WHERE bucket >= NOW() - INTERVAL '30 days';

-- For raw data, use time_bucket
SELECT
  time_bucket('1 hour', time) as bucket,
  avg(temperature)
FROM conditions
GROUP BY 1;
```

#### 3. Latest N Records

```sql
-- Good: Uses index order
SELECT * FROM conditions
WHERE location = 'office'
ORDER BY time DESC
LIMIT 100;

-- For latest per group, use DISTINCT ON
SELECT DISTINCT ON (location)
  location, time, temperature
FROM conditions
ORDER BY location, time DESC;
```

#### 4. Join Patterns

```sql
-- Good: Join on time buckets
WITH hourly AS (
  SELECT
    time_bucket('1 hour', time) as bucket,
    location,
    avg(temperature) as avg_temp
  FROM conditions
  GROUP BY 1, 2
)
SELECT h.bucket, h.location, h.avg_temp, w.humidity
FROM hourly h
JOIN weather w ON w.bucket = h.bucket AND w.location = h.location;
```

### Index Tuning

```sql
-- Analyze index usage
SELECT
  schemaname,
  relname,
  idx_scan,
  idx_tup_read,
  idx_tup_fetch
FROM pg_stat_user_indexes
WHERE relname LIKE '%_chunk_%'
ORDER BY idx_scan DESC;

-- Find unused indexes
SELECT
  indexrelname,
  pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_stat_user_indexes
WHERE idx_scan = 0
  AND indexrelname LIKE '%_chunk_%';

-- Check index correlation
SELECT
  attname,
  correlation
FROM pg_stats
WHERE tablename LIKE '%chunk%'
  AND attname = 'time';
```

### Vacuum and Analyze

```sql
-- Regular maintenance
VACUUM ANALYZE conditions;

-- For individual chunks
DO $$
DECLARE
    chunk RECORD;
BEGIN
    FOR chunk IN
        SELECT table_name
        FROM timescaledb_information.chunks
        WHERE hypertable_name = 'conditions'
    LOOP
        EXECUTE format('VACUUM ANALYZE %I', chunk.table_name);
    END LOOP;
END $$;
```

---

## Query Plan Debugging

### Understanding Plan Output

```sql
-- Detailed plan
EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
SELECT * FROM conditions WHERE time >= NOW() - INTERVAL '1 hour';
```

**Key sections:**
- `Plan Type`: Scan type used
- `Chunks Excluded`: Number of pruned chunks
- `Buffers`: Hit vs read ratios
- `Planning Time`: Time to generate plan
- `Execution Time`: Actual execution time

### Common Issues

| Issue | Symptom | Solution |
|-------|---------|----------|
| No pruning | All chunks scanned | Add time constraint |
| Sequential scans | Slow queries | Add proper indexes |
| High planning time | Complex queries | Use prepared statements |
| Memory pressure | OOM errors | Reduce chunk size |

---

## Related Documentation

- [TimescaleDB Architecture](./timescaledb-architecture.md)
- [Analytics Functions](./analytics-functions.md)
- [Rust Implementation](./rust-revision.md)
