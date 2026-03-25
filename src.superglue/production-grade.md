# Superglue Production-Grade Implementation Guide

**Focus:** Building a production-ready Superglue deployment

---

## Table of Contents

1. [Architecture Patterns](#architecture-patterns)
2. [High Availability](#high-availability)
3. [Scaling Strategies](#scaling-strategies)
4. [Security Implementation](#security-implementation)
5. [Monitoring & Observability](#monitoring--observability)
6. [Performance Optimization](#performance-optimization)
7. [Disaster Recovery](#disaster-recovery)
8. [CI/CD Pipeline](#cicd-pipeline)

---

## Architecture Patterns

### Production Reference Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLIENT LAYER                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Web App     │  │  Mobile App  │  │  Backend     │  │  Partners    │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
└─────────┼─────────────────┼─────────────────┼─────────────────┼────────────┘
          │                 │                 │                 │
          └─────────────────┼─────────────────┼─────────────────┘
                            │                 │
                   ┌────────▼────────┐ ┌─────▼────────┐
                   │   CDN/Edge      │ │ API Gateway  │
                   │   (Static)      │ │ (Kong/AWS)   │
                   └─────────────────┘ └──────┬───────┘
                                              │
         ┌────────────────────────────────────┼────────────────────────────────┐
         │                         KUBERNETES CLUSTER                          │
         │                                                                      │
         │  ┌─────────────────────────────────────────────────────────────┐    │
         │  │                     INGRESS CONTROLLER                       │    │
         │  │              (nginx-ingress / AWS ALB)                       │    │
         │  └────────────────────────┬────────────────────────────────────┘    │
         │                           │                                         │
         │         ┌─────────────────┼─────────────────┐                      │
         │         │                 │                 │                       │
         │  ┌──────▼──────┐  ┌──────▼──────┐  ┌───────▼──────┐                │
         │  │ Superglue   │  │ Superglue   │  │  Superglue   │                │
         │  │  Pod 1      │  │  Pod 2      │  │  Pod 3       │                │
         │  │  (Active)   │  │  (Active)   │  │  (Active)    │                │
         │  └──────┬──────┘  └──────┬──────┘  └───────┬──────┘                │
         │         │                 │                 │                       │
         └─────────┼─────────────────┼─────────────────┼───────────────────────┘
                   │                 │                 │
         ┌─────────▼─────────────────▼─────────────────▼───────────────────────┐
         │                        DATA LAYER                                    │
         │                                                                       │
         │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐      │
         │  │  Redis Cluster  │  │  Redis Cluster  │  │  Redis Cluster  │      │
         │  │   (Primary)     │  │   (Replica)     │  │   (Replica)     │      │
         │  │   us-east-1     │  │   us-west-2     │  │   eu-west-1     │      │
         │  └─────────────────┘  └─────────────────┘  └─────────────────┘      │
         │                                                                       │
         └───────────────────────────────────────────────────────────────────────┘
```

### Multi-Region Deployment

```yaml
# regions.yaml
regions:
  primary:
    name: us-east-1
    role: primary
    replicas: 3
    redis:
      nodes: 6
      mode: cluster

  secondary:
    name: us-west-2
    role: failover
    replicas: 2
    redis:
      nodes: 3
      mode: sentinel

  eu:
    name: eu-west-1
    role: active
    replicas: 2
    redis:
      nodes: 3
      mode: cluster
```

### Service Mesh Integration

For microservices architectures:

```yaml
# istio configuration
apiVersion: networking.istio.io/v1beta1
kind: VirtualService
metadata:
  name: superglue-vs
spec:
  hosts:
  - superglue.internal
  http:
  - route:
    - destination:
        host: superglue-service
        port:
          number: 3000
    retries:
      attempts: 3
      perTryTimeout: 2s
    timeout: 10s
```

---

## High Availability

### Pod Disruption Budget

```yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: superglue-pdb
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: superglue
```

### Health Checks

```rust
// src/health.rs
use axum::{Json, http::StatusCode};
use serde::Serialize;
use tokio::time::{timeout, Duration};

#[derive(Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: CheckStatus,
    pub redis: CheckStatus,
    pub openai: CheckStatus,
}

#[derive(Serialize)]
pub struct CheckStatus {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

pub async fn health_check(
    db: &DatabaseService,
    redis: &CacheService,
    openai: &TransformEngine,
) -> Json<HealthStatus> {
    let redis_status = check_redis(redis).await;
    let db_status = check_database(db).await;
    let openai_status = check_openai(openai).await;

    let all_healthy = redis_status.status == "healthy"
        && db_status.status == "healthy"
        && openai_status.status == "healthy";

    Json(HealthStatus {
        status: if all_healthy { "healthy" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks: HealthChecks {
            database: db_status,
            redis: redis_status,
            openai: openai_status,
        },
    })
}

async fn check_redis(redis: &CacheService) -> CheckStatus {
    let start = std::time::Instant::now();

    match timeout(Duration::from_secs(2), redis.ping()).await {
        Ok(Ok(true)) => CheckStatus {
            status: "healthy".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
            error: None,
        },
        Ok(Err(e)) => CheckStatus {
            status: "unhealthy".to_string(),
            latency_ms: None,
            error: Some(e.to_string()),
        },
        Err(_) => CheckStatus {
            status: "unhealthy".to_string(),
            latency_ms: None,
            error: Some("Timeout".to_string()),
        },
        _ => CheckStatus {
            status: "unknown".to_string(),
            latency_ms: None,
            error: None,
        },
    }
}
```

### Readiness vs Liveness Probes

```yaml
containers:
- name: superglue
  image: superglue/superglue:latest
  livenessProbe:
    httpGet:
      path: /health/live
      port: 3000
    initialDelaySeconds: 10
    periodSeconds: 10
    timeoutSeconds: 5
    failureThreshold: 3
  readinessProbe:
    httpGet:
      path: /health/ready
      port: 3000
    initialDelaySeconds: 5
    periodSeconds: 5
    timeoutSeconds: 3
    failureThreshold: 3
  startupProbe:
    httpGet:
      path: /health
      port: 3000
    failureThreshold: 30
    periodSeconds: 10
```

---

## Scaling Strategies

### Horizontal Pod Autoscaler

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: superglue-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: superglue
  minReplicas: 3
  maxReplicas: 50
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
  - type: Pods
    pods:
      metric:
        name: requests_per_second
      target:
        type: AverageValue
        averageValue: 1000
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
      - type: Percent
        value: 100
        periodSeconds: 15
      - type: Pods
        value: 10
        periodSeconds: 15
      selectPolicy: Max
```

### Rate Limiting Implementation

```rust
// src/middleware/rate_limit.rs
use governor::{
    Quota, RateLimiter,
    state::{InMemoryState, NotKeyed},
};
use std::{num::NonZeroU32, sync::Arc};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub struct RateLimitState {
    limiter: RateLimiter<NotKeyed, InMemoryState>,
}

impl RateLimitState {
    pub fn new(requests_per_second: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap());
        Self {
            limiter: RateLimiter::direct(quota),
        }
    }

    pub fn new_per_org(org_limit: u32) -> Arc<dashmap::DashMap<String, Self>> {
        Arc::new(dashmap::DashMap::new())
    }
}

pub async fn rate_limit_middleware(
    State(state): State<Arc<RateLimitState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if state.limiter.check().is_ok() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

// Per-API-key rate limiting
use axum::extract::FromRequestParts;
use http::request::Parts;

pub struct ApiKey(pub String);

impl<S> FromRequestParts<S> for ApiKey
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "Missing API key"))?;

        Ok(ApiKey(auth_header.to_string()))
    }
}

pub async fn per_key_rate_limit(
    State(limiters): State<Arc<dashmap::DashMap<String, RateLimitState>>>,
    api_key: ApiKey,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let limiter = limiters
        .entry(api_key.0.clone())
        .or_insert_with(|| RateLimitState::new(100));

    if limiter.limiter.check().is_ok() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}
```

### Circuit Breaker Pattern

```rust
// src/middleware/circuit_breaker.rs
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failure_count: RwLock<u32>,
    success_count: RwLock<u32>,
    last_failure_time: RwLock<Option<Instant>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_count: RwLock::new(0),
            success_count: RwLock::new(0),
            last_failure_time: RwLock::new(None),
            failure_threshold,
            success_threshold,
            timeout,
        }
    }

    pub async fn call<F, T>(&self, f: F) -> Result<T, &'static str>
    where
        F: std::future::Future<Output = Result<T, &'static str>>,
    {
        match *self.state.read().await {
            CircuitState::Open => {
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() > self.timeout {
                        let mut state = self.state.write().await;
                        *state = CircuitState::HalfOpen;
                    } else {
                        return Err("Circuit breaker is OPEN");
                    }
                }
            }
            _ => {}
        }

        match f.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(e) => {
                self.on_failure().await;
                Err(e)
            }
        }
    }

    async fn on_success(&self) {
        let mut success_count = self.success_count.write().await;
        *success_count += 1;

        if *success_count >= self.success_threshold {
            let mut state = self.state.write().await;
            *state = CircuitState::Closed;
            *success_count = 0;
            *failure_count = 0;
        }
    }

    async fn on_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;
        *self.last_failure_time.write().await = Some(Instant::now());

        if *failure_count >= self.failure_threshold {
            let mut state = self.state.write().await;
            *state = CircuitState::Open;
        }
    }
}
```

---

## Security Implementation

### Authentication & Authorization

```rust
// src/auth/mod.rs
use jsonwebtoken::{decode, Validation, Algorithm, DecodingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Insufficient permissions")]
    Forbidden,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub org_id: String,     // Organization ID
    pub role: Role,
    pub permissions: Vec<String>,
    pub exp: usize,         // Expiration time
    pub iat: usize,         // Issued at
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Editor,
    Viewer,
}

pub struct JwtValidator {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtValidator {
    pub fn new(secret: &[u8]) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        Self {
            decoding_key: DecodingKey::from_secret(secret),
            validation,
        }
    }

    pub fn validate(&self, token: &str) -> Result<Claims, AuthError> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::InvalidToken(e.to_string()),
            })
    }
}

// Axum middleware for authentication
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub struct AuthState {
    pub validator: JwtValidator,
}

pub async fn auth_middleware(
    State(state): State<Arc<AuthState>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = state.validator.validate(auth_header)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Insert claims into request extensions
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
```

### Secret Management

```rust
// src/config/secrets.rs
use secrecy::{Secret, ExposeSecret};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_secretsmanager::Client as SecretsManagerClient;

pub struct SecretManager {
    client: Option<SecretsManagerClient>,
    local_secrets: std::collections::HashMap<String, Secret<String>>,
}

impl SecretManager {
    pub async fn new(use_aws: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let client = if use_aws {
            let region_provider = RegionProviderChain::default_provider();
            let config = aws_config::from_env().region(region_provider).load().await;
            Some(SecretsManagerClient::new(&config))
        } else {
            None
        };

        Ok(Self {
            client,
            local_secrets: std::collections::HashMap::new(),
        })
    }

    pub async fn get_secret(&self, secret_name: &str) -> Result<Secret<String>, Box<dyn std::error::Error>> {
        // Check local cache first
        if let Some(secret) = self.local_secrets.get(secret_name) {
            return Ok(secret.clone());
        }

        // Fetch from AWS Secrets Manager
        if let Some(client) = &self.client {
            let response = client.get_secret_value()
                .secret_id(secret_name)
                .send()
                .await?;

            if let Some(secret_string) = response.secret_string {
                let secret = Secret::new(secret_string);
                return Ok(secret);
            }
        }

        // Fallback to environment variable
        let value = std::env::var(secret_name)
            .map_err(|_| format!("Secret {} not found", secret_name))?;

        Ok(Secret::new(value))
    }
}

// Usage in application
pub struct AppConfig {
    pub openai_api_key: Secret<String>,
    pub redis_password: Secret<String>,
    pub jwt_secret: Secret<String>,
}

impl AppConfig {
    pub async fn load(secret_manager: &SecretManager) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            openai_api_key: secret_manager.get_secret("OPENAI_API_KEY").await?,
            redis_password: secret_manager.get_secret("REDIS_PASSWORD").await?,
            jwt_secret: secret_manager.get_secret("JWT_SECRET").await?,
        })
    }
}

// When using the secret
fn connect_to_openai(config: &AppConfig) {
    let api_key = config.openai_api_key.expose_secret();
    // Use api_key...
}
```

### Input Validation

```rust
// src/validation.rs
use validator::{Validate, ValidationError};
use regex::Regex;

#[derive(Debug, Validate)]
pub struct ApiInputValidation {
    #[validate(length(min = 1, max = 2048, message = "URL host must be between 1-2048 characters"))]
    pub url_host: String,

    #[validate(custom = "validate_url_path")]
    pub url_path: Option<String>,

    #[validate(length(min = 1, max = 10000, message = "Instruction is required"))]
    pub instruction: String,

    #[validate(custom = "validate_json_schema")]
    pub response_schema: Option<String>,
}

fn validate_url_path(path: &str) -> Result<(), ValidationError> {
    if path.is_empty() {
        return Ok(());
    }

    // Must start with /
    if !path.starts_with('/') {
        return Err(ValidationError::new("url_path_must_start_with_slash"));
    }

    // No double slashes
    if path.contains("//") {
        return Err(ValidationError::new("url_path_no_double_slash"));
    }

    Ok(())
}

fn validate_json_schema(schema: &str) -> Result<(), ValidationError> {
    serde_json::from_str::<serde_json::Value>(schema)
        .map_err(|_| ValidationError::new("invalid_json_schema"))?;

    Ok(())
}

// Sanitize user input
use ammonia::Builder;

pub fn sanitize_input(input: &str) -> String {
    Builder::default()
        .tags(std::collections::HashSet::new())  // No HTML tags
        .clean(input)
        .to_string()
}

// Prevent injection attacks in JSONata expressions
pub fn validate_jsonata_expression(expr: &str) -> Result<(), ValidationError> {
    // Block dangerous patterns
    let dangerous_patterns = [
        "__proto__",
        "constructor",
        "prototype",
        "process.env",
        "require(",
        "eval(",
        "Function(",
    ];

    for pattern in dangerous_patterns {
        if expr.contains(pattern) {
            return Err(ValidationError::new("dangerous_expression"));
        }
    }

    Ok(())
}
```

---

## Monitoring & Observability

### OpenTelemetry Integration

```rust
// src/telemetry.rs
use opentelemetry::{
    global,
    trace::{Tracer, Span, SpanKind, Status},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{layer::SubscriberExt, Registry};

pub fn init_telemetry(
    service_name: &str,
    otlp_endpoint: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create OTLP exporter
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .with_trace_config(
            opentelemetry::sdk::trace::config()
                .with_resource(opentelemetry::sdk::Resource::new(vec![
                    KeyValue::new("service.name", service_name),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])),
        )
        .install_batch(opente_runtime::Tokio)?;

    // Create metrics exporter
    let meter = opentelemetry_otlp::new_pipeline()
        .metrics(opente_runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .build()?;

    global::set_tracer_provider(tracer.provider().clone());
    global::set_meter_provider(meter);

    // Set up tracing subscriber
    let telemetry_layer = OpenTelemetryLayer::new(tracer);

    let subscriber = Registry::default()
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env());

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

// Usage with instrumentation
use tracing::{info_span, Instrument};

pub async fn process_transform(
    config: &TransformConfig,
    data: &serde_json::Value,
) -> Result<TransformResult, TransformError> {
    let span = info_span!(
        "transform",
        config_id = config.id,
        instruction = %config.instruction
    );

    async move {
        // Add custom attributes
        Span::current().record("data_size", data.to_string().len());

        // Process...
        let result = execute_transform(config, data).await?;

        // Record metrics
        metrics::counter!("transforms_total", 1);
        metrics::histogram!("transform_duration_ms", start.elapsed().as_millis() as f64);

        Ok(result)
    }
    .instrument(span)
    .await
}
```

### Prometheus Metrics

```rust
// src/metrics.rs
use prometheus::{
    Registry, Counter, Gauge, Histogram, HistogramOpts, Opts,
};
use std::sync::Arc;

pub struct SuperglueMetrics {
    registry: Registry,
    requests_total: Counter,
    request_duration: Histogram,
    active_connections: Gauge,
    cache_hits: Counter,
    cache_misses: Counter,
    transform_errors: Counter,
    api_errors: Counter,
}

impl SuperglueMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let requests_total = Counter::new(
            "superglue_requests_total",
            "Total number of requests"
        )?;

        let request_duration = Histogram::with_opts(
            HistogramOpts::new(
                "superglue_request_duration_seconds",
                "Request duration in seconds"
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0])
        )?;

        let active_connections = Gauge::new(
            "superglue_active_connections",
            "Number of active connections"
        )?;

        let cache_hits = Counter::new(
            "superglue_cache_hits_total",
            "Total cache hits"
        )?;

        let cache_misses = Counter::new(
            "superglue_cache_misses_total",
            "Total cache misses"
        )?;

        let transform_errors = Counter::new(
            "superglue_transform_errors_total",
            "Total transform errors"
        )?;

        let api_errors = Counter::new(
            "superglue_api_errors_total",
            "Total API errors"
        )?;

        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(request_duration.clone()))?;
        registry.register(Box::new(active_connections.clone()))?;
        registry.register(Box::new(cache_hits.clone()))?;
        registry.register(Box::new(cache_misses.clone()))?;
        registry.register(Box::new(transform_errors.clone()))?;
        registry.register(Box::new(api_errors.clone()))?;

        Ok(Self {
            registry,
            requests_total,
            request_duration,
            active_connections,
            cache_hits,
            cache_misses,
            transform_errors,
            api_errors,
        })
    }

    pub fn record_request(&self, duration: f64) {
        self.requests_total.inc();
        self.request_duration.observe(duration);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.inc();
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.inc();
    }

    pub fn record_transform_error(&self) {
        self.transform_errors.inc();
    }

    pub fn record_api_error(&self) {
        self.api_errors.inc();
    }

    pub fn set_active_connections(&self, count: i64) {
        self.active_connections.set(count as f64);
    }

    pub fn gather(&self) -> Vec<prometheus::proto::MetricFamily> {
        self.registry.gather()
    }
}

// Axum middleware for metrics
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::Instant;

pub async fn metrics_middleware(
    State(metrics): State<Arc<SuperglueMetrics>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();

    let response = next.run(req).await;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();

    metrics.record_request(duration);

    if status >= 500 {
        metrics.record_api_error();
    }

    response
}

// Metrics endpoint
use axum::{Router, routing::get};

pub fn create_metrics_route(metrics: Arc<SuperglueMetrics>) -> Router {
    Router::new().route(
        "/metrics",
        get(move || {
            let metrics = metrics.clone();
            async move {
                let encoder = prometheus::TextEncoder::new();
                let metric_families = metrics.gather();
                let mut output = Vec::new();
                encoder.encode(&metric_families, &mut output).unwrap();
                output
            }
        })
    )
}
```

### Structured Logging

```rust
// src/logging.rs
use tracing::{event, Level, info, warn, error};
use tracing_subscriber::fmt::format::JsonFields;

pub fn init_logging() {
    tracing_subscriber::fmt()
        .json()  // JSON format for production
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

// Usage with structured fields
pub async fn process_api_call(config: &ApiConfig) -> Result<(), ApiError> {
    info!(
        target = "superglue::api",
        config_id = %config.id,
        url_host = %config.url_host,
        method = ?config.method,
        "Starting API call"
    );

    match execute_request(config).await {
        Ok(response) => {
            info!(
                target = "superglue::api",
                config_id = %config.id,
                status_code = response.status().as_u16(),
                response_time_ms = response.elapsed().as_millis(),
                "API call completed"
            );
            Ok(())
        }
        Err(e) => {
            error!(
                target = "superglue::api",
                config_id = %config.id,
                error = %e,
                "API call failed"
            );
            Err(e)
        }
    }
}

// Correlation IDs for request tracing
use uuid::Uuid;
use axum::{
    extract::State,
    http::{Request, HeaderMap},
    middleware::Next,
    response::Response,
};

pub async fn correlation_id_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    // Get or generate correlation ID
    let correlation_id = req.headers()
        .get("X-Correlation-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Add to request
    let (mut parts, body) = req.into_parts();
    parts.headers.insert(
        "X-Correlation-ID",
        correlation_id.parse().unwrap(),
    );

    let req = Request::from_parts(parts, body);

    // Add to response
    let response = next.run(req).await;
    response
}
```

---

## Performance Optimization

### Connection Pooling

```rust
// src/pool.rs
use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use reqwest::Client;

pub struct ConnectionPools {
    pub redis: Pool<RedisConnectionManager>,
    pub http: Client,
}

impl ConnectionPools {
    pub async fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Redis pool
        let redis_manager = RedisConnectionManager::new(config.redis_url.clone())?;
        let redis_pool = Pool::builder()
            .max_size(config.redis_pool_size)
            .min_idle(Some(config.redis_min_idle))
            .max_lifetime(Some(std::time::Duration::from_secs(300)))
            .idle_timeout(Some(std::time::Duration::from_secs(60)))
            .build(redis_manager)
            .await?;

        // HTTP client with connection pooling
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .pool_max_idle_per_host(config.http_pool_idle_per_host as usize)
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .tcp_keepalive(std::time::Duration::from_secs(30))
            .gzip(true)
            .build()?;

        Ok(Self {
            redis: redis_pool,
            http: http_client,
        })
    }
}
```

### Caching Strategy

```rust
// src/cache/layer.rs
use moka::future::Cache;
use std::time::Duration;

pub struct LayeredCache {
    // L1: In-memory cache (hot data)
    l1_cache: Cache<String, serde_json::Value>,

    // L2: Redis cache (warm data)
    l2_cache: Arc<CacheService>,
}

impl LayeredCache {
    pub fn new(l2_cache: Arc<CacheService>) -> Self {
        let l1_cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(300))  // 5 minutes
            .time_to_idle(Duration::from_secs(60))    // 1 minute
            .build();

        Self {
            l1_cache,
            l2_cache,
        }
    }

    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        // Try L1 first
        if let Some(value) = self.l1_cache.get(key).await {
            return Some(value);
        }

        // Try L2
        if let Ok(Some(value)) = self.l2_cache.get("data", key, None).await {
            // Populate L1
            self.l1_cache.insert(key.to_string(), value.clone()).await;
            return Some(value);
        }

        None
    }

    pub async fn set(&self, key: &str, value: serde_json::Value) {
        // Write to both L1 and L2
        self.l1_cache.insert(key.to_string(), value.clone()).await;
        let _ = self.l2_cache.set("data", key, &value, None).await;
    }

    pub async fn invalidate(&self, key: &str) {
        self.l1_cache.invalidate(key).await;
        let _ = self.l2_cache.delete("data", key, None).await;
    }
}
```

### Batch Processing

```rust
// src/batch.rs
use tokio::task::JoinSet;
use futures::stream::{self, StreamExt};

pub async fn process_batch<T, R, F, Fut>(
    items: Vec<T>,
    concurrency: usize,
    processor: F,
) -> Vec<Result<R>>
where
    F: Fn(T) -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = Result<R>> + Send + 'static,
    R: Send + 'static,
    T: Send + 'static,
{
    stream::iter(items)
        .map(|item| processor(item))
        .buffered(concurrency)
        .collect()
        .await
}

// Usage
let results = process_batch(
    config_ids,
    10,  // Process 10 concurrently
    |config_id| async move {
        process_single(config_id).await
    },
).await;
```

---

## Disaster Recovery

### Backup Strategy

```yaml
# k8s/backup-cronjob.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: superglue-backup
spec:
  schedule: "0 */6 * * *"  # Every 6 hours
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: backup
            image: superglue/backup:latest
            env:
            - name: REDIS_HOST
              value: "redis-service"
            - name: S3_BUCKET
              value: "superglue-backups"
            - name: AWS_REGION
              value: "us-east-1"
            volumeMounts:
            - name: backup-config
              mountPath: /etc/backup
          volumes:
          - name: backup-config
            secret:
              secretName: backup-credentials
          restartPolicy: OnFailure
```

### Point-in-Time Recovery

```rust
// src/recovery.rs
use chrono::{DateTime, Utc, Duration};

pub struct RecoveryManager {
    s3_client: aws_sdk_s3::Client,
    bucket: String,
}

impl RecoveryManager {
    pub async fn restore_to_point_in_time(
        &self,
        target_time: DateTime<Utc>,
    ) -> Result<(), RecoveryError> {
        // 1. Find the latest backup before target_time
        let backup_key = self.find_backup_before(target_time).await?;

        // 2. Download backup from S3
        let backup_data = self.download_backup(&backup_key).await?;

        // 3. Restore to Redis
        self.restore_to_redis(backup_data).await?;

        // 4. Replay transactions between backup and target_time
        self.replay_transactions(target_time).await?;

        Ok(())
    }

    async fn find_backup_before(&self, target_time: DateTime<Utc>) -> Result<String, RecoveryError> {
        // List objects with prefix
        let response = self.s3_client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix("backup/")
            .send()
            .await?;

        // Find the latest backup before target_time
        let mut latest_backup = None;
        for obj in response.contents() {
            if let Some(last_modified) = obj.last_modified() {
                let backup_time = DateTime::<Utc>::from(*last_modified);
                if backup_time <= target_time {
                    if latest_backup.is_none() || backup_time > latest_backup.unwrap() {
                        latest_backup = Some(backup_time);
                    }
                }
            }
        }

        latest_backup
            .map(|t| format!("backup/{}.json", t.timestamp()))
            .ok_or(RecoveryError::NoBackupFound)
    }
}
```

---

## CI/CD Pipeline

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
name: CI/CD

on:
  push:
    branches: [main, staging]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379

    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: dtolnay/rust-action@stable

    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

    - name: Run tests
      run: cargo test --all-features

    - name: Run clippy
      run: cargo clippy --all-targets --all-features -- -D warnings

    - name: Run fmt
      run: cargo fmt --all --check

  build:
    needs: test
    runs-on: ubuntu-latest
    if: github.event_name == 'push'

    steps:
    - uses: actions/checkout@v4

    - name: Set up Docker Buildx
      uses: docker/setup-buildx-action@v3

    - name: Login to Docker Hub
      uses: docker/login-action@v3
      with:
        username: ${{ secrets.DOCKER_USERNAME }}
        password: ${{ secrets.DOCKER_PASSWORD }}

    - name: Build and push
      uses: docker/build-push-action@v5
      with:
        context: .
        push: true
        tags: |
          superglue/superglue:latest
          superglue/superglue:${{ github.sha }}
        cache-from: type=registry,ref=superglue/superglue:buildcache
        cache-to: type=registry,ref=superglue/superglue:buildcache,mode=max

  deploy-staging:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/staging'

    steps:
    - uses: actions/checkout@v4

    - name: Deploy to staging
      run: |
        kubectl apply -f k8s/staging/
        kubectl rollout restart deployment/superglue -n staging

  deploy-production:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'

    environment: production

    steps:
    - uses: actions/checkout@v4

    - name: Deploy to production
      run: |
        kubectl apply -f k8s/production/
        kubectl rollout restart deployment/superglue -n production
```

---

**Document completed:** 2026-03-25
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.superglue/`
