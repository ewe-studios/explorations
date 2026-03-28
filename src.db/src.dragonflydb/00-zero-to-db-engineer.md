---
title: "Zero to DB Engineer: DragonflyDB"
subtitle: "Understanding modern in-memory datastores"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.dragonflydb
related: exploration.md
---

# 00 - Zero to DB Engineer: DragonflyDB

## Overview

This document explains the fundamentals of DragonflyDB - a modern, multi-threaded in-memory datastore that achieves 25X more throughput than Redis while maintaining full API compatibility.

## Part 1: Why DragonflyDB Exists

### The Redis Problem

Redis was designed in 2009 for a different era of hardware:

```
Single-threaded Architecture (Redis):
┌─────────────────────────────────────┐
│         Single Thread               │
│  ┌─────────────────────────────┐    │
│  │  All commands execute here  │    │
│  │  - GET/SET                  │    │
│  │  - HGET/HSET                │    │
│  │  - LPUSH/RPOP               │    │
│  │  - ...                      │    │
│  └─────────────────────────────┘    │
│                                     │
│  CPU Cores: 1 used, 15 idle         │
│  Throughput: ~200K QPS (capped)     │
└─────────────────────────────────────┘
```

Modern servers have 16, 32, 64+ CPU cores, but Redis can only use **one** core for command processing. This fundamental limitation means:

1. **Wasted CPU resources** - 15 out of 16 cores sit idle
2. **Throughput ceiling** - Single core caps at ~200K QPS
3. **Memory inefficiency** - Cannot leverage NUMA architectures

### The Dragonfly Solution

DragonflyDB was designed in 2022 with modern hardware in mind:

```
Shared-Nothing Architecture (Dragonfly):
┌─────────────────────────────────────────────────────────────┐
│                    Listener Threads                         │
│         (Handle client connections, parse protocol)         │
├─────────────────────────────────────────────────────────────┤
│                    Worker Threads                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐      │
│  │  Shard 0 │ │  Shard 1 │ │  Shard 2 │ │  Shard 3 │      │
│  │ Dashtable│ │ Dashtable│ │ Dashtable│ │ Dashtable│      │
│  │  Keys 0  │ │  Keys 1  │ │  Keys 2  │ │  Keys 3  │      │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘      │
│         │            │            │            │            │
│         └────────────┴────────────┴────────────┘            │
│                    VLL Transaction Layer                    │
│              (Coordinates multi-key operations)             │
├─────────────────────────────────────────────────────────────┤
│                    Memory Engine                            │
│         (DenseSet, Dashtable, efficient allocation)         │
└─────────────────────────────────────────────────────────────┘

Performance on 16-core instance:
- Throughput: 3.8M QPS (vs 200K for Redis)
- Latency P99: <1ms at peak throughput
- Memory: 30% more efficient than Redis
```

### Performance Comparison

| Instance Type | Redis QPS | Dragonfly QPS | Improvement |
|---------------|-----------|---------------|-------------|
| m5.large (1 core) | 194K | 191K | Comparable |
| m5.xlarge (2 cores) | 220K | 305K | 1.4X |
| c6gn.16xlarge (16 cores) | ~200K | 3,844K | **25X** |

Key insight: Dragonfly's algorithmic layer has minimal overhead when running single-threaded, but scales linearly with available cores.

## Part 2: Key-Value Fundamentals

### What is an In-Memory Datastore?

An in-memory datastore stores all data in RAM rather than on disk. This provides:

```
Memory vs Disk Access Times:
┌─────────────────────────────────────────┐
│ Operation          │ Time               │
├─────────────────────────────────────────┤
│ L1 Cache (CPU)     │ ~1 nanosecond      │
│ L2 Cache (CPU)     │ ~4 nanoseconds     │
│ L3 Cache (CPU)     │ ~17 nanoseconds    │
│ RAM (Main Memory)  │ ~100 nanoseconds   │
│ SSD (NVMe)         │ ~100 microseconds  │
│ HDD (Disk)         │ ~10 milliseconds   │
└─────────────────────────────────────────┘

RAM is ~100,000X faster than HDD
RAM is ~1,000X faster than NVMe SSD
```

### Basic Operations

Dragonfly supports Redis-compatible commands:

```
String Operations:
  SET key value [EX seconds] [PX milliseconds]
  GET key
  MSET key1 value1 key2 value2
  MGET key1 key2
  INCR key
  DECR key

List Operations:
  LPUSH key value [value ...]
  RPUSH key value [value ...]
  LPOP key
  RPOP key
  LRANGE key start stop
  BLPOP key [key ...] timeout  # Blocking variant

Set Operations:
  SADD key member [member ...]
  SMEMBERS key
  SISMEMBER key member
  SINTER key [key ...]
  SUNION key [key ...]

Hash Operations:
  HSET key field value [field value ...]
  HGET key field
  HGETALL key
  HDEL key field [field ...]

Sorted Set Operations:
  ZADD key score member [score member ...]
  ZRANGE key start stop [WITHSCORES]
  ZREM key member [member ...]
  ZRANK key member
```

### Example: Building a Session Store

```bash
# Store user session with 1 hour expiry
SET session:user123 '{"user_id": 123, "role": "admin"}' EX 3600

# Retrieve session
GET session:user123

# Update session TTL
EXPIRE session:user123 3600

# Delete session
DEL session:user123

# Check if session exists
EXISTS session:user123
```

### Example: Building a Cache

```bash
# Cache API response with 5 minute TTL
SET api:users:page1 '{"users": [...], "page": 1}' EX 300

# Cache-aside pattern
GET api:users:page1
# If nil, fetch from database and cache:
# SET api:users:page1 <result> EX 300

# Bulk cache operations
MSET api:users:page1 <data1> api:users:page2 <data2> api:users:page3 <data3>
```

## Part 3: Shared-Nothing Architecture Explained

### What is Shared-Nothing?

In a shared-nothing architecture, each thread owns its slice of data and processes requests independently:

```
Shared-Nothing vs Shared-Everything:

Shared-Everything (Traditional):
┌─────────────────────────────────────────┐
│           Global Lock (Mutex)           │
├─────────────────────────────────────────┤
│  Thread 1  │  Thread 2  │  Thread 3     │
│      │          │           │           │
│      └──────────┴───────────┘           │
│              │                          │
│         Shared Memory                   │
│      (Requires locking)                 │
└─────────────────────────────────────────┘

Problem: Lock contention limits scalability


Shared-Nothing (Dragonfly):
┌─────────────────────────────────────────┐
│  Thread 0  │  Thread 1  │  Thread 2     │
│  ┌──────┐  │  ┌──────┐  │  ┌──────┐    │
│  │Shard │  │  │Shard │  │  │Shard │    │
│  │ Data │  │  │ Data │  │  │ Data │    │
│  └──────┘  │  └──────┘  │  └──────┘    │
│     No         No          No           │
│   Locking    Locking     Locking       │
└─────────────────────────────────────────┘

Benefit: Each thread operates independently
```

### How Dragonfly Partitions Keys

Dragonfly uses **consistent hashing** to distribute keys across shards:

```
Key Distribution:

1. Compute hash of the key: hash = CRC16(key) % num_shards

2. Route to shard based on hash:
   - hash = 0 -> Shard 0
   - hash = 1 -> Shard 1
   - hash = 2 -> Shard 2
   - ...

Example:
  Key "user:123" -> hash("user:123") = 42 -> Shard 42
  Key "user:456" -> hash("user:456") = 17 -> Shard 17

Each shard has its own:
- Dashtable (storage)
- Lock manager (transactions)
- Expiry tracker (TTL)
```

### Single-Key Operations

Single-key operations are trivially parallel:

```
Concurrent SET Operations:

Client1: SET user:123 "Alice"    -> Shard 0 (processes independently)
Client2: SET user:456 "Bob"      -> Shard 1 (processes independently)
Client3: SET user:789 "Charlie"  -> Shard 2 (processes independently)

No coordination needed - each shard owns its keys
```

### Multi-Key Operations

Multi-key operations require coordination via VLL (described later):

```
MSET user:123 "Alice" user:456 "Bob":

1. Coordinator identifies shards involved (Shard 0, Shard 1)
2. Sends schedule message to both shards
3. Waits for acknowledgment
4. Sends execute message to both shards
5. Waits for completion
6. Returns response to client
```

## Part 4: ACID Transactions in Memory

### What Does ACID Mean?

```
A - Atomicity: All operations succeed or all fail
C - Consistency: Database remains in valid state
I - Isolation: Concurrent transactions don't interfere
D - Durability: Once committed, data persists
```

### Atomicity Examples

```
Transaction: Transfer $100 from Account A to Account B

1. READ balance_A
2. READ balance_B
3. balance_A = balance_A - 100
4. balance_B = balance_B + 100
5. WRITE balance_A
6. WRITE balance_B

Atomicity guarantees:
- Either ALL steps complete OR NONE complete
- No partial state where A is debited but B is not credited
```

### Isolation Levels

```
Isolation Level          │ Phantom Reads │ Dirty Reads │ Non-repeatable
─────────────────────────┼───────────────┼─────────────┼───────────────
Read Uncommitted         │ Yes           │ Yes         │ Yes
Read Committed           │ Yes           │ No          │ Yes
Repeatable Read          │ Yes           │ No          │ No
Serializable             │ No            │ No          │ No
Strict Serializable      │ No            │ No          │ No + Linearizable
```

Dragonfly provides **Strict Serializability** - the strongest isolation level:

- Operations appear to execute in some total order
- Order respects real-time ordering (linearizability)
- Multi-key operations are atomic

## Part 5: Memory Efficiency

### Why Memory Efficiency Matters

In-memory datastores store ALL data in RAM. RAM is expensive (~$5-10/GB). Efficient memory use directly reduces costs.

### Redis Memory Overhead

```
Redis stores each key-value pair with metadata:

dictEntry structure (24 bytes per entry):
┌─────────────────────┐
│  next* (8 bytes)    │ -> Points to next entry in chain
│  key* (8 bytes)     │ -> Points to key string
│  v ptr (8 bytes)    │ -> Points to value
└─────────────────────┘

Plus bucket array overhead:
- 8 bytes per bucket
- Load factor 50-100% utilization
- Average: 16-24 bytes overhead per record

Total overhead: 16-32 bytes per record
```

### Dragonfly Memory Efficiency

```
Dragonfly uses two key structures:

1. Dashtable (main storage):
   - 6-16 bytes overhead per record
   - 30% more efficient than Redis

2. DenseSet (for sets):
   - 12 bytes per record (vs 32 in Redis)
   - Pointer tagging eliminates entry allocations

Memory comparison (5GB dataset):
- Redis: 5.86GB peak (with overhead)
- Dragonfly: 3.23GB peak
- Savings: ~45%
```

## Part 6: Expiration and Eviction

### Key Expiration (TTL)

Both Redis and Dragonfly support time-to-live on keys:

```bash
# Set key with 1 hour TTL
SET session:user123 "data" EX 3600

# Set existing key with TTL
EXPIRE session:user123 3600

# Get remaining TTL
TTL session:user123

# Set millisecond precision
PEXPIRE session:user123 3600000
```

### Dragonfly's Expiration Strategy

Dragonfly uses a hybrid approach:

```
1. Passive Expiration (on access):
   - Check TTL when key is accessed
   - Delete if expired before returning
   - Zero CPU overhead for idle keys

2. Proactive Expiration (background):
   - Gradually scan dashtable segments
   - Delete expired keys during segment splits
   - Prevents memory buildup

3. Lazy Eviction (at maxmemory):
   - Only evict when approaching memory limit
   - Uses adaptive algorithm (not pure LRU/LFU)
   - Achieves higher hit rates with zero overhead
```

### Cache Mode

Dragonfly has a unified caching algorithm:

```bash
# Enable cache mode
dragonfly --cache_mode=true --maxmemory=8gb

# Cache behavior:
# - Evicts items least likely to be accessed
# - Only evicts when near maxmemory limit
# - Zero memory overhead (no LRU list)
# - Higher hit rates than LRU/LFU
```

## Part 7: Persistence Options

### RDB Snapshots

```bash
# Manual snapshot
BGSAVE  # Asynchronous
SAVE    # Synchronous (blocks)

# Automatic snapshots
# cron expression in config
snapshot_cron: "0 */6 * * *"  # Every 6 hours

# Snapshot files stored in /data
dump.rdb
dump-20260328-120000.rdb
```

RDB format is compact and fast to load:
- Single binary file
- Point-in-time snapshot
- Minutes to save GBs of data

### AOF (Append-Only File)

```bash
# Enable AOF
appendonly: "yes"
appendfsync: "everysec"  # fsync options: always, everysec, no
```

AOF logs every write operation:
- More durable (can recover to last write)
- Larger file size
- Slower recovery than RDB

### Dragonfly's Forkless Snapshot

```
Traditional BGSAVE (Redis):
1. fork() creates child process
2. Child writes RDB to disk
3. Copy-on-Write (CoW) shares memory
4. Parent continues serving requests
5. Memory spike during CoW

Dragonfly's Approach:
1. No fork() - runs asynchronously
2. Uses dashtable structure for iteration
3. Maintains point-in-time guarantees
4. No memory spike
5. Faster completion
```

## Part 8: Common Pitfalls and Best Practices

### Pitfall 1: Using KEYS in Production

```bash
# DON'T - KEYS blocks the server
KEYS user:*  # Scans ALL keys - O(N)

# DO - Use SCAN instead
SCAN 0 MATCH user:* COUNT 100  # Incremental - O(1) per call
```

### Pitfall 2: Large Keys in Transactions

```bash
# DON'T - Transaction with too many keys
MSET key1 v1 key2 v2 ... key100000 v100000  # Blocks all shards

# DO - Batch into smaller groups
MSET key1 v1 ... key100 v100
MSET key101 v101 ... key200 v200
```

### Pitfall 3: Ignoring Memory Limits

```bash
# DON'T - Let memory grow unbounded
# maxmemory: 0  # Unlimited - dangerous!

# DO - Set appropriate limit
maxmemory: 8gb
maxmemory-policy: volatile-lru  # Evict keys with TTL
```

### Best Practice 1: Use Connection Pooling

```python
# DON'T - Create new connection per request
for request in requests:
    redis = Redis()
    redis.get("key")

# DO - Use connection pool
pool = ConnectionPool(max_connections=100)
for request in requests:
    redis = Redis(connection_pool=pool)
    redis.get("key")
```

### Best Practice 2: Pipeline Commands

```python
# DON'T - One round-trip per command
for key in keys:
    value = redis.get(key)  # Network RTT each time

# DO - Pipeline reduces round-trips
pipe = redis.pipeline()
for key in keys:
    pipe.get(key)
results = pipe.execute()  # Single network RTT
```

### Best Practice 3: Monitor Memory

```bash
# Check memory usage
INFO memory

# Key metrics:
# used_memory: Total memory allocated
# used_memory_rss: Memory from OS perspective
# used_memory_peak: Maximum memory reached
# mem_fragmentation_ratio: rss / used_memory
```

---

## Summary

DragonflyDB fundamentals:

1. **Shared-Nothing Architecture** - Each thread owns its shard, enabling linear scaling
2. **Strict Serializability** - Strongest isolation with VLL transaction framework
3. **Memory Efficiency** - Dashtable and DenseSet reduce overhead by 30-45%
4. **Adaptive Caching** - Zero-overhead eviction with higher hit rates
5. **Forkless Snapshots** - No memory spikes during persistence
6. **Redis Compatible** - Drop-in replacement with 185+ commands

---

*This document is part of the DragonflyDB exploration series. See [exploration.md](./exploration.md) for the complete index.*
