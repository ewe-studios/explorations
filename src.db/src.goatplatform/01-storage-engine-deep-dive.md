---
title: "GoatPlatform Storage Engine Deep Dive"
subtitle: "goatdb internals, storage architecture, and real-time sync mechanisms"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.goatplatform
related: 00-zero-to-db-engineer.md
---

# 01 - Storage Engine Deep Dive: GoatPlatform

## Overview

This document covers goatdb storage internals - how the embedded real-time database stores data, manages indexes, handles WAL logging, and enables efficient synchronization.

## Part 1: Storage Architecture

### Embedded Storage Layout

```
goatdb File Structure:

┌─────────────────────────────────────────────────────────┐
│ goatdb File Layout                                      │
├─────────────────────────────────────────────────────────┤
│ Header (4KB)                                            │
│ - Magic number: 0xGOATDB                               │
│ - Version                                              │
│ - Page size                                            │
│ - Schema version                                       │
│ - Checksum                                             │
├─────────────────────────────────────────────────────────┤
│ Page Directory (variable)                              │
│ - Maps table_id + page_num → file offset               │
│ - Cached in memory for fast lookup                     │
├─────────────────────────────────────────────────────────┤
│ Data Pages (8KB each)                                  │
│ - Table data (heap pages)                              │
│ - Index pages (B+ tree nodes)                          │
│ - Overflow pages (large values)                        │
├─────────────────────────────────────────────────────────┤
│ WAL (Write-Ahead Log)                                  │
│ - Appended on every write                              │
│ - Circular buffer (recycled after checkpoint)          │
│ - Sync modes: FULL, NORMAL, OFF                        │
├─────────────────────────────────────────────────────────┤
│ Sync Metadata                                          │
│ - Local clock (vector clock)                           │
│ - Pending changes queue                                │
│ - Sync state                                           │
└───────────────────────────────────────────────────────────┘

Page Types:
- TABLE_PAGE: Stores row data (heap organization)
- INDEX_PAGE: B+ tree nodes for indexes
- BLOB_PAGE: Out-of-line large values
- FREE_PAGE: Available for allocation
```

### B+ Tree Index Structure

```
goatdb B+ Tree Layout:

┌─────────────────────────────────────────────────────────┐
│ B+ Tree Node Page (8KB)                                 │
├─────────────────────────────────────────────────────────┤
│ Node Header (32 bytes)                                  │
│ - node_type: INTERNAL | LEAF                           │
│ - parent_page: page number of parent                    │
│ - next_page: sibling page (for leaf scan)              │
│ - key_count: number of keys in node                    │
│ - free_offset: offset to free space                    │
├─────────────────────────────────────────────────────────┤
│ Cell Directory (4 bytes per key)                       │
│ - key_offset: offset to key data                       │
│ - value_offset: offset to value (leaf only)            │
│ - flags: key flags                                     │
├─────────────────────────────────────────────────────────┤
│ Free Space                                               │
│ - Grows from end of page                               │
│ - Used for new cells                                   │
├─────────────────────────────────────────────────────────┤
│ Cell Data (variable size)                               │
│ - Keys: sorted order                                   │
│ - Values: row data or child pointers                   │
│ - Leaf cells: [key | value]                            │
│ - Internal cells: [key | child_page]                   │
└───────────────────────────────────────────────────────────┘

B+ Tree Properties:
- All data in leaf nodes
- Internal nodes only for navigation
- Leaves linked for range scans
- Minimum 50% utilization (enforced on split)
```

```
B+ Tree Operations:

```rust
const PAGE_SIZE: usize = 8192;
const CELL_HEADER_SIZE: usize = 4;
const NODE_HEADER_SIZE: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeType {
    Internal,
    Leaf,
}

#[derive(Debug)]
struct BTreeNode {
    node_type: NodeType,
    parent_page: Option<u32>,
    next_page: Option<u32>,
    keys: Vec<Vec<u8>>,
    values: Vec<Vec<u8>>,  // Empty for internal nodes
    children: Vec<u32>,     // Only for internal nodes
}

impl BTreeNode {
    fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            parent_page: None,
            next_page: None,
            keys: Vec::new(),
            values: Vec::new(),
            children: Vec::new(),
        }
    }

    fn is_full(&self) -> bool {
        let estimated_size = NODE_HEADER_SIZE
            + (self.keys.len() * CELL_HEADER_SIZE)
            + self.keys.iter().map(|k| k.len()).sum::<usize>()
            + self.values.iter().map(|v| v.len()).sum::<usize>();

        estimated_size > PAGE_SIZE * 90 / 100  // 90% full
    }

    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        // Find insertion point (binary search)
        let pos = self.keys.iter()
            .position(|k| k >= &key)
            .unwrap_or(self.keys.len());

        self.keys.insert(pos, key);
        if self.node_type == NodeType::Leaf {
            self.values.insert(pos, value);
        }
    }

    fn split(&mut self) -> (BTreeNode, Vec<u8>) {
        // Split node at midpoint
        let mid = self.keys.len() / 2;
        let promoted_key = self.keys[mid].clone();

        let mut right = BTreeNode::new(self.node_type);

        // Move right half to new node
        for _ in 0..self.keys.len() - mid {
            let key = self.keys.pop().unwrap();
            right.keys.insert(0, key);

            if self.node_type == NodeType::Leaf {
                let value = self.values.pop().unwrap();
                right.values.insert(0, value);
            } else {
                let child = self.children.pop().unwrap();
                right.children.insert(0, child);
            }
        }

        if self.node_type == NodeType::Internal {
            // Also move rightmost child pointer
            if let Some(child) = self.children.pop() {
                right.children.insert(0, child);
            }
        }

        right.next_page = self.next_page;
        self.next_page = Some(right.get_first_page());

        (right, promoted_key)
    }

    fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.keys.iter()
            .position(|k| k == key)
            .and_then(|i| self.values.get(i))
    }
}
```
```

## Part 2: WAL (Write-Ahead Log)

### WAL Record Format

```
WAL Entry Structure:

┌─────────────────────────────────────────────────────────┐
│ WAL Record Header (20 bytes)                            │
├─────────────────────────────────────────────────────────┤
│ Offset  │ Size │ Description                            │
├─────────────────────────────────────────────────────────┤
│ 0       │ 4    │ Record length (total bytes)            │
│ 4       │ 4    │ CRC32C checksum                        │
│ 8       │ 8    │ LSN (Log Sequence Number)              │
│ 16      │ 4    │ Transaction ID                         │
├─────────────────────────────────────────────────────────┤
│ WAL Record Body (variable)                               │
│ - Operation type (1 byte)                                │
│ - Table ID (4 bytes)                                     │
│ - Page number (4 bytes)                                  │
│ - Cell offset (2 bytes)                                  │
│ - Old value length (4 bytes)                             │
│ - Old value (variable)                                   │
│ - New value length (4 bytes)                             │
│ - New value (variable)                                   │
└───────────────────────────────────────────────────────────┘

WAL Operations:
- OP_INSERT = 1
- OP_UPDATE = 2
- OP_DELETE = 3
- OP_COMMIT = 4
- OP_CHECKPOINT = 5
```

```
WAL Implementation:

```rust
use crc32c::crc32c;
use std::io::{Read, Write, Seek, SeekFrom};

const WAL_HEADER_SIZE: usize = 20;

#[derive(Debug, Clone)]
pub struct WalRecord {
    pub lsn: u64,
    pub transaction_id: u32,
    pub operation: WalOperation,
    pub table_id: u32,
    pub page_num: u32,
    pub cell_offset: u16,
    pub old_value: Option<Vec<u8>>,
    pub new_value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum WalOperation {
    Insert = 1,
    Update = 2,
    Delete = 3,
    Commit = 4,
    Checkpoint = 5,
}

pub struct WalWriter {
    file: std::fs::File,
    current_lsn: u64,
    buffer: Vec<u8>,
    sync_mode: SyncMode,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncMode {
    Off,      // No fsync (fast, not durable)
    Normal,   // fsync on commit
    Full,     // fsync on every write
}

impl WalWriter {
    pub fn new(path: &str, sync_mode: SyncMode) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Self {
            file,
            current_lsn: 0,
            buffer: Vec::with_capacity(4096),
            sync_mode,
        })
    }

    pub fn write(&mut self, record: WalRecord) -> std::io::Result<u64> {
        self.current_lsn += 1;

        // Serialize record
        let mut body = Vec::new();
        body.push(record.operation as u8);
        body.extend_from_slice(&record.table_id.to_le_bytes());
        body.extend_from_slice(&record.page_num.to_le_bytes());
        body.extend_from_slice(&record.cell_offset.to_le_bytes());

        // Old value
        if let Some(old) = &record.old_value {
            body.extend_from_slice(&(old.len() as u32).to_le_bytes());
            body.extend_from_slice(old);
        } else {
            body.extend_from_slice(&0u32.to_le_bytes());
        }

        // New value
        if let Some(new) = &record.new_value {
            body.extend_from_slice(&(new.len() as u32).to_le_bytes());
            body.extend_from_slice(new);
        } else {
            body.extend_from_slice(&0u32.to_le_bytes());
        }

        // Build header
        let total_len = WAL_HEADER_SIZE + body.len();
        let mut header = Vec::with_capacity(WAL_HEADER_SIZE);
        header.extend_from_slice(&(total_len as u32).to_le_bytes());

        let crc = crc32c(&body);
        header.extend_from_slice(&crc.to_le_bytes());

        header.extend_from_slice(&self.current_lsn.to_le_bytes());
        header.extend_from_slice(&record.transaction_id.to_le_bytes());

        // Write to buffer
        self.buffer.extend_from_slice(&header);
        self.buffer.extend_from_slice(&body);

        // Flush based on sync mode
        match self.sync_mode {
            SyncMode::Full => self.flush()?,
            _ => {
                if self.buffer.len() >= 65536 {
                    self.flush()?;
                }
            }
        }

        Ok(self.current_lsn)
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        self.file.write_all(&self.buffer)?;
        self.file.sync_data()?;
        self.buffer.clear();

        Ok(())
    }

    pub fn commit(&mut self, transaction_id: u32) -> std::io::Result<u64> {
        let record = WalRecord {
            lsn: self.current_lsn + 1,
            transaction_id,
            operation: WalOperation::Commit,
            table_id: 0,
            page_num: 0,
            cell_offset: 0,
            old_value: None,
            new_value: None,
        };

        let lsn = self.write(record)?;

        if self.sync_mode != SyncMode::Off {
            self.flush()?;
        }

        Ok(lsn)
    }
}
```
```

### Recovery from WAL

```
Crash Recovery Algorithm:

┌─────────────────────────────────────────────────────────┐
│ Recovery Process                                        │
│                                                         │
│ 1. Find last checkpoint                                │
│    - Scan WAL backwards for CHECKPOINT record          │
│    - Get checkpoint LSN                                │
│                                                         │
│ 2. Load checkpoint state                               │
│    - Restore page directory from checkpoint            │
│    - Load in-memory structures                         │
│                                                         │
│ 3. Replay WAL from checkpoint                          │
│    - Read each WAL record                              │
│    - Apply operation to page                           │
│    - Track committed transactions                      │
│                                                         │
│ 4. Undo uncommitted transactions                       │
│    - Find transactions without COMMIT                  │
│    - Apply old_value to revert changes                 │
│                                                         │
│ 5. Truncate WAL                                         │
│    - Remove replayed records                           │
│    - Start fresh WAL file                              │
└───────────────────────────────────────────────────────────┘

Recovery Implementation:

```rust
pub struct WalRecovery {
    wal_path: String,
    page_manager: Arc<PageManager>,
}

impl WalRecovery {
    pub fn new(wal_path: String, page_manager: Arc<PageManager>) -> Self {
        Self { wal_path, page_manager }
    }

    pub fn recover(&self) -> std::io::Result<RecoveryResult> {
        let mut file = std::fs::File::open(&self.wal_path)?;

        // Step 1: Find last checkpoint
        let checkpoint_lsn = self.find_last_checkpoint(&mut file)?;

        // Step 2: Seek to checkpoint
        file.seek(SeekFrom::Start(checkpoint_lsn))?;

        // Step 3: Replay WAL
        let mut committed_txns = std::collections::HashSet::new();
        let mut pending_ops: std::collections::HashMap<u32, Vec<WalRecord>> =
            std::collections::HashMap::new();

        while let Some(record) = self.read_wal_record(&mut file)? {
            match record.operation {
                WalOperation::Commit => {
                    committed_txns.insert(record.transaction_id);

                    // Apply all pending ops for this transaction
                    if let Some(ops) = pending_ops.remove(&record.transaction_id) {
                        for op in ops {
                            self.apply_wal_record(&op)?;
                        }
                    }
                }
                WalOperation::Checkpoint => {
                    // Already handled
                }
                _ => {
                    // Buffer operation until commit
                    pending_ops
                        .entry(record.transaction_id)
                        .or_insert_with(Vec::new)
                        .push(record);
                }
            }
        }

        // Step 4: Undo uncommitted transactions
        let mut undone = 0;
        for (txn_id, ops) in pending_ops {
            if !committed_txns.contains(&txn_id) {
                // Reverse apply (use old_value)
                for record in ops.into_iter().rev() {
                    self.undo_wal_record(&record)?;
                }
                undone += 1;
            }
        }

        Ok(RecoveryResult {
            checkpoint_lsn,
            records_replayed: committed_txns.len(),
            transactions_undone: undone,
        })
    }

    fn apply_wal_record(&self, record: &WalRecord) -> std::io::Result<()> {
        match record.operation {
            WalOperation::Insert | WalOperation::Update => {
                if let Some(value) = &record.new_value {
                    self.page_manager
                        .write_cell(record.table_id, record.page_num, record.cell_offset, value)?;
                }
            }
            WalOperation::Delete => {
                self.page_manager
                    .delete_cell(record.table_id, record.page_num, record.cell_offset)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn undo_wal_record(&self, record: &WalRecord) -> std::io::Result<()> {
        // Restore old value
        if let Some(old_value) = &record.old_value {
            self.page_manager
                .write_cell(record.table_id, record.page_num, record.cell_offset, old_value)?;
        } else {
            // No old value = this was an insert, delete the cell
            self.page_manager
                .delete_cell(record.table_id, record.page_num, record.cell_offset)?;
        }
        Ok(())
    }
}

pub struct RecoveryResult {
    pub checkpoint_lsn: u64,
    pub records_replayed: usize,
    pub transactions_undone: usize,
}
```
```

## Part 3: Sync Metadata

### Vector Clock Implementation

```
Vector Clock for Sync Tracking:

┌─────────────────────────────────────────────────────────┐
│ Vector Clock Structure                                  │
│                                                         │
│ Tracks causality across replicas:                       │
│                                                         │
│ Client A: {A: 5, B: 3, C: 7}                           │
│ Client B: {A: 5, B: 4, C: 7}                           │
│ Client C: {A: 5, B: 3, C: 8}                           │
│                                                         │
│ Comparison:                                             │
│ - A < B: A happened before B (all counters <=)         │
│ - A > B: A happened after B (all counters >=)          │
│ - A || B: Concurrent (some <, some >)                  │
│                                                         │
│ On local write:                                         │
│   clock[self_id] += 1                                   │
│                                                         │
│ On receive remote clock:                                │
│   for (id, count) in remote_clock:                     │
│     clock[id] = max(clock[id], count)                  │
│   clock[self_id] += 1                                   │
└───────────────────────────────────────────────────────────┘

Vector Clock Implementation:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VectorClock {
    counters: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }

    /// Increment local counter
    pub fn tick(&mut self, node_id: &str) {
        *self.counters.entry(node_id.to_string()).or_insert(0) += 1;
    }

    /// Merge with remote clock
    pub fn merge(&mut self, other: &VectorClock) {
        for (node_id, count) in &other.counters {
            let entry = self.counters.entry(node_id.clone()).or_insert(0);
            *entry = (*entry).max(*count);
        }
    }

    /// Compare clocks
    pub fn compare(&self, other: &VectorClock) -> ClockOrdering {
        let mut less = false;
        let mut greater = false;

        // Check all counters in both clocks
        let all_nodes: std::collections::HashSet<_> = self
            .counters
            .keys()
            .chain(other.counters.keys())
            .collect();

        for node in all_nodes {
            let self_count = self.counters.get(node).copied().unwrap_or(0);
            let other_count = other.counters.get(node).copied().unwrap_or(0);

            if self_count < other_count {
                less = true;
            } else if self_count > other_count {
                greater = true;
            }
        }

        match (less, greater) {
            (true, true) => ClockOrdering::Concurrent,
            (true, false) => ClockOrdering::Before,
            (false, true) => ClockOrdering::After,
            (false, false) => ClockOrdering::Equal,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(self.counters.len() as u32).to_le_bytes());

        for (node_id, count) in &self.counters {
            bytes.extend_from_slice(&(node_id.len() as u8).to_le_bytes());
            bytes.extend_from_slice(node_id.as_bytes());
            bytes.extend_from_slice(&count.to_le_bytes());
        }

        bytes
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, &'static str> {
        let mut offset = 0;
        let count = u32::from_le_bytes(
            data[offset..offset + 4].try_into().map_err(|_| "Invalid length")?
        ) as usize;
        offset += 4;

        let mut counters = HashMap::new();

        for _ in 0..count {
            let len = data[offset] as usize;
            offset += 1;

            let node_id = String::from_utf8(
                data[offset..offset + len].to_vec()
            ).map_err(|_| "Invalid UTF8")?;
            offset += len;

            let counter = u64::from_le_bytes(
                data[offset..offset + 8].try_into().map_err(|_| "Invalid counter")?
            );
            offset += 8;

            counters.insert(node_id, counter);
        }

        Ok(Self { counters })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClockOrdering {
    Before,
    After,
    Concurrent,
    Equal,
}
```
```

### Change Tracking

```
Change Log for Sync:

```rust
#[derive(Debug, Clone)]
pub struct ChangeRecord {
    pub lsn: u64,
    pub clock: VectorClock,
    pub table_id: u32,
    pub row_id: Vec<u8>,
    pub operation: ChangeOperation,
    pub old_value: Option<Vec<u8>>,
    pub new_value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ChangeOperation {
    Insert,
    Update,
    Delete,
}

pub struct ChangeLog {
    log: std::collections::VecDeque<ChangeRecord>,
    max_size: usize,
}

impl ChangeLog {
    pub fn new(max_size: usize) -> Self {
        Self {
            log: std::collections::VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn append(&mut self, record: ChangeRecord) {
        self.log.push_back(record);

        // Trim old entries
        while self.log.len() > self.max_size {
            self.log.pop_front();
        }
    }

    /// Get changes since a given clock
    pub fn changes_since(&self, clock: &VectorClock) -> Vec<&ChangeRecord> {
        self.log.iter()
            .filter(|record| {
                // Include if concurrent or after the given clock
                matches!(record.clock.compare(clock),
                    ClockOrdering::Concurrent | ClockOrdering::After)
            })
            .collect()
    }

    /// Get changes for specific table
    pub fn changes_for_table(&self, table_id: u32) -> Vec<&ChangeRecord> {
        self.log.iter()
            .filter(|record| record.table_id == table_id)
            .collect()
    }
}
```
```

---

*This document is part of the GoatPlatform exploration series. See [exploration.md](./exploration.md) for the complete index.*
