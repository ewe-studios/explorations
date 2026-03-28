---
title: "Storage Engine Deep Dive: DragonflyDB"
subtitle: "Dashtable, DenseSet, and memory efficiency"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.dragonflydb
related: 00-zero-to-db-engineer.md, exploration.md
---

# 01 - Storage Engine Deep Dive: DragonflyDB

## Overview

This document dives deep into DragonflyDB's storage engine - the data structures that make it 25X faster and 30% more memory efficient than Redis.

## Part 1: The Memory Problem

### Why Traditional Hashtables Waste Memory

Redis uses a classic chained hashtable (Redis Dictionary - RD):

```
Redis Dictionary Structure:

dict {
    dictht ht[2]  // Two tables for incremental resize
}

dictht {
    dictEntry **table  // Array of bucket pointers
    unsigned long size // Table size
    unsigned long used // Items count
}

dictEntry {
    dictEntry *next  // 8 bytes - chain pointer
    void *key        // 8 bytes - key pointer
    union {          // 8 bytes - value pointer
        void *v
        long val
        double dval
    }
}
```

### Memory Overhead Calculation

For N items at different load factors:

```
Case 1: 100% Load Factor (N items, N buckets)
┌───────────────────────────────────────┐
│ Bucket array: N * 8 bytes = 8N        │
│ dictEntry:    N * 24 bytes = 24N      │
├───────────────────────────────────────┤
│ Total: 32N bytes = 32 bytes/record    │
└───────────────────────────────────────┘

Case 2: 75% Load Factor (N items, 1.33N buckets)
┌───────────────────────────────────────┐
│ Bucket array: 1.33N * 8 = 10.64N      │
│ dictEntry:    N * 24 = 24N            │
├───────────────────────────────────────┤
│ Total: ~34.6N = 35 bytes/record       │
└───────────────────────────────────────┘

Case 3: 50% Load Factor (N items, 2N buckets)
┌───────────────────────────────────────┐
│ Bucket array: 2N * 8 = 16N            │
│ dictEntry:    N * 24 = 24N            │
├───────────────────────────────────────┤
│ Total: 40N = 40 bytes/record          │
└───────────────────────────────────────┘

Optimal storage: 16 bytes (two 8-byte pointers)
Redis overhead: 16-24 bytes per record
```

### Incremental Resize Memory Spike

When Redis grows its hashtable:

```
Before Resize (N items, fully utilized):
ht[0]: N buckets, N entries (32N bytes)
ht[1]: empty

During Resize:
ht[0]: N buckets, N entries (32N bytes)
ht[1]: 2N buckets (16N bytes allocated)

Total during resize: 48N bytes
Memory spike: 50% increase!

For 5GB dataset: 5GB -> 7.5GB spike
```

## Part 2: Dashtable Design

### The Dashtable Insight

Dashtable is based on the paper ["Dash: Scalable Hashing on Persistent Memory"](https://arxiv.org/abs/2003.07302). It solves Redis's problems with two key innovations:

1. **Segmented Architecture** - Independent growth units
2. **Open Addressing** - No chain pointers needed

```
Dashtable Structure:

Dashtable {
    Segment** directory  // Array of segment pointers
    size_t lognum_segments

    // Each segment is a mini-hashtable
}

Segment {
    Bucket[56] regular_buckets   // 56 regular buckets
    Bucket[4] stash_buckets      // 4 overflow buckets
}

Bucket {
    Slot[14] slots  // Each slot holds one key-value
}

Total capacity per segment: 60 * 14 = 840 records
```

### Visual Structure

```
Dashtable Directory:
┌─────────┬─────────┬─────────┬─────────┐
│   Seg 0 │   Seg 1 │   Seg 2 │   Seg 3 │ ...
└────┬────┴────┬────┴────┬────┴────┬────┘
     │         │         │         │
     ▼         ▼         ▼         ▼
┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐
│ Segment │ │ Segment │ │ Segment │ │ Segment │
│ ┌─────┐ │ │ ┌─────┐ │ │ ┌─────┐ │ │ ┌─────┐ │
│ │ B0  │ │ │ │ B0  │ │ │ │ B0  │ │ │ │ B0  │ │
│ │ B1  │ │ │ │ B1  │ │ │ │ B1  │ │ │ │ B1  │ │
│ │ ... │ │ │ │ ... │ │ │ │ ... │ │ │ │ ... │ │
│ │ B55 │ │ │ │ B55 │ │ │ │ B55 │ │ │ │ B55 │ │
│ │Stash│ │ │ │Stash│ │ │ │Stash│ │ │ │Stash│ │
│ └─────┘ │ │ └─────┘ │ │ └─────┘ │ │ └─────┘ │
└─────────┘ └─────────┘ └─────────┘ └─────────┘
```

### Bucket and Slot Layout

```
Segment Internal Structure:

Regular Buckets (0-55):
┌────────────────────────────────────────────┐
│ Bucket 0: [Slot0][Slot1]...[Slot13]        │
│ Bucket 1: [Slot0][Slot1]...[Slot13]        │
│ ...                                        │
│ Bucket 55: [Slot0][Slot1]...[Slot13]       │
├────────────────────────────────────────────┤
│ Stash Buckets (0-3):                       │
│ Stash 0: [Slot0][Slot1]...[Slot13]         │
│ ...                                        │
│ Stash 3: [Slot0][Slot1]...[Slot13]         │
└────────────────────────────────────────────┘

Each Slot:
┌────────────────────────────────────────────┐
│ hash_prefix (16 bits) │ key_ptr (48 bits)  │  <- 8 bytes
│ value_ptr (64 bits)                       │  <- 8 bytes
│ metadata (16 bits)                        │  <- 2 bytes
├────────────────────────────────────────────┤
│ Total per slot: ~18-20 bytes               │
│ Metadata includes: expiry, flags, type     │
└────────────────────────────────────────────┘
```

## Part 3: Insertion Algorithm

### Step-by-Step Insertion

```
Insert(key, value) into Dashtable:

1. Compute hash: h = hash(key)
2. Determine home segment: seg_idx = h >> (hash_bits - log_segments)
3. Determine home bucket: bucket_idx = h % num_buckets_in_segment
4. Try to insert into home bucket slots

Pseudocode:
fn insert(key, value):
    h = hash(key)
    seg = directory[h >> shift]
    home_bucket = h % 56

    // Try home bucket first
    for slot in seg.buckets[home_bucket].slots:
        if slot.empty() or slot.key == key:
            slot.write(key, value)
            return SUCCESS

    // Try neighbor bucket (right)
    neighbor = (home_bucket + 1) % 56
    for slot in seg.buckets[neighbor].slots:
        if slot.empty() or slot.key == key:
            slot.write(key, value)
            return SUCCESS

    // Try stash buckets
    for stash_idx in 0..4:
        for slot in seg.stash[stash_idx].slots:
            if slot.empty() or slot.key == key:
                slot.write(key, value)
                return SUCCESS

    // Segment is full - trigger split
    split_segment(seg_idx)
    return insert(key, value)  // Retry
```

### Segment Split Process

```
When a segment fills up:

Before Split:
Directory: [Seg0][Seg1][Seg2][Seg3]
Seg1 is full (840 items)

Split Process:
1. Allocate new segment Seg_new
2. Add to directory: [Seg0][Seg1][Seg2][Seg3][Seg_new]
3. Redistribute items from Seg1:
   - Items with bit N = 0 stay in Seg1
   - Items with bit N = 1 move to Seg_new
4. Update segment count

After Split:
Directory: [Seg0][Seg1][Seg2][Seg3][Seg_new]
Seg1: ~420 items
Seg_new: ~420 items

Only 2 segments touched - O(1) operation
```

### Memory Overhead Calculation

```
Dashtable memory for N items at 100% utilization:

Directory: N/840 segments * 8 bytes = 0.0095N bytes
Slots: N items * 16 bytes (key+value pointers) = 16N bytes
Metadata: N items * 2.5 bytes (hash, flags, expiry) = 2.5N bytes

Total: 16N + 2.5N + 0.01N ≈ 18.5N bytes
Overhead: ~2.5 bytes per record

Compare to Redis: 32 bytes/record
Dragonfly savings: ~13.5 bytes/record (42% reduction)

For 1 million keys:
- Redis: 32MB overhead
- Dragonfly: 2.5MB overhead
- Savings: 29.5MB
```

## Part 4: DenseSet for Sets

### The Set Problem

Redis Sets use dictEntry for each member:

```
Redis Set with 3 members:
{member1, member2, member3}

Memory layout:
dictEntry1 -> dictEntry2 -> dictEntry3
    │            │            │
    ▼            ▼            ▼
 member1      member2      member3

Each dictEntry: 24 bytes
Plus bucket chain: 8 bytes per entry
Total: 32 bytes per set member
```

### DenseSet Innovation

DenseSet uses **pointer tagging** to eliminate dictEntry allocations:

```
Pointer Tagging Concept:

64-bit pointer layout (x86-64 userspace):
┌────────────────────────────────────────────┐
│ Unused (12 bits) │ Address (52 bits)       │
│     bits 52-63   │      bits 0-51          │
└────────────────────────────────────────────┘

Userspace uses only 48 bits of addressing
Top 16 bits are always zero

Dragonfly uses bits 53-55 for metadata:
┌────────────────────────────────────────────┐
│Unused│Dir│Disp│Link│  Address (53 bits)   │
│56-63 │55 │ 54 │ 53 │       0-52           │
└────────────────────────────────────────────┘

Bit 53 (Link): 0 = points to data, 1 = points to next link
Bit 54 (Displaced): 1 = item is not in home bucket
Bit 55 (Direction): 0 = displaced left, 1 = displaced right
```

### DenseSet Structure

```
DenseSet Bucket Array:
┌─────────┬─────────┬─────────┬─────────┐
│ Bucket0 │ Bucket1 │ Bucket2 │ Bucket3 │ ...
└────┬────┴────┬────┴────┬────┴────┬────┘
     │         │         │         │
     ▼         ▼         ▼         ▼
  ┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐
  │Link0│   │Link1│   │Link2│   │Link3│
  └──┬──┘   └──┬──┘   └──┬──┘   └──┬──┘
     │         │         │         │
     ▼         ▼         ▼         ▼
  ┌─────┐   ┌─────┐   ┌─────┐   ┌─────┐
  │Link1│   │Data │   │Link3│   │Data │
  └──┬──┘   └─────┘   └──┬──┘   └─────┘
     │                   │
     ▼                   ▼
  ┌─────┐             ┌─────┐
  │Data │             │Data │
  └─────┘             └─────┘

Link entry: Points to next entry in chain
Data entry: Points directly to set member object

No separate dictEntry allocation needed!
```

### DenseSet Insertion

```
fn dense_set_insert(key):
    h = hash(key)

    // Check home bucket and neighbors (±1)
    for offset in [-1, 0, 1]:
        bucket = (h + offset) % num_buckets
        entry = buckets[bucket]

        if entry == null:
            // Empty bucket - insert directly
            buckets[bucket] = tag_pointer(key, LINK=0)
            return SUCCESS

        if entry.key == key:
            // Already exists
            return EXISTS

    // All buckets occupied - insert into chain
    home_bucket = h % num_buckets
    entry = buckets[home_bucket]

    // Traverse chain, insert at end
    while entry.link != null:
        if entry.key == key:
            return EXISTS
        entry = entry.link

    entry.link = tag_pointer(key, LINK=1)
    return SUCCESS
```

### Neighbor Cell Optimization

```
DenseSet reduces collisions with neighbor cells:

Standard chaining:
- Item hashes to bucket N
- If N is full, chain off N

DenseSet optimization:
- Item hashes to bucket N
- Check N, N-1, N+1 for empty slot
- Insert into first empty neighbor
- Mark as "displaced" with pointer tagging

Benefits:
- Reduces chain length
- Better cache locality
- Fewer allocations

Example:
Hash("apple") = bucket 42
Bucket 42 is full
Check bucket 41 - empty!
Insert into 41, mark as displaced-right

Later lookup:
- Check 41 (displaced), 42 (home), 43 (neighbor)
- Find "apple" in 41
```

### DenseSet Memory Savings

```
Redis Set (100 members):
dictEntry: 100 * 24 = 2400 bytes
Bucket chain: 100 * 8 = 800 bytes
Total: 3200 bytes = 32 bytes/member

DenseSet (100 members):
Bucket array: ~120 * 8 = 960 bytes (at 80% utilization)
Object pointers: 100 * 8 = 800 bytes
Link entries: ~20 * 8 = 160 bytes (for chains)
Total: ~1920 bytes = ~19 bytes/member

Savings: 13 bytes/member (40% reduction)

For 1 million set members:
- Redis: 32MB
- DenseSet: 19MB
- Savings: 13MB
```

## Part 5: Expiry Tracking

### TTL Storage Structure

Dragonfly stores expiry metadata in dashtable slots:

```
Slot Metadata (16 bits):
┌────────────────────────────────────────────┐
│ Expired (1) │ HasTTL (1) │ Reserved (14)  │
│    bit 15   │   bit 14   │   bits 0-13    │
└────────────────────────────────────────────┘

Separate expiry dashtable for TTL values:
┌────────────────────────────────────────────┐
│ key: "session:123" │ expiry: 1711670400000 │
│ key: "cache:abc"   │ expiry: 1711674000000 │
└────────────────────────────────────────────┘

Expiry dashtable:
- Only stores keys with TTL
- Sorted by expiry time internally
- Efficient range queries for cleanup
```

### Passive Expiration

```
On key access:

fn get(key):
    slot = lookup(key)

    if slot.has_ttl():
        expiry = get_expiry(key)
        if now() >= expiry:
            delete(key)  // Expired
            return NIL

    return slot.value

Zero CPU overhead for:
- Keys without TTL
- Keys not being accessed
```

### Proactive Expiration

```
Background garbage collection:

fn expiry_tick():
    // Run at configured frequency (default: 100 Hz)
    for each segment in dashtable:
        if segment.needs_split():
            // Split triggers GC scan
            scan_and_delete_expired(segment)

        // Gradual background scan
        if random() < 0.01:
            scan_segment_for_expired(segment)

fn scan_and_delete_expired(segment):
    for bucket in segment.buckets:
        for slot in bucket.slots:
            if slot.has_ttl() and slot.expired():
                slot.clear()
                segment.free_count++
```

### Expiry During Writes

```
Insert with expiry triggers GC:

fn setex(key, value, ttl_ms):
    segment = get_segment(key)

    if segment.is_near_capacity():
        // Garbage collect before growing
        deleted = scan_and_delete_expired(segment)

        if deleted > 0:
            // Maybe no longer need to split!
            if segment.has_space():
                insert_without_split(key, value, ttl_ms)
                return

    // Normal insert with split if needed
    insert_with_possible_split(key, value, ttl_ms)

Benefit: Expiry prevents unnecessary table growth
```

## Part 6: Benchmarks and Real-World Numbers

### Memory Efficiency Comparison

```
Test: debug populate 20000000 key 10 (20M keys, 10-byte values)

Single-threaded:
┌──────────────┬─────────────┬──────────────┐
│ Server       │ Time        │ Memory Used  │
├──────────────┼─────────────┼──────────────┤
│ Dragonfly    │ 10.8s       │ 1.0 GB       │
│ Redis 6      │ 16.0s       │ 1.73 GB      │
└──────────────┴─────────────┴──────────────┘

Redis overhead breakdown:
- used_memory_overhead: 1.0 GB
- Actual data: ~0.73 GB
- 58% of memory is overhead!

Dragonfly overhead:
- Estimated overhead: ~0.2 GB
- Actual data: ~0.8 GB
- 20% overhead

Multi-threaded (8 cores):
┌──────────────┬─────────────┬──────────────┐
│ Server       │ Time        │ Memory Used  │
├──────────────┼─────────────┼──────────────┤
│ Dragonfly    │ 2.43s       │ 896 MB       │
│ Redis 6      │ 16.0s       │ 1.73 GB      │
└──────────────┴─────────────┴──────────────┘

Dragonfly is 7X faster and uses 48% less memory
```

### BGSAVE Memory Spike

```
Test: 5GB dataset, update traffic during snapshot

Peak memory usage:
┌──────────────┬─────────────┬──────────────┐
│ Phase        │ Redis       │ Dragonfly    │
├──────────────┼─────────────┼──────────────┤
│ Before BGSAVE│ 5.86 GB     │ 4.2 GB       │
│ During       │ 12.5 GB     │ 4.3 GB       │
│ After        │ 5.86 GB     │ 4.2 GB       │
└──────────────┴─────────────┴──────────────┘

Redis CoW spike: 2.1X increase
Dragonfly: No visible spike

Why Dragonfly wins:
- No fork() needed
- Dashtable supports lock-free iteration
- Point-in-time snapshot without CoW
```

### Expiry Efficiency

```
Test: memtier_benchmark with 30-second TTL

Command:
memtier_benchmark --ratio 1:0 -n 600000 --threads=2 \
  --expiry-range=30-30 -d 256

Results:
┌──────────────┬─────────────┬──────────────┐
│ Server       │ Peak Memory │ SET QPS      │
├──────────────┼─────────────┼──────────────┤
│ Dragonfly    │ 1.45 GB     │ 131K         │
│ Redis 6      │ 1.95 GB     │ 100K         │
└──────────────┴─────────────┴──────────────┘

Working set size (30s window):
- Dragonfly: 131K * 30 = 3.93M items
- Redis: 100K * 30 = 3.0M items

Dragonfly holds 30% more items in 25% less memory
```

## Part 7: Implementation Details

### Dashtable in Rust (Valtron-style)

```rust
/// Dashtable segment with fixed-size buckets
struct Segment {
    /// 56 regular buckets
    regular: [Bucket; 56],
    /// 4 stash buckets for overflow
    stash: [Bucket; 4],
    /// Count of items in segment
    size: u32,
}

struct Bucket {
    /// 14 slots per bucket
    slots: [Slot; 14],
}

struct Slot {
    /// Lower 48 bits of hash for quick lookup
    hash_prefix: u16,
    /// Pointer to key (tagged with metadata)
    key_ptr: TaggedPtr,
    /// Pointer to value
    value_ptr: NonNull<Value>,
    /// Metadata: expiry flag, key type, etc.
    metadata: u16,
}

/// Tagged pointer using upper bits for metadata
struct TaggedPtr(u64);

impl TaggedPtr {
    const LINK_BIT: u64 = 1 << 53;
    const DISPLACED_BIT: u64 = 1 << 54;
    const DIRECTION_BIT: u64 = 1 << 55;

    fn new(ptr: *const u8) -> Self {
        Self(ptr as u64)
    }

    fn is_link(&self) -> bool {
        self.0 & Self::LINK_BIT != 0
    }

    fn is_displaced(&self) -> bool {
        self.0 & Self::DISPLACED_BIT != 0
    }

    fn ptr(&self) -> *const u8 {
        // Mask off top 12 bits
        (self.0 & ((1 << 53) - 1)) as *const u8
    }
}
```

### DenseSet in Rust

```rust
/// DenseSet using pointer tagging
struct DenseSet {
    /// Bucket array - pointers are either data or links
    buckets: Vec<TaggedPtr>,
    /// Number of items
    size: usize,
    /// Load factor threshold for growth
    load_factor: f64,
}

impl DenseSet {
    fn contains(&self, key: &str) -> bool {
        let hash = self.hash(key);
        let bucket_idx = hash % self.buckets.len();

        // Check home bucket and neighbors
        for offset in [-1i32, 0, 1] {
            let idx = ((bucket_idx as i32 + offset + self.buckets.len() as i32)
                       % self.buckets.len() as i32) as usize;

            let entry = self.buckets[idx];
            if entry.is_null() {
                continue;
            }

            if !entry.is_link() {
                // Direct data pointer
                if self.key_matches(entry.ptr(), key) {
                    return true;
                }
            } else {
                // Traverse chain
                let mut current = entry;
                while current.is_link() {
                    if self.key_matches(current.ptr(), key) {
                        return true;
                    }
                    current = *current.ptr() as TaggedPtr;
                }
                // Last link points to data
                if self.key_matches(current.ptr(), key) {
                    return true;
                }
            }
        }

        false
    }

    fn insert(&mut self, key: String) -> bool {
        let hash = self.hash(&key);
        let bucket_idx = hash % self.buckets.len();

        // Try home bucket first
        if self.buckets[bucket_idx].is_null() {
            self.buckets[bucket_idx] = TaggedPtr::data(key);
            self.size += 1;
            return true;
        }

        // Insert into chain
        self.insert_into_chain(bucket_idx, key)
    }
}
```

---

*This document is part of the DragonflyDB exploration series. See [exploration.md](./exploration.md) for the complete index.*
