---
title: "Replication and Consensus Deep Dive: Turso/libSQL"
subtitle: "Embedded replica sync, WAL propagation, and consistency guarantees"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.turso
explored_at: 2026-03-28
related: 00-zero-to-db-engineer.md, 01-storage-engine-deep-dive.md
---

# 03 - Consensus and Replication Deep Dive

## Overview

This document covers how libSQL replicates data from primary to embedded replicas, the sync protocol, consistency models, and how to reason about distributed database correctness.

## Part 1: Replication Fundamentals

### Why Replicate?

```
Single Database:
┌──────────────┐
│   Primary    │
│   Database   │
└──────┬───────┘
       │
   ┌───┴───┐
   ▼       ▼
Client1  Client2

Problem: Single point of failure, latency for distant clients


Replicated:
┌──────────────┐
│   Primary    │
│   Database   │
└──────┬───────┘
       │
   ┌───┼───┬─────────┐
   ▼   ▼   ▼         ▼
┌────────┐ ┌────────┐ ┌────────┐
│Replica │ │Replica │ │Replica │
│  (US)  │ │  (EU)  │ │  (AS)  │
└────────┘ └────────┘ └────────┘

Benefits:
- Fault tolerance (any replica can serve reads)
- Low latency (read from nearest replica)
- Load distribution (reads spread across replicas)
```

### Replication Topologies

**Primary-Secondary (libSQL model):**
```
         ┌──────────┐
         │  Primary │ ← All writes go here
         │ (Leader) │
         └────┬─────┘
              │ WAL frames
    ┌─────────┼─────────┐
    ▼         ▼         ▼
┌────────┐ ┌────────┐ ┌────────┐
│Secondary│ │Secondary│ │Secondary│
│(Follower)│ │(Follower)│ │(Follower)│
└────────┘ └────────┘ └────────┘

Reads: Any node
Writes: Primary only
Consistency: Eventual (replicas lag behind)
```

**Multi-Primary:**
```
┌──────────┐         ┌──────────┐
│ Primary  │ ←────→ │ Primary  │
│    A     │  Sync  │    B     │
└──────────┘         └──────────┘
     │                    │
     ▼                    ▼
  Clients              Clients

Reads: Any node
Writes: Any node (conflict resolution needed)
Consistency: Depends on sync protocol
```

**Leaderless (Dynamo-style):**
```
┌──────────┐
│  Node A  │
└────┬─────┘
     │ W+R > N
┌────┼─────┐
│    │     │
▼    ▼     ▼
Node B  Node C  Node D

Write to N nodes, read from N nodes
Quorum ensures consistency
```

## Part 2: WAL Propagation Protocol

### Sync Message Format

```protobuf
// Client (replica) requests sync from primary
message SyncRequest {
    // Replica's current position in WAL
    uint64 frame_offset = 1;

    // Replica identifier for tracking
    bytes replica_id = 2;

    // Optional: Request specific pages
    repeated uint32 page_numbers = 3;
}

// Primary responds with new frames
message SyncResponse {
    // Current WAL size at primary
    uint64 current_frame_offset = 1;

    // New frames since replica's offset
    repeated WalFrame frames = 2;

    // Current database size in pages
    uint32 database_size_pages = 3;

    // Checkpoint sequence number
    uint64 checkpoint_seq = 4;
}

// Individual WAL frame
message WalFrame {
    // Page number (1-indexed)
    uint32 page_number = 1;

    // Raw page data (page_size bytes)
    bytes page_data = 2;

    // Frame metadata
    uint32 db_size_after_commit = 3;  // 0 if not commit frame
    uint64 salt1 = 4;
    uint64 salt2 = 5;
}
```

### HTTP Sync Implementation

```rust
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SyncRequest {
    frame_offset: u64,
    replica_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_numbers: Option<Vec<u32>>,
}

#[derive(Deserialize, Debug)]
struct SyncResponse {
    current_frame_offset: u64,
    frames: Vec<WalFrame>,
    database_size_pages: u32,
    checkpoint_seq: u64,
}

#[derive(Deserialize, Debug, Clone)]
struct WalFrame {
    page_number: u32,
    #[serde(with = "base64")]
    page_data: Vec<u8>,
    db_size_after_commit: u32,
    salt1: u64,
    salt2: u64,
}

struct SyncClient {
    http: Client,
    primary_url: String,
    auth_token: String,
    replica_id: String,
}

impl SyncClient {
    fn new(primary_url: String, auth_token: String) -> Self {
        Self {
            http: Client::new(),
            primary_url,
            auth_token,
            replica_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    async fn sync(&self, current_offset: u64) -> Result<SyncResponse, SyncError> {
        let request = SyncRequest {
            frame_offset: current_offset,
            replica_id: self.replica_id.clone(),
            page_numbers: None,
        };

        let response = self.http
            .post(&format!("{}/v1/sync", self.primary_url))
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SyncError::HttpError(response.status()));
        }

        let sync_response = response.json().await?;
        Ok(sync_response)
    }

    async fn execute(&self, query: String, params: Vec<Value>) -> Result<ExecuteResponse, SyncError> {
        let response = self.http
            .post(&format!("{}/v1/sql", self.primary_url))
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .json(&ExecuteRequest {
                statements: vec![Statement { sql: query, params }],
            })
            .send()
            .await?;

        Ok(response.json().await?)
    }
}
```

### Replica Sync State Machine

```rust
#[derive(Debug, Clone)]
enum ReplicaState {
    /// Initial state - need full snapshot
    Empty,

    /// Received snapshot, applying
    ApplyingSnapshot {
        snapshot_id: String,
        pages_received: u32,
        total_pages: u32,
    },

    /// Caught up, receiving incremental WAL frames
    Synced {
        current_frame_offset: u64,
        last_checkpoint_seq: u64,
    },

    /// Behind primary, catching up
    CatchingUp {
        current_frame_offset: u64,
        target_frame_offset: u64,
    },

    /// Error - need to retry
    Error {
        error: SyncError,
        retry_after: Duration,
    },
}

struct Replica {
    state: ReplicaState,
    local_db: SqliteDatabase,
    wal_file: WalFile,
    sync_client: SyncClient,
}

impl Replica {
    async fn sync_loop(&mut self) -> Result<(), SyncError> {
        loop {
            match &self.state {
                ReplicaState::Empty => {
                    // Request initial snapshot
                    let snapshot = self.sync_client.request_snapshot().await?;
                    self.apply_snapshot(snapshot).await?;
                }

                ReplicaState::Synced { current_frame_offset, .. } => {
                    // Request incremental sync
                    let response = self.sync_client.sync(*current_frame_offset).await?;

                    if response.frames.is_empty() {
                        // Already caught up, wait before next sync
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        continue;
                    }

                    self.state = ReplicaState::CatchingUp {
                        current_frame_offset: *current_frame_offset,
                        target_frame_offset: response.current_frame_offset,
                    };
                }

                ReplicaState::CatchingUp { current_frame_offset, .. } => {
                    let response = self.sync_client.sync(*current_frame_offset).await?;

                    // Apply frames
                    let new_offset = self.apply_frames(&response.frames).await?;

                    if new_offset >= response.current_frame_offset {
                        // Caught up!
                        self.state = ReplicaState::Synced {
                            current_frame_offset: new_offset,
                            last_checkpoint_seq: response.checkpoint_seq,
                        };
                    } else {
                        // Still catching up
                        self.state = ReplicaState::CatchingUp {
                            current_frame_offset: new_offset,
                            target_frame_offset: response.current_frame_offset,
                        };
                    }
                }

                ReplicaState::Error { retry_after, .. } => {
                    tokio::time::sleep(*retry_after).await;
                    // Retry
                }

                ReplicaState::ApplyingSnapshot { .. } => {
                    // Handled in Empty branch
                }
            }
        }
    }

    async fn apply_frames(&mut self, frames: &[WalFrame]) -> Result<u64, SyncError> {
        let mut offset = match &self.state {
            ReplicaState::CatchingUp { current_frame_offset, .. } => *current_frame_offset,
            _ => 0,
        };

        for frame in frames {
            // Validate frame
            if !self.validate_frame(frame) {
                return Err(SyncError::InvalidFrame {
                    page_number: frame.page_number,
                });
            }

            // Write to local WAL
            self.wal_file.append_frame(frame)?;

            // Update offset
            offset += frame.size_on_disk();
        }

        // Checkpoint if this was a commit
        if frames.iter().any(|f| f.db_size_after_commit > 0) {
            self.checkpoint()?;
        }

        Ok(offset)
    }

    fn validate_frame(&self, frame: &WalFrame) -> bool {
        // Verify salt matches WAL header
        // Verify checksum
        // Verify page_number is valid
        true
    }

    fn checkpoint(&mut self) -> Result<(), SyncError> {
        // Apply WAL frames to database file
        self.local_db.checkpoint()?;
        Ok(())
    }
}
```

## Part 3: Consistency Models

### Strong Consistency (Linearizability)

```
Definition: All operations appear to execute atomically in a single total order

Timeline:
Client A: Write(x=1)─────────────────────Commit
                │
                ▼
Client B:              Read(x)──────────→ 1
                │
                ▼
Client C:                     Read(x)───→ 1

Guarantee: Once write commits, ALL future reads see that value

Cost: Must wait for primary acknowledgment
      Cannot read from stale replicas
```

### Eventual Consistency

```
Definition: If no new updates, eventually all reads return the last updated value

Timeline:
Client A: Write(x=1)─────────Commit at Primary
                │
                ├───────→ Replica 1 (receives update)
                │
                ├───────→ Replica 2 (delayed)
                │
                ▼
Client B:              Read from R1 ─────→ 1  ✓
                │
                ▼
Client C:              Read from R2 ─────→ 0  ✗ (stale)
                │
                │ (sync happens)
                ▼
Client C:              Read from R2 ─────→ 1  ✓ (now consistent)

Guarantee: Will eventually see the update
Cost: Reads may return stale data
Benefit: Lower latency (read from local replica)
```

### Read-After-Write Consistency

```
Definition: After writing, the writer always sees their own write

Timeline:
Client A: Write(x=1)─────────Commit
                │
                │ (A's next read goes to primary or synced replica)
                ▼
Client A:              Read(x) ──────────→ 1  ✓

Client B:              Read from R2 ─────→ 0  (allowed to be stale)

Guarantee: Writer sees their own writes
Benefit: Good compromise - users see their own changes immediately
```

### Consistency Levels in libSQL

```rust
#[derive(Debug, Clone, Copy)]
enum ConsistencyLevel {
    /// Strong: Always read from primary
    Strong,

    /// Eventual: Read from local replica (may be stale)
    Eventual,

    /// ReadAfterWrite: Track writer's session, ensure they see their writes
    ReadAfterWrite,
}

impl Database {
    async fn read_with_consistency(
        &self,
        query: &str,
        consistency: ConsistencyLevel,
    ) -> Result<ResultSet, Error> {
        match consistency {
            ConsistencyLevel::Strong => {
                // Always go to primary
                self.execute_remote(query, &[]).await
            }

            ConsistencyLevel::Eventual => {
                // Read from local replica
                self.execute_local(query, &[])
            }

            ConsistencyLevel::ReadAfterWrite => {
                // Check if client has uncommitted writes
                if let Some(write_lsn) = self.session.write_lsn {
                    // Wait until replica catches up to write_lsn
                    self.wait_for_replica(write_lsn).await?;
                }
                self.execute_local(query, &[])
            }
        }
    }

    async fn wait_for_replica(&self, target_lsn: u64) -> Result<(), TimeoutError> {
        let timeout = Duration::from_secs(5);
        let start = Instant::now();

        while start.elapsed() < timeout {
            let current = self.replica.current_lsn();
            if current >= target_lsn {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Err(TimeoutError::ReplicaSync)
    }
}
```

## Part 4: Conflict Resolution

### Why Conflicts Happen

```
Multi-Primary Scenario:

Primary A                    Primary B
    │                            │
    │ Write(x=1)                 │
    │ Local commit               │
    │                            │ Write(x=2)
    │                            │ Local commit
    │                            │
    │ ←── Sync ──→               │
    │                            │
    │ Conflict! x=1 vs x=2       │

Resolution strategies needed
```

### Resolution Strategies

**Last-Writer-Wins (LWW):**
```rust
struct VersionedValue {
    value: Value,
    timestamp: u64,  // Unix timestamp
    writer_id: String,
}

fn resolve_lww(a: VersionedValue, b: VersionedValue) -> VersionedValue {
    if a.timestamp >= b.timestamp {
        a
    } else {
        b
    }
}

// Problem: Clock skew can cause data loss!
```

**Vector Clocks:**
```rust
#[derive(Clone)]
struct VectorClock {
    /// Map of writer_id → counter
    counters: HashMap<String, u64>,
}

impl VectorClock {
    fn increment(&mut self, writer_id: &str) {
        *self.counters.entry(writer_id.to_string()).or_insert(0) += 1;
    }

    /// Returns:
    /// - Ordering::Less if self happened-before other
    /// - Ordering::Greater if other happened-before self
    /// - Ordering::Equal if concurrent (conflict!)
    fn compare(&self, other: &VectorClock) -> Ordering {
        let self_greater = other.counters.iter()
            .any(|(id, &v)| self.counters.get(id).unwrap_or(&0) < &v);
        let other_greater = self.counters.iter()
            .any(|(id, &v)| other.counters.get(id).unwrap_or(&0) < &v);

        match (self_greater, other_greater) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => Ordering::Equal,  // Identical
            (true, true) => Ordering::Greater,  // Concurrent! Conflict!
        }
    }
}

// On conflict: application must resolve
```

**Application-Defined Resolution:**
```rust
trait ConflictResolver {
    fn resolve(&self, local: Value, remote: Value) -> Value;
}

// Example: Sum counters (CRDT-style)
struct SumResolver;
impl ConflictResolver for SumResolver {
    fn resolve(&self, local: Value, remote: Value) -> Value {
        Value::Integer(local.as_integer() + remote.as_integer())
    }
}

// Example: Keep max value
struct MaxResolver;
impl ConflictResolver for MaxResolver {
    fn resolve(&self, local: Value, remote: Value) -> Value {
        Value::Integer(local.as_integer().max(remote.as_integer()))
    }
}

// Example: Custom business logic
struct BusinessResolver;
impl ConflictResolver for BusinessResolver {
    fn resolve(&self, local: Value, remote: Value) -> Value {
        // Apply domain-specific rules
        // e.g., "higher balance wins" for banking
        if local.as_integer() > remote.as_integer() {
            local
        } else {
            remote
        }
    }
}
```

### libSQL's Approach (Single Primary)

```
libSQL avoids conflicts by design:

┌──────────────┐
│   Primary    │ ← ONLY writer
└──────┬───────┘
       │
       │ One-way WAL propagation
       ▼
┌──────────────┐
│   Replica    │ ← Read-only
└──────────────┘

Benefits:
- No conflicts (single source of truth)
- Simple mental model
- WAL ordering guarantees consistency

Trade-offs:
- Write latency (must reach primary)
- Primary is bottleneck for writes
- Primary failure = write unavailability
```

## Part 5: Failure Scenarios

### Network Partition

```
┌──────────────┐         ┌──────────────┐
│   Primary    │         │   Replica    │
│              │         │              │
│  Write(x=1)  │         │  Read(x) → ? │
│              │         │              │
└──────────────┘         └──────────────┘
         ╳═══════════════╳
              Partition

Replica behavior depends on consistency level:
- Strong: Fail (cannot reach primary)
- Eventual: Return stale value
- ReadAfterWrite: Timeout waiting for sync
```

### Primary Failure

```
Before Failure:
┌──────────────┐
│   Primary    │
└──────┬───────┘
       │
   ┌───┴───┐
   ▼       ▼
Replica1  Replica2

After Primary Failure:
┌──────────────┐
│   PRIMARY    │  ← Unresponsive
│   (dead)     │
└──────┬───────┘
       │
   ┌───┴───┐
   ▼       ▼
Replica1  Replica2

Options:
1. Manual failover: Promote Replica1 to primary
2. Automatic failover: Consensus protocol (Raft) elects new leader
3. Read-only mode: Serve reads from replicas, reject writes
```

### Replica Corruption

```
Primary sends:    WAL frame with page 42, checksum ABCD
Replica receives: WAL frame with page 42, checksum WXYZ (corrupted!)

Detection:
1. Checksum mismatch
2. Salt mismatch
3. Frame ordering violation

Recovery:
1. Reject corrupted frame
2. Request retransmission
3. If persistent, full resync from snapshot
```

## Part 6: Monitoring and Observability

### Key Metrics

```rust
struct ReplicaMetrics {
    /// Current WAL offset at replica
    current_frame_offset: u64,

    /// Current WAL offset at primary (from last sync)
    primary_frame_offset: u64,

    /// Lag in frames
    frame_lag: u64,

    /// Lag in time (estimated from frame timestamps)
    time_lag: Duration,

    /// Sync operations
    sync_count: u64,
    sync_errors: u64,

    /// Query metrics
    queries_local: u64,
    queries_remote: u64,

    /// Latency
    sync_latency_p50: Duration,
    sync_latency_p99: Duration,
    query_latency_p50: Duration,
    query_latency_p99: Duration,
}

impl ReplicaMetrics {
    fn is_healthy(&self) -> bool {
        // Lag should be under threshold
        self.frame_lag < 1000 && self.time_lag < Duration::from_secs(60)
    }

    fn lag_percentage(&self) -> f64 {
        if self.primary_frame_offset == 0 {
            0.0
        } else {
            (self.frame_lag as f64 / self.primary_frame_offset as f64) * 100.0
        }
    }
}
```

### Alerting Rules

```yaml
# Prometheus alerting rules

groups:
- name: libsql_replica
  rules:
  - alert: ReplicaLagHigh
    expr: libsql_replica_lag_seconds > 60
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "Replica {{ $labels.replica_id }} lag is high"
      description: "Replica is {{ $value }} seconds behind primary"

  - alert: ReplicaSyncFailing
    expr: rate(libsql_replica_sync_errors_total[5m]) > 0.1
    for: 2m
    labels:
      severity: critical
    annotations:
      summary: "Replica {{ $labels.replica_id }} sync is failing"

  - alert: ReplicaCompletelyStale
    expr: libsql_replica_lag_seconds > 3600
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "Replica {{ $labels.replica_id }} is severely behind"
      description: "Replica is {{ $value }} seconds behind - may need resync"
```

---

*This document is part of the Turso/libSQL exploration series. See [exploration.md](./exploration.md) for the complete index.*
