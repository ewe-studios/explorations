# ZeroClaw Production-Grade Guide

**Document Type:** Production Deployment Reference
**Last Updated:** 2026-03-22
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/zeroclaw`

---

## Table of Contents

1. [Production Readiness Checklist](#production-readiness-checklist)
2. [Deployment Architecture](#deployment-architecture)
3. [Security Hardening](#security-hardening)
4. [High Availability](#high-availability)
5. [Monitoring & Observability](#monitoring--observability)
6. [Performance Tuning](#performance-tuning)
7. [Backup & Recovery](#backup--recovery)
8. [Scaling Strategies](#scaling-strategies)
9. [Incident Response](#incident-response)
10. [Upgrade Procedures](#upgrade-procedures)

---

## Production Readiness Checklist

### Pre-Deployment

- [ ] Security audit completed (`cargo audit`, CodeQL)
- [ ] Rate limits configured (actions/hour, cost/day)
- [ ] Gateway pairing enabled
- [ ] Channel allowlists configured (deny-by-default)
- [ ] Workspace scoping enabled (`workspace_only = true`)
- [ ] Forbidden paths verified
- [ ] Secret encryption enabled (`secrets.encrypt = true`)
- [ ] TLS configured for gateway (via tunnel or reverse proxy)
- [ ] Health checks configured
- [ ] Monitoring backend connected (Prometheus/OTel)
- [ ] Log aggregation configured
- [ ] Backup strategy defined
- [ ] Rollback procedure documented
- [ ] On-call rotation defined

### Deployment

- [ ] Running as system service (systemd/launchd)
- [ ] Resource limits set (memory, CPU)
- [ ] Network policies applied
- [ ] Secrets injected via secure mechanism (not env vars)
- [ ] Config validated (`zeroclaw doctor`)
- [ ] Channel health verified (`zeroclaw channel doctor`)

### Post-Deployment

- [ ] All health checks passing
- [ ] Metrics flowing to dashboard
- [ ] Logs visible in aggregator
- [ ] First backup completed successfully
- [ ] Incident runbook tested
- [ ] Alert thresholds configured

---

## Deployment Architecture

### Single-Node Deployment

```
┌─────────────────────────────────────────────────────────────┐
│                     Production Host                          │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                  ZeroClaw Daemon                       │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐               │  │
│  │  │ Gateway │  │ Agent   │  │ Channels│               │  │
│  │  │ :3000   │  │ Loop    │  │ (TG, DS)│               │  │
│  │  └────┬────┘  └────┬────┘  └────┬────┘               │  │
│  │       │            │            │                     │  │
│  │       └────────────┴────────────┘                     │  │
│  │                      │                                │  │
│  │  ┌───────────────────┴───────────────────────────┐   │  │
│  │  │              SQLite Memory                     │   │  │
│  │  │  - Vector embeddings (BLOB)                   │   │  │
│  │  │  - FTS5 keyword search                        │   │  │
│  │  │  - Response cache                             │   │  │
│  │  └───────────────────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────────────┘  │
│                            │                                │
│  ┌─────────────────────────┴───────────────────────────┐   │
│  │              Reverse Proxy (nginx/Caddy)             │   │
│  │  - TLS termination                                  │   │
│  │  - Rate limiting                                    │   │
│  │  - Access logging                                   │   │
│  └─────────────────────────┬───────────────────────────┘   │
└────────────────────────────┼───────────────────────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        Telegram       Discord        Webhook
        Users          Users          Integrations
```

### Multi-Node Deployment

```
┌──────────────────────────────────────────────────────────────────┐
│                         Load Balancer                             │
│                    (nginx/HAProxy/Cloud)                          │
│  - TLS termination                                               │
│  - Health checks                                                 │
│  - Session affinity (sticky cookies)                             │
└────────────────────────────┬─────────────────────────────────────┘
                             │
        ┌────────────────────┼────────────────────┐
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│  ZeroClaw #1  │   │  ZeroClaw #2  │   │  ZeroClaw #N  │
│  :3000        │   │  :3000        │   │  :3000        │
│               │   │               │   │               │
│ ┌───────────┐ │   │ ┌───────────┐ │   │ ┌───────────┐ │
│ │  Memory   │ │   │ │  Memory   │ │   │ │  Memory   │ │
│ │ (local)   │ │   │ │ (local)   │ │   │ │ (local)   │ │
│ └───────────┘ │   │ └───────────┘ │   │ └───────────┘ │
└───────┬───────┘   └───────┬───────┘   └───────┬───────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
        ┌───────────────────┴───────────────────┐
        ▼                                       ▼
┌───────────────────┐                 ┌───────────────────┐
│  PostgreSQL       │                 │  Prometheus       │
│  (shared memory)  │                 │  (metrics)        │
└───────────────────┘                 └───────────────────┘
```

### Docker Deployment

```yaml
# docker-compose.yml
version: '3.8'

services:
  zeroclaw:
    image: zeroclaw/zeroclaw:latest
    container_name: zeroclaw
    restart: unless-stopped
    ports:
      - "127.0.0.1:3000:3000"
    volumes:
      - zeroclaw-config:/home/zeroclaw/.zeroclaw
      - zeroclaw-data:/home/zeroclaw/.zeroclaw/workspace
      - /etc/localtime:/etc/localtime:ro
    environment:
      - ZEROCLAW_API_KEY=${ZEROCLAW_API_KEY}
      - RUST_LOG=info
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    read_only: true
    tmpfs:
      - /tmp
    healthcheck:
      test: ["CMD", "zeroclaw", "doctor"]
      interval: 30s
      timeout: 10s
      retries: 3

  postgres:
    image: postgres:16-alpine
    container_name: zeroclaw-postgres
    restart: unless-stopped
    volumes:
      - postgres-data:/var/lib/postgresql/data
    environment:
      - POSTGRES_USER=zeroclaw
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
      - POSTGRES_DB=zeroclaw

volumes:
  zeroclaw-config:
  zeroclaw-data:
  postgres-data:
```

### systemd Service

```ini
# /etc/systemd/system/zeroclaw.service
[Unit]
Description=ZeroClaw AI Assistant Daemon
Documentation=https://github.com/zeroclaw-labs/zeroclaw
After=network.target postgresql.service

[Service]
Type=notify
User=zeroclaw
Group=zeroclaw

# Binary
ExecStart=/usr/local/bin/zeroclaw daemon --host 127.0.0.1 --port 3000

# Restart policy
Restart=always
RestartSec=5s

# Resource limits
MemoryLimit=512M
CPUQuota=100%

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
PrivateTmp=true
ReadWritePaths=/home/zeroclaw/.zeroclaw

# Environment
Environment="RUST_LOG=info"
EnvironmentFile=-/etc/zeroclaw/env

[Install]
WantedBy=multi-user.target
```

---

## Security Hardening

### Gateway Security

**Default (Secure):**
```toml
[gateway]
port = 3000
host = "127.0.0.1"        # Localhost only
require_pairing = true    # 6-digit code required
allow_public_bind = false # Refuse 0.0.0.0
```

**Production with Tunnel:**
```toml
[gateway]
port = 3000
host = "127.0.0.1"
require_pairing = true
allow_public_bind = false

[tunnel]
provider = "cloudflare"   # Or tailscale, ngrok
```

### Channel Security

**Deny-by-Default:**
```toml
[channels_config.telegram]
bot_token = "..."
allowed_users = []  # Empty = deny all

[channels_config.discord]
bot_token = "..."
allowed_users = []  # Empty = deny all
```

**Add Users via CLI:**
```bash
# Bind Telegram identity
zeroclaw channel bind-telegram @username

# Or by numeric ID
zeroclaw channel bind-telegram 123456789
```

### Autonomy Policy

**Production Defaults:**
```toml
[autonomy]
level = "supervised"         # Require approval for risky ops
workspace_only = true        # Scoped to workspace
allowed_commands = [         # Explicit allowlist
    "git", "npm", "cargo",
    "ls", "cat", "grep",
    "find", "wc", "head", "tail"
]
forbidden_paths = [          # Blocked paths
    "/etc", "/root", "/home",
    "/usr", "/bin", "/sbin",
    "~/.ssh", "~/.gnupg", "~/.aws"
]
max_actions_per_hour = 20    # Rate limit
max_cost_per_day_cents = 500 # $5/day limit
require_approval_for_medium_risk = true
block_high_risk_commands = true
```

### Secret Management

**Encrypted Storage:**
```toml
[secrets]
encrypt = true  # ChaCha20-Poly1305
```

**Secret Files:**
```
~/.zeroclaw/
├── .secret_key          # Encryption key (600 permissions)
├── config.toml          # Main config (no secrets)
├── auth-profiles.json   # Encrypted auth profiles
└── secrets.db           # Encrypted secret store
```

**Production Secret Injection:**
```bash
# Via environment (not recommended for secrets)
export ZEROCLAW_API_KEY="sk-..."

# Via file. File (recommended)
echo "sk-..." > /etc/zeroclaw/api_key
chmod 600 /etc/zeroclaw/api_key

# In config.toml
api_key = "file:/etc/zeroclaw/api_key"

# Via secrets manager (HashiCorp Vault, AWS Secrets Manager)
api_key = "vault://secret/zeroclaw#api_key"
```

### Sandbox Execution

**Docker Sandbox:**
```toml
[runtime]
kind = "docker"

[runtime.docker]
image = "alpine:3.20"
network = "none"          # No network access
memory_limit_mb = 512
cpu_limit = 1.0
read_only_rootfs = true
mount_workspace = true
```

**Landlock (Linux):**
```toml
[runtime]
kind = "native"

# Enable Landlock LSM
[features]
sandbox-landlock = true
```

---

## High Availability

### Health Checks

**Endpoint:** `GET /health`

```bash
curl http://127.0.0.1:3000/health
# Response: {"status": "healthy"}
```

**CLI Health:**
```bash
zeroclaw doctor
# Checks:
# - Config loaded
# - Memory backend accessible
# - Channels healthy
# - Gateway responsive
```

**Channel Health:**
```bash
zeroclaw channel doctor
# Checks each configured channel
```

### Heartbeat System

**Configuration:**
```toml
[heartbeat]
enabled = true
interval_minutes = 30
tasks = [
    "zeroclaw status > /var/log/zeroclaw/heartbeat.log",
    "zeroclaw memory recall 'last_backup' 1"
]
```

**Custom Heartbeat Tasks:**
```toml
[heartbeat.custom]
enabled = true
interval_minutes = 60
command = "curl -f http://monitoring/health || systemctl restart zeroclaw"
```

### Failure Detection

**Prometheus Alerts:**
```yaml
# alerts.yml
groups:
  - name: zeroclaw
    rules:
      - alert: ZeroClawDown
        expr: up{job="zeroclaw"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "ZeroClaw instance is down"

      - alert: ZeroClawHighLatency
        expr: histogram_quantile(0.99, rate(zeroclaw_request_duration_seconds_bucket[5m])) > 5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "ZeroClaw P99 latency > 5s"

      - alert: ZeroClawMemoryFull
        expr: zeroclaw_memory_entries > 10000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "ZeroClaw memory approaching capacity"
```

### Failover Strategy

**Active-Passive:**
```
┌─────────────┐     ┌─────────────┐
│   Primary   │────▶│  Standby    │
│  ZeroClaw   │     │  ZeroClaw   │
│  :3000      │     │  (hot)      │
└─────────────┘     └─────────────┘
       │                   │
       └─────────┬─────────┘
                 ▼
        ┌─────────────────┐
        │  Shared Postgres│
        │  (memory store) │
        └─────────────────┘
```

**Failover Trigger:**
```bash
# Manual failover
zeroclaw service stop  # Primary
zeroclaw service start # Standby

# Automatic (via keepalived)
vrrp_script check_zeroclaw {
    script "/usr/local/bin/check_zeroclaw.sh"
    interval 5
}
```

---

## Monitoring & Observability

### Prometheus Metrics

**Configuration:**
```toml
[observability]
backend = "prometheus"
port = 9090
path = "/metrics"
```

**Key Metrics:**

| Metric | Type | Description |
|--------|------|-------------|
| `zeroclaw_requests_total` | Counter | Total LLM requests |
| `zeroclaw_request_duration_seconds` | Histogram | Request latency |
| `zeroclaw_tokens_used_total` | Counter | Total tokens consumed |
| `zeroclaw_cost_usd_total` | Counter | Total cost in USD |
| `zeroclaw_memory_entries` | Gauge | Memory entry count |
| `zeroclaw_active_sessions` | Gauge | Active channel sessions |
| `zeroclaw_tool_calls_total` | Counter | Tool invocations |
| `zeroclaw_tool_call_duration_seconds` | Histogram | Tool execution time |
| `zeroclaw_channel_messages_total` | Counter | Messages per channel |
| `zeroclaw_errors_total` | Counter | Error count |

**Example Query:**
```promql
# P99 latency over 5 minutes
histogram_quantile(0.99, rate(zeroclaw_request_duration_seconds_bucket[5m]))

# Error rate
rate(zeroclaw_errors_total[5m])

# Cost per day
increase(zeroclaw_cost_usd_total[24h])
```

### OpenTelemetry

**Configuration:**
```toml
[observability]
backend = "opentelemetry"

[observability.opentelemetry]
endpoint = "http://otel-collector:4317"
service_name = "zeroclaw"
traces_enabled = true
metrics_enabled = true
```

**Exported Spans:**
- `agent.turn` - Full conversation turn
- `provider.chat` - LLM API call
- `tool.execute` - Tool execution
- `memory.recall` - Memory retrieval
- `channel.send` - Message send

### Logging

**Configuration:**
```toml
[observability]
backend = "log"

[observability.log]
level = "info"  # trace, debug, info, warn, error
format = "json" # or "pretty"
```

**Environment Override:**
```bash
export RUST_LOG=zeroclaw=debug,hyper=info
```

**Log Aggregation:**
```yaml
# Fluentd config
<match zeroclaw.**>
  @type elasticsearch
  host elasticsearch
  port 9200
  index_name zeroclaw-logs
</match>
```

### Grafana Dashboard

**Key Panels:**

1. **Request Rate & Latency**
   - Requests per second
   - P50, P95, P99 latency

2. **Token Usage & Cost**
   - Tokens per hour
   - Cost per day (stacked by provider)

3. **Memory Statistics**
   - Entry count
   - Recall rate
   - Cache hit ratio

4. **Channel Activity**
   - Messages per channel
   - Response time

5. **Tool Execution**
   - Calls per tool
   - Success/failure rate
   - Execution time

6. **Error Tracking**
   - Errors by component
   - Error rate over time

---

## Performance Tuning

### Build Optimization

**Release Profile (Default):**
```toml
[profile.release]
opt-level = "z"       # Optimize for size
lto = "thin"          # Link-time optimization
codegen-units = 1     # Single unit (low RAM)
strip = true          # Remove debug symbols
panic = "abort"       # Smaller binary
```

**Fast Build Profile (Powerful Machines):**
```toml
[profile.release-fast]
inherits = "release"
codegen-units = 8     # Parallel codegen
```

**Build Commands:**
```bash
# Standard release (size-optimized)
cargo build --release

# Faster build (16GB+ RAM)
cargo build --profile release-fast

# Maximum optimization (distribution)
cargo build --profile dist
```

### Memory Optimization

**SQLite Configuration:**
```toml
[memory]
backend = "sqlite"
auto_save = true
embedding_provider = "none"  # Use noop for lower memory
vector_weight = 0.7
keyword_weight = 0.3
max_entries = 10000          # Limit memory size
```

**SQLite Tuning:**
```sql
-- In ~/.zeroclaw/workspace/memory.db
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;  -- 64MB cache
PRAGMA temp_store = MEMORY;
```

### Concurrency Tuning

**Tokio Runtime:**
```rust
// In main.rs
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    // ...
}
```

**Channel Buffer Sizes:**
```rust
// Increase buffer for high-throughput channels
let (tx, rx) = tokio::sync::mpsc::channel(100);  // Default: 32
```

### Caching

**Response Cache:**
```toml
[memory]
response_cache_enabled = true
response_cache_ttl_hours = 24
response_cache_max_entries = 1000
```

**Embedding Cache:**
```toml
[memory]
embedding_cache_enabled = true
embedding_cache_max_entries = 5000
```

---

## Backup & Recovery

### Backup Strategy

**Files to Backup:**
```
~/.zeroclaw/
├── config.toml           # Configuration
├── .secret_key           # Encryption key
├── auth-profiles.json    # Auth profiles
├── secrets.db            # Secret store
└── workspace/
    ├── memory.db         # SQLite memory
    ├── memory.db-shm     # WAL shared memory
    └── memory.db-wal     # WAL log
```

**Backup Script:**
```bash
#!/bin/bash
# /usr/local/bin/zeroclaw-backup.sh

BACKUP_DIR="/var/backups/zeroclaw"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/zeroclaw-${TIMESTAMP}.tar.gz"

# Stop daemon
systemctl stop zeroclaw

# Create backup
tar -czf "$BACKUP_FILE" ~/.zeroclaw

# Restart daemon
systemctl start zeroclaw

# Upload to remote storage
aws s3 cp "$BACKUP_FILE" s3://backups/zeroclaw/

# Prune old backups (keep 7 days)
find "$BACKUP_DIR" -name "zeroclaw-*.tar.gz" -mtime +7 -delete

echo "Backup completed: $BACKUP_FILE"
```

**Cron Schedule:**
```cron
# Daily backup at 2 AM
0 2 * * * /usr/local/bin/zeroclaw-backup.sh
```

### Recovery Procedure

**Full Recovery:**
```bash
# Stop daemon
systemctl stop zeroclaw

# Download backup
aws s3 cp s3://backups/zeroclaw/zeroclaw-20260322-020000.tar.gz /tmp/

# Restore
tar -xzf /tmp/zeroclaw-20260322-020000.tar.gz -C ~/

# Set permissions
chmod 600 ~/.zeroclaw/.secret_key
chown -R zeroclaw:zeroclaw ~/.zeroclaw

# Start daemon
systemctl start zeroclaw

# Verify
zeroclaw doctor
```

### Migration Backup

**Before Upgrades:**
```bash
# Create pre-upgrade backup
zeroclaw-backup.sh pre-upgrade-$(zeroclaw --version)

# Export memory to JSON
zeroclaw memory export > /tmp/memory-export.json

# Document config
cp ~/.zeroclaw/config.toml /tmp/config-pre-upgrade.toml
```

---

## Scaling Strategies

### Vertical Scaling

**Increase Resources:**
```ini
# systemd override
# /etc/systemd/system/zeroclaw.service.d/override.conf
[Service]
MemoryLimit=2G
CPUQuota=200%
```

### Horizontal Scaling

**Multiple Instances:**
```
                    Load Balancer
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   ZeroClaw #1     ZeroClaw #2     ZeroClaw #3
   (stateless)     (stateless)     (stateless)
        │                │                │
        └────────────────┴────────────────┘
                         │
        ┌────────────────┴────────────────┐
        ▼                                 ▼
   PostgreSQL                        Prometheus
   (shared state)                   (centralized)
```

**Stateless Configuration:**
```toml
[memory]
backend = "postgres"  # Shared memory store

[storage.provider.config]
provider = "postgres"
db_url = "postgres://user:pass@db-host:5432/zeroclaw"
```

### Load Balancing

**nginx Configuration:**
```nginx
upstream zeroclaw {
    least_conn;
    server zeroclaw1:3000;
    server zeroclaw2:3000;
    server zeroclaw3:3000;
}

server {
    listen 443 ssl;
    server_name zeroclaw.example.com;

    location / {
        proxy_pass http://zeroclaw;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        ip_hash;  # Session affinity
    }
}
```

---

## Incident Response

### Common Incidents

#### 1. High Memory Usage

**Symptoms:**
- Slow responses
- OOM kills
- `zeroclaw memory_entries > 10000`

**Resolution:**
```bash
# Check memory count
zeroclaw status

# Prune old memories
sqlite3 ~/.zeroclaw/workspace/memory.db <<EOF
DELETE FROM memories WHERE timestamp < datetime('now', '-30 days');
VACUUM;
EOF

# Or reduce max_entries in config
```

#### 2. Gateway Unreachable

**Symptoms:**
- Webhook failures
- Channel disconnections
- Health check failures

**Resolution:**
```bash
# Check service status
systemctl status zeroclaw

# Check port binding
ss -tlnp | grep 3000

# Restart service
systemctl restart zeroclaw

# Check logs
journalctl -u zeroclaw -f
```

#### 3. Channel Disconnected

**Symptoms:**
- No incoming messages
- Bot appears offline

**Resolution:**
```bash
# Check channel health
zeroclaw channel doctor

# Restart channels
zeroclaw channel start

# Check token validity
# (Telegram tokens expire, Discord bots need re-auth)
```

#### 4. Cost Overrun

**Symptoms:**
- `zeroclaw_cost_usd_total` exceeds budget
- Alerts firing

**Resolution:**
```bash
# Check current cost
zeroclaw status

# Reduce max_cost_per_day_cents
# Edit ~/.zeroclaw/config.toml

# Or temporarily disable agent
systemctl stop zeroclaw
```

### Runbook Template

```markdown
# Incident: [Brief Description]

## Detection
- Alert: [Alert name]
- Detected by: [Monitoring system]
- Time: [UTC timestamp]

## Impact
- Affected users: [Count/description]
- Affected channels: [List]
- Duration: [Minutes]

## Root Cause
[Analysis of what caused the incident]

## Resolution
[Steps taken to resolve]

## Prevention
[Actions to prevent recurrence]
```

---

## Upgrade Procedures

### Pre-Upgrade Checklist

- [ ] Review changelog for breaking changes
- [ ] Create backup (`zeroclaw-backup.sh pre-upgrade`)
- [ ] Test upgrade in staging environment
- [ ] Document rollback procedure
- [ ] Notify stakeholders of maintenance window

### Upgrade Process

```bash
# 1. Stop service
systemctl stop zeroclaw

# 2. Backup current state
zeroclaw-backup.sh pre-v$(zeroclaw --version)

# 3. Download new version
curl -LO https://github.com/zeroclaw-labs/zeroclaw/releases/download/v0.2.0/zeroclaw-x86_64-unknown-linux-gnu.tar.gz

# 4. Extract
tar -xzf zeroclaw-x86_64-unknown-linux-gnu.tar.gz
sudo mv zeroclaw /usr/local/bin/

# 5. Verify binary
zeroclaw --version

# 6. Check config compatibility
zeroclaw doctor

# 7. Start service
systemctl start zeroclaw

# 8. Verify health
zeroclaw status
zeroclaw channel doctor
curl http://127.0.0.1:3000/health
```

### Rollback Procedure

```bash
# 1. Stop service
systemctl stop zeroclaw

# 2. Restore previous version
sudo mv /usr/local/bin/zeroclaw /usr/local/bin/zeroclaw.new
sudo mv /usr/local/bin/zeroclaw.old /usr/local/bin/zeroclaw

# 3. Restore config if needed
cp ~/.zeroclaw/config.new.toml ~/.zeroclaw/config.toml

# 4. Start service
systemctl start zeroclaw

# 5. Verify
zeroclaw --version
zeroclaw doctor
```

### Post-Upgrade Verification

```bash
# Version check
zeroclaw --version

# Config validation
zeroclaw doctor

# Channel health
zeroclaw channel doctor

# Gateway health
curl http://127.0.0.1:3000/health

# Test message
zeroclaw agent -m "Upgrade verification test"

# Check metrics
curl http://127.0.0.1:9090/metrics | grep zeroclaw
```

---

## Conclusion

ZeroClaw is designed for **production-grade deployment** with:

1. **Security First** - Secure defaults, encryption, sandboxing
2. **High Availability** - Health checks, failover, monitoring
3. **Scalability** - Horizontal scaling, load balancing
4. **Observability** - Prometheus, OpenTelemetry, logging
5. **Recoverability** - Backup scripts, migration support
6. **Performance** - Optimized builds, caching, tuning

Following this guide ensures ZeroClaw runs reliably in production environments.
