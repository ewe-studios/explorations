# iroh-metrics Deep Dive

## Overview

`iroh-metrics` provides a metrics collection library for the iroh ecosystem. It supports Prometheus-compatible metrics with both enabled and disabled (no-op) modes through feature flags.

**Version:** 0.35
**Repository:** https://github.com/n0-computer/iroh
**License:** MIT/Apache-2.0

---

## Architecture and Design Decisions

### Core Design Philosophy

The metrics library is designed with several key principles:

1. **Zero Overhead When Disabled**: When the `metrics` feature is disabled, all metric operations become no-ops with zero runtime cost
2. **Type-Safe Metric Definitions**: Derive macros ensure type-safe metric group definitions
3. **Prometheus Compatibility**: Output format is compatible with Prometheus scraping
4. **Composable Groups**: Metrics can be organized into logical groups

### Feature-Gated Design

The library uses a feature-gated approach:

```rust
// With "metrics" feature - collects actual data
#[cfg(feature = "metrics")]
pub(crate) value: AtomicU64,

// Without "metrics" feature - no-op operations
#[cfg(not(feature = "metrics"))]
// Operations return 0, do nothing
```

This allows:
- Production builds with full metrics
- Development/test builds without metrics overhead
- Library consumers to opt-in to metrics collection

### Metric Types

The library supports two metric types:

1. **Counter**: Monotonically increasing value (e.g., request count)
2. **Gauge**: Value that can increase or decrease (e.g., active connections)

---

## Key APIs and Data Structures

### Core Traits

```rust
/// Trait for metric items
pub trait Metric: std::fmt::Debug {
    /// Returns the type of this metric
    fn r#type(&self) -> MetricType;

    /// Returns the current value of this metric
    fn value(&self) -> MetricValue;

    /// Casts this metric to Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

/// Group of related metrics
pub trait MetricsGroup: std::fmt::Debug + Send + Sync {
    /// Name of the metric group
    fn name(&self) -> &'static str;

    /// Key for this metric group
    fn key(&self) -> &'static str {
        self.name()
    }

    /// Iterator over metrics in this group
    fn iter(&self) -> Box<dyn Iterator<Item = (&'static str, &'static dyn Metric)> + '_>;
}
```

### Metric Types

```rust
/// Types of metrics
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
}

/// Value of a metric
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MetricValue {
    Counter(u64),
    Gauge(i64),
}

/// Counter metric - monotonically increasing
#[derive(Debug, Default)]
pub struct Counter {
    #[cfg(feature = "metrics")]
    pub(crate) value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self { Self::default() }

    /// Increment by 1
    pub fn inc(&self) -> u64;

    /// Increment by value
    pub fn inc_by(&self, v: u64) -> u64;

    /// Set value (use sparingly)
    pub fn set(&self, v: u64) -> u64;

    /// Get current value
    pub fn get(&self) -> u64;
}

/// Gauge metric - can increase or decrease
#[derive(Debug, Default)]
pub struct Gauge {
    #[cfg(feature = "metrics")]
    pub(crate) value: AtomicI64,
}

impl Gauge {
    pub fn new() -> Self { Self::default() }

    /// Increment by 1
    pub fn inc(&self) -> i64;

    /// Decrement by 1
    pub fn dec(&self) -> i64;

    /// Add value
    pub fn add(&self, v: i64) -> i64;

    /// Set value
    pub fn set(&self, v: i64) -> i64;

    /// Get current value
    pub fn get(&self) -> i64;
}
```

### Derive Macros

```rust
/// Derives MetricsGroup and Iterable for a struct
///
/// Fields should have doc comments that become metric help text
#[derive(MetricsGroup)]
pub struct MyMetrics {
    /// Number of requests processed
    pub requests: Counter,

    /// Current active connections
    pub connections: Gauge,
}

/// Derives MetricsGroupSet for aggregating metric groups
#[derive(MetricsGroupSet)]
pub struct AllMetrics {
    pub blobs: Arc<BlobsMetrics>,
    pub sync: Arc<SyncMetrics>,
    pub gossip: Arc<GossipMetrics>,
}
```

### Registry

```rust
/// Registry for metric groups
pub struct Registry {
    groups: HashMap<&'static str, Arc<dyn MetricsGroup>>,
}

impl Registry {
    /// Get default registry
    pub fn default() -> Self;

    /// Register a metric group
    pub fn register(&mut self, group: Arc<dyn MetricsGroup>);

    /// Get metrics as Prometheus format
    pub fn render(&self) -> Result<String>;
}
```

---

## Protocol Details

### Prometheus Output Format

Metrics are rendered in Prometheus text format:

```
# HELP blobs_requests_total Number of requests processed
# TYPE blobs_requests_total counter
blobs_requests_total 1234

# HELP blobs_connections_current Current active connections
# TYPE blobs_connections_current gauge
blobs_connections_current 42
```

### Metric Naming Convention

```
<group>_<name>_<type>

Examples:
- blobs_requests_total    (counter)
- blobs_bytes_sent_total  (counter)
- blobs_connections_current (gauge)
```

---

## Integration with Main Iroh Endpoint

### Endpoint Metrics

```rust
// Metrics are integrated throughout iroh
let endpoint = Endpoint::builder()
    .metrics(metrics.clone())
    .bind()
    .await?;

// Metrics automatically updated
endpoint.connect(addr).await?;  // Increments connection counter
```

### Protocol Handler Metrics

```rust
#[derive(MetricsGroup)]
pub struct BlobsMetrics {
    /// Total bytes sent
    pub bytes_sent: Counter,
    /// Total bytes received
    pub bytes_received: Counter,
    /// Active connections
    pub connections: Gauge,
    /// Request errors
    pub request_errors: Counter,
}

impl BlobsProtocol {
    fn new(metrics: Arc<BlobsMetrics>) -> Self {
        Self { metrics }
    }

    async fn handle_request(&self, req: Request) {
        self.metrics.requests.inc();

        match self.process(req).await {
            Ok(bytes) => {
                self.metrics.bytes_sent.inc_by(bytes.len() as u64);
            }
            Err(_) => {
                self.metrics.request_errors.inc();
            }
        }
    }
}
```

### Metrics Service

```rust
// Optional HTTP service for metrics scraping
#[cfg(feature = "service")]
pub mod service {
    pub async fn serve_metrics(
        registry: Arc<Registry>,
        addr: SocketAddr,
    ) -> Result<()> {
        // Start HTTP server on addr
        // GET /metrics returns Prometheus format
    }
}
```

---

## Production Usage Patterns

### Defining Custom Metrics

```rust
use iroh_metrics::{MetricsGroup, Counter, Gauge};

#[derive(MetricsGroup, Debug)]
pub struct MyServiceMetrics {
    /// Total number of operations
    pub operations_total: Counter,

    /// Current queue depth
    pub queue_depth: Gauge,

    /// Processing errors
    pub errors_total: Counter,

    /// Average processing time (microseconds)
    pub processing_time_us: Gauge,
}

impl MyServiceMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_operation(&self) {
        self.operations_total.inc();
    }

    pub fn record_error(&self) {
        self.errors_total.inc();
    }

    pub fn update_queue(&self, depth: usize) {
        self.queue_depth.set(depth as i64);
    }
}
```

### Using Metrics in Components

```rust
pub struct Downloader {
    metrics: Arc<DownloaderMetrics>,
}

impl Downloader {
    pub fn new(metrics: Arc<DownloaderMetrics>) -> Self {
        Self { metrics }
    }

    pub async fn download(&self, hash: Hash) -> Result<Bytes> {
        self.metrics.downloads_started.inc();
        let start = Instant::now();

        let data = self.fetch(hash).await?;

        self.metrics.bytes_downloaded.inc_by(data.len() as u64);
        self.metrics.download_time_us.set(start.elapsed().as_micros() as i64);
        self.metrics.downloads_completed.inc();

        Ok(data)
    }
}
```

### Metrics Collection and Export

```rust
// Collect and render metrics
fn collect_metrics(registry: &Registry) -> String {
    registry.render().unwrap_or_default()
}

// HTTP handler for Prometheus scraping
async fn metrics_handler(
    State(registry): State<Arc<Registry>>,
) -> impl IntoResponse {
    let metrics = collect_metrics(&registry);
    (
        [(CONTENT_TYPE, "text/plain; version=0.0.4")],
        metrics,
    )
}
```

### Conditional Metrics

```rust
// Code works the same with or without metrics feature
fn process_data(data: &[u8], metrics: &MyMetrics) {
    // These are no-ops when metrics feature is disabled
    metrics.bytes_processed.inc_by(data.len() as u64);
    metrics.operations.inc();

    // Actual processing
    // ...
}
```

---

## Rust Revision Notes

### Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| serde | 1.0 | Serialization |
| snafu | 0.8 | Error handling |
| parking_lot | 0.12 | Synchronization (when disabled) |

### Notable Rust Patterns

1. **Feature-Gated Atomics**: `AtomicU64` only present when feature enabled
2. **Derive Macros**: Custom derive for metric group definitions
3. **Trait Objects**: `&dyn Metric` for heterogeneous metric collections
4. **No-Op Pattern**: Compile-out code when feature disabled

### Atomic Operations

```rust
impl Counter {
    #[cfg(feature = "metrics")]
    pub fn inc(&self) -> u64 {
        self.value.fetch_add(1, Ordering::Relaxed)
    }

    #[cfg(not(feature = "metrics"))]
    pub fn inc(&self) -> u64 {
        0  // No-op
    }
}
```

### Potential Enhancements

1. **Histogram Support**: For latency distributions
2. **Summary Support**: For quantile calculations
3. **Labels/Tags**: Prometheus-style label support
4. **Automatic Collection**: Periodic background collection

---

## Summary

`iroh-metrics` provides:

- **Zero Overhead**: Feature-gated design eliminates overhead when disabled
- **Type Safety**: Derive macros ensure correct metric definitions
- **Prometheus Compatible**: Standard output format for monitoring
- **Composable Design**: Easy to add metrics to new components

The library enables comprehensive observability for iroh deployments with minimal performance impact.
