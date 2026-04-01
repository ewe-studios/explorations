---
title: "Delta Lake Concurrency Control Deep Dive"
subtitle: "MVCC, optimistic concurrency, and conflict resolution"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.delta
related: exploration.md, 01-transaction-log-deep-dive.md
---

# 03 - Concurrency Control Deep Dive: Delta Lake

## Overview

This document covers Delta Lake's concurrency control mechanism - how MVCC enables serializable isolation, optimistic concurrency control, and conflict resolution.

## Part 1: MVCC Implementation

### Snapshot Isolation

```
Multi-Version Concurrency Control in Delta Lake:

┌─────────────────────────────────────────────────────────┐
│ Timeline of Commits                                      │
│                                                          │
│ Version 0: Initial state                                 │
│   Files: [A, B]                                          │
│                                                          │
│ Version 1: Writer 1 commits                              │
│   Files: [A, B, C] (C added)                             │
│                                                          │
│ Version 2: Writer 2 commits                              │
│   Files: [A, C] (B removed)                              │
│                                                          │
│ Version 3: Writer 3 commits                              │
│   Files: [A, C, D] (D added)                             │
│                                                          │
│ Readers can see any version:                             │
│   - Reader at t=1 sees version 1: [A, B, C]              │
│   - Reader at t=2 sees version 2: [A, C]                 │
│   - Reader at t=3 sees version 3: [A, C, D]              │
│                                                          │
│ Each version is a consistent snapshot                    │
└───────────────────────────────────────────────────────────┘

Snapshot Structure:
```rust
pub struct Snapshot {
    /// Version number
    version: i64,

    /// Table state at this version
    state: TableState,

    /// Timestamp of commit
    timestamp: i64,

    /// Log checksum
    checksum: Vec<u8>,
}

pub struct TableState {
    /// Active files
    files: Vec<AddAction>,

    /// Table metadata
    metadata: MetadataAction,

    /// Protocol version
    protocol: ProtocolAction,
}
```
```

### Optimistic Concurrency Control

```
OCC Protocol:

┌─────────────────────────────────────────────────────────┐
│ Writer Transaction Flow                                  │
│                                                          │
│ 1. Read snapshot at version N                            │
│    - Load table state                                    │
│    - Plan writes based on state                          │
│                                                          │
│ 2. Write new data files                                  │
│    - Write Parquet files to storage                      │
│    - Files not yet visible to readers                    │
│                                                          │
│ 3. Attempt commit at version N+1                         │
│    - Create log entry                                    │
│    - Atomic rename to 0000000000000000000N+1.json        │
│                                                          │
│ 4. Conflict Detection                                    │
│    - Check if conflicting commit exists                  │
│    - If conflict: retry from step 1                      │
│    - If no conflict: commit succeeds                     │
│                                                          │
│ 5. Update snapshot                                       │
│    - New version visible to readers                      │
└───────────────────────────────────────────────────────────┘

Conflict Matrix:

┌──────────────┬──────────┬──────────┬──────────┐
│              │ Read     │ Append   │ Update   │
├──────────────┼──────────┼──────────┼──────────┤
│ Read         │ ✓        │ ✓        │ ✓        │
│ Append       │ ✓        │ ✓        │ ✓        │
│ Update       │ ✓        │ ✓        │ ✗        │
│ Delete       │ ✓        │ ✓        │ ✗        │
└──────────────┴──────────┴──────────┴──────────┘

✓ = Compatible (no conflict)
✗ = Conflicting (one must retry)

Conflict Rules:
- Reads never conflict with anything
- Appends never conflict (disjoint files)
- Updates conflict with updates/deletes to same data
- Deletes conflict with updates/deletes to same data
```

## Part 2: Conflict Resolution

### Conflict Detection

```rust
/// Conflict detection for OCC
pub struct ConflictChecker {
    read_set: HashSet<FileId>,
    write_set: HashSet<FileId>,
    partition_writes: HashMap<PartitionKey, Vec<FileId>>,
}

impl ConflictChecker {
    /// Check if transaction conflicts with committed transaction
    pub fn has_conflict(&self, committed: &CommittedTransaction) -> bool {
        // Check read-write conflicts
        for file_id in &self.read_set {
            if committed.deletes.contains(file_id) {
                return true; // Read something that was deleted
            }
            if committed.updates.contains(file_id) {
                return true; // Read something that was updated
            }
        }

        // Check write-write conflicts (same partition)
        for (partition, files) in &self.partition_writes {
            if let(committed_files) = committed.partition_writes.get(partition) {
                // Overlapping writes to same partition
                if !files.is_disjoint(committed_files) {
                    return true;
                }
            }
        }

        // No conflict
        false
    }
}

/// Transaction metadata for conflict detection
pub struct CommittedTransaction {
    version: i64,
    deletes: HashSet<FileId>,
    updates: HashSet<FileId>,
    appends: HashSet<FileId>,
    partition_writes: HashMap<PartitionKey, HashSet<FileId>>,
}
```

### Retry Logic

```rust
/// Optimistic transaction with retry
pub struct OptimisticTransaction<T> {
    table: DeltaTable,
    max_retries: u32,
    _phantom: PhantomData<T>,
}

impl<T> OptimisticTransaction<T> {
    pub fn execute<F>(&mut self, mut operation: F) -> Result<T>
    where
        F: FnMut(&TableState) -> Result<Transaction>,
    {
        let mut attempts = 0;

        loop {
            // Read current snapshot
            let snapshot = self.table.read_snapshot()?;

            // Execute operation (may read/modify state)
            let transaction = operation(&snapshot.state)?;

            // Attempt commit
            match self.table.commit(transaction) {
                Ok(version) => {
                    return Ok(transaction.result);
                }
                Err(CommitError::Conflict) => {
                    attempts += 1;
                    if attempts >= self.max_retries {
                        return Err(Error::MaxRetriesExceeded);
                    }
                    // Retry with new snapshot
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

// Usage
let mut txn = OptimisticTransaction::new(table, 3);
txn.execute(|state| {
    // Read state
    let files = state.get_files();

    // Plan modifications
    let new_files = compute_new_files(files);

    Ok(Transaction {
        adds: new_files,
        removes: vec![],
        result: (),
    })
})?;
```

## Part 3: Isolation Levels

### Serializable Isolation

```
Delta Lake Isolation Guarantees:

Default: Serializable Isolation
- All transactions appear to execute serially
- Equivalent to some total ordering
- No anomalies (dirty reads, non-repeatable reads, phantoms)

Implementation via OCC:
- Conflicts detected at commit time
- Conflicting transactions must retry
- Final result is serializable

Example Anomaly Prevention:

┌─────────────────────────────────────────────────────────┐
│ Dirty Read Prevention                                    │
│                                                          │
│ T1: Write A=100 (uncommitted)                            │
│ T2: Read A (sees old value, not uncommitted)             │
│ T1: Commit                                               │
│                                                          │
│ Result: T2 never sees uncommitted data                   │
└───────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Non-Repeatable Read Prevention                           │
│                                                          │
│ T1: Read A=50                                            │
│ T2: Update A=100, Commit                                 │
│ T1: Read A=50 (same snapshot, still sees 50)             │
│                                                          │
│ Result: T1 sees consistent snapshot throughout           │
└───────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Phantom Read Prevention                                  │
│                                                          │
│ T1: SELECT COUNT(*) FROM table WHERE x > 10 (count=5)    │
│ T2: INSERT (15, ...), Commit                             │
│ T1: SELECT COUNT(*) FROM table WHERE x > 10 (count=5)    │
│                                                          │
│ Result: T1 sees consistent snapshot, no phantoms         │
└───────────────────────────────────────────────────────────┘
```

---

*This document is part of the Delta Lake exploration series. See [exploration.md](./exploration.md) for the complete index.*
