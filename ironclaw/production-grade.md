---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/ironclaw
explored_at: 2026-03-22
---

# IronClaw Production-Grade Deployment Guide

This document covers production deployment considerations, operational patterns, and best practices for running IronClaw in production environments.

---

## 1. Production Architecture

### 1.1 Deployment Topologies

#### Single-Instance Deployment (Default)

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Single Instance                                │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    IronClaw Binary                          │   │
│  │                                                             │   │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌─────────┐ │   │
│  │  │   Agent   │  │ Channels  │  │   Tools   │  │ Gateway │ │   │
│  │  │   Loop    │  │           │  │           │  │         │ │   │
│  │  └───────────┘  └───────────┘  └───────────┘  └─────────┘ │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │                                       │
│                            ▼                                       │
│                   ┌─────────────────┐                             │
│                   │   PostgreSQL    │                             │
│                   │   + pgvector    │                             │
│                   └─────────────────┘                             │
└─────────────────────────────────────────────────────────────────────┘
```

**Best For:** Personal use, development, low-traffic scenarios

#### Multi-Instance with Load Balancer

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Load Balancer (nginx/HAProxy)                   │
│                            :443 / :80                               │
└─────────────────────────┬───────────────────────────────────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        │                 │                 │
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│  IronClaw #1  │ │  IronClaw #2  │ │  IronClaw #3  │
│  (Stateless)  │ │  (Stateless)  │ │  (Stateless)  │
└───────┬───────┘ └───────┬───────┘ └───────┬───────┘
        │                 │                 │
        └─────────────────┼─────────────────┘
                          │
                          ▼
                   ┌─────────────────┐
                   │   PostgreSQL    │
                   │   (Primary)     │
                   └────────┬────────┘
                            │
                            ▼
                   ┌─────────────────┐
                   │   PostgreSQL    │
                   │   (Replica)     │
                   └─────────────────┘
```

**Best For:** High availability, horizontal scaling

#### Docker Sandbox Enabled

```
┌─────────────────────────────────────────────────────────────────────┐
│                      IronClaw Host                                  │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   Orchestrator API                          │   │
│  │                   (localhost:9999)                          │   │
│  └─────────────────────────┬───────────────────────────────────┘   │
│                            │                                       │
│              ┌─────────────┴─────────────┐                        │
│              │                           │                        │
│              ▼                           ▼                        │
│  ┌─────────────────┐         ┌─────────────────┐                 │
│  │  Docker Daemon  │         │  Docker Daemon  │                 │
│  │                 │         │                 │                 │
│  │ ┌─────────────┐ │         │ ┌─────────────┐ │                 │
│  │ │  Worker #1  │ │         │ │  Worker #2  │ │                 │
│  │ │  Container  │ │   ...   │ │  Container  │ │                 │
│  │ └─────────────┘ │         │ └─────────────┘ │                 │
│  └─────────────────┘         └─────────────────┘                 │
└─────────────────────────────────────────────────────────────────────┘
```

**Best For:** Untrusted tool execution, code generation, shell access

---

## 2. Infrastructure Requirements

### 2.1 Minimum Requirements

| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| Memory | 2 GB | 4-8 GB |
| Storage | 10 GB SSD | 50+ GB SSD |
| Network | 10 Mbps | 100+ Mbps |

### 2.2 Database Requirements

**PostgreSQL:**
- Version: 15+ with pgvector extension
- Memory: 2GB+ shared buffers
- Connections: 20+ (configure pool size)
- Storage: SSD recommended for vector indexes

**libSQL (embedded):**
- Storage: Local SSD path
- No network requirements
- Suitable for single-instance only

### 2.3 Docker Requirements (for Sandbox)

- Docker 24+ with compose support
- 2GB+ additional memory for containers
- Network bridge configuration
- Volume mounts for workspace

---

## 3. Configuration Management

### 3.1 Environment-Based Configuration

```bash
# Production .env file
# Store in secure location, restrict permissions

# Database
DATABASE_BACKEND=postgres
DATABASE_URL=postgres://user:password@db.example.com:5432/ironclaw
DATABASE_POOL_SIZE=20

# NEAR AI
NEARAI_SESSION_TOKEN=sess_xxx
NEARAI_MODEL=claude-3-5-sonnet-20241022
NEARAI_BASE_URL=https://private.near.ai

# Security
GATEWAY_AUTH_TOKEN=<random-secure-token>
SANDBOX_ENABLED=true
SANDBOX_IMAGE=ironclaw-worker:latest

# Performance
MAX_PARALLEL_JOBS=10
HEARTBEAT_INTERVAL_SECS=1800

# Observability
RUST_LOG=ironclaw=info,tower_http=warn
TRACING_ENDPOINT=https://otel.example.com
```

### 3.2 Settings File (`~/.ironclaw/settings.toml`)

```toml
[database]
backend = "postgres"
url = "postgres://..."
pool_size = 20

[llm.nearai]
session_token = "sess_xxx"  # Encrypted in keychain
model = "claude-3-5-sonnet-20241022"
base_url = "https://private.near.ai"

[gateway]
enabled = true
host = "0.0.0.0"  # Bind to all interfaces for LB
port = 3001
auth_token = "xxx"  # Encrypted

[sandbox]
enabled = true
image = "ironclaw-worker:latest"
memory_limit_mb = 512
timeout_secs = 1800

[embeddings]
enabled = true
provider = "nearai"
model = "text-embedding-3-small"

[heartbeat]
enabled = true
interval_secs = 1800
notify_channel = "gateway"

[routines]
enabled = true
max_concurrent = 5
cron_interval_secs = 60
```

### 3.3 Configuration Validation

```bash
# Validate configuration before deployment
ironclaw config validate

# Check database connectivity
ironclaw config test-db

# Verify secrets access
ironclaw config test-secrets
```

---

## 4. Security Hardening

### 4.1 Network Security

**Firewall Rules:**
```bash
# Allow only necessary ports
ufw allow 443/tcp    # HTTPS (gateway)
ufw allow 22/tcp     # SSH (admin)
ufw deny 3001/tcp    # Block direct gateway access (use LB)

# Database (if remote)
ufw allow from <db_subnet> to any port 5432
```

**TLS Termination:**
```nginx
# nginx configuration
server {
    listen 443 ssl;
    server_name ironclaw.example.com;

    ssl_certificate /etc/ssl/certs/ironclaw.crt;
    ssl_certificate_key /etc/ssl/private/ironclaw.key;
    ssl_protocols TLSv1.3 TLSv1.2;

    location / {
        proxy_pass http://localhost:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### 4.2 Secrets Management

**Production Pattern:**
```bash
# Use system keychain/secret service
# macOS: Keychain
# Linux: GNOME Keyring / KWallet
# Windows: Credential Manager (TODO)

# Set secrets via CLI
ironclaw secret set nearei_session_token "sess_xxx"
ironclaw secret set gateway_auth_token "secure-random-token"

# Secrets are encrypted with AES-256-GCM
# Master key stored in platform keychain
```

**Database Encryption:**
```sql
-- Secrets table stores encrypted values
CREATE TABLE secrets (
    id UUID PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,  -- AES-256-GCM
    key_salt TEXT NOT NULL,          -- For key derivation
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### 4.3 Docker Sandbox Security

**Container Isolation:**
```yaml
# docker-compose.yml for worker
version: '3.8'
services:
  worker:
    image: ironclaw-worker:latest
    user: "1000:1000"  # Non-root
    read_only: true    # Read-only filesystem
    tmpfs:
      - /tmp
      - /home/worker/.cache
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    networks:
      - ironclaw_internal
    # No port exposure (orchestrator only)
```

**Network Policies:**
```rust
// WASM tool network allowlist
{
  "http": {
    "allowlist": [
      { "host": "api.openai.com", "path_prefix": "/v1/" },
      { "host": "slack.com", "path_prefix": "/api/" }
    ],
    "deny_private_ranges": true,  // Block 10.x, 192.168.x, etc.
    "deny_localhost": true
  }
}
```

### 4.4 Audit Logging

```rust
// Enable audit logging for sensitive operations
tracing_subscriber::fmt()
    .with_env_filter("ironclaw=info,ironclaw::secrets=debug,ironclaw::tools=debug")
    .with_target(true)
    .with_thread_ids(true)
    .init();
```

**Log Aggregation:**
```yaml
# OpenTelemetry Collector config
receivers:
  otlp:
    protocols:
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 1s
    send_batch_size: 1024

exporters:
  logging:
    loglevel: debug
  otlp/jaeger:
    endpoint: jaeger:4317
    tls:
      insecure: true

service:
  pipelines:
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: [logging, otlp/jaeger]
```

---

## 5. High Availability

### 5.1 Database High Availability

**PostgreSQL Streaming Replication:**
```yaml
# Primary configuration
wal_level = replica
max_wal_senders = 3
wal_keep_size = 64MB

# Synchronous commit (for critical data)
synchronous_commit = on
synchronous_standby_names = 'ironclaw_replica'
```

**Failover Strategy:**
```bash
# Use Patroni or similar for automatic failover
patronictl -c /etc/patroni/patroni.yml failover
```

### 5.2 Application-Level HA

**Health Check Endpoint:**
```rust
// GET /api/health
pub async fn health_check(
    db: Arc<dyn Database>,
    llm: Arc<dyn LlmProvider>,
) -> Result<HealthStatus> {
    let db_status = db.health_check().await?;
    let llm_status = llm.health_check().await?;

    Ok(HealthStatus {
        status: if db_status && llm_status { "healthy" } else { "degraded" },
        database: db_status,
        llm_provider: llm_status,
        timestamp: Utc::now(),
    })
}
```

**Load Balancer Health Checks:**
```nginx
upstream ironclaw_backend {
    server ironclaw-1:3001;
    server ironclaw-2:3001;
    server ironclaw-3:3001;

    health_check interval=10s fails=3 passes=2;
}
```

### 5.3 Session Affinity

For stateful operations, use session affinity:

```nginx
upstream ironclaw_backend {
    ip_hash;  # Simple session affinity
    server ironclaw-1:3001;
    server ironclaw-2:3001;
}
```

---

## 6. Monitoring & Observability

### 6.1 Metrics to Track

| Metric | Type | Alert Threshold |
|--------|------|-----------------|
| `ironclaw_jobs_total` | Counter | - |
| `ironclaw_jobs_duration_seconds` | Histogram | p99 > 60s |
| `ironclaw_tool_calls_total` | Counter | - |
| `ironclaw_tool_errors_total` | Counter | Error rate > 5% |
| `ironclaw_llm_calls_total` | Counter | - |
| `ironclaw_llm_tokens_total` | Counter | Cost threshold |
| `ironclaw_database_connections` | Gauge | > 80% pool |
| `ironclaw_database_query_duration` | Histogram | p99 > 1s |
| `ironclaw_websocket_connections` | Gauge | - |
| `ironclaw_sandbox_containers_active` | Gauge | > limit |

### 6.2 Prometheus Exporter

```rust
use prometheus::{Registry, Counter, Histogram};

pub struct Metrics {
    registry: Registry,
    job_counter: Counter,
    job_duration: Histogram,
    tool_errors: Counter,
}

impl Metrics {
    pub fn record_job_completed(&self, duration: f64) {
        self.job_counter.inc();
        self.job_duration.observe(duration);
    }

    pub fn record_tool_error(&self) {
        self.tool_errors.inc();
    }
}
```

### 6.3 Distributed Tracing

```rust
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry_otlp::new_exporter;

fn setup_tracing() {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(new_exporter().tonic())
        .install_batch(opentelemetry::runtime::Tokio)
        .unwrap();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();
}
```

**Trace Propagation:**
```
Client Request → Gateway → Agent → Worker → LLM Provider
     │              │         │         │         │
     └──────────────┴─────────┴─────────┴─────────┘
                    Trace Context (W3C Traceparent)
```

### 6.4 Alerting Rules (Prometheus)

```yaml
groups:
  - name: ironclaw
    rules:
      - alert: HighJobErrorRate
        expr: rate(ironclaw_job_errors_total[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High job error rate"

      - alert: DatabaseConnectionPoolExhausted
        expr: ironclaw_database_connections / ironclaw_database_pool_size > 0.8
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Database pool nearly exhausted"

      - alert: LLMAPIUnavailable
        expr: up{job="llm-provider"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "LLM provider unavailable"

      - alert: HighSandboxContainerCount
        expr: ironclaw_sandbox_containers_active > 50
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Many sandbox containers active"
```

---

## 7. Backup & Recovery

### 7.1 Database Backup

**PostgreSQL:**
```bash
#!/bin/bash
# Daily backup script

BACKUP_DIR="/backups/ironclaw"
DATE=$(date +%Y-%m-%d_%H-%M-%S)

# Full backup with pg_dump
pg_dump -h localhost -U ironclaw \
    --format=custom \
    --verbose \
    ironclaw > "${BACKUP_DIR}/ironclaw_${DATE}.dump"

# Encrypt backup
age -r "age1xxx" "${BACKUP_DIR}/ironclaw_${DATE}.dump" \
    > "${BACKUP_DIR}/ironclaw_${DATE}.dump.age"

# Upload to S3
aws s3 cp "${BACKUP_DIR}/ironclaw_${DATE}.dump.age" \
    s3://my-backup-bucket/ironclaw/

# Retention: keep 30 days
find "${BACKUP_DIR}" -name "*.age" -mtime +30 -delete
```

**libSQL:**
```bash
# Copy the SQLite file
cp ~/.ironclaw/ironclaw.db /backups/ironclaw_$(date +%Y-%m-%d).db

# For Turso Cloud, use their backup API
turso db backup create ironclaw
```

### 7.2 Secrets Backup

**Critical:** Backup encrypted secrets separately from decryption keys.

```bash
# Export encrypted secrets
psql -c "COPY secrets TO STDOUT" > /backups/secrets_$(date +%Y-%m-%d).sql

# Backup keychain separately (platform-specific)
# macOS: Use Time Machine for Keychain
# Linux: Backup ~/.local/keyrings/
```

### 7.3 Recovery Procedure

```bash
# 1. Restore database
pg_restore -h localhost -U ironclaw -d ironclaw \
    /backups/ironclaw_2024-01-15.dump

# 2. Restore secrets (if needed)
psql -h localhost -U ironclaw -d ironclaw \
    < /backups/secrets_2024-01-15.sql

# 3. Verify data integrity
ironclaw config validate

# 4. Restart application
systemctl restart ironclaw
```

---

## 8. Scaling Strategies

### 8.1 Vertical Scaling

Increase resources for single instance:
- More CPU cores for parallel job execution
- More RAM for caching and embeddings
- Faster SSD for database I/O

### 8.2 Horizontal Scaling

**Stateless Application Tier:**
```yaml
# Kubernetes deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ironclaw
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: ironclaw
        image: ironclaw:latest
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ironclaw-secrets
              key: database-url
        resources:
          requests:
            cpu: "1"
            memory: "2Gi"
          limits:
            cpu: "2"
            memory: "4Gi"
```

**Database Scaling:**
- Read replicas for search queries
- Connection pooling (PgBouncer)
- Partitioning for large tables (jobs, actions)

### 8.3 Docker Sandbox Scaling

```yaml
# Limit concurrent containers
[sandbox]
max_containers = 10
container_memory_mb = 512
container_timeout_secs = 1800

# Queue overflow handling
[job_queue]
max_queue_size = 100
overflow_action = "reject"  # or "block"
```

---

## 9. Performance Tuning

### 9.1 Database Tuning

**PostgreSQL Configuration:**
```conf
# postgresql.conf tuning
shared_buffers = 2GB              # 25% of RAM
effective_cache_size = 6GB        # 75% of RAM
work_mem = 64MB                   # Per-operation
maintenance_work_mem = 512MB

# Connection pooling
max_connections = 100
superuser_reserved_connections = 3

# WAL settings
wal_buffers = 64MB
checkpoint_completion_target = 0.9

# Vector index tuning (pgvector)
vector.max_probes = 100
```

**Index Strategy:**
```sql
-- Workspace search indexes
CREATE INDEX idx_memory_chunks_embedding
    ON memory_chunks USING ivfflat (embedding vector_cosine_ops)
    WITH (lists = 100);

CREATE INDEX idx_memory_chunks_fts
    ON memory_chunks USING GIN (fts);

-- Job queries
CREATE INDEX idx_agent_jobs_state_created
    ON agent_jobs (state, created_at DESC);

-- Conversation lookups
CREATE INDEX idx_conversations_user_sender
    ON conversations (user_id, sender_id);
```

### 9.2 Application Tuning

**Connection Pool:**
```rust
let pool = Pool::builder()
    .max_size(20)           // Adjust based on load
    .min_idle(Some(5))      // Keep warm connections
    .acquire_timeout(Duration::from_secs(30))
    .build(client)?;
```

**Job Scheduler:**
```rust
let scheduler = Scheduler::new(
    max_parallel_jobs: 10,
    priority_boost_threshold: Duration::from_secs(300),  // 5 min
);
```

### 9.3 Caching Strategy

**In-Memory Caching:**
```rust
use moka::future::Cache;

// Tool schema cache
let schema_cache = Cache::builder()
    .max_capacity(1000)
    .time_to_live(Duration::from_secs(3600))
    .build();

// Session cache
let session_cache = Cache::builder()
    .max_capacity(100)
    .time_to_idle(Duration::from_secs(1800))
    .build();
```

---

## 10. Deployment Automation

### 10.1 Docker Deployment

```dockerfile
# Dockerfile for production
FROM rust:1.85 AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

COPY . .
RUN cargo build --release

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ironclaw /usr/local/bin/

RUN useradd -m -u 1000 ironclaw
USER ironclaw

WORKDIR /home/ironclaw
ENV RUST_LOG=ironclaw=info,tower_http=warn

ENTRYPOINT ["ironclaw"]
CMD ["run"]
```

### 10.2 Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ironclaw
  labels:
    app: ironclaw
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ironclaw
  template:
    metadata:
      labels:
        app: ironclaw
    spec:
      containers:
      - name: ironclaw
        image: ironclaw:0.1.3
        ports:
        - containerPort: 3001
          name: gateway
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: ironclaw-secrets
              key: database-url
        - name: GATEWAY_AUTH_TOKEN
          valueFrom:
            secretKeyRef:
              name: ironclaw-secrets
              key: gateway-token
        livenessProbe:
          httpGet:
            path: /api/health
            port: 3001
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /api/health
            port: 3001
          initialDelaySeconds: 5
          periodSeconds: 10
        resources:
          requests:
            cpu: "500m"
            memory: "1Gi"
          limits:
            cpu: "2"
            memory: "4Gi"
        volumeMounts:
        - name: workspace
          mountPath: /home/ironclaw/.ironclaw/workspace
        - name: secrets
          mountPath: /home/ironclaw/.ironclaw/secrets
          readOnly: true
      volumes:
      - name: workspace
        persistentVolumeClaim:
          claimName: ironclaw-workspace
      - name: secrets
        secret:
          secretName: ironclaw-secrets
---
apiVersion: v1
kind: Service
metadata:
  name: ironclaw
spec:
  selector:
    app: ironclaw
  ports:
  - port: 80
    targetPort: 3001
  type: ClusterIP
```

### 10.3 Systemd Service

```ini
# /etc/systemd/system/ironclaw.service
[Unit]
Description=IronClaw AI Assistant
After=network.target postgresql.service
Wants=postgresql.service

[Service]
Type=notify
User=ironclaw
Group=ironclaw
WorkingDirectory=/home/ironclaw
EnvironmentFile=/etc/ironclaw/.env

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true
ReadWritePaths=/home/ironclaw/.ironclaw

# Resource limits
MemoryMax=4G
CPUQuota=200%

# Restart policy
Restart=on-failure
RestartSec=10
StartLimitBurst=5
StartLimitIntervalSec=60

ExecStart=/usr/local/bin/ironclaw run
ExecReload=/bin/kill -HUP $MAINPID

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=ironclaw

[Install]
WantedBy=multi-user.target
```

---

## 11. Disaster Recovery

### 11.1 RTO/RPO Targets

| Scenario | RTO (Recovery Time) | RPO (Recovery Point) |
|----------|---------------------|----------------------|
| Application crash | < 1 minute | 0 (no data loss) |
| Database failure | < 5 minutes | < 1 hour (WAL) |
| Region failure | < 1 hour | < 1 day (backup) |

### 11.2 Runbook: Database Failure

```bash
# 1. Detect failure
ironclaw config test-db  # Returns error

# 2. Check database status
systemctl status postgresql
journalctl -u postgresql -n 50

# 3. Attempt restart
systemctl restart postgresql

# 4. If restart fails, check logs
tail -f /var/log/postgresql/postgresql-15-main.log

# 5. Failover to replica (if configured)
patronictl -c /etc/patroni/patroni.yml failover

# 6. Update connection string
ironclaw config set database.url "postgres://...@replica:5432/ironclaw"

# 7. Restart application
systemctl restart ironclaw

# 8. Verify functionality
ironclaw status
```

### 11.3 Runbook: Data Corruption

```bash
# 1. Stop application to prevent further damage
systemctl stop ironclaw

# 2. Identify corruption scope
psql -c "SELECT COUNT(*) FROM agent_jobs WHERE created_at > '2024-01-15';"

# 3. Locate most recent clean backup
aws s3 ls s3://my-backup-bucket/ironclaw/ | tail -10

# 4. Download and decrypt backup
aws s3 cp s3://my-backup-bucket/ironclaw/ironclaw_2024-01-14.dump.age ./
age -d -o ironclaw.dump ironclaw_2024-01-14.dump.age

# 5. Restore database
pg_restore -h localhost -U ironclaw -d ironclaw ironclaw.dump

# 6. Run integrity checks
ironclaw config validate

# 7. Restart application
systemctl start ironclaw

# 8. Verify data
ironclaw memory search "test"
```

---

## 12. Cost Optimization

### 12.1 LLM Cost Tracking

```sql
-- Track LLM spending
SELECT
    DATE_TRUNC('day', created_at) AS day,
    SUM(estimated_cost) AS total_cost,
    COUNT(*) AS total_calls
FROM llm_calls
GROUP BY DATE_TRUNC('day', created_at)
ORDER BY day DESC
LIMIT 30;
```

### 12.2 Cost Alerts

```rust
pub async fn check_daily_budget(
    db: Arc<dyn Database>,
    daily_budget: Decimal,
) -> Result<bool> {
    let spent = db.get_today_llm_cost().await?;
    Ok(spent < daily_budget)
}
```

### 12.3 Optimization Strategies

1. **Context Compaction:** Reduce token usage with summarization
2. **Embedding Caching:** Cache embeddings for repeated queries
3. **Job Batching:** Combine related tasks into single LLM calls
4. **Model Selection:** Use cheaper models for simple queries

---

## 13. Compliance Considerations

### 13.1 Data Residency

- Configure database region based on user location
- Use region-specific LLM endpoints when available
- Document data flow in architecture diagrams

### 13.2 Audit Trail

```sql
-- Enable audit logging for sensitive tables
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    table_name TEXT NOT NULL,
    action TEXT NOT NULL,  -- INSERT, UPDATE, DELETE
    old_value JSONB,
    new_value JSONB,
    user_id TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_audit_log_table_created
    ON audit_log (table_name, created_at);
```

### 13.3 Data Retention

```sql
-- Automatic job cleanup after 90 days
CREATE OR REPLACE FUNCTION cleanup_old_jobs()
RETURNS void AS $$
BEGIN
    DELETE FROM agent_jobs
    WHERE created_at < NOW() - INTERVAL '90 days'
    AND state IN ('completed', 'failed');
END;
$$ LANGUAGE plpgsql;

-- Schedule with pg_cron
SELECT cron.schedule(
    'cleanup-old-jobs',
    '0 3 * * *',  -- Daily at 3 AM
    $$SELECT cleanup_old_jobs()$$
);
```

---

## 14. Production Checklist

### Pre-Deployment

- [ ] Database configured with backups
- [ ] Secrets stored in keychain/secret service
- [ ] TLS certificates installed
- [ ] Firewall rules configured
- [ ] Monitoring/alerting setup
- [ ] Log aggregation configured
- [ ] Health checks verified
- [ ] Backup/restore tested

### Post-Deployment

- [ ] Health endpoint responds
- [ ] Gateway accessible via HTTPS
- [ ] Database queries working
- [ ] Tool execution working
- [ ] Memory search functional
- [ ] Routines executing on schedule
- [ ] Alerts firing correctly
- [ ] Logs flowing to aggregator

### Ongoing Maintenance

- [ ] Weekly: Review error logs
- [ ] Weekly: Check backup integrity
- [ ] Monthly: Review cost reports
- [ ] Monthly: Test disaster recovery
- [ ] Quarterly: Security updates
- [ ] Quarterly: Performance review

---

## Related Documents

- [`exploration.md`](./exploration.md) - Main exploration overview
- [`architecture-deep-dive.md`](./architecture-deep-dive.md) - Architecture analysis
- [`rust-revision.md`](./rust-revision.md) - Rust implementation details
