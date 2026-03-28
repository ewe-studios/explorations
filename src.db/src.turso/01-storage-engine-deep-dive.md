---
title: "Storage Engine Deep Dive: Turso/libSQL"
subtitle: "WAL architecture, B-tree internals, and embedded replica storage"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.turso
explored_at: 2026-03-28
related: 00-zero-to-db-engineer.md, exploration.md
---

# 01 - Storage Engine Deep Dive

## Overview

This document dives into how libSQL stores data on disk, manages the Write-Ahead Log, and synchronizes replicas. We'll examine the actual file formats, page structures, and sync protocol.

## Part 1: SQLite File Format

### Database File Structure

```
SQLite Database File (main.db)
┌─────────────────────────────────┐
│   100-byte header               │  ← Database configuration
├─────────────────────────────────┤
│   Page 1 (B-tree for schema)    │  ← sqlite_master table
├─────────────────────────────────┤
│   Page 2                        │  ← User data or overflow
├─────────────────────────────────┤
│   Page 3                        │
├─────────────────────────────────┤
│   ...                           │
├─────────────────────────────────┤
│   Page N                        │
├─────────────────────────────────┤
│   WAL header (if WAL mode)      │  ← Points to WAL file
└─────────────────────────────────┘

Page size: Typically 4096 bytes (configurable: 512 to 65536)
```

### The 100-Byte Header

```
Offset  Size  Description
──────  ────  ───────────────────────────────────────────
0       16    Header string: "SQLite format 3\000"
16      2     Page size (e.g., 4096 = 0x1000)
18      1     File format write version (1=legacy, 2=WAL)
19      1     File format read version
20      1     Reserved space at end of each page
21      1     Max embedded payload fraction (must be 64)
22      1     Min embedded payload fraction (must be 32)
23      1     Leaf payload fraction (must be 32)
24      4     File change counter
28      4     Database size in pages
32      4     First freelist trunk page
36      4     Total freelist pages
40      4     Schema cookie
44      4     Schema format number
48      4     Default page cache size
52      4     Largest root B-tree page (auto-vacuum)
56      4     Database text encoding (1=UTF8, 2=UTF16LE, 3=UTF16BE)
60      4     User version
64      4     Incremental vacuum mode
68      4     Application ID
72      4     Reserved for expansion
76      4     Version-valid-for (same as change counter)
80      20    SQLite version number
```

### Reading the Header (Rust Example)

```rust
#[derive(Debug)]
struct SqliteHeader {
    magic: [u8; 16],
    page_size: u16,
    write_version: u8,
    read_version: u8,
    reserved_space: u8,
    max_payload_fraction: u8,
    min_payload_fraction: u8,
    leaf_payload_fraction: u8,
    file_change_counter: u32,
    database_size_pages: u32,
    // ... more fields
}

impl SqliteHeader {
    fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < 100 {
            return Err(ParseError::TooShort);
        }

        Ok(Self {
            magic: data[0..16].try_into().unwrap(),
            page_size: u16::from_be_bytes([data[16], data[17]]),
            write_version: data[18],
            read_version: data[19],
            reserved_space: data[20],
            max_payload_fraction: data[21],
            min_payload_fraction: data[22],
            leaf_payload_fraction: data[23],
            file_change_counter: u32::from_be_bytes([data[24], data[25], data[26], data[27]]),
            database_size_pages: u32::from_be_bytes([data[28], data[29], data[30], data[31]]),
            // ...
        })
    }
}
```

## Part 2: B-Tree Page Structure

### Page Types

```
Page Type Indicator (first byte of page content):
- 0x00: Invalid/unused
- 0x02: Interior index B-tree page
- 0x05: Interior table B-tree page
- 0x0a: Leaf index B-tree page
- 0x0d: Leaf table B-tree page
```

### Leaf Table B-Tree Page Structure

```
┌─────────────────────────────────┐
│ Page Type (0x0d)                │  ← 1 byte
├─────────────────────────────────┤
│ First Freeblock Offset          │  ← 2 bytes
├─────────────────────────────────┤
│ Number of Cells                 │  ← 2 bytes
├─────────────────────────────────┤
│ Cell Content Area Start         │  ← 2 bytes
├─────────────────────────────────┤
│ Fragmented Free Bytes           │  ← 1 byte
├─────────────────────────────────┤
│ Cell Pointer Array              │  ← N × 2 bytes (offsets to cells)
├─────────────────────────────────┤
│ Unallocated Space               │
├─────────────────────────────────┤
│ Cell Content Area               │  ← Grows from end of page toward start
│   ┌─────────────────────────┐   │
│   │ Cell 1                  │   │
│   │ ┌─────────────────────┐ │   │
│   │ │ Payload Length      │ │   │
│   │ ├─────────────────────┤ │   │
│   │ │ Row ID              │ │   │
│   │ ├─────────────────────┤ │   │
│   │ │ Payload (data)      │ │   │
│   │ └─────────────────────┘ │   │
│   └─────────────────────────┘   │
└─────────────────────────────────┘
```

### Cell Structure (Leaf Table)

```
Variable-length integer (varint) encoding:
- Uses 1-9 bytes
- Lower 7 bits of each byte are data
- High bit indicates continuation (1=more bytes, 0=last byte)

Example: 300 = 0x12C
Binary: 100101100
Split:  0000010 0101100
Bytes:  10000010 00101100 = 0x82 0x2C

Cell format:
┌──────────────────────┐
│ Payload Length       │  ← Varint (1-9 bytes)
├──────────────────────┤
│ Row ID               │  ← Varint (1-9 bytes)
├──────────────────────┤
│ Payload              │  ← Variable
│ ┌──────────────────┐ │
│ │ Header Size      │ │  ← Varint
│ ├──────────────────┤ │
│ │ Column Type 1    │ │  ← Varint (0=nil, 1=uint8, 2=int16, 3=int24, 4=int32...)
│ ├──────────────────┤ │
│ │ Column Type 2    │ │
│ ├──────────────────┤ │
│ │ ...              │ │
│ ├──────────────────┤ │
│ │ Column Value 1   │ │  ← Raw bytes
│ ├──────────────────┤ │
│ │ Column Value 2   │ │
│ └──────────────────┘ │
└──────────────────────┘
```

### Interior B-Tree Page

```
Interior Table B-Tree Page:
┌─────────────────────────────────┐
│ Page Type (0x05)                │
├─────────────────────────────────┤
│ First Freeblock Offset          │
├─────────────────────────────────┤
│ Number of Cells                 │
├─────────────────────────────────┤
│ Cell Content Area Start         │
├─────────────────────────────────┤
│ Fragmented Free Bytes           │
├─────────────────────────────────┤
│ Right-most Pointer              │  ← 4 bytes (child page > all keys)
├─────────────────────────────────┤
│ Cell Pointer Array              │
└─────────────────────────────────┘

Interior Cell:
┌──────────────────────┐
│ Left Child Page      │  ← 4 bytes
├──────────────────────┤
│ Row ID (key)         │  ← Varint (largest key in left subtree)
└──────────────────────┘
```

## Part 3: WAL File Format

### WAL Header (32 bytes)

```
Offset  Size  Description
──────  ────  ───────────────────────────────────────────
0       4     Magic number (0x377f0682 or 0x377f0683)
                - 0x377f0682: Native byte order
                - 0x377f0683: Swapped byte order
4       4     WAL format version (3007000 for SQLite 3.7.0)
8       4     Database page size
12      4     Checkpoint sequence number
16      4     Salt-1 (random value for validation)
20      4     Salt-2 (random value for validation)
24      4     Checksum-1 (CRC32 of header)
28      4     Checksum-2
```

### WAL Frame Header (24 bytes)

```
Offset  Size  Description
──────  ────  ───────────────────────────────────────────
0       4     Page number (0 = no-op)
4       4     For commit: size of database in pages
                For non-commit: 0
8       4     Salt-1 (must match WAL header)
12      4     Salt-2 (must match WAL header)
16      4     Checksum-1 (data + header)
20      4     Checksum-2
```

### Complete WAL File Structure

```
┌─────────────────────────────────┐
│ WAL Header (32 bytes)           │
├─────────────────────────────────┤
│ Frame Header (24 bytes)         │  ← Frame 1
├─────────────────────────────────┤
│ Page Data (4096 bytes)          │  ← Page NNN
├─────────────────────────────────┤
│ Frame Header (24 bytes)         │  ← Frame 2
├─────────────────────────────────┤
│ Page Data (4096 bytes)          │  ← Page MMM
├─────────────────────────────────┤
│ ...                             │
├─────────────────────────────────┤
│ Frame Header (24 bytes)         │  ← Last frame (commit)
├─────────────────────────────────┤
│ Page Data (4096 bytes)          │
└─────────────────────────────────┘

Each frame = 24 byte header + page_size bytes of data
```

### WAL Implementation (Rust)

```rust
const WAL_HEADER_SIZE: usize = 32;
const WAL_FRAME_HEADER_SIZE: usize = 24;

#[derive(Debug)]
struct WalHeader {
    magic: u32,
    version: u32,
    page_size: u32,
    checkpoint_seq: u32,
    salt1: u32,
    salt2: u32,
    checksum1: u32,
    checksum2: u32,
}

#[derive(Debug)]
struct WalFrame {
    page_number: u32,
    db_size_pages: u32,  // 0 if not a commit frame
    salt1: u32,
    salt2: u32,
    checksum1: u32,
    checksum2: u32,
    data: Vec<u8>,  // page_size bytes
}

impl WalFrame {
    fn is_commit(&self) -> bool {
        self.db_size_pages > 0
    }

    fn size_on_disk(page_size: u32) -> u32 {
        WAL_FRAME_HEADER_SIZE as u32 + page_size
    }
}

struct WalFile {
    header: WalHeader,
    frames: Vec<WalFrame>,
}

impl WalFile {
    fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() < WAL_HEADER_SIZE {
            return Err(ParseError::TooShort);
        }

        let header = WalHeader {
            magic: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
            version: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            page_size: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            checkpoint_seq: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
            salt1: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
            salt2: u32::from_be_bytes([data[20], data[21], data[22], data[23]]),
            checksum1: u32::from_be_bytes([data[24], data[25], data[26], data[27]]),
            checksum2: u32::from_be_bytes([data[28], data[29], data[30], data[31]]),
        };

        // Validate magic
        if header.magic != 0x377f0682 && header.magic != 0x377f0683 {
            return Err(ParseError::InvalidMagic);
        }

        // Parse frames
        let mut frames = Vec::new();
        let mut offset = WAL_HEADER_SIZE;

        while offset + WAL_FRAME_HEADER_SIZE <= data.len() {
            let page_number = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);

            let db_size_pages = u32::from_be_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);

            let frame_data = data[offset + WAL_FRAME_HEADER_SIZE..]
                [..header.page_size as usize]
                .to_vec();

            frames.push(WalFrame {
                page_number,
                db_size_pages,
                salt1: u32::from_be_bytes([
                    data[offset + 8],
                    data[offset + 9],
                    data[offset + 10],
                    data[offset + 11],
                ]),
                salt2: u32::from_be_bytes([
                    data[offset + 12],
                    data[offset + 13],
                    data[offset + 14],
                    data[offset + 15],
                ]),
                checksum1: u32::from_be_bytes([
                    data[offset + 16],
                    data[offset + 17],
                    data[offset + 18],
                    data[offset + 19],
                ]),
                checksum2: u32::from_be_bytes([
                    data[offset + 20],
                    data[offset + 21],
                    data[offset + 22],
                    data[offset + 23],
                ]),
                data: frame_data,
            });

            offset += WAL_FRAME_HEADER_SIZE + header.page_size as usize;
        }

        Ok(Self { header, frames })
    }

    fn get_latest_page(&self, page_num: u32) -> Option<&[u8]> {
        // Search frames in reverse, return last frame with this page number
        self.frames
            .iter()
            .rev()
            .find(|f| f.page_number == page_num)
            .map(|f| f.data.as_slice())
    }

    fn commit_frame(&self) -> Option<&WalFrame> {
        self.frames.iter().rev().find(|f| f.is_commit())
    }
}
```

## Part 4: Checkpoint Process

### What is Checkpointing?

Checkpointing transfers committed WAL frames back to the main database file:

```
Before Checkpoint:
┌─────────────────┐         ┌─────────────────┐
│  main.db        │         │  main.db-wal    │
│                 │         │                 │
│ Page 1: v1      │ ──────→ │ Frame 1: P1 v2  │
│ Page 2: v1      │         │ Frame 2: P2 v2  │
│ Page 3: v1      │         │ Frame 3: P1 v3  │  ← Latest!
└─────────────────┘         └─────────────────┘

After Checkpoint:
┌─────────────────┐         ┌─────────────────┐
│  main.db        │         │  main.db-wal    │
│                 │         │                 │
│ Page 1: v3      │         │ (empty)         │
│ Page 2: v2      │         │                 │
│ Page 3: v1      │         │                 │
└─────────────────┘         └─────────────────┘
```

### Checkpoint Modes

**PASSIVE** (default):
- Only checkpoints frames that readers have finished with
- Non-blocking: readers/writers can continue
- May not checkpoint all frames

**FULL**:
- Waits for all readers to finish
- Checkpoints ALL committed frames
- Blocks new readers briefly

**RESTART**:
- Full checkpoint + resets WAL
- Prevents new writes until complete
- Used for backup/vacuum

**TRUNCATE**:
- Full checkpoint + truncates WAL file
- Frees disk space
- Requires exclusive access

### Checkpoint Algorithm

```rust
struct CheckpointResult {
    busy_count: u32,      // Pages blocked by readers
    log_pages: u32,       // Total pages in WAL
    checkpointed_pages: u32,
}

fn checkpoint_passive(
    db_file: &mut File,
    wal_file: &WalFile,
    read_locks: &[bool; 4],  // 4 reader locks
) -> Result<CheckpointResult, Error> {
    let mut result = CheckpointResult::default();
    result.log_pages = wal_file.frames.len() as u32;

    // Find commit frame
    let Some(commit_frame) = wal_file.commit_frame() else {
        return Ok(result);  // No committed data to checkpoint
    };

    // Check each page
    for frame in &wal_file.frames {
        let page_idx = frame.page_number as usize - 1;  // 1-indexed

        // Check if any reader has this page locked
        if read_locks.iter().any(|&lock| lock) {
            result.busy_count += 1;
            continue;  // Skip this page
        }

        // Seek to page location in db_file
        db_file.seek(SeekFrom::Start(
            (page_idx * wal_file.header.page_size as usize) as u64
        ))?;

        // Write page data
        db_file.write_all(&frame.data)?;
        result.checkpointed_pages += 1;
    }

    // Sync database file
    db_file.sync_all()?;

    Ok(result)
}
```

### WAL Index (Shared Memory)

```
The WAL index (-shm file) maps which pages are in WAL:

┌─────────────────────────────────────────┐
│ WAL Index Header                        │
├─────────────────────────────────────────┤
│ Hash Table: page_num → frame_offset     │
│ ┌─────────┬─────────┬─────────┐         │
│ │ Page 1  │ Page 42 │ Page 99 │ ...     │
│ │ 0x0058  │ 0x1120  │ 0x2348  │         │
│ └─────────┴─────────┴─────────┘         │
└─────────────────────────────────────────┘

Why? Fast lookup: "Where is page N?"
Without index: Scan entire WAL (O(n))
With index: Hash lookup (O(1))
```

## Part 5: libSQL Extensions

### Embedded Replica Storage

```
libSQL adds a replication layer on top of SQLite:

┌──────────────────────────────────────────┐
│          libSQL Client                   │
├──────────────────────────────────────────┤
│  ┌────────────────────────────────────┐  │
│  │   Embedded Replica (local SQLite)  │  │
│  │   ┌─────────┐ ┌─────────────────┐  │  │
│  │   │ main.db │ │ main.db-wal     │  │  │
│  │   └─────────┘ └─────────────────┘  │  │
│  └────────────────────────────────────┘  │
│                    │                      │
│                    ▼                      │
│  ┌────────────────────────────────────┐  │
│  │   Sync Engine                      │  │
│  │   - Track WAL position            │  │
│  │   - Fetch new frames from primary  │  │
│  │   - Apply frames to local WAL      │  │
│  └────────────────────────────────────┘  │
└──────────────────────────────────────────┘
                    │
                    │ HTTP/gRPC
                    ▼
┌──────────────────────────────────────────┐
│          Primary Database                │
│  ┌────────────────────────────────────┐  │
│  │   SQLite with WAL                  │  │
│  └────────────────────────────────────┘  │
└──────────────────────────────────────────┘
```

### Sync Protocol Messages

```protobuf
// Request: Client → Primary
message SyncRequest {
    uint64 replica_frame_offset = 1;  // "I have frames up to this offset"
    optional bytes replica_uuid = 2;   // Replica identifier
}

// Response: Primary → Client
message SyncResponse {
    uint64 current_frame_offset = 1;   // "Latest frame is at offset X"
    repeated WalFrame frames = 2;       // New frames since offset
    uint64 database_size_pages = 3;    // Current DB size
}

// Individual WAL Frame
message WalFrame {
    uint32 page_number = 1;
    bytes page_data = 2;
    bool is_commit = 3;
}
```

### Replica State Machine

```rust
enum ReplicaState {
    /// Initial state: need full snapshot
    Empty,

    /// Catching up: applying WAL frames
    Syncing {
        target_offset: u64,
        current_offset: u64,
    },

    /// Fully synced, ready for queries
    Idle {
        current_offset: u64,
    },

    /// Error during sync, retry needed
    Error {
        error: SyncError,
        retry_after: Duration,
    },
}

impl ReplicaState {
    fn handle_sync_request(&mut self, frames: &[WalFrame]) -> Result<(), SyncError> {
        match self {
            ReplicaState::Empty => {
                // Need initial snapshot first
                Err(SyncError::NeedSnapshot)
            }
            ReplicaState::Syncing { current_offset, target_offset } => {
                for frame in frames {
                    self.apply_frame(frame)?;
                    *current_offset += frame.size_on_disk();
                }

                if *current_offset >= *target_offset {
                    *self = ReplicaState::Idle {
                        current_offset: *current_offset,
                    };
                }
                Ok(())
            }
            ReplicaState::Idle { current_offset } => {
                if frames.is_empty() {
                    return Ok(());  // Already caught up
                }

                let target = *current_offset + frames.iter()
                    .map(|f| f.size_on_disk())
                    .sum::<u64>();

                *self = ReplicaState::Syncing {
                    target_offset: target,
                    current_offset: *current_offset,
                };

                self.handle_sync_request(frames)
            }
            ReplicaState::Error { .. } => {
                // Reset to Idle and retry
                let offset = *current_offset;
                *self = ReplicaState::Idle { current_offset: offset };
                self.handle_sync_request(frames)
            }
        }
    }

    fn apply_frame(&mut self, frame: &WalFrame) -> Result<(), SyncError> {
        // Write frame to local WAL file
        // Update WAL index
        // If commit frame, update database size
        Ok(())
    }
}
```

## Part 6: Memory Management

### Page Cache

```rust
struct PageCache {
    /// Cached pages (page_num → page_data)
    cache: HashMap<u32, Box<[u8; 4096]>>,

    /// LRU ordering for eviction
    lru: VecDeque<u32>,

    /// Maximum cache size in pages
    max_pages: usize,

    /// Dirty pages (need to be written)
    dirty: HashSet<u32>,
}

impl PageCache {
    fn new(max_size_mb: usize) -> Self {
        let max_pages = (max_size_mb * 1024 * 1024) / 4096;
        Self {
            cache: HashMap::with_capacity(max_pages),
            lru: VecDeque::with_capacity(max_pages),
            max_pages,
            dirty: HashSet::new(),
        }
    }

    fn get(&mut self, page_num: u32) -> Option<&[u8; 4096]> {
        // Move to front of LRU (most recently used)
        if let Some(pos) = self.lru.iter().position(|&p| p == page_num) {
            self.lru.remove(pos);
            self.lru.push_front(page_num);
        }

        self.cache.get(&page_num)
    }

    fn put(&mut self, page_num: u32, data: Box<[u8; 4096]>) {
        // Evict if full
        while self.cache.len() >= self.max_pages {
            if let Some(oldest) = self.lru.pop_back() {
                self.cache.remove(&oldest);
                self.dirty.remove(&oldest);
            }
        }

        self.cache.insert(page_num, data);
        self.lru.push_front(page_num);
    }

    fn mark_dirty(&mut self, page_num: u32) {
        self.dirty.insert(page_num);
    }

    fn flush_dirty(&mut self, file: &mut File) -> io::Result<()> {
        for &page_num in &self.dirty.clone() {
            let data = self.cache.get(&page_num).unwrap();
            let offset = (page_num as u64 - 1) * 4096;
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(data)?;
        }
        self.dirty.clear();
        file.sync_all()?;
        Ok(())
    }
}
```

### Memory-Mapped I/O

```rust
// libsql uses mmap for efficient page access

use memmap2::MmapMut;

struct MmapStorage {
    mmap: MmapMut,
    page_size: usize,
}

impl MmapStorage {
    fn open(path: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;

        // Get file size
        let len = file.metadata()?.len();

        // Create memory map
        let mut mmap = unsafe { MmapMut::map_mut(&file)? };

        // Initialize if empty
        if len == 0 {
            // Initialize with SQLite header
            mmap[..100].copy_from_slice(&SQLITE_HEADER_TEMPLATE);
            mmap.flush()?;
        }

        Ok(Self {
            mmap,
            page_size: 4096,
        })
    }

    fn read_page(&self, page_num: u32) -> &[u8] {
        let offset = (page_num - 1) as usize * self.page_size;
        &self.mmap[offset..offset + self.page_size]
    }

    fn write_page(&mut self, page_num: u32, data: &[u8]) {
        let offset = (page_num - 1) as usize * self.page_size;
        self.mmap[offset..offset + self.page_size].copy_from_slice(data);
    }

    fn sync(&mut self) -> io::Result<()> {
        self.mmap.flush()?;
        Ok(())
    }
}
```

## Part 7: Performance Optimization

### Batch Writes

```rust
// BAD: Individual writes (each is a transaction)
for row in rows {
    db.execute("INSERT INTO t VALUES (?)", [row])?;
    // Each: Begin → Write WAL → Sync → Commit
}

// GOOD: Single transaction
db.execute("BEGIN")?;
for row in rows {
    db.execute("INSERT INTO t VALUES (?)", [row])?;
}
db.execute("COMMIT")?;
// One: Begin → Write WAL (all) → Sync → Commit

// Performance difference:
// 1000 individual: ~1000ms (1ms each)
// 1000 batched: ~10ms (100x faster!)
```

### WAL Write Coalescing

```
Without coalescing:
Frame 1 → Write (4KB) → Sync
Frame 2 → Write (4KB) → Sync
Frame 3 → Write (4KB) → Sync
Total: 3 writes, 3 syncs

With coalescing:
Frame 1 → Buffer
Frame 2 → Buffer
Frame 3 → Buffer (commit) → Write (12KB) → Sync
Total: 1 write, 1 sync
```

### Read Path Optimization

```rust
/// Optimized read: Check WAL index first, then cache, then disk
fn read_page_optimized(
    db: &Database,
    page_num: u32,
) -> Result<Cow<[u8]>, Error> {
    // 1. Check WAL index for latest version
    if let Some(frame_offset) = db.wal_index.lookup(page_num) {
        // Found in WAL - check frame cache first
        if let Some(cached) = db.frame_cache.get(frame_offset) {
            return Ok(Cow::Borrowed(cached));
        }

        // Read from WAL file
        let frame = db.read_wal_frame(frame_offset)?;
        db.frame_cache.insert(frame_offset, frame.data.clone());
        return Ok(Cow::Owned(frame.data));
    }

    // 2. Check page cache
    if let Some(cached) = db.page_cache.get(page_num) {
        return Ok(Cow::Borrowed(cached.as_slice()));
    }

    // 3. Read from database file
    let mut page = Box::new([0u8; 4096]);
    let offset = (page_num - 1) as u64 * 4096;
    db.file.seek(SeekFrom::Start(offset))?;
    db.file.read_exact(&mut page[..])?;

    // Cache for next time
    db.page_cache.put(page_num, page.clone());

    Ok(Cow::Owned(page.to_vec()))
}
```

---

*This document is part of the Turso/libSQL exploration series. See [exploration.md](./exploration.md) for the complete index.*
