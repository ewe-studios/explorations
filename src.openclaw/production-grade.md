# src.openclaw Production Grade Exploration

## Overview

This document examines the production-grade features and considerations of **src.openclaw**, focusing on deployment, security, observability, and operational excellence.

**Source Path:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/src.openclaw/`

---

## Table of Contents

1. [Production Readiness Assessment](#production-readiness-assessment)
2. [Deployment Architecture](#deployment-architecture)
3. [Security Hardening](#security-hardening)
4. [Observability & Monitoring](#observability--monitoring)
5. [High Availability & Scalability](#high-availability--scalability)
6. [Disaster Recovery](#disaster-recovery)
7. [Operational Procedures](#operational-procedures)
8. [Compliance Considerations](#compliance-considerations)

---

## Production Readiness Assessment

### Feature Completeness Matrix

| Category | Feature | Status | Notes |
|----------|---------|--------|-------|
| **Core** | Multi-channel support | Production | 15+ channels supported |
| **Core** | Session management | Production | HMAC-chained storage |
| **Core** | Agent system | Production | Multi-agent support |
| **Core** | Tool execution | Production | Sandbox execution |
| **Security** | Authentication | Production | Device pairing + tokens |
| **Security** | Authorization | Production | RBAC + allowlists |
| **Security** | Secret management | Production | Encrypted at rest |
| **Security** | Audit logging | Production | Clauditor integration |
| **Deployment** | Docker | Production | Official images |
| **Deployment** | Nix | Production | NixOS modules |
| **Deployment** | Ansible | Production | Deployment roles |
| **Observability** | Logging | Production | Structured JSON |
| **Observability** | Metrics | Production | Prometheus endpoint |
| **Observability** | Tracing | Beta | OpenTelemetry |
| **HA** | Clustering | Limited | Single gateway primary |
| **HA** | Failover | Manual | No automatic failover |
| **DR** | Backup | Manual | Session backup scripts |
| **DR** | Restore | Manual | Documented procedures |

### Production Maturity Levels

```
Level 5: Optimized         [  ] - Future state
Level 4: Managed           [X] - Achieved
Level 3: Defined           [X] - Achieved
Level 2: Repeatable        [X] - Achieved
Level 1: Initial           [X] - Achieved
```

---

## Deployment Architecture

### Reference Architecture: Single Node

```
┌─────────────────────────────────────────────────────────────────┐
│                    Single Node Deployment                        │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  Reverse Proxy (nginx/traefik/caddy)                            │
│  - TLS termination                                               │
│  - Rate limiting                                                │
│  - Request routing                                              │
└─────────────────────────────┬───────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  OpenClaw Gateway (Docker container)                            │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Gateway Process (Node.js)                                 │  │
│  │  - HTTP API server (port 18788)                           │  │
│  │  - WebSocket server (port 18789)                          │  │
│  │  - Control UI (port 18790)                                │  │
│  └───────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  Sidecar: Clauditor (sysaudit daemon)                     │  │
│  │  - Tamper-evident logging                                 │  │
│  │  - Security event detection                               │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Persistent Volumes:                                            │
│  - /data/openclaw/config  (Configuration)                       │
│  - /data/openclaw/sessions (Session state)                      │
│  - /data/openclaw/logs    (Audit logs)                          │
│  - /data/openclaw/secrets (Encrypted secrets)                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  External Dependencies                                          │
│  - PostgreSQL/SQLite (optional, for extended storage)          │
│  - Redis (optional, for distributed caching)                   │
│  - LanceDB (vector memory backend)                             │
└─────────────────────────────────────────────────────────────────┘
```

### Reference Architecture: High Availability

```
┌─────────────────────────────────────────────────────────────────┐
│              High Availability Deployment                        │
└─────────────────────────────────────────────────────────────────┘

                    ┌─────────────────┐
                    │  Load Balancer  │
                    │  (HAProxy/ALB)  │
                    └────────┬────────┘
                             │
           ┌─────────────────┼─────────────────┐
           │                 │                 │
           ▼                 ▼                 ▼
    ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
    │  Gateway A  │  │  Gateway B  │  │  Gateway C  │
    │  (Active)   │  │  (Standby)  │  │  (Standby)  │
    └──────┬──────┘  └──────┬──────┘  └──────┬──────┘
           │                │                │
           └────────────────┼────────────────┘
                            │
                            ▼
                   ┌─────────────────┐
                   │  Shared Storage │
                   │  (PostgreSQL)   │
                   └─────────────────┘
                            │
           ┌────────────────┼────────────────┐
           │                │                │
           ▼                ▼                ▼
    ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
    │   Redis     │  │  LanceDB    │  │   S3/MinIO  │
    │   Cache     │  │   Vector    │  │   Objects   │
    └─────────────┘  └─────────────┘  └─────────────┘

Note: HA requires external session storage and distributed
      coordination. Current implementation is primarily
      single-node; HA is achievable with infrastructure.
```

### Docker Deployment

**docker-compose.yml:**
```yaml
version: '3.8'

services:
  openclaw-gateway:
    image: ghcr.io/openclaw/openclaw:latest
    container_name: openclaw-gateway
    restart: unless-stopped
    ports:
      - "18788:18788"  # HTTP API
      - "18789:18789"  # WebSocket
      - "18790:18790"  # Control UI
    environment:
      - OPENCLAW_CONFIG_PATH=/data/config
      - OPENCLAW_SESSIONS_PATH=/data/sessions
      - OPENCLAW_LOG_LEVEL=info
      - TZ=UTC
    volumes:
      - ./config:/data/config:ro
      - ./sessions:/data/sessions
      - ./logs:/data/logs
    secrets:
      - openclaw_master_token
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:18788/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    networks:
      - openclaw-network
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 2G
        reservations:
          cpus: '0.5'
          memory: 512M

  clauditor:
    image: ghcr.io/openclaw/clauditor:latest
    container_name: clauditor
    pid: host
    privileged: true
    volumes:
      - /var/run/sysaudit:/run/sysaudit
      - ./logs/audit:/var/log/sysaudit
    secrets:
      - clauditor_hmac_key

secrets:
  openclaw_master_token:
    file: ./secrets/master_token.txt
  clauditor_hmac_key:
    file: ./secrets/hmac_key.txt

networks:
  openclaw-network:
    driver: bridge
```

### NixOS Deployment

**flake.nix:**
```nix
{
  description = "OpenClaw Gateway NixOS deployment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    openclaw.url = "github:openclaw/openclaw";
  };

  outputs = { self, nixpkgs, openclaw }: {
    nixosConfigurations.gateway = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        openclaw.nixosModules.openclaw
        ({ config, lib, ... }: {
          services.openclaw = {
            enable = true;
            package = openclaw.packages.${pkgs.system}.default;
            settings = {
              agents.main = {
                model = "anthropic:claude-sonnet-4-20250514";
                tools = ["@openclaw/filesystem" "@openclaw/shell"];
              };
              channels.telegram = {
                enable = true;
                botToken = config.age.secrets.telegram-bot-token.path;
              };
            };
          };

          age.secrets = {
            telegram-bot-token.file = ./secrets/telegram-bot-token.age;
            openclaw-master-token.file = ./secrets/master-token.age;
          };
        })
      ];
    };
  };
}
```

### Ansible Deployment

**playbook.yml:**
```yaml
---
- name: Deploy OpenClaw Gateway
  hosts: gateway_servers
  become: true
  vars:
    openclaw_version: "latest"
    openclaw_config_path: /etc/openclaw
    openclaw_data_path: /var/lib/openclaw

  roles:
    - role: openclaw.openclaw
      vars:
        openclaw_agents:
          - name: main
            model: anthropic:claude-sonnet-4-20250514
            tools:
              - "@openclaw/filesystem"
              - "@openclaw/shell"
        openclaw_channels:
          telegram:
            enabled: true
            bot_token: "{{ telegram_bot_token }}"
          discord:
            enabled: false

  handlers:
    - name: Restart openclaw
      systemd:
        name: openclaw
        state: restarted
```

---

## Security Hardening

### Security Configuration Checklist

#### Network Security

```yaml
# config/security.yml
security:
  # TLS Configuration
  tls:
    enabled: true
    certPath: /etc/openclaw/tls/server.crt
    keyPath: /etc/openclaw/tls/server.key
    minVersion: "TLS1.3"

  # CORS Configuration
  cors:
    allowedOrigins:
      - "https://admin.example.com"
    allowedMethods: ["GET", "POST", "OPTIONS"]
    allowCredentials: true

  # Rate Limiting
  rateLimit:
    enabled: true
    requestsPerMinute: 100
    burstSize: 20

  # SSRF Protection
  ssrf:
    enabled: true
    blockedRanges:
      - "10.0.0.0/8"
      - "172.16.0.0/12"
      - "192.168.0.0/16"
      - "169.254.169.254/32"  # Cloud metadata
```

#### Authentication Hardening

```yaml
# config/auth.yml
auth:
  # Token Configuration
  tokens:
    algorithm: "HS256"
    expiry: "24h"
    refreshExpiry: "7d"

  # Device Pairing
  pairing:
    enabled: true
    codeLength: 8
    expiryMinutes: 5
    maxAttempts: 3

  # Session Security
  sessions:
    requireAuthentication: true
    idleTimeout: "30m"
    absoluteTimeout: "24h"
    concurrentLimit: 5

  # API Authentication
  api:
    requireSignature: true
    signatureAlgorithm: "HMAC-SHA256"
    timestampTolerance: "5m"
```

#### Access Control

```yaml
# config/rbac.yml
rbac:
  roles:
    admin:
      permissions:
        - "gateway:*"
        - "agents:*"
        - "config:*"
        - "secrets:*"
        - "audit:read"

    operator:
      permissions:
        - "gateway:read"
        - "agents:read"
        - "agents:invoke"
        - "audit:read"

    viewer:
      permissions:
        - "gateway:read"
        - "agents:read"

  # Tool Permissions
  tools:
    "@openclaw/filesystem":
      allowedRoles: ["admin", "operator"]
      requireApproval: true

    "@openclaw/shell":
      allowedRoles: ["admin"]
      requireApproval: always

    "@openclaw/browser":
      allowedRoles: ["admin", "operator"]
      requireApproval: for-mutations
```

### Clauditor Security Watchdog

**Installation and Configuration:**

```bash
# Install Clauditor
curl -fsSL https://openclaw.ai/install-clauditor.sh | sudo bash

# Generate HMAC key
sudo dd if=/dev/urandom of=/etc/sysaudit/key bs=64 count=1
sudo chmod 640 /etc/sysaudit/key
sudo chown root:sysaudit /etc/sysaudit/key

# Configure clauditor
cat > /etc/sysaudit/config.toml << 'EOF'
[daemon]
log_dir = "/var/log/sysaudit"
heartbeat_path = "/run/sysaudit/heartbeat"
heartbeat_interval_secs = 10

[collector]
mode = "privileged"  # Uses fanotify
exec_filter = ["node", "bun", "sh", "bash"]

[alerter]
mode = "syslog"
syslog_facility = "auth"
alert_on_severity = ["high", "critical"]

[detector]
sequence_window_secs = 300
orphan_session_ttl_secs = 300
EOF

# Start the daemon
sudo systemctl enable sysaudit
sudo systemctl start sysaudit
```

**Detection Rules:**

```typescript
// detector/src/rules.ts
export const SECURITY_RULES = [
  {
    name: "credential-exfiltration",
    description: "Detect credential read followed by network activity",
    severity: "critical",
    pattern: [
      { type: "open", path: /.*\.(env|key|pem|crt)$/ },
      { type: "exec", command: /(curl|wget|nc|ssh|scp)/ },
    ],
    window: 300, // 5 minutes
  },
  {
    name: "privilege-escalation",
    description: "Detect privilege escalation attempts",
    severity: "critical",
    pattern: [
      { type: "exec", command: /sudo|su|pkexec/ },
    ],
  },
  {
    name: "persistence-mechanism",
    description: "Detect persistence mechanism installation",
    severity: "high",
    pattern: [
      { type: "exec", command: /(crontab|systemctl.*enable)/ },
      { type: "write", path: /\/etc\/(rc\.local|cron\.|systemd)/ },
    ],
  },
  {
    name: "log-tampering",
    description: "Detect log tampering attempts",
    severity: "critical",
    pattern: [
      { type: "exec", command: /(rm|truncate|echo.*>).*log/ },
      { type: "write", path: /\/var\/log/ },
    ],
  },
];
```

### Secret Management

**External Secret Store Integration:**

```yaml
# config/secrets.yml
secrets:
  # Local encrypted store (default)
  local:
    enabled: true
    encryptionKey: "$secret:local-encryption-key"
    path: /var/lib/openclaw/secrets

  # HashiCorp Vault integration
  vault:
    enabled: true
    address: https://vault.example.com:8200
    authToken: "$secret:vault-token"
    mountPath: secret/openclaw

  # AWS Secrets Manager
  aws:
    enabled: true
    region: us-east-1
    accessKeyId: "$secret:aws-access-key"
    secretAccessKey: "$secret:aws-secret-key"

  # Azure Key Vault
  azure:
    enabled: true
    vaultName: my-openclaw-vault
    tenantId: "$secret:azure-tenant-id"
    clientId: "$secret:azure-client-id"
    clientSecret: "$secret:azure-client-secret"
```

---

## Observability & Monitoring

### Logging Configuration

```yaml
# config/logging.yml
logging:
  level: "info"  # debug, info, warn, error

  # Console output
  console:
    enabled: true
    format: "json"  # json, text
    colors: false   # Disable colors in production

  # File output
  file:
    enabled: true
    path: /var/log/openclaw/gateway.log
    maxSize: "100MB"
    maxFiles: 10
    compress: true

  # Syslog output
  syslog:
    enabled: true
    facility: "local0"
    format: "rfc5424"

  # Structured fields
  fields:
    - timestamp
    - level
    - logger
    - agentId
    - sessionKey
    - channelType
    - requestId
    - duration
```

### Metrics Export

**Prometheus Metrics Endpoint:**

```
# GET http://localhost:18788/metrics

# Gateway metrics
openclaw_gateway_requests_total{method="POST",endpoint="/chat/send"} 1234
openclaw_gateway_request_duration_seconds{endpoint="/chat/send",quantile="0.99"} 0.256
openclaw_gateway_active_connections 42

# Session metrics
openclaw_sessions_active{agent="main"} 15
openclaw_sessions_total{agent="main"} 1523
openclaw_session_messages_total{agent="main",direction="inbound"} 45678

# Agent metrics
openclaw_agent_invocations_total{agent="main",model="anthropic"} 8901
openclaw_agent_tool_calls_total{agent="main",tool="@openclaw/filesystem"} 234
openclaw_agent_tool_call_duration_seconds{tool="@openclaw/filesystem",quantile="0.95"} 0.045

# Channel metrics
openclaw_channel_messages_sent_total{channel="telegram"} 12345
openclaw_channel_messages_received_total{channel="telegram"} 23456
openclaw_channel_webhook_requests_total{channel="telegram"} 567

# Error metrics
openclaw_errors_total{type="authentication",source="gateway"} 12
openclaw_errors_total{type="rate_limit",source="api"} 89

# Resource metrics
openclaw_memory_heap_bytes 134217728
openclaw_memory_rss_bytes 268435456
openclaw_event_loop_lag_seconds 0.003
```

**Grafana Dashboard JSON:**

```json
{
  "dashboard": {
    "title": "OpenClaw Gateway",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(openclaw_gateway_requests_total[5m])",
            "legendFormat": "{{method}} {{endpoint}}"
          }
        ]
      },
      {
        "title": "Active Sessions",
        "targets": [
          {
            "expr": "sum(openclaw_sessions_active)",
            "legendFormat": "Active Sessions"
          }
        ]
      },
      {
        "title": "Tool Invocation Latency (p95)",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(openclaw_agent_tool_call_duration_seconds_bucket[5m]))",
            "legendFormat": "{{tool}}"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(openclaw_errors_total[5m])",
            "legendFormat": "{{type}}"
          }
        ]
      }
    ]
  }
}
```

### Distributed Tracing

**OpenTelemetry Configuration:**

```yaml
# config/telemetry.yml
telemetry:
  tracing:
    enabled: true
    exporter: "otlp"
    endpoint: "http://otel-collector:4317"
    serviceName: "openclaw-gateway"
    sampleRate: 0.1  # 10% sampling

  instrumentation:
    http: true
    websocket: true
    database: true
    redis: true

  # Custom attributes
  attributes:
    deployment.environment: "production"
    service.version: "2026.3.8"
```

**Trace Span Structure:**

```
trace_id: abc123...
  │
  ├─ span_id: def456... [gateway.request]
  │   ├─ span_id: ghi789... [auth.validate]
  │   ├─ span_id: jkl012... [session.resolve]
  │   ├─ span_id: mno345... [agent.process]
  │   │   ├─ span_id: pqr678... [model.invoke]
  │   │   └─ span_id: stu901... [tool.execute]
  │   └─ span_id: vwx234... [channel.send]
```

### Alerting Rules

**Prometheus Alert Rules:**

```yaml
groups:
  - name: openclaw
    interval: 30s
    rules:
      - alert: HighErrorRate
        expr: rate(openclaw_errors_total[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} errors/sec"

      - alert: GatewayDown
        expr: up{job="openclaw"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "OpenClaw Gateway is down"
          description: "Gateway instance {{ $labels.instance }} is not responding"

      - alert: HighSessionCount
        expr: sum(openclaw_sessions_active) > 1000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High active session count"
          description: "{{ $value }} active sessions"

      - alert: ClauditorHeartbeatMissing
        expr: clauditor_heartbeat_age_seconds > 60
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Clauditor heartbeat missing"
          description: "Security watchdog heartbeat not received for {{ $value }} seconds"

      - alert: SecurityAlertDetected
        expr: clauditor_alerts_total{severity="critical"} > 0
        for: 0m
        labels:
          severity: critical
        annotations:
          summary: "Critical security alert detected"
          description: "{{ $value }} critical security alerts"
```

---

## High Availability & Scalability

### Scaling Strategies

#### Horizontal Scaling

```
┌─────────────────────────────────────────────────────────────────┐
│              Horizontal Scaling Architecture                     │
└─────────────────────────────────────────────────────────────────┘

                    ┌─────────────────┐
                    │   Load Balancer │
                    │   (Round Robin) │
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
  │  Gateway 1  │     │  Gateway 2  │     │  Gateway N  │
  │  (Stateless)│     │  (Stateless)│     │  (Stateless)│
  └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             │
                             ▼
                  ┌───────────────────┐
                  │  Session Store    │
                  │  (Redis Cluster)  │
                  └───────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
  │ PostgreSQL  │     │  LanceDB    │     │  S3/MinIO   │
  │   Primary   │     │   Cluster   │     │   Cluster   │
  └─────────────┘     └─────────────┘     └─────────────┘
```

**Scaling Limits:**

| Component | Single Node | Cluster | Notes |
|-----------|-------------|---------|-------|
| Concurrent Sessions | 500-1000 | 10,000+ | Limited by memory |
| Messages/sec | 100-200 | 1000+ | Limited by model API |
| Active Agents | 10-20 | 100+ | Limited by concurrency |
| WebSocket Connections | 1000-2000 | 10,000+ | Limited by file descriptors |

### Performance Tuning

**Node.js Optimization:**

```yaml
# Environment variables for production
NODE_ENV: production
NODE_OPTIONS: "--max-old-space-size=2048 --max-semi-space-size=128"

# Cluster mode (optional)
OPENCLAW_CLUSTER: "true"
OPENCLAW_WORKERS: "auto"  # Auto-detect CPU cores
```

**Gateway Configuration:**

```yaml
# config/performance.yml
performance:
  # Concurrency limits
  concurrency:
    maxAgents: 20
    maxSessionsPerAgent: 100
    maxConcurrentToolCalls: 10

  # Memory management
  memory:
    sessionCompactionThreshold: 100  # Messages before compaction
    maxSessionHistoryMessages: 500
    gcInterval: "5m"

  # Queue configuration
  queue:
    maxQueueSize: 1000
    processingTimeout: "60s"
    retryAttempts: 3

  # Connection limits
  connections:
    maxWebsockets: 2000
    idleTimeout: "30m"
    pingInterval: "30s"
```

---

## Disaster Recovery

### Backup Strategy

**Automated Backup Script:**

```bash
#!/bin/bash
# /opt/openclaw/scripts/backup.sh

set -euo pipefail

BACKUP_DIR="/var/backups/openclaw"
DATE=$(date +%Y%m%d_%H%M%S)
RETENTION_DAYS=30

# Create backup directory
mkdir -p "${BACKUP_DIR}/${DATE}"

# Backup configuration
cp -r /etc/openclaw "${BACKUP_DIR}/${DATE}/config"

# Backup sessions (with compaction)
openclaw sessions compact --all
cp -r /var/lib/openclaw/sessions "${BACKUP_DIR}/${DATE}/sessions"

# Backup secrets (already encrypted)
cp -r /var/lib/openclaw/secrets "${BACKUP_DIR}/${DATE}/secrets"

# Backup audit logs
cp -r /var/log/sysaudit "${BACKUP_DIR}/${DATE}/audit-logs"

# Create archive
cd "${BACKUP_DIR}"
tar -czf "openclaw-backup-${DATE}.tar.gz" "${DATE}"
rm -rf "${DATE}"

# Rotate old backups
find "${BACKUP_DIR}" -name "*.tar.gz" -mtime +${RETENTION_DAYS} -delete

# Upload to remote storage (optional)
aws s3 cp "openclaw-backup-${DATE}.tar.gz" "s3://backups/openclaw/"

echo "Backup completed: ${BACKUP_DIR}/openclaw-backup-${DATE}.tar.gz"
```

**Cron Job:**

```bash
# /etc/cron.d/openclaw-backup
# Daily backup at 2:00 AM
0 2 * * * root /opt/openclaw/scripts/backup.sh >> /var/log/openclaw/backup.log 2>&1
```

### Recovery Procedures

**Full System Recovery:**

```bash
#!/bin/bash
# /opt/openclaw/scripts/restore.sh

set -euo pipefail

BACKUP_FILE="$1"

if [ -z "${BACKUP_FILE}" ]; then
    echo "Usage: $0 <backup-file.tar.gz>"
    exit 1
fi

# Stop services
systemctl stop openclaw
systemctl stop clauditor

# Extract backup
BACKUP_DIR=$(mktemp -d)
tar -xzf "${BACKUP_FILE}" -C "${BACKUP_DIR}"

# Restore configuration
rsync -av "${BACKUP_DIR}/"*/config/ /etc/openclaw/

# Restore sessions
rsync -av "${BACKUP_DIR}/"*/sessions/ /var/lib/openclaw/sessions/

# Restore secrets
rsync -av "${BACKUP_DIR}/"*/secrets/ /var/lib/openclaw/secrets/

# Set permissions
chown -R openclaw:openclaw /etc/openclaw /var/lib/openclaw
chmod 600 /var/lib/openclaw/secrets/*

# Start services
systemctl start clauditor
systemctl start openclaw

# Verify health
sleep 5
openclaw system health

echo "Restore completed successfully"
```

### Failover Procedure

**Manual Failover:**

```bash
# 1. Stop primary gateway
ssh primary.openclaw.example.com "systemctl stop openclaw"

# 2. Promote secondary
ssh secondary.openclaw.example.com << 'EOF'
  # Update configuration to point to primary storage
  sed -i 's/role: standby/role: primary/' /etc/openclaw/config.yml

  # Start gateway
  systemctl start openclaw

  # Verify health
  openclaw system health
EOF

# 3. Update DNS/load balancer
# (Update to point to secondary)

# 4. Verify service
curl -f https://openclaw.example.com/health
```

---

## Operational Procedures

### Runbook: Service Restart

```markdown
# Runbook: Gateway Service Restart

## Purpose
Safely restart the OpenClaw Gateway service.

## Pre-conditions
- Access to server with sudo privileges
- Backup completed within last 24 hours

## Procedure

1. Check current status
   ```bash
   systemctl status openclaw
   openclaw system health
   ```

2. Notify active sessions (optional)
   ```bash
   openclaw broadcast "Gateway will restart in 2 minutes for maintenance"
   ```

3. Stop the service
   ```bash
   systemctl stop openclaw
   ```

4. Verify stopped
   ```bash
   systemctl status openclaw  # Should show "inactive"
   netstat -tlnp | grep 18788 # Should show no listeners
   ```

5. Start the service
   ```bash
   systemctl start openclaw
   ```

6. Verify health
   ```bash
   systemctl status openclaw
   curl -f http://localhost:18788/health
   openclaw system health
   ```

7. Monitor for errors
   ```bash
   journalctl -u openclaw -f --since "2 minutes ago"
   ```

## Rollback
If service fails to start:
1. Check logs: `journalctl -u openclaw -n 100`
2. Restore from backup if needed
3. Contact on-call engineer

## Post-conditions
- Service is running and healthy
- No error logs in last 5 minutes
```

### Runbook: Security Incident Response

```markdown
# Runbook: Security Incident Response

## Purpose
Respond to security alerts from Clauditor.

## Trigger
- Clauditor alert received (syslog, email, PagerDuty)
- Severity: high or critical

## Immediate Actions

1. Assess the alert
   ```bash
   # View recent alerts
   clauditor digest --since "1 hour ago"

   # View specific alert details
   clauditor digest --alert-id <alert-id>
   ```

2. Isolate if necessary
   ```bash
   # Block network access (if exfiltration suspected)
   iptables -A OUTPUT -d <suspicious-ip> -j DROP

   # Kill suspicious processes
   ps aux | grep -E "(nc|curl|wget)" | grep -v grep
   ```

3. Preserve evidence
   ```bash
   # Export audit logs
   clauditor digest --format json > /tmp/incident-logs.json

   # Capture system state
   ps auxf > /tmp/process-tree.txt
   netstat -tulpn > /tmp/network-connections.txt
   ```

4. Rotate credentials
   ```bash
   # Rotate all secrets
   openclaw secrets rotate --all

   # Invalidate sessions
   openclaw sessions invalidate --all
   ```

5. Notify stakeholders
   - Security team
   - Management
   - Legal (if PII involved)

## Investigation

1. Review audit logs
2. Identify affected systems/data
3. Determine root cause
4. Document timeline

## Remediation

1. Patch vulnerability
2. Update configurations
3. Restore from clean backup if needed
4. Re-enable services

## Post-Incident

1. Write post-mortem
2. Update runbooks
3. Implement additional monitoring
4. Schedule security review
```

---

## Compliance Considerations

### Data Protection

**GDPR Compliance Checklist:**

- [ ] Data processing agreement with LLM providers
- [ ] User consent mechanisms for data collection
- [ ] Data subject access request (DSAR) procedures
- [ ] Right to erasure implementation
- [ ] Data retention policies
- [ ] Cross-border transfer safeguards

**Data Retention Configuration:**

```yaml
# config/retention.yml
retention:
  # Session data
  sessions:
    activeRetention: "90d"
    archivedRetention: "1y"
    compactionAfter: "100 messages"

  # Audit logs
  audit:
    retention: "7y"  # Legal requirement
    storage: "immutable"

  # Telemetry
  metrics:
    retention: "30d"
    aggregation: "1h"

  # Message content
  messages:
    retention: "30d"
    encryption: true
```

### Security Certifications

**SOC 2 Controls Mapping:**

| Control | Implementation |
|---------|----------------|
| CC6.1 - Logical Access | RBAC, MFA support |
| CC6.6 - Encryption | TLS 1.3, AES-256 at rest |
| CC6.7 - Transmission Security | Certificate validation |
| CC7.1 - Intrusion Detection | Clauditor integration |
| CC7.2 - Incident Response | Documented procedures |
| CC8.1 - Change Management | Version control, CI/CD |

---

## Summary

**src.openclaw Production Readiness:**

| Area | Status | Notes |
|------|--------|-------|
| Deployment | Production Ready | Docker, Nix, Ansible |
| Security | Production Ready | Clauditor, RBAC, encryption |
| Observability | Production Ready | Logging, metrics, tracing |
| HA/Scalability | Beta | Single-node primary, HA achievable |
| Disaster Recovery | Production Ready | Backup/restore procedures |
| Compliance | Configurable | GDPR, SOC 2 support |

**Key Strengths:**
1. Comprehensive security model with tamper-evident logging
2. Multiple deployment options for different environments
3. Extensive observability with Prometheus/Grafana integration
4. Well-documented operational procedures

**Areas for Improvement:**
1. Native clustering support for true HA
2. Automated failover mechanisms
3. Enhanced multi-region deployment support
4. Built-in backup automation (currently script-based)
