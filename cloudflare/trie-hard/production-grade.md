# Production-Grade Implementation Guide

**Document 08** | Deployment, Scaling, and Monitoring
**Source:** Cloudflare Pingora production use | **Date:** 2026-03-27

---

## Executive Summary

trie-hard powers Cloudflare's header filtering at **30 million requests per second**. This document covers:
- Deployment architecture
- Scaling strategies
- Monitoring and observability
- Operational best practices

---

## Part 1: Production Architecture

### Cloudflare's Deployment Model

```
┌─────────────────────────────────────────────────────────────────┐
│                        Cloudflare Edge                          │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │  Pingora    │  │  Pingora    │  │  Pingora    │             │
│  │  Worker 1   │  │  Worker 2   │  │  Worker N   │             │
│  │             │  │             │  │             │             │
│  │  ┌───────┐  │  │  ┌───────┐  │  │  ┌───────┐  │             │
│  │  │TrieHard│  │  │  │TrieHard│  │  │  │TrieHard│  │             │
│  │  │(shared)│  │  │  │(shared)│  │  │  │(shared)│  │             │
│  │  └───────┘  │  │  └───────┘  │  │  └───────┘  │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│         │                │                │                     │
│         └────────────────┴────────────────┘                     │
│                          │                                      │
│              ┌───────────┴───────────┐                         │
│              │   Configuration Store │                         │
│              │   (header list)       │                         │
│              └───────────────────────┘                         │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **One trie per worker** | Avoid cross-worker coordination |
| **Shared read-only** | Arc<TrieHard>, lock-free reads |
| **Bulk rebuild on update** | Simple consistency model |
| **In-memory only** | ~4KB fits in cache, no I/O needed |

---

## Part 2: Configuration Management

### Static Configuration

```rust
// For unchanging header lists
static KNOWN_HEADERS: once_cell::sync::Lazy<TrieHard<'static, &'static str>> =
    once_cell::sync::Lazy::new(|| {
        [
            "accept",
            "accept-encoding",
            "accept-language",
            "authorization",
            "cache-control",
            // ... ~120 standard HTTP headers
        ]
        .into_iter()
        .collect()
    });
```

**Pros:** Zero runtime overhead, compiled-in
**Cons:** Requires redeploy to update

### Dynamic Configuration

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;
use trie_hard::TrieHard;

pub struct HeaderConfig {
    trie: AtomicPtr<TrieHard<'static, &'static str>>,
}

impl HeaderConfig {
    pub fn new(initial: TrieHard<'static, &'static str>) -> Self {
        Self {
            trie: AtomicPtr::new(Box::into_raw(Box::new(initial))),
        }
    }

    pub fn get(&self) -> &TrieHard<'static, &'static str> {
        unsafe { &*self.trie.load(Ordering::Acquire) }
    }

    pub fn update(&self, new_trie: TrieHard<'static, &'static str>) {
        let new_ptr = Box::into_raw(Box::new(new_trie));
        let old_ptr = self.trie.swap(new_ptr, Ordering::AcqRel);

        // Safe because all readers have finished (document in production)
        unsafe {
            drop(Box::from_raw(old_ptr));
        }
    }
}

// Safer: use arc-swap crate
use arc_swap::ArcSwap;

pub struct HeaderConfigSafe {
    trie: ArcSwap<TrieHard<'static, &'static str>>,
}

impl HeaderConfigSafe {
    pub fn get(&self) -> Arc<TrieHard<'static, &'static str>> {
        self.trie.load_full()
    }

    pub fn update(&self, new_trie: Arc<TrieHard<'static, &'static str>>) {
        self.trie.store(new_trie);
    }
}
```

### Configuration Update Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Config API  │────▶│  Validator   │────▶│  Trie Builder│
│  (HTTP PUT)  │     │  (new list)  │     │  (bulk load) │
└──────────────┘     └──────────────┘     └──────────────┘
                                                  │
                                                  ▼
                                         ┌──────────────┐
                                         │  Atomic Swap │
                                         │  (lock-free) │
                                         └──────────────┘
                                                  │
                                                  ▼
                                         ┌──────────────┐
                                         │  Old Trie    │
                                         │  (deferred   │
                                         │   cleanup)   │
                                         └──────────────┘
```

---

## Part 3: Scaling Strategies

### Horizontal Scaling

```yaml
# Kubernetes deployment example
apiVersion: apps/v1
kind: Deployment
metadata:
  name: header-filter
spec:
  replicas: 10  # Scale based on CPU utilization
  template:
    spec:
      containers:
      - name: filter
        image: header-filter:latest
        resources:
          requests:
            cpu: "500m"   # 0.5 CPU cores
            memory: "64Mi" # trie is tiny
          limits:
            cpu: "1000m"
            memory: "128Mi"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
```

### Scaling Metrics

| Metric | Target | Action |
|--------|--------|--------|
| CPU utilization | < 70% | Add replicas |
| P99 latency | < 100μs | Investigate |
| Memory usage | < 50MB | Normal (trie is small) |
| Request rate | Scale linearly | Add replicas |

### Load Shedding

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

pub struct RateLimiter {
    requests: AtomicUsize,
    last_reset: parking_lot::Mutex<Instant>,
    limit: usize,
}

impl RateLimiter {
    pub fn new(limit: usize) -> Self {
        Self {
            requests: AtomicUsize::new(0),
            last_reset: parking_lot::Mutex::new(Instant::now()),
            limit,
        }
    }

    pub fn try_acquire(&self) -> bool {
        let current = self.requests.fetch_add(1, Ordering::Relaxed);

        if current >= self.limit {
            // Check if we should reset counter
            let elapsed = self.last_reset.lock().elapsed();
            if elapsed > Duration::from_secs(1) {
                self.requests.store(0, Ordering::Relaxed);
                *self.last_reset.lock() = Instant::now();
                return true;
            }
            return false;  // Rate limited
        }

        true
    }
}
```

---

## Part 4: Monitoring and Observability

### Metrics to Track

```rust
use metrics::{counter, histogram, gauge};

pub struct TrieMetrics {
    lookups: counter::Counter,
    hits: counter::Counter,
    latency: histogram::Histogram,
    trie_size: gauge::Gauge,
}

impl TrieMetrics {
    pub fn record_lookup(&self, found: bool, latency: Duration) {
        self.lookups.increment(1);
        if found {
            self.hits.increment(1);
        }
        self.latency.record(latency.as_nanos() as u64);
    }

    pub fn set_trie_size(&self, entries: usize) {
        self.trie_size.set(entries as f64);
    }
}

// In request handler
let start = Instant::now();
let found = header_filter.get(header_name).is_some();
metrics.record_lookup(found, start.elapsed());
```

### Recommended Dashboards

#### Header Filter Performance

```
Panel 1: Lookup Latency (P50, P90, P99)
  - Query: histogram_quantile(0.50, rate(trie_lookup_latency_bucket[5m]))
  - Query: histogram_quantile(0.90, rate(trie_lookup_latency_bucket[5m]))
  - Query: histogram_quantile(0.99, rate(trie_lookup_latency_bucket[5m]))

Panel 2: Hit Rate
  - Query: sum(rate(trie_hits_total[5m])) / sum(rate(trie_lookups_total[5m]))

Panel 3: Requests per Second
  - Query: sum(rate(trie_lookups_total[5m]))

Panel 4: Trie Size (entries)
  - Query: trie_entries_count
```

#### Resource Utilization

```
Panel 1: CPU Usage by Pod
  - Query: rate(process_cpu_seconds_total[5m])

Panel 2: Memory Usage
  - Query: process_resident_memory_bytes

Panel 3: Goroutine/Thread Count
  - Query: go_goroutines (or equivalent)
```

### Alerting Rules

```yaml
# Prometheus alerting rules
groups:
- name: header-filter
  rules:
  - alert: HighLookupLatency
    expr: histogram_quantile(0.99, rate(trie_lookup_latency_bucket[5m])) > 100000
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "Header filter P99 latency above 100μs"

  - alert: LowHitRate
    expr: sum(rate(trie_hits_total[1h])) / sum(rate(trie_lookups_total[1h])) < 0.3
    for: 30m
    labels:
      severity: info
    annotations:
      summary: "Header filter hit rate below 30%"

  - alert: HighErrorRate
    expr: sum(rate(trie_errors_total[5m])) / sum(rate(trie_lookups_total[5m])) > 0.01
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "Header filter error rate above 1%"
```

---

## Part 5: Deployment Checklist

### Pre-Deployment

- [ ] Benchmark with production-like data
- [ ] Set up metrics collection
- [ ] Configure alerting rules
- [ ] Test configuration updates
- [ ] Document rollback procedure

### Deployment

- [ ] Deploy to canary (1% traffic)
- [ ] Monitor error rates for 5 minutes
- [ ] Increase to 10% traffic
- [ ] Monitor for 15 minutes
- [ ] Full rollout if healthy

### Post-Deployment

- [ ] Verify all metrics reporting
- [ ] Check P99 latency is acceptable
- [ ] Confirm hit rate is expected
- [ ] Review resource utilization

---

## Part 6: Failure Modes and Recovery

### Failure Mode 1: Memory Exhaustion

**Symptoms:**
- OOM killer terminates process
- Memory usage spikes

**Causes:**
- Configuration bug creates huge trie
- Memory leak elsewhere in application

**Recovery:**
```bash
# Rollback to previous configuration
kubectl rollout undo deployment/header-filter

# Or manually scale down
kubectl scale deployment/header-filter --replicas=0
```

**Prevention:**
```rust
// Validate trie size before swapping
const MAX_TRIE_ENTRIES: usize = 100_000;

fn validate_config(new_headers: &[&str]) -> Result<(), ConfigError> {
    if new_headers.len() > MAX_TRIE_ENTRIES {
        return Err(ConfigError::TooLarge(new_headers.len()));
    }
    Ok(())
}
```

### Failure Mode 2: High Latency

**Symptoms:**
- P99 latency spikes
- Request timeouts

**Causes:**
- CPU contention
- Cache thrashing
- Configuration too large

**Diagnosis:**
```bash
# Check CPU usage
kubectl top pods -l app=header-filter

# Check cache misses with perf
perf stat -e cache-misses,cache-references -p <pid>
```

**Recovery:**
```bash
# Scale up to reduce contention
kubectl scale deployment/header-filter --replicas=20
```

### Failure Mode 3: Stale Configuration

**Symptoms:**
- New headers not recognized
- Old headers still filtered

**Causes:**
- Configuration update failed
- Atomic swap didn't propagate

**Recovery:**
```bash
# Force configuration refresh
curl -X POST http://header-filter/admin/refresh

# Or restart pods
kubectl rollout restart deployment/header-filter
```

---

## Part 7: Cost Analysis

### Infrastructure Costs (Estimated)

```
For 30M requests/second:

Assumptions:
- 100ns average lookup latency
- 1 CPU core handles ~10M req/s
- $0.05/hour per vCPU (cloud pricing)

Compute needed: 30M / 10M = 3 cores
With redundancy: 6 cores (2x replicas)
Monthly cost: 6 * 24 * 30 * $0.05 = ~$216/month

Add 20% overhead for spikes: ~$260/month
```

### Comparison: trie-hard vs HashMap

```
For same workload:

trie-hard:
- 6 cores @ 100ns/lookup
- $260/month

HashMap:
- 10 cores @ 200ns/lookup (estimated)
- $433/month

Savings with trie-hard: ~$173/month (40%)
```

---

## Summary

Production deployment of trie-hard:

1. **Immutable after construction** - Simple concurrency model
2. **Arc-based sharing** - Lock-free reads across threads
3. **Atomic configuration updates** - Hot-swappable tries
4. **Comprehensive monitoring** - Latency, hit rate, errors
5. **Horizontal scaling** - Linear scaling with replicas

### Next Steps

Continue to **[05-valtron-integration.md](05-valtron-integration.md)** for:
- Lambda deployment with Valtron
- TaskIterator pattern
- NO async/await, NO tokio
- Serverless patterns

---

## Appendix: Sample Configuration Files

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: header-filter
  labels:
    app: header-filter
spec:
  replicas: 3
  selector:
    matchLabels:
      app: header-filter
  template:
    metadata:
      labels:
        app: header-filter
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      containers:
      - name: filter
        image: header-filter:latest
        ports:
        - containerPort: 8080
          name: http
        - containerPort: 9090
          name: metrics
        resources:
          requests:
            cpu: "500m"
            memory: "64Mi"
          limits:
            cpu: "1000m"
            memory: "128Mi"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
        env:
        - name: RUST_LOG
          value: "info"
        - name: HEADER_CONFIG_URL
          value: "http://config-service/headers"
```

### Prometheus Scrape Config

```yaml
scrape_configs:
- job_name: 'header-filter'
  kubernetes_sd_configs:
  - role: pod
  relabel_configs:
  - source_labels: [__meta_kubernetes_pod_label_app]
    regex: header-filter
    action: keep
  - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
    regex: "true"
    action: keep
```
