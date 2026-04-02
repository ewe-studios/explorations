---
source: /home/darkvoid/Boxxed/@dev/repo-expolorations/llamacpp/TabbyML
repository: https://github.com/TabbyML/tabby
revised_at: 2026-04-02
---

# Production-Grade TabbyML: Building for Enterprise

## Overview

This guide covers building a production-ready TabbyML deployment, including:
- High availability and scaling
- Security and authentication
- Monitoring and observability
- Multi-tenant architecture
- Cost optimization
- Disaster recovery

## Table of Contents

1. [Deployment Architecture](#1-deployment-architecture)
2. [Security Hardening](#2-security-hardening)
3. [Scaling Strategies](#3-scaling-strategies)
4. [Monitoring and Observability](#4-monitoring-and-observability)
5. [Multi-Tenancy](#5-multi-tenancy)
6. [Cost Optimization](#6-cost-optimization)
7. [Disaster Recovery](#7-disaster-recovery)
8. [Operational Runbook](#8-operational-runbook)

---

## 1. Deployment Architecture

### Production Reference Architecture

```
                                    ┌─────────────────┐
                                    │   Load Balancer │
                                    │   (nginx/ALB)   │
                                    └────────┬────────┘
                                             │
                    ┌────────────────────────┼────────────────────────┐
                    │                        │                        │
            ┌───────▼────────┐      ┌───────▼────────┐      ┌───────▼────────┐
            │  Tabby Node 1  │      │  Tabby Node 2  │      │  Tabby Node 3  │
            │                │      │                │      │                │
            │ ┌────────────┐ │      │ ┌────────────┐ │      │ ┌────────────┐ │
            │ │  Inference │ │      │ │  Inference │ │      │ │  Inference │ │
            │ │  (llama)   │ │      │ │  (llama)   │ │      │ │  (llama)   │ │
            │ └────────────┘ │      │ └────────────┘ │      │ └────────────┘ │
            │ ┌────────────┐ │      │ ┌────────────┐ │      │ ┌────────────┐ │
            │ │   Search   │ │      │ │   Search   │ │      │ │   Search   │ │
            │ │  (Tantivy) │ │      │ │  (Tantivy) │ │      │ │  (Tantivy) │ │
            │ └────────────┘ │      │ └────────────┘ │      │ └────────────┘ │
            └───────┬────────┘      └───────┬────────┘      └───────┬────────┘
                    │                        │                        │
                    └────────────────────────┼────────────────────────┘
                                             │
                    ┌────────────────────────┼────────────────────────┐
                    │                        │                        │
            ┌───────▼────────┐      ┌───────▼────────┐      ┌───────▼────────┐
            │   Redis Cache  │      │  PostgreSQL    │      │  Object Store  │
            │   (Sessions)   │      │  (Auth/Logs)   │      │   (Models)     │
            └────────────────┘      └────────────────┘      └────────────────┘
```

### Kubernetes Deployment

```yaml
# k8s/tabby-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: tabby
  namespace: ai-services
spec:
  replicas: 3
  selector:
    matchLabels:
      app: tabby
  template:
    metadata:
      labels:
        app: tabby
    spec:
      nodeSelector:
        nvidia.com/gpu: "true"
      containers:
      - name: tabby
        image: tabbyml/tabby:latest
        ports:
        - containerPort: 8080
        resources:
          requests:
            nvidia.com/gpu: 1
            memory: 16Gi
            cpu: "4"
          limits:
            nvidia.com/gpu: 1
            memory: 32Gi
            cpu: "8"
        env:
        - name: RUST_LOG
          value: "info"
        - name: TABBY_MODEL_CACHE
          value: "/models/cache"
        volumeMounts:
        - name: models
          mountPath: /models
        - name: config
          mountPath: /etc/tabby
        livenessProbe:
          httpGet:
            path: /v1/health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /v1/health
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: models
        persistentVolumeClaim:
          claimName: tabby-models-pvc
      - name: config
        configMap:
          name: tabby-config
---
apiVersion: v1
kind: Service
metadata:
  name: tabby-service
  namespace: ai-services
spec:
  selector:
    app: tabby
  ports:
  - port: 80
    targetPort: 8080
  type: ClusterIP
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: tabby-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: tabby
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 10
        periodSeconds: 60
```

### Docker Compose for Small Teams

```yaml
# docker-compose.yml
version: '3.8'

services:
  tabby:
    image: tabbyml/tabby:latest
    ports:
      - "8080:8080"
    volumes:
      - ./models:/data/models
      - ./config:/data/config
      - ./index:/data/index
    environment:
      - RUST_LOG=info
    deploy:
      resources:
        reservations:
          devices:
            - driver: nvidia
              count: 1
              capabilities: [gpu]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/v1/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes

  postgres:
    image: postgres:15-alpine
    ports:
      - "5432:5432"
    environment:
      - POSTGRES_USER=tabby
      - POSTGRES_PASSWORD=${DB_PASSWORD}
      - POSTGRES_DB=tabby
    volumes:
      - postgres-data:/var/lib/postgresql/data

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
    depends_on:
      - tabby

volumes:
  redis-data:
  postgres-data:
```

---

## 2. Security Hardening

### Authentication Configuration

```toml
# /etc/tabby/config.toml

[server.auth]
# JWT configuration
jwt_secret = "${JWT_SECRET}"  # Use environment variable
token_expiry_hours = 24
refresh_token_expiry_days = 30

# Session configuration
session_cookie_secure = true
session_cookie_http_only = true
session_cookie_same_site = "strict"

# Rate limiting
[server.rate_limit]
completions_per_minute = 100
chat_messages_per_minute = 20
search_queries_per_minute = 60

# OAuth providers
[server.oauth.github]
enabled = true
client_id = "${GITHUB_CLIENT_ID}"
client_secret = "${GITHUB_CLIENT_SECRET}"

[server.oauth.google]
enabled = true
client_id = "${GOOGLE_CLIENT_ID}"
client_secret = "${GOOGLE_CLIENT_SECRET}"

# LDAP integration
[server.ldap]
enabled = true
host = "ldap.example.com"
port = 636
use_ssl = true
base_dn = "dc=example,dc=com"
bind_dn = "cn=admin,dc=example,dc=com"
bind_password = "${LDAP_BIND_PASSWORD}"
user_filter = "(uid={})"
```

### Network Security

```nginx
# nginx.conf
server {
    listen 80;
    server_name tabby.example.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name tabby.example.com;

    # SSL configuration
    ssl_certificate /etc/nginx/ssl/fullchain.pem;
    ssl_certificate_key /etc/nginx/ssl/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256;
    ssl_prefer_server_ciphers on;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
    add_header Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline';" always;

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=tabby_limit:10m rate=10r/s;
    limit_req zone=tabby_limit burst=20 nodelay;

    location / {
        proxy_pass http://tabby-service:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # Health endpoint (no auth required)
    location /v1/health {
        proxy_pass http://tabby-service:8080;
        access_log off;
    }

    # Block sensitive paths
    location ~ /\. {
        deny all;
    }
}
```

### API Security

```rust
// Security middleware for Axum

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Validation, Algorithm};

#[derive(Debug, Clone)]
pub struct AppState {
    pub jwt_secret: SecretKey,
    pub db: DatabasePool,
}

pub async fn auth_middleware<B>(
    State(state): State<AppState>,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Parse Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate JWT
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    let token_data = decode::<Claims>(token, &state.jwt_secret, &validation)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Inject user into request extensions
    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}

#[derive(Debug, serde::Deserialize)]
pub struct Claims {
    pub sub: String,  // User ID
    pub email: String,
    pub team_id: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

// Rate limiting middleware
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;

pub struct RateLimitState {
    pub completions: RateLimiter<dashmap::DashMap<String, ()>>,
    pub chat: RateLimiter<dashmap::DashMap<String, ()>>,
}

pub async fn rate_limit_middleware(
    State(state): State<Arc<RateLimitState>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let user_id = request
        .extensions()
        .get::<Claims>()
        .map(|c| c.sub.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    // Check rate limit
    if state.completions.check_key(&user_id).is_err() {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}
```

---

## 3. Scaling Strategies

### Horizontal Scaling

```yaml
# k8s/hpa-custom-metrics.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: tabby-hpa-custom
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: tabby
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Pods
    pods:
      metric:
        name: tabby_completion_latency_p95
      target:
        type: AverageValue
        averageValue: "500ms"
  - type: Pods
    pods:
      metric:
        name: tabby_queue_depth
      target:
        type: AverageValue
        averageValue: "10"
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 60
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 10
        periodSeconds: 60
```

### Model Sharding

```rust
// Multi-model routing for different languages

use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct ModelRouter {
    models: RwLock<HashMap<String, Arc<dyn CompletionEngine>>>,
    default_model: String,
}

impl ModelRouter {
    pub fn new() -> Self {
        let mut models = HashMap::new();

        // Load different models for different use cases
        models.insert(
            "completion".to_string(),
            Arc::new(LlamaEngine::new("StarCoder-3B.gguf").unwrap()),
        );
        models.insert(
            "chat".to_string(),
            Arc::new(LlamaEngine::new("Qwen-7B-Instruct.gguf").unwrap()),
        );
        models.insert(
            "embedding".to_string(),
            Arc::new(LlamaEngine::new("all-MiniLM-L6.gguf").unwrap()),
        );

        Self {
            models: RwLock::new(models),
            default_model: "completion".to_string(),
        }
    }

    pub async fn route(&self, language: &str) -> Arc<dyn CompletionEngine> {
        let models = self.models.read().await;

        // Route based on language/model requirements
        match language {
            "rust" | "cpp" | "java" => {
                models.get("completion").cloned().unwrap_or_else(|| {
                    models.get(&self.default_model).cloned().unwrap()
                })
            }
            "chat" => models.get("chat").cloned().unwrap_or_else(|| {
                models.get(&self.default_model).cloned().unwrap()
            }),
            _ => models.get(&self.default_model).cloned().unwrap(),
        }
    }
}
```

### Caching Layer

```rust
// Redis-backed distributed cache

use redis::{Client, AsyncCommands};
use serde::{Serialize, Deserialize};

pub struct CompletionCache {
    client: Client,
    ttl_seconds: u64,
}

#[derive(Serialize, Deserialize)]
pub struct CachedCompletion {
    text: String,
    created_at: u64,
    access_count: u32,
}

impl CompletionCache {
    pub fn new(redis_url: &str, ttl_hours: u64) -> Self {
        let client = Client::open(redis_url).unwrap();
        Self {
            client,
            ttl_seconds: ttl_hours * 3600,
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.client.get_async_connection().await.ok()?;

        let cached: Option<CachedCompletion> = conn.get(key).await.ok()?;
        let cached = cached?;

        // Update access count (async, don't wait)
        let _ = conn.touch::<_, ()>(key).await;

        Some(cached.text)
    }

    pub async fn set(&self, key: &str, text: &str) {
        let mut conn = match self.client.get_async_connection().await {
            Ok(c) => c,
            Err(_) => return,
        };

        let cached = CachedCompletion {
            text: text.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            access_count: 0,
        };

        let _: () = conn
            .set_ex(key, cached, self.ttl_seconds)
            .await
            .unwrap_or(());
    }

    pub fn generate_key(&self, prefix: &str, suffix: &str, language: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(prefix.as_bytes());
        hasher.update(suffix.as_bytes());
        hasher.update(language.as_bytes());
        let hash = hasher.finalize();
        format!("completion:{:x}", hash)
    }
}
```

---

## 4. Monitoring and Observability

### Prometheus Metrics

```rust
// Metrics instrumentation

use prometheus::{Registry, Counter, Histogram, Gauge, register_counter, register_histogram};

pub struct TabbyMetrics {
    pub completion_requests_total: Counter,
    pub completion_latency_seconds: Histogram,
    pub active_connections: Gauge,
    pub cache_hit_rate: Gauge,
    pub model_load_seconds: Histogram,
}

impl TabbyMetrics {
    pub fn new(registry: &Registry) -> Self {
        Self {
            completion_requests_total: register_counter!(
                "tabby_completion_requests_total",
                "Total completion requests",
                "status" => ["success", "error"],
                registry: registry,
            ).unwrap(),

            completion_latency_seconds: register_histogram!(
                "tabby_completion_latency_seconds",
                "Completion latency in seconds",
                vec![0.1, 0.25, 0.5, 1.0, 2.5, 5.0],
                registry: registry,
            ).unwrap(),

            active_connections: register_gauge!(
                "tabby_active_connections",
                "Number of active connections",
                registry: registry,
            ).unwrap(),

            cache_hit_rate: register_gauge!(
                "tabby_cache_hit_rate",
                "Cache hit rate",
                registry: registry,
            ).unwrap(),

            model_load_seconds: register_histogram!(
                "tabby_model_load_seconds",
                "Model load time in seconds",
                vec![1.0, 5.0, 10.0, 30.0, 60.0],
                registry: registry,
            ).unwrap(),
        }
    }
}

// Usage in handlers
async fn completions_handler(
    metrics: Arc<TabbyMetrics>,
    ...
) -> Result<Response, Error> {
    let start = std::time::Instant::now();

    let result = handle_completion(...).await;

    let latency = start.elapsed().as_secs_f64();
    metrics.completion_latency_seconds.observe(latency);

    match &result {
        Ok(_) => metrics.completion_requests_total
            .with_label_values(&["success"]).inc(),
        Err(_) => metrics.completion_requests_total
            .with_label_values(&["error"]).inc(),
    }

    result
}
```

### Grafana Dashboard

```json
{
  "dashboard": {
    "title": "TabbyML Production Dashboard",
    "panels": [
      {
        "title": "Completion Latency (p50, p95, p99)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.50, rate(tabby_completion_latency_seconds_bucket[5m]))",
            "legendFormat": "p50"
          },
          {
            "expr": "histogram_quantile(0.95, rate(tabby_completion_latency_seconds_bucket[5m]))",
            "legendFormat": "p95"
          },
          {
            "expr": "histogram_quantile(0.99, rate(tabby_completion_latency_seconds_bucket[5m]))",
            "legendFormat": "p99"
          }
        ]
      },
      {
        "title": "Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(tabby_completion_requests_total[5m])",
            "legendFormat": "{{status}}"
          }
        ]
      },
      {
        "title": "Cache Hit Rate",
        "type": "gauge",
        "targets": [
          {
            "expr": "tabby_cache_hit_rate"
          }
        ]
      },
      {
        "title": "GPU Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "nvidia_gpu_memory_used_bytes / nvidia_gpu_memory_total_bytes * 100",
            "legendFormat": "GPU {{gpu_id}}"
          }
        ]
      }
    ]
  }
}
```

### Distributed Tracing

```rust
// OpenTelemetry integration

use opentelemetry::{global, sdk, trace::Tracer};
use tracing_opentelemetry::OpenTelemetryLayer;

pub fn init_tracing(service_name: &str) -> Tracer {
    let tracer = sdk::trace::TracerProvider::builder()
        .with_simple_exporter(opentelemetry_otlp::new_exporter().tonic())
        .with_config(sdk::trace::Config::default().with_service_name(service_name))
        .build();

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    tracing_subscriber::registry().with(telemetry).init();

    tracer
}

// Usage in handlers
use tracing::{instrument, Span};

#[instrument(skip_all, fields(completion_id = %uuid::Uuid::new_v4()))]
async fn handle_completion(...) -> Result<Completion, Error> {
    Span::current().record("model", &model_name);
    Span::current().record("language", &language);

    // ... handler logic

    Ok(completion)
}
```

---

## 5. Multi-Tenancy

### Team Isolation

```rust
// Multi-tenant data model

use sqlx::PgPool;

pub struct TenantConfig {
    pub id: String,
    pub name: String,
    pub model_quota: u32,
    pub rate_limit: RateLimitConfig,
}

pub struct RateLimitConfig {
    pub completions_per_minute: u32,
    pub chat_per_minute: u32,
}

// Database schema for multi-tenancy

/*
CREATE TABLE teams (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE team_members (
    team_id UUID REFERENCES teams(id),
    user_id UUID REFERENCES users(id),
    role VARCHAR(50) NOT NULL,
    PRIMARY KEY (team_id, user_id)
);

CREATE TABLE usage_records (
    id UUID PRIMARY KEY,
    team_id UUID REFERENCES teams(id),
    user_id UUID REFERENCES users(id),
    action VARCHAR(50) NOT NULL,
    model VARCHAR(100) NOT NULL,
    tokens_used INTEGER NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_usage_team ON usage_records(team_id, created_at);
*/

// Usage tracking service
pub struct UsageService {
    db: PgPool,
}

impl UsageService {
    pub async fn record_usage(
        &self,
        team_id: &str,
        user_id: &str,
        model: &str,
        tokens: u32,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            INSERT INTO usage_records (team_id, user_id, action, model, tokens_used)
            VALUES ($1, $2, 'completion', $3, $4)
            "#,
        )
        .bind(team_id)
        .bind(user_id)
        .bind(model)
        .bind(tokens as i32)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn get_team_usage(
        &self,
        team_id: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> sqlx::Result<Vec<UsageStats>> {
        sqlx::query_as(
            r#"
            SELECT
                DATE_TRUNC('day', created_at) as day,
                model,
                SUM(tokens_used) as total_tokens,
                COUNT(*) as request_count
            FROM usage_records
            WHERE team_id = $1 AND created_at BETWEEN $2 AND $3
            GROUP BY day, model
            ORDER BY day DESC
            "#,
        )
        .bind(team_id)
        .bind(start)
        .bind(end)
        .fetch_all(&self.db)
        .await
    }
}
```

---

## 6. Cost Optimization

### Model Selection Strategy

```toml
# Tiered model configuration

[models.tiers]
# Fast tier for simple completions
[models.tiers.fast]
model = "StarCoder-1B"
max_tokens = 64
use_cases = ["single-line", "simple-completion"]

# Quality tier for complex completions
[models.tiers.quality]
model = "StarCoder-7B"
max_tokens = 256
use_cases = ["multi-line", "complex-completion"]

# Chat tier for conversations
[models.tiers.chat]
model = "Qwen-7B-Instruct"
max_tokens = 512
use_cases = ["chat", "answer-engine"]
```

### GPU Resource Optimization

```yaml
# k8s/gpu-sharing.yaml
# Use MIG (Multi-Instance GPU) for A100/H100
apiVersion: nvidia.com/v1
kind: GpuDevicePlugin
metadata:
  name: mig-config
spec:
  config:
    - device: 0
      instances:
        - count: 3
          profile: mig-1g.5gb
---
# Use time-slicing for consumer GPUs
apiVersion: v1
kind: ConfigMap
metadata:
  name: nvidia-time-slicing
data:
  config.yaml: |
    version: v1
    sharing:
      timeSlicing:
        resources:
          - name: nvidia.com/gpu
            replicas: 4  # Share GPU among 4 pods
```

---

## 7. Disaster Recovery

### Backup Strategy

```bash
#!/bin/bash
# backup.sh - Automated backup script

set -e

BACKUP_DIR="/backups/tabby"
DATE=$(date +%Y%m%d-%H%M%S)
RETENTION_DAYS=30

# Backup database
pg_dump -h localhost -U tabby tabby > "$BACKUP_DIR/db-$DATE.sql"

# Backup index
tar -czf "$BACKUP_DIR/index-$DATE.tar.gz" ~/.tabby/index/

# Backup models config
tar -czf "$BACKUP_DIR/config-$DATE.tar.gz" ~/.tabby/config/

# Upload to S3
aws s3 cp "$BACKUP_DIR/db-$DATE.sql" "s3://tabby-backups/db/"
aws s3 cp "$BACKUP_DIR/index-$DATE.tar.gz" "s3://tabby-backups/index/"
aws s3 cp "$BACKUP_DIR/config-$DATE.tar.gz" "s3://tabby-backups/config/"

# Clean old backups
find "$BACKUP_DIR" -name "*.sql" -mtime +$RETENTION_DAYS -delete
find "$BACKUP_DIR" -name "*.tar.gz" -mtime +$RETENTION_DAYS -delete
aws s3 ls "s3://tabby-backups/" | while read -r line; do
    file_date=$(echo $line | awk '{print $1, $2}')
    if [[ $(date -d "$file_date" +%s) -lt $(date -d "-$RETENTION_DAYS days" +%s) ]]; then
        aws s3 rm "s3://tabby-backups/$line"
    fi
done
```

### Recovery Runbook

```markdown
# Disaster Recovery Runbook

## Scenario 1: Node Failure

1. Identify failed node
   ```bash
   kubectl get pods -n ai-services
   ```

2. Check if pod restarts automatically
   ```bash
   kubectl describe pod tabby-xxx -n ai-services
   ```

3. If not, manually delete and recreate
   ```bash
   kubectl delete pod tabby-xxx -n ai-services
   ```

## Scenario 2: Database Corruption

1. Stop all Tabby pods
   ```bash
   kubectl scale deployment tabby -n ai-services --replicas=0
   ```

2. Restore database from backup
   ```bash
   psql -h localhost -U tabby tabby < backup.sql
   ```

3. Restart pods
   ```bash
   kubectl scale deployment tabby -n ai-services --replicas=3
   ```

## Scenario 3: Model Corruption

1. Clear model cache
   ```bash
   rm -rf /models/cache/*
   ```

2. Trigger model re-download
   ```bash
   tabby download --model TabbyML/StarCoder-1B --force
   ```
```

---

## 8. Operational Runbook

### Daily Operations

```markdown
## Daily Checks

1. **Health Check**
   ```bash
   curl http://tabby-service/v1/health
   ```

2. **Check Metrics Dashboard**
   - Open Grafana dashboard
   - Review latency p95 (<500ms target)
   - Check error rate (<1% target)
   - Verify cache hit rate (>50% target)

3. **Review Logs**
   ```bash
   kubectl logs -l app=tabby -n ai-services --since=24h | grep -i error
   ```

4. **Check Disk Usage**
   ```bash
   kubectl exec tabby-xxx -- df -h /data
   ```

## Weekly Operations

1. **Review Usage Reports**
   - Generate weekly usage report
   - Identify top users
   - Check for quota violations

2. **Model Performance Review**
   - Review acceptance rates
   - Identify poorly performing models
   - Consider model updates

3. **Security Review**
   - Review authentication logs
   - Check for suspicious patterns
   - Update rate limits if needed
```

### Alerting Rules

```yaml
# Prometheus alerting rules
groups:
- name: tabby-alerts
  rules:
  - alert: HighLatency
    expr: histogram_quantile(0.95, rate(tabby_completion_latency_seconds_bucket[5m])) > 1.0
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High completion latency"
      description: "P95 latency is {{ $value }}s"

  - alert: HighErrorRate
    expr: rate(tabby_completion_requests_total{status="error"}[5m]) / rate(tabby_completion_requests_total[5m]) > 0.01
    for: 5m
    labels:
      severity: critical
    annotations:
      summary: "High error rate"
      description: "Error rate is {{ $value | humanizePercentage }}"

  - alert: LowCacheHitRate
    expr: tabby_cache_hit_rate < 0.3
    for: 15m
    labels:
      severity: warning
    annotations:
      summary: "Low cache hit rate"
      description: "Cache hit rate is {{ $value | humanizePercentage }}"

  - alert: HighMemoryUsage
    expr: container_memory_usage_bytes / container_spec_memory_limit_bytes > 0.9
    for: 5m
    labels:
      severity: warning
    annotations:
      summary: "High memory usage"
      description: "Memory usage is {{ $value | humanizePercentage }}"
```

---

## Conclusion

Building a production-grade TabbyML deployment requires attention to:

1. **High Availability** - Multi-node deployments with automatic failover
2. **Security** - Authentication, rate limiting, network security
3. **Scaling** - Horizontal scaling, model sharding, caching
4. **Observability** - Metrics, logging, tracing
5. **Multi-Tenancy** - Team isolation, usage tracking
6. **Cost Control** - Tiered models, GPU optimization
7. **Disaster Recovery** - Backups, runbooks, alerting

The key insight is that **production ML systems require operational excellence** - automate everything, monitor everything, and always have a rollback plan.
