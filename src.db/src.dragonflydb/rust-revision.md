---
title: "Rust Revision: DragonflyDB"
subtitle: "Valtron-based Rust translation guide"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.dragonflydb
related: 00-zero-to-db-engineer.md, 01-storage-engine-deep-dive.md, exploration.md
---

# Rust Revision: DragonflyDB

## Overview

This document provides a guide for translating DragonflyDB's C++ implementation to Rust using the Valtron executor pattern - no async/await, no tokio, just pure synchronous Rust with algebraic effects for I/O.

**Note:** Unlike Turso/libSQL which required translating SQLite's C code to Rust, DragonflyDB is already written in C++ with modern patterns. This translation focuses on:
1. Replacing async patterns with Valtron's task iterator
2. Implementing the shared-nothing architecture in Rust
3. Using Rust's type system for memory safety

## Part 1: Core Architecture Translation

### C++ to Rust Mapping

```cpp
// C++ (Dragonfly original)
class Shard {
public:
    Dashtable* dashtable;
    EngineShard* engine;
    Journal* journal;

    void Init();
    void Shutdown();
    OpStatus Add(const std::string& key, std::string_view value);
};
```

```rust
// Rust (Valtron style)
struct Shard {
    dashtable: Dashtable,
    engine: EngineShard,
    journal: Option<Journal>,
    shard_id: ShardId,
}

impl Shard {
    fn new(shard_id: ShardId) -> Self {
        Self {
            dashtable: Dashtable::new(),
            engine: EngineShard::new(shard_id),
            journal: None,
            shard_id,
        }
    }

    fn add(&mut self, key: &str, value: &[u8]) -> OpStatus {
        self.dashtable.insert(key, value);
        if let Some(journal) = &mut self.journal {
            journal.record(JournalOp::Insert {
                key: key.to_string(),
                value: value.to_vec(),
            });
        }
        OpStatus::Ok
    }
}
```

### Thread Model Translation

```cpp
// C++ - Uses helio library for fiber scheduling
// Each shard runs on its own thread

void EngineShard::StartThread() {
    fb2::Fiber("shard_task", [this]() {
        while (running_) {
            ProcessMessages();
            ExecuteTransactions();
        }
    }).Detach();
}
```

```rust
// Rust - Using standard threading with Valtron pattern
// No external async runtime needed

struct ShardThread {
    handle: Option<JoinHandle<()>>,
    stop_signal: Arc<AtomicBool>,
    message_queue: Arc<Mutex<VecDeque<ShardMessage>>>,
}

impl ShardThread {
    fn spawn(shard_id: ShardId) -> Self {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let message_queue = Arc::new(Mutex::new(VecDeque::new()));

        let stop_clone = stop_signal.clone();
        let queue_clone = message_queue.clone();

        let handle = spawn(move || {
            let mut shard = Shard::new(shard_id);

            while !stop_clone.load(Ordering::Relaxed) {
                // Process messages
                let mut messages = queue_clone.lock().unwrap();
                while let Some(msg) = messages.pop_front() {
                    shard.handle_message(msg);
                }
                drop(messages);

                // Yield to avoid busy-spinning
                std::thread::yield_now();
            }
        });

        Self {
            handle: Some(handle),
            stop_signal,
            message_queue,
        }
    }
}
```

## Part 2: Dashtable in Rust

### Core Structures

```rust
/// Dashtable - memory-efficient hashtable
pub struct Dashtable {
    /// Directory of segments
    directory: Vec<SegmentPtr>,
    /// Log2 of directory size
    log_size: u32,
    /// Total item count
    size: usize,
}

/// Pointer to segment (with metadata)
struct SegmentPtr(NonNull<Segment>);

/// Segment - fixed-size bucket array
struct Segment {
    /// 56 regular buckets
    regular: [Bucket; 56],
    /// 4 stash buckets for overflow
    stash: [Bucket; 4],
    /// Reference count for concurrency
    refcount: AtomicU32,
}

/// Bucket with 14 slots
struct Bucket {
    slots: [Slot; 14],
}

/// Slot holding one key-value pair
struct Slot {
    /// Lower 16 bits of hash for quick lookup
    hash_prefix: u16,
    /// Key pointer (or inline for small keys)
    key: KeyStorage,
    /// Value pointer
    value: ValuePtr,
    /// Metadata: TTL flag, type, etc.
    metadata: u16,
}

/// Inline storage for small keys (< 24 bytes)
enum KeyStorage {
    Inline([u8; 24]),
    Heap(Box<[u8]>),
}
```

### Insertion Implementation

```rust
impl Dashtable {
    pub fn insert(&mut self, key: &str, value: &[u8]) -> InsertResult {
        let hash = self.hash(key);
        let (seg_idx, bucket_idx) = self.locate(hash);

        let segment = &mut self.directory[seg_idx];

        // Try home bucket first
        if let Some(slot) = segment.find_empty_slot(bucket_idx) {
            slot.write(hash, key, value);
            self.size += 1;
            return InsertResult::Inserted;
        }

        // Try neighbor bucket
        let neighbor = (bucket_idx + 1) % 56;
        if let Some(slot) = segment.find_empty_slot(neighbor) {
            slot.write(hash, key, value);
            self.size += 1;
            return InsertResult::Inserted;
        }

        // Try stash buckets
        for stash_idx in 0..4 {
            if let Some(slot) = segment.stash[stash_idx].first_empty() {
                slot.write(hash, key, value);
                self.size += 1;
                return InsertResult::Inserted;
            }
        }

        // Segment full - need to split
        self.split_segment(seg_idx);
        self.insert(key, value)  // Retry
    }

    fn split_segment(&mut self, seg_idx: usize) {
        let old_seg = &self.directory[seg_idx];

        // Create new segment
        let mut new_seg = Segment::new();

        // Determine which items to move based on hash bit
        let split_bit = self.log_size;

        for bucket_idx in 0..56 {
            for slot_idx in 0..14 {
                let slot = &old_seg.regular[bucket_idx].slots[slot_idx];
                if slot.is_occupied() {
                    let should_move = (slot.hash_prefix >> split_bit) & 1 == 1;
                    if should_move {
                        new_seg.insert_from_slot(slot);
                        slot.clear();
                    }
                }
            }
        }

        // Add new segment to directory
        self.directory.insert(seg_idx + 1, new_seg);
        self.log_size += 1;
    }
}
```

### Memory-Efficient Slot Design

```rust
/// Tagged pointer using upper bits
#[derive(Clone, Copy)]
struct TaggedPtr(u64);

impl TaggedPtr {
    /// Bit 53: 0 = data pointer, 1 = link pointer
    const LINK_BIT: u64 = 1 << 53;

    /// Bit 54: Item is displaced from home bucket
    const DISPLACED_BIT: u64 = 1 << 54;

    /// Bit 55: Displacement direction (0=left, 1=right)
    const DIRECTION_BIT: u64 = 1 << 55;

    fn data(ptr: *const u8) -> Self {
        Self(ptr as u64)
    }

    fn link(ptr: *const LinkEntry) -> Self {
        Self((ptr as u64) | Self::LINK_BIT)
    }

    fn displaced(ptr: *const u8, direction: bool) -> Self {
        let mut tagged = Self(ptr as u64);
        tagged.0 |= Self::DISPLACED_BIT;
        if direction {
            tagged.0 |= Self::DIRECTION_BIT;
        }
        tagged
    }

    fn ptr(&self) -> *const u8 {
        // Mask off top 12 bits
        (self.0 & ((1 << 53) - 1)) as *const u8
    }

    fn is_link(&self) -> bool {
        self.0 & Self::LINK_BIT != 0
    }

    fn is_displaced(&self) -> bool {
        self.0 & Self::DISPLACED_BIT != 0
    }
}

/// DenseSet entry using tagged pointers
struct DenseSetEntry {
    /// Either points to data or next link
    ptr: TaggedPtr,
}

enum DenseSetEntryType<'a> {
    Data(&'a [u8]),
    Link(&'a DenseSetEntry),
}

impl DenseSetEntry {
    fn as_type(&self) -> DenseSetEntryType<'_> {
        if self.ptr.is_link() {
            // Follow chain
            let link = unsafe { &*(self.ptr.ptr() as *const DenseSetEntry) };
            DenseSetEntryType::Link(link)
        } else {
            // Direct data
            let data = unsafe { std::slice::from_raw_parts(self.ptr.ptr(), /* length from header */) };
            DenseSetEntryType::Data(data)
        }
    }
}
```

## Part 3: Transaction Coordinator

### Valtron Task Pattern

```rust
/// Coordinates multi-shard transactions
pub struct TransactionCoordinator {
    /// Transaction state machine
    state: CoordinatorState,
    /// Effects pending execution
    pending_effects: VecDeque<ShardEffect>,
    /// Results from shards
    results: HashMap<ShardId, ShardResult>,
    /// Final result when complete
    final_result: Option<TransactionResult>,
}

enum CoordinatorState {
    Init,
    Scheduling {
        sequence_num: u64,
        shards_pending: BitSet,
        retry_count: u32,
    },
    Executing {
        current_hop: usize,
        shards_pending: BitSet,
    },
    Complete,
}

enum ShardEffect {
    /// Schedule transaction on shard
    Schedule {
        shard: ShardId,
        txn_id: u64,
        sequence: u64,
        keys: Vec<String>,
    },
    /// Execute micro-operation
    Execute {
        shard: ShardId,
        op: MicroOp,
        is_last_hop: bool,
    },
    /// Finish transaction (release locks)
    Finish {
        shard: ShardId,
        txn_id: u64,
    },
}

impl Task for TransactionCoordinator {
    type Output = TransactionResult;
    type Effect = ShardEffect;

    fn next(&mut self) -> TaskResult<Self::Output, Self::Effect> {
        match &mut self.state {
            CoordinatorState::Init => {
                // Allocate global sequence number
                let seq = GLOBAL_SEQUENCE.fetch_add(1, Ordering::SeqCst);

                self.state = CoordinatorState::Scheduling {
                    sequence_num: seq,
                    shards_pending: BitSet::new(),
                    retry_count: 0,
                };

                // Queue schedule effects for all involved shards
                for shard in self.involved_shards() {
                    self.pending_effects.push_back(ShardEffect::Schedule {
                        shard,
                        txn_id: self.txn_id,
                        sequence: seq,
                        keys: self.keys_for_shard(shard),
                    });
                }

                TaskResult::Continue
            }

            CoordinatorState::Scheduling { sequence_num, shards_pending, retry_count } => {
                // Process schedule acknowledgments
                if let Some(effect) = self.pending_effects.pop_front() {
                    return TaskResult::Effect(effect);
                }

                // Check if all shards acknowledged
                if shards_pending.is_empty() {
                    // Move to execution phase
                    self.state = CoordinatorState::Executing {
                        current_hop: 0,
                        shards_pending: BitSet::new(),
                    };

                    // Queue execute effects for first hop
                    let op = &self.micro_ops[0];
                    for shard in op.involved_shards() {
                        self.pending_effects.push_back(ShardEffect::Execute {
                            shard,
                            op: op.clone(),
                            is_last_hop: self.micro_ops.len() == 1,
                        });
                    }

                    return TaskResult::Continue;
                }

                // Handle conflicts (retry with new sequence number)
                if *retry_count > MAX_RETRIES {
                    self.state = CoordinatorState::Complete;
                    self.final_result = Some(TransactionResult::Aborted);
                    return TaskResult::Complete(TransactionResult::Aborted);
                }

                // Retry scheduling
                *retry_count += 1;
                let new_seq = GLOBAL_SEQUENCE.fetch_add(1, Ordering::SeqCst);
                *sequence_num = new_seq;

                for shard in self.involved_shards() {
                    self.pending_effects.push_back(ShardEffect::Schedule {
                        shard,
                        txn_id: self.txn_id,
                        sequence: new_seq,
                        keys: self.keys_for_shard(shard),
                    });
                }

                TaskResult::Continue
            }

            CoordinatorState::Executing { current_hop, shards_pending } => {
                // Process execute acknowledgments
                if let Some(effect) = self.pending_effects.pop_front() {
                    return TaskResult::Effect(effect);
                }

                if shards_pending.is_empty() {
                    *current_hop += 1;

                    if *current_hop >= self.micro_ops.len() {
                        // All hops complete - finish transaction
                        self.state = CoordinatorState::Complete;
                        self.final_result = Some(self.combine_results());

                        // Queue finish effects
                        for shard in self.involved_shards() {
                            self.pending_effects.push_back(ShardEffect::Finish {
                                shard,
                                txn_id: self.txn_id,
                            });
                        }

                        return TaskResult::Continue;
                    }

                    // Queue execute effects for next hop
                    let op = &self.micro_ops[*current_hop];
                    for shard in op.involved_shards() {
                        self.pending_effects.push_back(ShardEffect::Execute {
                            shard,
                            op: op.clone(),
                            is_last_hop: *current_hop == self.micro_ops.len() - 1,
                        });
                    }
                }

                TaskResult::Continue
            }

            CoordinatorState::Complete => {
                TaskResult::Complete(self.final_result.take().unwrap())
            }
        }
    }
}
```

### Shard Message Handler

```rust
/// Handles transaction messages on shard thread
pub struct ShardHandler {
    /// Pending transactions in sequence order
    tx_queue: VecDeque<ScheduledTransaction>,
    /// Intent locks for conflict detection
    intent_locks: HashMap<String, u32>,
    /// Current epoch for versioning
    epoch: u64,
    /// Storage engine
    storage: Dashtable,
}

struct ScheduledTransaction {
    txn_id: u64,
    sequence_num: u64,
    keys: Vec<String>,
    state: TransactionPhase,
}

enum TransactionPhase {
    Scheduled,
    Executing { current_hop: usize },
    Finished,
}

impl ShardHandler {
    pub fn handle_message(&mut self, msg: ShardMessage) -> ShardResponse {
        match msg {
            ShardMessage::Schedule { txn_id, sequence_num, keys } => {
                self.handle_schedule(txn_id, sequence_num, keys)
            }
            ShardMessage::Execute { txn_id, op, is_last_hop } => {
                self.handle_execute(txn_id, op, is_last_hop)
            }
            ShardMessage::Finish { txn_id } => {
                self.handle_finish(txn_id)
            }
        }
    }

    fn handle_schedule(
        &mut self,
        txn_id: u64,
        sequence_num: u64,
        keys: Vec<String>,
    ) -> ShardResponse {
        // Check for conflicts with earlier transactions
        for key in &keys {
            if self.intent_locks.get(key).map_or(false, |&c| c > 0) {
                // Conflict detected
                return ShardResponse::Conflict;
            }
        }

        // No conflicts - add to queue in order
        self.insert_in_order(ScheduledTransaction {
            txn_id,
            sequence_num,
            keys: keys.clone(),
            state: TransactionPhase::Scheduled,
        });

        // Acquire intent locks
        for key in keys {
            *self.intent_locks.entry(key).or_insert(0) += 1;
        }

        ShardResponse::Ack
    }

    fn handle_execute(
        &mut self,
        txn_id: u64,
        op: MicroOp,
        is_last_hop: bool,
    ) -> ShardResponse {
        // Find transaction in queue
        let txn = match self.tx_queue.iter_mut().find(|t| t.txn_id == txn_id) {
            Some(t) => t,
            None => return ShardResponse::NotFound,
        };

        // Execute micro-operation
        let result = op.execute(&mut self.storage);

        if is_last_hop {
            txn.state = TransactionPhase::Finished;
        }

        ShardResponse::Ok(result)
    }

    fn handle_finish(&mut self, txn_id: u64) -> ShardResponse {
        // Find and remove transaction
        if let Some(pos) = self.tx_queue.iter().position(|t| t.txn_id == txn_id) {
            let txn = self.tx_queue.remove(pos).unwrap();

            // Release intent locks
            for key in &txn.keys {
                let count = self.intent_locks.get_mut(key).unwrap();
                *count -= 1;
                if *count == 0 {
                    self.intent_locks.remove(key);
                }
            }

            ShardResponse::Ack
        } else {
            ShardResponse::NotFound
        }
    }

    fn insert_in_order(&mut self, txn: ScheduledTransaction) {
        // Insert in sequence number order
        let pos = self.tx_queue
            .iter()
            .position(|t| t.sequence_num > txn.sequence_num)
            .unwrap_or(self.tx_queue.len());
        self.tx_queue.insert(pos, txn);
    }
}
```

## Part 4: Journal and Replication

### Journal Implementation

```rust
/// Write-ahead journal for replication
pub struct Journal {
    /// Journal entries buffer
    entries: Vec<JournalEntry>,
    /// Current LSN
    current_lsn: u64,
    /// Writer for persistence
    writer: Option<JournalWriter>,
}

struct JournalEntry {
    lsn: u64,
    timestamp: u64,
    shard_id: ShardId,
    operation: JournalOp,
    crc32: u32,
}

enum JournalOp {
    Insert { key: String, value: Vec<u8> },
    Delete { key: String },
    Update { key: String, value: Vec<u8> },
    Expire { key: String, expiry_ms: u64 },
    Multi { tx_id: u64, ops: Vec<(ShardId, JournalOp)> },
}

impl Journal {
    pub fn record(&mut self, op: JournalOp) {
        self.current_lsn += 1;

        let entry = JournalEntry {
            lsn: self.current_lsn,
            timestamp: current_timestamp_ms(),
            shard_id: self.shard_id,
            operation: op,
            crc32: 0,  // Calculate after serialization
        };

        self.entries.push(entry);

        // Flush if buffer is full
        if self.entries.len() >= FLUSH_THRESHOLD {
            self.flush();
        }
    }

    pub fn stream_entries(&self, from_lsn: u64) -> impl Iterator<Item = &JournalEntry> {
        self.entries.iter().filter(move |e| e.lsn > from_lsn)
    }

    fn flush(&mut self) {
        if let Some(writer) = &mut self.writer {
            for entry in &self.entries {
                writer.write(entry).expect("Journal write failed");
            }
            writer.sync().expect("Journal sync failed");
        }
        self.entries.clear();
    }
}
```

### Replica Task

```rust
/// Replication task for Valtron executor
pub struct ReplicaTask {
    state: ReplicaState,
    config: ReplicaConfig,
    /// Received RDB data
    rdb_buffer: Vec<u8>,
    /// Pending journal entries
    journal_buffer: Vec<JournalEntry>,
    /// Local shards for applying data
    local_shards: Vec<Shard>,
}

enum ReplicaState {
    Init,
    Connecting,
    Handshaking,
    ReceivingRdb { received: usize, total: usize },
    LoadingRdb { shard_idx: usize },
    CatchingUp { current_lsn: u64 },
    Stable { last_ack: Instant },
    Error { retry_after: Duration },
}

impl Task for ReplicaTask {
    type Output = ReplicaStats;
    type Effect = ReplicaEffect;

    fn next(&mut self) -> TaskResult<Self::Output, Self::Effect> {
        match &mut self.state {
            ReplicaState::Init => {
                self.state = ReplicaState::Connecting;
                TaskResult::Effect(ReplicaEffect::Connect {
                    host: self.config.master_host.clone(),
                    port: self.config.master_port,
                })
            }

            ReplicaState::Connecting => {
                self.state = ReplicaState::Handshaking;
                TaskResult::Effect(ReplicaEffect::SendHandshake)
            }

            ReplicaState::Handshaking => {
                // Wait for handshake response
                self.state = ReplicaState::ReceivingRdb {
                    received: 0,
                    total: self.config.num_shards,
                };
                TaskResult::Effect(ReplicaEffect::StartRdbTransfer)
            }

            ReplicaState::ReceivingRdb { received, total } => {
                if *received >= *total {
                    // RDB complete, start loading
                    self.state = ReplicaState::LoadingRdb { shard_idx: 0 };
                    return TaskResult::Continue;
                }

                // Receive next RDB chunk
                TaskResult::Effect(ReplicaEffect::ReceiveRdbChunk)
            }

            ReplicaState::LoadingRdb { shard_idx } => {
                if *shard_idx >= self.config.num_shards {
                    // All shards loaded, start catching up
                    self.state = ReplicaState::CatchingUp { current_lsn: 0 };
                    return TaskResult::Continue;
                }

                // Load RDB into shard
                let rdb_data = self.extract_rdb_for_shard(*shard_idx);
                self.local_shards[*shard_idx].load_rdb(rdb_data);
                *shard_idx += 1;

                TaskResult::Continue
            }

            ReplicaState::CatchingUp { current_lsn } => {
                // Process journal entries
                if let Some(entry) = self.journal_buffer.pop() {
                    self.apply_journal_entry(entry);
                    *current_lsn = entry.lsn;
                    return TaskResult::Continue;
                }

                // Check if caught up
                if *current_lsn >= self.config.master_lsn {
                    self.state = ReplicaState::Stable {
                        last_ack: Instant::now(),
                    };
                    return TaskResult::Continue;
                }

                // Wait for more journal entries
                TaskResult::Effect(ReplicaEffect::ReceiveJournal)
            }

            ReplicaState::Stable { last_ack } => {
                // Send periodic ACK
                if last_ack.elapsed() > ACK_INTERVAL {
                    *last_ack = Instant::now();
                    TaskResult::Effect(ReplicaEffect::SendAck {
                        lsn: self.current_lsn,
                    })
                } else {
                    // Process any new journal entries
                    TaskResult::Effect(ReplicaEffect::ReceiveJournal)
                }
            }

            ReplicaState::Error { retry_after } => {
                TaskResult::Effect(ReplicaEffect::Sleep(*retry_after))
            }
        }
    }
}
```

## Part 5: Memory Management

### Arena Allocation for Slots

```rust
/// Arena for allocating slot metadata
pub struct SlotArena {
    /// Current block
    current: NonNull<SlotBlock>,
    /// Free list within current block
    free_list: Option<u32>,
    /// Total allocated
    allocated: usize,
}

struct SlotBlock {
    /// Fixed-size slot array
    slots: [Slot; BLOCK_SIZE],
    /// Next block in arena
    next: Option<NonNull<SlotBlock>>,
}

impl SlotArena {
    pub fn allocate(&mut self) -> &mut Slot {
        // Check free list first
        if let Some(idx) = self.free_list {
            let slot = &mut self.current.as_mut().slots[idx as usize];
            self.free_list = slot.next_free;
            return slot;
        }

        // Allocate from current block
        let block = self.current.as_mut();
        for (idx, slot) in block.slots.iter_mut().enumerate() {
            if slot.is_free() {
                return slot;
            }
        }

        // Need new block
        self.allocate_block();
        self.allocate()
    }

    fn allocate_block(&mut self) {
        let new_block = Box::new(SlotBlock {
            slots: [Slot::free(); BLOCK_SIZE],
            next: None,
        });

        let new_ptr = Box::leak(new_block) as *mut _ as NonNull<_>;

        // Link to current
        self.current.as_mut().next = Some(new_ptr);
        self.current = new_ptr;
        self.allocated += BLOCK_SIZE;
    }
}
```

### Custom Allocator for DenseSet

```rust
/// Custom allocator for DenseSet entries
struct DenseSetAllocator {
    /// Large allocation pool
    pool: NonNull<u8>,
    /// Pool size
    size: usize,
    /// Current offset
    offset: usize,
}

impl DenseSetAllocator {
    pub fn new(size: usize) -> Self {
        let layout = Layout::from_size_align(size, 8).unwrap();
        let ptr = unsafe { alloc(layout) };
        Self {
            pool: NonNull::new(ptr).unwrap(),
            size,
            offset: 0,
        }
    }

    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();

        // Align offset
        let aligned = (self.offset + align - 1) & !(align - 1);

        if aligned + size > self.size {
            return None;  // Out of memory
        }

        let ptr = unsafe { self.pool.as_ptr().add(aligned) };
        self.offset = aligned + size;

        Some(NonNull::new(ptr).unwrap())
    }
}
```

---

*This document is part of the DragonflyDB exploration series. See [exploration.md](./exploration.md) for the complete index.*
