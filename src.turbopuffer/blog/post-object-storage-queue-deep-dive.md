# Building a Distributed Queue in a Single JSON File on Object Storage - Deep Dive

## Executive Summary

Turbopuffer replaced a sharded job queue with a **single queue file on object storage** using a stateless broker pattern. The new design achieves:

- **10x lower tail latency** compared to the sharded implementation
- **At-least-once delivery** guarantees
- **FIFO ordering** for jobs
- **Simpler operations** (no shard rebalancing)

This post explains how to build a distributed queue on object storage, starting from the simplest possible design and adding complexity only as needed.

---

## The Problem: Sharded Queue Bottlenecks

### Original Architecture

```
┌─────────────────────────────────────────────────────────────┐
│           Sharded Queue Architecture (Old)                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐                                                │
│  │ Job     │                                                │
│  │ Producer│                                                │
│  └────┬────┘                                                │
│       │                                                      │
│       ▼                                                      │
│  ┌─────────────────────────────────────────────────┐       │
│  │           Hash-based Sharding                    │       │
│  │  (namespace_id % num_shards)                     │       │
│  └─────────────────────────────────────────────────┘       │
│       │         │         │         │                      │
│       ▼         ▼         ▼         ▼                      │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐             │
│  │ Shard 0│ │ Shard 1│ │ Shard 2│ │ Shard 3│   ...       │
│  │ Queue  │ │ Queue  │ │ Queue  │ │ Queue  │             │
│  └───┬────┘ └───┬────┘ └───┬────┘ └───┬────┘             │
│      │          │          │          │                    │
│      ▼          ▼          ▼          ▼                    │
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐             │
│  │Worker 0│ │Worker 1│ │Worker 2│ │Worker 3│   ...       │
│  └────────┘ └────────┘ └────────┘ └────────┘             │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Problem: Slow node blocks all jobs for its shard!
```

### The Slow Node Problem

```
Scenario: Indexing namespace "customer-123"
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  Shard 5 is assigned to Worker 3                           │
│                                                             │
│  Worker 3 is overloaded (95% CPU, high memory pressure)    │
│                                                             │
│  Result:                                                    │
│  - Jobs for Shard 5 queue up                              │
│  - Customer-123's index updates delayed by minutes         │
│  - Other workers are idle but can't help                  │
│                                                             │
│  This happened repeatedly in production!                   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Design Goal: Single Global Queue

### Requirements

```
Must Have:
┌─────────────────────────────────────────────────────────────┐
│  ✓ FIFO ordering (jobs processed in order)                 │
│  ✓ At-least-once delivery (no lost jobs)                   │
│  ✓ Exactly-once processing (idempotent execution)          │
│  ✓ High availability (survive node failures)               │
│  ✓ Low tail latency (<100ms p99)                           │
│  ✓ Scale to 1000+ jobs/second                              │
└─────────────────────────────────────────────────────────────┘

Nice to Have:
┌─────────────────────────────────────────────────────────────┐
│  ○ Simple to operate                                        │
│  ○ No complex coordination                                  │
│  ○ Easy to reason about                                     │
└─────────────────────────────────────────────────────────────┘
```

### Why Object Storage?

```
Object Storage (S3/GCS) Characteristics:
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  Durability: 11 nines (99.999999999%)                      │
│  Availability: 99.99% (four nines)                          │
│  Throughput: Essentially unlimited                          │
│  Cost: ~$0.023/GB/month (S3 Standard)                      │
│  Latency: 50-200ms for PUT/GET                             │
│                                                             │
│  Key insight: For a JOB QUEUE (not hot path),              │
│  object storage has ALL the properties we need!            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Step 1: The Simplest Thing That Works

### queue.json Design

```
Single file on object storage:

s3://turbopuffer-queues/production/indexing-queue.json

Content:
{
  "version": 42,
  "jobs": [
    {"id": "job-001", "status": "done",   "payload": {...}},
    {"id": "job-002", "status": "done",   "payload": {...}},
    {"id": "job-003", "status": "claimed", "payload": {...}, "claimed_by": "worker-1", "claimed_at": "2024-01-15T10:30:00Z"},
    {"id": "job-004", "status": "pending", "payload": {...}},
    {"id": "job-005", "status": "pending", "payload": {...}},
    {"id": "job-006", "status": "pending", "payload": {...}}
  ]
}
```

### Push Operation (with Compare-And-Set)

```rust
async fn push_job(job: Job) -> Result<()> {
    loop {
        // 1. Read current queue state
        let (queue, etag) = s3_get_with_etag("indexing-queue.json").await?;

        // 2. Append new job
        queue.jobs.push(JobEntry {
            id: generate_id(),
            status: "pending",
            payload: job,
        });

        // 3. Write back with CAS (fails if etag changed)
        match s3_put_conditional(
            "indexing-queue.json",
            &queue,
            &etag,  // Only succeeds if file hasn't changed
        ).await {
            Ok(new_etag) => {
                return Ok(());
            }
            Err(PreconditionFailed) => {
                // Another writer modified the file
                // Retry with new state
                continue;
            }
        }
    }
}
```

### Claim Operation (Worker)

```rust
async fn claim_job() -> Result<Option<Job>> {
    loop {
        // 1. Read current queue state
        let (queue, etag) = s3_get_with_etag("indexing-queue.json").await?;

        // 2. Find first pending job
        let job_idx = queue.jobs.iter()
            .position(|j| j.status == "pending")?;

        // 3. Mark as claimed
        queue.jobs[job_idx].status = "claimed";
        queue.jobs[job_idx].claimed_by = Some(WORKER_ID);
        queue.jobs[job_idx].claimed_at = Some(chrono::now());

        // 4. Write back with CAS
        match s3_put_conditional(
            "indexing-queue.json",
            &queue,
            &etag,
        ).await {
            Ok(_) => {
                return Ok(Some(queue.jobs[job_idx].clone()));
            }
            Err(PreconditionFailed) => {
                // Retry with fresh state
                continue;
            }
        }
    }
}
```

### Acknowledge Operation

```rust
async fn acknowledge_job(job_id: &str) -> Result<()> {
    loop {
        let (queue, etag) = s3_get_with_etag("indexing-queue.json").await?;

        // Find job and mark as done
        if let Some(job) = queue.jobs.iter_mut().find(|j| j.id == job_id) {
            job.status = "done";
        }

        match s3_put_conditional("indexing-queue.json", &queue, &etag).await {
            Ok(_) => return Ok(()),
            Err(PreconditionFailed) => continue,
        }
    }
}
```

---

## Step 2: Group Commit for Throughput

### The Bottleneck

```
Object Storage Write Limits:
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  GCS: 1 write/second per object (hard limit)               │
│  S3:  1000s of writes/second (but high latency)            │
│                                                             │
│  CAS Write Latency: ~200ms p50, ~500ms p99                 │
│                                                             │
│  With direct writes:                                       │
│  Max throughput = 1 / 0.2s = 5 jobs/second                 │
│                                                             │
│  We need 1000+ jobs/second!                                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Group Commit Solution

```
┌─────────────────────────────────────────────────────────────┐
│              Group Commit Architecture                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                    │
│  │ Pusher  │  │ Pusher  │  │ Pusher  │   ...              │
│  └────┬────┘  └────┬────┘  └────┬────┘                    │
│       │           │           │                            │
│       └───────────┼───────────┘                            │
│                   │                                         │
│                   ▼                                         │
│          ┌────────────────┐                                │
│          │  Write Buffer  │ ← Accumulate requests          │
│          │  [job1, job2,  │                                │
│          │   job3, ...]   │                                │
│          └───────┬────────┘                                │
│                  │                                         │
│            When: buffer.len() > 100                       │
│               OR: 10ms elapsed                            │
│                  │                                         │
│                  ▼                                         │
│          ┌────────────────┐                                │
│          │  Single CAS    │ ← One write for many jobs      │
│          │  Write to S3   │                                │
│          └────────────────┘                                │
│                                                             │
│  Throughput: 100 jobs / 0.2s = 500 jobs/second             │
│  (100x improvement!)                                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Group Commit Implementation

```rust
struct GroupCommitQueue {
    /// Pending jobs waiting to be committed
    buffer: Mutex<Vec<Job>>,

    /// Signal that a flush is in progress
    flush_in_progress: AtomicBool,

    /// Handle to background flush task
    flush_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl GroupCommitQueue {
    async fn push(&self, job: Job) -> Result<()> {
        // Add to buffer
        let mut buffer = self.buffer.lock().await;
        buffer.push(job);

        let should_flush = buffer.len() >= 100;
        drop(buffer);

        // Flush if buffer is full
        if should_flush {
            self.flush().await?;
        }

        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // Only one flush at a time
        if self.flush_in_progress.swap(true, Ordering::SeqCst) {
            return Ok(()); // Another flush is in progress
        }

        // Drain buffer
        let jobs = {
            let mut buffer = self.buffer.lock().await;
            std::mem::take(&mut *buffer)
        };

        // Write to S3
        loop {
            let (queue, etag) = s3_get_with_etag("indexing-queue.json").await?;

            let mut new_queue = queue.clone();
            for job in &jobs {
                new_queue.jobs.push(JobEntry::from(job));
            }

            match s3_put_conditional("indexing-queue.json", &new_queue, &etag).await {
                Ok(_) => break,
                Err(PreconditionFailed) => continue,
            }
        }

        self.flush_in_progress.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Background flush task (runs every 10ms)
    fn spawn_flush_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let _ = self.flush().await;
            }
        });
    }
}
```

---

## Step 3: Stateless Broker for Coordination

### The Contention Problem

```
Even with group commit:
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  100 pushers all trying to CAS the same object            │
│                                                             │
│  CAS guarantees: Only one writer succeeds at a time        │
│                                                             │
│  With 100 concurrent writers:                              │
│  - 99 CAS attempts fail per successful write              │
│  - Each failure = read + retry                            │
│  - Effective throughput collapses                         │
│                                                             │
│  We need to reduce contention!                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Broker Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 Broker Architecture                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                    │
│  │ Pusher  │  │ Pusher  │  │ Pusher  │   (100s)           │
│  └────┬────┘  └────┬────┘  └────┬────┘                    │
│       │           │           │                            │
│       │ gRPC      │ gRPC      │ gRPC                       │
│       ▼           ▼           ▼                            │
│  ┌─────────────────────────────────────────────┐          │
│  │                                             │          │
│  │           STATELESS BROKER                  │          │
│  │                                             │          │
│  │  ┌─────────────────────────────────┐       │          │
│  │  │     Single Group Commit Loop    │       │          │
│  │  │     (One writer to S3)          │       │          │
│  │  └─────────────────────────────────┘       │          │
│  │                                             │          │
│  └─────────────────────────────────────────────┘          │
│       │           │           │                            │
│       │ gRPC      │ gRPC      │ gRPC                       │
│       ▼           ▼           ▼                            │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                    │
│  │ Worker  │  │ Worker  │  │ Worker  │   (100s)           │
│  └─────────┘  └─────────┘  └─────────┘                    │
│                                                             │
│  Broker is the ONLY writer to object storage              │
│  No CAS contention!                                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Broker Implementation

```rust
struct Broker {
    /// In-memory queue state
    queue: RwLock<QueueState>,

    /// Notify workers of new jobs
    job_available: Notify,

    /// Signal for shutdown
    shutdown: AtomicBool,
}

struct QueueState {
    jobs: VecDeque<JobEntry>,
    version: u64,
    inflight: HashMap<String, InFlightJob>,
}

impl Broker {
    /// Handle push requests from clients
    async fn handle_push(&self, job: Job) -> Result<()> {
        let mut queue = self.queue.write().await;
        queue.jobs.push_back(JobEntry::pending(job));
        queue.version += 1;

        // Notify workers
        self.job_available.notify_one();

        Ok(())
    }

    /// Handle claim requests from workers
    async fn handle_claim(&self, worker_id: &str) -> Result<Option<Job>> {
        let mut queue = self.queue.write().await;

        // Find first pending job
        let job = queue.jobs.iter_mut()
            .find(|j| j.status == JobStatus::Pending);

        if let Some(job) = job {
            job.status = JobStatus::InFlight;
            job.inflight_since = Some(chrono::now());
            job.claimed_by = Some(worker_id.to_string());

            queue.inflight.insert(
                job.id.clone(),
                InFlightJob {
                    worker_id: worker_id.to_string(),
                    claimed_at: chrono::now(),
                }
            );

            return Ok(Some(job.payload.clone()));
        }

        Ok(None)
    }

    /// Background task: Persist to S3 periodically
    async fn persist_loop(self: Arc<Self>) {
        while !self.shutdown.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let queue = self.queue.read().await;
            let _ = s3_put("indexing-queue.json", &*queue).await;
        }
    }

    /// Recovery: Load state from S3 on startup
    async fn recover_from_s3(&self) -> Result<()> {
        let queue = s3_get("indexing-queue.json").await?;
        *self.queue.write().await = queue;
        Ok(())
    }
}
```

### Client-Broker Protocol

```rust
/// gRPC service definition
service JobQueue {
    /// Push a new job
    rpc PushJob(PushJobRequest) returns (PushJobResponse);

    /// Claim the next available job
    rpc ClaimJob(ClaimJobRequest) returns (ClaimJobResponse);

    /// Acknowledge job completion
    rpc AckJob(AckJobRequest) returns (AckJobResponse);

    /// Requeue a job (worker crashed)
    rpc RequeueJob(RequeueJobRequest) returns (RequeueJobResponse);
}

message PushJobRequest {
    Job job = 1;
}

message ClaimJobRequest {
    string worker_id = 1;
    int32 timeout_ms = 2;  // How long to wait for a job
}

message ClaimJobResponse {
    optional Job job = 1;
    string claim_token = 2;
}
```

---

## High Availability

### Broker Failover

```
┌─────────────────────────────────────────────────────────────┐
│               HA Broker Deployment                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────┐          │
│  │           Load Balancer                      │          │
│  └─────────────────────────────────────────────┘          │
│       │              │              │                      │
│       ▼              ▼              ▼                      │
│  ┌─────────┐   ┌─────────┐   ┌─────────┐                │
│  │ Broker  │   │ Broker  │   │ Broker  │                │
│  │   A     │   │   B     │   │   C     │                │
│  │(Leader) │   │(Follower│   │(Follower│                │
│  │  ACTIVE │   │ STANDBY)│   │ STANDBY)│                │
│  └─────────┘   └─────────┘   └─────────┘                │
│       │                                                 │
│       ▼                                                 │
│  ┌─────────────────────────────────────────┐            │
│  │      Shared State (S3 + DynamoDB)        │            │
│  │      - Queue state in S3                │            │
│  │      - Leader election in DynamoDB       │            │
│  └─────────────────────────────────────────┘            │
│                                                             │
└─────────────────────────────────────────────────────────────┘

Leader Election:
- Use DynamoDB conditional writes for leader lease
- Lease expires after 30 seconds (no heartbeat = new leader)
- New leader recovers state from S3
```

### Job Redelivery (At-Least-Once)

```rust
/// Background task: Requeue stalled jobs
async fn requeue_stalled_jobs(self: Arc<Self>) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let mut queue = self.queue.write().await;
        let now = chrono::now();

        // Find jobs that have been inflight too long
        let stale_threshold = Duration::from_secs(300); // 5 minutes

        let mut to_requeue = Vec::new();
        for (job_id, inflight) in &queue.inflight {
            if now - inflight.claimed_at > stale_threshold {
                to_requeue.push(job_id.clone());
            }
        }

        // Requeue stale jobs
        for job_id in &to_requeue {
            if let Some(job) = queue.jobs.iter_mut().find(|j| j.id == *job_id) {
                job.status = JobStatus::Pending;
                job.claimed_by = None;
                job.inflight_since = None;
                job.retry_count += 1;
            }
            queue.inflight.remove(job_id);
        }

        if !to_requeue.is_empty() {
            println!("Requeued {} stalled jobs", to_requeue.len());
        }
    }
}
```

### Idempotent Execution

```
Even with at-least-once delivery, we need exactly-once SEMANTICS.

Solution: Idempotent job execution

{
  "job_id": "index-namespace-123",
  "payload": {
    "namespace_id": "customer-123",
    "operation": "build_index"
  },
  "dedup_key": "customer-123:build_index:2024-01-15"
}

Worker checks dedup_key before executing:
- If result exists for dedup_key, return cached result
- If no result, execute and store result with dedup_key

This ensures jobs are effectually executed exactly once,
even if delivered multiple times.
```

---

## Performance Results

### Latency Comparison

```
┌─────────────────────────────────────────────────────────────┐
│              Job Queue Latency (p99)                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Sharded Queue (Old)                                        │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  ~500ms  │
│                                                             │
│  Single File + Group Commit                                 │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  ~150ms  │
│                                                             │
│  Broker Architecture (Current)                              │
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓  ~50ms                               │
│                                                             │
│  Improvement: 10x lower tail latency                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Throughput

```
┌─────────────────────────────────────────────────────────────┐
│              Jobs Per Second                                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Sharded Queue:        ~50 jobs/sec (limited by slow node) │
│  Group Commit:         ~500 jobs/sec                        │
│  Broker + Batching:    ~5000 jobs/sec                       │
│                                                             │
│  Current production load: ~500 jobs/sec                    │
│  Headroom: 10x                                              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Reliability

```
Before (Sharded Queue):
- Weekly incidents: 3-5
- Common causes: Slow nodes, shard imbalance
- MTTR: 30-60 minutes (manual rebalancing)

After (Broker Queue):
- Weekly incidents: 0-1
- Common causes: S3 outage (rare)
- MTTR: 5 minutes (automatic failover)
```

---

## Tradeoffs and Lessons Learned

### What Worked Well

```
1. Object Storage as Primary Store
   - Durable, scalable, predictable cost
   - No database to manage
   - Easy backups (just copy the file)

2. Stateless Broker
   - Easy to scale (add more brokers)
   - No state to migrate on failover
   - Recovery from S3 is fast (~100ms)

3. Group Commit
   - Dramatically improved throughput
   - Also reduced S3 costs (fewer PUTs)
```

### What Didn't Work

```
Attempt 1: Sharding by Namespace
Problem: Hot shards (some namespaces have 1000x more writes)
Solution: Single global queue

Attempt 2: In-Memory Queue with Periodic Flush
Problem: Lost jobs on broker crash
Solution: S3 as source of truth + immediate durability

Attempt 3: Multiple Brokers Without Coordination
Problem: Race conditions, duplicate job delivery
Solution: Leader election for single writer
```

### Design Principles

1. **Start simple, add complexity as needed**: queue.json worked for surprising low traffic

2. **Object storage is underrated**: For many use cases, it's the right primitive

3. **Stateless is easier**: Brokers can crash without data loss

4. **Measure before optimizing**: Sharding seemed right, but wasn't the bottleneck

---

## Summary

### Key Takeaways

1. **Single queue file works**: For moderate traffic, a single file with CAS is sufficient

2. **Group commit enables throughput**: Batch writes to overcome object storage latency

3. **Stateless broker eliminates contention**: Single writer, many readers

4. **At-least-once + idempotency = exactly-once semantics**: Don't fight distributed systems, work with them

5. **Object storage is a great primitive**: Durable, scalable, cost-effective

### When to Use This Pattern

```
Good fit:
- Job queues (indexing, notifications, batch processing)
- Event logs
- Configuration distribution
- Coordination for distributed systems

Not a good fit:
- High-frequency trading (<1ms latency)
- Real-time messaging (<100ms latency)
- High-contention counters (use DynamoDB/Redis)
```

### Future Improvements

- **Regional queues**: Deploy broker per region for lower latency
- **Priority queues**: Support job priorities
- **Dead letter queue**: Handle permanently failing jobs
- **Metrics and observability**: Better monitoring of queue health

---

## Appendix: Complete Broker Code

```rust
use std::sync::Arc;
use tokio::sync::{RwLock, Notify};
use std::collections::{VecDeque, HashMap};

pub struct Broker {
    queue: RwLock<QueueState>,
    job_available: Notify,
    shutdown: AtomicBool,
}

struct QueueState {
    jobs: VecDeque<JobEntry>,
    version: u64,
    inflight: HashMap<String, InFlightJob>,
}

struct JobEntry {
    id: String,
    status: JobStatus,
    payload: Job,
    claimed_by: Option<String>,
    inflight_since: Option<DateTime<Utc>>,
    retry_count: u32,
    dedup_key: String,
}

enum JobStatus {
    Pending,
    InFlight,
    Done,
}

struct InFlightJob {
    worker_id: String,
    claimed_at: DateTime<Utc>,
}

impl Broker {
    pub async fn new() -> Result<Arc<Self>> {
        let broker = Arc::new(Broker {
            queue: RwLock::new(QueueState {
                jobs: VecDeque::new(),
                version: 0,
                inflight: HashMap::new(),
            }),
            job_available: Notify::new(),
            shutdown: AtomicBool::new(false),
        });

        // Recover from S3
        broker.recover_from_s3().await?;

        // Start background tasks
        let broker_clone = broker.clone();
        tokio::spawn(async move {
            broker_clone.persist_loop().await;
        });

        let broker_clone = broker.clone();
        tokio::spawn(async move {
            broker_clone.requeue_stalled_jobs().await;
        });

        Ok(broker)
    }

    pub async fn push_job(&self, job: Job) -> Result<String> {
        let job_id = generate_id();
        let dedup_key = job.dedup_key.clone();

        let mut queue = self.queue.write().await;
        queue.jobs.push_back(JobEntry {
            id: job_id.clone(),
            status: JobStatus::Pending,
            payload: job,
            claimed_by: None,
            inflight_since: None,
            retry_count: 0,
            dedup_key,
        });
        queue.version += 1;

        self.job_available.notify_one();

        Ok(job_id)
    }

    pub async fn claim_job(
        &self,
        worker_id: &str,
        timeout: Duration,
    ) -> Result<Option<Job>> {
        tokio::time::timeout(timeout, async {
            loop {
                {
                    let mut queue = self.queue.write().await;

                    if let Some(job) = queue.jobs.iter_mut().find(|j| {
                        j.status == JobStatus::Pending
                    }) {
                        job.status = JobStatus::InFlight;
                        job.inflight_since = Some(chrono::now());
                        job.claimed_by = Some(worker_id.to_string());

                        queue.inflight.insert(
                            job.id.clone(),
                            InFlightJob {
                                worker_id: worker_id.to_string(),
                                claimed_at: chrono::now(),
                            }
                        );

                        return Some(job.payload.clone());
                    }
                }

                // No job available, wait for notification
                self.job_available.notified().await;
            }
        }).await.ok().flatten()
    }

    pub async fn ack_job(&self, job_id: &str) -> Result<()> {
        let mut queue = self.queue.write().await;

        if let Some(job) = queue.jobs.iter_mut().find(|j| j.id == job_id) {
            job.status = JobStatus::Done;
        }
        queue.inflight.remove(job_id);
        queue.version += 1;

        Ok(())
    }

    async fn persist_loop(self: Arc<Self>) {
        while !self.shutdown.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let queue = self.queue.read().await;
            if let Err(e) = s3_put("indexing-queue.json", &*queue).await {
                eprintln!("Failed to persist queue: {}", e);
            }
        }
    }

    async fn requeue_stalled_jobs(self: Arc<Self>) {
        while !self.shutdown.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let mut queue = self.queue.write().await;
            let now = chrono::now();
            let threshold = Duration::from_secs(300);

            let stale_jobs: Vec<_> = queue.inflight.iter()
                .filter(|(_, job)| now - job.claimed_at > threshold)
                .map(|(id, _)| id.clone())
                .collect();

            for job_id in &stale_jobs {
                if let Some(job) = queue.jobs.iter_mut().find(|j| j.id == *job_id) {
                    job.status = JobStatus::Pending;
                    job.claimed_by = None;
                    job.inflight_since = None;
                    job.retry_count += 1;
                }
                queue.inflight.remove(job_id);
            }

            if !stale_jobs.is_empty() {
                println!("Requeued {} stale jobs", stale_jobs.len());
                self.job_available.notify_one();
            }
        }
    }

    async fn recover_from_s3(&self) -> Result<()> {
        match s3_get::<QueueState>("indexing-queue.json").await {
            Ok(queue) => {
                *self.queue.write().await = queue;
                println!("Recovered {} jobs from S3", self.queue.read().await.jobs.len());
            }
            Err(e) => {
                println!("No existing queue found: {}", e);
            }
        }
        Ok(())
    }
}
```
