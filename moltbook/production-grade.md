# Production-Grade Considerations for Moltbook Ecosystem

## Executive Summary

This document covers production deployment, security hardening, monitoring, and operational considerations for the Moltbook ecosystem. It addresses both the TypeScript-based components (Moltbot, MoltHub) and the Rust-based lightweight agents (Zeroclaw).

---

## 1. Deployment Architecture

### 1.1 Component Overview

| Component | Runtime | Deployment Target | Resource Profile |
|-----------|---------|-------------------|------------------|
| **Moltbot Gateway** | Node.js 22.12+ | macOS/Linux server, Docker | 1GB+ RAM |
| **MoltHub Web** | TanStack Start | Vercel, Netlify | Serverless |
| **MoltHub Backend** | Convex | Convex Cloud | Managed |
| **Zeroclaw Daemon** | Rust (static binary) | Any Linux/ARM/x86 | <5MB RAM |
| **CLAWDINATOR** | NixOS EC2 | AWS | t3.small+ |

### 1.2 Recommended Topologies

#### Small Scale (Personal Use)

```
┌─────────────────────┐
│   macOS Mini /      │
│   Linux Server      │
│   (Moltbot Gateway) │
├─────────────────────┤
│ - WhatsApp channel  │
│ - Telegram channel  │
│ - SQLite memory     │
│ - Local LLM API     │
└─────────────────────┘
         │
         ▼
┌─────────────────────┐
│   Remote LLM        │
│   (Venice/OpenAI)   │
└─────────────────────┘
```

#### Medium Scale (Team/Small Business)

```
┌─────────────────────────────────────────────────────────┐
│                    Gateway Host (Primary)               │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Moltbot Gateway (127.0.0.1:18789)              │    │
│  │  - Multiple channels (WhatsApp, Telegram, etc.) │    │
│  │  - SQLite + sqlite-vec memory                   │    │
│  └─────────────────────────────────────────────────┘    │
│                        │                                  │
│  ┌─────────────────────┴──────────────────────┐          │
│  │              Tailscale Serve               │          │
│  │         (Private network exposure)         │          │
│  └────────────────────────────────────────────┘          │
└─────────────────────────────────────────────────────────┘
         │
         ▼ (encrypted tunnel)
┌─────────────────────┐    ┌─────────────────────┐
│   Team Devices      │    │   Remote LLM        │
│   (paired via       │    │   (OpenRouter/      │
│    device tokens)   │    │    Anthropic)       │
└─────────────────────┘    └─────────────────────┘
```

#### Large Scale (Production/Enterprise)

```
┌─────────────────────────────────────────────────────────────────┐
│                      Load Balancer (nginx/HAProxy)              │
│                    TLS termination + rate limiting              │
└─────────────────────────────────────────────────────────────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌─────────┐ ┌─────────┐
│ Gateway │ │ Gateway │  (Active-Active with shared state)
│   #1    │ │   #2    │
└─────────┘ └─────────┘
    │         │
    └────┬────┘
         ▼
┌─────────────────────────────────────────────────────────┐
│              PostgreSQL (shared memory backend)         │
│              - High availability                        │
│              - Point-in-time recovery                   │
│              - Read replicas for search                 │
└─────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────┐
│              Redis Cluster (session cache)              │
│              - Distributed session state                │
│              - Pub/Sub for real-time sync               │
└─────────────────────────────────────────────────────────┘
```

### 1.3 MoltHub Deployment (Vercel + Convex)

**Step 1: Deploy Convex Backend**

```bash
# From moltbot/molthub directory
bunx convex deploy

# Required environment variables:
# AUTH_GITHUB_ID - GitHub OAuth application ID
# AUTH_GITHUB_SECRET - GitHub OAuth secret
# CONVEX_SITE_URL - Your Convex deployment URL
# JWT_PRIVATE_KEY - PEM-encoded private key for auth
# JWKS - JSON Web Key Set for token verification
# OPENAI_API_KEY - For embedding generation
# SITE_URL - Your web app URL
```

**Step 2: Deploy Vercel Web App**

```bash
# From moltbot/molthub directory
vercel --prod

# Required environment variables:
# VITE_CONVEX_URL - Convex deployment URL
# VITE_CONVEX_SITE_URL - Convex site URL (same as CONVEX_SITE_URL)
# SITE_URL - Your web app URL (for redirects)
```

**Step 3: Configure API Routing**

The `vercel.json` rewrites `/api/*` to Convex:

```json
{
  "rewrites": [
    {
      "source": "/api/:path*",
      "destination": "https://your-deployment.convex.site/api/:path*"
    }
  ]
}
```

**Post-Deploy Verification**

```bash
# Test search API
curl -i "https://molthub.com/api/v1/search?q=test"

# Test skills API
curl -i "https://molthub.com/api/v1/skills/gifgrep"

# Test authentication flow
molthub login --site https://molthub.com
molthub whoami
```

---

## 2. Security Hardening

### 2.1 Gateway Security (Moltbot)

#### Pairing System

The gateway requires device pairing for all connections:

```toml
# ~/.zeroclaw/config.toml (or Moltbot equivalent)
[gateway]
require_pairing = true
host = "127.0.0.1"  # Never bind to 0.0.0.0 without tunnel
allow_public_bind = false  # Refuse public exposure
```

**Pairing Flow:**

1. Gateway generates 6-digit one-time code on startup
2. Client exchanges code via `POST /pair` for bearer token
3. All subsequent `/webhook` requests require `Authorization: Bearer <token>`
4. Device identity stored in `~/.clawdbot/pairing.json`

#### Channel Allowlists (Deny-by-Default)

Empty allowlist denies all inbound messages:

```toml
[channels_config.telegram]
allowed_users = []  # Deny all (default)
# allowed_users = ["*"]  # Allow all (explicit opt-in)
# allowed_users = ["your_username", "123456789"]  # Specific users
```

**Operator Approval Flow:**

1. User sends message to bot
2. Bot responds with approval hint: `Run: zeroclaw channel bind-telegram <identity>`
3. Operator runs command locally to approve
4. User retries - message now processed

#### Filesystem Scoping

```toml
[autonomy]
workspace_only = true  # Default: restrict to workspace
allowed_commands = ["git", "npm", "cargo", "ls", "cat", "grep"]
forbidden_paths = [
  "/etc", "/root", "/proc", "/sys",
  "~/.ssh", "~/.gnupg", "~/.aws", "~/.config/claude"
]
```

**Null Byte Injection Prevention:**
- All file paths canonicalized before access
- Symlink escape detection via resolved-path checks
- Workspace boundary enforced after resolution

### 2.2 TLS Configuration

#### For Public Exposure (Not Recommended)

```bash
# Generate self-signed cert (testing only)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem \
  -days 365 -nodes -subj "/CN=localhost"

# For production, use Let's Encrypt:
certbot certonly --standalone -d your-domain.com
```

**Moltbot Gateway with TLS:**

```toml
[gateway]
tls_cert = "/path/to/cert.pem"
tls_key = "/path/to/key.pem"
```

#### SSH Tunnel (Recommended for Remote Access)

```bash
# Local port forward
ssh -N -L 18789:127.0.0.1:18789 user@host

# Reverse tunnel (from server to client)
ssh -N -R 18789:localhost:18789 user@client
```

#### Tailscale Funnel (Zero-Trust Network)

```bash
# Enable Tailscale Funnel for HTTPS exposure
tailscale funnel 18789

# Or Serve for private network only
tailscale serve https:/18789
```

### 2.3 Secret Management

#### Zeroclaw Encrypted Secrets

```toml
[secrets]
encrypt = true  # API keys encrypted with local key file
```

**Encryption Details:**
- Key file: `~/.zeroclaw/.secret_key` (chmod 400)
- Algorithm: ChaCha20-Poly1305 (AEAD)
- Key derivation: Argon2id from system secrets

#### MoltHub Environment Variables

Use platform secret management:

```bash
# Vercel
vercel env add OPENAI_API_KEY secret production

# Convex
bunx convex env set OPENAI_API_KEY
```

**Never commit secrets:**

```gitignore
# .gitignore
.env
.env.local
.env.*.local
*.pem
*.key
secrets.json
pairing.json
.secret_key
```

### 2.4 Docker Security

```bash
# Secure Docker run
docker run --read-only --cap-drop=ALL \
  --security-opt no-new-privileges:true \
  --user 1000:1000 \
  -v moltbot-data:/app/data:rw \
  -v moltbot-config:/app/config:ro \
  --tmpfs /tmp:rw,noexec,nosuid,size=100M \
  moltbot/moltbot:latest
```

**Docker Compose (Production):**

```yaml
version: '3.8'

services:
  moltbot:
    image: moltbot/moltbot:latest
    read_only: true
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    user: "1000:1000"
    volumes:
      - moltbot-data:/app/data:rw
      - moltbot-config:/app/config:ro
    tmpfs:
      - /tmp:rw,noexec,nosuid,size=100M
    networks:
      - moltbot-net
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '0.5'
          memory: 512M

volumes:
  moltbot-data:
  moltbot-config:

networks:
  moltbot-net:
    driver: bridge
    ipam:
      config:
        - subnet: 172.28.0.0/16
```

### 2.5 Security Checklist

| # | Item | Status | Implementation |
|---|------|--------|----------------|
| 1 | Gateway not publicly exposed | ✅ | Binds `127.0.0.1` by default, refuses `0.0.0.0` |
| 2 | Pairing required | ✅ | 6-digit OTP, bearer token exchange |
| 3 | Filesystem scoped | ✅ | `workspace_only=true`, 14 system dirs blocked |
| 4 | Access via tunnel only | ✅ | Tailscale, Cloudflare, ngrok support |
| 5 | Channel allowlists | ✅ | Deny-by-default, explicit opt-in |
| 6 | Secrets encrypted at rest | ✅ | ChaCha20-Poly1305 AEAD |
| 7 | Node.js security patches | ✅ | Requires 22.12.0+ (CVE-2025-59466, CVE-2026-21636) |
| 8 | Docker non-root | ✅ | Runs as `node` user in official image |

---

## 3. Monitoring & Observability

### 3.1 Health Endpoints

**Moltbot Gateway:**

```bash
# WebSocket health check
moltbot health --json

# Full status with deep probe
moltbot status --deep --usage

# Gateway RPC health
moltbot gateway health
```

**Response:**

```json
{
  "ok": true,
  "gateway": {
    "status": "healthy",
    "uptime_ms": 86400000,
    "channels": {
      "whatsapp": "connected",
      "telegram": "connected",
      "discord": "disconnected"
    },
    "memory": {
      "backend": "sqlite",
      "index_size": 15420,
      "last_sync": "2026-03-22T10:30:00Z"
    },
    "provider": {
      "primary": "anthropic",
      "quota_remaining": 95000,
      "quota_reset": "2026-04-01T00:00:00Z"
    }
  }
}
```

### 3.2 Logging Configuration

**Structured Logging (Moltbot):**

```toml
[logging]
level = "info"  # debug, info, warn, error
format = "json"  # or "pretty"
output = "stdout"  # or file path

# Log filtering
filters = [
  { target = "gateway::websocket", level = "debug" },
  { target = "channels::whatsapp", level = "info" },
  { target = "provider::anthropic", level = "warn" }
]
```

**Zeroclaw Tracing:**

```bash
# Enable debug tracing
RUST_LOG=zeroclaw=debug,zeroclaw::gateway=trace cargo run

# Or in config:
[observability]
tracing_level = "info"
tracing_format = "json"  # or "pretty"
```

### 3.3 Metrics Export (Zeroclaw)

**Prometheus Metrics:**

```toml
[observability.prometheus]
enabled = true
port = 9090
path = "/metrics"

# Custom labels
[observability.prometheus.labels]
environment = "production"
instance = "zeroclaw-01"
```

**Sample Metrics:**

```prometheus
# HELP zeroclaw_agent_requests_total Total number of agent requests
# TYPE zeroclaw_agent_requests_total counter
zeroclaw_agent_requests_total{provider="openrouter"} 1547

# HELP zeroclaw_agent_latency_seconds Agent request latency
# TYPE zeroclaw_agent_latency_seconds histogram
zeroclaw_agent_latency_seconds_bucket{le="0.1"} 234
zeroclaw_agent_latency_seconds_bucket{le="0.5"} 891
zeroclaw_agent_latency_seconds_bucket{le="1.0"} 1423
zeroclaw_agent_latency_seconds_bucket{le="+Inf"} 1547

# HELP zeroclaw_memory_index_size Current memory index size
# TYPE zeroclaw_memory_index_size gauge
zeroclaw_memory_index_size 15420

# HELP zeroclaw_channel_messages_total Messages processed by channel
# TYPE zeroclaw_channel_messages_total counter
zeroclaw_channel_messages_total{channel="telegram"} 3421
zeroclaw_channel_messages_total{channel="discord"} 1205
```

### 3.4 OpenTelemetry Integration

**Zeroclaw OTLP Export:**

```toml
[observability.opentelemetry]
enabled = true
exporter = "otlp"
endpoint = "https://otel-collector.example.com:4317"
protocol = "grpc"  # or "http"

[observability.opentelemetry.tracing]
enabled = true
sample_rate = 0.1  # 10% sampling

[observability.opentelemetry.metrics]
enabled = true
push_interval_secs = 60
```

**Cargo Feature:**

```bash
cargo build --release --features opentelemetry
```

### 3.5 Alerting

**Health Check Cron:**

```bash
# /etc/cron.d/moltbot-health
*/5 * * * * root /usr/local/bin/moltbot health --json | \
  jq -e '.ok == true' || \
  curl -X POST https://alerts.example.com/webhook \
    -H "Content-Type: application/json" \
    -d '{"alert":"moltbot-unhealthy","host":"'$(hostname)'"}'
```

**Prometheus Alert Rules:**

```yaml
groups:
  - name: moltbook
    rules:
      - alert: MoltbotGatewayDown
        expr: up{job="moltbot"} == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Moltbot Gateway is down"

      - alert: MoltbotHighLatency
        expr: histogram_quantile(0.95, rate(zeroclaw_agent_latency_seconds_bucket[5m])) > 5
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Moltbot latency above 5s (p95)"

      - alert: MoltbotMemoryLow
        expr: zeroclaw_memory_index_size < 1000
        for: 1h
        labels:
          severity: info
        annotations:
          summary: "Memory index unusually small"
```

---

## 4. Performance Optimization

### 4.1 Memory Indexing

**Vector Search Optimization:**

```toml
[memory]
backend = "sqlite"
embedding_provider = "openai"  # or "local" for offline

[memory.search]
hybrid_enabled = true
vector_weight = 0.7
keyword_weight = 0.3
candidate_multiplier = 4

[memory.cache]
enabled = true
max_entries = 50000
```

**SQLite Optimization:**

```toml
[memory.store.vector]
enabled = true  # Use sqlite-vec extension
extension_path = "/usr/lib/sqlite-vec.so"  # Optional

# Connection tuning
sqlite_open_timeout_secs = 30
```

**Batch Embeddings:**

```toml
[memory.remote.batch]
enabled = true
concurrency = 2
wait = true
poll_interval_ms = 1000
timeout_minutes = 30
```

### 4.2 Session Compaction

**Pre-Compaction Memory Flush:**

```json5
{
  agents: {
    defaults: {
      compaction: {
        reserveTokensFloor: 20000,
        memoryFlush: {
          enabled: true,
          softThresholdTokens: 4000,
          systemPrompt: "Session nearing compaction. Store durable memories now.",
          prompt: "Write any lasting notes to memory/YYYY-MM-DD.md; reply with NO_REPLY."
        }
      }
    }
  }
}
```

### 4.3 Provider Failover

**Fallback Chain:**

```json5
{
  agents: {
    defaults: {
      model: {
        primary: "anthropic/claude-opus-4-5",
        fallbacks: [
          "openai/gpt-4o",
          "openrouter/anthropic/claude-3-5-sonnet",
          "venice/claude-opus-45"
        ]
      }
    }
  }
}
```

**Automatic Failover Conditions:**
- 429 (Rate Limited) - retry with exponential backoff
- 500/502/503 (Server Error) - immediate failover
- Timeout (>60s) - failover after 2 retries

---

## 5. Backup & Recovery

### 5.1 Data Locations

| Component | Data Path | Backup Priority |
|-----------|-----------|-----------------|
| Moltbot Config | `~/.clawdbot/config.json` | Critical |
| Moltbot Sessions | `~/.clawdbot/agents/*/sessions/` | High |
| Moltbot Memory | `~/clawd/memory/`, `~/clawd/MEMORY.md` | Critical |
| Moltbot Pairing | `~/.clawdbot/pairing.json` | Critical |
| Zeroclaw Config | `~/.zeroclaw/config.toml` | Critical |
| Zeroclaw Secrets | `~/.zeroclaw/.secret_key` | Critical |
| Zeroclaw Memory | `~/.zeroclaw/memory/` | High |
| Zeroclaw SQLite | `~/.zeroclaw/data/*.sqlite` | Critical |

### 5.2 Backup Script

```bash
#!/bin/bash
# backup-moltbook.sh

BACKUP_DIR="/backup/moltbook/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Moltbot backups
cp -r ~/.clawdbot/config.json "$BACKUP_DIR/"
cp -r ~/.clawdbot/pairing.json "$BACKUP_DIR/"
cp -r ~/clawd/memory "$BACKUP_DIR/"
cp -r ~/clawd/MEMORY.md "$BACKUP_DIR/"
cp -r ~/.clawdbot/agents "$BACKUP_DIR/agents"

# Zeroclaw backups
cp -r ~/.zeroclaw/config.toml "$BACKUP_DIR/"
cp -r ~/.zeroclaw/.secret_key "$BACKUP_DIR/"
cp -r ~/.zeroclaw/memory "$BACKUP_DIR/"
cp -r ~/.zeroclaw/data "$BACKUP_DIR/"

# Compress
tar -czf "$BACKUP_DIR.tar.gz" -C "$(dirname $BACKUP_DIR)" "$(basename $BACKUP_DIR)"
rm -rf "$BACKUP_DIR"

# Upload to S3 (optional)
aws s3 cp "$BACKUP_DIR.tar.gz" s3://your-bucket/moltbook/backups/

# Retention: keep last 30 days
find /backup/moltbook -name "*.tar.gz" -mtime +30 -delete

echo "Backup completed: $BACKUP_DIR.tar.gz"
```

### 5.3 Recovery Procedure

**Full Restore:**

```bash
# 1. Download backup
aws s3 cp s3://your-bucket/moltbook/backups/20260322-103000.tar.gz .
tar -xzf 20260322-103000.tar.gz -C /

# 2. Restore permissions
chmod 400 ~/.zeroclaw/.secret_key
chmod 600 ~/.clawdbot/pairing.json

# 3. Restart services
systemctl restart moltbot
systemctl restart zeroclaw

# 4. Verify
moltbot status
zeroclaw status
```

---

## 6. Scaling Strategies

### 6.1 Horizontal Scaling (Multi-Gateway)

**Architecture:**

```
                    ┌─────────────┐
                    │   nginx     │
                    │  (TLS/LB)   │
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│   Gateway 1   │ │   Gateway 2   │ │   Gateway 3   │
│   10.0.1.10   │ │   10.0.1.11   │ │   10.0.1.12   │
└───────┬───────┘ └───────┬───────┘ └───────┬───────┘
        │                 │                 │
        └─────────────────┼─────────────────┘
                          ▼
                 ┌─────────────────┐
                 │  PostgreSQL     │
                 │  (shared state) │
                 └─────────────────┘
```

**Configuration:**

```toml
# All gateways share PostgreSQL memory backend
[memory]
backend = "postgres"

[storage.provider.config]
provider = "postgres"
db_url = "postgres://user:pass@db.example.com:5432/moltbot"
schema = "public"
table = "memories"
```

**Session Affinity:**

Use consistent session keys per channel to ensure messages from the same conversation always route to the same gateway.

### 6.2 Vertical Scaling

**Resource Allocation:**

| Workload | CPU | RAM | Storage |
|----------|-----|-----|---------|
| Light (personal) | 2 cores | 2GB | 10GB SSD |
| Medium (team) | 4 cores | 4GB | 50GB SSD |
| Heavy (production) | 8 cores | 16GB | 200GB NVMe |

**macOS Specific:**

```bash
# launchd configuration for auto-restart
# /Library/LaunchDaemons/bot.molt.gateway.plist
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>bot.molt.gateway</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/moltbot</string>
    <string>gateway</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key>
    <false/>
    <key>Crashed</key>
    <true/>
  </dict>
  <key>StandardOutPath</key>
  <string>/var/log/moltbot/gateway.out</string>
  <key>StandardErrorPath</key>
  <string>/var/log/moltbot/gateway.err</string>
</dict>
</plist>
```

---

## 7. Production Checklist

### Pre-Deployment

- [ ] Security audit completed (`moltbot security audit --deep`)
- [ ] All secrets rotated from development values
- [ ] TLS certificates configured (if public exposure required)
- [ ] Firewall rules configured (gateway not publicly accessible)
- [ ] Backup strategy implemented and tested
- [ ] Monitoring/alerting configured
- [ ] Rate limits configured for LLM providers
- [ ] Channel allowlists populated with known users

### Deployment

- [ ] Services started with correct configuration
- [ ] Health endpoints responding correctly
- [ ] All channels connected and authenticated
- [ ] Memory index populated (if restoring from backup)
- [ ] Pairing completed for all operator devices

### Post-Deployment

- [ ] Load testing completed (if applicable)
- [ ] Failover testing completed (provider fallbacks work)
- [ ] Backup verification (restore test successful)
- [ ] Alert thresholds tuned based on baseline metrics
- [ ] Documentation updated with actual deployment details
- [ ] Runbook created for common operational tasks

### Ongoing Operations

- [ ] Daily: Check health dashboards
- [ ] Weekly: Review logs for anomalies
- [ ] Monthly: Rotate API keys and secrets
- [ ] Quarterly: Review and update allowlists
- [ ] As needed: Apply security patches (Node.js, Rust dependencies)

---

## 8. Troubleshooting

### Common Issues

**Gateway Won't Start:**

```bash
# Check port conflict
lsof -i :18789

# Check configuration syntax
moltbot config get  # Should output valid JSON

# Check logs
moltbot logs --limit 100
```

**Channel Disconnected:**

```bash
# Check channel status
moltbot channels status --probe

# Re-authenticate channel
moltbot channels login --channel whatsapp

# Check rate limits (WhatsApp specific)
# WhatsApp has strict rate limits - wait 24h if banned
```

**Memory Search Slow:**

```bash
# Check index size
moltbot memory status

# Rebuild index if corrupted
moltbot memory index --full

# Check sqlite-vec extension loaded
sqlite3 ~/.clawdbot/memory/default.sqlite \
  "SELECT * FROM sqlite_master WHERE type='table' AND name LIKE 'vec_%';"
```

**Provider Quota Exhausted:**

```bash
# Check usage
moltbot status --usage

# Switch to fallback provider
moltbot models set venice/claude-opus-45

# Or add more fallbacks
moltbot models fallbacks add openai/gpt-4o
```

---

## 9. Cost Optimization

### Provider Cost Comparison (as of 2026)

| Provider | Model | Input (per 1M) | Output (per 1M) | Context |
|----------|-------|----------------|-----------------|---------|
| Venice AI | Llama 3.3 70B | $0.15 | $0.15 | 128K |
| Venice AI | Claude Opus 45 | $3.00 | $15.00 | 200K |
| OpenRouter | Claude 3.5 Sonnet | $3.00 | $15.00 | 200K |
| OpenAI | GPT-4o | $2.50 | $10.00 | 128K |
| Anthropic | Claude Opus 4.5 | $5.00 | $25.00 | 200K |

**Recommendation:** Use Venice AI for cost-effective Opus access, or Llama 3.3 for 99% cost savings on non-critical tasks.

### Memory Optimization

**Local Embeddings (Free):**

```toml
[memory]
provider = "local"
local.modelPath = "hf:ggml-org/embeddinggemma-300M-GGUF/embeddinggemma-300M-Q8_0.gguf"
fallback = "none"  # Disable remote fallback
```

**Trade-offs:**
- Local: Free, private, ~0.6GB model download, slower than API
- Remote: ~$0.02 per 1K requests, requires API key, faster

---

## 10. Security Incident Response

### If Gateway Compromised

1. **Immediate:**
   ```bash
   # Stop gateway
   systemctl stop moltbot

   # Revoke all pairing tokens
   rm ~/.clawdbot/pairing.json

   # Rotate all API keys
   # - LLM providers (OpenAI, Anthropic, etc.)
   # - Channel credentials (WhatsApp, Telegram, etc.)
   ```

2. **Investigation:**
   ```bash
   # Check logs for suspicious activity
   moltbot logs --limit 10000 | grep -E "(unauthorized|failed|denied)"

   # Check pairing history
   cat ~/.clawdbot/pairing.json

   # Check for unauthorized config changes
   git diff ~/.clawdbot/config.json
   ```

3. **Recovery:**
   ```bash
   # Restore from known-good backup
   # Re-pair all authorized devices
   # Re-deploy with updated secrets
   ```

### If Memory Data Leaked

1. **Assess Impact:**
   - Review `MEMORY.md` and `memory/*.md` contents
   - Identify any exposed secrets, API keys, or sensitive information

2. **Mitigate:**
   ```bash
   # Redact sensitive data from memory files
   # Rotate any exposed credentials
   # Enable memory encryption at rest (if available)
   ```

3. **Prevent:**
   - Review workspace access controls
   - Enable filesystem sandboxing
   - Audit tool allowlists

---

*Production-grade considerations - Part of Moltbook ecosystem exploration*
*Last updated: 2026-03-22*
