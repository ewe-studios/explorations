# Orbitinghail -- Production Patterns

This document covers production considerations for running the orbitinghail storage ecosystem: durability, recovery, monitoring, scaling, and operational concerns.

**Aha:** The single most important production consideration for LSM-tree databases is configuring the right write buffer size. Too small (4MB) and you generate too many SSTables, triggering excessive compaction. Too large (256MB) and you risk losing more data on crash (more data in the memtable) and have longer pause times during flush. The default is 16 MiB — enough to amortize the flush cost, small enough to recover quickly after a crash.

## Durability Guarantees

### PersistMode Selection

| Mode | fsync Frequency | Data Loss Window | Throughput | Use Case |
|------|----------------|-----------------|------------|----------|
| `SyncAll` | Every `persist()` call | None (if persist called frequently) | Low | Financial, audit |
| `SyncData` | Every `persist()` call (data only) | Metadata may be lost | Medium | General purpose |
| `Buffer` | OS flush interval (~30s) | Up to 30s of data | High | Ephemeral, derived |

**Aha:** `PersistMode::Buffer` doesn't mean data is lost on application crash — the OS still flushes dirty pages to disk. It means data is lost on power loss or kernel crash. For most applications, `Buffer` with periodic manual `persist(SyncAll)` every few seconds is the right balance.

### WAL Recovery

On startup, the WAL is replayed:

```rust
// During Database::open()
let journal = Journal::open(&path)?;
journal.replay(|batch| {
    for op in batch.operations() {
        apply_to_memtable(op);
    }
    Ok(())
})?;
```

Partially written batches are detected by checksum failure and discarded. The recovery process is idempotent — replaying the same WAL twice produces the same state.

### Graft Crash Recovery

Graft tracks a `pending_commit` during remote sync:

```
Local state: sync_point = LSN 100
Pending: LSN 101 (commit uploaded, but not confirmed)
```

On restart:
1. Check if `pending_commit` is set
2. If set, fetch the remote commit at that LSN
3. If the remote commit exists with matching hash, sync succeeded — clear pending and update sync_point
4. If not, re-upload the commit

## Monitoring

### Key Metrics

| Metric | Source | Warning Threshold |
|--------|--------|-------------------|
| `fjall_memtable_size` | Memtable atomic counter | >80% of flush threshold |
| `fjall_sstables_per_level` | Compaction state | L0 > 10 SSTables |
| `fjall_wal_size` | Journal file size | >256MB |
| `graft_pending_commits` | Volume state | >0 for >5 minutes |
| `graft_sync_lag` | LSN difference | >1000 LSNs |
| `graft_remote_errors` | OpenDAL errors | >1 per minute |
| `splinter_compression_ratio` | Bitmap size / uncompressed | <1.5x (low density benefit) |

### Health Checks

Monitor key metrics through fjall's internal state and graft's runtime. The memtable size is tracked via an atomic counter, SSTable counts are available through the compaction state, and WAL size can be measured from the journal file. For production deployments, expose these as Prometheus metrics and alert when memtable approaches the write buffer threshold or L0 accumulates too many SSTables.

## Scaling

### Read Scaling

Reads scale by adding block cache:

```rust
use lsm_tree::config::Cache;

let cache = Cache::with_capacity_bytes(16 * 1_024 * 1_024);  // 16 MiB cache
let config = Config::new("/tmp/my-store", seqno, visible_seqno)
    .cache(cache)
    .build()?;
```

Hot data stays in the cache, avoiding disk reads. The cache is shared across all keyspaces.

### Write Scaling

Writes scale horizontally by sharding keyspaces:

```rust
// Shard by key prefix
let shard_0 = db.open_keyspace("shard-0")?;
let shard_1 = db.open_keyspace("shard-1")?;
// Each shard has its own memtable and SSTables
```

Each shard can be flushed and compacted independently, reducing write stalls.

### Remote Scaling

Graft's remote sync scales by using multiple segments:

```rust
// Each segment is uploaded independently
// Multiple segments can be uploaded concurrently
let concurrency = 5;
```

## Backup and Restore

### Snapshot-Based Backup

```rust
// Create a point-in-time snapshot
let snapshot = db.snapshot()?;

// Export all data from the snapshot
let mut export = Vec::new();
for keyspace in db.keyspaces() {
    for entry in snapshot.scan_keyspace(&keyspace, ..)? {
        export.push((keyspace.name(), entry.key(), entry.value()));
    }
}
```

### WAL-Based Point-in-Time Recovery

```rust
// Restore to a specific LSN
let db = Database::builder()
    .path("/tmp/backup")
    .restore_to_lsn(LSN::new(1000)?)
    .open()?;
```

The WAL contains all changes up to the target LSN. Changes after the target LSN are discarded.

## Compaction Tuning

### Compaction Configuration

Compaction in lsm-tree is trait-based rather than enum-based. The `CompactionStrategy` trait allows custom implementations:

```rust
// Compaction is configured via the Config builder
// Default uses leveled-style compaction with DEFAULT_LEVEL_COUNT = 7 levels
let config = Config::new("/tmp/my-store", seqno, visible_seqno)
    .write_buffer_size_policy(16 * 1_024 * 1_024)  // 16 MiB memtable
    .build()?;
```

The default configuration uses 7 levels. Each level grows by a factor determined by the internal configuration. Level 0 contains recently flushed SSTables (which may overlap), while L1+ SSTables are non-overlapping within their level.

**Aha:** For write-heavy workloads, consider tiered compaction. It merges all SSTables at once when a threshold is reached, resulting in lower write amplification but higher read amplification until compaction runs. The choice between leveled and tiered depends on whether reads or writes are the bottleneck.

See [LSM-Tree](02-lsm-tree.md) for compaction strategies.
See [Fjall Database](03-fjall-database.md) for durability settings.
See [S3 Remote Optimizations](10-s3-remote-optimizations.md) for remote reliability.
