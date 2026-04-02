---
source: /home/darkvoid/Boxxed/@formulas/src.AppOSS
reference_projects: n8n, baserow, Penpot, Budibase, Skia
created_at: 2026-04-02
tags: production, scalability, reliability, security, deployment
---

# Production-Grade AppOSS: Building for Scale

## Overview

This guide covers what it takes to build production-grade versions of the AppOSS applications. We cover architecture patterns, operational concerns, security, and scaling strategies.

---

## Part 1: Architecture Patterns

### 1.1 Microservices vs Monolith

#### Monolith (Recommended for Start)

```
┌─────────────────────────────────────────────────────────┐
│                   Application Server                     │
│  ┌───────────┬───────────┬───────────┬───────────┐     │
│  │   HTTP    │   Core    │   Data    │  Render   │     │
│  │   Layer   │  Logic    │   Layer   │  Engine   │     │
│  └───────────┴───────────┴───────────┴───────────┘     │
│                        │                                │
│                   [Database]                            │
│                   [Redis]                               │
│                   [S3]                                  │
└─────────────────────────────────────────────────────────┘
```

**Pros:**
- Simple deployment
- Easy debugging
- No distributed system complexity
- Lower latency (no network calls)

**Cons:**
- Harder to scale independently
- Single point of failure
- Codebase can become unwieldy

#### Microservices (For Scale)

```
┌─────────────────────────────────────────────────────────┐
│                      API Gateway                         │
│                   (Kong, Envoy, NGINX)                  │
└─────────────────────────────────────────────────────────┘
         │              │              │
         ▼              ▼              ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│   User      │ │  Document   │ │   Render    │
│   Service   │ │  Service    │ │   Service   │
│             │ │             │ │             │
│  [Postgres] │ │  [Postgres] │ │  [Redis]    │
└─────────────┘ └─────────────┘ └─────────────┘
         │              │              │
         └──────────────┼──────────────┘
                        │
                        ▼
               ┌─────────────┐
               │  Message    │
               │   Queue     │
               │  (NATS/     │
               │   RabbitMQ) │
               └─────────────┘
```

**When to split:**
- Team size > 10 engineers
- Different scaling requirements
- Different reliability requirements
- Clear domain boundaries

### 1.2 Event-Driven Architecture

```rust
// Event types
#[derive(Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    // Document events
    DocumentCreated { id: String, user_id: String },
    DocumentUpdated { id: String, user_id: String, changes: Vec<Change> },
    DocumentDeleted { id: String, user_id: String },
    
    // Collaboration events
    UserJoined { document_id: String, user_id: String },
    UserLeft { document_id: String, user_id: String },
    CursorMoved { document_id: String, user_id: String, position: Position },
    
    // System events
    RenderRequested { document_id: String, format: RenderFormat },
    ExportCompleted { document_id: String, url: String },
}

// Event bus trait
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> Result<(), Error>;
    async fn subscribe(&self, handler: Box<dyn EventHandler>) -> Subscription;
}

// NATS implementation
pub struct NatsEventBus {
    connection: async_nats::Client,
}

impl EventBus for NatsEventBus {
    async fn publish(&self, event: DomainEvent) -> Result<(), Error> {
        let topic = format!("app.{}", event.type_name());
        let data = serde_json::to_vec(&event)?;
        self.connection.publish(topic, data.into()).await?;
        Ok(())
    }
    
    async fn subscribe(&self, handler: Box<dyn EventHandler>) -> Subscription {
        // Subscribe to relevant topics
    }
}
```

### 1.3 CQRS Pattern

```rust
// Command side (writes)
pub struct CommandHandler {
    repo: Arc<dyn DocumentRepository>,
    event_bus: Arc<dyn EventBus>,
}

impl CommandHandler {
    pub async fn create_document(&self, cmd: CreateDocument) -> Result<DocumentId, Error> {
        let doc = Document::create(&cmd);
        
        // Persist
        self.repo.save(&doc).await?;
        
        // Emit event
        self.event_bus.publish(DomainEvent::DocumentCreated {
            id: doc.id.to_string(),
            user_id: cmd.user_id,
        }).await?;
        
        Ok(doc.id)
    }
}

// Query side (reads)
pub struct QueryHandler {
    read_db: Arc<dyn ReadRepository>,
}

impl QueryHandler {
    pub async fn get_document(&self, id: DocumentId) -> Result<DocumentView, Error> {
        // Optimized read model (denormalized)
        self.read_db.find_document(id).await
    }
    
    pub async fn search_documents(&self, query: &str, user_id: UserId) -> Result<Vec<DocumentSummary>, Error> {
        // Full-text search optimized
        self.read_db.search_documents(query, user_id).await
    }
}
```

---

## Part 2: Data Layer

### 2.1 Database Schema

```sql
-- Core tables
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255),
    display_name VARCHAR(255),
    avatar_url VARCHAR(500),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE team_members (
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL, -- 'owner', 'admin', 'member', 'viewer'
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (team_id, user_id)
);

CREATE TABLE documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    team_id UUID REFERENCES teams(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    
    -- For optimistic locking
    version INTEGER DEFAULT 0
);

CREATE INDEX idx_documents_team ON documents(team_id);
CREATE INDEX idx_documents_created_by ON documents(created_by);
CREATE INDEX idx_documents_updated_at ON documents(updated_at DESC);

-- Document content (stored as JSONB or separate blob storage)
CREATE TABLE document_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    content JSONB NOT NULL,
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(document_id, version)
);

-- For real-time collaboration state
CREATE TABLE document_states (
    document_id UUID PRIMARY KEY REFERENCES documents(id) ON DELETE CASCADE,
    state_vector JSONB NOT NULL, -- LWW or CRDT state
    last_sync_at TIMESTAMPTZ DEFAULT NOW()
);

-- Audit log
CREATE TABLE audit_logs (
    id BIGSERIAL PRIMARY KEY,
    document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    action VARCHAR(100) NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_audit_logs_document ON audit_logs(document_id);
CREATE INDEX idx_audit_logs_created_at ON audit_logs(created_at DESC);
```

### 2.2 Connection Pooling

```rust
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

pub async fn create_pool(database_url: &str) -> Result<Pool<Postgres>, Error> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(database_url)
        .await?;
    
    Ok(pool)
}

// Health check
pub async fn check_database_health(pool: &Pool<Postgres>) -> bool {
    sqlx::query("SELECT 1")
        .fetch_optional(pool)
        .await
        .is_ok()
}
```

### 2.3 Caching Strategy

```rust
use redis::{Client, Connection, AsyncCommands};
use std::time::Duration;

pub struct Cache {
    client: Client,
}

impl Cache {
    pub fn new(redis_url: &str) -> Result<Self, Error> {
        let client = Client::open(redis_url)?;
        Ok(Self { client })
    }
    
    pub async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>, Error> {
        let mut conn = self.client.get_async_connection().await?;
        let value: Option<String> = conn.get(key).await?;
        
        match value {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }
    
    pub async fn set(&self, key: &str, value: &impl serde::Serialize, ttl: Duration) -> Result<(), Error> {
        let mut conn = self.client.get_async_connection().await?;
        let serialized = serde_json::to_string(value)?;
        conn.set_ex(key, serialized, ttl.as_secs()).await?;
        Ok(())
    }
    
    pub async fn invalidate(&self, key: &str) -> Result<(), Error> {
        let mut conn = self.client.get_async_connection().await?;
        conn.del(key).await?;
        Ok(())
    }
}

// Cache-aside pattern
pub struct CachedDocumentRepo {
    db: Pool<Postgres>,
    cache: Cache,
}

impl CachedDocumentRepo {
    pub async fn get_document(&self, id: DocumentId) -> Result<Option<Document>, Error> {
        let cache_key = format!("document:{}", id);
        
        // Try cache first
        if let Some(doc) = self.cache.get(&cache_key).await? {
            metrics::CACHE_HITS.inc();
            return Ok(Some(doc));
        }
        
        // Fallback to database
        metrics::CACHE_MISSES.inc();
        let doc = sqlx::query_as!(Document, "SELECT * FROM documents WHERE id = $1", id)
            .fetch_optional(&self.db)
            .await?;
        
        // Populate cache
        if let Some(ref d) = doc {
            self.cache.set(&cache_key, d, Duration::from_secs(300)).await?;
        }
        
        Ok(doc)
    }
}
```

---

## Part 3: Real-Time Collaboration

### 3.1 CRDT Implementation

```rust
use automerge::{Automerge, Change, ActorId};

pub struct CollaborativeDocument {
    doc: Automerge,
    actor: ActorId,
}

impl CollaborativeDocument {
    pub fn new() -> Self {
        Self {
            doc: Automerge::new(),
            actor: ActorId::random(),
        }
    }
    
    pub fn apply_change(&mut self, change: Change) -> Result<(), Error> {
        self.doc.apply_change(change)?;
        Ok(())
    }
    
    pub fn insert_text(&mut self, path: &[String], text: &str) -> Vec<Change> {
        let mut tx = self.doc.transaction();
        
        // Navigate to path and insert
        let mut obj = tx.root();
        for key in path {
            obj = obj.get(key).unwrap();
        }
        
        tx.insert(obj, text)?;
        
        tx.commit().into_iter().collect()
    }
    
    pub fn encode(&self) -> Vec<u8> {
        self.doc.save()
    }
    
    pub fn decode(data: &[u8]) -> Result<Self, Error> {
        let doc = Automerge::load(data)?;
        Ok(Self {
            doc,
            actor: ActorId::random(),
        })
    }
}
```

### 3.2 WebSocket Server

```rust
use tokio::sync::broadcast;
use axum::extract::ws::{Message, WebSocket};

pub struct CollaborationServer {
    rooms: DashMap<DocumentId, broadcast::Sender<CollabMessage>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum CollabMessage {
    Sync { data: Vec<u8> },
    Cursor { user_id: UserId, position: Position },
    Presence { user_id: UserId, status: UserStatus },
    Ack { sequence: u64 },
}

impl CollaborationServer {
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
        }
    }
    
    pub async fn handle_connection(
        &self,
        ws: WebSocket,
        document_id: DocumentId,
        user_id: UserId,
    ) {
        let (tx, mut rx) = self.get_or_create_room(document_id);
        let (mut sink, mut stream) = ws.split();
        
        // Send existing state
        let initial_state = self.get_document_state(document_id).await;
        let _ = sink.send(Message::Binary(initial_state)).await;
        
        // Broadcast presence
        tx.send(CollabMessage::Presence {
            user_id,
            status: UserStatus::Online,
        }).ok();
        
        loop {
            tokio::select! {
                // Receive from client
                msg = stream.next() => {
                    match msg {
                        Some(Ok(Message::Binary(data))) => {
                            // Process and broadcast
                            if let Ok(change) = deserialize_change(&data) {
                                self.apply_change(document_id, change).await;
                                tx.send(CollabMessage::Sync { data }).ok();
                            }
                        }
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(cursor) = serde_json::from_str::<Position>(&text) {
                                tx.send(CollabMessage::Cursor { user_id, position: cursor }).ok();
                            }
                        }
                        None | Some(Err(_)) => break,
                        _ => {}
                    }
                }
                
                // Broadcast to client
                msg = rx.recv() => {
                    match msg {
                        Ok(CollabMessage::Sync { data }) => {
                            if sink.send(Message::Binary(data)).await.is_err() {
                                break;
                            }
                        }
                        Ok(CollabMessage::Cursor { user_id: sender, position }) => {
                            if sender != user_id {
                                let msg = serde_json::to_string(&position).unwrap();
                                if sink.send(Message::Text(msg)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Ok(CollabMessage::Presence { user_id: sender, status }) => {
                            if sender != user_id {
                                let msg = serde_json::to_string(&status).unwrap();
                                if sink.send(Message::Text(msg)).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        
        // Cleanup
        tx.send(CollabMessage::Presence {
            user_id,
            status: UserStatus::Offline,
        }).ok();
        
        self.cleanup_if_empty(document_id);
    }
    
    fn get_or_create_room(&self, document_id: DocumentId) -> broadcast::Sender<CollabMessage> {
        self.rooms
            .entry(document_id)
            .or_insert_with(|| broadcast::channel(1000).0)
            .clone()
    }
}
```

---

## Part 4: Security

### 4.1 Authentication

```rust
use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
use jsonwebtoken::{encode, decode, Header, Validation, Algorithm};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,      // User ID
    email: String,
    team_ids: Vec<String>,
    exp: usize,       // Expiration
    iat: usize,       // Issued at
}

pub struct AuthService {
    db: Pool<Postgres>,
    jwt_secret: Vec<u8>,
}

impl AuthService {
    pub fn new(db: Pool<Postgres>, jwt_secret: String) -> Self {
        Self {
            db,
            jwt_secret: jwt_secret.into_bytes(),
        }
    }
    
    pub async fn register(&self, email: &str, password: &str) -> Result<UserId, Error> {
        // Hash password
        let salt = argon2::password_hash::SaltString::generate(rand::thread_rng());
        let phc = argon2::PasswordHash::generate(argon2::Argon2::default(), password, salt)?;
        let password_hash = phc.to_string();
        
        // Create user
        let user_id = uuid::Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, email, password_hash) VALUES ($1, $2, $3)",
            user_id,
            email,
            password_hash,
        )
        .execute(&self.db)
        .await?;
        
        Ok(user_id)
    }
    
    pub async fn login(&self, email: &str, password: &str) -> Result<String, Error> {
        // Get user
        let user = sqlx::query!(
            "SELECT id, password_hash FROM users WHERE email = $1",
            email,
        )
        .fetch_one(&self.db)
        .await?;
        
        // Verify password
        let parsed_hash = PasswordHash::new(&user.password_hash)?;
        Argon2::default().verify_password(password.as_bytes(), &parsed_hash)?;
        
        // Generate JWT
        let claims = Claims {
            sub: user.id.to_string(),
            email: email.to_string(),
            team_ids: self.get_user_teams(user.id).await?,
            exp: (chrono::Utc::now() + chrono::Duration::days(7)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
        };
        
        let token = encode(&Header::default(), &claims, &jsonwebtoken::EncodingKey::from_secret(&self.jwt_secret))?;
        Ok(token)
    }
    
    pub fn verify_token(&self, token: &str) -> Result<Claims, Error> {
        let data = decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(&self.jwt_secret),
            &Validation::new(Algorithm::HS256),
        )?;
        Ok(data.claims)
    }
}
```

### 4.2 Authorization Middleware

```rust
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub async fn auth_middleware(
    auth_state: Extension<AuthState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());
    
    match auth_header {
        Some(header) => {
            if let Some(token) = header.strip_prefix("Bearer ") {
                if let Ok(claims) = auth_state.service.verify_token(token) {
                    request.extensions_mut().insert(claims);
                    return Ok(next.run(request).await);
                }
            }
        }
        None => {}
    }
    
    Err(StatusCode::UNAUTHORIZED)
}

pub async fn require_permission(
    Extension(claims): Extension<Claims>,
    Path(document_id): Path<String>,
    Extension(db): Extension<Pool<Postgres>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let doc_id = uuid::Uuid::parse_str(&document_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    // Check permission
    let has_access = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM documents d
            JOIN team_members tm ON d.team_id = tm.team_id
            WHERE d.id = $1 AND tm.user_id = $2
        )
        "#,
        doc_id,
        claims.sub,
    )
    .fetch_one(&db)
    .await
    .map(|r| r.exists)
    .unwrap_or(false);
    
    if !has_access {
        return Err(StatusCode::FORBIDDEN);
    }
    
    Ok(next.run(request).await)
}
```

### 4.3 Rate Limiting

```rust
use governor::{Quota, RateLimiter, clock::DefaultClock};
use std::num::NonZeroU32;
use tower_governor::{GovernorConfig, governor_axum::GovernorLayer};

pub fn rate_limit_config() -> GovernorLayer {
    let config = GovernorConfig::builder()
        .per_second(10)
        .burst_size(50)
        .finish()
        .unwrap();
    
    GovernorLayer { config }
}

// Per-user rate limiting
pub struct UserRateLimiter {
    limiters: DashMap<UserId, RateLimiter<DefaultClock>>,
    quota: Quota,
}

impl UserRateLimiter {
    pub fn new(requests_per_second: u32, burst: u32) -> Self {
        Self {
            limiters: DashMap::new(),
            quota: Quota::with_period(
                std::time::Duration::from_millis(1000 / requests_per_second as u64)
            ).unwrap().allow_burst(NonZeroU32::new(burst).unwrap()),
        }
    }
    
    pub fn check(&self, user_id: &UserId) -> Result<(), governor::NotUntil> {
        let limiter = self.limiters
            .entry(*user_id)
            .or_insert_with(|| RateLimiter::direct(self.quota));
        
        limiter.check()
    }
}
```

---

## Part 5: Observability

### 5.1 Structured Logging

```rust
use tracing::{info, warn, error, Span};
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_thread_ids(true)
        .json()
        .init();
}

// Usage
pub async fn create_document(
    user_id: UserId,
    name: String,
) -> Result<DocumentId, Error> {
    let span = Span::current();
    span.record("user_id", &user_id.to_string());
    span.record("document_name", &name);
    
    info!("Creating document");
    
    match do_create_document(user_id, name).await {
        Ok(id) => {
            info!(document_id = %id, "Document created");
            Ok(id)
        }
        Err(e) => {
            error!(error = %e, "Failed to create document");
            Err(e)
        }
    }
}
```

### 5.2 Metrics (Prometheus)

```rust
use prometheus::{Registry, Counter, Histogram, opts};

pub struct Metrics {
    registry: Registry,
    http_requests: Counter,
    request_duration: Histogram,
    db_query_duration: Histogram,
    cache_hits: Counter,
    cache_misses: Counter,
    websocket_connections: Gauge,
}

impl Metrics {
    pub fn new() -> Result<Self, Error> {
        let registry = Registry::new();
        
        let http_requests = Counter::new("http_requests_total", "Total HTTP requests")?;
        registry.register(Box::new(http_requests.clone()))?;
        
        let request_duration = Histogram::with_opts(opts!(
            "http_request_duration_seconds",
            "HTTP request duration in seconds"
        ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]))?;
        registry.register(Box::new(request_duration.clone()))?;
        
        Ok(Self {
            registry,
            http_requests,
            request_duration,
            db_query_duration: Histogram::with_opts(opts!("db_query_duration_seconds", "DB query duration"))?,
            cache_hits: Counter::new("cache_hits_total", "Cache hits")?,
            cache_misses: Counter::new("cache_misses_total", "Cache misses")?,
            websocket_connections: Gauge::new("websocket_connections", "Active WebSocket connections")?,
        })
    }
    
    pub fn record_request(&self, duration: f64) {
        self.http_requests.inc();
        self.request_duration.observe(duration);
    }
}

// Metrics endpoint
pub async fn metrics_handler(
    Extension(metrics): Extension<Arc<Metrics>>,
) -> impl IntoResponse {
    use prometheus::Encoder;
    
    let encoder = prometheus::TextEncoder::new();
    let metric_families = metrics.registry.gather();
    
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    Response::builder()
        .header(http::header::CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(buffer)
        .unwrap()
}
```

### 5.3 Distributed Tracing (OpenTelemetry)

```rust
use opentelemetry::{global, sdk::trace as sdktrace};
use opentelemetry_otlp::WithExportConfig;
use tracing_opentelemetry::OpenTelemetryLayer;

pub fn init_tracing(service_name: &str) -> Result<(), Error> {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            sdktrace::config()
                .with_resource(Resource::new(vec![KeyValue::new("service.name", service_name)]))
                .with_sampler(sdktrace::Sampler::AlwaysOn),
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(OpenTelemetryLayer::new(tracer))
        .init();
    
    Ok(())
}

// Usage - trace is automatically propagated
#[instrument(skip(self, db), fields(document_id = %id))]
pub async fn get_document(&self, id: DocumentId) -> Result<Document, Error> {
    // Span context propagates to DB calls, HTTP calls, etc.
    self.repo.find(id).await
}
```

---

## Part 6: Deployment

### 6.1 Docker Configuration

```dockerfile
# Dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/myapp /usr/local/bin/

EXPOSE 3000

CMD ["myapp"]
```

### 6.2 Docker Compose

```yaml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgresql://postgres:password@db:5432/app
      - REDIS_URL=redis://redis:6379
      - JWT_SECRET=${JWT_SECRET}
      - RUST_LOG=info
    depends_on:
      db:
        condition: service_healthy
      redis:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  db:
    image: postgres:15-alpine
    volumes:
      - postgres_data:/var/lib/postgresql/data
    environment:
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=app
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  nginx:
    image: nginx:alpine
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
    ports:
      - "80:80"
      - "443:443"
    depends_on:
      - app

volumes:
  postgres_data:
  redis_data:
```

### 6.3 Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: app
  labels:
    app: myapp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: myapp
  template:
    metadata:
      labels:
        app: myapp
    spec:
      containers:
      - name: app
        image: myregistry/myapp:latest
        ports:
        - containerPort: 3000
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: app-secrets
              key: database-url
        - name: JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: app-secrets
              key: jwt-secret
        - name: REDIS_URL
          value: "redis://redis-master:6379"
        resources:
          requests:
            memory: "256Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: app-service
spec:
  selector:
    app: myapp
  ports:
  - port: 80
    targetPort: 3000
  type: ClusterIP
---
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: app-hpa
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: app
  minReplicas: 3
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
```

---

## Part 7: Testing Strategy

### 7.1 Test Pyramid

```
              ▲
             / \
            /   \
           / E2E \          Few tests, full stack
          /───────\
         /         \
        /Integration\      Service boundaries
       /─────────────\
      /               \
     /    Unit Tests   \   Many tests, isolated
    /───────────────────\
```

### 7.2 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_transform_composition() {
        let t1 = Transform::translate(10.0, 20.0);
        let t2 = Transform::scale(2.0, 2.0);
        
        let composed = t1.compose(&t2);
        
        assert_eq!(composed.translation, [20.0, 40.0]);
        assert_eq!(composed.scale, [2.0, 2.0]);
    }
    
    #[tokio::test]
    async fn test_document_creation() {
        let db = create_test_db().await;
        let repo = DocumentRepo::new(db);
        
        let doc = repo.create("Test Doc", 800.0, 600.0).await.unwrap();
        
        assert_eq!(doc.name, "Test Doc");
        assert!(doc.id.is_some());
    }
}
```

### 7.3 Integration Tests

```rust
// tests/integration/api.rs
use reqwest::Client;

#[tokio::test]
async fn test_document_crud() {
    let app = spawn_test_app().await;
    let client = Client::new();
    let base_url = format!("http://{}", app.addr);
    
    // Create
    let resp = client
        .post(&format!("{}/api/documents", base_url))
        .json(&serde_json::json!({
            "name": "Test Doc",
            "width": 800,
            "height": 600
        }))
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 201);
    let doc: DocumentResponse = resp.json().await.unwrap();
    
    // Get
    let resp = client
        .get(&format!("{}/api/documents/{}", base_url, doc.id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    
    // Delete
    let resp = client
        .delete(&format!("{}/api/documents/{}", base_url, doc.id))
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 204);
}
```

---

## Summary: Production Checklist

### Infrastructure
- [ ] Database with connection pooling
- [ ] Redis for caching and sessions
- [ ] Object storage (S3) for files
- [ ] Message queue for async tasks
- [ ] Load balancer
- [ ] CDN for static assets

### Security
- [ ] HTTPS everywhere
- [ ] JWT authentication
- [ ] Role-based authorization
- [ ] Rate limiting
- [ ] Input validation
- [ ] SQL injection prevention
- [ ] XSS protection

### Observability
- [ ] Structured logging
- [ ] Metrics (Prometheus)
- [ ] Distributed tracing
- [ ] Health check endpoints
- [ ] Alerting (PagerDuty, etc.)

### Reliability
- [ ] Database migrations
- [ ] Backup strategy
- [ ] Disaster recovery plan
- [ ] Circuit breakers
- [ ] Retry with backoff
- [ ] Graceful degradation

### Deployment
- [ ] CI/CD pipeline
- [ ] Blue-green or canary deployments
- [ ] Rollback capability
- [ ] Infrastructure as code
- [ ] Container orchestration

### Testing
- [ ] Unit tests (>80% coverage)
- [ ] Integration tests
- [ ] E2E tests
- [ ] Load testing
- [ ] Chaos engineering
