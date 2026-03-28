---
title: "Production-Grade Concurrency"
subtitle: "High concurrency deployment, scaling, and monitoring"
parent: exploration.md
---

# Production-Grade Concurrency

## Introduction

This document covers production considerations for deploying high-concurrency systems, including performance optimizations, scaling strategies, monitoring, and observability.

---

## Part 1: Performance Optimizations

### Memory Management

#### Object Pooling

```rust
struct ConnectionPool {
    connections: Vec<Connection>,
    available: Vec<usize>,
}

impl ConnectionPool {
    fn acquire(&mut self) -> Connection {
        if let Some(idx) = self.available.pop() {
            self.connections[idx].reset();
            self.connections[idx].clone()
        } else {
            Connection::new()
        }
    }

    fn release(&mut self, conn: Connection) {
        // Return to pool instead of dropping
    }
}
```

#### Buffer Reuse

```rust
struct BufferPool {
    buffers: Vec<Vec<u8>>,
}

impl BufferPool {
    fn get(&mut self, size: usize) -> Vec<u8> {
        // Reuse existing buffer if possible
        self.buffers
            .iter()
            .position(|b| b.capacity() >= size)
            .map(|idx| self.buffers.swap_remove(idx))
            .unwrap_or_else(|| Vec::with_capacity(size))
    }

    fn return_buffer(&mut self, mut buf: Vec<u8>) {
        buf.clear();
        self.buffers.push(buf);
    }
}
```

### Batching

#### Request Batching

```rust
struct Batcher<T> {
    items: Vec<T>,
    max_size: usize,
    flush_interval: Duration,
}

impl<T> Batcher<T> {
    async fn run(mut self) {
        loop {
            tokio::select! {
                // Flush when full
                _ = async {
                    while self.items.len() < self.max_size {
                        tokio::task::yield_now().await;
                    }
                } => self.flush().await,

                // Flush on timeout
                _ = tokio::time::sleep(self.flush_interval) => {
                    if !self.items.is_empty() {
                        self.flush().await;
                    }
                }
            }
        }
    }

    async fn flush(&mut self) {
        let items = std::mem::take(&mut self.items);
        process_batch(items).await;
    }
}
```

### Backpressure

#### Rate Limiting

```rust
use tokio::sync::Semaphore;

struct RateLimiter {
    sem: Arc<Semaphore>,
    refill_interval: Duration,
}

impl RateLimiter {
    async fn acquire(&self) -> Result<(), RateLimitError> {
        self.sem.acquire().await
            .map_err(|_| RateLimitError::Closed)?
            .forget();
        Ok(())
    }

    fn spawn_refiller(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(self.refill_interval).await;
                self.sem.add_permits(100);
            }
        });
    }
}
```

---

## Part 2: Scaling Strategies

### Horizontal Scaling

```
┌─────────────────────────────────────────────┐
│            Load Balancer                     │
└─────────────────┬───────────────────────────┘
                  │
    ┌─────────────┼─────────────┐
    │             │             │
┌───▼───┐   ┌────▼────┐   ┌───▼───┐
│Node 1 │   │ Node 2  │   │Node 3 │
└───────┘   └─────────┘   └───────┘
```

### Vertical Scaling

```rust
// Thread-per-core scaling
use glommio::{LocalExecutorPoolBuilder, PoolPlacement};

LocalExecutorPoolBuilder::new(PoolPlacement::Unbound(4))
    .spawn(|| async move {
        // Each executor pinned to a core
    });
```

### Partitioning

#### Data Sharding

```rust
struct ShardManager {
    shards: Vec<Shard>,
}

impl ShardManager {
    fn get_shard(&self, key: &str) -> &Shard {
        let hash = self.hash(key);
        let shard_idx = hash % self.shards.len();
        &self.shards[shard_idx]
    }

    fn hash(&self, key: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut h = DefaultHasher::new();
        key.hash(&mut h);
        h.finish()
    }
}
```

---

## Part 3: Monitoring

### Metrics Collection

```rust
use prometheus::{Counter, Histogram, Registry};

struct Metrics {
    requests_total: Counter,
    request_duration: Histogram,
    active_connections: Counter,
}

impl Metrics {
    fn new(registry: &Registry) -> Self {
        Metrics {
            requests_total: Counter::new("requests_total", "Total requests").unwrap(),
            request_duration: Histogram::with_opts(
                histogram_opts!("request_duration_seconds", "Request duration")
            ).unwrap(),
            active_connections: Counter::new("active_connections", "Active connections").unwrap(),
        }
    }

    fn record_request(&self, duration: f64) {
        self.requests_total.inc();
        self.request_duration.observe(duration);
    }
}
```

### Distributed Tracing

```rust
use tracing::{info, span, Level};
use tracing_opentelemetry::OpenTelemetrySpanExt;

async fn handle_request(request: Request) {
    let span = span!(Level::INFO, "handle_request", id = %request.id);
    let _guard = span.enter();

    info!("Processing request");

    // Add trace context
    span.set_attribute("request.method", request.method);
    span.set_attribute("request.path", request.path);

    process(request).await;
}
```

### Health Checks

```rust
struct HealthChecker {
    checks: Vec<Box<dyn HealthCheck>>,
}

impl HealthChecker {
    async fn check(&self) -> HealthStatus {
        let mut status = HealthStatus::Healthy;

        for check in &self.checks {
            match check.run().await {
                Ok(_) => {},
                Err(e) => {
                    status = HealthStatus::Unhealthy(e);
                    break;
                }
            }
        }

        status
    }
}

trait HealthCheck: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self) -> BoxFuture<Result<(), HealthError>>;
}
```

---

## Part 4: Observability

### Structured Logging

```rust
use tracing::{info, warn, error, event, Level};

// Good: structured fields
info!(
    user_id = %user.id,
    action = "login",
    ip = %request.ip,
    "User logged in"
);

// Bad: unstructured message
info!("User {} logged in from {}", user.id, request.ip);
```

### Log Aggregation

```
┌──────────┐   ┌──────────┐   ┌──────────┐
│  App 1   │   │  App 2   │   │  App 3   │
└────┬─────┘   └────┬─────┘   └────┬─────┘
     │              │              │
     └──────────────┴──────────────┘
                    │
             ┌──────▼──────┐
             │  Log Agent  │
             └──────┬──────┘
                    │
             ┌──────▼──────┐
             │ Elasticsearch│
             └──────┬──────┘
                    │
             ┌──────▼──────┐
             │   Kibana    │
             └─────────────┘
```

### Alerting

```rust
struct AlertManager {
    rules: Vec<AlertRule>,
    channels: Vec<AlertChannel>,
}

struct AlertRule {
    name: String,
    condition: Box<dyn Condition>,
    threshold: u64,
    window: Duration,
}

trait Condition {
    fn evaluate(&self, metrics: &Metrics) -> bool;
}

impl AlertManager {
    async fn check_alerts(&self) {
        for rule in &self.rules {
            if rule.condition.evaluate(&self.metrics) {
                self.send_alert(&rule).await;
            }
        }
    }
}
```

---

## Part 5: Production Patterns

### Circuit Breaker

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

struct CircuitBreaker {
    failures: AtomicUsize,
    state: AtomicUsize,  // 0=closed, 1=open, 2=half-open
    threshold: usize,
}

impl CircuitBreaker {
    async fn call<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: Future<Output = Result<T, Error>>,
    {
        match self.state.load(Ordering::SeqCst) {
            0 => {
                // Closed - allow through
                match f.await {
                    Ok(result) => {
                        self.failures.store(0, Ordering::SeqCst);
                        Ok(result)
                    }
                    Err(e) => {
                        self.failures.fetch_add(1, Ordering::SeqCst);
                        if self.failures.load(Ordering::SeqCst) >= self.threshold {
                            self.state.store(1, Ordering::SeqCst);
                        }
                        Err(e)
                    }
                }
            }
            1 => Err(Error::CircuitOpen),
            2 => {
                // Half-open - try one request
                match f.await {
                    Ok(result) => {
                        self.state.store(0, Ordering::SeqCst);
                        Ok(result)
                    }
                    Err(e) => {
                        self.state.store(1, Ordering::SeqCst);
                        Err(e)
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}
```

### Retry with Backoff

```rust
use std::time::Duration;

async fn retry_with_backoff<F, T, E>(
    mut f: F,
    max_retries: u32,
) -> Result<T, E>
where
    F: FnMut() -> impl Future<Output = Result<T, E>>,
{
    let mut delay = Duration::from_millis(100);
    let mut attempts = 0;

    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(e);
                }

                tokio::time::sleep(delay).await;
                delay = std::cmp::min(delay * 2, Duration::from_secs(30));
            }
        }
    }
}
```

### Graceful Shutdown

```rust
use tokio::sync::broadcast;

struct GracefulShutdown {
    tx: broadcast::Sender<()>,
}

impl GracefulShutdown {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        GracefulShutdown { tx }
    }

    fn signal(&self) {
        let _ = self.tx.send(());
    }

    async fn wait(&self) {
        let mut rx = self.tx.subscribe();
        let _ = rx.recv().await;
    }
}

async fn server(shutdown: GracefulShutdown) {
    tokio::select! {
        _ = server_loop() => {},
        _ = shutdown.wait() => {
            // Clean shutdown
            cleanup().await;
        }
    }
}
```

---

## Part 6: Deployment Considerations

### Container Resource Limits

```yaml
# Kubernetes deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: concurrency-app
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: app
        resources:
          requests:
            cpu: "500m"
            memory: "512Mi"
          limits:
            cpu: "1000m"
            memory: "1Gi"
```

### Auto-Scaling

```yaml
# Horizontal Pod Autoscaler
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: concurrency-app-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: concurrency-app
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

---

*This document covers essential production patterns for high-concurrency systems...*
