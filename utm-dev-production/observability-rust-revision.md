# utm-dev Production - Observability & Telemetry Rust Revision

## Overview

This document provides a comprehensive Rust implementation for observability and telemetry systems, enabling build analytics, performance monitoring, error reporting, usage metrics, and production alerting. The implementation replaces the mixed Go/Rust/TypeScript architecture with idiomatic Rust.

**Key Goals:**
- Unified telemetry collection with async batch processing
- Real-time performance profiling and system monitoring
- Comprehensive error tracking with backtrace capture
- Usage analytics with privacy-first design
- Flexible alerting with multiple notification channels
- Dashboard-ready metrics aggregation

## Workspace Structure

```
utm-observability/
├── Cargo.toml                 # Workspace root
├── README.md
├── utm-observability-core/    # Core traits and types
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── events.rs          # Telemetry event types
│       ├── error.rs           # Error types
│       └── config.rs          # Configuration
├── utm-telemetry/             # Telemetry collection
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── collector.rs       # Telemetry collector
│       ├── exporter.rs        # Batch exporter
│       ├── buffer.rs          # Local buffering
│       └── sampler.rs         # Sampling strategies
├── utm-profiler/              # Performance profiling
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── system.rs          # System profiler
│       ├── phase.rs           # Build phase profiler
│       ├── flamegraph.rs      # Flame graph generation
│       └── resources.rs       # Resource monitoring
├── utm-error-tracker/         # Error tracking
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── tracker.rs         # Error tracker
│       ├── report.rs          # Error reports
│       ├── panic.rs           # Panic hook integration
│       └── rate_limit.rs      # Rate limiting
├── utm-metrics/               # Metrics aggregation
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── aggregator.rs      # Metrics aggregator
│       ├── time_series.rs     # Time series data
│       ├── anomaly.rs         # Anomaly detection
│       └── summary.rs         # Summary statistics
├── utm-usage/                 # Usage analytics
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── tracker.rs         # Usage tracker
│       ├── commands.rs        # Command tracking
│       └── privacy.rs         # Privacy controls
├── utm-alerting/              # Alerting system
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── rules.rs           # Alert rules
│       ├── evaluator.rs       # Alert evaluator
│       ├── notify.rs          # Notifications
│       └── builtins.rs        # Built-in alerts
└── utm-observability-cli/     # CLI tool
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── commands/
            ├── metrics.rs
            ├── profile.rs
            └── alerts.rs
```

## Crate Breakdown

| Crate | Purpose | Platforms |
|-------|---------|-----------|
| `utm-observability-core` | Shared traits, event types | All |
| `utm-telemetry` | Telemetry collection & export | All |
| `utm-profiler` | Performance profiling | All |
| `utm-error-tracker` | Error tracking & reporting | All |
| `utm-metrics` | Metrics aggregation & analysis | All |
| `utm-usage` | Usage analytics | All |
| `utm-alerting` | Alert rules & notifications | All |
| `utm-observability-cli` | CLI for observability | All |

## Recommended Dependencies

### utm-observability-core/Cargo.toml
```toml
[package]
name = "utm-observability-core"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
```

### utm-telemetry/Cargo.toml
```toml
[package]
name = "utm-telemetry"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-observability-core = { path = "../utm-observability-core" }
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
tracing = "0.1"
rand = "0.8"
```

### utm-profiler/Cargo.toml
```toml
[package]
name = "utm-profiler"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-observability-core = { path = "../utm-observability-core" }
tokio = { version = "1.0", features = ["full"] }
sysinfo = "0.30"
inferno = "0.11"
parking_lot = "0.12"
tracing = "0.1"
```

### utm-alerting/Cargo.toml
```toml
[package]
name = "utm-alerting"
version = "0.1.0"
edition = "2021"
license = "MIT"

[dependencies]
utm-observability-core = { path = "../utm-observability-core" }
utm-metrics = { path = "../utm-metrics" }
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde_json = "1.0"
chrono = "0.4"
```

## Type System Design

### Core Event Types (utm-observability-core)

```rust
// utm-observability-core/src/events.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use chrono::{DateTime, Utc};

/// Unique event identifier
#[derive(Debug, Clone)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Base telemetry event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Unique event ID
    pub event_id: EventId,

    /// Event type/name
    pub event_type: EventType,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Session ID (groups related events)
    pub session_id: String,

    /// User/organization ID (anonymized)
    pub user_id: Option<String>,

    /// Project identifier (anonymized)
    pub project_id: Option<String>,

    /// Event properties
    pub properties: HashMap<String, PropertyValue>,

    /// Context information
    pub context: EventContext,
}

/// Event type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Build lifecycle events
    BuildStarted,
    BuildCompleted,
    BuildFailed,
    BuildCancelled,

    // Phase events
    PhaseStarted,
    PhaseCompleted,
    PhaseFailed,

    // Performance events
    CacheHit,
    CacheMiss,
    SlowOperation,
    ResourceHigh,

    // Error events
    ErrorOccurred,
    WarningOccurred,
    PanicOccurred,

    // Usage events
    CommandExecuted,
    PluginLoaded,
    FeatureUsed,
    ConfigurationChanged,
}

/// Property value types (JSON-like)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PropertyValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<PropertyValue>),
    Object(HashMap<String, PropertyValue>),
}

impl PropertyValue {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            PropertyValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            PropertyValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl From<String> for PropertyValue {
    fn from(s: String) -> Self {
        PropertyValue::String(s)
    }
}

impl From<f64> for PropertyValue {
    fn from(n: f64) -> Self {
        PropertyValue::Number(n)
    }
}

impl From<bool> for PropertyValue {
    fn from(b: bool) -> Self {
        PropertyValue::Boolean(b)
    }
}

impl From<i64> for PropertyValue {
    fn from(n: i64) -> Self {
        PropertyValue::Number(n as f64)
    }
}

impl From<u64> for PropertyValue {
    fn from(n: u64) -> Self {
        PropertyValue::Number(n as f64)
    }
}

/// Event context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContext {
    /// utm-dev version
    pub utm_version: String,

    /// Operating system info
    pub os: OsInfo,

    /// Hardware info
    pub hardware: HardwareInfo,

    /// Build configuration
    pub build_config: BuildConfigContext,

    /// Environment variables (opt-in)
    pub environment: HashMap<String, String>,
}

/// Operating system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub kernel: Option<String>,
}

/// Hardware information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub disk_type: Option<DiskType>,
}

/// Disk type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiskType {
    Hdd,
    Ssd,
    Nvme,
}

/// Build configuration context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfigContext {
    pub target: Option<String>,
    pub profile: String,
    pub features: Vec<String>,
    pub incremental: bool,
    pub parallel_jobs: u32,
}

impl Default for EventContext {
    fn default() -> Self {
        Self {
            utm_version: env!("CARGO_PKG_VERSION").to_string(),
            os: OsInfo {
                name: std::env::consts::OS.to_string(),
                version: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                kernel: None,
            },
            hardware: HardwareInfo {
                cpu_cores: num_cpus::get() as u32,
                memory_gb: 0, // Would be populated from sysinfo
                disk_type: None,
            },
            build_config: BuildConfigContext {
                target: None,
                profile: "debug".to_string(),
                features: Vec::new(),
                incremental: false,
                parallel_jobs: num_cpus::get() as u32,
            },
            environment: HashMap::new(),
        }
    }
}

/// Build-specific event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildEventData {
    /// Build duration in milliseconds
    pub duration_ms: u64,

    /// Total crates compiled
    pub crates_compiled: u32,

    /// Total lines of code
    pub total_loc: u32,

    /// Cache statistics
    pub cache_stats: CacheStats,

    /// Resource utilization
    pub resource_usage: ResourceUsage,

    /// Artifacts produced
    pub artifacts: Vec<ArtifactInfo>,

    /// Errors/warnings count
    pub errors: u32,
    pub warnings: u32,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u32,
    pub misses: u32,
    pub hit_rate: f64,
    pub size_mb: f64,
}

/// Resource usage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_max_percent: f64,
    pub cpu_avg_percent: f64,
    pub memory_max_mb: u64,
    pub memory_avg_mb: u64,
    pub io_read_mb: f64,
    pub io_write_mb: f64,
}

/// Artifact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub name: String,
    pub size_bytes: u64,
    pub artifact_type: String,
}
```

### Error Types (utm-observability-core)

```rust
// utm-observability-core/src/error.rs
use thiserror::Error;

/// Telemetry operation result
pub type TelemetryResult<T> = Result<T, TelemetryError>;

/// Unified telemetry error type
#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("Telemetry disabled")]
    Disabled,

    #[error("Channel full, dropping event")]
    ChannelFull,

    #[error("Export failed: {0}")]
    ExportFailed(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Privacy error: {0}")]
    PrivacyError(String),
}

/// Error tracker result
pub type ErrorTrackerResult<T> = Result<T, ErrorTrackerError>;

/// Error tracker error type
#[derive(Error, Debug)]
pub enum ErrorTrackerError {
    #[error("Error tracking disabled")]
    Disabled,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Metrics aggregation result
pub type MetricsResult<T> = Result<T, MetricsError>;

/// Metrics error type
#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("Metric not found: {0}")]
    NotFound(String),

    #[error("Invalid time range")]
    InvalidTimeRange,

    #[error("Aggregation error: {0}")]
    AggregationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Alert evaluation result
pub type AlertResult<T> = Result<T, AlertError>;

/// Alert error type
#[derive(Error, Debug)]
pub enum AlertError {
    #[error("Invalid rule: {0}")]
    InvalidRule(String),

    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    #[error("Notification error: {0}")]
    NotificationError(String),

    #[error("HTTP error: {0}")]
    HttpError(String),
}
```

## Telemetry Collector (utm-telemetry)

```rust
// utm-telemetry/src/collector.rs
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::time::Duration;
use utm_observability_core::{
    TelemetryEvent, EventType, PropertyValue, TelemetryResult, TelemetryError,
    EventContext, EventId,
};
use crate::exporter::BatchExporter;
use crate::sampler::Sampler;

/// Telemetry collector configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Enable/disable telemetry
    pub enabled: bool,

    /// Sample rate (0.0 - 1.0)
    pub sample_rate: f64,

    /// Batch size for exports
    pub batch_size: usize,

    /// Export interval
    pub export_interval: Duration,

    /// Export endpoint
    pub endpoint: Option<String>,

    /// API key for export
    pub api_key: Option<String>,

    /// Anonymize user data
    pub anonymize: bool,

    /// Opt-in for error reporting
    pub error_reporting: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sample_rate: 1.0,
            batch_size: 100,
            export_interval: Duration::from_secs(30),
            endpoint: None,
            api_key: None,
            anonymize: true,
            error_reporting: true,
        }
    }
}

/// Telemetry collector for gathering and exporting events
pub struct TelemetryCollector {
    config: TelemetryConfig,
    event_tx: mpsc::Sender<TelemetryEvent>,
    buffer: Arc<RwLock<Vec<TelemetryEvent>>>,
    shutdown: Arc<tokio::sync::Notify>,
    sampler: Sampler,
}

impl TelemetryCollector {
    /// Create a new telemetry collector
    pub fn new(config: TelemetryConfig) -> Self {
        let (event_tx, mut event_rx) = mpsc::channel(1000);
        let buffer = Arc::new(RwLock::new(Vec::with_capacity(config.batch_size)));
        let shutdown = Arc::new(tokio::sync::Notify::new());
        let sampler = Sampler::new(config.sample_rate);

        // Spawn batch processor
        let processor_buffer = Arc::clone(&buffer);
        let processor_config = config.clone();
        let processor_shutdown = Arc::clone(&shutdown);
        let exporter = BatchExporter::new(&processor_config);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(processor_config.export_interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Flush buffer on interval
                        let events = {
                            let mut buf = processor_buffer.write().await;
                            std::mem::take(&mut *buf)
                        };

                        if !events.is_empty() {
                            if let Err(e) = exporter.export(&events).await {
                                tracing::warn!("Telemetry export failed: {}", e);
                            }
                        }
                    }
                    _ = processor_shutdown.notified() => {
                        // Flush remaining events on shutdown
                        let events = {
                            let mut buf = processor_buffer.write().await;
                            std::mem::take(&mut *buf)
                        };

                        if !events.is_empty() {
                            let _ = exporter.export(&events).await;
                        }
                        break;
                    }
                    Some(event) = event_rx.recv() => {
                        // Add event to buffer
                        let mut buf = processor_buffer.write().await;
                        buf.push(event);

                        // Flush if buffer is full
                        if buf.len() >= processor_config.batch_size {
                            let events = std::mem::take(&mut *buf);
                            if let Err(e) = exporter.export(&events).await {
                                tracing::warn!("Telemetry export failed: {}", e);
                            }
                        }
                    }
                }
            }
        });

        Self {
            config,
            event_tx,
            buffer,
            shutdown,
            sampler,
        }
    }

    /// Record a telemetry event
    pub fn record_event(&self, mut event: TelemetryEvent) -> TelemetryResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Apply sampling
        if !self.sample(&event) {
            return Ok(());
        }

        // Apply anonymization if configured
        if self.config.anonymize {
            self.anonymize_event(&mut event);
        }

        self.event_tx
            .try_send(event)
            .map_err(|_| TelemetryError::ChannelFull)?;

        Ok(())
    }

    /// Sample an event based on sample rate
    fn sample(&self, _event: &TelemetryEvent) -> bool {
        self.sample.sample()
    }

    /// Anonymize event data
    fn anonymize_event(&self, event: &mut TelemetryEvent) {
        use sha2::{Sha256, Digest};

        // Hash user ID if present
        if let Some(ref user_id) = event.user_id {
            let mut hasher = Sha256::new();
            hasher.update(user_id.as_bytes());
            event.user_id = Some(format!("{:x}", hasher.finalize())[..16].to_string());
        }

        // Hash project ID if present
        if let Some(ref project_id) = event.project_id {
            let mut hasher = Sha256::new();
            hasher.update(project_id.as_bytes());
            event.project_id = Some(format!("{:x}", hasher.finalize())[..16].to_string());
        }

        // Filter sensitive environment variables
        let sensitive_patterns = ["PASSWORD", "SECRET", "TOKEN", "KEY", "CREDENTIAL", "PRIVATE"];
        event.context.environment.retain(|k, _| {
            !sensitive_patterns.iter().any(|p| k.to_uppercase().contains(p))
        });
    }

    /// Record build started event
    pub fn record_build_started(&self, build_id: &str, target: Option<&str>, profile: &str, features: &[String]) {
        let event = TelemetryEvent {
            event_id: EventId::new(),
            event_type: EventType::BuildStarted,
            timestamp: Utc::now(),
            session_id: self.generate_session_id(),
            user_id: self.get_anonymized_user_id(),
            project_id: Some(self.generate_project_id()),
            properties: std::collections::HashMap::from([
                ("build_id".to_string(), PropertyValue::String(build_id.to_string())),
                ("target".to_string(), PropertyValue::String(target.unwrap_or("").to_string())),
                ("profile".to_string(), PropertyValue::String(profile.to_string())),
                ("features".to_string(), PropertyValue::Array(
                    features.iter()
                        .map(|f| PropertyValue::String(f.clone()))
                        .collect()
                )),
            ]),
            context: EventContext::default(),
        };

        let _ = self.record_event(event);
    }

    /// Record build completed event
    pub fn record_build_completed(&self, build_id: &str, data: &crate::BuildEventData) {
        let event = TelemetryEvent {
            event_id: EventId::new(),
            event_type: EventType::BuildCompleted,
            timestamp: Utc::now(),
            session_id: self.generate_session_id(),
            user_id: self.get_anonymized_user_id(),
            project_id: Some(self.generate_project_id()),
            properties: std::collections::HashMap::from([
                ("build_id".to_string(), PropertyValue::String(build_id.to_string())),
                ("duration_ms".to_string(), PropertyValue::Number(data.duration_ms as f64)),
                ("crates_compiled".to_string(), PropertyValue::Number(data.crates_compiled as f64)),
                ("cache_hit_rate".to_string(), PropertyValue::Number(data.cache_stats.hit_rate)),
                ("cpu_max".to_string(), PropertyValue::Number(data.resource_usage.cpu_max_percent)),
                ("memory_max_mb".to_string(), PropertyValue::Number(data.resource_usage.memory_max_mb as f64)),
            ]),
            context: EventContext::default(),
        };

        let _ = self.record_event(event);
    }

    /// Record build failed event
    pub fn record_build_failed(&self, build_id: &str, error_type: &str, error_message: &str) {
        if !self.config.error_reporting {
            return;
        }

        let event = TelemetryEvent {
            event_id: EventId::new(),
            event_type: EventType::BuildFailed,
            timestamp: Utc::now(),
            session_id: self.generate_session_id(),
            user_id: self.get_anonymized_user_id(),
            project_id: Some(self.generate_project_id()),
            properties: std::collections::HashMap::from([
                ("build_id".to_string(), PropertyValue::String(build_id.to_string())),
                ("error_type".to_string(), PropertyValue::String(error_type.to_string())),
                ("error_message".to_string(), PropertyValue::String(error_message.to_string())),
            ]),
            context: EventContext::default(),
        };

        let _ = self.record_event(event);
    }

    /// Generate a session ID
    fn generate_session_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("session_{}", timestamp)
    }

    /// Get anonymized user ID
    fn get_anonymized_user_id(&self) -> Option<String> {
        if !self.config.anonymize {
            return None;
        }

        // Generate from machine fingerprint (simplified)
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"utm-dev-user");
        Some(format!("{:x}", hasher.finalize())[..16].to_string())
    }

    /// Generate project ID
    fn generate_project_id(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"utm-dev-project");
        format!("{:x}", hasher.finalize())[..16].to_string()
    }

    /// Shutdown the collector gracefully
    pub async fn shutdown(self) {
        self.shutdown.notify_one();
        // Allow time for final flush
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Build event data for recording
pub struct BuildEventData {
    pub duration_ms: u64,
    pub crates_compiled: u32,
    pub cache_stats: CacheStats,
    pub resource_usage: ResourceUsage,
}

pub struct CacheStats {
    pub hits: u32,
    pub misses: u32,
    pub hit_rate: f64,
}

pub struct ResourceUsage {
    pub cpu_max_percent: f64,
    pub memory_max_mb: u64,
}
```

```rust
// utm-telemetry/src/exporter.rs
use reqwest::Client;
use utm_observability_core::{TelemetryEvent, TelemetryResult};
use crate::TelemetryConfig;

/// Batch exporter for telemetry events
pub struct BatchExporter {
    config: TelemetryConfig,
    client: Client,
}

impl BatchExporter {
    pub fn new(config: &TelemetryConfig) -> Self {
        Self {
            config: config.clone(),
            client: Client::new(),
        }
    }

    /// Export a batch of events
    pub async fn export(&self, events: &[TelemetryEvent]) -> TelemetryResult<()> {
        if let Some(endpoint) = &self.config.endpoint {
            let payload = serde_json::json!({
                "events": events,
                "api_version": "1.0",
            });

            let mut request = self.client.post(endpoint).json(&payload);

            if let Some(api_key) = &self.config.api_key {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }

            let response = request.send().await
                .map_err(|e| utm_observability_core::TelemetryError::HttpError(e.to_string()))?;

            if !response.status().is_success() {
                tracing::warn!("Telemetry export failed with status: {}", response.status());
            } else {
                tracing::debug!("Exported {} telemetry events", events.len());
            }
        }

        Ok(())
    }
}
```

```rust
// utm-telemetry/src/sampler.rs
/// Sampler for telemetry events
pub struct Sampler {
    sample_rate: f64,
}

impl Sampler {
    pub fn new(sample_rate: f64) -> Self {
        Self { sample_rate }
    }

    /// Sample an event based on sample rate
    pub fn sample(&self) -> bool {
        if self.sample_rate >= 1.0 {
            return true;
        }

        rand::random::<f64>() <= self.sample_rate
    }
}
```

## Performance Profiler (utm-profiler)

```rust
// utm-profiler/src/system.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{Duration, Instant};
use sysinfo::{ProcessExt, System, SystemExt, CpuExt};
use utm_observability_core::TelemetryResult;

/// High-resolution system profiler
pub struct SystemProfiler {
    sampling_interval: Duration,
    metrics: Arc<RwLock<ProfilerMetrics>>,
    running: Arc<std::sync::AtomicBool>,
}

/// Profiler metrics data
pub struct ProfilerMetrics {
    /// CPU samples
    pub cpu_samples: Vec<CpuSample>,

    /// Memory samples
    pub memory_samples: Vec<MemorySample>,

    /// I/O samples
    pub io_samples: Vec<IoSample>,
}

/// CPU sample data
#[derive(Debug, Clone)]
pub struct CpuSample {
    pub timestamp: Instant,
    pub usage_percent: f64,
    pub per_core: Vec<f64>,
    pub load_avg_1m: f64,
    pub load_avg_5m: f64,
    pub load_avg_15m: f64,
}

/// Memory sample data
#[derive(Debug, Clone)]
pub struct MemorySample {
    pub timestamp: Instant,
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub utm_process_mb: u64,
}

/// I/O sample data
#[derive(Debug, Clone)]
pub struct IoSample {
    pub timestamp: Instant,
    pub read_bytes_sec: u64,
    pub write_bytes_sec: u64,
    pub read_ops_sec: u64,
    pub write_ops_sec: u64,
}

impl SystemProfiler {
    /// Create a new system profiler
    pub fn new(sampling_interval: Duration) -> Self {
        Self {
            sampling_interval,
            metrics: Arc::new(RwLock::new(ProfilerMetrics {
                cpu_samples: Vec::new(),
                memory_samples: Vec::new(),
                io_samples: Vec::new(),
            })),
            running: Arc::new(std::sync::AtomicBool::new(false)),
        }
    }

    /// Start profiling
    pub fn start(&self) {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);

        let metrics = Arc::clone(&self.metrics);
        let running = Arc::clone(&self.running);
        let interval = self.sampling_interval;

        tokio::spawn(async move {
            let mut system = System::new_all();
            let mut ticker = tokio::time::interval(interval);

            while running.load(std::sync::atomic::Ordering::SeqCst) {
                ticker.tick().await;

                // Refresh system info
                system.refresh_all();

                let sample_time = Instant::now();

                // Collect CPU metrics
                let cpu_sample = CpuSample {
                    timestamp: sample_time,
                    usage_percent: system.global_cpu_usage() as f64,
                    per_core: system.cpus().iter().map(|c| c.cpu_usage() as f64).collect(),
                    load_avg_1m: sysinfo::get_load_average().one,
                    load_avg_5m: sysinfo::get_load_average().five,
                    load_avg_15m: sysinfo::get_load_average().fifteen,
                };

                // Collect memory metrics
                let mem_info = MemorySample {
                    timestamp: sample_time,
                    total_mb: system.total_memory() / 1024,
                    used_mb: system.used_memory() / 1024,
                    available_mb: system.available_memory() / 1024,
                    utm_process_mb: get_process_memory(&system),
                };

                // Collect I/O metrics (simplified)
                let io_sample = IoSample {
                    timestamp: sample_time,
                    read_bytes_sec: 0,
                    write_bytes_sec: 0,
                    read_ops_sec: 0,
                    write_ops_sec: 0,
                };

                // Store samples
                let mut metrics_guard = metrics.write().await;
                metrics_guard.cpu_samples.push(cpu_sample);
                metrics_guard.memory_samples.push(mem_info);
                metrics_guard.io_samples.push(io_sample);

                // Trim old samples (keep last 5 minutes)
                let cutoff = sample_time - Duration::from_secs(300);
                metrics_guard.cpu_samples.retain(|s| s.timestamp > cutoff);
                metrics_guard.memory_samples.retain(|s| s.timestamp > cutoff);
                metrics_guard.io_samples.retain(|s| s.timestamp > cutoff);
            }
        });
    }

    /// Stop profiling
    pub fn stop(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Get current metrics
    pub async fn get_current_metrics(&self) -> ProfilerMetrics {
        self.metrics.read().await.clone()
    }

    /// Get peak memory usage
    pub async fn get_peak_memory(&self) -> u64 {
        self.metrics.read().await.memory_samples.iter()
            .map(|s| s.utm_process_mb)
            .max()
            .unwrap_or(0)
    }

    /// Get average CPU usage
    pub async fn get_avg_cpu(&self) -> f64 {
        let metrics = self.metrics.read().await;
        if metrics.cpu_samples.is_empty() {
            return 0.0;
        }

        metrics.cpu_samples.iter()
            .map(|s| s.usage_percent)
            .sum::<f64>() / metrics.cpu_samples.len() as f64
    }
}

/// Get memory usage of current process
fn get_process_memory(system: &System) -> u64 {
    let pid = sysinfo::get_current_pid().ok()?;
    system.process(pid).map(|p| p.memory() / 1024 / 1024).unwrap_or(0)
}
```

```rust
// utm-profiler/src/phase.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::system::{SystemProfiler, ProfilerMetrics};

/// Build phase profiler
pub struct BuildPhaseProfiler {
    phases: HashMap<String, PhaseMetrics>,
    current_phase: Option<(String, Instant)>,
}

/// Phase metrics
#[derive(Debug, Clone)]
pub struct PhaseMetrics {
    pub name: String,
    pub samples: Vec<PhaseSample>,
}

/// Phase sample data
#[derive(Debug, Clone)]
pub struct PhaseSample {
    pub duration: Duration,
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub io_read_mb: f64,
    pub io_write_mb: f64,
}

impl BuildPhaseProfiler {
    /// Create a new build phase profiler
    pub fn new() -> Self {
        Self {
            phases: HashMap::new(),
            current_phase: None,
        }
    }

    /// Start profiling a build phase
    pub fn start_phase(&mut self, name: &str) {
        self.current_phase = Some((name.to_string(), Instant::now()));
    }

    /// End the current build phase
    pub async fn end_phase(&mut self, profiler: &SystemProfiler) -> PhaseMetrics {
        if let Some((name, start_time)) = self.current_phase.take() {
            let duration = start_time.elapsed();

            // Get ending metrics
            let end_metrics = profiler.get_current_metrics().await;

            // Calculate phase metrics
            let phase_metrics = PhaseMetrics {
                name: name.clone(),
                samples: vec![PhaseSample {
                    duration,
                    cpu_percent: end_metrics.cpu_samples.last()
                        .map(|s| s.usage_percent)
                        .unwrap_or(0.0),
                    memory_mb: end_metrics.memory_samples.last()
                        .map(|s| s.utm_process_mb)
                        .unwrap_or(0),
                    io_read_mb: end_metrics.io_samples.last()
                        .map(|s| s.read_bytes_sec as f64 / 1024.0 / 1024.0)
                        .unwrap_or(0.0),
                    io_write_mb: end_metrics.io_samples.last()
                        .map(|s| s.write_bytes_sec as f64 / 1024.0 / 1024.0)
                        .unwrap_or(0.0),
                }],
            };

            self.phases.insert(name, phase_metrics.clone());
            phase_metrics
        } else {
            PhaseMetrics {
                name: String::new(),
                samples: Vec::new(),
            }
        }
    }

    /// Get phase summary
    pub fn get_phase_summary(&self) -> Vec<PhaseMetrics> {
        self.phases.values().cloned().collect()
    }
}

impl Default for BuildPhaseProfiler {
    fn default() -> Self {
        Self::new()
    }
}
```

## Error Tracker (utm-error-tracker)

```rust
// utm-error-tracker/src/tracker.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use backtrace::Backtrace;
use utm_observability_core::{TelemetryResult, TelemetryError};
use crate::rate_limit::RateLimiter;
use crate::report::{ErrorReport, ErrorContext, ErrorSeverity};

/// Error tracker configuration
#[derive(Debug, Clone)]
pub struct ErrorTrackerConfig {
    pub enabled: bool,
    pub dsn: Option<String>,
    pub environment: String,
    pub release: String,
    pub sample_rate: f64,
    pub max_pending: usize,
    pub include_backtrace: bool,
}

impl Default for ErrorTrackerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dsn: None,
            environment: "production".to_string(),
            release: env!("CARGO_PKG_VERSION").to_string(),
            sample_rate: 1.0,
            max_pending: 100,
            include_backtrace: true,
        }
    }
}

/// Error tracker for capturing and reporting errors
pub struct ErrorTracker {
    config: ErrorTrackerConfig,
    pending_reports: Arc<RwLock<Vec<ErrorReport>>>,
    rate_limiter: RateLimiter,
}

impl ErrorTracker {
    /// Create a new error tracker
    pub fn new(config: ErrorTrackerConfig) -> Self {
        Self {
            config,
            pending_reports: Arc::new(RwLock::new(Vec::new())),
            rate_limiter: RateLimiter::new(10, Duration::from_secs(60)),
        }
    }

    /// Capture an error
    pub fn capture_error(
        &self,
        error: &dyn std::error::Error,
        context: ErrorContext,
        severity: ErrorSeverity,
    ) -> Option<Uuid> {
        if !self.config.enabled {
            return None;
        }

        // Rate limiting
        if !self.rate_limiter.allow() {
            return None;
        }

        // Sampling
        if self.config.sample_rate < 1.0 && rand::random::<f64>() > self.config.sample_rate {
            return None;
        }

        let backtrace = if self.config.include_backtrace {
            format!("{:?}", Backtrace::new())
        } else {
            String::new()
        };

        let fingerprint = self.generate_fingerprint(error, &context);

        let report = ErrorReport {
            error_id: Uuid::new_v4(),
            error_type: std::any::type_name_of_val(error).to_string(),
            message: error.to_string(),
            backtrace,
            timestamp: Utc::now(),
            fingerprint,
            severity,
            context,
        };

        let report_id = report.error_id;

        // Store pending report
        let mut pending = self.pending_reports.blocking_write();
        pending.push(report);

        if pending.len() >= self.config.max_pending {
            // Flush oldest reports
            self.flush_reports();
        }

        Some(report_id)
    }

    /// Generate error fingerprint for grouping
    fn generate_fingerprint(&self, error: &dyn std::error::Error, context: &ErrorContext) -> String {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(std::any::type_name_of_val(error).as_bytes());
        hasher.update(error.to_string().as_bytes());

        format!("{:x}", hasher.finalize())
    }

    /// Flush pending reports
    async fn flush_reports(&self) {
        let reports = {
            let mut pending = self.pending_reports.write().await;
            std::mem::take(&mut *pending)
        };

        if let Some(dsn) = &self.config.dsn {
            let client = reqwest::Client::new();

            for report in reports {
                let payload = serde_json::json!({
                    "event_id": report.error_id.to_string(),
                    "level": match report.severity {
                        ErrorSeverity::Debug => "debug",
                        ErrorSeverity::Info => "info",
                        ErrorSeverity::Warning => "warning",
                        ErrorSeverity::Error => "error",
                        ErrorSeverity::Fatal => "fatal",
                    },
                    "exception": {
                        "type": report.error_type,
                        "value": report.message,
                        "stacktrace": report.backtrace,
                    },
                    "fingerprint": [report.fingerprint],
                    "timestamp": report.timestamp.to_rfc3339(),
                    "environment": self.config.environment,
                    "release": self.config.release,
                });

                let _ = client.post(dsn).json(&payload).send().await;
            }
        }
    }
}
```

## Alerting System (utm-alerting)

```rust
// utm-alerting/src/rules.rs
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Alert rule definition
#[derive(Debug, Clone)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub cooldown: Duration,
    pub notification_channels: Vec<NotificationChannel>,
}

/// Alert condition types
#[derive(Debug, Clone)]
pub enum AlertCondition {
    /// Threshold-based alert
    Threshold {
        metric: String,
        operator: ThresholdOperator,
        value: f64,
        for_duration: Duration,
    },

    /// Rate of change alert
    RateOfChange {
        metric: String,
        increase_percent: f64,
        window: Duration,
    },

    /// Anomaly detection alert
    Anomaly {
        metric: String,
        sensitivity: f64,
        baseline_window: Duration,
    },

    /// Absence detection (no data)
    Absence {
        metric: String,
        timeout: Duration,
    },
}

/// Threshold operators
#[derive(Debug, Clone)]
pub enum ThresholdOperator {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Notification channel types
#[derive(Debug, Clone)]
pub enum NotificationChannel {
    Slack { webhook_url: String, channel: String },
    Email { recipients: Vec<String> },
    PagerDuty { routing_key: String },
    Webhook { url: String, headers: std::collections::HashMap<String, String> },
}

/// Built-in alert rules
pub mod builtins {
    use super::*;

    /// Build failure rate alert
    pub fn build_failure_rate() -> AlertRule {
        AlertRule {
            id: "build-failure-rate".to_string(),
            name: "High Build Failure Rate".to_string(),
            description: "More than 20% of builds are failing".to_string(),
            condition: AlertCondition::Threshold {
                metric: "build_failure_rate".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 20.0,
                for_duration: Duration::from_secs(300),
            },
            severity: AlertSeverity::High,
            cooldown: Duration::from_secs(600),
            notification_channels: vec![],
        }
    }

    /// Slow build alert
    pub fn slow_build() -> AlertRule {
        AlertRule {
            id: "slow-build".to_string(),
            name: "Build Duration Exceeded".to_string(),
            description: "Build taking longer than expected".to_string(),
            condition: AlertCondition::Threshold {
                metric: "build_duration_p95".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 600000.0,
                for_duration: Duration::from_secs(0),
            },
            severity: AlertSeverity::Medium,
            cooldown: Duration::from_secs(1800),
            notification_channels: vec![],
        }
    }

    /// Cache degradation alert
    pub fn cache_degradation() -> AlertRule {
        AlertRule {
            id: "cache-degradation".to_string(),
            name: "Cache Hit Rate Degraded".to_string(),
            description: "Cache hit rate has dropped significantly".to_string(),
            condition: AlertCondition::RateOfChange {
                metric: "cache_hit_rate".to_string(),
                increase_percent: -30.0,
                window: Duration::from_secs(3600),
            },
            severity: AlertSeverity::Medium,
            cooldown: Duration::from_secs(3600),
            notification_channels: vec![],
        }
    }
}
```

```rust
// utm-alerting/src/evaluator.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use crate::rules::{AlertRule, AlertCondition, ThresholdOperator, AlertSeverity};
use crate::notify::{Alert, NotificationSender};

/// Alert evaluator for checking rules against metrics
pub struct AlertEvaluator {
    rules: Vec<AlertRule>,
    alert_state: HashMap<String, AlertState>,
    notification_sender: NotificationSender,
}

struct AlertState {
    last_triggered: Option<Instant>,
    is_firing: bool,
    current_value: Option<f64>,
    for_start: Option<Instant>,
}

impl AlertEvaluator {
    pub fn new(rules: Vec<AlertRule>) -> Self {
        let notification_sender = NotificationSender::new(
            rules.iter()
                .flat_map(|r| r.notification_channels.clone())
                .collect()
        );

        Self {
            rules,
            alert_state: HashMap::new(),
            notification_sender,
        }
    }

    /// Evaluate all rules against current metrics
    pub async fn evaluate(&mut self, metrics: &dyn MetricsStore) -> Vec<Alert> {
        let mut alerts = Vec::new();

        for rule in &self.rules {
            if let Some(alert) = self.evaluate_rule(rule, metrics).await {
                alerts.push(alert);
            }
        }

        alerts
    }

    async fn evaluate_rule(&mut self, rule: &AlertRule, metrics: &dyn MetricsStore) -> Option<Alert> {
        let state = self.alert_state.entry(rule.id.clone()).or_insert(AlertState {
            last_triggered: None,
            is_firing: false,
            current_value: None,
            for_start: None,
        });

        // Check cooldown
        if let Some(last) = state.last_triggered {
            if last.elapsed() < rule.cooldown {
                return None;
            }
        }

        // Get metric value
        let value = match self.get_metric_value(&rule.condition, metrics).await {
            Some(v) => v,
            None => return None,
        };
        state.current_value = Some(value);

        // Evaluate condition
        let should_fire = match &rule.condition {
            AlertCondition::Threshold { operator, value: threshold, for_duration, .. } => {
                let passes = match operator {
                    ThresholdOperator::GreaterThan => value > *threshold,
                    ThresholdOperator::GreaterThanOrEqual => value >= *threshold,
                    ThresholdOperator::LessThan => value < *threshold,
                    ThresholdOperator::LessThanOrEqual => value <= *threshold,
                    ThresholdOperator::Equal => (value - threshold).abs() < 0.001,
                };

                if passes {
                    if for_duration.is_zero() {
                        true
                    } else if state.for_start.is_none() {
                        state.for_start = Some(Instant::now());
                        false
                    } else {
                        state.for_start.unwrap().elapsed() >= *for_duration
                    }
                } else {
                    state.for_start = None;
                    false
                }
            }
            _ => false, // Simplified for brevity
        };

        if should_fire && !state.is_firing {
            state.is_firing = true;
            state.last_triggered = Some(Instant::now());

            Some(Alert {
                rule_id: rule.id.clone(),
                rule_name: rule.name.clone(),
                severity: rule.severity,
                value,
                triggered_at: Utc::now(),
            })
        } else if !should_fire && state.is_firing {
            state.is_firing = false;
            None
        } else {
            None
        }
    }

    async fn get_metric_value(&self, condition: &AlertCondition, metrics: &dyn MetricsStore) -> Option<f64> {
        metrics.get_current(condition.metric()).await
    }
}

/// Metrics store trait for alert evaluation
pub trait MetricsStore: Send + Sync {
    async fn get_current(&self, metric: &str) -> Option<f64>;
}
```

## Code Examples

### Full Telemetry Integration

```rust
use utm_telemetry::{TelemetryCollector, TelemetryConfig};
use utm_profiler::{SystemProfiler, BuildPhaseProfiler};
use utm_error_tracker::{ErrorTracker, ErrorTrackerConfig};
use utm_alerting::rules::builtins;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize telemetry collector
    let telemetry_config = TelemetryConfig {
        enabled: true,
        sample_rate: 1.0,
        endpoint: Some("https://telemetry.example.com/api/v1/events".to_string()),
        api_key: Some("your-api-key".to_string()),
        ..Default::default()
    };
    let telemetry = TelemetryCollector::new(telemetry_config);

    // Initialize system profiler
    let profiler = SystemProfiler::new(Duration::from_secs(1));
    profiler.start();

    // Initialize error tracker
    let error_tracker = ErrorTracker::new(ErrorTrackerConfig::default());

    // Record build started
    telemetry.record_build_started(
        "build-123",
        Some("x86_64-unknown-linux-gnu"),
        "release",
        &["feature1".to_string(), "feature2".to_string()],
    );

    // Profile build phases
    let mut phase_profiler = BuildPhaseProfiler::new();
    phase_profiler.start_phase("compile");

    // Simulate build work
    tokio::time::sleep(Duration::from_secs(2)).await;

    let phase_metrics = phase_profiler.end_phase(&profiler).await;
    println!("Compile phase took {:?}", phase_metrics.samples[0].duration);

    // Record build completed
    telemetry.record_build_completed("build-123", &utm_telemetry::BuildEventData {
        duration_ms: 5000,
        crates_compiled: 100,
        cache_stats: utm_telemetry::CacheStats {
            hits: 80,
            misses: 20,
            hit_rate: 0.8,
        },
        resource_usage: utm_telemetry::ResourceUsage {
            cpu_max_percent: 95.0,
            memory_max_mb: 2048,
        },
    });

    // Shutdown gracefully
    profiler.stop();
    telemetry.shutdown().await;

    Ok(())
}
```

### Error Tracking Integration

```rust
use utm_error_tracker::{ErrorTracker, ErrorTrackerConfig, ErrorContext, ErrorSeverity};

fn run_build() -> Result<(), BuildError> {
    // Build logic
    Err(BuildError::CompilationFailed("error".to_string()))
}

fn main() {
    let error_tracker = ErrorTracker::new(ErrorTrackerConfig::default());

    if let Err(e) = run_build() {
        error_tracker.capture_error(
            &e,
            ErrorContext {
                build_id: Some("build-123".to_string()),
                command: Some("build".to_string()),
                ..Default::default()
            },
            ErrorSeverity::Error,
        );
    }
}
```

## Migration Path

### Phase 1: Core Infrastructure (Week 1-2)
- Implement `utm-observability-core` crate
- Set up event types and error handling
- Create basic telemetry collector

### Phase 2: Profiling (Week 3-4)
- Implement system profiler with sysinfo
- Add build phase profiling
- Create flame graph generation

### Phase 3: Error Tracking (Week 5)
- Implement error tracker
- Add panic hook integration
- Create error reporting pipeline

### Phase 4: Metrics & Alerting (Week 6-7)
- Implement metrics aggregator
- Create alert rule engine
- Add notification channels

### Phase 5: Integration (Week 8)
- Integrate with utm-dev build system
- Set up dashboards
- Configure production alerting

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use utm_telemetry::{TelemetryCollector, TelemetryConfig};

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert!(config.enabled);
        assert_eq!(config.sample_rate, 1.0);
    }

    #[tokio::test]
    async fn test_telemetry_collector_creation() {
        let config = TelemetryConfig::default();
        let collector = TelemetryCollector::new(config);
        // Would test actual event recording
    }

    #[tokio::test]
    async fn test_error_tracker_rate_limiting() {
        let tracker = ErrorTracker::new(ErrorTrackerConfig::default());
        // Would test rate limiting behavior
    }
}
```

## Open Considerations

1. **Distributed Tracing**: Add OpenTelemetry integration for distributed tracing

2. **Log Aggregation**: Integrate with structured logging (tracing-subscriber)

3. **Metrics Backend**: Support for Prometheus, Grafana Loki, or TimescaleDB

4. **Real-time Dashboards**: WebSocket-based real-time metric streaming

5. **ML-based Anomaly Detection**: More sophisticated anomaly detection algorithms

6. **Privacy Compliance**: GDPR/CCPA compliance features for data retention

7. **Offline Mode**: Queue events when offline, sync when connected

8. **Cost Optimization**: Tiered telemetry with different detail levels
