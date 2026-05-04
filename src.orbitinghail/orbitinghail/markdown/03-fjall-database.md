# Orbitinghail -- Fjall Database

Fjall is an embeddable KV database built on top of `lsm-tree`. It adds multiple keyspaces (column families), serializable transactions, MVCC snapshots, crash recovery via WAL journal, and automatic background compaction. It is the storage foundation for graft.

**Aha:** Fjall's multi-keyspace design shares a single WAL but has separate memtables and SSTables per keyspace. This means a single `db.persist()` fsyncs all keyspaces atomically — either all keyspaces are durable or none are. Cross-keyspace batch writes are supported: insert into keyspace A and delete from keyspace B in a single atomic operation. The shared WAL is the key insight that enables atomicity across keyspaces.

Source: `fjall/src/database.rs` — Database struct
Source: `fjall/src/journal/` — WAL journal implementation

## Architecture

```mermaid
flowchart TD
    subgraph Database ["Database"]
        WAL["Shared WAL<br/>Write-Ahead Log"]
        KS1["Keyspace A<br/>Memtable + SSTables"]
        KS2["Keyspace B<br/>Memtable + SSTables"]
        KS3["Keyspace C<br/>Memtable + SSTables"]
    end

    subgraph Workers ["Background Workers"]
        FLUSH["Flush Worker<br/>Memtable → SSTable"]
        COMPACT["Compaction Worker<br/>Merge SSTables"]
        GC["GC Worker<br/>Clean old files"]
    end

    subgraph Tx ["Transaction Layer"]
        OPTTX["OptimisticTx<br/>Conflict detection"]
        SWT["SingleWriterTx<br/>No conflicts"]
    end

    subgraph MVCC ["MVCC"]
        SNAP["Snapshot<br/>Point-in-time view"]
        ORACLE["Oracle<br/>Version tracking"]
    end

    WAL --> KS1
    WAL --> KS2
    WAL --> KS3
    KS1 -.flush.-> FLUSH
    KS2 -.flush.-> FLUSH
    KS1 -.compact.-> COMPACT
    KS2 -.compact.-> COMPACT
    OPTTX --> SNAP
    SWT --> SNAP
    SNAP --> ORACLE
```

## Keyspaces (Column Families)

```rust
let db = Database::builder().path("/tmp/my-db").open()?;

let users = db.open_keyspace("users")?;
let orders = db.open_keyspace("orders")?;

users.insert(b"user:1", b"Alice")?;
orders.insert(b"order:100", b"user:1,book")?;
```

Each keyspace is an independent LSM-tree with its own memtable and SSTables. Keyspaces share the WAL and the background compaction worker.

## Write-Ahead Log (WAL)

Source: `fjall/src/journal/`

Every write is first appended to the WAL before being applied to the memtable:

```
WAL Entry:
┌─────────────────────────────────┐
│ Batch header                    │
│ - batch_id: u64                 │
│ - timestamp: u64                │
│ - num_operations: u32           │
├─────────────────────────────────┤
│ Operation 1: Insert/Delete      │
│ - keyspace_id: u8               │
│ - key_len: u16                  │
│ - key: bytes                    │
│ - value_len: u16 (if Insert)   │
│ - value: bytes (if Insert)     │
├─────────────────────────────────┤
│ Operation 2: ...                │
└─────────────────────────────────┘
```

On crash recovery, the WAL is replayed: each entry is read and applied to the memtable. Entries that were fully written are applied; partially written entries are detected by a trailing checksum and discarded.

**Aha:** The WAL uses append-only writes with a checksum at the end of each batch. During recovery, the last batch is checked — if the checksum doesn't match, the batch was partially written during the crash and is discarded. This is a form of "fuzzy" checkpointing that doesn't require fsync after every write.

## Transactions

### SingleWriterTx

For workloads where only one thread writes at a time:

```rust
let tx = db.single_writer_tx()?;
tx.insert(&users, b"key", b"value")?;
tx.commit()?;
```

No conflict detection needed — there's only one writer. Reads can happen concurrently from any number of readers.

### OptimisticTx

For concurrent writers with conflict detection:

```rust
let tx = db.optimistic_tx()?;
tx.insert(&users, b"key", b"value")?;

// On commit, check if any keys we read were modified by another transaction
match tx.commit() {
    Ok(()) => println!("Committed successfully"),
    Err(CommitError::Conflict) => println!("Retry the transaction"),
}
```

**Aha:** Optimistic transactions don't acquire locks. They record which keys they read, and on commit, they check if any of those keys have been modified since the transaction started. If so, the commit fails with a conflict error and the application retries. This is much faster than lock-based transactions for low-contention workloads but requires retry logic.

## MVCC Snapshots

```rust
let snapshot = db.snapshot()?;  // Point-in-time view
let value = snapshot.get(&users, b"key")?;
```

A snapshot captures the current state of all keyspaces. Subsequent writes don't affect the snapshot. The snapshot tracks which SSTable versions were visible at creation time.

Source: `fjall/src/snapshot.rs`

## PersistMode

```rust
pub enum PersistMode {
    SyncAll,     // fsync all keyspaces
    SyncData,    // fsync data only (metadata may be delayed)
    Buffer,      // Buffered write, no immediate fsync
}
```

`db.persist(PersistMode::SyncAll)` fsyncs all keyspaces, ensuring all writes are durable. This is the safest mode but the slowest. `PersistMode::Buffer` defers durability to the OS page cache — fast but risky on crash.

## Flush and Compaction Workers

Background workers handle maintenance:

```mermaid
flowchart TD
    A["Memtable reaches threshold"] --> B["Freeze memtable"]
    B --> C["Flush Worker writes SSTable"]
    C --> D["Update keyspace metadata"]
    D --> E{"Too many SSTables?"}
    E -->|Yes| F["Compaction Worker merges"]
    E -->|No| G["Done"]
    F --> H["Write new merged SSTables"]
    H --> I["Delete old SSTables"]
    I --> J["Update metadata"]
    J --> G
```

The flush worker runs when the memtable is full. The compaction worker runs when the number of SSTables exceeds a threshold. Both run in the background — writes and reads continue during maintenance.

## Configuration

```rust
let db = ConfigBuilder::new()
    .path("/tmp/my-db")
    .max_write_buffer_size(16 * 1_024 * 1_024)  // 16 MiB memtable (default)
    .data_block_size(4096)                       // 4KB blocks
    // Bloom filters are configured per-level:
    // L0: FalsePositiveRate(0.0001), L1+: FalsePositiveRate(0.01)
    .persist_mode(PersistMode::SyncAll)          // Durability
    .open()?;
```

## Replicating in Rust

Fjall is already a production-ready Rust implementation. For application use:

```rust
// Simple key-value store
use fjall::{ConfigBuilder, PersistMode};

let db = ConfigBuilder::new().path("/tmp/app-db").open()?;
let kv = db.open_keyspace("app-data")?;

kv.insert(b"config", serde_json::to_vec(&config)?)?;
db.persist(PersistMode::SyncAll)?;

// Transactional update
let tx = db.optimistic_tx()?;
let old = tx.get(&kv, b"counter")?;
let new = parse_counter(&old) + 1;
tx.insert(&kv, b"counter", new.to_string().into_bytes())?;
tx.commit()?;
```

See [LSM-Tree](02-lsm-tree.md) for the underlying storage engine.
See [Graft Storage](04-graft-storage.md) for the syncable storage layer built on fjall.
See [Storage Formats](08-storage-formats.md) for the WAL and SSTable layouts.
