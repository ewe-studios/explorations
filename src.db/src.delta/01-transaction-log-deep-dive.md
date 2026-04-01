---
title: "Delta Lake Transaction Log Deep Dive"
subtitle: "Log structure, checkpointing, and metadata management"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.delta
related: exploration.md, 00-zero-to-lakehouse-engineer.md
---

# 01 - Transaction Log Deep Dive: Delta Lake

## Overview

This document covers Delta Lake's transaction log - the core data structure that enables ACID transactions, time travel, and scalable metadata management.

## Part 1: Log Structure

### JSON Log Format

```
Delta Log Entry Types:

┌─────────────────────────────────────────────────────────┐
│ add - File Addition                                       │
│                                                          │
│ {                                                        │
│   "add": {                                               │
│     "path": "part-00001-abc123.parquet",                │
│     "size": 1234567,                                     │
│     "partitionValues": {"date": "2024-01-01"},          │
│     "modificationTime": 1679875200000,                   │
│     "dataChange": true,                                  │
│     "stats": "{\"numRecords\":10000,...}",              │
│     "tags": {"quality": "gold"}                          │
│   }                                                      │
│ }                                                        │
│                                                          │
│ Key fields:                                              │
│ - path: Relative path in table directory                  │
│ - partitionValues: Partition column values                │
│ - stats: JSON string with min/max/nulls                   │
│ - dataChange: true if this modifies table data            │
└───────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ remove - File Removal                                     │
│                                                          │
│ {                                                        │
│   "remove": {                                            │
│     "path": "part-00001-abc123.parquet",                │
│     "deletionTimestamp": 1679875300000,                  │
│     "dataChange": true,                                  │
│     "extendedFileMetadata": true                         │
│   }                                                      │
│ }                                                        │
│                                                          │
│ Key fields:                                              │
│ - deletionTimestamp: When file was logically removed      │
│ - Vacuum retains files for retention period               │
└───────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ metaData - Table Metadata                                 │
│                                                          │
│ {                                                        │
│   "metaData": {                                          │
│     "id": "table-uuid-123",                              │
│     "format": {"provider": "parquet", "options": {}},   │
│     "schemaString": "{\"type\":\"struct\",...}",        │
│     "partitionColumns": ["date"],                        │
│     "configuration": {"delta.logRetentionDuration": ...},│
│     "createdTime": 1679875200000                         │
│   }                                                      │
│ }                                                        │
│                                                          │
│ Schema is stored as JSON string (Delta schema format)    │
└───────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ protocol - Version Protocol                               │
│                                                          │
│ {                                                        │
│   "protocol": {                                          │
│     "minReaderVersion": 1,                               │
│     "minWriterVersion": 2                                │
│   }                                                      │
│ }                                                        │
│                                                          │
│ Ensures backward/forward compatibility                   │
└───────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ commitInfo - Commit Metadata                              │
│                                                          │
│ {                                                        │
│   "commitInfo": {                                        │
│     "timestamp": 1679875200000,                          │
│     "userId": "user@example.com",                        │
│     "userName": "John Doe",                              │
│     "operation": "WRITE",                                │
│     "operationParameters": {"mode": "Append"},           │
│     "notebook": {"notebookId": "abc123"},                │
│     "clusterId": "cluster-456",                          │
│     "isolationLevel": "Serializable",                    │
│     "isBlindAppend": true                                │
│   }                                                      │
│ }                                                        │
└───────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ txn - Transaction Identifier                              │
│                                                          │
│ {                                                        │
│   "txn": {                                               │
│     "appId": "app-123",                                  │
│     "version": 5                                         │
│   }                                                      │
│ }                                                        │
│                                                          │
│ Enables idempotent writes                                │
└───────────────────────────────────────────────────────────┘
```

### Log File Naming

```
Log File Naming Convention:

┌─────────────────────────────────────────────────────────┐
│ 00000000000000000000.json  (20 digits, zero-padded)      │
│ 00000000000000000001.json                                │
│ 00000000000000000002.json                                │
│ ...                                                      │
│ 00000000000000000099.json                                │
│ 00000000000000000100.json                                │
│                                                          │
│ Why 20 digits?                                           │
│ - Supports 10^20 commits (effectively infinite)          │
│ - Lexicographic sorting = chronological order            │
│ - Fixed width for consistent parsing                     │
└───────────────────────────────────────────────────────────┘

Atomic Commit Protocol:

┌─────────────────────────────────────────────────────────┐
│ Step 1: Write to temp file                               │
│   s3://table/_delta_log/00000000000000000005.json.tmp   │
│                                                          │
│ Step 2: Atomic rename                                    │
│   rename(00000000000000000005.json.tmp,                  │
│          00000000000000000005.json)                      │
│                                                          │
│ Cloud Storage Behavior:                                  │
│ - S3: Atomic for single PUT                              │
│ - ADLS: Atomic rename                                      │
│ - GCS: Atomic rewrite                                      │
│                                                          │
│ Step 3: Conflict detection                               │
│   If rename fails (file exists), commit failed           │
│   Retry with new version number                          │
└───────────────────────────────────────────────────────────┘
```

## Part 2: Checkpointing

### Checkpoint Parquet Files

```
Why Checkpoints?

Problem: Reading all JSON logs is slow for large tables
Solution: Periodic checkpoint summarizing state

Checkpoint File:
┌─────────────────────────────────────────────────────────┐
│ 00000000000000000000.checkpoint.parquet                  │
│ 00000000000000000100.checkpoint.parquet                  │
│ 00000000000000000200.checkpoint.parquet                  │
│                                                          │
│ Created every N commits (default: 10)                    │
└───────────────────────────────────────────────────────────┘

Checkpoint Structure (Parquet Schema):
```
message checkpoint {
  optional group add {
    optional binary path (UTF8);
    optional int64 size;
    optional binary partitionValues (JSON);
    optional int64 modificationTime;
    optional boolean dataChange;
    optional binary stats (JSON);
    optional binary stats_parsed (JSON);
    optional group minValues (Message) {
      optional binary date (UTF8);
      optional int32 id;
    }
    optional group maxValues (Message) {
      optional binary date (UTF8);
      optional int32 id;
    }
  }

  optional group remove {
    optional binary path (UTF8);
    optional int64 deletionTimestamp;
    optional boolean dataChange;
  }

  optional group metaData {
    optional binary id (UUID);
    optional binary schemaString (UTF8);
    optional binary partitionColumns (JSON);
  }

  optional group protocol {
    optional int32 minReaderVersion;
    optional int32 minWriterVersion;
  }

  optional group txn {
    optional binary appId (UTF8);
    optional int64 version;
  }
}
```

Checkpoint Creation Process:

┌─────────────────────────────────────────────────────────┐
│ Step 1: Read all commits since last checkpoint           │
│   - Replay commits 0-99 for checkpoint at 100            │
│   - Apply add/remove operations                          │
│   - Track current table state                            │
│                                                          │
│ Step 2: Write checkpoint Parquet                         │
│   - One row per active file                              │
│   - Include metadata, stats, partitions                  │
│   - Include protocol, metadata, txns                     │
│                                                          │
│ Step 3: Update _last_checkpoint file                     │
│   {                                                      │
│     "version": 100,                                      │
│     "size": 12345,                                       │
│     "sizeInBytes": 987654                                │
│   }                                                      │
│                                                          │
│ Step 4: Cleanup old logs (optional)                      │
│   - Delete logs before checkpoint (after retention)      │
│   - Keep last N logs for safety                          │
└───────────────────────────────────────────────────────────┘
```

### Incremental Checkpoints

```
Sidecar Files (Delta Lake 2.0+):

Problem: Full checkpoint is expensive for large tables
Solution: Incremental checkpoint with sidecar files

Structure:
┌─────────────────────────────────────────────────────────┐
│ _delta_log/                                              │
│ ├── 00000000000000000000.json                           │
│ ├── ...                                                 │
│ ├── 00000000000000000100.checkpoint.parquet (full)      │
│ ├── 00000000000000000100.abc123.json (sidecar)          │
│ ├── 00000000000000000200.checkpoint.parquet (full)      │
│ └── 00000000000000000200.def456.json (sidecar)          │
└─────────────────────────────────────────────────────────┘

Sidecar contains:
- Only commits since last full checkpoint
- Much smaller than full checkpoint
- Faster to write, slightly slower to read

Reading with sidecars:
1. Read last full checkpoint
2. Read sidecar files
3. Read any remaining JSON logs
4. Replay to get current state
```

## Part 3: Metadata Management

### Table State Reconstruction

```rust
/// Reconstruct table state from transaction log
pub struct DeltaTable {
    table_path: String,
    version: i64,
    state: TableState,
}

pub struct TableState {
    /// Active files in table
    files: HashMap<String, AddAction>,
    /// Removed files (for vacuum)
    tombstones: HashSet<String>,
    /// Table metadata
    metadata: Option<MetadataAction>,
    /// Protocol version
    protocol: Option<ProtocolAction>,
    /// Transaction identifiers
    transactions: HashMap<String, i64>,
}

impl DeltaTable {
    /// Load table at specific version
    pub fn load_at_version(&mut self, version: i64) -> Result<()> {
        // Find checkpoint at or before requested version
        let checkpoint = self.find_checkpoint(version)?;

        // Load checkpoint state
        if let Some(cp) = checkpoint {
            self.state = self.read_checkpoint(&cp)?;
            self.version = cp.version;
        }

        // Replay commits from checkpoint to requested version
        while self.version < version {
            self.version += 1;
            let commit = self.read_commit(self.version)?;
            self.apply_commit(commit)?;
        }

        Ok(())
    }

    /// Apply single commit to state
    fn apply_commit(&mut self, commit: LogCommit) -> Result<()> {
        for action in commit.actions {
            match action {
                Action::Add(add) => {
                    self.state.files.insert(add.path.clone(), add);
                }
                Action::Remove(remove) => {
                    self.state.files.remove(&remove.path);
                    self.state.tombstones.insert(remove.path);
                }
                Action::Metadata(metadata) => {
                    self.state.metadata = Some(metadata);
                }
                Action::Protocol(protocol) => {
                    self.state.protocol = Some(protocol);
                }
                Action::Txn(txn) => {
                    self.state.transactions.insert(txn.app_id, txn.version);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Get current files for query
    pub fn get_files(&self) -> Vec<&AddAction> {
        self.state.files.values().collect()
    }

    /// Get files for partition filter
    pub fn get_files_filtered(&self, filter: &PartitionFilter) -> Vec<&AddAction> {
        self.state.files.values()
            .filter(|f| filter.matches(&f.partition_values))
            .collect()
    }
}
```

### Log Compaction

```
Log Retention and Cleanup:

Vacuum Command:
```sql
-- Remove files older than retention period
VACUUM table_name RETAIN 168 HOURS;

-- Default retention: 7 days (168 hours)
-- Minimum: 24 hours (for concurrent queries)
```

Log Cleanup:
```sql
-- Clean up old log files
DELETE FROM delta.`_delta_log`
WHERE version < (SELECT max(version) - 30 FROM delta.`_last_checkpoint`);
```

Retention Policies:
- Logs: Keep 30 commits or 30 days
- Checkpoints: Keep last 2 full checkpoints
- Data files: Keep for vacuum retention period
```

---

*This document is part of the Delta Lake exploration series. See [exploration.md](./exploration.md) for the complete index.*
