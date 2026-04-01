---
title: "TigerBeetle Storage Engine Deep Dive"
subtitle: "WAL format, data files, checkpoints, and recovery"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.tigerbeetle
related: 00-zero-to-ledger-engineer.md, exploration.md
---

# 01 - Storage Engine Deep Dive: TigerBeetle

## Overview

This document covers TigerBeetle's storage engine internals - how data is persisted to disk, the write-ahead log format, checkpoint mechanism, and recovery procedures.

## Part 1: Storage Architecture

### Fixed-Size Data Files

```
TigerBeetle Data File Structure:

File Size: Exactly 10 GB (fixed)
┌─────────────────────────────────────────────────────────┐
│ Header (4 KB)                                            │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Magic Number: "TIGERBEETLE" (16 bytes)              │ │
│ │ Version: u32 (4 bytes)                              │ │
│ │ Checksum: u32 (4 bytes)                             │ │
│ │ File Type: u8 (1 = primary, 2 = superblock)         │ │
│ │ Cluster ID: u128 (16 bytes)                         │ │
│ │ Reserved: 3955 bytes                                │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ Superblock (4 KB)                                        │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Replica Index: u32                                  │ │
│ │ Replica Count: u32                                  │ │
│ │ File Size: u64                                      │ │
│ │ WAL LSN: u64                                        │ │
│ │ Checksum: u32                                       │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ WAL Region (Variable, typically 1-2 GB)                  │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Write-Ahead Log entries                             │ │
│ │ Circular buffer structure                           │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ Data Region (Remaining space, ~7-8 GB)                   │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Account records (fixed 128 bytes each)              │ │
│ │ Transfer records (fixed 128 bytes each)             │ │
│ │ Free space management                               │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘

Why fixed 10 GB?
- Predictable memory mapping (mmap)
- Simplifies allocation algorithms
- Enables pre-allocation (fallocate)
- Bounded recovery time
- Easy file replication
```

### Memory-Mapped I/O

```
TigerBeetle uses mmap for direct memory access:

┌─────────────────────────────────────────────────────────┐
│                    Linux Kernel                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Page Cache                          │    │
│  └─────────────────┬───────────────────────────────┘    │
│                    │ mmap()                              │
│                    ▼                                     │
│  ┌─────────────────────────────────────────────────┐    │
│  │              User Space                          │    │
│  │  TigerBeetle Process                             │    │
│  │  ┌─────────────────────────────────────────┐    │    │
│  │  │ Direct memory access to data file       │    │    │
│  │  │ No read()/write() syscalls needed       │    │    │
│  │  │ Zero-copy reads                         │    │    │
│  │  └─────────────────────────────────────────┘    │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

Benefits:
- Zero-copy reads (data directly from page cache)
- Kernel manages caching automatically
- msync() for durability instead of fsync()
- Efficient random access

Code structure:
```rust
struct StorageEngine {
    /// Memory-mapped data file
    mmap: MmapMut,

    /// Current WAL head position
    wal_head: u64,

    /// Checkpoint metadata
    checkpoint: CheckpointMetadata,

    /// Free list for allocation
    free_list: FreeList,
}

impl StorageEngine {
    fn read_account(&self, account_id: u128) -> Option<Account> {
        let offset = self.account_offset(account_id);
        let bytes = &self.mmap[offset..offset + size_of::<Account>()];
        Some(Account::from_bytes(bytes))
    }

    fn write_account(&mut self, account: &Account) {
        let offset = self.account_offset(account.id);
        self.mmap[offset..offset + size_of::<Account>()]
            .copy_from_slice(&account.to_bytes());
    }
}
```

## Part 2: Write-Ahead Log (WAL)

### WAL Entry Format

```
WAL Entry Structure (Fixed 128 bytes per entry):

┌─────────────────────────────────────────────────────────┐
│ 0x00: Entry Header (32 bytes)                            │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ 0x00: Magic: u32 = 0xDEADBEEF (4 bytes)             │ │
│ │ 0x04: Entry Type: u8                                │ │
│ │       0x01 = Account Create                         │ │
│ │       0x02 = Account Update                         │ │
│ │       0x03 = Transfer Create                        │ │
│ │       0x04 = Transfer Commit                        │ │
│ │       0x05 = Transfer Void                          │ │
│ │       0x06 = Checkpoint                             │ │
│ │ 0x05: Flags: u8 (16 bytes)                          │ │
│ │ 0x06: LSN (Log Sequence Number): u64 (8 bytes)      │ │
│ │ 0x0E: Timestamp: u64 (8 bytes)                      │ │
│ │ 0x16: Checksum: u32 (4 bytes)                       │ │
│ │ 0x1A: Reserved (6 bytes)                            │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ 0x20: Entry Data (96 bytes)                              │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ For Account Create/Update:                          │ │
│ │   Account struct (serialized)                       │ │
│ │                                                     │ │
│ │ For Transfer Create:                                │ │
│ │   Transfer struct (serialized)                      │ │
│ │                                                     │ │
│ │ For Checkpoint:                                     │ │
│ │   CheckpointMetadata (serialized)                   │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘

Entry validation:
1. Magic number check (0xDEADBEEF)
2. Checksum verification (CRC32C)
3. LSN must be sequential
4. Entry type must be valid
```

### WAL Circular Buffer

```
WAL Region Layout (Circular Buffer):

┌─────────────────────────────────────────────────────────┐
│ WAL Region (1-2 GB)                                      │
│                                                          │
│  Head ──► ┌─────────────────────────────────────┐       │
│           │  Committed entries (can be truncated)│       │
│           └─────────────────────────────────────┘       │
│                                  │                      │
│                                  ▼                      │
│                         ┌─────────────────┐            │
│                         │  Active entries │            │
│                         │  (not yet chkpt)│            │
│                         └─────────────────┘            │
│                                  │                      │
│                                  ▼                      │
│                         ┌─────────────────┐            │
│                         │  Free space     │            │
│                         │  (available)    │            │
│                         └─────────────────┘            │
│                                                      ◄──┘
│                                                       Tail

Head: Oldest uncommitted entry
Tail: Next write position

When tail wraps around:
- Head must have advanced (entries checkpointed)
- Otherwise: WAL full, must flush synchronously

Write path:
1. Append entry at tail
2. Update tail pointer
3. Flush to disk (fsync/msync)
4. Return success to client
5. Apply to data region asynchronously
```

### WAL Write Path

```
Synchronous WAL Write (Safe Mode):

Client Request
     │
     ▼
┌─────────────────┐
│ 1. Validate     │ ──► Check transfer/account validity
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 2. Create WAL   │ ──► Build WAL entry with checksum
│    Entry        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 3. Append to    │ ──► Write at tail position
│    WAL          │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 4. fsync()      │ ──► Durability guarantee
│    (or msync)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 5. Apply to     │ ──► Update account/transfer in data
│    Data Region  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 6. Return       │ ──► Success to client
│    Success      │
└─────────────────┘

Latency: ~100μs (NVMe SSD)

Async WAL Write (Batched Mode):

Multiple Client Requests
     │
     ▼
┌─────────────────┐
│ 1. Batch        │ ──► Collect N requests
│    Requests     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 2. Create WAL   │ ──► Build batch entry
│    Batch Entry  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 3. Append &     │ ──► Single fsync for batch
│    Flush        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ 4. Apply All &  │ ──► Update data region
│    Return       │
└─────────────────┘

Throughput: 100K+ entries/second
```

### Checksum Algorithm

```
CRC32C (Castagnoli CRC32) used for WAL entries:

Advantages:
- Hardware accelerated on modern CPUs (SSE 4.2)
- Detects all single-bit errors
- Detects all errors up to 3 bits
- Detects all odd-number of bit errors
- Detects burst errors up to 32 bits

Implementation:
```rust
use crc32c::crc32c;

fn compute_entry_checksum(entry: &WALEntry) -> u32 {
    // Checksum covers header (excluding checksum field) + data
    let header_bytes = &entry.header[..28]; // Exclude checksum field
    let data_bytes = &entry.data;

    let mut hasher = crc32c::Crc32cHasher::new();
    hasher.write(header_bytes);
    hasher.write(data_bytes);
    hasher.finish()
}

fn verify_entry_checksum(entry: &WALEntry) -> bool {
    let computed = compute_entry_checksum(entry);
    computed == entry.header.checksum
}
```

Corruption detection:
- Read entry from WAL
- Compute checksum
- Compare with stored checksum
- On mismatch: entry corrupted, recovery fails
```

## Part 3: Data File Structure

### Account Record Layout

```
Account Record (Fixed 128 bytes):

┌─────────────────────────────────────────────────────────┐
│ Offset 0x00: ID (u128, 16 bytes)                         │
│ Account unique identifier                                │
├─────────────────────────────────────────────────────────┤
│ Offset 0x10: User Data (u128, 16 bytes)                  │
│ Application-specific data                                │
├─────────────────────────────────────────────────────────┤
│ Offset 0x20: Ledger (u32, 4 bytes)                       │
│ Ledger identifier for multi-ledger support               │
├─────────────────────────────────────────────────────────┤
│ Offset 0x24: Code (u16, 2 bytes)                         │
│ Account type code (1=Asset, 2=Liability, etc.)           │
├─────────────────────────────────────────────────────────┤
│ Offset 0x26: Flags (u16, 2 bytes)                        │
│ Account flags (linked, debit limits, credit limits)      │
├─────────────────────────────────────────────────────────┤
│ Offset 0x28: Debits Pending (u64, 8 bytes)               │
│ Pending debits awaiting commit                           │
├─────────────────────────────────────────────────────────┤
│ Offset 0x30: Debits Posted (u64, 8 bytes)                │
│ Committed debits                                         │
├─────────────────────────────────────────────────────────┤
│ Offset 0x38: Credits Pending (u64, 8 bytes)              │
│ Pending credits awaiting commit                          │
├─────────────────────────────────────────────────────────┤
│ Offset 0x40: Credits Posted (u64, 8 bytes)               │
│ Committed credits                                        │
├─────────────────────────────────────────────────────────┤
│ Offset 0x48-0x7F: Reserved (56 bytes)                    │
│ Future extensions, padding                               │
└─────────────────────────────────────────────────────────┘

Total: 128 bytes

Account lookup:
- Hash table: account_id -> file offset
- Direct indexing: offset = base + (account_id * 128)
```

### Transfer Record Layout

```
Transfer Record (Fixed 128 bytes):

┌─────────────────────────────────────────────────────────┐
│ Offset 0x00: ID (u128, 16 bytes)                         │
│ Transfer unique identifier                               │
├─────────────────────────────────────────────────────────┤
│ Offset 0x10: Debit Account ID (u128, 16 bytes)           │
│ Source account                                           │
├─────────────────────────────────────────────────────────┤
│ Offset 0x20: Credit Account ID (u128, 16 bytes)          │
│ Destination account                                      │
├─────────────────────────────────────────────────────────┤
│ Offset 0x30: Amount (u64, 8 bytes)                       │
│ Transfer amount in smallest currency unit                │
├─────────────────────────────────────────────────────────┤
│ Offset 0x38: Ledger (u32, 4 bytes)                       │
│ Must match both accounts' ledger                         │
├─────────────────────────────────────────────────────────┤
│ Offset 0x3C: Code (u16, 2 bytes)                         │
│ Transfer type code                                       │
├─────────────────────────────────────────────────────────┤
│ Offset 0x3E: Flags (u16, 2 bytes)                        │
│ Transfer flags (linked, pending, etc.)                   │
├─────────────────────────────────────────────────────────┤
│ Offset 0x40: Timestamp (u64, 8 bytes)                    │
│ Nanosecond-precision timestamp                           │
├─────────────────────────────────────────────────────────┤
│ Offset 0x48: Timeout (u64, 8 bytes)                      │
│ Two-phase commit timeout (nanoseconds)                   │
├─────────────────────────────────────────────────────────┤
│ Offset 0x50: Pending ID (u128, 16 bytes)                 │
│ For two-phase commits (references pending transfer)      │
├─────────────────────────────────────────────────────────┤
│ Offset 0x60-0x7F: Reserved (32 bytes)                    │
│ Future extensions, padding                               │
└─────────────────────────────────────────────────────────┘

Total: 128 bytes

Transfer indexing:
- Primary index: transfer_id -> file offset
- Secondary index: pending_id -> transfer_id (for two-phase)
```

### Free Space Management

```
Free List Data Structure:

TigerBeetle uses a bitmap-based free list for data region:

┌─────────────────────────────────────────────────────────┐
│ Free List Header (512 bytes)                             │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Total Blocks: u64                                   │ │
│ │ Free Blocks: u64                                    │ │
│ │ Next Allocation Hint: u64                           │ │
│ │ Checksum: u32                                       │ │
│ └─────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────┤
│ Block Bitmap (Variable)                                  │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Each bit represents one 4KB block                   │ │
│ │ 0 = Free, 1 = Allocated                             │ │
│ │                                                     │ │
│ │ Example: 0b1100_1000_0000_0000                      │ │
│ │          Blocks 0, 1, 4 are allocated               │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘

Allocation strategy:
1. Start at hint position
2. Find first zero bit (free block)
3. Mark bit as 1 (allocated)
4. Update hint for next allocation
5. Return block offset

Deallocation:
1. Mark bit as 0 (free)
2. No immediate coalescing needed

For 8 GB data region with 4KB blocks:
- Total blocks: 2,097,152
- Bitmap size: 262,144 bytes (256 KB)
```

## Part 4: Checkpoint Mechanism

### Checkpoint Process

```
Checkpoint Workflow:

┌─────────────────────────────────────────────────────────┐
│ Phase 1: Prepare Checkpoint                              │
│                                                          │
│ 1. Acquire checkpoint lock                               │
│ 2. Record current LSN                                    │
│ 3. Flush all pending WAL entries                         │
│ 4. Write checkpoint begin marker                         │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Phase 2: Write Checkpoint Data                           │
│                                                          │
│ 5. Iterate all active accounts                           │
│ 6. Write account snapshots to checkpoint region          │
│ 7. Iterate all active transfers                          │
│ 8. Write transfer snapshots                              │
│ 9. Write free list state                                 │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Phase 3: Finalize Checkpoint                             │
│                                                          │
│ 10. Write checkpoint metadata                            │
│     - LSN at checkpoint                                 │
│     - Timestamp                                         │
│     - Checksum                                          │
│ 11. fsync() checkpoint region                            │
│ 12. Write checkpoint complete marker                     │
│ 13. Update superblock checkpoint LSN                     │
│ 14. Release checkpoint lock                              │
│ 15. Truncate WAL entries before checkpoint LSN           │
└─────────────────────────────────────────────────────────┘

Checkpoint frequency: Every N transactions or M seconds
Typical: Every 1000 transactions or 60 seconds

Checkpoint size: Proportional to active data (not WAL size)
```

### Checkpoint Metadata

```
Checkpoint Metadata Structure (256 bytes):

┌─────────────────────────────────────────────────────────┐
│ Magic: u32 = 0xCHKPT (4 bytes)                           │
│ Identifies this as checkpoint metadata                   │
├─────────────────────────────────────────────────────────┤
│ Version: u32 (4 bytes)                                   │
│ Checkpoint format version                                │
├─────────────────────────────────────────────────────────┤
│ LSN: u64 (8 bytes)                                       │
│ Log Sequence Number at checkpoint                        │
├─────────────────────────────────────────────────────────┤
│ Timestamp: u64 (8 bytes)                                 │
│ Nanosecond-precision checkpoint time                     │
├─────────────────────────────────────────────────────────┤
│ Account Count: u64 (8 bytes)                             │
│ Number of accounts in checkpoint                         │
├─────────────────────────────────────────────────────────┤
│ Transfer Count: u64 (8 bytes)                            │
│ Number of transfers in checkpoint                        │
├─────────────────────────────────────────────────────────┤
│ Data Region Offset: u64 (8 bytes)                        │
│ Offset to checkpoint data region                         │
├─────────────────────────────────────────────────────────┤
│ Data Region Size: u64 (8 bytes)                          │
│ Size of checkpoint data                                  │
├─────────────────────────────────────────────────────────┤
│ Free List Offset: u64 (8 bytes)                          │
│ Offset to free list snapshot                             │
├─────────────────────────────────────────────────────────┤
│ Checksum: u32 (4 bytes)                                  │
│ CRC32C of entire metadata                                │
├─────────────────────────────────────────────────────────┤
│ Reserved (188 bytes)                                     │
│ Future extensions                                        │
└─────────────────────────────────────────────────────────┘

Multiple checkpoints:
- TigerBeetle keeps last N checkpoints
- Enables point-in-time recovery
- Typical: Keep last 3 checkpoints
```

### Incremental Checkpoints

```
Incremental vs Full Checkpoints:

Full Checkpoint:
┌─────────────────────────────────────────────────────────┐
│ Writes ALL accounts and transfers                        │
│                                                          │
│ Pros: Simple recovery                                    │
│ Cons: Large, slow for big databases                      │
│                                                          │
│ Use: First checkpoint, periodic full backup              │
└─────────────────────────────────────────────────────────┘

Incremental Checkpoint:
┌─────────────────────────────────────────────────────────┐
│ Writes ONLY modified accounts/transfers since last       │
│                                                          │
│ Pros: Fast, small                                        │
│ Cons: Recovery requires chain of checkpoints             │
│                                                          │
│ Use: Frequent checkpoints between full backups           │
└─────────────────────────────────────────────────────────┘

TigerBeetle Strategy:
- Full checkpoint every N full checkpoints (e.g., every 10th)
- Incremental checkpoints in between
- Checkpoint chain tracked in superblock

Recovery with incremental:
1. Find latest full checkpoint
2. Apply incremental checkpoints in order
3. Replay WAL from last incremental checkpoint
```

## Part 5: Recovery Process

### Crash Recovery Steps

```
Recovery After Crash:

┌─────────────────────────────────────────────────────────┐
│ Step 1: Validate Data File                               │
│                                                          │
│ - Check magic number                                     │
│ - Verify superblock checksum                             │
│ - Validate file size (must be 10 GB)                     │
│ - Check file type matches role                           │
│                                                          │
│ On failure: Database corrupted, restore from backup      │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 2: Load Latest Checkpoint                           │
│                                                          │
│ - Read checkpoint metadata                               │
│ - Verify checkpoint checksum                             │
│ - Load account snapshots                                 │
│ - Load transfer snapshots                                │
│ - Load free list                                         │
│                                                          │
│ On failure: Try previous checkpoint                      │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 3: Replay WAL                                       │
│                                                          │
│ - Start from checkpoint LSN                              │
│ - Read each WAL entry                                    │
│ - Verify entry checksum                                  │
│ - Apply entry to data region                             │
│ - Stop at first invalid/corrupted entry                  │
│                                                          │
│ Entries after checkpoint = committed but not checkpointed│
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 4: Validate State                                   │
│                                                          │
│ - Verify double-entry invariant:                         │
│   sum(credits_posted) - sum(debits_posted) = constant    │
│ - Verify all pending transfers have valid timeout        │
│ - Verify free list consistency                           │
│                                                          │
│ On failure: Recovery failed, restore from backup         │
└─────────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│ Step 5: Resume Normal Operation                          │
│                                                          │
│ - Initialize WAL tail pointer                            │
│ - Accept client connections                              │
│ - Process pending two-phase commits                      │
│                                                          │
│ Recovery complete                                        │
└─────────────────────────────────────────────────────────┘

Recovery Time Objective (RTO): < 1 minute for typical DB
Recovery Point Objective (RPO): Zero (no committed data lost)
```

### WAL Replay Algorithm

```rust
/// Replay WAL entries from checkpoint LSN
fn replay_wal(storage: &mut StorageEngine) -> Result<()> {
    let checkpoint_lsn = storage.checkpoint.lsn;
    let mut current_lsn = checkpoint_lsn + 1;

    loop {
        // Read WAL entry at current position
        let entry = match storage.read_wal_entry(current_lsn) {
            Ok(entry) => entry,
            Err(WALError::NotFound) => break, // End of WAL
            Err(e) => return Err(e.into()),
        };

        // Verify checksum
        if !verify_entry_checksum(&entry) {
            log::warn!("WAL entry {} has invalid checksum", current_lsn);
            break; // Stop at corrupted entry
        }

        // Apply entry to data region
        match entry.entry_type {
            EntryType::AccountCreate => {
                let account = Account::from_bytes(&entry.data);
                storage.write_account(&account);
            }
            EntryType::AccountUpdate => {
                let account = Account::from_bytes(&entry.data);
                storage.write_account(&account);
            }
            EntryType::TransferCreate => {
                let transfer = Transfer::from_bytes(&entry.data);
                storage.write_transfer(&transfer);

                // Apply transfer to accounts
                storage.apply_transfer(&transfer)?;
            }
            EntryType::TransferCommit => {
                let pending_id = Transfer::from_bytes(&entry.data).pending_id;
                storage.commit_pending_transfer(pending_id)?;
            }
            EntryType::TransferVoid => {
                let pending_id = Transfer::from_bytes(&entry.data).pending_id;
                storage.void_pending_transfer(pending_id)?;
            }
            EntryType::Checkpoint => {
                // Update checkpoint metadata
                storage.checkpoint = Checkpoint::from_bytes(&entry.data);
            }
        }

        current_lsn += 1;
    }

    log::info!("WAL replay complete: {} entries replayed",
               current_lsn - checkpoint_lsn);

    Ok(())
}
```

### Partial Write Handling

```
Partial Write Detection:

Problem: Crash during WAL write leaves partial entry

Detection:
1. Magic number present but entry truncated
2. Checksum mismatch
3. LSN gap (missing sequential entry)

Handling:
┌─────────────────────────────────────────────────────────┐
│ 1. Scan WAL for valid entries                            │
│                                                          │
│    Position 1000: Valid (checksum OK)                    │
│    Position 1001: Valid (checksum OK)                    │
│    Position 1002: CORRUPTED (partial write)              │
│    Position 1003: Not found (beyond tail)                │
│                                                          │
│ 2. Truncate at first corrupted entry                       │
│                                                          │
│    - Entries 1000, 1001: Kept                            │
│    - Entry 1002: Discarded (partial)                     │
│                                                          │
│ 3. Recovery continues from last valid entry              │
│                                                          │
│ Trade-off: Lost uncommitted writes (acceptable)          │
│            Committed writes never lost (fsync before ack)│
└─────────────────────────────────────────────────────────┘

Prevention:
- Write entries in aligned blocks (4KB)
- Use writev() for atomic multi-buffer writes
- fsync() after each entry
```

## Part 6: Performance Optimizations

### Batched Commits

```
Batched WAL Commits:

Without Batching:
Client1: WRITE ──► fsync ──► ACK (100μs)
Client2: WRITE ──► fsync ──► ACK (100μs)
Client3: WRITE ──► fsync ──► ACK (100μs)
Total: 300μs for 3 writes

With Batching:
Client1: WRITE ──┐
Client2: WRITE ──┼─► Batch (3) ──► fsync ──► ACK all (100μs)
Client3: WRITE ──┘
Total: 100μs for 3 writes (3x throughput)

Implementation:
```rust
struct WALBatcher {
    pending: Vec<WALEntry>,
    batch_size: usize,
    batch_timeout: Duration,
    last_flush: Instant,
}

impl WALBatcher {
    fn add_entry(&mut self, entry: WALEntry) -> oneshot::Receiver<Result<()>> {
        let (tx, rx) = oneshot::channel();
        self.pending.push(PendingEntry { entry, tx });

        if self.pending.len() >= self.batch_size
            || self.last_flush.elapsed() > self.batch_timeout
        {
            self.flush();
        }

        rx
    }

    fn flush(&mut self) {
        // Build batch WAL entry
        let batch = self.build_batch();

        // Write and fsync once
        self.storage.append_and_fsync(&batch);

        // Notify all clients
        for pending in self.pending.drain(..) {
            pending.tx.send(Ok(()));
        }

        self.last_flush = Instant::now();
    }
}
```

### Parallel Checkpointing

```
Parallel Checkpoint Write:

Traditional (Sequential):
1. Write accounts (100ms)
2. Write transfers (50ms)
3. Write free list (10ms)
4. Write metadata (5ms)
Total: 165ms

Parallel:
┌─────────────────────────────────────────────────────────┐
│ Thread 1: Write accounts ────────────► 100ms            │
│ Thread 2: Write transfers ─────────► 50ms               │
│ Thread 3: Write free list ─► 10ms                       │
│ Main: Write metadata (after all complete) ─► 5ms        │
│                                                          │
│ Total: 105ms (parallel) + 5ms (metadata) = 110ms        │
│ Speedup: 1.5x                                           │
└─────────────────────────────────────────────────────────┘

Implementation:
```rust
fn parallel_checkpoint(storage: &StorageEngine) -> Checkpoint {
    let (account_tx, account_rx) = channel();
    let (transfer_tx, transfer_rx) = channel();

    // Thread 1: Serialize accounts
    spawn(move || {
        let accounts = storage.snapshot_accounts();
        account_tx.send(accounts);
    });

    // Thread 2: Serialize transfers
    spawn(move || {
        let transfers = storage.snapshot_transfers();
        transfer_tx.send(transfers);
    });

    // Main thread: Wait for both
    let accounts = account_rx.recv().unwrap();
    let transfers = transfer_rx.recv().unwrap();

    // Write checkpoint data
    storage.write_checkpoint_data(&accounts, &transfers);
}
```

### Compression

```
WAL Entry Compression:

Without Compression:
WAL Entry: 128 bytes
100K entries/second = 12.8 MB/s WAL growth

With Compression (LZ4):
WAL Entry: ~50 bytes average (60% reduction)
100K entries/second = 5 MB/s WAL growth

Trade-offs:
┌─────────────────────────────────────────────────────────┐
│ Pros:                                                    │
│ - Reduced WAL size                                       │
│ - Faster network replication (less data)                 │
│ - Longer WAL retention (more history)                    │
│                                                          │
│ Cons:                                                    │
│ - CPU overhead for compression/decompression             │
│ - Added latency on write path                            │
│ - Complexity in recovery                                 │
└─────────────────────────────────────────────────────────┘

TigerBeetle Approach:
- Optional LZ4 compression for WAL entries
- Compression threshold: Only compress entries > 64 bytes
- Checksum covers compressed data (detect corruption early)
```

---

*This document is part of the TigerBeetle exploration series. See [exploration.md](./exploration.md) for the complete index.*
