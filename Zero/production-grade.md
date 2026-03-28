---
title: "Zero Production-Grade Implementation Guide"
subtitle: "Performance optimizations, monitoring, and deployment strategies for production Zero systems"
---

# Zero Production-Grade Implementation Guide

## 1. Overview

This guide covers production considerations for deploying Zero-based sync systems:

- Performance optimizations
- Memory management
- Batching and throughput
- Serving infrastructure
- Monitoring and observability
- Scaling strategies

## 2. Performance Optimizations

### 2.1 Query Optimization

#### Index Selection

```typescript
// Zero schema with indexes
const schema = createSchema({
  version: 1,
  tables: {
    issue: table('issue')
      .columns({
        id: 'string',
        projectId: 'string',
        status: 'string',
        priority: 'number',
        created: 'number',
      })
      .primaryKey('id')
      .indexes([
        { name: 'by_project', columns: ['projectId'] },
        { name: 'by_status', columns: ['status'] },
        { name: 'by_project_status', columns: ['projectId', 'status'] },
      ]),
  },
});
```

**Index selection strategy:**

| Query Pattern | Recommended Index |
|---------------|-------------------|
| `WHERE projectId = ?` | `by_project` |
| `WHERE projectId = ? AND status = ?` | `by_project_status` (composite) |
| `WHERE status = ? ORDER BY created` | `by_status` + filesort |

#### Query Plan Analysis

```typescript
// Enable query plan debugging
const query = zero.query.issue
  .where('projectId', 'abc123')
  .where('status', 'open');

// Analyze the query plan
const plan = await query.analyze();
console.log(plan);

// Output:
// {
//   operators: [
//     { type: 'scan', table: 'issue', estimatedRows: 10000 },
//     { type: 'filter', condition: 'projectId = ?', selectivity: 0.1 },
//     { type: 'filter', condition: 'status = ?', selectivity: 0.3 },
//   ],
//   estimatedOutputRows: 300,
//   estimatedCost: 1300,
// }
```

### 2.2 IVM Optimization

#### Operator Fusion

```rust
// Rust: Fuse filter + projection into single operator
pub struct FilterProjectOperator {
    filter: FilterOperator,
    projection: ProjectOperator,
}

impl Operator for FilterProjectOperator {
    fn apply(&self, change: &Change) -> Vec<Change> {
        // Apply filter first
        let filtered = self.filter.apply(change);

        // Then project (avoid intermediate allocations)
        filtered.into_iter()
            .map(|c| self.project(c))
            .collect()
    }
}

// Benefits:
// - Single pass through data
// - No intermediate Change allocations
// - Better cache locality
```

#### Batched Change Processing

```rust
// Rust: Process changes in batches
pub struct BatchedIVMPipeline {
    operators: Vec<Box<dyn Operator>>,
    batch_size: usize,
}

impl BatchedIVMPipeline {
    pub fn process_batch(&self, changes: &[Change]) -> Vec<Change> {
        let mut current_batch: Vec<Change> = changes.to_vec();

        for operator in &self.operators {
            let mut next_batch = Vec::with_capacity(current_batch.len());

            // Process entire batch through operator
            for change in current_batch {
                next_batch.extend(operator.apply(&change));
            }

            current_batch = next_batch;
        }

        current_batch
    }
}
```

### 2.3 Memory Optimization

#### Object Pooling

```rust
// Rust: Pool for Change objects
use object_pool::Pool;

pub struct ChangePool {
    pool: Pool<Change>,
}

impl ChangePool {
    pub fn new(size: usize) -> Self {
        let pool = Pool::new(size, || Change::Add(AddChange {
            relation: String::new(),
            node: Node::empty(),
        }));

        Self { pool }
    }

    pub fn acquire(&self) -> PooledObject<Change> {
        self.pool.acquire()
    }
}

// Usage:
let mut change = pool.acquire();
change.relation = "issue".to_string();
// ... populate change
// Automatically returned to pool when dropped
```

#### Arena Allocation for Nodes

```rust
// Rust: Arena allocation for batch processing
use typed_arena::Arena;

pub struct PipelineArena {
    arena: Arena<Node>,
    batch_size: usize,
}

impl PipelineArena {
    pub fn with_capacity(batch_size: usize) -> Self {
        Self {
            arena: Arena::with_capacity(batch_size * size_of::<Node>()),
            batch_size,
        }
    }

    pub fn alloc(&self, row: Row) -> &Node {
        self.arena.alloc(Node {
            row,
            relationships: HashMap::new(),
        })
    }

    pub fn reset(&mut self) {
        // Clear all allocations at once (O(1))
        self.arena.reset();
    }
}
```

## 3. Batching and Throughput

### 3.1 Change Batching

```rust
// Rust: Adaptive change batcher
pub struct AdaptiveChangeBatcher {
    batch: Vec<Change>,
    max_size: usize,
    max_latency_ms: u64,
    timer: Option<Instant>,
    current_throughput: f64, // changes per second
}

impl AdaptiveChangeBatcher {
    pub fn add(&mut self, change: Change) -> Option<Vec<Change>> {
        self.batch.push(change);
        self.current_throughput = self.calculate_throughput();

        // Adaptive flush decision
        if self.should_flush() {
            return Some(self.flush());
        }

        None
    }

    fn should_flush(&self) -> bool {
        // Flush if batch is full
        if self.batch.len() >= self.max_size {
            return true;
        }

        // Flush if latency threshold exceeded
        if let Some(timer) = self.timer {
            if timer.elapsed().as_millis() as u64 >= self.max_latency_ms {
                return true;
            }
        }

        // Flush if throughput is low (no benefit to waiting)
        if self.current_throughput < 100.0 {
            return true;
        }

        false
    }

    pub fn flush(&mut self) -> Vec<Change> {
        self.timer = Some(Instant::now());
        std::mem::take(&mut self.batch)
    }
}
```

### 3.2 Write Coalescing

```rust
// Rust: Coalesce multiple mutations to same row
pub struct MutationCoalescer {
    pending: HashMap<String, Vec<Mutation>>,
    timer: Option<Instant>,
    coalesce_window_ms: u64,
}

impl MutationCoalescer {
    pub fn add(&mut self, mutation: Mutation) {
        let key = format!("{}:{}", mutation.table, mutation.row_id);
        self.pending.entry(key).or_insert_with(Vec::new).push(mutation);
    }

    pub fn flush(&mut self) -> Vec<Mutation> {
        let mut result = Vec::new();

        for (_, mutations) in self.pending.drain() {
            if mutations.len() == 1 {
                result.push(mutations.into_iter().next().unwrap());
            } else {
                // Coalesce multiple mutations into one
                let coalesced = self.coalesce_mutations(mutations);
                result.push(coalesced);
            }
        }

        result
    }

    fn coalesce_mutations(&self, mutations: Vec<Mutation>) -> Mutation {
        // Last-write-wins for simple updates
        mutations.into_iter().last().unwrap()
    }
}
```

### 3.3 Read Batching

```rust
// Rust: Batch multiple query initializations
pub struct QueryInitBatcher {
    pending_queries: Vec<QueryInitRequest>,
    batch_size: usize,
}

impl QueryInitBatcher {
    pub fn add(&mut self, request: QueryInitRequest) -> Option<Vec<QueryInitResult>> {
        self.pending_queries.push(request);

        if self.pending_queries.len() >= self.batch_size {
            return Some(self.flush());
        }

        None
    }

    pub fn flush(&mut self) -> Vec<QueryInitResult> {
        let queries = std::mem::take(&mut self.pending_queries);

        // Execute all queries in single transaction
        let results = self.execute_batch(queries);

        results
    }

    fn execute_batch(&self, queries: Vec<QueryInitRequest>) -> Vec<QueryInitResult> {
        // Group by table for efficient scanning
        let by_table = self.group_by_table(&queries);

        let mut results = Vec::new();

        for (table, table_queries) in by_table {
            // Single scan for all queries on this table
            let rows = self.scan_table(&table);

            for query in table_queries {
                let filtered = self.apply_filters(&rows, &query.filters);
                results.push(QueryInitResult {
                    query_id: query.id,
                    rows: filtered,
                });
            }
        }

        results
    }
}
```

## 4. Serving Infrastructure

### 4.1 Connection Management

```rust
// Rust: Connection pool with limits
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct ConnectionManager {
    max_connections: usize,
    current_connections: AtomicUsize,
    waiting_queue: VecDeque<ConnectionRequest>,
}

impl ConnectionManager {
    pub fn accept(&self) -> Result<ConnectionHandle, ConnectionError> {
        let current = self.current_connections.load(Ordering::Relaxed);

        if current >= self.max_connections {
            // Queue the request
            return Err(ConnectionError::QueueFull);
        }

        self.current_connections.fetch_add(1, Ordering::Relaxed);
        Ok(ConnectionHandle::new(self.current_connections.clone()))
    }

    pub fn release(&self) {
        self.current_connections.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct ConnectionHandle {
    counter: Arc<AtomicUsize>,
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}
```

### 4.2 Load Balancing

```
┌─────────────────────────────────────────────────────────┐
│                    Load Balancer                        │
│              (nginx, HAProxy, ALB)                      │
└─────────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│  Zero Cache │ │  Zero Cache │ │  Zero Cache │
│   Server 1  │ │   Server 2  │ │   Server 3  │
│   (us-east) │ │   (us-west) │ │   (eu-west) │
└─────────────┘ └─────────────┘ └─────────────┘
```

#### Sticky Sessions for WebSocket

```yaml
# nginx configuration for WebSocket sticky sessions
upstream zero_cache {
    least_conn;
    server cache1.example.com:8080;
    server cache2.example.com:8080;
    server cache3.example.com:8080;
}

server {
    location /ws {
        proxy_pass http://zero_cache;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Sticky sessions based on connection_id
        proxy_set_header X-Real-IP $remote_addr;
        sticky_cookie srv_id expires=1h path=/;
    }
}
```

### 4.3 Health Checks

```rust
// Rust: Comprehensive health check
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String, // "healthy", "degraded", "unhealthy"
    pub version: String,
    pub checks: HashMap<String, CheckStatus>,
}

#[derive(Debug, Serialize)]
pub struct CheckStatus {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

pub async fn health_check() -> HealthStatus {
    let mut checks = HashMap::new();

    // Check PostgreSQL connection
    checks.insert("postgres".to_string(), check_postgres().await);

    // Check change source
    checks.insert("change_source".to_string(), check_change_source().await);

    // Check memory usage
    checks.insert("memory".to_string(), check_memory().await);

    // Determine overall status
    let status = if checks.values().all(|c| c.status == "healthy") {
        "healthy"
    } else if checks.values().any(|c| c.status == "unhealthy") {
        "unhealthy"
    } else {
        "degraded"
    };

    HealthStatus {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks,
    }
}
```

## 5. Monitoring and Observability

### 5.1 Metrics Collection

```rust
// Rust: OpenTelemetry metrics integration
use opentelemetry::{global, metrics::Counter};

pub struct ZeroMetrics {
    changes_processed: Counter<u64>,
    query_latency: Histogram<f64>,
    active_connections: Gauge<u64>,
    mutation_errors: Counter<u64>,
}

impl ZeroMetrics {
    pub fn new() -> Self {
        let meter = global::meter("zero-cache");

        Self {
            changes_processed: meter.u64_counter("changes_processed").init(),
            query_latency: meter.f64_histogram("query_latency_ms").init(),
            active_connections: meter.u64_gauge("active_connections").init(),
            mutation_errors: meter.u64_counter("mutation_errors").init(),
        }
    }

    pub fn record_change(&self, count: u64) {
        self.changes_processed.add(count, &[]);
    }

    pub fn record_query_latency(&self, latency_ms: f64) {
        self.query_latency.record(latency_ms, &[]);
    }

    pub fn set_active_connections(&self, count: u64) {
        self.active_connections.record(count, &[]);
    }

    pub fn record_mutation_error(&self) {
        self.mutation_errors.add(1, &[]);
    }
}
```

### 5.2 Distributed Tracing

```rust
// Rust: OpenTelemetry tracing
use opentelemetry::trace::{Span, Tracer};
use opentelemetry::Context;

pub async fn process_mutation(
    cx: Context,
    mutation: Mutation,
) -> Result<MutationResult, SyncError> {
    let tracer = global::tracer("zero-cache");

    let span = tracer
        .span_builder("process_mutation")
        .start_with_context(&tracer, &cx);

    let mut cx = Context::current_with_span(span);

    // Add attributes
    cx.span().set_attribute(Key::new("mutation.type").string(mutation.r#type.clone()));
    cx.span().set_attribute(Key::new("mutation.table").string(mutation.table.clone()));

    // Process mutation
    let result = self.apply_mutation(mutation).await;

    // Record result
    match &result {
        Ok(r) => {
            cx.span().set_attribute(Key::new("mutation.success").bool(true));
            cx.span().set_attribute(Key::new("mutation.result").string(r.to_string()));
        }
        Err(e) => {
            cx.span().set_attribute(Key::new("mutation.success").bool(false));
            cx.span().record_error(e);
        }
    }

    result
}
```

### 5.3 Logging

```rust
// Rust: Structured logging with tracing
use tracing::{info, warn, error, instrument};

#[instrument(skip(self, changes), fields(change_count = changes.len()))]
pub fn apply_changes(&self, changes: &[Change]) {
    info!("Applying batch of changes");

    for change in changes {
        match change {
            Change::Add(add) => {
                debug!(relation = %add.relation, "Adding row");
            }
            Change::Remove(remove) => {
                warn!(relation = %remove.relation, "Removing row");
            }
            Change::Edit(edit) => {
                debug!(relation = %edit.relation, "Editing row");
            }
        }
    }
}
```

### 5.4 Alerting Rules

```yaml
# Prometheus alerting rules
groups:
  - name: zero-cache
    rules:
      - alert: HighMutationErrorRate
        expr: rate(mutation_errors_total[5m]) > 0.01
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High mutation error rate"
          description: "Mutation error rate is {{ $value }}% over 5 minutes"

      - alert: HighQueryLatency
        expr: histogram_quantile(0.99, query_latency_ms_bucket) > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High query latency"
          description: "99th percentile query latency is {{ $value }}ms"

      - alert: ConnectionPoolExhausted
        expr: active_connections / max_connections > 0.9
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Connection pool nearly exhausted"
          description: "{{ $value | humanizePercentage }} of connections in use"

      - alert: ChangeSourceDisconnected
        expr: change_source_connected == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Change source disconnected"
          description: "Zero cache is not receiving changes from PostgreSQL"
```

## 6. Scaling Strategies

### 6.1 Horizontal Scaling

```
┌─────────────────────────────────────────────────────────┐
│                   Global Load Balancer                   │
└─────────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   Region: US    │ │  Region: EU     │ │  Region: AP     │
│                 │ │                 │ │                 │
│  ┌───────────┐  │ │  ┌───────────┐  │ │  ┌───────────┐  │
│  │ Zero Cache│  │ │  │ Zero Cache│  │ │  │ Zero Cache│  │
│  └───────────┘  │ │  └───────────┘  │ │  └───────────┘  │
│        │        │ │        │        │ │        │        │
│  ┌───────────┐  │ │  ┌───────────┐  │ │  ┌───────────┐  │
│  │ PostgreSQL│  │ │  │ PostgreSQL│  │ │  │ PostgreSQL│  │
│  │ (replica) │  │ │  │ (replica) │  │ │  │ (replica) │  │
│  └───────────┘  │ │  └───────────┘  │ │  └───────────┘  │
└─────────────────┘ └─────────────────┘ └─────────────────┘
         │              │              │
         └──────────────┼──────────────┘
                        │
                        ▼
               ┌─────────────────┐
               │  PostgreSQL     │
               │   (Primary)     │
               │   (us-east)     │
               └─────────────────┘
```

### 6.2 Sharding Strategies

```rust
// Rust: Query-based sharding
pub struct ShardRouter {
    shards: HashMap<String, ShardConnection>,
}

impl ShardRouter {
    pub fn route(&self, query: &Query) -> &ShardConnection {
        // Route by project ID
        let project_id = query.get_project_id();
        let shard_id = self.hash_shard(&project_id);
        &self.shards[&shard_id]
    }

    fn hash_shard(&self, key: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let result = hasher.finalize();

        // Use first 4 bytes for shard selection
        let shard_num = u32::from_be_bytes([result[0], result[1], result[2], result[3]])
            % self.shards.len() as u32;

        format!("shard_{}", shard_num)
    }
}
```

### 6.3 Read Replicas

```rust
// Rust: Read/write splitting
pub struct DatabaseRouter {
    primary: PgConnection,
    replicas: Vec<PgConnection>,
    replica_index: AtomicUsize,
}

impl DatabaseRouter {
    pub fn get_for_read(&self) -> &PgConnection {
        // Round-robin across replicas
        let idx = self.replica_index.fetch_add(1, Ordering::Relaxed);
        &self.replicas[idx % self.replicas.len()]
    }

    pub fn get_for_write(&self) -> &PgConnection {
        &self.primary
    }
}
```

## 7. Deployment Checklist

### 7.1 Pre-Deployment

- [ ] Schema versioning strategy defined
- [ ] Migration rollback plan
- [ ] Connection limits configured
- [ ] Rate limits configured
- [ ] Monitoring dashboards created
- [ ] Alert rules configured
- [ ] Runbook for common issues documented

### 7.2 Deployment

- [ ] Blue/green or canary deployment
- [ ] Health checks passing
- [ ] Metrics flowing
- [ ] Logs visible
- [ ] Traces captured

### 7.3 Post-Deployment

- [ ] Error rates within SLA
- [ ] Latency within SLA
- [ ] No connection leaks
- [ ] Memory usage stable
- [ ] Change lag acceptable

## 8. SLA Considerations

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Availability** | 99.9% | Uptime / Total time |
| **Change Latency** | < 500ms (p99) | DB change → Client receive |
| **Mutation Latency** | < 200ms (p99) | Client send → Client confirm |
| **Query Latency** | < 100ms (p99) | Query subscribe → Initial data |
| **Connection Recovery** | < 5s | Disconnect → Reconnected |

---

*This completes the Zero exploration. See [exploration.md](exploration.md) for the full index.*
