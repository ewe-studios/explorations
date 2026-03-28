---
title: "Production-Grade Pavex: Deployment, Scaling, and CI/CD"
subtitle: "Best practices for running Pavex in production environments"
based_on: "pavex CI/CD configuration and deployment patterns"
level: "Advanced - Production deployment focus"
---

# Production-Grade Pavex

## Overview

This document covers production considerations for using Pavex: CI/CD integration, performance optimizations, scaling strategies, monitoring, and deployment best practices.

---

## 1. CI/CD Integration

### 1.1 GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_TOOLCHAIN: nightly-2025-03-26

jobs:
  # Generate SDK and verify it compiles
  generate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          components: rust-docs-json

      - name: Install Pavex CLI
        run: |
          curl -L https://install.pavex.dev | sh
          pavex self activate "${{ secrets.PAVEX_ACTIVATION_KEY }}"

      - name: Generate SDK
        run: pavex generate blueprint.ron --output ./server_sdk

      - name: Verify generated code
        run: |
          cd server_sdk
          cargo check --locked

  # Run tests
  test:
    needs: generate
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
        with:
          cache-on-failure: true

      - name: Run tests
        run: cargo test --workspace --locked

  # Lint and format
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-action@stable
        with:
          toolchain: ${{ env.RUST_TOOLCHAIN }}
          components: rustfmt, clippy

      - name: Check formatting
        run: cargo fmt --check

      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings
```

### 1.2 ci_utils for Generated CI

Pavex includes a `ci_utils` crate that generates CI configurations:

```rust
// ci_utils/src/main.rs
use pavex_ci_utils::generate_github_actions;

fn main() {
    let config = CiConfig {
        rust_toolchain: "nightly-2025-03-26",
        pavex_version: "0.1.80",
        test_command: "cargo test --workspace",
        lint_command: "cargo clippy -- -D warnings",
    };

    generate_github_actions(&config).unwrap();
}
```

### 1.3 Docker Build

```dockerfile
# Dockerfile
FROM rust:1.78-slim as builder

# Install Pavex CLI
RUN curl -L https://install.pavex.dev | sh

WORKDIR /app

# Copy workspace
COPY Cargo.toml Cargo.lock ./
COPY app/ app/
COPY blueprint.ron ./

# Generate SDK
RUN pavex generate blueprint.ron --output ./server_sdk

# Build server
RUN cargo build --release -p server

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/server /usr/local/bin/

EXPOSE 8080
CMD ["server"]
```

### 1.4 Multi-Stage Build with Cache

```dockerfile
# Optimized Dockerfile with caching
FROM rust:1.78-slim as base
RUN apt-get update && apt-get install -y pkg-config libssl-dev

FROM base as builder
WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# Install Pavex
RUN curl -L https://install.pavex.dev | sh

# Copy source
COPY . .

# Generate SDK (uses Pavex cache)
RUN --mount=type=cache,target=/root/.pavex \
    pavex generate blueprint.ron --output ./server_sdk

# Build
RUN cargo build --release -p server

FROM debian:bookworm-slim as runtime
COPY --from=builder /app/target/release/server /usr/local/bin/
EXPOSE 8080
CMD ["server"]
```

---

## 2. Performance Optimizations

### 2.1 Build Time Optimization

**Problem:** rustdoc JSON generation is slow.

**Solutions:**

```toml
# .cargo/config.toml
[build]
# Use jemalloc for faster allocations
jemalloc = true

# Increase parallelism
jobs = 8

[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

**Pavex-specific optimizations:**

```rust
// Cache warming script
fn warm_pavex_cache() {
    let common_crates = vec![
        "serde", "tokio", "hyper", "bytes", "tracing",
    ];

    for crate_name in common_crates {
        // Trigger cache population for common dependencies
        generate_rustdoc_for_crate(crate_name);
    }
}
```

### 2.2 Memory Optimization

```rust
// Use streaming for large rustdoc JSON
use std::io::BufReader;

fn load_large_rustdoc(path: &Path) -> Result<Crate, Error> {
    let file = fs_err::File::open(path)?;
    let reader = BufReader::new(file);
    let mut deserializer = serde_json::Deserializer::from_reader(reader);

    // Disable recursion limit for deeply nested types
    deserializer.disable_recursion_limit();

    // Use serde_stacker to prevent stack overflow
    let deserializer = serde_stacker::Deserializer::new(&mut deserializer);

    Crate::deserialize(deserializer)
}
```

### 2.3 Incremental Build Optimization

```toml
# Cargo.toml
[profile.dev]
# Faster compilation for development
opt-level = 0
debug = "line-tables-only"
split-debuginfo = "unpacked"

[profile.release]
# Full optimizations for production
lto = "thin"
codegen-units = 1
```

---

## 3. Scaling Strategies

### 3.1 Monorepo Support

```
monorepo/
├── apps/
│   ├── api/
│   │   ├── blueprint.ron
│   │   └── src/
│   └── worker/
│       ├── blueprint.ron
│       └── src/
├── libs/
│   ├── shared_types/
│   └── common_utils/
└── Cargo.toml (workspace)
```

**Workspace configuration:**

```toml
# Cargo.toml
[workspace]
members = ["apps/*", "libs/*"]
resolver = "3"

[workspace.dependencies]
pavex = "0.1"
shared_types = { path = "libs/shared_types" }
```

### 3.2 Shared Blueprint Pattern

```rust
// libs/common_blueprints/src/lib.rs
use pavex::blueprint::Blueprint;

/// Common middleware applied to all APIs
pub fn common_middleware() -> Blueprint {
    let mut bp = Blueprint::new();

    bp.middleware(f!(crate::logging::log_requests));
    bp.middleware(f!(crate::tracing::trace_requests));
    bp.error_handler(f!(crate::errors::handle_common_errors));

    bp
}

// apps/api/src/blueprint.rs
use common_blueprints::common_middleware;

pub fn blueprint() -> Blueprint {
    let mut bp = common_middleware();

    // Add API-specific routes
    bp.route(GET, "/users", f!(crate::handlers::list_users));

    bp
}
```

### 3.3 Multi-Region Deployment

```
┌─────────────────────────────────────────────────────────┐
│              Multi-Region Architecture                   │
│                                                          │
│  Region A (us-east-1)    Region B (eu-west-1)           │
│  ┌──────────────┐        ┌──────────────┐              │
│  │  API Server  │        │  API Server  │              │
│  │  (Pavex)     │        │  (Pavex)     │              │
│  └──────┬───────┘        └──────┬───────┘              │
│         │                       │                       │
│         └───────────┬───────────┘                       │
│                     │                                   │
│              ┌──────▼───────┐                          │
│              │  Global DB   │                          │
│              │  (Cockroach) │                          │
│              └──────────────┘                          │
└─────────────────────────────────────────────────────────┘
```

---

## 4. Monitoring and Observability

### 4.1 Tracing Integration

```rust
// app/src/telemetry.rs
use tracing_subscriber::{layer::SubscriberExt, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry::sdk::{trace, Resource};

pub fn init_telemetry(service_name: &str) -> tracing::subscriber::DefaultGuard {
    let tracer = trace::TracerProvider::builder()
        .with_resource(Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name),
        ]))
        .with_batch_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .build();

    let telemetry_layer = OpenTelemetryLayer::new(tracer);

    let subscriber = Registry::default()
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer());

    tracing::subscriber::set_default(subscriber)
}
```

### 4.2 Metrics Collection

```rust
// app/src/middleware/metrics.rs
use metrics::{counter, histogram};
use std::time::Instant;

pub fn metrics_middleware(
    request: &RequestHead,
    handler: impl FnOnce(&RequestHead) -> Response,
) -> Response {
    let start = Instant::now();

    counter!("http_requests_total", "method" => request.method().to_string())
        .increment(1);

    let response = handler(request);

    let duration = start.elapsed();
    histogram!("http_request_duration_seconds")
        .record(duration.as_secs_f64());

    counter!("http_responses_total",
        "method" => request.method().to_string(),
        "status" => response.status().as_str(),
    ).increment(1);

    response
}
```

### 4.3 Health Checks

```rust
// app/src/handlers/health.rs
use pavex::response::Response;
use pavex::http::StatusCode;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthStatus {
    pub status: &'static str,
    pub version: &'static str,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: CheckResult,
    pub cache: CheckResult,
}

#[derive(Serialize)]
pub struct CheckResult {
    pub status: &'static str,
    pub latency_ms: u64,
}

pub async fn health_check(
    db_pool: &DatabasePool,
    cache: &CacheClient,
) -> Response {
    let start = std::time::Instant::now();
    let db_status = db_pool.health_check().await;
    let db_latency = start.elapsed().as_millis() as u64;

    let start = std::time::Instant::now();
    let cache_status = cache.ping().await;
    let cache_latency = start.elapsed().as_millis() as u64;

    let status = if db_status.is_ok() && cache_status.is_ok() {
        "healthy"
    } else {
        "unhealthy"
    };

    let health = HealthStatus {
        status,
        version: env!("CARGO_PKG_VERSION"),
        checks: HealthChecks {
            database: CheckResult {
                status: if db_status.is_ok() { "pass" } else { "fail" },
                latency_ms: db_latency,
            },
            cache: CheckResult {
                status: if cache_status.is_ok() { "pass" } else { "fail" },
                latency_ms: cache_latency,
            },
        },
    };

    Response::builder()
        .status(if status == "healthy" {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        })
        .json(&health)
        .build()
}
```

---

## 5. Security Considerations

### 5.1 Input Validation

```rust
// app/src/middleware/validation.rs
use pavex::request::RequestHead;
use pavex::response::Response;
use pavex::http::StatusCode;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10MB

pub fn validate_request_size(request: &RequestHead) -> Result<(), Response> {
    if let Some(content_length) = request
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
    {
        if content_length > MAX_BODY_SIZE {
            return Err(Response::builder()
                .status(StatusCode::PAYLOAD_TOO_LARGE)
                .body("Request body too large".into())
                .build());
        }
    }
    Ok(())
}
```

### 5.2 Rate Limiting

```rust
// app/src/middleware/rate_limit.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    limit: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(limit: usize, window: Duration) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            limit,
            window,
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window;

        let mut requests = self.requests.lock().unwrap();
        let entry = requests.entry(key.to_string()).or_default();

        // Remove old requests
        entry.retain(|&t| t > cutoff);

        if entry.len() >= self.limit {
            return false;
        }

        entry.push(now);
        true
    }
}
```

### 5.3 CORS Configuration

```rust
// app/src/middleware/cors.rs
use pavex::response::Response;

pub fn add_cors_headers(response: Response) -> Response {
    let mut builder = Response::builder();

    builder = builder
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
        .header("Access-Control-Max-Age", "86400");

    builder.body(response.into_body()).build()
}
```

---

## 6. Deployment Patterns

### 6.1 Kubernetes Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: api-server
  template:
    metadata:
      labels:
        app: api-server
    spec:
      containers:
      - name: api-server
        image: myregistry/api-server:latest
        ports:
        - containerPort: 8080
        env:
        - name: SERVER_HOST
          value: "0.0.0.0"
        - name: SERVER_PORT
          value: "8080"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: db-secret
              key: url
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
```

### 6.2 ECS Deployment

```yaml
# ecs-task-definition.json
{
  "family": "api-server",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "256",
  "memory": "512",
  "containerDefinitions": [
    {
      "name": "api-server",
      "image": "myregistry/api-server:latest",
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        { "name": "SERVER_HOST", "value": "0.0.0.0" },
        { "name": "SERVER_PORT", "value": "8080" }
      ],
      "secrets": [
        {
          "name": "DATABASE_URL",
          "valueFrom": "arn:aws:secretsmanager:us-east-1:123456789:secret:db-secret"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/api-server",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

---

## 7. Testing in Production

### 7.1 Canary Deployments

```yaml
# k8s/canary-deployment.yaml
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: api-server-rollout
spec:
  replicas: 10
  strategy:
    canary:
      steps:
      - setWeight: 10  # 10% traffic to canary
      - pause: { duration: 5m }
      - setWeight: 25
      - pause: { duration: 5m }
      - setWeight: 50
      - pause: { duration: 5m }
      - setWeight: 100
```

### 7.2 Feature Flags

```rust
// app/src/feature_flags.rs
use std::sync::Arc;
use dashmap::DashMap;

pub struct FeatureFlags {
    flags: Arc<DashMap<String, bool>>,
}

impl FeatureFlags {
    pub fn is_enabled(&self, flag: &str) -> bool {
        self.flags.get(flag).map(|r| *r).unwrap_or(false)
    }

    pub fn enable(&self, flag: &str) {
        self.flags.insert(flag.to_string(), true);
    }
}

// Usage in handler
pub fn new_feature_handler(
    flags: Arc<FeatureFlags>,
) -> Response {
    if flags.is_enabled("new_feature") {
        new_feature_logic()
    } else {
        fallback_logic()
    }
}
```

---

## Key Takeaways

1. **CI/CD integration** - Generate SDK, verify compilation, run tests
2. **Docker optimization** - Multi-stage builds, cache mounting
3. **Build performance** - jemalloc, parallel compilation, LTO
4. **Monitoring** - OpenTelemetry tracing, metrics collection
5. **Health checks** - Database and dependency health verification
6. **Security** - Input validation, rate limiting, CORS
7. **Kubernetes** - Deployments, readiness probes, resource limits
8. **Canary deployments** - Gradual traffic shifting
9. **Feature flags** - Runtime feature toggling

---

## Related Files

- **CI configuration**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/.github/workflows/`
- **ci_utils**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/ci_utils/`
- **Example projects**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.BuildTooling/pavex/examples/`

---

*Next: [05-valtron-integration.md](05-valtron-integration.md)*
