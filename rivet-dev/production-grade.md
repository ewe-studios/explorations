---
source: /home/darkvoid/Boxxed/@formulas/src.rivet-dev/rivetkit
repository: github.com/rivet-dev/rivetkit
explored_at: 2026-04-05
focus: Production deployment, monitoring, scaling, cluster configuration, security
---

# Production-Grade RivetKit Applications

## Overview

This document covers production deployment patterns for RivetKit applications including horizontal scaling, monitoring with Prometheus, cluster configuration, security hardening, and performance optimization.

## Architecture

```mermaid
flowchart TB
    subgraph Edge
        LB[Load Balancer]
        CDN[CDN for Static]
    end
    
    subgraph Cluster["RivetKit Cluster (N nodes)"]
        Node1[Node 1<br/>Actors + HTTP]
        Node2[Node 2<br/>Actors + HTTP]
        Node3[Node 3<br/>Actors + HTTP]
    end
    
    subgraph Services
        Redis[Redis Cluster<br/>PubSub + Caching]
        DB[(PostgreSQL<br/>Primary + Replicas)]
        Prometheus[Prometheus]
        Grafana[Grafana]
    end
    
    CDN --> LB
    LB --> Node1
    LB --> Node2
    LB --> Node3
    
    Node1 <--> Redis
    Node2 <--> Redis
    Node3 <--> Redis
    
    Node1 <--> DB
    Node2 <--> DB
    Node3 <--> DB
    
    Node1 -.metrics.-> Prometheus
    Node2 -.metrics.-> Prometheus
    Node3 -.metrics.-> Prometheus
    
    Prometheus --> Grafana
    
    subgraph K8s
        Node1
        Node2
        Node3
    end
```

## Cluster Configuration

### Environment Setup

```bash
# .env.production

# Node configuration
NODE_ID=node-1
NODE_PORT=3000
NODE_ADDRESS=0.0.0.0

# Cluster configuration
CLUSTER_ENABLED=true
DISCOVERY_NODES=redis://redis-cluster:6379

# Database
DATABASE_URL=postgresql://user:pass@postgres:5432/rivetkit
DATABASE_POOL_SIZE=20
DATABASE_READ_REPLICAS=postgresql://replica1:5432/rivetkit,postgresql://replica2:5432/rivetkit

# Redis
REDIS_URL=redis://redis-cluster:6379
REDIS_KEY_PREFIX=rivetkit:

# Security
SECRET_KEY_BASE=your-secret-key-base-here
JWT_SECRET=your-jwt-secret-here

# Monitoring
PROMETHEUS_PORT=9090
LOG_LEVEL=info
```

### Docker Configuration

```dockerfile
# Dockerfile

FROM rust:1.75 AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source for deps fetch
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo fetch --locked
RUN rm -rf src

# Copy source
COPY . .

# Build release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/rivetkit-server ./

ENV RUST_LOG=info
ENV NODE_PORT=3000

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

ENTRYPOINT ["./rivetkit-server"]
```

```yaml
# docker-compose.yml

version: '3.8'

services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - NODE_ID=app-1
      - DATABASE_URL=postgresql://postgres:postgres@db:5432/rivetkit
      - REDIS_URL=redis://redis:6379
      - SECRET_KEY_BASE=${SECRET_KEY_BASE}
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_healthy
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '0.5'
          memory: 512M
    networks:
      - app-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  db:
    image: postgres:15
    volumes:
      - postgres-data:/var/lib/postgresql/data
    environment:
      - POSTGRES_DB=rivetkit
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=postgres
    networks:
      - app-network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
    networks:
      - app-network
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    ports:
      - "9090:9090"
    networks:
      - app-network

  grafana:
    image: grafana/grafana:latest
    volumes:
      - grafana-data:/var/lib/grafana
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    networks:
      - app-network

volumes:
  postgres-data:
  redis-data:
  prometheus-data:
  grafana-data:

networks:
  app-network:
    driver: bridge
```

### Kubernetes Deployment

```yaml
# k8s/deployment.yaml

apiVersion: apps/v1
kind: Deployment
metadata:
  name: rivetkit
  labels:
    app: rivetkit
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rivetkit
  template:
    metadata:
      labels:
        app: rivetkit
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "3000"
        prometheus.io/path: "/metrics"
    spec:
      serviceAccountName: rivetkit
      containers:
        - name: rivetkit
          image: rivetkit:latest
          imagePullPolicy: Always
          ports:
            - name: http
              containerPort: 3000
              protocol: TCP
          env:
            - name: NODE_ID
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: rivetkit-secrets
                  key: database-url
            - name: REDIS_URL
              valueFrom:
                configMapKeyRef:
                  name: rivetkit-config
                  key: redis-url
            - name: SECRET_KEY_BASE
              valueFrom:
                secretKeyRef:
                  name: rivetkit-secrets
                  key: secret-key-base
            - name: RUST_LOG
              value: "info"
          readinessProbe:
            httpGet:
              path: /health/readiness
              port: 3000
            initialDelaySeconds: 5
            periodSeconds: 10
            timeoutSeconds: 5
            failureThreshold: 3
          livenessProbe:
            httpGet:
              path: /health/liveness
              port: 3000
            initialDelaySeconds: 30
            periodSeconds: 20
            timeoutSeconds: 5
            failureThreshold: 3
          resources:
            requests:
              memory: "512Mi"
              cpu: "250m"
            limits:
              memory: "2Gi"
              cpu: "2000m"
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchLabels:
                    app: rivetkit
                topologyKey: kubernetes.io/hostname
---
apiVersion: v1
kind: Service
metadata:
  name: rivetkit
  labels:
    app: rivetkit
spec:
  type: ClusterIP
  ports:
    - port: 3000
      targetPort: http
      protocol: TCP
      name: http
  selector:
    app: rivetkit
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: rivetkit-config
data:
  redis-url: "redis://redis-headless:6379"
---
apiVersion: v1
kind: Secret
metadata:
  name: rivetkit-secrets
type: Opaque
stringData:
  database-url: "postgresql://user:pass@postgres:5432/rivetkit"
  secret-key-base: "change-me-to-a-secure-random-string"
```

## Monitoring

### Prometheus Metrics

```rust
// src/metrics.rs

use prometheus::{Registry, Counter, Gauge, Histogram, register_counter_with_registry, register_gauge_with_registry, register_histogram_with_registry};

pub struct Metrics {
    pub registry: Registry,
    
    // Actor metrics
    pub actor_count: Gauge,
    pub actor_created_total: Counter,
    pub actor_terminated_total: Counter,
    
    // Action metrics
    pub action_calls_total: Counter,
    pub action_duration: Histogram,
    pub action_errors_total: Counter,
    
    // Storage metrics
    pub storage_load_duration: Histogram,
    pub storage_save_duration: Histogram,
    pub storage_errors_total: Counter,
    
    // WebSocket metrics
    pub ws_connections: Gauge,
    pub ws_messages_total: Counter,
    pub ws_broadcast_total: Counter,
    
    // Memory metrics
    pub actor_memory_bytes: Gauge,
}

impl Metrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        let actor_count = register_gauge_with_registry!(
            "rivetkit_actor_count",
            "Current number of active actors",
            registry
        )?;
        
        let actor_created_total = register_counter_with_registry!(
            "rivetkit_actor_created_total",
            "Total number of actors created",
            registry
        )?;
        
        let actor_terminated_total = register_counter_with_registry!(
            "rivetkit_actor_terminated_total",
            "Total number of actors terminated",
            registry
        )?;
        
        let action_calls_total = register_counter_with_registry!(
            "rivetkit_action_calls_total",
            "Total number of action calls",
            registry
        )?;
        
        let action_duration = register_histogram_with_registry!(
            "rivetkit_action_duration_seconds",
            "Action execution duration in seconds",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0],
            registry
        )?;
        
        let storage_load_duration = register_histogram_with_registry!(
            "rivetkit_storage_load_duration_seconds",
            "Storage load duration in seconds",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5],
            registry
        )?;
        
        let storage_save_duration = register_histogram_with_registry!(
            "rivetkit_storage_save_duration_seconds",
            "Storage save duration in seconds",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5],
            registry
        )?;
        
        let ws_connections = register_gauge_with_registry!(
            "rivetkit_websocket_connections",
            "Current WebSocket connections",
            registry
        )?;
        
        Ok(Self {
            registry,
            actor_count,
            actor_created_total,
            actor_terminated_total,
            action_calls_total,
            action_duration,
            action_errors_total: register_counter_with_registry!(
                "rivetkit_action_errors_total",
                "Total action errors",
                registry
            )?,
            storage_load_duration,
            storage_save_duration,
            storage_errors_total: register_counter_with_registry!(
                "rivetkit_storage_errors_total",
                "Total storage errors",
                registry
            )?,
            ws_connections,
            ws_messages_total: register_counter_with_registry!(
                "rivetkit_ws_messages_total",
                "Total WebSocket messages",
                registry
            )?,
            ws_broadcast_total: register_counter_with_registry!(
                "rivetkit_ws_broadcast_total",
                "Total WebSocket broadcasts",
                registry
            )?,
            actor_memory_bytes: register_gauge_with_registry!(
                "rivetkit_actor_memory_bytes",
                "Memory used by actors in bytes",
                registry
            )?,
        })
    }
    
    /// Expose metrics endpoint
    pub fn metrics_handler(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}
```

### Tracing Setup

```rust
// src/tracing.rs

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing() {
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,rivetkit=debug"));
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "RivetKit Dashboard",
    "panels": [
      {
        "title": "Active Actors",
        "type": "graph",
        "targets": [
          {
            "expr": "rivetkit_actor_count",
            "legendFormat": "Active Actors"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 0}
      },
      {
        "title": "Action Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(rivetkit_action_calls_total[1m])",
            "legendFormat": "Actions/s"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 0}
      },
      {
        "title": "Action Duration (p95)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, sum(rate(rivetkit_action_duration_seconds_bucket[5m])) by (le))",
            "legendFormat": "p95"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 0, "y": 8}
      },
      {
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(rivetkit_action_errors_total[5m])",
            "legendFormat": "Errors/s"
          }
        ],
        "gridPos": {"h": 8, "w": 12, "x": 12, "y": 8}
      },
      {
        "title": "WebSocket Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "rivetkit_websocket_connections",
            "legendFormat": "Connections"
          }
        ],
        "gridPos": {"h": 8, "w": 24, "x": 0, "y": 16}
      },
      {
        "title": "Storage Operations",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(rivetkit_storage_load_duration_seconds_sum[5m]) / rate(rivetkit_storage_load_duration_seconds_count[5m])",
            "legendFormat": "Load (avg)"
          },
          {
            "expr": "rate(rivetkit_storage_save_duration_seconds_sum[5m]) / rate(rivetkit_storage_save_duration_seconds_count[5m])",
            "legendFormat": "Save (avg)"
          }
        ],
        "gridPos": {"h": 8, "w": 24, "x": 0, "y": 24}
      }
    ]
  }
}
```

## Performance Optimization

### Connection Pooling

```rust
// src/pool.rs

use sqlx::postgres::{PgPool, PgPoolOptions};
use redis::{Client, aio::MultiplexedConnection};
use std::time::Duration;

pub fn create_db_pool(database_url: &str, pool_size: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(pool_size)
        .min_connections(pool_size / 4)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect_lazy(database_url)
}

pub fn create_redis_client(redis_url: &str) -> Result<Client, redis::RedisError> {
    Client::open(redis_url)
}

// Usage
let pool = create_db_pool(&config.database_url, 20)?;
let redis = create_redis_client(&config.redis_url)?;
```

### Actor Eviction

```rust
// src/eviction.rs

use crate::registry::Registry;
use std::collections::HashMap;
use tokio::time::{interval, Duration};

pub struct EvictionPolicy {
    pub max_actors: usize,
    pub idle_timeout: Duration,
    pub check_interval: Duration,
}

pub struct EvictionManager {
    registry: Registry,
    policy: EvictionPolicy,
    access_times: HashMap<String, u64>,
}

impl EvictionManager {
    pub fn new(registry: Registry, policy: EvictionPolicy) -> Self {
        Self {
            registry,
            policy,
            access_times: HashMap::new(),
        }
    }
    
    pub fn start(&self) {
        let mut interval = interval(self.policy.check_interval);
        
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                self.evict_idle_actors().await;
            }
        });
    }
    
    async fn evict_idle_actors(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let current_count = self.registry.actor_count();
        
        if current_count > self.policy.max_actors {
            // Find idle actors to evict
            let mut idle: Vec<_> = self.access_times
                .iter()
                .filter(|(_, last_access)| now - *last_access > self.policy.idle_timeout.as_millis() as u64)
                .map(|(id, _)| id.clone())
                .collect();
            
            // Evict oldest first
            idle.sort();
            
            for actor_id in idle.iter().take(current_count - self.policy.max_actors) {
                self.registry.evict(actor_id).await;
                self.access_times.remove(actor_id);
            }
        }
    }
    
    pub fn record_access(&mut self, actor_id: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.access_times.insert(actor_id.to_string(), now);
    }
}
```

## Security Hardening

### Authentication Middleware

```rust
// src/auth.rs

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub async fn auth_middleware(
    State(secret): State<String>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    
    match auth_header {
        Some(token) => {
            let decoding_key = DecodingKey::from_secret(secret.as_bytes());
            
            match decode::<Claims>(token, &decoding_key, &Validation::default()) {
                Ok(token_data) => {
                    req.extensions_mut().insert(token_data.claims);
                    Ok(next.run(req).await)
                }
                Err(_) => Err(StatusCode::UNAUTHORIZED),
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
```

### Rate Limiting

```rust
// src/rate_limit.rs

use axum::{
    http::StatusCode,
    response::Response,
    middleware::Next,
    extract::Request,
};
use governor::{Quota, RateLimiter};
use std::{num::NonZeroU32, sync::Arc};
use dashmap::DashMap;

pub struct RateLimitState {
    limiters: DashMap<String, RateLimiter>,
    quota: Quota,
}

impl RateLimitState {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            limiters: DashMap::new(),
            quota: Quota::per_second(NonZeroU32::new(requests_per_second).unwrap()),
        }
    }
    
    fn get_limiter(&self, key: &str) -> RateLimiter {
        self.limiters
            .entry(key.to_string())
            .or_insert_with(|| RateLimiter::direct(self.quota))
            .clone()
    }
}

pub async fn rate_limit_middleware(
    State(state): State<Arc<RateLimitState>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = req
        .extensions()
        .get::<String>()
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());
    
    let limiter = state.get_limiter(&client_ip);
    
    if limiter.check().is_ok() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}
```

## Deployment Pipeline

### CI/CD with GitHub Actions

```yaml
# .github/workflows/deploy.yml

name: Deploy

on:
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
      redis:
        image: redis:7-alpine
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 6379:6379
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup Rust
        uses: dtolnay/rust-action@stable
      
      - name: Cache cargo
        uses: Swatinem/rust-cache@v2
      
      - name: Run tests
        run: cargo test
        env:
          DATABASE_URL: postgresql://postgres:postgres@localhost:5432/postgres
          REDIS_URL: redis://localhost:6379
      
      - name: Run clippy
        run: cargo clippy -- -D warnings
      
      - name: Run fmt check
        run: cargo fmt --check

  build:
    needs: test
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v2
      
      - name: Login to Container Registry
        uses: docker/login-action@v2
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      
      - name: Build and push
        uses: docker/build-push-action@v4
        with:
          context: .
          push: true
          tags: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:latest,${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.sha }}
          cache-from: type=registry,ref=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:buildcache
          cache-to: type=registry,ref=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:buildcache,mode=max

  deploy:
    needs: build
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Set up kubectl
        uses: azure/setup-kubectl@v3
      
      - name: Configure kubectl
        run: |
          echo "${{ secrets.KUBECONFIG }}" | base64 -d > kubeconfig
          export KUBECONFIG=kubeconfig
      
      - name: Deploy to Kubernetes
        run: |
          kubectl apply -f k8s/
          kubectl rollout restart deployment/rivetkit
          kubectl rollout status deployment/rivetkit
      
      - name: Run smoke tests
        run: |
          curl -f https://api.example.com/health/readiness
```

## Conclusion

Production-grade RivetKit applications require:

1. **Cluster Configuration**: Proper node discovery, distributed state
2. **Monitoring**: Prometheus metrics, Grafana dashboards, tracing
3. **Performance**: Connection pooling, actor eviction, caching
4. **Security**: Authentication middleware, rate limiting, input validation
5. **Scaling**: Horizontal scaling with Kubernetes, load balancing
6. **CI/CD**: Automated testing, container builds, deployment pipelines
