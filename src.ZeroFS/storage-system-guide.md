# Storage System Guide: Building Resilient Distributed Storage

**A Practical Guide for Engineers**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Storage Fundamentals](#storage-fundamentals)
3. [Building Your First Storage System](#building-your-first-storage-system)
4. [Common Pitfalls](#common-pitfalls)
5. [Best Practices](#best-practices)
6. [Step-by-Step Implementation Guide](#step-by-step-implementation-guide)
7. [Glossary](#glossary)

---

## Introduction

### Who This Guide Is For

This guide is for engineers who:
- Are new to distributed systems
- Want to understand storage system basics
- Need practical, actionable advice
- Don't have a PhD in computer science

### Who This Guide Is NOT For

This guide is NOT for:
- Experts looking for cutting-edge research
- Theoretical deep-dives without practical application
- Specific vendor product documentation

### What You'll Learn

By the end of this guide, you'll understand:
- How distributed storage works
- Common patterns and anti-patterns
- How to avoid costly mistakes
- Step-by-step implementation approach

---

## Storage Fundamentals

### What is Distributed Storage?

**Simple definition:** Storing data across multiple computers so it's:
- **Reliable**: Survives hardware failures
- **Scalable**: Grows with your data
- **Available**: Accessible even when some computers are down

**Analogy:** Think of distributed storage like a library with multiple branches:
- Your book (data) might be at any branch
- If one branch closes, you can get it from another
- Adding more branches means more books can be stored

### Key Concepts

#### 1. Replication

```
Simple Replication (3x):
Data: "Hello"

Stored as:
Server 1: "Hello"
Server 2: "Hello"
Server 3: "Hello"

Pros: Simple, fast reads
Cons: 3x storage cost
```

#### 2. Erasure Coding

```
Erasure Coding (4+2):
Data: [A] [B] [C] [D]

Encoded as:
Server 1: [A]
Server 2: [B]
Server 3: [C]
Server 4: [D]
Server 5: [P1] ← Parity
Server 6: [P2] ← Parity

Can lose ANY 2 servers and still recover!
Pros: 1.5x storage cost (vs 3x for replication)
Cons: More CPU for encoding/decoding
```

#### 3. Consistency Models

```
Strong Consistency:
Client 1 writes "X=5" → All clients immediately see "X=5"
Like: A single notebook everyone shares

Eventual Consistency:
Client 1 writes "X=5" → Clients might see old value briefly
Like: Email - takes time to propagate to all servers
```

### Storage Hierarchy

```
┌─────────────────────────────────────────┐
│          Storage Pyramid                 │
├─────────────────────────────────────────┤
│                                         │
│              ▲                          │
│             / \                         │
│            /   \                        │
│           / CPU \                       │
│          / Registers \                  │
│         ├─────────────┤                 │
│        /   L1/L2/L3   \                 │
│       /     Cache      \                │
│      ├───────────────────┤              │
│     /      RAM (GBs)     \              │
│    ├───────────────────────┤            │
│   /    SSD (100s of GBs)   \            │
│  ├───────────────────────────┤          │
│ /   HDD / Network Storage    \          │
│├───────────────────────────────┤        │
│/    Object Storage (S3)        \       │
│─────────────────────────────────        │
│                                         │
│  Fast ↑              Cheap ↓            │
│  Small ↑             Large ↓            │
│                                         │
└─────────────────────────────────────────┘
```

---

## Building Your First Storage System

### Start Simple: Single Node

```rust
// Step 1: Basic key-value store
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

struct SimpleStore {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl SimpleStore {
    fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn put(&self, key: String, value: Vec<u8>) {
        let mut data = self.data.write().unwrap();
        data.insert(key, value);
    }

    fn get(&self, key: &str) -> Option<Vec<u8>> {
        let data = self.data.read().unwrap();
        data.get(key).cloned()
    }
}

// Usage:
// let store = SimpleStore::new();
// store.put("name".to_string(), b"Alice".to_vec());
// let name = store.get("name");  // Some([65, 108, 105, 99, 101])
```

### Add Persistence: Write-Ahead Log

```rust
// Step 2: Add durability with WAL
use std::fs::File;
use std::io::{Write, Read};

struct PersistentStore {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    wal_file: Arc<Mutex<File>>,
}

impl PersistentStore {
    fn put(&self, key: String, value: Vec<u8>) -> Result<()> {
        // 1. Write to WAL FIRST (durability)
        {
            let mut wal = self.wal_file.lock().unwrap();
            wal.write_all(b"PUT ")?;
            wal.write_all(key.as_bytes())?;
            wal.write_all(b" ")?;
            wal.write_all(&value)?;
            wal.write_all(b"\n")?;
            wal.sync()?;  // Force to disk!
        }

        // 2. Then update in-memory data
        {
            let mut data = self.data.write().unwrap();
            data.insert(key, value);
        }

        Ok(())
    }

    // On startup: replay WAL to restore data
    fn recover(&mut self) -> Result<()> {
        let mut wal = self.wal_file.lock().unwrap();
        let mut contents = String::new();
        wal.read_to_string(&mut contents)?;

        for line in contents.lines() {
            // Parse and replay each operation
            if line.starts_with("PUT ") {
                // ... parse and restore
            }
        }

        Ok(())
    }
}
```

### Add Replication: Two Copies

```rust
// Step 3: Add a replica
struct ReplicatedStore {
    local: SimpleStore,
    replica_url: String,
}

impl ReplicatedStore {
    fn put(&self, key: String, value: Vec<u8>) -> Result<()> {
        // Write locally
        self.local.put(key.clone(), value.clone());

        // Write to replica
        let client = reqwest::blocking::Client::new();
        client
            .post(&format!("{}/put", self.replica_url))
            .json(&PutRequest { key, value })
            .send()?;

        Ok(())
    }
}
```

---

## Common Pitfalls

### Pitfall 1: Not Handling Failures

```rust
// ❌ BAD: Assumes everything always works
fn write_data(data: &[u8]) {
    let mut file = File::create("data.txt").unwrap();  // What if disk is full?
    file.write_all(data).unwrap();  // What if write fails?
}

// ✅ GOOD: Handle errors properly
fn write_data(data: &[u8]) -> Result<()> {
    let mut file = File::create("data.txt")
        .context("Failed to create file")?;
    file.write_all(data)
        .context("Failed to write data")?;
    file.sync_all()
        .context("Failed to sync to disk")?;
    Ok(())
}
```

### Pitfall 2: Race Conditions

```rust
// ❌ BAD: Race condition
let current = counter.load(Ordering::Relaxed);
counter.store(current + 1, Ordering::Relaxed);  // Another thread might have changed it!

// ✅ GOOD: Atomic increment
counter.fetch_add(1, Ordering::SeqCst);

// ✅ GOOD: With locking
let mut lock = mutex.lock().unwrap();
*lock += 1;
```

### Pitfall 3: Not Syncing to Disk

```rust
// ❌ BAD: Data might be lost on crash
file.write_all(data)?;
// Data is in OS cache, not on disk!

// ✅ GOOD: Force to disk
file.write_all(data)?;
file.sync_all()?;  // Now it's durable
```

### Pitfall 4: Infinite Retries

```rust
// ❌ BAD: Infinite retry loop
loop {
    match write_to_network() {
        Ok(_) => break,
        Err(_) => continue,  // Retries forever!
    }
}

// ✅ GOOD: Bounded retries with backoff
let mut attempts = 0;
loop {
    match write_to_network() {
        Ok(_) => break,
        Err(e) if attempts < MAX_ATTEMPTS => {
            attempts += 1;
            sleep(Duration::from_millis(100 * attempts));  // Backoff
        }
        Err(e) => return Err(e),  // Give up
    }
}
```

### Pitfall 5: Ignoring Network Partitions

```
Network Partition Scenario:

Before partition:
[Client] ←→ [Server A] ←→ [Server B]
     ↓           ↓             ↓
   Works     Works         Works

After partition:
[Client] ←→ [Server A]    [Server B]
     ↓           ↓             ↓
   Works     Works        ISOLATED!

Question: Can Client still write?
- If yes: Risk of inconsistency (split-brain)
- If no: Risk of unavailability
```

---

## Best Practices

### 1. Design for Failure

```rust
// Expect failures, don't be surprised by them
struct ResilientClient {
    servers: Vec<String>,
    timeout: Duration,
    max_retries: u32,
}

impl ResilientClient {
    async fn request(&self, op: Operation) -> Result<Response> {
        for attempt in 0..self.max_retries {
            for server in &self.servers {
                match self.try_server(server, &op).await {
                    Ok(response) => return Ok(response),
                    Err(e) => {
                        warn!("Server {} failed: {}", server, e);
                        // Try next server
                    }
                }
            }
            // All servers failed, backoff and retry
            tokio::time::sleep(Duration::from_millis(100 * 2_u64.pow(attempt))).await;
        }
        Err(Error::AllRetriesExhausted)
    }
}
```

### 2. Use Exponential Backoff

```rust
async fn retry_with_backoff<F, T>(operation: F) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let mut delay = Duration::from_millis(100);
    let max_delay = Duration::from_secs(30);

    for attempt in 0..MAX_ATTEMPTS {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt < MAX_ATTEMPTS - 1 => {
                warn!("Attempt {} failed: {}", attempt + 1, e);
                tokio::time::sleep(delay).await;
                delay = min(delay * 2, max_delay);  // Exponential backoff
            }
            Err(e) => return Err(e),
        }
    }

    Err(Error::MaxRetriesExceeded)
}
```

### 3. Implement Circuit Breakers

```rust
use std::time::{Duration, Instant};

enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing if recovered
}

struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
    failure_threshold: u32,
    reset_timeout: Duration,
}

impl CircuitBreaker {
    fn call<F, T>(&mut self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        match self.state {
            CircuitState::Open => {
                // Check if we should try again
                if self.last_failure.map(|t| t.elapsed() > self.reset_timeout).unwrap_or(false) {
                    self.state = CircuitState::HalfOpen;
                } else {
                    return Err(Error::CircuitOpen);  // Reject immediately
                }
            }
            _ => {}
        }

        match operation() {
            Ok(result) => {
                // Success - reset state
                self.failure_count = 0;
                self.state = CircuitState::Closed;
                Ok(result)
            }
            Err(e) => {
                // Failure - increment counter
                self.failure_count += 1;
                self.last_failure = Some(Instant::now());

                if self.failure_count >= self.failure_threshold {
                    self.state = CircuitState::Open;
                }

                Err(e)
            }
        }
    }
}
```

### 4. Monitor Everything

```rust
// Add metrics to your operations
use prometheus::{IntCounter, Histogram, register_int_counter, register_histogram};

struct StoreMetrics {
    gets: IntCounter,
    puts: IntCounter,
    latency: Histogram,
}

impl StoreMetrics {
    fn new() -> Self {
        Self {
            gets: register_int_counter!("store_gets_total", "Total get operations").unwrap(),
            puts: register_int_counter!("store_puts_total", "Total put operations").unwrap(),
            latency: register_histogram!(
                "store_operation_latency_seconds",
                "Operation latency",
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
            ).unwrap(),
        }
    }
}

impl PersistentStore {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let start = Instant::now();
        self.metrics.gets.inc();

        let result = self.data.read().unwrap().get(key).cloned();

        self.metrics.latency.observe(start.elapsed().as_secs_f64());

        Ok(result)
    }
}
```

### 5. Test Failure Scenarios

```rust
// Chaos testing: intentionally break things
#[cfg(test)]
mod chaos_tests {
    #[tokio::test]
    async fn test_server_crash_during_write() {
        let cluster = TestCluster::new(3);

        // Start a write
        let write_handle = tokio::spawn(async move {
            cluster.put("key", "value").await
        });

        // Kill one server mid-write
        tokio::time::sleep(Duration::from_millis(10)).await;
        cluster.kill_server(1).await;

        // Write should still succeed (quorum available)
        let result = write_handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_network_partition() {
        let cluster = TestCluster::new(5);

        // Partition the network
        cluster.partition(vec![0, 1], vec![2, 3, 4]).await;

        // Writes to minority partition should fail or timeout
        let result = cluster.servers[0].put("key", "value").await;
        assert!(result.is_err());

        // Writes to majority partition should succeed
        let result = cluster.servers[2].put("key", "value").await;
        assert!(result.is_ok());
    }
}
```

---

## Step-by-Step Implementation Guide

### Phase 1: Single Node (Week 1-2)

**Goal:** Working single-node storage

```
Day 1-2: Design
□ Define data model (key-value? document? block?)
□ Define API (get, put, delete, list)
□ Choose storage format (files? SQLite?)

Day 3-5: Implementation
□ Implement basic CRUD operations
□ Add error handling
□ Write unit tests

Day 6-7: Testing
□ Test normal operations
□ Test edge cases (empty data, large data)
□ Test error cases (disk full, permission denied)

Day 8-10: Persistence
□ Add write-ahead log
□ Implement recovery on startup
□ Test crash recovery
```

### Phase 2: Replication (Week 3-4)

**Goal:** Two-node replication

```
Day 1-2: Network Layer
□ Choose protocol (HTTP? gRPC? custom?)
□ Implement basic message passing
□ Add timeouts

Day 3-5: Replication Logic
□ Implement leader-follower or multi-leader
□ Handle write propagation
□ Handle read routing

Day 6-7: Failure Handling
□ Detect failed nodes
□ Handle split-brain scenarios
□ Implement failover

Day 8-10: Testing
□ Test normal replication
□ Test node failures
□ Test network partitions
```

### Phase 3: Production Readiness (Week 5-6)

**Goal:** Production-ready system

```
Day 1-3: Monitoring
□ Add metrics (latency, throughput, errors)
□ Add logging
□ Add health checks

Day 4-6: Configuration
□ Make settings configurable
□ Add sensible defaults
□ Document configuration options

Day 7-10: Documentation & Deployment
□ Write deployment guide
□ Create Docker image
□ Document runbooks
```

### Checklist for Each Phase

```
Phase Completion Checklist:
□ All planned features implemented
□ Unit tests passing
□ Integration tests passing
□ Error cases handled
□ Documentation written
□ Code reviewed
□ Performance benchmarks run
□ No known critical bugs
```

---

## Glossary

| Term | Definition |
|------|------------|
| **Availability** | System is up and responding |
| **Backup** | Copy of data for recovery |
| **CAS (Compare-And-Swap)** | Atomic operation: update if value unchanged |
| **Checkpoint** | Point-in-time snapshot |
| **Compaction** | Merging data files to reclaim space |
| **Consistency** | All nodes see the same data |
| **Durability** | Data survives crashes |
| **Erasure Coding** | Math-based redundancy (vs copying) |
| **Failover** | Automatic switch to backup |
| **Idempotent** | Operation can be repeated safely |
| **Leader** | Primary node coordinating writes |
| **LSM Tree** | Log-Structured Merge tree (storage format) |
| **Partition** | Network failure splitting cluster |
| **Quorum** | Minimum nodes needed for operation |
| **Replication** | Copying data to multiple nodes |
| **SSTable** | Sorted String Table (storage format) |
| **Sharding** | Splitting data across nodes |
| **Tombstone** | Marker for deleted data |
| **WAL** | Write-Ahead Log (for durability) |

---

## Summary

### Key Takeaways

1. **Start simple**: Single node first, then distribute
2. **Expect failures**: Design for things going wrong
3. **Test thoroughly**: Normal cases AND failure cases
4. **Monitor everything**: You can't fix what you can't measure
5. **Document**: Future you will thank present you

### Next Steps

1. Build a simple key-value store
2. Add persistence with WAL
3. Add a second node
4. Test what happens when you kill nodes
5. Add monitoring
6. Iterate and improve

### Recommended Reading

- [Designing Data-Intensive Applications](https://dataintensive.net/) by Martin Kleppmann
- [Site Reliability Engineering](https://sre.google/books/) by Google
- [The Little Book of Semaphores](https://greenteapress.com/wp/semaphores/) by Allen B. Downey

---

*Remember: Every expert was once a beginner. The key is to start simple, learn from mistakes, and keep improving.*
