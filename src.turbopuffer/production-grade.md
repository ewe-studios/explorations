# Production-Grade Considerations

## What It Takes to Run Vector Search in Production

This document covers the operational aspects of running a production-grade vector search system like Turbopuffer.

---

## Table of Contents

1. [System Architecture](#system-architecture)
2. [Reliability Patterns](#reliability-patterns)
3. [Observability](#observability)
4. [Scaling Strategies](#scaling-strategies)
5. [Data Durability](#data-durability)
6. [Security](#security)
7. [Cost Optimization](#cost-optimization)
8. [Incident Response](#incident-response)

---

## System Architecture

### Production Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      Global Load Balancer                       │
│                    (Cloud Load Balancing / DNS)                 │
└───────────────────────────┬─────────────────────────────────────┘
                            │
         ┌──────────────────┼──────────────────┐
         │                  │                  │
         ▼                  ▼                  ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│   Region: US    │ │  Region: EU     │ │  Region: APAC   │
│   us-central1   │ │  europe-west1   │ │  asia-southeast1│
├─────────────────┤ ├─────────────────┤ ├─────────────────┤
│ ┌─────────────┐ │ │ ┌─────────────┐ │ │ ┌─────────────┐ │
│ │ API Gateway │ │ │ │ API Gateway │ │ │ │ API Gateway │ │
│ └──────┬──────┘ │ │ └──────┬──────┘ │ │ └──────┬──────┘ │
│        │        │ │        │        │ │        │        │
│ ┌──────▼──────┐ │ │ ┌──────▼──────┐ │ │ ┌──────▼──────┐ │
│ │Query Service│ │ │ │Query Service│ │ │ │Query Service│ │
│ └──────┬──────┘ │ │ └──────┬──────┘ │ │ └──────┬──────┘ │
│        │        │ │        │        │ │        │        │
│ ┌──────▼──────┐ │ │ ┌──────▼──────┐ │ │ ┌──────▼──────┐ │
│ │Vector Store │ │ │ │Vector Store │ │ │ │Vector Store │ │
│ │   (SSD)     │ │ │ │   (SSD)     │ │ │ │   (SSD)     │ │
│ └─────────────┘ │ │ └─────────────┘ │ │ └─────────────┘ │
└─────────────────┘ └─────────────────┘ └─────────────────┘
         │                  │                  │
         └──────────────────┼──────────────────┘
                            │
                            ▼
                 ┌─────────────────────┐
                 │  Object Storage     │
                 │  (Cross-region repl)│
                 └─────────────────────┘
```

### Service Components

**1. API Gateway:**
- Authentication and authorization
- Rate limiting per API key
- Request validation
- Response caching (for repeated queries)

**2. Query Service:**
- Stateless query processing
- Horizontal scaling based on load
- Circuit breakers for downstream protection

**3. Vector Store:**
- Stateful storage layer
- SSD-backed for low latency
- Replication for durability

**4. Background Services:**
- Index compaction
- Garbage collection
- Metrics aggregation
- Backup creation

---

## Reliability Patterns

### 1. Circuit Breakers

```rust
use circuit_breaker::CircuitBreaker;
use std::time::Duration;

struct QueryCircuitBreaker {
    cb: CircuitBreaker<QueryError>,
}

impl QueryCircuitBreaker {
    fn new() -> Self {
        Self {
            cb: CircuitBreaker::builder()
                .failure_threshold(5)      // Trip after 5 failures
                .success_threshold(2)      // Reset after 2 successes
                .timeout(Duration::from_secs(30))  // Half-open after 30s
                .build(),
        }
    }

    async fn query(&mut self, request: QueryRequest) -> Result<QueryResult, QueryError> {
        self.cb.call(async {
            // Execute query with timeout
            tokio::time::timeout(
                Duration::from_millis(100),
                execute_query(request)
            ).await??
        }).await
    }
}
```

### 2. Retry with Backoff

```rust
use retry::{retry_with_delay, Error};
use std::time::Duration;

async fn query_with_retry(
    engine: &QueryEngine,
    request: QueryRequest,
) -> Result<QueryResult> {
    retry_with_delay(
        || async {
            engine.query(request.clone()).await
                .map_err(|e| Error::Transient(e))?
        },
        ExponentialBackoff::builder()
            .with_min_delay(Duration::from_millis(100))
            .with_max_delay(Duration::from_secs(5))
            .with_max_retries(3)
            .build(),
    ).await
}
```

### 3. Rate Limiting

```rust
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;

struct RateLimitedQueryService {
    // Per-client rate limiters
    limiters: DashMap<String, RateLimiter>,
    default_quota: Quota,
}

impl RateLimitedQueryService {
    fn new() -> Self {
        Self {
            limiters: DashMap::new(),
            default_quota: Quota::per_second(NonZeroU32::new(100).unwrap()),
        }
    }

    fn get_limiter(&self, api_key: &str) -> RateLimiter {
        self.limiters
            .entry(api_key.to_string())
            .or_insert_with(|| {
                RateLimiter::direct(self.default_quota)
            })
            .clone()
    }

    async fn query(&self, api_key: &str, request: QueryRequest) -> Result<QueryResult> {
        let limiter = self.get_limiter(api_key);

        // Wait for rate limit token
        limiter.until_ready().await;

        self.engine.query(request).await
    }
}
```

### 4. Load Shedding

```rust
use tokio::sync::Semaphore;

struct LoadSheddingService {
    semaphore: Semaphore,
    max_concurrent: usize,
}

impl LoadSheddingService {
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Semaphore::new(max_concurrent),
            max_concurrent,
        }
    }

    async fn query(&self, request: QueryRequest) -> Result<QueryResult, RejectedError> {
        let _permit = self.semaphore
            .try_acquire()
            .map_err(|_| RejectedError)?;

        // Check current load
        let current_load = self.max_concurrent - self.semaphore.available_permits();
        if current_load > self.max_concurrent * 90 / 100 {
            // At 90% capacity, start increasing latency artificially
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        self.engine.query(request).await
    }
}
```

### 5. Bulkheads

```rust
use tokio::task::JoinSet;

struct BulkheadService {
    // Separate thread pools for different priorities
    high_priority_pool: ThreadPool,
    low_priority_pool: ThreadPool,
}

impl BulkheadService {
    async fn query_high_priority(&self, request: QueryRequest) -> Result<QueryResult> {
        self.high_priority_pool
            .spawn(async move { execute_query(request).await })
            .await?
    }

    async fn query_low_priority(&self, request: QueryRequest) -> Result<QueryResult> {
        self.low_priority_pool
            .spawn(async move { execute_query(request).await })
            .await?
    }
}
```

---

## Observability

### Metrics to Track

**1. Latency Metrics:**
```rust
use metrics::{histogram, gauge};

// Query latency histogram
histogram!("query_latency_seconds")
    .record(query_start.elapsed().as_secs_f64());

// Latency by percentile
let p50 = latency_snapshot.percentile(50.0);
let p99 = latency_snapshot.percentile(99.0);
gauge!("query_latency_p50_seconds").set(p50);
gauge!("query_latency_p99_seconds").set(p99);
```

**2. Throughput Metrics:**
```rust
use metrics::counter;

// Requests per second
counter!("requests_total").increment(1);
counter!("requests_successful").increment(1);
counter!("requests_failed").increment(1);

// Vectors scanned per query
histogram!("vectors_scanned_per_query").record(result.total_scanned as f64);
```

**3. Resource Metrics:**
```rust
// Memory usage
gauge!("memory_used_bytes").set(get_memory_usage() as f64);

// Cache hit rates
let hit_rate = cache.hits() as f64 / (cache.hits() + cache.misses()) as f64;
gauge!("cache_hit_ratio").set(hit_rate);

// Connection pool usage
gauge!("connection_pool_used").set(pool.used_count() as f64);
gauge!("connection_pool_available").set(pool.available_count() as f64);
```

### Distributed Tracing

```rust
use tracing::{instrument, info, warn, error};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[instrument(
    skip(self, request),
    fields(
        query_id = %uuid::Uuid::new_v4(),
        api_key = %request.api_key,
        top_k = request.top_k,
    )
)]
async fn query(&self, request: QueryRequest) -> Result<QueryResult> {
    let start = Instant::now();

    // Add custom attributes to span
    Span::current().record("dimension", request.vector.len());

    info!("Starting query");

    let result = match self.engine.query(request).await {
        Ok(result) => {
            counter!("queries_successful").increment(1);
            result
        }
        Err(e) => {
            error!(error = %e, "Query failed");
            counter!("queries_failed").increment(1);
            return Err(e);
        }
    };

    // Record timing
    let duration = start.elapsed();
    histogram!("query_latency_seconds").record(duration.as_secs_f64());

    // Warn on slow queries
    if duration > Duration::from_millis(100) {
        warn!(latency_ms = duration.as_millis(), "Slow query detected");
    }

    Ok(result)
}
```

### Alerting Rules

```yaml
# Prometheus alerting rules
groups:
  - name: vector_search
    rules:
      - alert: HighQueryLatency
        expr: histogram_quantile(0.99, rate(query_latency_seconds_bucket[5m])) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "P99 query latency above 100ms"

      - alert: HighErrorRate
        expr: rate(queries_failed_total[5m]) / rate(queries_total[5m]) > 0.01
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Query error rate above 1%"

      - alert: LowCacheHitRate
        expr: cache_hit_ratio < 0.8
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Cache hit ratio below 80%"

      - alert: DiskSpaceLow
        expr: node_filesystem_avail_bytes / node_filesystem_size_bytes < 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Less than 10% disk space remaining"
```

---

## Scaling Strategies

### Horizontal Scaling

**Query Service (Stateless):**
```yaml
# Kubernetes HPA configuration
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: query-service
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: query-service
  minReplicas: 3
  maxReplicas: 100
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Pods
      pods:
        metric:
          name: queries_per_second
        target:
          type: AverageValue
          averageValue: "1000"
```

### Vertical Scaling

**Vector Store (Stateful):**
- SSD size determines max vectors per node
- Scale vertically (larger instances) for more capacity
- Partition data across nodes for horizontal scale

### Read Replicas

```
Primary Node (Read-Write)
│
├── Replica 1 (Read-Only) ──▶ Serve 50% of queries
├── Replica 2 (Read-Only) ──▶ Serve 50% of queries
└── Replica 3 (Read-Only) ──▶ Standby
```

---

## Data Durability

### Replication Strategy

**Synchronous Replication (within region):**
```
Write Operation:
1. Write to Primary
2. Replicate to 2 followers (quorum = 2)
3. Acknowledge to client

Read Operation:
1. Read from any replica
2. Verify consistency with checksum
```

**Asynchronous Replication (cross-region):**
```
Region A (Primary) ──async──▶ Region B (DR)
       │
       └─────────────────────▶ Region C (DR)

RPO: < 1 minute
RTO: < 5 minutes
```

### Backup Strategy

```yaml
Backup Schedule:
  - Full backup: Daily at 02:00 UTC
  - Incremental: Every 15 minutes
  - WAL archiving: Continuous

Retention:
  - Daily backups: 30 days
  - Weekly backups: 12 weeks
  - Monthly backups: 12 months

Storage:
  - Primary: S3 Standard
  - Archive: S3 Glacier (after 30 days)
```

### Recovery Procedures

```rust
async fn restore_from_backup(
    backup_id: &str,
    target_path: &str,
) -> Result<()> {
    info!("Starting restore from backup {}", backup_id);

    // 1. Download backup metadata
    let metadata = download_backup_metadata(backup_id).await?;

    // 2. Restore data files
    download_backup_data(backup_id, target_path).await?;

    // 3. Replay WAL entries after backup
    replay_wal_entries(backup_id, target_path).await?;

    // 4. Verify integrity
    verify_integrity(target_path).await?;

    info!("Restore completed successfully");
    Ok(())
}
```

---

## Security

### Authentication

```rust
use jsonwebtoken::{decode, Validation, Algorithm};

async fn authenticate_request(
    auth_header: &str,
) -> Result<ApiToken, AuthError> {
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::MissingToken)?;

    let key = get_signing_key().await?;
    let decoded = decode::<TokenClaims>(
        token,
        &key,
        &Validation::new(Algorithm::HS256),
    )?;

    Ok(ApiToken {
        api_key: decoded.claims.api_key,
        permissions: decoded.claims.permissions,
        expires_at: decoded.claims.exp,
    })
}
```

### Authorization

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Permission {
    NamespaceRead,
    NamespaceWrite,
    NamespaceDelete,
}

async fn authorize(
    token: &ApiToken,
    required: Permission,
    namespace: &str,
) -> Result<(), AuthError> {
    let permissions = get_permissions_for_api_key(&token.api_key).await?;

    if !permissions.contains(&required) {
        return Err(AuthError::InsufficientPermissions);
    }

    // Check namespace-level permissions
    if !has_namespace_access(&token.api_key, namespace).await {
        return Err(AuthError::NamespaceAccessDenied);
    }

    Ok(())
}
```

### Encryption

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;

// Encryption at rest
fn encrypt_vector(vector: &[f32], key: &Key<Aes256Gcm>) -> Vec<u8> {
    let cipher = Aes256Gcm::new(key);
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);

    let ciphertext = cipher.encrypt(
        Nonce::from_slice(&nonce),
        bytemuck::cast_slice(vector),
    ).unwrap();

    [nonce.as_slice(), &ciphertext].concat()
}

// Encryption in transit (HTTPS)
// Handled by TLS termination at load balancer
```

---

## Cost Optimization

### Storage Tiering

```
Hot Storage (SSD):
- Frequently accessed namespaces
- Recent data (< 30 days)
- Cost: ~$0.10/GB/month

Warm Storage (HDD):
- Infrequently accessed namespaces
- Older data (30-90 days)
- Cost: ~$0.03/GB/month

Cold Storage (Object):
- Archive data (> 90 days)
- Backup snapshots
- Cost: ~$0.004/GB/month
```

### Compute Optimization

```rust
// Right-size instances based on workload
fn select_instance_type(workload: &WorkloadProfile) -> InstanceType {
    match (workload.qps, workload.memory_gb) {
        (qps, _) if qps < 100 => InstanceType::Small,
        (qps, _) if qps < 1000 => InstanceType::Medium,
        (qps, _) if qps < 10000 => InstanceType::Large,
        _ => InstanceType::XLarge,
    }
}

// Spot instances for non-critical workloads
// Use spot for: batch indexing, background compaction
// Use on-demand for: query serving
```

---

## Incident Response

### Runbook: High Latency

```
1. Check metrics dashboard
   - Query latency percentiles
   - Error rates
   - Resource utilization

2. Identify affected region/service
   - Is it regional or global?
   - Is it specific to certain namespaces?

3. Check for recent changes
   - Deployments in last 24 hours
   - Configuration changes
   - Traffic pattern changes

4. Immediate mitigations
   - Scale up affected service
   - Enable load shedding
   - Increase timeout thresholds

5. Root cause analysis
   - Review logs for errors
   - Check for slow queries
   - Analyze resource contention

6. Post-incident
   - Document timeline
   - Identify preventive measures
   - Update runbook
```

### Runbook: Data Corruption

```
1. Stop writes to affected namespace
   - Enable read-only mode
   - Notify affected users

2. Assess corruption scope
   - How many vectors affected?
   - When did corruption start?

3. Initiate recovery
   - Identify last known good backup
   - Restore to new location
   - Verify integrity

4. Switchover
   - Update routing to restored data
   - Re-enable writes
   - Monitor for issues

5. Post-incident
   - Investigate root cause
   - Review backup procedures
   - Update validation checks
```

---

## Summary

Production-grade vector search requires:

1. **Reliability:** Circuit breakers, retries, rate limiting
2. **Observability:** Metrics, tracing, alerting
3. **Scaling:** Horizontal for queries, vertical for storage
4. **Durability:** Replication, backups, recovery procedures
5. **Security:** Auth, encryption, access control
6. **Cost Management:** Storage tiering, right-sizing
7. **Incident Response:** Runbooks, monitoring, quick mitigation

Building these capabilities takes time—start with the basics (metrics, logging, backups) and add sophistication as your system grows.
