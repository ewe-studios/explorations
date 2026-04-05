---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.extism/extism
repository: git@github.com:extism/extism.git
explored_at: 2026-04-04
---

# Extism Production Grade: Building Secure, Scalable Plugin Systems

## Overview

This guide covers production deployment patterns for Extism-based plugin systems. We address security sandboxing, performance optimization, observability, scaling strategies, and operational considerations for running WebAssembly plugins at scale.

## Architecture Overview

```mermaid
flowchart TB
    subgraph Edge Layer
        A[Load Balancer] --> B[API Gateway]
    end
    
    subgraph Application Tier
        B --> C[App Server 1]
        B --> D[App Server 2]
        B --> E[App Server N]
    end
    
    subgraph Extism Runtime (per server)
        C --> F[Plugin Pool]
        D --> G[Plugin Pool]
        E --> H[Plugin Pool]
    end
    
    subgraph Plugin Isolation
        F --> I[Plugin Instance 1]
        F --> J[Plugin Instance 2]
        G --> K[Plugin Instance 1]
        G --> L[Plugin Instance 2]
    end
    
    subgraph Shared Resources
        F --> M[(Module Cache)]
        G --> M
        H --> M
        F --> N[(Variable Store)]
        G --> N
        H --> N
    end
    
    subgraph Observability
        C --> O[Metrics]
        D --> O
        E --> O
        C --> P[Logging]
        D --> P
        E --> P
    end
```

## Security Considerations

### 1. Plugin Isolation

```rust
use extism::{Plugin, Manifest, Wasm};
use std::time::Duration;

/// Secure plugin configuration
pub struct SecurePluginConfig {
    /// Maximum memory per plugin
    pub max_memory: u64,
    /// Maximum execution time
    pub timeout: Duration,
    /// Allowed HTTP hosts (empty = none)
    pub allowed_hosts: Vec<String>,
    /// Allowed filesystem paths
    pub allowed_paths: std::collections::HashMap<String, String>,
    /// Enable WASI (default: false for security)
    pub enable_wasi: bool,
}

impl Default for SecurePluginConfig {
    fn default() -> Self {
        Self {
            max_memory: 32 * 1024 * 1024, // 32MB
            timeout: Duration::from_secs(5),
            allowed_hosts: Vec::new(),
            allowed_paths: std::collections::HashMap::new(),
            enable_wasi: false,
        }
    }
}

/// Create a securely configured plugin
pub fn create_secure_plugin(
    wasm: Wasm,
    config: SecurePluginConfig,
) -> Result<Plugin, extism::Error> {
    let manifest = Manifest::new([wasm])
        .with_memory_limit(config.max_memory)
        .with_allowed_hosts(config.allowed_hosts)
        .with_allowed_paths(config.allowed_paths);
    
    let mut plugin = Plugin::new(&manifest, [], true)?;
    
    // Set execution timeout
    plugin.set_timeout(config.timeout);
    
    // Disable WASI if not explicitly enabled
    if !config.enable_wasi {
        // Extism disables WASI by default, but be explicit
        // about not registering any WASI functions
    }
    
    Ok(plugin)
}
```

### 2. Input Validation

```rust
use extism::{CurrentPlugin, Error, Val};
use serde::Deserialize;

/// Maximum input sizes
const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_OUTPUT_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_STRING_LENGTH: usize = 1 * 1024 * 1024; // 1MB

/// Validate and read plugin input
pub fn validate_input(plugin: &mut CurrentPlugin) -> Result<Vec<u8>, Error> {
    let offset = plugin.input_offset();
    let length = plugin.input_length() as usize;
    
    // Size validation
    if length > MAX_INPUT_SIZE {
        return Err(Error::msg(format!(
            "Input size {} exceeds maximum {}",
            length, MAX_INPUT_SIZE
        )));
    }
    
    // Read and validate content
    let data = plugin.memory_get::<Vec<u8>>(offset)?;
    
    Ok(data)
}

/// Validate JSON input against schema
#[derive(Deserialize)]
struct ValidatedInput {
    #[serde(rename = "action")]
    action: String,
    #[serde(rename = "payload")]
    payload: serde_json::Value,
}

pub fn validate_json_input(input: &[u8]) -> Result<ValidatedInput, Error> {
    // Check size before parsing
    if input.len() > MAX_INPUT_SIZE {
        return Err(Error::msg("JSON input too large"));
    }
    
    // Parse and validate
    serde_json::from_slice(input)
        .map_err(|e| Error::msg(format!("Invalid JSON: {}", e)))
}

/// Sanitize string output
pub fn sanitize_output(output: &str) -> String {
    // Remove potential XSS vectors
    output
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
```

### 3. Host Function Security

```rust
use extism::{CurrentPlugin, Error, Val};
use std::sync::Arc;

/// Secure host function context
pub struct HostContext {
    /// API keys (if needed by plugins)
    api_keys: std::collections::HashMap<String, String>,
    /// Rate limiter
    rate_limiter: Arc<tokio::sync::Semaphore>,
    /// Audit logger
    audit_logger: Arc<dyn AuditLogger + Send + Sync>,
}

/// Audit log trait
pub trait AuditLogger {
    fn log(&self, plugin: &str, function: &str, input_size: usize, success: bool);
}

/// Safe database query host function
pub fn safe_db_query(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
) -> Result<(), Error> {
    // Read input
    let offset = inputs[0].unwrap_i64() as u64;
    let query_input = plugin.memory_get::<Vec<u8>>(offset)?;
    
    // Validate size
    if query_input.len() > 4096 {
        return Err(Error::msg("Query input too large"));
    }
    
    // Validate content (example: only allow specific query types)
    let query_str = String::from_utf8(query_input)
        .map_err(|_| Error::msg("Invalid UTF-8 in query"))?;
    
    // Sanitize - in production, use parameterized queries
    if query_str.contains(';') || query_str.contains("--") {
        return Err(Error::msg("Potentially malicious query rejected"));
    }
    
    // Execute with limits
    // let result = database.query(&query_str)?;
    
    Ok(())
}

/// Rate-limited HTTP request
pub async fn rate_limited_http(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    context: Arc<HostContext>,
) -> Result<(), Error> {
    // Acquire rate limit permit
    let _permit = context.rate_limiter.acquire().await
        .map_err(|_| Error::msg("Rate limit service unavailable"))?;
    
    // Validate and execute request
    // ... implementation
    
    Ok(())
}
```

### 4. Capability-Based Security

```rust
use extism::Manifest;

/// Capability configuration for a plugin
#[derive(Debug, Clone)]
pub struct PluginCapabilities {
    /// Can make HTTP requests
    pub http: HttpCapabilities,
    /// Can access filesystem
    pub filesystem: FsCapabilities,
    /// Can access environment variables
    pub env: bool,
    /// Can access random number generator
    pub random: bool,
}

#[derive(Debug, Clone, Default)]
pub struct HttpCapabilities {
    /// Allowed hosts (empty = no HTTP)
    pub allowed_hosts: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Max request body size
    pub max_body_size: usize,
}

#[derive(Debug, Clone, Default)]
pub struct FsCapabilities {
    /// Allowed paths (guest -> host mapping)
    pub allowed_paths: std::collections::HashMap<String, String>,
    /// Read-only mode
    pub read_only: bool,
}

impl PluginCapabilities {
    /// Create a manifest with these capabilities
    pub fn to_manifest(&self, wasm: extism::Wasm) -> Manifest {
        let mut manifest = Manifest::new([wasm]);
        
        if !self.http.allowed_hosts.is_empty() {
            manifest = manifest.with_allowed_hosts(self.http.allowed_hosts.clone());
        }
        
        if !self.fs.allowed_paths.is_empty() {
            manifest = manifest.with_allowed_paths(self.fs.allowed_paths.clone());
        }
        
        manifest
    }
    
    /// Minimal capabilities (no network, no filesystem)
    pub fn minimal() -> Self {
        Self {
            http: HttpCapabilities::default(),
            filesystem: FsCapabilities::default(),
            env: false,
            random: true, // Usually safe to allow
        }
    }
    
    /// Full capabilities (use with caution)
    pub fn full() -> Self {
        Self {
            http: HttpCapabilities {
                allowed_hosts: vec!["*".to_string()],
                allowed_methods: vec!["GET".into(), "POST".into(), "PUT".into(), "DELETE".into()],
                max_body_size: 10 * 1024 * 1024,
            },
            filesystem: FsCapabilities {
                allowed_paths: [("/tmp".into(), "/tmp".into())].into_iter().collect(),
                read_only: false,
            },
            env: true,
            random: true,
        }
    }
}
```

## Performance Optimization

### 1. Plugin Pooling

```rust
use extism::{Plugin, Manifest, Wasm, Error};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Plugin pool for efficient reuse
pub struct PluginPool {
    /// Pool of idle plugins
    idle: Arc<Mutex<Vec<Plugin>>>,
    /// Semaphore limiting concurrent plugins
    semaphore: Arc<Semaphore>,
    /// Manifest for creating new instances
    manifest: Arc<Manifest>,
    /// Maximum pool size
    max_size: usize,
}

impl PluginPool {
    /// Create a new plugin pool
    pub async fn new(manifest: Manifest, max_size: usize) -> Result<Self, Error> {
        let mut idle = Vec::with_capacity(max_size);
        
        // Pre-warm the pool
        for _ in 0..max_size {
            let plugin = Plugin::new(&manifest, [], true)?;
            idle.push(plugin);
        }
        
        Ok(Self {
            idle: Arc::new(Mutex::new(idle)),
            semaphore: Arc::new(Semaphore::new(max_size)),
            manifest: Arc::new(manifest),
            max_size,
        })
    }
    
    /// Acquire a plugin from the pool
    pub async fn acquire(&self) -> Result<PluginGuard, Error> {
        // Wait for available slot
        let permit = self.semaphore.acquire().await
            .map_err(|_| Error::msg("Pool closed"))?;
        
        // Get plugin from pool or create new
        let plugin = {
            let mut idle = self.idle.lock().await;
            idle.pop()
        }.unwrap_or_else(|| {
            // Create new instance if pool empty
            Plugin::new(&self.manifest, [], true)
                .expect("Failed to create plugin")
        });
        
        Ok(PluginGuard {
            plugin: Some(plugin),
            pool: self.clone(),
            _permit: permit,
        })
    }
}

impl Clone for PluginPool {
    fn clone(&self) -> Self {
        Self {
            idle: Arc::clone(&self.idle),
            semaphore: Arc::clone(&self.semaphore),
            manifest: Arc::clone(&self.manifest),
            max_size: self.max_size,
        }
    }
}

/// Guard that returns plugin to pool on drop
pub struct PluginGuard {
    plugin: Option<Plugin>,
    pool: PluginPool,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl Drop for PluginGuard {
    fn drop(&mut self) {
        if let Some(plugin) = self.plugin.take() {
            // Return to pool (async, fire-and-forget)
            let idle = Arc::clone(&self.pool.idle);
            tokio::spawn(async move {
                let mut idle = idle.lock().await;
                if idle.len() < self.pool.max_size {
                    idle.push(plugin);
                }
                // Drop plugin if pool full
            });
        }
    }
}

/// Usage example
pub async fn handle_request(pool: &PluginPool, input: &[u8]) -> Result<Vec<u8>, Error> {
    let guard = pool.acquire().await?;
    let mut plugin = guard.plugin.unwrap();
    
    let result = plugin.call("handle", input)?;
    
    Ok(result)
}
```

### 2. Module Caching

```rust
use wasmtime::{Engine, Module};
use std::collections::HashMap;
use std::path::Path;
use sha2::{Sha256, Digest};
use tokio::sync::RwLock;

/// Module cache for faster instantiation
pub struct ModuleCache {
    engine: Engine,
    modules: RwLock<HashMap<Vec<u8>, Module>>,
    cache_dir: Option<std::path::PathBuf>,
}

impl ModuleCache {
    pub fn new(engine: Engine, cache_dir: Option<&Path>) -> Self {
        Self {
            engine,
            modules: RwLock::new(HashMap::new()),
            cache_dir: cache_dir.map(Path::to_path_buf),
        }
    }
    
    /// Get or compile a module
    pub async fn get_or_compile(&self, wasm_bytes: &[u8]) -> Result<Module, wasmtime::Error> {
        let hash = Self::hash(wasm_bytes);
        
        // Check memory cache
        {
            let modules = self.modules.read().await;
            if let Some(module) = modules.get(&hash) {
                return Ok(module.clone());
            }
        }
        
        // Check disk cache
        if let Some(cache_dir) = &self.cache_dir {
            let cached_path = cache_dir.join(hex::encode(&hash));
            if cached_path.exists() {
                if let Ok(cached_bytes) = tokio::fs::read(&cached_path).await {
                    let module = Module::from_binary(&self.engine, &cached_bytes)?;
                    
                    // Add to memory cache
                    let mut modules = self.modules.write().await;
                    modules.insert(hash, module.clone());
                    
                    return Ok(module);
                }
            }
        }
        
        // Compile new module
        let module = Module::from_binary(&self.engine, wasm_bytes)?;
        
        // Cache in memory
        let mut modules = self.modules.write().await;
        modules.insert(hash.clone(), module.clone());
        
        // Cache on disk
        if let Some(cache_dir) = &self.cache_dir {
            let cached_path = cache_dir.join(hex::encode(&hash));
            let _ = tokio::fs::write(&cached_path, wasm_bytes).await;
        }
        
        Ok(module)
    }
    
    fn hash(bytes: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hasher.finalize().to_vec()
    }
}
```

### 3. Async Execution

```rust
use extism::{Plugin, Error};
use tokio::task::spawn_blocking;

/// Execute plugin call asynchronously
pub async fn call_plugin_async(
    mut plugin: Plugin,
    function: &str,
    input: Vec<u8>,
) -> Result<Vec<u8>, Error> {
    // WASM execution is blocking, so run in thread pool
    spawn_blocking(move || {
        plugin.call(function, input)
    }).await?
}

/// Batch plugin calls for throughput
pub struct BatchExecutor {
    pool: crate::PluginPool,
    batch_size: usize,
}

impl BatchExecutor {
    pub fn new(pool: crate::PluginPool, batch_size: usize) -> Self {
        Self { pool, batch_size }
    }
    
    /// Execute multiple calls in parallel
    pub async fn execute_batch(
        &self,
        calls: Vec<(&str, Vec<u8>)>,
    ) -> Vec<Result<Vec<u8>, Error>> {
        // Split into batches
        let batches: Vec<_> = calls.chunks(self.batch_size).collect();
        
        let mut results = Vec::new();
        
        for batch in batches {
            // Execute batch in parallel
            let batch_results: Vec<_> = batch
                .iter()
                .map(|(func, input)| {
                    let pool = &self.pool;
                    let func = *func;
                    let input = input.clone();
                    tokio::spawn(async move {
                        let guard = pool.acquire().await?;
                        // ... execute call
                        Ok(Vec::new()) // placeholder
                    })
                })
                .collect();
            
            // Collect results
            for result in batch_results {
                results.push(result.await?);
            }
        }
        
        results
    }
}
```

### 4. Memory Management

```rust
use extism::Plugin;
use std::time::Duration;
use tokio::time::interval;

/// Memory monitoring and cleanup
pub struct MemoryManager {
    check_interval: Duration,
    max_memory_per_plugin: u64,
    max_total_memory: u64,
}

impl MemoryManager {
    pub fn new(
        check_interval: Duration,
        max_memory_per_plugin: u64,
        max_total_memory: u64,
    ) -> Self {
        Self {
            check_interval,
            max_memory_per_plugin,
            max_total_memory,
        }
    }
    
    /// Start memory monitoring
    pub async fn monitor(mut self, plugins: Arc<Mutex<Vec<Plugin>>>) {
        let mut interval = interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            let plugins_guard = plugins.lock().await;
            let mut total_memory = 0u64;
            
            for plugin in plugins_guard.iter() {
                // Note: Extism doesn't expose current memory usage directly
                // In production, you'd track allocations or use WASM runtime APIs
                total_memory += self.max_memory_per_plugin;
            }
            
            if total_memory > self.max_total_memory {
                // Trigger GC or reduce pool size
                tracing::warn!(
                    "Total plugin memory {} exceeds limit {}",
                    total_memory,
                    self.max_total_memory
                );
            }
        }
    }
}
```

## Observability

### 1. Metrics Collection

```rust
use prometheus::{IntCounter, IntGauge, Histogram, Registry};
use std::time::Instant;

/// Plugin metrics
pub struct PluginMetrics {
    /// Total calls by function
    calls_total: IntCounterVec,
    /// Call duration histogram
    call_duration: HistogramVec,
    /// Current active calls
    active_calls: IntGauge,
    /// Plugin errors by type
    errors_total: IntCounterVec,
    /// Memory usage
    memory_usage: IntGaugeVec,
}

impl PluginMetrics {
    pub fn new(registry: &Registry) -> Self {
        let calls_total = IntCounterVec::new(
            prometheus::Opts::new("plugin_calls_total", "Total plugin calls"),
            &["function", "plugin"],
        ).unwrap();
        
        let call_duration = HistogramVec::new(
            prometheus::HistogramOpts::new("plugin_call_duration_seconds", "Call duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["function", "plugin"],
        ).unwrap();
        
        let active_calls = IntGauge::new("plugin_active_calls", "Active calls").unwrap();
        
        let errors_total = IntCounterVec::new(
            prometheus::Opts::new("plugin_errors_total", "Total plugin errors"),
            &["function", "error_type"],
        ).unwrap();
        
        registry.register(Box::new(calls_total.clone())).unwrap();
        registry.register(Box::new(call_duration.clone())).unwrap();
        registry.register(Box::new(active_calls.clone())).unwrap();
        registry.register(Box::new(errors_total.clone())).unwrap();
        
        Self {
            calls_total,
            call_duration,
            active_calls,
            errors_total,
            memory_usage: IntGaugeVec::new(
                prometheus::Opts::new("plugin_memory_bytes", "Memory usage by plugin"),
                &["plugin"],
            ).unwrap(),
        }
    }
    
    /// Record a plugin call
    pub fn record_call<F, T>(&self, function: &str, plugin: &str, f: F) -> Result<T, extism::Error>
    where
        F: FnOnce() -> Result<T, extism::Error>,
    {
        let start = Instant::now();
        self.active_calls.inc();
        self.calls_total.with_label_values(&[function, plugin]).inc();
        
        let result = f();
        
        self.active_calls.dec();
        let duration = start.elapsed().as_secs_f64();
        self.call_duration
            .with_label_values(&[function, plugin])
            .observe(duration);
        
        if let Err(e) = &result {
            let error_type = self.classify_error(e);
            self.errors_total
                .with_label_values(&[function, &error_type])
                .inc();
        }
        
        result
    }
    
    fn classify_error(&self, error: &extism::Error) -> String {
        let msg = error.to_string();
        if msg.contains("timeout") {
            "timeout".into()
        } else if msg.contains("memory") {
            "memory".into()
        } else if msg.contains("function") {
            "not_found".into()
        } else {
            "other".into()
        }
    }
}
```

### 2. Structured Logging

```rust
use tracing::{info, warn, error, Span};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize logging
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,extism=debug"));
    
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

/// Plugin execution span
pub fn plugin_call_span(
    plugin_id: &str,
    function: &str,
    input_size: usize,
) -> Span {
    tracing::info_span!(
        "plugin_call",
        plugin_id = plugin_id,
        function = function,
        input_size = input_size,
    )
}

/// Example usage with tracing
#[tracing::instrument(skip(plugin, input), fields(plugin_id = "example", function = "process"))]
pub async fn instrumented_call(
    plugin: &mut Plugin,
    input: &[u8],
) -> Result<Vec<u8>, Error> {
    tracing::debug!("Calling plugin function");
    
    let result = plugin.call("process", input)?;
    
    tracing::debug!(
        output_size = result.len(),
        "Plugin call completed"
    );
    
    Ok(result)
}
```

### 3. Distributed Tracing

```rust
use tracing_opentelemetry::OpenTelemetrySpanExt;
use opentelemetry::{global, trace::Tracer};

/// Trace plugin execution
pub fn traced_plugin_call(
    plugin: &mut Plugin,
    function: &str,
    input: &[u8],
) -> Result<Vec<u8>, Error> {
    let tracer = global::tracer("extism");
    
    let span = tracer
        .span_builder(format!("plugin.{}", function))
        .with_kind(opentelemetry::trace::SpanKind::Internal)
        .start(&tracer);
    
    let _guard = tracer.in_span(format!("plugin.{}", function), |cx| {
        // Execute call
        plugin.call(function, input)
    });
    
    // Result handled by span guard
}
```

## Testing Strategies

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use extism::{Manifest, Wasm, Plugin};
    
    #[test]
    fn test_basic_plugin() {
        let manifest = Manifest::new([Wasm::file("tests/plugins/greeter.wasm")]);
        let mut plugin = Plugin::new(&manifest, [], true).unwrap();
        
        let result = plugin.call("greet", "World").unwrap();
        
        assert_eq!(String::from_utf8_lossy(&result), "Hello, World!");
    }
    
    #[test]
    fn test_plugin_with_config() {
        let manifest = Manifest::new([Wasm::file("tests/plugins/config.wasm")])
            .with_config(serde_json::json!({
                "greeting": "Hey"
            }));
        
        let mut plugin = Plugin::new(&manifest, [], true).unwrap();
        let result = plugin.call("greet", "World").unwrap();
        
        assert_eq!(String::from_utf8_lossy(&result), "Hey, World!");
    }
    
    #[test]
    fn test_plugin_error() {
        let manifest = Manifest::new([Wasm::file("tests/plugins/error.wasm")]);
        let mut plugin = Plugin::new(&manifest, [], true).unwrap();
        
        let result = plugin.call("fail", "");
        
        assert!(result.is_err());
    }
}
```

### 2. Integration Tests

```rust
#[cfg(test)]
mod integration {
    use crate::{PluginPool, SecurePluginConfig};
    use tokio::time::{timeout, Duration};
    
    #[tokio::test]
    async fn test_plugin_pool_concurrency() {
        let manifest = create_test_manifest();
        let pool = PluginPool::new(manifest, 5).await.unwrap();
        
        // Execute concurrent calls
        let tasks: Vec<_> = (0..10)
            .map(|i| {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let guard = pool.acquire().await.unwrap();
                    // ... execute
                    Ok::<_, extism::Error>(())
                })
            })
            .collect();
        
        let results = futures::future::join_all(tasks).await;
        
        assert!(results.iter().all(|r| r.as_ref().unwrap().is_ok()));
    }
    
    #[tokio::test]
    async fn test_plugin_timeout() {
        let config = SecurePluginConfig {
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        
        let manifest = create_test_manifest();
        let mut plugin = create_secure_plugin(
            Wasm::file("tests/plugins/slow.wasm"),
            config,
        ).unwrap();
        
        let result = timeout(
            Duration::from_secs(1),
            tokio::task::spawn_blocking(move || {
                plugin.call("slow_function", "")
            })
        ).await;
        
        assert!(result.is_err()); // Should timeout
    }
}
```

### 3. End-to-End Tests

```rust
#[cfg(test)]
mod e2e {
    use reqwest::Client;
    use crate::server::create_app;
    
    #[tokio::test]
    async fn test_full_request_cycle() {
        let app = create_app().await;
        let client = Client::new();
        
        let response = client
            .post("http://localhost:3000/api/process")
            .json(&serde_json::json!({"action": "test"}))
            .send()
            .await
            .unwrap();
        
        assert!(response.status().is_success());
    }
}
```

## Deployment

### Docker Configuration

```dockerfile
# Dockerfile
FROM rust:1.77-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Build application
COPY . .
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/null app
USER app

WORKDIR /app
COPY --from=builder /app/target/release/my-app .
COPY --from=builder /app/plugins ./plugins

ENV RUST_LOG=info

EXPOSE 8080

CMD ["./my-app"]
```

### Kubernetes Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: extism-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: extism
  template:
    metadata:
      labels:
        app: extism
    spec:
      containers:
      - name: app
        image: my-registry/extism-app:latest
        ports:
        - containerPort: 8080
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        env:
        - name: RUST_LOG
          value: "info,extism=debug"
        - name: MAX_PLUGINS
          value: "10"
        - name: PLUGIN_TIMEOUT_MS
          value: "5000"
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 30
---
apiVersion: v1
kind: Service
metadata:
  name: extism-service
spec:
  selector:
    app: extism
  ports:
  - port: 80
    targetPort: 8080
  type: LoadBalancer
```

## Scaling Considerations

### Horizontal Scaling

```rust
/// Plugin distribution across instances
pub struct DistributedPluginManager {
    /// Consistent hashing for plugin affinity
    hasher: ConsistentHasher,
    /// Local plugin cache
    local_cache: Arc<Mutex<HashMap<String, Plugin>>>,
    /// Remote plugin registry (Redis, etc.)
    registry: PluginRegistry,
}

impl DistributedPluginManager {
    /// Get or create plugin, distributed across instances
    pub async fn get_plugin(&self, plugin_id: &str) -> Result<Plugin, Error> {
        // Check local cache first
        {
            let cache = self.local_cache.lock().await;
            if let Some(plugin) = cache.get(plugin_id) {
                return Ok(plugin.clone());
            }
        }
        
        // Fetch from registry or create
        let plugin = self.registry.get_or_create(plugin_id).await?;
        
        // Cache locally
        let mut cache = self.local_cache.lock().await;
        cache.insert(plugin_id.to_string(), plugin.clone());
        
        Ok(plugin)
    }
}
```

### Plugin Hot-Reloading

```rust
use notify::{Watcher, RecursiveMode, watcher};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Hot-reload plugins when WASM files change
pub struct HotReloader {
    plugins: Arc<RwLock<HashMap<String, Plugin>>>,
    manifests: HashMap<String, Manifest>,
}

impl HotReloader {
    pub async fn start(
        plugins: Arc<RwLock<HashMap<String, Plugin>>>,
        watch_dirs: Vec<&str>,
    ) -> notify::Result<Self> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        
        // Set up file watcher
        let mut watcher = watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;
        
        for dir in watch_dirs {
            watcher.watch(Path::new(dir), RecursiveMode::Recursive)?;
        }
        
        // Spawn reload handler
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if event.kind.is_modify() {
                    // Extract plugin ID from path
                    // Reload plugin
                    // Replace in plugins map
                }
            }
        });
        
        Ok(Self {
            plugins,
            manifests: HashMap::new(),
        })
    }
    
    /// Reload a specific plugin
    pub async fn reload_plugin(&mut self, plugin_id: &str) -> Result<(), Error> {
        let manifest = self.manifests.get(plugin_id)
            .ok_or_else(|| Error::msg("Plugin not found"))?
            .clone();
        
        let new_plugin = Plugin::new(&manifest, [], true)?;
        
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id.to_string(), new_plugin);
        
        tracing::info!("Reloaded plugin: {}", plugin_id);
        
        Ok(())
    }
}
```

## Security Checklist

```
Deployment Security:
□ Run as non-root user
□ Read-only filesystem (except /tmp)
□ Network policies restrict egress
□ Resource limits (CPU, memory)
□ Pod security policies/standards

Plugin Security:
□ Minimal capabilities per plugin
□ Input validation on all plugin inputs
□ Output sanitization before rendering
□ Execution timeouts configured
□ Memory limits enforced

Operational Security:
□ Plugin signing and verification
□ Audit logging enabled
□ Metrics and alerting configured
□ Secrets management (Vault, etc.)
□ Regular security updates

Runtime Security:
□ WASM sandboxing enabled
□ WASI disabled unless required
□ Host functions validated
□ Rate limiting implemented
□ Circuit breakers for plugin calls
```

## Conclusion

Production-grade Extism deployments require attention to:

1. **Security**: Capability-based access, input validation, sandboxing
2. **Performance**: Plugin pooling, module caching, async execution
3. **Observability**: Metrics, logging, distributed tracing
4. **Testing**: Unit, integration, and E2E test coverage
5. **Deployment**: Container orchestration, scaling, hot-reloading
6. **Operations**: Monitoring, alerting, incident response

Following these patterns ensures reliable, secure, and performant plugin systems at scale.
