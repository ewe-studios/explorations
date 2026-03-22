# Taubyte Production-Grade Considerations

## Overview

This document covers production-grade considerations for deploying and operating Taubyte at scale. It addresses reliability, scalability, security, observability, and operational best practices.

---

## Architecture Review for Production

### High Availability Design

```
┌─────────────────────────────────────────────────────────────────────┐
│                      PRODUCTION ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    LOAD BALANCER                              │   │
│  │              (HAProxy / nginx / cloud LB)                     │   │
│  └──────────────────────────────────────────────────────────────┘   │
│                              │                                        │
│         ┌────────────────────┼────────────────────┐                  │
│         │                    │                    │                   │
│  ┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐           │
│  │  Gateway-1  │     │  Gateway-2  │     │  Gateway-N  │           │
│  └──────┬──────┘     └──────┬──────┘     └──────┬──────┘           │
│         │                    │                    │                   │
│         └────────────────────┴────────────────────┘                  │
│                              │                                        │
│  ┌───────────────────────────┴───────────────────────────────────┐  │
│  │                     P2P NETWORK                                │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐      │  │
│  │  │Monkey-1│ │Monkey-2│ │Monkey-3│ │Monkey-N│ │Monkey-X│      │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘      │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐      │  │
│  │  │Patrick │ │ Seer-1 │ │ Seer-2 │ │Hoarder-│ │ Auth-1 │      │  │
│  │  │ (HA)   │ │  (HA)  │ │  (HA)  │ │1,2,3   │ │  (HA)  │      │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘      │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    STORAGE LAYER                               │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐           │  │
│  │  │   Pebble    │  │   Hoarder   │  │  External   │           │  │
│  │  │   (Local)   │  │   (S3)      │  │  (Postgres) │           │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘           │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Service Replication Strategy

| Service | Replicas | State | Notes |
|---------|----------|-------|-------|
| Gateway | N+1 | Stateless | Behind LB |
| Monkey | N | Stateless | Auto-scale based on load |
| Patrick | 2 | Pebble DB | Leader-follower |
| Seer | 2+ | Pebble DB | DNS requires coordination |
| Hoarder | N | S3/Object | Content-addressable |
| Auth | 2 | Pebble DB | Session affinity needed |
| TNS | 3+ | DHT | Fully distributed |

---

## Deployment Strategies

### Container Deployment (Docker)

```yaml
# docker-compose.production.yml
version: '3.8'

services:
  tau-gateway:
    image: taubyte/tau:latest
    command: tau start gateway
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
    networks:
      - tau-network
    depends_on:
      - tau-auth
      - tau-seer

  tau-monkey:
    image: taubyte/tau:latest
    command: tau start monkey
    deploy:
      replicas: 5
      resources:
        limits:
          cpus: '4'
          memory: 8G
    networks:
      - tau-network
    volumes:
      - monkey-storage:/var/lib/tau/monkey

  tau-patrick:
    image: taubyte/tau:latest
    command: tau start patrick
    deploy:
      replicas: 2
    networks:
      - tau-network
    volumes:
      - patrick-storage:/var/lib/tau/patrick

  tau-seer:
    image: taubyte/tau:latest
    command: tau start seer
    ports:
      - "53:53/udp"
      - "53:53/tcp"
    deploy:
      replicas: 2
    networks:
      - tau-network
    volumes:
      - seer-storage:/var/lib/tau/seer

  tau-hoarder:
    image: taubyte/tau:latest
    command: tau start hoarder
    deploy:
      replicas: 3
    networks:
      - tau-network
    volumes:
      - hoarder-storage:/var/lib/tau/hoarder

  tau-auth:
    image: taubyte/tau:latest
    command: tau start auth
    deploy:
      replicas: 2
    networks:
      - tau-network
    volumes:
      - auth-storage:/var/lib/tau/auth

networks:
  tau-network:
    driver: overlay
    attachable: true

volumes:
  monkey-storage:
  patrick-storage:
  seer-storage:
  hoarder-storage:
  auth-storage:
```

### Kubernetes Deployment

```yaml
# k8s/tau-statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: tau-monkey
spec:
  serviceName: tau-monkey
  replicas: 5
  selector:
    matchLabels:
      app: tau-monkey
  template:
    metadata:
      labels:
        app: tau-monkey
    spec:
      containers:
      - name: monkey
        image: taubyte/tau:latest
        command: ["tau", "start", "monkey"]
        resources:
          requests:
            cpu: "1"
            memory: "2Gi"
          limits:
            cpu: "4"
            memory: "8Gi"
        volumeMounts:
        - name: tau-storage
          mountPath: /var/lib/tau
        env:
        - name: TAU_CONFIG_PATH
          value: "/etc/tau/config"
        - name: TAU_DEV_MODE
          value: "false"
        readinessProbe:
          httpGet:
            path: /health
            port: 7777
          initialDelaySeconds: 10
          periodSeconds: 5
        livenessProbe:
          httpGet:
            path: /health
            port: 7777
          initialDelaySeconds: 30
          periodSeconds: 30
  volumeClaimTemplates:
  - metadata:
      name: tau-storage
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: "fast-ssd"
      resources:
        requests:
          storage: 50Gi
```

---

## Configuration Management

### Production Configuration

```yaml
# config/production.yaml
global:
  dev_mode: false
  log_level: info
  log_format: json

network:
  p2p:
    listen_addresses:
      - "/ip4/0.0.0.0/tcp/4001"
      - "/ip6/::/tcp/4001"
    bootstrap_peers:
      - "/ip4/BOOTSTRAP-1/ip4/4001/tcp/p2p/ID1"
      - "/ip4/BOOTSTRAP-2/ip4/4001/tcp/p2p/ID2"
    connection_manager:
      low_water: 100
      high_water: 400
      grace_period: 30s

gateway:
  port: 80
  tls_port: 443
  max_connections: 10000
  read_timeout: 30s
  write_timeout: 30s
  idle_timeout: 120s

monkey:
  max_containers: 500
  container_max_age: 15m
  build:
    timeout: 30m
    memory_limit: 4GB
  execution:
    default_memory: 128MB
    default_timeout: 30s
    max_memory: 1GB
    max_timeout: 300s

patrick:
  workers: 20
  max_retries: 3
  retry_delay: 10s
  queue_size: 5000

seer:
  dns:
    port: 53
    cache:
      positive_ttl: 5m
      negative_ttl: 1m
      max_size: 100000
  heartbeat:
    timeout: 5m
    check_interval: 30s

hoarder:
  storage:
    type: s3
    bucket: tau-artifacts
    region: us-east-1
  cache:
    max_size: 10GB
  replication:
    factor: 3

auth:
  github:
    client_id: "${GITHUB_CLIENT_ID}"
    client_secret: "${GITHUB_CLIENT_SECRET}"
  acme:
    email: "admin@example.com"
    directory_url: "https://acme-v02.api.letsencrypt.org/directory"
  session:
    timeout: 24h
    secure_cookies: true
```

### Environment Variables

```bash
# .env.production
# Global
TAU_ENV=production
TAU_LOG_LEVEL=info
TAU_CONFIG_PATH=/etc/tau/config

# GitHub OAuth
GITHUB_CLIENT_ID=your_client_id
GITHUB_CLIENT_SECRET=your_client_secret

# S3 Storage
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_REGION=us-east-1
S3_BUCKET=tau-artifacts

# Database
PEBBLE_DATA_DIR=/var/lib/tau/storage

# TLS
TLS_CERT_PATH=/etc/ssl/tau/cert.pem
TLS_KEY_PATH=/etc/ssl/tau/key.pem

# Monitoring
PROMETHEUS_PORT=9090
JAEGER_ENDPOINT=http://jaeger:14268/api/traces
```

---

## Scalability Considerations

### Horizontal Scaling

```
┌─────────────────────────────────────────────────────────────┐
│                  MONKEY AUTO-SCALING                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Metrics:                                                   │
│  • Container count                                          │
│  • Job queue depth                                          │
│  • CPU utilization                                          │
│  • Request latency                                          │
│                                                             │
│  Scale Up:                                                  │
│  • Trigger: queue_depth > 100 OR cpu > 70%                 │
│  • Action: +2 replicas                                      │
│  • Cooldown: 5 minutes                                      │
│                                                             │
│  Scale Down:                                                │
│  • Trigger: queue_depth < 20 AND cpu < 30%                 │
│  • Action: -1 replica                                       │
│  • Cooldown: 10 minutes                                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Database Scaling

```go
// Pebble configuration for production
import "github.com/cockroachdb/pebble"

func configurePebble(path string) (*pebble.DB, error) {
    opts := &pebble.Options{
        Cache: pebble.NewCache(4 * 1024 * 1024 * 1024), // 4GB cache
        MemTableSize: 64 << 20,                          // 64MB
        MaxOpenFiles: 10000,
        WALBytesPerSync: 1 << 20,                        // 1MB
        L0CompactionThreshold: 4,
        L0StopWritesThreshold: 12,
        LBaseMaxBytes: 512 << 20,                        // 512MB
        Levels: []pebble.LevelOptions{
            {
                BlockSize: 32 << 10,                     // 32KB
                Compression: func() pebble.Compression {
                    return pebble.SnappyCompression
                }(),
            },
        },
    }

    return pebble.Open(path, opts)
}
```

---

## Observability

### Metrics Collection

```go
// Prometheus metrics
import "github.com/prometheus/client_golang/prometheus"

var (
    // Monkey metrics
    monkeyContainersActive = prometheus.NewGauge(prometheus.GaugeOpts{
        Name: "tau_monkey_containers_active",
        Help: "Number of active WASM containers",
    })

    monkeyJobQueueDepth = prometheus.NewGauge(prometheus.GaugeOpts{
        Name: "tau_monkey_job_queue_depth",
        Help: "Number of jobs in queue",
    })

    monkeyExecutionDuration = prometheus.NewHistogram(prometheus.HistogramOpts{
        Name:    "tau_monkey_execution_duration_seconds",
        Help:    "Function execution duration",
        Buckets: prometheus.DefBuckets,
    })

    // Patrick metrics
    patrickJobsTotal = prometheus.NewCounterVec(prometheus.CounterOpts{
        Name: "tau_patrick_jobs_total",
        Help: "Total number of jobs processed",
    }, []string{"type", "status"})

    // Seer metrics
    seerDNSQueriesTotal = prometheus.NewCounter(prometheus.CounterOpts{
        Name: "tau_seer_dns_queries_total",
        Help: "Total DNS queries",
    })

    seerDNSCacheHits = prometheus.NewCounter(prometheus.CounterOpts{
        Name: "tau_seer_dns_cache_hits_total",
        Help: "DNS cache hits",
    })
)

func registerMetrics() {
    prometheus.MustRegister(
        monkeyContainersActive,
        monkeyJobQueueDepth,
        monkeyExecutionDuration,
        patrickJobsTotal,
        seerDNSQueriesTotal,
        seerDNSCacheHits,
    )
}
```

### Distributed Tracing

```go
// OpenTelemetry tracing
import (
    "go.opentelemetry.io/otel"
    "go.opentelemetry.io/otel/exporters/jaeger"
    "go.opentelemetry.io/otel/sdk/trace"
)

func initTracing() (*trace.TracerProvider, error) {
    exporter, err := jaeger.New(jaeger.WithCollectorEndpoint(
        jaeger.WithEndpoint("http://jaeger:14268/api/traces"),
    ))
    if err != nil {
        return nil, err
    }

    tp := trace.NewTracerProvider(
        trace.WithBatcher(exporter),
        trace.WithSampler(trace.AlwaysSample()),
    )

    otel.SetTracerProvider(tp)
    return tp, nil
}

// Usage in service
func handleRequest(ctx context.Context, req *Request) (*Response, error) {
    ctx, span := otel.Tracer("tau/monkey").Start(ctx, "handleRequest")
    defer span.End()

    span.SetAttributes(
        attribute.String("function.id", req.FunctionID),
        attribute.String("method", req.Method),
    )

    // ... process request
}
```

### Logging

```go
// Structured logging with Zap
import "go.uber.org/zap"

var logger *zap.Logger

func initLogger(config *Config) error {
    cfg := zap.NewProductionConfig()
    cfg.OutputPaths = []string{config.LogPath}
    cfg.Level.SetLevel(zap.InfoLevel)

    var err error
    logger, err = cfg.Build()
    return err
}

// Usage
logger.Info("Starting job execution",
    zap.String("job_id", job.ID),
    zap.String("function_id", job.FunctionID),
    zap.Int("retry_count", job.Retries),
)
```

---

## Security Hardening

### Network Security

```yaml
# Network policies (Kubernetes)
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: tau-network-policy
spec:
  podSelector:
    matchLabels:
      app: tau
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress
    ports:
    - protocol: TCP
      port: 80
    - protocol: TCP
      port: 443
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: storage
    ports:
    - protocol: TCP
      port: 443  # S3
  - to:
    - namespaceSelector:
        matchLabels:
          name: external
    ports:
    - protocol: TCP
      port: 443  # GitHub, ACME
```

### Secret Management

```go
// Using HashiCorp Vault
import vault "github.com/hashicorp/vault/api"

func getSecret(path string) (string, error) {
    client, _ := vault.NewClient(vault.DefaultConfig())

    secret, err := client.Logical().Read(path)
    if err != nil {
        return "", err
    }

    return secret.Data["value"].(string), nil
}

// Usage in service
githubSecret, _ := getSecret("secret/tau/github/client_secret")
s3Secret, _ := getSecret("secret/tau/s3/access_key")
```

### TLS Configuration

```go
// TLS configuration
import "crypto/tls"

func configureTLS() *tls.Config {
    return &tls.Config{
        MinVersion: tls.VersionTLS13,
        CipherSuites: []uint16{
            tls.TLS_AES_256_GCM_SHA384,
            tls.TLS_CHACHA20_POLY1305_SHA256,
            tls.TLS_AES_128_GCM_SHA256,
        },
        PreferServerCipherSuites: true,
        CurvePreferences: []tls.CurveID{
            tls.X25519,
            tls.CurveP256,
        },
    }
}
```

---

## Disaster Recovery

### Backup Strategy

```bash
#!/bin/bash
# backup.sh - Production backup script

BACKUP_DIR="/backup/tau"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Backup Pebble databases
for service in patrick seer auth tns; do
    echo "Backing up $service..."
    tar -czf $BACKUP_DIR/${service}_${TIMESTAMP}.tar.gz \
        /var/lib/tau/${service}/
done

# Sync to S3
aws s3 sync $BACKUP_DIR s3://tau-backups/$TIMESTAMP/

# Cleanup old backups (keep 30 days)
find $BACKUP_DIR -mtime +30 -delete

echo "Backup completed: $TIMESTAMP"
```

### Recovery Procedure

```bash
#!/bin/bash
# recovery.sh - Disaster recovery script

BACKUP_TIMESTAMP=$1

echo "Starting recovery from backup: $BACKUP_TIMESTAMP"

# Stop services
systemctl stop tau-*

# Download backup
aws s3 sync s3://tau-backups/$BACKUP_TIMESTAMP/ /restore/

# Restore databases
for service in patrick seer auth tns; do
    echo "Restoring $service..."
    tar -xzf /restore/${service}_*.tar.gz -C /var/lib/tau/
done

# Start services
systemctl start tau-seer
sleep 5
systemctl start tau-auth
sleep 5
systemctl start tau-patrick
systemctl start tau-monkey
systemctl start tau-gateway

echo "Recovery completed"
```

---

## Performance Tuning

### P2P Tuning

```go
// Libp2p configuration for production
import (
    libp2p "github.com/libp2p/go-libp2p"
    "github.com/libp2p/go-libp2p-core/peerstore"
)

func configureLibp2p() ([]libp2p.Option, error) {
    return []libp2p.Option{
        libp2p.ConnectionManager(connmgr.NewConnManager(
            100,  // Low water
            400,  // High water
            30*time.Second,  // Grace period
        )),
        libp2p.Peerstore(peerstore.NewPeerstore()),
        libp2p.Routing(func(h host.Host) (routing.PeerRouting, error) {
            return dht.New(context.Background(), h, dht.ModeServer)
        }),
        libp2p.Transport(tcp.NewTCPTransport),
        libp2p.Security(noise.New),
        libp2p.Muxer("/yamux/1.0.0", yamux.DefaultTransport),
        libp2p.EnableNATService(),
        libp2p.EnableAutoRelay(),
    }, nil
}
```

### WASM Performance

```go
// Wazero configuration for production
import "github.com/tetratelabs/wazero"

func configureWazero() wazero.RuntimeConfig {
    return wazero.NewRuntimeConfig().
        WithCoreFeatures(api.CoreFeaturesV2).
        WithMemoryLimit(1024 * 1024 * 1024).  // 1GB
        WithCompilationCache(wazero.NewCompilationCache())
}
```

---

## Monitoring Dashboard

### Grafana Dashboard JSON (excerpt)

```json
{
  "dashboard": {
    "title": "Taubyte Production",
    "panels": [
      {
        "title": "Monkey Containers",
        "targets": [
          {
            "expr": "tau_monkey_containers_active",
            "legendFormat": "Active Containers"
          }
        ]
      },
      {
        "title": "Job Queue Depth",
        "targets": [
          {
            "expr": "tau_monkey_job_queue_depth",
            "legendFormat": "Queue Depth"
          }
        ]
      },
      {
        "title": "DNS Queries/sec",
        "targets": [
          {
            "expr": "rate(tau_seer_dns_queries_total[1m])",
            "legendFormat": "QPS"
          }
        ]
      }
    ]
  }
}
```

---

## Operational Runbook

### Incident Response

#### High CPU Usage

1. Check metrics: `kubectl top pods -l app=tau-monkey`
2. Identify hot functions: Check execution duration metrics
3. Scale up: `kubectl scale statefulset tau-monkey --replicas=10`
4. Investigate: Review function logs

#### DNS Resolution Failures

1. Check Seer health: `curl http://seer:7780/health`
2. Verify DNS port: `netstat -tulpn | grep :53`
3. Check cache: Review cache hit rate metrics
4. Restart if needed: `kubectl rollout restart statefulset tau-seer`

#### Build Queue Backlog

1. Check Patrick queue: Review queue depth metrics
2. Scale workers: Increase Patrick replicas
3. Check Hoarder: Verify storage availability
4. Review failed jobs: Check error rates

---

## Related Documents

- `exploration.md` - Main exploration
- `architecture-deep-dive.md` - Architecture details
- `subsystems/*.md` - Service-specific guides
- `rust-revision.md` - Rust implementation guide
